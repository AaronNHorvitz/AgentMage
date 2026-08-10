#!/usr/bin/env python3
"""Enforce the additions-only AgentMage canonical inventory baseline."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from copy import deepcopy
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Final

ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_REGISTRY: Final = ROOT / "requirements" / "registry.json"
DEFAULT_INVENTORY: Final = ROOT / "Agent-Scaffolding-Inventory.md"
DEFAULT_BASELINE: Final = ROOT / "requirements" / "additions-only-baseline.json"
HEADING: Final = re.compile(r"^#{1,6}\s+(.+?)\s*$")
CHECKLIST: Final = re.compile(r"^- \[[ xX]\] `([^`]+)`\s+(.+?)\s*$")
REQUIREMENT_FIELDS: Final = (
    "id",
    "kind",
    "title",
    "release",
    "dependencies",
    "disposition",
    "acceptance_tests",
)
EXACT_REQUIREMENT_FIELDS: Final = (
    "kind",
    "title",
    "release",
    "disposition",
)
SET_REQUIREMENT_FIELDS: Final = (
    "dependencies",
    "acceptance_tests",
)


class AdditionsOnlyError(ValueError):
    """Raised when additions-only inputs are malformed."""


@dataclass(frozen=True, order=True)
class Diagnostic:
    category: str
    location: str
    message: str


def load_json(path: Path) -> dict[str, object]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AdditionsOnlyError(f"{path}: expected a JSON object")
    return value


def checklist_id(heading: str, label: str, statement: str) -> str:
    identity = "\0".join((heading, label, statement)).encode("utf-8")
    return "CL-" + hashlib.sha256(identity).hexdigest()[:20].upper()


def parse_checklist(path: Path = DEFAULT_INVENTORY) -> list[dict[str, object]]:
    """Parse all canonical checklist entries, independent of checkbox state."""
    heading = ""
    in_fence = False
    records: list[dict[str, object]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if line.startswith("```"):
            in_fence = not in_fence
            continue
        heading_match = HEADING.match(line)
        if heading_match and not in_fence:
            heading = heading_match.group(1)
            continue
        if in_fence:
            continue
        checklist_match = CHECKLIST.match(line)
        if checklist_match is None:
            continue
        label, statement = checklist_match.groups()
        records.append(
            {
                "id": checklist_id(heading, label, statement),
                "label": label,
                "statement": statement,
                "source": {
                    "document": path.name,
                    "heading": heading,
                    "line": line_number,
                    "semantic_sha256": hashlib.sha256(
                        "\0".join((heading, label, statement)).encode("utf-8")
                    ).hexdigest(),
                },
            }
        )

    identifiers = [str(record["id"]) for record in records]
    duplicates = sorted(
        identifier for identifier, count in Counter(identifiers).items() if count > 1
    )
    if duplicates:
        raise AdditionsOnlyError(
            "duplicate canonical checklist identities: " + ", ".join(duplicates)
        )
    if not records:
        raise AdditionsOnlyError("canonical inventory has no checklist entries")
    return sorted(records, key=lambda record: str(record["id"]))


def baseline_requirement(record: dict[str, object]) -> dict[str, object]:
    missing = [field for field in REQUIREMENT_FIELDS if field not in record]
    if missing:
        raise AdditionsOnlyError(
            f"requirement {record.get('id', '<unknown>')} is missing: {', '.join(missing)}"
        )
    source = record.get("source")
    if not isinstance(source, dict):
        raise AdditionsOnlyError(f"requirement {record['id']} has no source object")
    return {
        field: deepcopy(record[field]) for field in REQUIREMENT_FIELDS
    } | {
        "source": {
            "document": source.get("document"),
            "heading": source.get("heading"),
        }
    }


def current_snapshot(
    registry: dict[str, object],
    inventory_path: Path = DEFAULT_INVENTORY,
) -> dict[str, list[dict[str, object]]]:
    requirements = registry.get("requirements")
    if not isinstance(requirements, list) or any(
        not isinstance(record, dict) for record in requirements
    ):
        raise AdditionsOnlyError("registry requirements must be an array of objects")
    baseline_requirements = [baseline_requirement(record) for record in requirements]
    requirement_ids = [str(record["id"]) for record in baseline_requirements]
    duplicates = sorted(
        identifier for identifier, count in Counter(requirement_ids).items() if count > 1
    )
    if duplicates:
        raise AdditionsOnlyError(
            "duplicate requirement identities: " + ", ".join(duplicates)
        )
    return {
        "requirements": sorted(
            baseline_requirements,
            key=lambda record: str(record["id"]),
        ),
        "checklist": parse_checklist(inventory_path),
    }


def build_initial_baseline(
    registry: dict[str, object],
    inventory_path: Path = DEFAULT_INVENTORY,
) -> dict[str, object]:
    snapshot = current_snapshot(registry, inventory_path)
    return {
        "schema_version": 1,
        "policy": {
            "mode": "additions_only",
            "removed_requirement": "blocked",
            "weakened_requirement": "blocked",
            "removed_or_reclassified_checklist_entry": "blocked",
            "new_requirement_or_checklist_entry": "allowed_and_reviewable",
            "supersession": "preserve_original_and_add_replacement",
        },
        "counts": {
            "requirements": len(snapshot["requirements"]),
            "checklist": len(snapshot["checklist"]),
        },
        **snapshot,
    }


def audit_additions_only(
    baseline: dict[str, object],
    registry: dict[str, object],
    inventory_path: Path = DEFAULT_INVENTORY,
) -> list[Diagnostic]:
    """Return deterministic diagnostics without mutating inputs or files."""
    if baseline.get("schema_version") != 1:
        raise AdditionsOnlyError("unsupported additions-only baseline version")
    baseline_requirements = baseline.get("requirements")
    baseline_checklist = baseline.get("checklist")
    if not isinstance(baseline_requirements, list) or not isinstance(
        baseline_checklist, list
    ):
        raise AdditionsOnlyError("baseline requirement and checklist arrays are required")

    current = current_snapshot(registry, inventory_path)
    current_requirements = {
        str(record["id"]): record for record in current["requirements"]
    }
    diagnostics: list[Diagnostic] = []

    for prior in baseline_requirements:
        if not isinstance(prior, dict) or not isinstance(prior.get("id"), str):
            raise AdditionsOnlyError("baseline contains a malformed requirement")
        identifier = str(prior["id"])
        active = current_requirements.get(identifier)
        if active is None:
            diagnostics.append(
                Diagnostic(
                    "removed_requirement",
                    identifier,
                    "baseline requirement is absent from the canonical registry",
                )
            )
            continue

        for field in EXACT_REQUIREMENT_FIELDS:
            if active.get(field) != prior.get(field):
                diagnostics.append(
                    Diagnostic(
                        "weakened_or_changed_requirement",
                        identifier,
                        f"protected field {field} changed from {prior.get(field)!r} "
                        f"to {active.get(field)!r}",
                    )
                )
        for field in SET_REQUIREMENT_FIELDS:
            prior_values = set(prior.get(field, []))
            active_values = set(active.get(field, []))
            removed = sorted(prior_values - active_values)
            if removed:
                diagnostics.append(
                    Diagnostic(
                        "weakened_requirement",
                        identifier,
                        f"protected {field} entries were removed: {', '.join(removed)}",
                    )
                )
        if active.get("source") != prior.get("source"):
            diagnostics.append(
                Diagnostic(
                    "moved_requirement",
                    identifier,
                    "canonical source document or heading changed",
                )
            )

    current_checklist = {str(record["id"]): record for record in current["checklist"]}
    by_statement = {
        str(record["statement"]): record for record in current["checklist"]
    }
    for prior in baseline_checklist:
        if not isinstance(prior, dict) or not isinstance(prior.get("id"), str):
            raise AdditionsOnlyError("baseline contains a malformed checklist entry")
        identifier = str(prior["id"])
        if identifier in current_checklist:
            continue
        same_statement = by_statement.get(str(prior.get("statement")))
        if same_statement is not None:
            category = "moved_or_reclassified_checklist_entry"
            message = (
                "checklist statement remains but its heading or policy label changed "
                f"to {same_statement['source']['heading']!r}/"
                f"{same_statement['label']!r}"
            )
        else:
            category = "removed_or_modified_checklist_entry"
            message = "baseline checklist statement is absent or textually modified"
        diagnostics.append(Diagnostic(category, identifier, message))

    return sorted(set(diagnostics))


def update_baseline(
    baseline: dict[str, object],
    registry: dict[str, object],
    inventory_path: Path = DEFAULT_INVENTORY,
) -> dict[str, object]:
    """Append current additions only after all prior entries still pass."""
    diagnostics = audit_additions_only(baseline, registry, inventory_path)
    if diagnostics:
        raise AdditionsOnlyError(
            "cannot update a baseline with removed or changed protected entries"
        )
    current = current_snapshot(registry, inventory_path)
    updated = deepcopy(baseline)
    prior_requirement_ids = {
        str(record["id"]) for record in updated["requirements"]
    }
    prior_checklist_ids = {str(record["id"]) for record in updated["checklist"]}
    updated["requirements"].extend(
        record
        for record in current["requirements"]
        if str(record["id"]) not in prior_requirement_ids
    )
    updated["checklist"].extend(
        record
        for record in current["checklist"]
        if str(record["id"]) not in prior_checklist_ids
    )
    updated["requirements"] = sorted(
        updated["requirements"], key=lambda record: str(record["id"])
    )
    updated["checklist"] = sorted(
        updated["checklist"], key=lambda record: str(record["id"])
    )
    updated["counts"] = {
        "requirements": len(updated["requirements"]),
        "checklist": len(updated["checklist"]),
    }
    return updated


def render(value: dict[str, object]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def print_diagnostics(diagnostics: list[Diagnostic]) -> None:
    for diagnostic in diagnostics:
        print(
            f"- [{diagnostic.category}] {diagnostic.location}: {diagnostic.message}",
            file=sys.stderr,
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--initialize", action="store_true")
    action.add_argument("--update", action="store_true")
    action.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        registry = load_json(args.registry)
        if args.initialize:
            if args.baseline.exists():
                raise AdditionsOnlyError(
                    f"refusing to overwrite existing baseline: {args.baseline}"
                )
            args.baseline.parent.mkdir(parents=True, exist_ok=True)
            args.baseline.write_text(
                render(build_initial_baseline(registry, args.inventory)),
                encoding="utf-8",
            )
            return 0

        baseline = load_json(args.baseline)
        if args.update:
            args.baseline.write_text(
                render(update_baseline(baseline, registry, args.inventory)),
                encoding="utf-8",
            )
            return 0

        diagnostics = audit_additions_only(baseline, registry, args.inventory)
        if args.json:
            print(
                json.dumps(
                    {
                        "schema_version": 1,
                        "ok": not diagnostics,
                        "diagnostics": [asdict(item) for item in diagnostics],
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
        elif diagnostics:
            print("Additions-only inventory validation failed:", file=sys.stderr)
            print_diagnostics(diagnostics)
        else:
            print(
                f"Protected {len(baseline['requirements'])} requirement and "
                f"{len(baseline['checklist'])} checklist baseline entries."
            )
        return 0 if not diagnostics else 1
    except (OSError, UnicodeError, json.JSONDecodeError, AdditionsOnlyError) as error:
        print(f"Additions-only error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
