#!/usr/bin/env python3
"""Generate and validate review artifacts for configuration Story 3.1."""

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
PROFILE_PATH = ROOT / "configuration/profiles/synthetic-test.json"
CATALOG_PATH = ROOT / "configuration/profiles/catalog.json"
DELTA_PATH = ROOT / "configuration/profiles/capability-deltas.json"
MIGRATION_ROOT = ROOT / "fixtures/configuration/migration"
DIFF_FIXTURE_PATH = (
    ROOT / "schemas/configuration/examples/configuration-diff.valid.json"
)
ROLLBACK_FIXTURE_PATH = (
    ROOT / "schemas/configuration/examples/configuration-rollback-report.valid.json"
)
REPORT_PATH = (
    ROOT
    / "artifacts/sprints/sprint-3/story-3.1/configuration-review-artifacts-report.json"
)
MIGRATION_FIXTURES = (
    "fixtures/configuration/migration/v0.valid.json",
    "fixtures/configuration/migration/v1.expected.json",
    "fixtures/configuration/migration/v1.not-migratable.invalid.json",
    "fixtures/configuration/migration/v0.reserved-version.invalid.json",
    "fixtures/configuration/migration/v0.missing-section.invalid.json",
)
OUTPUT_PATHS = (
    *MIGRATION_FIXTURES,
    "configuration/profiles/capability-deltas.json",
    "schemas/configuration/examples/configuration-diff.valid.json",
    "schemas/configuration/examples/configuration-rollback-report.valid.json",
)
SOURCE_PATHS = (
    "schemas/configuration/common.schema.json",
    "schemas/configuration/agent-configuration.schema.json",
    "schemas/configuration/configuration-diff.schema.json",
    "schemas/configuration/configuration-rollback-report.schema.json",
    "configuration/profiles/catalog.json",
    "configuration/profiles/synthetic-test.json",
    "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json",
    "kernel/engine/src/configuration.rs",
    "scripts/configuration_review_artifacts.py",
    "tests/test_configuration_review_artifacts.py",
)
SECTION_NAMES = (
    "core",
    "platform",
    "model",
    "workspace",
    "tool",
    "permission",
    "budget",
    "logging",
    "retention",
    "skill",
    "shell",
)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-configuration-review-", dir=path.parent
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


def migration_outputs(profile: dict[str, Any]) -> dict[str, Any]:
    legacy = copy.deepcopy(profile)
    legacy["schema_version"] = 0
    for section in SECTION_NAMES:
        legacy[section].pop("schema_version")
    legacy["core"]["profile"] = legacy["core"].pop("profile_id")

    reserved = copy.deepcopy(legacy)
    reserved["core"]["schema_version"] = 0
    missing = copy.deepcopy(legacy)
    missing.pop("shell")
    return {
        MIGRATION_FIXTURES[0]: legacy,
        MIGRATION_FIXTURES[1]: profile,
        MIGRATION_FIXTURES[2]: profile,
        MIGRATION_FIXTURES[3]: reserved,
        MIGRATION_FIXTURES[4]: missing,
    }


def profile_delta_output(catalog: dict[str, Any]) -> dict[str, Any]:
    baseline = next(
        item for item in catalog["profiles"] if item["profile_id"] == "strict-local-read-only"
    )
    baseline_effective = set(baseline["effective_capabilities"])
    profiles = []
    for item in catalog["profiles"]:
        effective = set(item["effective_capabilities"])
        planned = set(item["planned_capabilities"])
        profiles.append(
            {
                "profile_id": item["profile_id"],
                "phase": item["phase"],
                "activation_status": item["activation_status"],
                "effective_added_from_baseline": sorted(effective - baseline_effective),
                "effective_removed_from_baseline": sorted(baseline_effective - effective),
                "planned_not_effective": sorted(planned - effective),
                "effective_authority_broadening": bool(effective - baseline_effective),
                "network_effective": item["network_effective"],
                "product_registration": item["product_registration"],
                "blocked_by": item["blocked_by"],
            }
        )
    return {
        "schema_version": 1,
        "record_type": "configuration-profile-capability-deltas",
        "baseline_profile_id": baseline["profile_id"],
        "profiles": profiles,
        "active_product_profile_count": 0,
        "effective_authority_broadening_profile_count": 0,
        "network_effective_profile_count": 0,
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def diff_fixture() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "configuration-diff",
        "before_sha256": "a" * 64,
        "after_sha256": "b" * 64,
        "authority_broadening": True,
        "changes": [
            {
                "path": "/budget/maximum_output_bytes",
                "before_sha256": "c" * 64,
                "after_sha256": "d" * 64,
                "impact": "resource-increase",
            },
            {
                "path": "/permission/allowed_capabilities",
                "before_sha256": "e" * 64,
                "after_sha256": "f" * 64,
                "impact": "authority-broadening",
            },
        ],
    }


def rollback_fixture() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "configuration-rollback-report",
        "operation_id": "synthetic-configuration-rollback",
        "status": "restored",
        "profile_id": "strict-local-read-only",
        "expected_current_sha256": "1" * 64,
        "backup_sha256": "2" * 64,
        "restored_sha256": "3" * 64,
        "preimage_verified": True,
        "backup_retained": True,
        "atomic_replace": True,
        "raw_configuration_persisted": False,
        "private_path_persisted": False,
        "error_code": None,
    }


