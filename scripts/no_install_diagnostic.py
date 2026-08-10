#!/usr/bin/env python3
"""Report dependency presence without installing, executing, or changing state."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
from pathlib import Path
from typing import Any, Mapping


ROOT = Path(__file__).resolve().parents[1]
INVENTORY_PATH = ROOT / "architecture" / "optional-component-inventory.json"
EXPECTED_COMPONENTS = {
    "linux-bubblewrap": ("bwrap", {"linux"}, "required-runtime-prerequisite"),
    "linux-resource-control": (
        "systemd-run",
        {"linux"},
        "required-runtime-prerequisite",
    ),
    "linux-secret-service-client": (
        "secret-tool",
        {"linux"},
        "required-runtime-prerequisite",
    ),
    "docker-compatibility": (
        "docker",
        {"linux", "macos"},
        "optional-compatibility",
    ),
    "rust-development": ("cargo", {"all"}, "development-only"),
    "node-development": ("node", {"all"}, "development-only"),
    "python-development": ("python3", {"all"}, "development-only"),
    "macos-swift-build": ("swift", {"macos"}, "maintainer-platform-build"),
    "macos-xcode-build": (
        "xcodebuild",
        {"macos"},
        "maintainer-platform-build",
    ),
}
EXPECTED_SIDE_EFFECTS = {
    "executes_external_commands": False,
    "writes_files": False,
    "installs_components": False,
    "uses_network": False,
    "emits_paths": False,
    "emits_environment_values": False,
}
EXPECTED_PLATFORM_STATUS = {
    "macos_implementation": "blocked-macos",
    "macos_verification": "blocked-macos",
}


def load_inventory(path: Path = INVENTORY_PATH) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def normalized_platform(os_name: str) -> str:
    if os_name == "darwin":
        return "macos"
    if os_name == "linux":
        return "linux"
    return "unsupported"


def validate_inventory(inventory: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(inventory, dict):
        return ["optional component inventory must be an object"]
    if inventory.get("schema_version") != 1:
        failures.append("schema_version must equal 1")
    if inventory.get("status") != "enforced":
        failures.append("optional component inventory status must be enforced")
    if inventory.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("diagnostic side-effect contract was weakened")
    if inventory.get("platform_status") != EXPECTED_PLATFORM_STATUS:
        failures.append("diagnostic must retain blocked macOS status")

    raw_components = inventory.get("components")
    if not isinstance(raw_components, list):
        return [*failures, "components must be an array"]
    components: dict[str, dict[str, Any]] = {}
    for component in raw_components:
        if not isinstance(component, dict) or not isinstance(component.get("id"), str):
            failures.append("each component must have a string id")
            continue
        component_id = component["id"]
        if component_id in components:
            failures.append(f"duplicate component id: {component_id}")
        components[component_id] = component
    if set(components) != set(EXPECTED_COMPONENTS):
        failures.append("optional component set does not match the accepted inventory")
    for component_id, (executable, platforms, classification) in EXPECTED_COMPONENTS.items():
        component = components.get(component_id, {})
        actual = (
            component.get("executable"),
            set(component.get("platforms", [])),
            component.get("classification"),
        )
        if actual != (executable, platforms, classification):
            failures.append(f"component metadata drifted: {component_id}")
        if not isinstance(component.get("meaning"), str) or not component.get("meaning"):
            failures.append(f"component meaning is missing: {component_id}")
    return failures


def build_report(
    inventory: dict[str, Any],
    environment: Mapping[str, str],
    os_name: str,
    architecture: str,
) -> dict[str, Any]:
    platform = normalized_platform(os_name)
    search_path = environment.get("PATH", "")
    components = []
    for component in inventory["components"]:
        applicable = "all" in component["platforms"] or platform in component["platforms"]
        if not applicable:
            status = "not-applicable"
        else:
            status = (
                "present-unverified"
                if shutil.which(component["executable"], path=search_path) is not None
                else "missing"
            )
        components.append(
            {
                "id": component["id"],
                "classification": component["classification"],
                "status": status,
                "meaning": component["meaning"],
            }
        )
    missing_optional = sum(
        item["classification"] == "optional-compatibility" and item["status"] == "missing"
        for item in components
    )
    missing_required = sum(
        item["classification"] == "required-runtime-prerequisite"
        and item["status"] == "missing"
        for item in components
    )
    return {
        "schema_version": 1,
        "diagnostic": "agentmage-no-install-dependency-check",
        "platform": {"os": platform, "architecture": architecture},
        "side_effect_contract": inventory["side_effect_contract"],
        "platform_status": inventory["platform_status"],
        "components": components,
        "summary": {
            "missing_required_runtime_prerequisites": missing_required,
            "missing_optional_compatibility_components": missing_optional,
            "changes_made": 0,
            "external_commands_executed": 0,
        },
    }


def validate_report(report: Any, inventory: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if not isinstance(report, dict):
        return ["diagnostic report must be an object"]
    if report.get("schema_version") != 1:
        failures.append("report schema_version must equal 1")
    if report.get("side_effect_contract") != EXPECTED_SIDE_EFFECTS:
        failures.append("report side-effect contract was weakened")
    if report.get("platform_status") != EXPECTED_PLATFORM_STATUS:
        failures.append("report must retain blocked macOS status")
    expected_ids = [item["id"] for item in inventory["components"]]
    actual_ids = [item.get("id") for item in report.get("components", [])]
    if actual_ids != expected_ids:
        failures.append("report component order or closure does not match inventory")
    allowed_status = {"missing", "not-applicable", "present-unverified"}
    for component in report.get("components", []):
        if component.get("status") not in allowed_status:
            failures.append(f"invalid component status: {component.get('id', 'unknown')}")
    summary = report.get("summary", {})
    if summary.get("changes_made") != 0 or summary.get("external_commands_executed") != 0:
        failures.append("diagnostic report claims a prohibited side effect")
    serialized = json.dumps(report, sort_keys=True)
    for value in (os.environ.get("HOME"), os.environ.get("PATH")):
        if value and value in serialized:
            failures.append("diagnostic report exposed an environment value")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        inventory = load_inventory()
    except (OSError, json.JSONDecodeError) as error:
        print(f"no-install diagnostic failed: {error}", file=sys.stderr)
        return 1
    failures = validate_inventory(inventory)
    report = build_report(inventory, os.environ, sys.platform, os.uname().machine)
    failures.extend(validate_report(report, inventory))
    if failures:
        for failure in failures:
            print(f"no-install diagnostic failed: {failure}", file=sys.stderr)
        return 1
    if args.check:
        print("no-install diagnostic contract passed")
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
