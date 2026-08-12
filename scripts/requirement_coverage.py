#!/usr/bin/env python3
"""Audit AgentMage requirement references and normative PRD coverage."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes  # type: ignore[no-redef]


ROOT: Final = Path(__file__).resolve().parents[1]
DEFAULT_REGISTRY: Final = ROOT / "requirements" / "registry.json"
DEFAULT_NORMATIVE_MAP: Final = ROOT / "requirements" / "normative-map.json"
HEADING: Final = re.compile(r"^(#{1,6})\s+(.+?)\s*$")
NORMATIVE_TERM: Final = re.compile(
    r"\b(?:must(?:\s+not)?|cannot|never|required)\b|\bfails?\s+closed\b",
    re.IGNORECASE,
)


@dataclass(frozen=True, order=True)
class Diagnostic:
    """One deterministic requirement-coverage failure."""

    category: str
    location: str
    message: str


def load_json(path: Path) -> dict[str, object]:
    """Load one UTF-8 JSON object."""
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected a JSON object")
    return value


def normative_statements(
    root: Path,
    normative_map: dict[str, object],
) -> list[dict[str, object]]:
    """Return explicit normative lines inside the configured PRD scope."""
    source = normative_map.get("source")
    if not isinstance(source, dict):
        raise ValueError("normative map source must be an object")

    document = source.get("document")
    start_heading = source.get("start_heading")
    end_heading = source.get("end_heading")
    if not all(isinstance(value, str) and value for value in (
        document,
        start_heading,
        end_heading,
    )):
        raise ValueError("normative map source fields must be non-empty strings")

    path = root / document
    statements: list[dict[str, object]] = []
    current_heading = ""
    in_scope = False
    in_fence = False
    found_start = False
    found_end = False

    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if line.startswith("```"):
            in_fence = not in_fence
            continue

        heading_match = HEADING.match(line)
        if heading_match and not in_fence:
            current_heading = heading_match.group(2)
            if current_heading == start_heading:
                in_scope = True
                found_start = True
            elif current_heading == end_heading:
                in_scope = False
                found_end = True
                break
            continue

        if (
            in_scope
            and not in_fence
            and not line.lstrip().startswith("|")
            and NORMATIVE_TERM.search(line)
        ):
            statements.append(
                {
                    "document": document,
                    "heading": current_heading,
                    "line": line_number,
                    "statement_sha256": hashlib.sha256(
                        (line + "\n").encode("utf-8")
                    ).hexdigest(),
                }
            )

    if not found_start:
        raise ValueError(f"normative start heading not found: {start_heading}")
    if not found_end:
        raise ValueError(f"normative end heading not found: {end_heading}")
    return statements


def add(
    diagnostics: list[Diagnostic],
    category: str,
    location: str,
    message: str,
) -> None:
    diagnostics.append(Diagnostic(category, location, message))


def audit_registry(
    registry: dict[str, object],
    diagnostics: list[Diagnostic],
    root: Path = ROOT,
) -> dict[str, dict[str, object]]:
    """Audit duplicate IDs, references, tests, and release compatibility."""
    if registry.get("schema_version") != 2:
        add(
            diagnostics,
            "malformed_registry",
            "schema_version",
            "expected supported schema version 2",
        )
    requirements = registry.get("requirements")
    if not isinstance(requirements, list):
        add(diagnostics, "malformed_registry", "requirements", "expected an array")
        return {}

    records = [record for record in requirements if isinstance(record, dict)]
    ids = [record.get("id") for record in records if isinstance(record.get("id"), str)]
    if ids != sorted(ids):
        add(
            diagnostics,
            "noncanonical_order",
            "requirements",
            "requirement records must be ordered by stable identifier",
        )
    for identifier, count in sorted(Counter(ids).items()):
        if count > 1:
            add(
                diagnostics,
                "duplicate_identifier",
                identifier,
                f"identifier is defined {count} times",
            )

    by_id: dict[str, dict[str, object]] = {}
    for record in records:
        identifier = record.get("id")
        if isinstance(identifier, str) and identifier not in by_id:
            by_id[identifier] = record

    referenced_tests: set[str] = set()
    for record in records:
        identifier = record.get("id")
        if not isinstance(identifier, str):
            add(
                diagnostics,
                "malformed_registry",
                "requirements",
                "record has no string identifier",
            )
            continue

        dependencies = record.get("dependencies")
        acceptance_tests = record.get("acceptance_tests")
        if not isinstance(dependencies, list) or not isinstance(acceptance_tests, list):
            add(
                diagnostics,
                "malformed_registry",
                identifier,
                "dependencies and acceptance_tests must be arrays",
            )
            continue

        for dependency_id in dependencies:
            dependency = by_id.get(dependency_id)
            if dependency is None or dependency.get("kind") != "product_requirement":
                add(
                    diagnostics,
                    "unresolved_dependency",
                    identifier,
                    f"dependency does not resolve to a product requirement: {dependency_id}",
                )
            elif dependency.get("release") != record.get("release"):
                add(
                    diagnostics,
                    "release_mismatch",
                    identifier,
                    f"dependency {dependency_id} has release {dependency.get('release')!r}, "
                    f"expected {record.get('release')!r}",
                )

        if record.get("kind") == "product_requirement" and not acceptance_tests:
            add(
                diagnostics,
                "missing_acceptance_test",
                identifier,
                "product requirement has no acceptance test",
            )

        for test_id in acceptance_tests:
            referenced_tests.add(str(test_id))
            test_record = by_id.get(test_id)
            if test_record is None or test_record.get("kind") != "acceptance_test":
                add(
                    diagnostics,
                    "missing_acceptance_test",
                    identifier,
                    f"acceptance-test reference does not resolve: {test_id}",
                )
            elif test_record.get("release") != record.get("release"):
                add(
                    diagnostics,
                    "release_mismatch",
                    identifier,
                    f"acceptance test {test_id} has release {test_record.get('release')!r}, "
                    f"expected {record.get('release')!r}",
                )

    for identifier, record in sorted(by_id.items()):
        if record.get("kind") == "acceptance_test" and identifier not in referenced_tests:
            add(
                diagnostics,
                "missing_acceptance_test",
                identifier,
                "acceptance test is not assigned to a product requirement",
            )

    audit_source_anchors(root, registry, records, diagnostics)

    return by_id


def audit_source_anchors(
    root: Path,
    registry: dict[str, object],
    records: list[dict[str, object]],
    diagnostics: list[Diagnostic],
) -> None:
    """Verify that registry source lines and hashes resolve exactly."""
    source = registry.get("source")
    if not isinstance(source, dict):
        add(diagnostics, "malformed_registry", "source", "expected an object")
        return
    document = source.get("document")
    expected_document_hash = source.get("sha256")
    if not isinstance(document, str) or not document:
        add(
            diagnostics,
            "malformed_registry",
            "source.document",
            "expected a non-empty document path",
        )
        return
    path = root / document
    try:
        source_bytes = path.read_bytes()
        text = source_bytes.decode("utf-8")
    except (OSError, UnicodeError) as error:
        add(
            diagnostics,
            "stale_source_anchor",
            document,
            f"canonical source cannot be read: {error}",
        )
        return
    actual_document_hash = hashlib.sha256(source_bytes).hexdigest()
    if expected_document_hash != actual_document_hash:
        add(
            diagnostics,
            "stale_source_anchor",
            document,
            "canonical source hash does not match the registry",
        )

    lines = text.splitlines()
    headings: list[str] = []
    current_heading = ""
    for line in lines:
        heading_match = HEADING.match(line)
        if heading_match:
            current_heading = heading_match.group(2)
        headings.append(current_heading)

    for record in records:
        identifier = record.get("id")
        location = str(identifier) if isinstance(identifier, str) else "requirements"
        anchor = record.get("source")
        if not isinstance(anchor, dict):
            add(
                diagnostics,
                "malformed_registry",
                location,
                "source anchor must be an object",
            )
            continue
        if anchor.get("document") != document:
            add(
                diagnostics,
                "stale_source_anchor",
                location,
                "source document differs from the registry authority",
            )
        line_number = anchor.get("line")
        if (
            isinstance(line_number, bool)
            or not isinstance(line_number, int)
            or not 1 <= line_number <= len(lines)
        ):
            add(
                diagnostics,
                "stale_source_anchor",
                location,
                "source line is outside the canonical document",
            )
            continue
        source_line = lines[line_number - 1]
        actual_hash = hashlib.sha256((source_line + "\n").encode("utf-8")).hexdigest()
        if anchor.get("definition_sha256") != actual_hash:
            add(
                diagnostics,
                "stale_source_anchor",
                location,
                "definition hash does not match the anchored source line",
            )
        if anchor.get("heading") != headings[line_number - 1]:
            add(
                diagnostics,
                "stale_source_anchor",
                location,
                "source heading does not match the anchored source line",
            )
        if isinstance(identifier, str) and not source_line.startswith(f"| `{identifier}` |"):
            add(
                diagnostics,
                "stale_source_anchor",
                location,
                "anchored source line does not define the requirement identifier",
            )


def audit_normative_map(
    root: Path,
    normative_map: dict[str, object],
    by_id: dict[str, dict[str, object]],
    diagnostics: list[Diagnostic],
) -> int:
    """Audit exact PRD statement mappings to release-compatible AM records."""
    statements = normative_statements(root, normative_map)
    statement_by_hash = {
        str(statement["statement_sha256"]): statement for statement in statements
    }
    mappings = normative_map.get("mappings")
    if not isinstance(mappings, list):
        add(diagnostics, "malformed_normative_map", "mappings", "expected an array")
        return len(statements)

    mapping_hashes = [
        mapping.get("statement_sha256")
        for mapping in mappings
        if isinstance(mapping, dict) and isinstance(mapping.get("statement_sha256"), str)
    ]
    for statement_hash, count in sorted(Counter(mapping_hashes).items()):
        if count > 1:
            add(
                diagnostics,
                "duplicate_normative_mapping",
                statement_hash,
                f"statement hash is mapped {count} times",
            )

    source = normative_map.get("source", {})
    expected_release = source.get("release") if isinstance(source, dict) else None
    mapped_hashes: set[str] = set()
    for mapping in mappings:
        if not isinstance(mapping, dict):
            add(
                diagnostics,
                "malformed_normative_map",
                "mappings",
                "mapping must be an object",
            )
            continue
        statement_hash = mapping.get("statement_sha256")
        if not isinstance(statement_hash, str):
            add(
                diagnostics,
                "malformed_normative_map",
                "mappings",
                "mapping has no statement_sha256",
            )
            continue
        mapped_hashes.add(statement_hash)
        statement = statement_by_hash.get(statement_hash)
        if statement is None:
            add(
                diagnostics,
                "stale_normative_mapping",
                statement_hash,
                "mapped statement is absent from the configured normative scope",
            )
            continue

        location = f"{statement['document']}:{statement['line']}"
        for field in ("heading", "line"):
            if mapping.get(field) != statement.get(field):
                add(
                    diagnostics,
                    "stale_normative_mapping",
                    location,
                    f"mapped {field} does not match the current source",
                )

        requirement_ids = mapping.get("requirement_ids")
        if not isinstance(requirement_ids, list) or not requirement_ids:
            add(
                diagnostics,
                "unmapped_normative_statement",
                location,
                "normative statement has no product-requirement mapping",
            )
            continue
        if len(requirement_ids) != len(set(requirement_ids)):
            add(
                diagnostics,
                "duplicate_normative_mapping",
                location,
                "requirement_ids contains duplicates",
            )

        for requirement_id in requirement_ids:
            requirement = by_id.get(requirement_id)
            if requirement is None or requirement.get("kind") != "product_requirement":
                add(
                    diagnostics,
                    "unresolved_normative_reference",
                    location,
                    f"mapping does not resolve to a product requirement: {requirement_id}",
                )
            elif requirement.get("release") != expected_release:
                add(
                    diagnostics,
                    "release_mismatch",
                    location,
                    f"mapped requirement {requirement_id} has release "
                    f"{requirement.get('release')!r}, expected {expected_release!r}",
                )

    for statement_hash, statement in sorted(statement_by_hash.items()):
        if statement_hash not in mapped_hashes:
            add(
                diagnostics,
                "unmapped_normative_statement",
                f"{statement['document']}:{statement['line']}",
                f"normative statement under {statement['heading']!r} is not mapped",
            )

    return len(statements)


def build_coverage_report(
    registry: dict[str, object],
    normative_map: dict[str, object],
    root: Path = ROOT,
) -> dict[str, object]:
    """Build one deterministic, side-effect-free coverage report."""
    diagnostics: list[Diagnostic] = []
    by_id = audit_registry(registry, diagnostics, root)
    normative_count = audit_normative_map(
        root,
        normative_map,
        by_id,
        diagnostics,
    )
    ordered = sorted(set(diagnostics))
    by_category = Counter(diagnostic.category for diagnostic in ordered)
    return {
        "schema_version": 1,
        "ok": not ordered,
        "counts": {
            "registry_requirements": len(registry.get("requirements", [])),
            "normative_statements": normative_count,
            "normative_mappings": len(normative_map.get("mappings", [])),
            "diagnostics": len(ordered),
            "by_category": {
                category: by_category[category] for category in sorted(by_category)
            },
        },
        "diagnostics": [asdict(diagnostic) for diagnostic in ordered],
    }


def rebase_normative_map(
    root: Path, normative_map: dict[str, object]
) -> dict[str, object]:
    """Refresh only heading and line metadata for unchanged mapped statements."""

    statements = normative_statements(root, normative_map)
    by_hash = {statement["statement_sha256"]: statement for statement in statements}
    mappings = normative_map.get("mappings")
    if not isinstance(mappings, list):
        raise ValueError("normative map mappings must be an array")
    rebased: list[dict[str, object]] = []
    for mapping in mappings:
        if not isinstance(mapping, dict):
            raise ValueError("normative map mapping must be an object")
        statement = by_hash.get(mapping.get("statement_sha256"))
        if statement is None:
            raise ValueError("cannot rebase a changed or missing normative statement")
        rebased.append(
            {
                **mapping,
                "heading": statement["heading"],
                "line": statement["line"],
            }
        )
    return {**normative_map, "mappings": rebased}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    parser.add_argument("--normative-map", type=Path, default=DEFAULT_NORMATIVE_MAP)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--json", action="store_true", help="emit the complete JSON report")
    parser.add_argument(
        "--update-map-lines",
        action="store_true",
        help="refresh locations only when every statement hash is unchanged",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.update_map_lines:
            current_map = load_json(args.normative_map)
            atomic_write(
                args.normative_map,
                canonical_json_bytes(
                    rebase_normative_map(args.root, current_map)
                ),
            )
        report = build_coverage_report(
            load_json(args.registry),
            load_json(args.normative_map),
            args.root,
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        print(f"Requirement coverage error: {error}", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    elif report["ok"]:
        counts = report["counts"]
        print(
            f"Validated {counts['registry_requirements']} requirement records and "
            f"{counts['normative_statements']} normative statement mappings."
        )
    else:
        print("Requirement coverage failed:", file=sys.stderr)
        for diagnostic in report["diagnostics"]:
            print(
                f"- [{diagnostic['category']}] {diagnostic['location']}: "
                f"{diagnostic['message']}",
                file=sys.stderr,
            )
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
