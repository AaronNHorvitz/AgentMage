#!/usr/bin/env python3
"""Validate the declared AgentMage repository module skeleton."""

from __future__ import annotations

import json
import sys
from pathlib import Path, PurePosixPath
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
INVENTORY_PATH = ROOT / "architecture" / "module-inventory.json"

EXPECTED_CATEGORIES = {
    "kernel": "kernel",
    "platform-adapters": "platforms",
    "capability-packs": "capabilities",
    "shells": "shells",
    "fixtures": "fixtures",
    "packaging": "packaging",
    "documentation": "docs",
    "release-tooling": "release",
}
EXPECTED_MODULES = {
    "kernel-contracts": ("kernel/contracts", "kernel", "rust", "cargo"),
    "kernel-engine": ("kernel/engine", "kernel", "rust", "cargo"),
    "platform-linux": ("platforms/linux", "platform-adapters", "rust", "cargo"),
    "platform-macos": (
        "platforms/macos",
        "platform-adapters",
        "swift",
        "swiftpm-xcode",
    ),
    "capability-read-only": (
        "capabilities/read-only",
        "capability-packs",
        "rust",
        "cargo",
    ),
    "shell-host": ("shells/host", "shells", "rust", "cargo"),
    "shell-vscode": ("shells/vscode", "shells", "typescript", "npm"),
    "fixture-corpus": ("fixtures/corpus", "fixtures", "data", "none"),
    "packaging-linux": (
        "packaging/linux",
        "packaging",
        "declarative",
        "cargo-xtask",
    ),
    "packaging-macos": (
        "packaging/macos",
        "packaging",
        "declarative",
        "xcode",
    ),
    "documentation": (
        "docs",
        "documentation",
        "markdown",
        "documentation-checks",
    ),
    "release-xtask": ("release/xtask", "release-tooling", "rust", "cargo"),
}


def load_inventory(path: Path = INVENTORY_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _safe_relative_path(raw_path: Any) -> bool:
    if not isinstance(raw_path, str) or not raw_path:
        return False
    path = PurePosixPath(raw_path)
    return not path.is_absolute() and ".." not in path.parts and str(path) == raw_path


def _index(records: Any, label: str, failures: list[str]) -> dict[str, dict[str, Any]]:
    if not isinstance(records, list):
        failures.append(f"{label} records must be an array")
        return {}

    result: dict[str, dict[str, Any]] = {}
    for record in records:
        if not isinstance(record, dict) or not isinstance(record.get("id"), str):
            failures.append(f"every {label} record must have a string id")
            continue
        record_id = record["id"]
        if record_id in result:
            failures.append(f"duplicate {label} id: {record_id}")
        result[record_id] = record
    return result


def validate_inventory(inventory: Any, root: Path = ROOT) -> list[str]:
    failures: list[str] = []
    if not isinstance(inventory, dict):
        return ["module inventory must be an object"]
    if inventory.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if inventory.get("decision_id") != "ADR-0004":
        failures.append("decision_id must equal ADR-0004")
    if inventory.get("status") != "scaffolded":
        failures.append("module inventory status must be scaffolded")

    categories = _index(inventory.get("categories"), "category", failures)
    if set(categories) != set(EXPECTED_CATEGORIES):
        failures.append("category set does not match the repository architecture")
    for category_id, expected_path in EXPECTED_CATEGORIES.items():
        record = categories.get(category_id, {})
        actual_path = record.get("path")
        if actual_path != expected_path:
            failures.append(f"{category_id} path must be {expected_path}")
            continue
        if not _safe_relative_path(actual_path):
            failures.append(f"{category_id} path must be repository-relative and normalized")
        if not (root / actual_path).is_dir():
            failures.append(f"missing category directory: {actual_path}")

    modules = _index(inventory.get("modules"), "module", failures)
    if set(modules) != set(EXPECTED_MODULES):
        failures.append("module set does not match the repository architecture")

    seen_paths: set[str] = set()
    for module_id, expected in EXPECTED_MODULES.items():
        record = modules.get(module_id, {})
        actual = (
            record.get("path"),
            record.get("category"),
            record.get("language"),
            record.get("build_system"),
        )
        if actual != expected:
            failures.append(f"{module_id} metadata does not match the repository architecture")
            continue

        module_path = record["path"]
        if not _safe_relative_path(module_path):
            failures.append(f"{module_id} path must be repository-relative and normalized")
        if module_path in seen_paths:
            failures.append(f"duplicate module path: {module_path}")
        seen_paths.add(module_path)

        category_path = categories.get(record["category"], {}).get("path", "")
        if module_path != category_path and not module_path.startswith(f"{category_path}/"):
            failures.append(f"{module_id} is outside its declared category")
        if not (root / module_path).is_dir():
            failures.append(f"missing module directory: {module_path}")
        if not (root / module_path / "README.md").is_file():
            failures.append(f"missing module marker: {module_path}/README.md")
        if not isinstance(record.get("platforms"), list) or not record.get("platforms"):
            failures.append(f"{module_id} must declare at least one platform")

    return failures


def main() -> int:
    try:
        inventory = load_inventory()
    except (OSError, json.JSONDecodeError) as error:
        print(f"module inventory validation failed: {error}", file=sys.stderr)
        return 1

    failures = validate_inventory(inventory)
    if failures:
        for failure in failures:
            print(f"module inventory validation failed: {failure}", file=sys.stderr)
        return 1

    print("module inventory validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
