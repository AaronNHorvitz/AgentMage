#!/usr/bin/env python3
"""Validate single-owner runtime ownership across the AgentMage module layers.

Decision 0042 forbids a second context manager, operational store, runtime loop,
policy engine, tool dispatcher, or completion verifier. This validator makes that
checkable: each role names exactly one owner, only the kernel layer may own one, and
every implemented module belongs to exactly one layer, so a duplicate owner cannot be
introduced by adding a module, a layer, or a second claim on a role.

Ownership is cross-checked against `architecture/module-inventory.json` so the record
cannot drift into naming modules that do not exist.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OWNERSHIP_PATH: Final = ROOT / "architecture" / "runtime-ownership.json"
INVENTORY_PATH: Final = ROOT / "architecture" / "module-inventory.json"

# The closed set of runtime roles that may exist exactly once in the product.
SINGLETON_ROLES: Final = [
    "context-manager",
    "operational-store",
    "runtime-loop",
    "policy-engine",
    "tool-dispatcher",
    "completion-verifier",
]
EXPECTED_LAYERS: Final = [
    "kernel",
    "capability",
    "platform",
    "shell",
    "parser",
    "mcp",
]
# Only the kernel may own a singleton runtime role.
LAYERS_PERMITTED_TO_OWN: Final = {"kernel"}
LAYER_KEYS: Final = {
    "id",
    "modules",
    "languages",
    "may_own_singleton_roles",
    "implemented",
}


def load_ownership(path: Path = OWNERSHIP_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_inventory(path: Path = INVENTORY_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _validate_layers(
    record: Any, inventory_modules: dict[str, Any], failures: list[str]
) -> dict[str, Any]:
    layers = record.get("layers")
    if not isinstance(layers, list):
        failures.append("layers must be an array")
        return {}

    indexed: dict[str, Any] = {}
    module_to_layers: dict[str, list[str]] = {}
    for layer in layers:
        if not isinstance(layer, dict) or not isinstance(layer.get("id"), str):
            failures.append("every layer must have a string id")
            continue
        layer_id = layer["id"]
        if layer_id in indexed:
            failures.append(f"duplicate layer id: {layer_id}")
        indexed[layer_id] = layer

        unknown = set(layer) - LAYER_KEYS
        if unknown:
            failures.append(f"layer {layer_id} has unknown keys: " + ", ".join(sorted(unknown)))

        expected_permitted = layer_id in LAYERS_PERMITTED_TO_OWN
        if layer.get("may_own_singleton_roles") is not expected_permitted:
            failures.append(
                f"layer {layer_id} may_own_singleton_roles must be {expected_permitted}"
            )

        modules = layer.get("modules")
        if not isinstance(modules, list):
            failures.append(f"layer {layer_id} modules must be an array")
            continue
        if sorted(modules) != list(modules):
            failures.append(f"layer {layer_id} modules must be sorted")
        for module_id in modules:
            if module_id not in inventory_modules:
                failures.append(f"layer {layer_id} names unknown module {module_id}")
                continue
            module_to_layers.setdefault(module_id, []).append(layer_id)

        # An unimplemented layer must be honest about owning nothing yet.
        if layer.get("implemented") is False and modules:
            failures.append(f"unimplemented layer {layer_id} must declare no modules")

        languages = layer.get("languages")
        expected_languages = sorted(
            {
                inventory_modules[module_id]["language"]
                for module_id in modules
                if module_id in inventory_modules
            }
        )
        if languages != expected_languages:
            failures.append(
                f"layer {layer_id} languages must equal its modules' languages: "
                + ", ".join(expected_languages)
            )

    if list(indexed) != EXPECTED_LAYERS:
        failures.append(
            "layers must record exactly " + ", ".join(EXPECTED_LAYERS) + " in order"
        )

    # A module in two layers would give the product two owners for the same code.
    for module_id, owning_layers in sorted(module_to_layers.items()):
        runtime_layers = [
            layer_id for layer_id in owning_layers if layer_id != "parser"
        ]
        if len(runtime_layers) > 1:
            failures.append(
                f"module {module_id} is claimed by more than one layer: "
                + ", ".join(sorted(runtime_layers))
            )

    # Every inventory module must be placed or explicitly set aside.
    unassigned = record.get("unassigned_modules")
    if not isinstance(unassigned, list):
        failures.append("unassigned_modules must be an array")
    else:
        if sorted(unassigned) != list(unassigned):
            failures.append("unassigned_modules must be sorted")
        placed = set(module_to_layers) | set(unassigned)
        missing = set(inventory_modules) - placed
        if missing:
            failures.append(
                "modules missing an ownership decision: " + ", ".join(sorted(missing))
            )
        overlap = set(module_to_layers) & set(unassigned)
        if overlap:
            failures.append(
                "modules both owned and unassigned: " + ", ".join(sorted(overlap))
            )
    return indexed


def _validate_roles(
    record: Any,
    layers: dict[str, Any],
    inventory_modules: dict[str, Any],
    failures: list[str],
) -> None:
    roles = record.get("singleton_roles")
    if not isinstance(roles, list):
        failures.append("singleton_roles must be an array")
        return

    module_layer: dict[str, str] = {}
    for layer_id, layer in layers.items():
        for module_id in layer.get("modules", []) or []:
            module_layer.setdefault(module_id, layer_id)

    seen: list[str] = []
    for role in roles:
        if not isinstance(role, dict) or not isinstance(role.get("id"), str):
            failures.append("every singleton role must have a string id")
            continue
        role_id = role["id"]
        if role_id in seen:
            failures.append(f"duplicate singleton role: {role_id}")
        seen.append(role_id)

        owners = role.get("owners")
        if not isinstance(owners, list):
            failures.append(f"role {role_id} owners must be an array")
            continue
        if len(owners) != 1:
            failures.append(
                f"role {role_id} must have exactly one owner, found {len(owners)}"
            )
        for owner in owners:
            if owner not in inventory_modules:
                failures.append(f"role {role_id} names unknown owner {owner}")
                continue
            owner_layer = module_layer.get(owner)
            if owner_layer is None:
                failures.append(f"role {role_id} owner {owner} belongs to no layer")
            elif owner_layer not in LAYERS_PERMITTED_TO_OWN:
                failures.append(
                    f"role {role_id} cannot be owned by the {owner_layer} layer"
                )

    if seen != SINGLETON_ROLES:
        failures.append(
            "singleton_roles must record exactly " + ", ".join(SINGLETON_ROLES) + " in order"
        )


def validate_ownership(record: Any, inventory: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(record, dict):
        return ["runtime ownership must be an object"]

    if record.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if record.get("decision_id") != "ADR-0042":
        failures.append("decision_id must equal ADR-0042")
    if record.get("status") != "accepted":
        failures.append("status must equal accepted")

    inventory_modules: dict[str, Any] = {}
    if isinstance(inventory, dict) and isinstance(inventory.get("modules"), list):
        for module in inventory["modules"]:
            if isinstance(module, dict) and isinstance(module.get("id"), str):
                inventory_modules[module["id"]] = module
    else:
        failures.append("module inventory must provide modules")

    layers = _validate_layers(record, inventory_modules, failures)
    _validate_roles(record, layers, inventory_modules, failures)
    return failures


def main() -> int:
    try:
        record = load_ownership()
        inventory = load_inventory()
    except (OSError, json.JSONDecodeError) as error:
        print(f"runtime ownership validation failed: {error}", file=sys.stderr)
        return 1

    failures = validate_ownership(record, inventory)
    if failures:
        for failure in failures:
            print(f"runtime ownership validation failed: {failure}", file=sys.stderr)
        return 1

    print("runtime ownership validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