def build_outputs(root: Path = ROOT) -> dict[str, Any]:
    profile = read_json(root / PROFILE_PATH.relative_to(ROOT))
    catalog = read_json(root / CATALOG_PATH.relative_to(ROOT))
    return {
        **migration_outputs(profile),
        DELTA_PATH.relative_to(ROOT).as_posix(): profile_delta_output(catalog),
        DIFF_FIXTURE_PATH.relative_to(ROOT).as_posix(): diff_fixture(),
        ROLLBACK_FIXTURE_PATH.relative_to(ROOT).as_posix(): rollback_fixture(),
    }


def validate_outputs(root: Path = ROOT) -> list[str]:
    failures = []
    expected = build_outputs(root)
    if tuple(expected) != OUTPUT_PATHS:
        failures.append("configuration review output closure is invalid")
    for relative, value in expected.items():
        try:
            actual = read_json(root / relative)
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read configuration review artifact {relative}: {error}")
            continue
        if actual != value:
            failures.append(f"configuration review artifact is stale: {relative}")
    delta = expected[DELTA_PATH.relative_to(ROOT).as_posix()]
    if (
        delta["active_product_profile_count"] != 0
        or delta["effective_authority_broadening_profile_count"] != 0
        or delta["network_effective_profile_count"] != 0
        or delta["macos_execution_status"] != "blocked-macos"
        or any(
            item["effective_authority_broadening"]
            or item["network_effective"]
            or item["product_registration"]
            for item in delta["profiles"]
        )
    ):
        failures.append("configuration profile delta artifact made an unsupported claim")
    diff = expected[DIFF_FIXTURE_PATH.relative_to(ROOT).as_posix()]
    if [item["path"] for item in diff["changes"]] != sorted(
        item["path"] for item in diff["changes"]
    ):
        failures.append("configuration diff changes are not path sorted")
    serialized_diff = json.dumps(diff, sort_keys=True)
    if "before_value" in serialized_diff or "after_value" in serialized_diff:
        failures.append("configuration diff contains raw changed values")
    rollback = expected[ROLLBACK_FIXTURE_PATH.relative_to(ROOT).as_posix()]
    if (
        rollback["raw_configuration_persisted"]
        or rollback["private_path_persisted"]
        or not rollback["preimage_verified"]
        or not rollback["backup_retained"]
        or not rollback["atomic_replace"]
    ):
        failures.append("configuration rollback report weakened safety or privacy")
    return failures


def build_report(root: Path = ROOT) -> dict[str, Any]:
    failures = validate_outputs(root)
    if failures:
        raise ValueError("; ".join(failures))
    component_inventory = read_json(
        root / "artifacts/sprints/sprint-3/story-3.1/component-inventory-report.json"
    )
    if component_inventory.get("task_id") != "3.1.1.6":
        raise ValueError("dependency and executable inventory is not authoritative")
    return {
        "schema_version": 1,
        "task_ids": ["3.1.2.1", "3.1.2.2", "3.1.2.3", "3.1.2.4"],
        "status": "pass-shared-configuration-review-artifacts",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in SOURCE_PATHS
        ],
        "review_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in OUTPUT_PATHS
        ],
        "summary": {
            "migration_fixture_count": len(MIGRATION_FIXTURES),
            "valid_migration_input_count": 1,
            "expected_migration_output_count": 1,
            "invalid_migration_input_count": 3,
            "profile_delta_count": 7,
            "effective_authority_broadening_profile_count": 0,
            "dependency_and_executable_inventory_bound": True,
            "configuration_report_schema_count": 2,
        },
        "raw_configuration_values_in_diff": False,
        "private_paths_in_rollback_report": False,
        "product_profile_activation_claim": "none",
        "platform_execution_claim": "none",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration review artifact report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_ids")
        != ["3.1.2.1", "3.1.2.2", "3.1.2.3", "3.1.2.4"]
        or value.get("status") != "pass-shared-configuration-review-artifacts"
    ):
        failures.append("configuration review artifact report identity is invalid")
    if (
        value.get("raw_configuration_values_in_diff") is not False
        or value.get("private_paths_in_rollback_report") is not False
        or value.get("product_profile_activation_claim") != "none"
        or value.get("platform_execution_claim") != "none"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration review artifact report made an unsupported claim")
    try:
        expected = build_report(root)
    except (KeyError, OSError, TypeError, ValueError) as error:
        failures.append(f"cannot rebuild configuration review artifact report: {error}")
    else:
        if value != expected:
            failures.append("configuration review artifact report is stale or non-deterministic")
    return failures


def check_artifacts(root: Path = ROOT) -> list[str]:
    try:
        failures = validate_outputs(root)
    except (KeyError, OSError, TypeError, ValueError) as error:
        return [f"cannot validate configuration review artifacts: {error}"]
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        failures.append(f"cannot read configuration review artifact report: {error}")
    else:
        failures.extend(validate_report(report, root))
    return failures


def write_outputs(root: Path = ROOT) -> None:
    for relative, value in build_outputs(root).items():
        write_atomic(root / relative, canonical_json(value))
    write_atomic(root / REPORT_PATH.relative_to(ROOT), canonical_json(build_report(root)))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            write_outputs()
        failures = check_artifacts()
    except (KeyError, OSError, TypeError, ValueError) as error:
        print(f"configuration review artifacts failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration review artifacts failed: {failure}")
        return 1
    print("Story 3.1 configuration review artifacts validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
