#!/usr/bin/env python3
"""Generate and validate bounded configuration profile definitions."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
BASE_PATH = ROOT / "schemas/configuration/examples/agent-configuration.valid.json"
CATALOG_PATH = ROOT / "configuration/profiles/catalog.json"
REPORT_PATH = (
    ROOT / "artifacts/sprints/sprint-3/story-3.1/profile-catalog-report.json"
)
PROFILE_SPECS = (
    (
        "development",
        "development",
        "current-foundation",
        "inactive-no-product-registration",
        (),
        (),
        ("product-configuration-loader",),
    ),
    (
        "synthetic-test",
        "synthetic-test",
        "current-foundation",
        "inactive-no-product-registration",
        ("workspace.read",),
        ("workspace.read",),
        ("product-configuration-loader",),
    ),
    (
        "strict-local-read-only",
        "strict-local-read-only",
        "current-foundation",
        "inactive-no-product-registration",
        ("workspace.read",),
        ("workspace.read",),
        ("product-configuration-loader", "story-16-read-only-tools"),
    ),
    (
        "knowledge",
        "knowledge",
        "future-v0.2",
        "future-disabled",
        ("workspace.read",),
        ("knowledge.index", "knowledge.search", "workspace.read"),
        ("gate-v0.1", "gate-v0.2", "product-configuration-loader"),
    ),
    (
        "write",
        "write",
        "future-v0.3",
        "future-disabled",
        ("workspace.read",),
        ("workspace.read", "workspace.write"),
        ("gate-v0.2", "gate-v0.3", "product-configuration-loader"),
    ),
    (
        "coding",
        "coding",
        "future-v0.4",
        "future-disabled",
        ("workspace.read",),
        ("process.run", "workspace.read", "workspace.write"),
        ("gate-v0.3", "gate-v0.4", "product-configuration-loader"),
    ),
    (
        "later-network",
        "network-enabled",
        "future-v0.7",
        "future-disabled",
        ("workspace.read",),
        ("network.read", "workspace.read"),
        ("gate-v0.6", "gate-v0.7", "product-configuration-loader"),
    ),
)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-configuration-profile-", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def profile_configuration(
    base: dict[str, Any],
    profile_id: str,
    mode: str,
    effective_capabilities: tuple[str, ...],
) -> dict[str, Any]:
    value = copy.deepcopy(base)
    value["core"]["profile_id"] = profile_id
    value["core"]["configuration_mode"] = mode
    value["core"]["strict_local"] = mode != "network-enabled"
    value["model"]["enabled"] = False
    value["model"]["network_access"] = False
    value["model"]["automatic_routing"] = False
    value["permission"]["allowed_capabilities"] = list(effective_capabilities)
    value["permission"]["network"] = {
        "mode": "deny-all",
        "allowed_endpoints": [],
    }
    value["shell"]["network_access"] = False
    value["shell"]["direct_command_execution"] = False
    value["shell"]["model_to_tool_channel"] = "prohibited"
    value["skill"]["catalog_paths"] = []
    value["skill"]["maximum_enabled_skills"] = 0
    value["skill"]["capability_ceiling"] = []
    if profile_id == "synthetic-test":
        value["tool"]["tools"] = [
            item
            for item in value["tool"]["tools"]
            if set(item["required_capabilities"]).issubset(effective_capabilities)
        ]
    else:
        value["tool"]["tools"] = []
        value["workspace"]["roots"] = [
            {
                "root_id": "user-selected-workspace",
                "locator_kind": "platform-resolved-handle",
                "locator": "unresolved-until-explicit-user-selection",
                "access": "read-only",
                "follow_mount_changes": False,
            }
        ]
    value["logging"]["level"] = "debug" if profile_id == "development" else "info"
    return value


def build_profiles(root: Path = ROOT) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    base = read_json(root / BASE_PATH.relative_to(ROOT))
    configurations = {}
    entries = []
    for (
        profile_id,
        mode,
        phase,
        activation_status,
        effective,
        planned,
        blocked_by,
    ) in PROFILE_SPECS:
        relative = f"configuration/profiles/{profile_id}.json"
        configurations[relative] = profile_configuration(
            base, profile_id, mode, effective
        )
        entries.append(
            {
                "profile_id": profile_id,
                "configuration_path": relative,
                "phase": phase,
                "activation_status": activation_status,
                "configuration_mode": mode,
                "platform_scope": ["fedora-x86_64", "ubuntu-x86_64"],
                "effective_capabilities": list(effective),
                "planned_capabilities": list(planned),
                "blocked_by": list(blocked_by),
                "network_effective": False,
                "product_registration": False,
                "macos_execution_status": "blocked-macos",
                "macos_support_claim": "none",
            }
        )
    catalog = {
        "schema_version": 1,
        "catalog_id": "agentmage-configuration-profile-catalog-v1",
        "catalog_version": "1.0.0",
        "status": "defined-library-loader-no-product-registration",
        "loader_status": "library-implemented-product-not-registered",
        "profiles": entries,
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }
    return catalog, configurations


def validate_catalog(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration profile catalog must be an object"]
    failures = []
    expected, configurations = build_profiles(root)
    if value != expected:
        failures.append("configuration profile catalog is stale or non-deterministic")
    profiles = value.get("profiles", [])
    if [item.get("profile_id") for item in profiles] != [item[0] for item in PROFILE_SPECS]:
        failures.append("configuration profile identity closure is invalid")
    if sum(item.get("phase") == "current-foundation" for item in profiles) != 3:
        failures.append("configuration current-foundation profile count is invalid")
    future = [item for item in profiles if item.get("phase") != "current-foundation"]
    if len(future) != 4 or any(
        item.get("activation_status") != "future-disabled"
        or item.get("product_registration") is not False
        or item.get("network_effective") is not False
        for item in future
    ):
        failures.append("future configuration profiles are not fail-closed")
    for item in profiles:
        relative = item.get("configuration_path")
        path = root / relative if isinstance(relative, str) else root
        if relative not in configurations or not path.is_file():
            failures.append(f"configuration profile file is missing: {relative}")
        elif read_json(path) != configurations[relative]:
            failures.append(f"configuration profile is stale: {relative}")
        configuration = configurations.get(relative, {})
        if configuration.get("core", {}).get("profile_id") != item.get("profile_id"):
            failures.append(f"configuration profile identity mismatch: {relative}")
        if configuration.get("permission", {}).get("allowed_capabilities") != item.get(
            "effective_capabilities"
        ):
            failures.append(f"configuration capability mismatch: {relative}")
        if (
            configuration.get("permission", {}).get("network", {}).get("mode")
            != "deny-all"
            or configuration.get("model", {}).get("network_access") is not False
            or configuration.get("shell", {}).get("network_access") is not False
        ):
            failures.append(f"configuration profile enabled network: {relative}")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    catalog = read_json(root / CATALOG_PATH.relative_to(ROOT))
    failures = validate_catalog(catalog, root)
    if failures:
        raise ValueError("; ".join(failures))
    artifacts = [
        "schemas/configuration/profile-catalog.schema.json",
        "scripts/configuration_profiles.py",
        "tests/test_configuration_profiles.py",
        CATALOG_PATH.relative_to(ROOT).as_posix(),
        *[item["configuration_path"] for item in catalog["profiles"]],
    ]
    future = [
        item for item in catalog["profiles"] if item["phase"] != "current-foundation"
    ]
    return {
        "schema_version": 1,
        "task_id": "3.1.1.2",
        "status": "pass-profile-catalog-future-disabled",
        "artifacts": [
            {"path": path, "sha256": sha256_file(root / path)} for path in artifacts
        ],
        "summary": {
            "profile_count": 7,
            "current_foundation_profile_count": 3,
            "future_disabled_profile_count": 4,
            "active_product_profile_count": 0,
            "product_registered_profile_count": 0,
            "network_effective_profile_count": 0,
            "future_profiles_with_only_baseline_effective_authority": sum(
                set(item["effective_capabilities"]).issubset({"workspace.read"})
                for item in future
            ),
        },
        "profile_ids": [item["profile_id"] for item in catalog["profiles"]],
        "loader_status": "library-implemented-product-not-registered",
        "private_user_data_used": False,
        "network_used": False,
        "product_profile_activation_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration profile report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.1.2"
        or value.get("status") != "pass-profile-catalog-future-disabled"
    ):
        failures.append("configuration profile report identity is invalid")
    if value.get("summary") != {
        "profile_count": 7,
        "current_foundation_profile_count": 3,
        "future_disabled_profile_count": 4,
        "active_product_profile_count": 0,
        "product_registered_profile_count": 0,
        "network_effective_profile_count": 0,
        "future_profiles_with_only_baseline_effective_authority": 4,
    }:
        failures.append("configuration profile report summary is invalid")
    if (
        value.get("private_user_data_used") is not False
        or value.get("network_used") is not False
        or value.get("product_profile_activation_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration profile report made an unsupported claim")
    try:
        expected = build_report(root)
    except (OSError, ValueError, KeyError, TypeError) as error:
        failures.append(f"cannot rebuild configuration profile report: {error}")
    else:
        if value != expected:
            failures.append("configuration profile report is stale or non-deterministic")
    return failures


def write_artifacts(root: Path = ROOT) -> None:
    catalog, configurations = build_profiles(root)
    for relative, configuration in configurations.items():
        write_atomic(root / relative, canonical_json(configuration))
    write_atomic(root / CATALOG_PATH.relative_to(ROOT), canonical_json(catalog))
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def check_artifacts(root: Path = ROOT) -> list[str]:
    failures = []
    try:
        catalog = read_json(root / CATALOG_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read configuration profile catalog: {error}")
    else:
        failures.extend(validate_catalog(catalog, root))
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read configuration profile report: {error}")
    else:
        failures.extend(validate_report(report, root))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_artifacts()
        failures = check_artifacts()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"configuration profiles failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration profiles failed: {failure}")
        return 1
    print("Story 3.1 configuration profiles validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
