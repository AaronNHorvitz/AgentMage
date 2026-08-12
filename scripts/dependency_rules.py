#!/usr/bin/env python3
"""Validate AgentMage's one-way module dependency rules."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

try:
    from scripts.module_inventory import INVENTORY_PATH, load_inventory
except ModuleNotFoundError:
    from module_inventory import INVENTORY_PATH, load_inventory


ROOT = Path(__file__).resolve().parents[1]
RULES_PATH = ROOT / "architecture" / "dependency-rules.json"

EXPECTED_IMPORTS = {
    "kernel-contracts": set(),
    "kernel-engine": {"kernel-contracts"},
    "platform-linux": {"kernel-contracts", "kernel-engine"},
    "platform-macos": {"kernel-contracts", "kernel-engine"},
    "platform-windows": {"kernel-contracts"},
    "capability-read-only": {"kernel-contracts"},
    "shell-host": {
        "capability-read-only",
        "kernel-contracts",
        "kernel-engine",
        "platform-linux",
        "platform-macos",
    },
    "shell-vscode": {"kernel-contracts"},
    "fixture-corpus": set(),
    "packaging-linux": set(),
    "packaging-macos": set(),
    "documentation": set(),
    "release-xtask": set(),
}
EXPECTED_ASSEMBLY_INPUTS = {
    "kernel-contracts": set(),
    "kernel-engine": set(),
    "platform-linux": set(),
    "platform-macos": set(),
    "platform-windows": set(),
    "capability-read-only": set(),
    "shell-host": set(),
    "shell-vscode": set(),
    "fixture-corpus": set(),
    "packaging-linux": {
        "capability-read-only",
        "platform-linux",
        "shell-host",
        "shell-vscode",
    },
    "packaging-macos": {
        "capability-read-only",
        "platform-macos",
        "shell-host",
        "shell-vscode",
    },
    "documentation": set(),
    "release-xtask": {
        "documentation",
        "fixture-corpus",
        "packaging-linux",
        "packaging-macos",
    },
}
EXPECTED_LAYERS = {
    "kernel-contracts": 0,
    "fixture-corpus": 0,
    "documentation": 0,
    "kernel-engine": 1,
    "platform-linux": 2,
    "platform-macos": 2,
    "platform-windows": 2,
    "capability-read-only": 1,
    "shell-host": 3,
    "shell-vscode": 3,
    "packaging-linux": 4,
    "packaging-macos": 4,
    "release-xtask": 5,
}
EXPECTED_CATEGORY_CONSTRAINTS = {
    "kernel": {"capability-packs", "platform-adapters", "shells"},
    "platform-adapters": {"capability-packs", "shells"},
    "capability-packs": {"platform-adapters", "shells"},
}
COMPILE_MODULE_IDS = (
    "kernel-contracts",
    "kernel-engine",
    "platform-linux",
    "platform-macos",
    "platform-windows",
    "capability-read-only",
    "shell-host",
    "shell-vscode",
)


def load_rules(path: Path = RULES_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


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


def _find_cycle(graph: dict[str, set[str]]) -> list[str] | None:
    visited: set[str] = set()
    active: list[str] = []
    active_set: set[str] = set()

    def visit(node: str) -> list[str] | None:
        if node in active_set:
            index = active.index(node)
            return [*active[index:], node]
        if node in visited:
            return None
        active.append(node)
        active_set.add(node)
        for target in sorted(graph.get(node, set())):
            cycle = visit(target)
            if cycle:
                return cycle
        active.pop()
        active_set.remove(node)
        visited.add(node)
        return None

    for node in sorted(graph):
        cycle = visit(node)
        if cycle:
            return cycle
    return None


def validate_rules(rules: Any, inventory: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(rules, dict):
        return ["dependency rules must be an object"]
    if not isinstance(inventory, dict):
        return ["module inventory must be an object"]
    if rules.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if rules.get("decision_id") != "ADR-0004":
        failures.append("decision_id must equal ADR-0004")
    if rules.get("status") != "accepted":
        failures.append("dependency rules status must be accepted")

    modules = _index(inventory.get("modules"), "inventory module", failures)
    policies = _index(rules.get("module_rules"), "module rule", failures)
    expected_ids = set(EXPECTED_IMPORTS)
    if set(modules) != expected_ids or set(policies) != expected_ids:
        failures.append("dependency rules and module inventory must have identical module ids")

    graph: dict[str, set[str]] = {}
    for module_id in sorted(expected_ids):
        policy = policies.get(module_id, {})
        allowed = policy.get("allowed_imports")
        declared = policy.get("declared_imports")
        assembly = policy.get("assembly_inputs")
        if not all(isinstance(value, list) for value in (allowed, declared, assembly)):
            failures.append(f"{module_id} dependency fields must be arrays")
            continue
        allowed_set = set(allowed)
        declared_set = set(declared)
        assembly_set = set(assembly)
        if len(allowed_set) != len(allowed) or len(declared_set) != len(declared):
            failures.append(f"{module_id} contains duplicate import entries")
        if len(assembly_set) != len(assembly):
            failures.append(f"{module_id} contains duplicate assembly inputs")
        if allowed_set != EXPECTED_IMPORTS[module_id]:
            failures.append(f"{module_id} allowed imports do not match the accepted architecture")
        if declared_set != EXPECTED_IMPORTS[module_id]:
            failures.append(f"{module_id} declared imports do not match the accepted architecture")
        for target in sorted(declared_set - allowed_set):
            failures.append(
                f"prohibited import edge: {module_id} -> {target}; "
                f"outside its allowlist"
            )
        if assembly_set != EXPECTED_ASSEMBLY_INPUTS[module_id]:
            failures.append(f"{module_id} assembly inputs do not match the accepted architecture")
        if policy.get("layer") != EXPECTED_LAYERS[module_id]:
            failures.append(f"{module_id} layer does not match the accepted architecture")
        for target in allowed_set | declared_set | assembly_set:
            if target not in expected_ids:
                failures.append(f"{module_id} references unknown module: {target}")
            if target == module_id:
                failures.append(f"{module_id} cannot depend on itself")
        graph[module_id] = declared_set

    constraints: dict[str, set[str]] = {}
    raw_constraints = rules.get("category_constraints")
    if not isinstance(raw_constraints, list):
        failures.append("category_constraints must be an array")
    else:
        for constraint in raw_constraints:
            if not isinstance(constraint, dict):
                failures.append("every category constraint must be an object")
                continue
            source = constraint.get("source_category")
            forbidden = constraint.get("forbidden_target_categories")
            if not isinstance(source, str) or not isinstance(forbidden, list):
                failures.append("category constraints require a source and forbidden array")
                continue
            if source in constraints:
                failures.append(f"duplicate category constraint: {source}")
            constraints[source] = set(forbidden)
    if constraints != EXPECTED_CATEGORY_CONSTRAINTS:
        failures.append("category constraints do not match the accepted architecture")

    for source_id, targets in graph.items():
        source_category = modules.get(source_id, {}).get("category")
        forbidden_categories = constraints.get(source_category, set())
        for target_id in targets:
            target_category = modules.get(target_id, {}).get("category")
            if target_category in forbidden_categories:
                failures.append(
                    f"prohibited reverse edge: {source_id} -> {target_id} "
                    f"({source_category} cannot import {target_category})"
                )

    cycle = _find_cycle(graph)
    if cycle:
        failures.append("compile dependency cycle: " + " -> ".join(cycle))
    return failures


def prohibited_compile_edges() -> list[tuple[str, str]]:
    """Return every source-module edge outside the accepted compile allowlists."""
    return [
        (source, target)
        for source in COMPILE_MODULE_IDS
        for target in COMPILE_MODULE_IDS
        if source != target and target not in EXPECTED_IMPORTS[source]
    ]


def main() -> int:
    try:
        rules = load_rules()
        inventory = load_inventory(INVENTORY_PATH)
    except (OSError, json.JSONDecodeError) as error:
        print(f"dependency rule validation failed: {error}", file=sys.stderr)
        return 1

    failures = validate_rules(rules, inventory)
    if failures:
        for failure in failures:
            print(f"dependency rule validation failed: {failure}", file=sys.stderr)
        return 1

    print("dependency rule validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
