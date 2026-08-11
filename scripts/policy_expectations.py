#!/usr/bin/env python3
"""Build the canonical exclusion, rejected-default, and deferral registry."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Final


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_PRD: Final = ROOT / "PRD.md"
DEFAULT_INVENTORY: Final = ROOT / "Agent-Scaffolding-Inventory.md"
DEFAULT_OUTPUT: Final = ROOT / "requirements" / "policy-expectations.json"
SCHEMA_VERSION: Final = 1
HEADING: Final = re.compile(r"^#{1,6}\s+(.+?)\s*$")
CHECKLIST: Final = re.compile(r"^- \[ \] `([^`]+)`\s+(.+?)\s*$")
REFERENCE_ID: Final = re.compile(r"\b(?:AM|AT|CR)-[A-Z0-9.-]+\b")
EXPECTED_COUNTS: Final = {
    "deferred": 30,
    "rejected_default": 11,
    "release_deferred": 19,
    "v0.1_exclusion": 14,
}


class PolicyExpectationError(ValueError):
    """Raised when canonical policy source structures are invalid."""


def source_name(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path.name


def unique_references(statement: str) -> list[str]:
    seen: set[str] = set()
    references: list[str] = []
    for identifier in REFERENCE_ID.findall(statement):
        if identifier not in seen:
            seen.add(identifier)
            references.append(identifier)
    return references


def expectation_id(kind: str, statement_hash: str) -> str:
    prefix = {
        "deferred": "DEF",
        "rejected_default": "REJ",
        "release_deferred": "RDM",
        "v0.1_exclusion": "EXC",
    }[kind]
    return f"PX-{prefix}-{statement_hash[:16].upper()}"


def make_expectation(
    *,
    kind: str,
    statement: str,
    document: str,
    heading: str,
    line_number: int,
    source_line: str,
) -> dict[str, object]:
    statement_hash = hashlib.sha256((source_line + "\n").encode("utf-8")).hexdigest()
    behavior = {
        "deferred": (
            "until_promoted",
            "disabled_until_formally_promoted",
            "accepted_decision_and_appended_stable_backlog",
        ),
        "rejected_default": (
            "all_applicable_releases",
            "denied_by_default",
            "source_specific_gate_or_never",
        ),
        "release_deferred": (
            "before_target_release",
            "disabled_before_target_release",
            "target_release_gate",
        ),
        "v0.1_exclusion": (
            "v0.1",
            "absent_or_denied_in_v0.1",
            "new_release_requirement_and_gate",
        ),
    }
    scope, expected_result, promotion_rule = behavior[kind]
    return {
        "id": expectation_id(kind, statement_hash),
        "kind": kind,
        "statement": statement,
        "source": {
            "document": document,
            "heading": heading,
            "line": line_number,
            "statement_sha256": statement_hash,
        },
        "scope": scope,
        "expected_result": expected_result,
        "promotion_rule": promotion_rule,
        "test_contract_id": "PX-TEST-001",
        "acceptance_tests": unique_references(statement),
        "status": "enforced_policy",
    }


def parse_prd_exclusions(path: Path) -> list[dict[str, object]]:
    """Parse every bullet in the canonical v0.1 Non-Goals section."""
    document = source_name(path)
    heading = ""
    in_non_goals = False
    found_heading = False
    records: list[dict[str, object]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        heading_match = HEADING.match(line)
        if heading_match:
            heading = heading_match.group(1)
            if heading == "4. Non-Goals":
                in_non_goals = True
                found_heading = True
            elif in_non_goals:
                break
            continue
        if in_non_goals and line.startswith("- "):
            statement = line[2:].strip()
            records.append(
                make_expectation(
                    kind="v0.1_exclusion",
                    statement=statement,
                    document=document,
                    heading=heading,
                    line_number=line_number,
                    source_line=line,
                )
            )
    if not found_heading:
        raise PolicyExpectationError("PRD non-goals heading is missing")
    return records


def parse_inventory_expectations(path: Path) -> list[dict[str, object]]:
    """Parse DEFER, ROADMAP, and explicitly rejected-default checklist rows."""
    document = source_name(path)
    heading = ""
    records: list[dict[str, object]] = []
    found_rejected_heading = False
    rejected_rows = 0
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        heading_match = HEADING.match(line)
        if heading_match:
            heading = heading_match.group(1)
            if heading == "35A. Explicitly Rejected Defaults":
                found_rejected_heading = True
            continue

        checklist_match = CHECKLIST.match(line)
        if checklist_match is None:
            continue
        label, statement = checklist_match.groups()

        if heading == "35A. Explicitly Rejected Defaults":
            kind = "rejected_default"
            rejected_rows += 1
        elif label == "DEFER":
            kind = "deferred"
        elif label == "ROADMAP":
            kind = "release_deferred"
        else:
            continue

        records.append(
            make_expectation(
                kind=kind,
                statement=statement,
                document=document,
                heading=heading,
                line_number=line_number,
                source_line=line,
            )
        )

    if not found_rejected_heading or rejected_rows == 0:
        raise PolicyExpectationError("explicitly rejected defaults are missing")
    return records


def build_policy_registry(
    prd_path: Path = DEFAULT_PRD,
    inventory_path: Path = DEFAULT_INVENTORY,
) -> dict[str, object]:
    """Build the deterministic machine-readable policy expectation registry."""
    records = parse_prd_exclusions(prd_path) + parse_inventory_expectations(inventory_path)
    identifiers = [str(record["id"]) for record in records]
    duplicates = sorted(
        identifier for identifier, count in Counter(identifiers).items() if count > 1
    )
    if duplicates:
        raise PolicyExpectationError(
            "duplicate policy expectation identifiers: " + ", ".join(duplicates)
        )

    by_kind = Counter(str(record["kind"]) for record in records)
    actual_counts = {kind: by_kind[kind] for kind in sorted(by_kind)}
    if actual_counts != EXPECTED_COUNTS:
        raise PolicyExpectationError(
            f"canonical policy counts changed: expected {EXPECTED_COUNTS}, "
            f"found {actual_counts}"
        )

    ordered = sorted(records, key=lambda record: str(record["id"]))
    return {
        "schema_version": SCHEMA_VERSION,
        "sources": [
            {
                "document": source_name(path),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
            for path in (prd_path, inventory_path)
        ],
        "test_contract": {
            "id": "PX-TEST-001",
            "probe": "attempt_or_detect_scoped_capability_claim_or_default",
            "allowed_results": [
                "absent",
                "disabled",
                "denied_before_effect",
            ],
            "prohibited_results": [
                "authority_granted",
                "capability_activated",
                "policy_silently_broadened",
                "state_or_external_side_effect",
            ],
            "required_evidence": [
                "source_identity",
                "probe_result",
                "side_effect_assertion",
                "promotion_or_gate_state",
            ],
        },
        "counts": {
            "total": len(ordered),
            "by_kind": actual_counts,
        },
        "expectations": ordered,
    }


def render_policy_registry(registry: dict[str, object]) -> str:
    return json.dumps(registry, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def write_policy_registry(
    prd_path: Path,
    inventory_path: Path,
    output_path: Path,
) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        render_policy_registry(build_policy_registry(prd_path, inventory_path)),
        encoding="utf-8",
    )


def check_policy_registry(
    prd_path: Path,
    inventory_path: Path,
    output_path: Path,
) -> bool:
    expected = render_policy_registry(build_policy_registry(prd_path, inventory_path))
    try:
        actual = output_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"Policy expectation registry is missing: {output_path}", file=sys.stderr)
        return False
    if actual != expected:
        print(
            "Policy expectation registry is stale; run "
            "`python3 scripts/policy_expectations.py`.",
            file=sys.stderr,
        )
        return False
    return True


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--prd", type=Path, default=DEFAULT_PRD)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.check:
            return 0 if check_policy_registry(
                args.prd,
                args.inventory,
                args.output,
            ) else 1
        write_policy_registry(args.prd, args.inventory, args.output)
    except (OSError, UnicodeError, PolicyExpectationError) as error:
        print(f"Policy expectation error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
