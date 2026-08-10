#!/usr/bin/env python3
"""Build the canonical machine-readable AM/AT/CR requirement registry."""

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
DEFAULT_SOURCE: Final = ROOT / "Agent-Scaffolding-Inventory.md"
DEFAULT_OUTPUT: Final = ROOT / "requirements" / "registry.json"
SCHEMA_VERSION: Final = 2

KIND_BY_PREFIX: Final = {
    "AM": "product_requirement",
    "AT": "acceptance_test",
    "CR": "competitive_requirement",
}
DEFINITION_ROW: Final = re.compile(
    r"^\|\s*`((AM|AT|CR)-[A-Z0-9.-]+)`\s*\|", re.MULTILINE
)
HEADING: Final = re.compile(r"^#{1,6}\s+(.+?)\s*$")
IDENTIFIER_CELL: Final = re.compile(r"^`((AM|AT|CR)-[A-Z0-9.-]+)`$")
REFERENCE_ID: Final = re.compile(r"\b(?:AM|AT|CR)-[A-Z0-9.-]+\b")
EXPECTED_COLUMNS: Final = {"AM": 6, "AT": 3, "CR": 5}
EXPECTED_HEADING: Final = {
    "AM": "Executable v0.1 Backlog",
    "AT": "31B. v0.1 Quantitative Acceptance Matrix",
    "CR": "35. Competitive Review Integration Register",
}


class RegistryError(ValueError):
    """Raised when canonical requirement definitions are invalid."""


def extract_identifiers(markdown: str) -> list[tuple[str, str]]:
    """Return sorted unique identifier/kind pairs from canonical table rows."""
    matches = [
        (identifier, KIND_BY_PREFIX[prefix])
        for identifier, prefix in DEFINITION_ROW.findall(markdown)
    ]
    identifiers = [identifier for identifier, _ in matches]
    duplicates = sorted(identifier for identifier, count in Counter(identifiers).items() if count > 1)
    if duplicates:
        raise RegistryError(f"duplicate requirement definitions: {', '.join(duplicates)}")

    present_prefixes = {identifier.split("-", 1)[0] for identifier in identifiers}
    missing_prefixes = sorted(set(KIND_BY_PREFIX) - present_prefixes)
    if missing_prefixes:
        raise RegistryError(
            "missing canonical requirement categories: " + ", ".join(missing_prefixes)
        )

    return sorted(matches, key=lambda item: item[0])


def split_table_row(line: str) -> list[str]:
    """Split one canonical Markdown table row into trimmed cells."""
    stripped = line.strip()
    if not stripped.startswith("|") or not stripped.endswith("|"):
        raise RegistryError("canonical requirement row must start and end with `|`")
    return [cell.strip() for cell in stripped[1:-1].split("|")]


def referenced_ids(cell: str, prefix: str) -> list[str]:
    """Extract ordered, unique stable references of one prefix from a cell."""
    seen: set[str] = set()
    result: list[str] = []
    for identifier in REFERENCE_ID.findall(cell):
        if identifier.startswith(f"{prefix}-") and identifier not in seen:
            seen.add(identifier)
            result.append(identifier)
    return result


def definition_from_cells(
    cells: list[str],
    *,
    heading: str,
    line_number: int,
    source_document: str,
    source_line: str,
) -> dict[str, object]:
    """Normalize a canonical AM, AT, or CR table row."""
    identifier_match = IDENTIFIER_CELL.fullmatch(cells[0])
    if identifier_match is None:
        raise RegistryError(f"invalid requirement identifier at line {line_number}")
    identifier, prefix = identifier_match.groups()

    expected_columns = EXPECTED_COLUMNS[prefix]
    if len(cells) != expected_columns:
        raise RegistryError(
            f"{identifier} at line {line_number} has {len(cells)} columns; "
            f"expected {expected_columns}"
        )
    if heading != EXPECTED_HEADING[prefix]:
        raise RegistryError(
            f"{identifier} is defined under {heading!r}; expected "
            f"{EXPECTED_HEADING[prefix]!r}"
        )

    if prefix == "AM":
        title = cells[1]
        release = cells[4]
        dependencies = referenced_ids(cells[2], "AM")
        disposition = cells[3].lower()
        acceptance_tests = referenced_ids(cells[5], "AT")
    elif prefix == "AT":
        title = cells[1]
        release = "v0.1"
        dependencies = []
        disposition = "required"
        acceptance_tests = []
    else:
        title = cells[1]
        release = cells[2]
        dependencies = []
        disposition = "integrated"
        acceptance_tests = []

    return {
        "id": identifier,
        "kind": KIND_BY_PREFIX[prefix],
        "title": title,
        "source": {
            "document": source_document,
            "heading": heading,
            "line": line_number,
            "definition_sha256": hashlib.sha256(
                (source_line + "\n").encode("utf-8")
            ).hexdigest(),
        },
        "release": release,
        "dependencies": dependencies,
        "disposition": disposition,
        "acceptance_tests": acceptance_tests,
        "status": "planned",
    }


