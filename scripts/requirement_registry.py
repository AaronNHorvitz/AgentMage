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
SCHEMA_VERSION: Final = 1

KIND_BY_PREFIX: Final = {
    "AM": "product_requirement",
    "AT": "acceptance_test",
    "CR": "competitive_requirement",
}
DEFINITION_ROW: Final = re.compile(
    r"^\|\s*`((AM|AT|CR)-[A-Z0-9.-]+)`\s*\|", re.MULTILINE
)


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


def build_registry(source_path: Path = DEFAULT_SOURCE) -> dict[str, object]:
    """Build a deterministic registry object from the canonical inventory."""
    source_bytes = source_path.read_bytes()
    markdown = source_bytes.decode("utf-8")
    identifiers = extract_identifiers(markdown)
    by_kind = Counter(kind for _, kind in identifiers)

    try:
        source_name = source_path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        source_name = source_path.name

    return {
        "schema_version": SCHEMA_VERSION,
        "source": {
            "document": source_name,
            "sha256": hashlib.sha256(source_bytes).hexdigest(),
        },
        "counts": {
            "total": len(identifiers),
            "by_kind": {kind: by_kind[kind] for kind in sorted(by_kind)},
        },
        "requirements": [
            {"id": identifier, "kind": kind} for identifier, kind in identifiers
        ],
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