def parse_definitions(markdown: str, source_document: str) -> list[dict[str, object]]:
    """Parse all canonical requirement definitions with traceability fields."""
    definitions: list[dict[str, object]] = []
    current_heading = ""
    for line_number, line in enumerate(markdown.splitlines(), start=1):
        heading_match = HEADING.match(line)
        if heading_match:
            current_heading = heading_match.group(1)
            continue

        row_match = DEFINITION_ROW.match(line)
        if row_match:
            definitions.append(
                definition_from_cells(
                    split_table_row(line),
                    heading=current_heading,
                    line_number=line_number,
                    source_document=source_document,
                    source_line=line,
                )
            )

    identifiers = [str(definition["id"]) for definition in definitions]
    duplicates = sorted(
        identifier for identifier, count in Counter(identifiers).items() if count > 1
    )
    if duplicates:
        raise RegistryError(f"duplicate requirement definitions: {', '.join(duplicates)}")

    present_prefixes = {identifier.split("-", 1)[0] for identifier in identifiers}
    missing_prefixes = sorted(set(KIND_BY_PREFIX) - present_prefixes)
    if missing_prefixes:
        raise RegistryError(
            "missing canonical requirement categories: " + ", ".join(missing_prefixes)
        )

    return sorted(definitions, key=lambda definition: str(definition["id"]))


def build_registry(source_path: Path = DEFAULT_SOURCE) -> dict[str, object]:
    """Build a deterministic registry object from the canonical inventory."""
    source_bytes = source_path.read_bytes()
    markdown = source_bytes.decode("utf-8")

    try:
        source_name = source_path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        source_name = source_path.name
    definitions = parse_definitions(markdown, source_name)
    by_kind = Counter(str(definition["kind"]) for definition in definitions)

    return {
        "schema_version": SCHEMA_VERSION,
        "source": {
            "document": source_name,
            "sha256": hashlib.sha256(source_bytes).hexdigest(),
        },
        "counts": {
            "total": len(definitions),
            "by_kind": {kind: by_kind[kind] for kind in sorted(by_kind)},
        },
        "requirements": definitions,
    }


def render_registry(registry: dict[str, object]) -> str:
    """Serialize a registry in its canonical byte representation."""
    return json.dumps(registry, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def write_registry(source_path: Path, output_path: Path) -> None:
    """Write the deterministic registry, creating only its parent directory."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(render_registry(build_registry(source_path)), encoding="utf-8")


def check_registry(source_path: Path, output_path: Path) -> bool:
    """Return whether the committed registry matches current canonical input."""
    expected = render_registry(build_registry(source_path))
    try:
        actual = output_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        print(f"Requirement registry is missing: {output_path}", file=sys.stderr)
        return False

    if actual != expected:
        print(
            "Requirement registry is stale; run "
            "`python3 scripts/requirement_registry.py`.",
            file=sys.stderr,
        )
        return False
    return True


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail without writing when the committed registry is stale",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.check:
            return 0 if check_registry(args.source, args.output) else 1
        write_registry(args.source, args.output)
    except (OSError, UnicodeError, RegistryError) as error:
        print(f"Requirement registry error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
