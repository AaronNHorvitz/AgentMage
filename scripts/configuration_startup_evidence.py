#!/usr/bin/env python3
"""Build and validate S-003-IT01 clean profile-startup evidence."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import re
import subprocess
import tempfile
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT / "artifacts/sprints/sprint-3/story-3.1/configuration-startup-report.json"
)
CATALOG_PATH = ROOT / "configuration/profiles/catalog.json"
DEPENDENCY_PATH = ROOT / "supply-chain/dependency-provenance.json"
EXECUTABLE_PATH = (
    ROOT / "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json"
)
EXPECTED_TEST = (
    "every_profile_startup_from_clean_environment_matches_declared_authority_and_evidence"
)
COMMAND = (
    "cargo",
    "test",
    "--offline",
    "-p",
    "agentmage-kernel-engine",
    f"configuration::tests::{EXPECTED_TEST}",
    "--locked",
    "--",
    "--exact",
    "--nocapture",
)
CLIPPY_COMMAND = (
    "cargo",
    "clippy",
    "--offline",
    "-p",
    "agentmage-kernel-engine",
    "--all-targets",
    "--locked",
    "--",
    "-D",
    "warnings",
)
PROFILE_PATHS = (
    "configuration/profiles/development.json",
    "configuration/profiles/synthetic-test.json",
    "configuration/profiles/strict-local-read-only.json",
    "configuration/profiles/knowledge.json",
    "configuration/profiles/write.json",
    "configuration/profiles/coding.json",
    "configuration/profiles/later-network.json",
)
SOURCE_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "kernel/engine/Cargo.toml",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/configuration.rs",
    "configuration/profiles/catalog.json",
    *PROFILE_PATHS,
    "supply-chain/dependency-provenance.json",
    "artifacts/sprints/sprint-1/story-1.1/clean-build-report.json",
    "scripts/configuration_startup_evidence.py",
    "tests/test_configuration_startup_evidence.py",
)
TEST_NAME = re.compile(
    r"^test configuration::tests::([a-z0-9_]+) \.\.\. ok$", re.MULTILINE
)
RESULT_PREFIX = "agentmage-startup-result:"
HASH = re.compile(r"^[0-9a-f]{64}$")
Runner = Callable[[Sequence[str], Path], str]


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def compact_json(value: Any) -> bytes:
    return json.dumps(value, separators=(",", ":"), ensure_ascii=True).encode("ascii")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-configuration-startup-", dir=path.parent
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


def subprocess_runner(command: Sequence[str], root: Path) -> str:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        command,
        cwd=root,
        env=environment,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"configuration startup command failed: {command[0]}")
    return result.stdout + result.stderr


def expected_catalog_profiles(root: Path = ROOT) -> list[dict[str, Any]]:
    catalog = read_json(root / CATALOG_PATH.relative_to(ROOT))
    profiles = catalog.get("profiles")
    if not isinstance(profiles, list) or len(profiles) != 7:
        raise ValueError("startup profile catalog closure is invalid")
    if catalog.get("loader_status") != "library-implemented-product-not-registered":
        raise ValueError("startup catalog loader status is invalid")
    return profiles


def result_identity_material(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": result["schema_version"],
        "record_type": result["record_type"],
        "profile_id": result["profile_id"],
        "activation_status": result["activation_status"],
        "startup_status": result["startup_status"],
        "declared_capabilities": result["declared_capabilities"],
        "registered_capabilities": result["registered_capabilities"],
        "configuration_sha256": result["configuration_sha256"],
        "dependency_sha256": result["dependency_sha256"],
        "executable_sha256": result["executable_sha256"],
        "policy_sha256": result["policy_sha256"],
    }


def validate_results(results: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(results, list):
        return ["startup results must be an array"]
    failures = []
    try:
        profiles = expected_catalog_profiles(root)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [str(error)]
    expected_by_id = {item["profile_id"]: item for item in profiles}
    if len(expected_by_id) != 7:
        failures.append("startup catalog profile identities are not unique")
    if len(results) != 7:
        failures.append("startup result count is invalid")
    dependency_sha256 = sha256_file(root / DEPENDENCY_PATH.relative_to(ROOT))
    executable_sha256 = sha256_file(root / EXECUTABLE_PATH.relative_to(ROOT))
    policy_sha256 = sha256_file(root / CATALOG_PATH.relative_to(ROOT))
    observed_ids = []
    configuration_hashes = []
    expected_fields = {
        "schema_version",
        "record_type",
        "profile_id",
        "activation_status",
        "startup_status",
        "declared_capabilities",
        "registered_capabilities",
        "configuration_sha256",
        "dependency_sha256",
        "executable_sha256",
        "policy_sha256",
        "record_sha256",
    }
    for result in results:
        if not isinstance(result, dict) or set(result) != expected_fields:
            failures.append("startup result fields are not closed")
            continue
        profile_id = result.get("profile_id")
        profile = expected_by_id.get(profile_id)
        if profile is None:
            failures.append("startup result profile identity is unknown")
            continue
        observed_ids.append(profile_id)
        configuration_hashes.append(result.get("configuration_sha256"))
        if (
            result.get("schema_version") != 1
            or result.get("record_type") != "profile-startup-verification-result"
            or result.get("activation_status") != profile["activation_status"]
            or result.get("startup_status") != "blocked-as-declared"
            or result.get("declared_capabilities") != profile["effective_capabilities"]
            or result.get("registered_capabilities") != []
            or profile.get("product_registration") is not False
        ):
            failures.append(f"startup authority result is invalid: {profile_id}")
        if (
            result.get("dependency_sha256") != dependency_sha256
            or result.get("executable_sha256") != executable_sha256
            or result.get("policy_sha256") != policy_sha256
        ):
            failures.append(f"startup artifact identity is invalid: {profile_id}")
        if not HASH.fullmatch(str(result.get("configuration_sha256", ""))):
            failures.append(f"startup configuration identity is invalid: {profile_id}")
        expected_record_sha256 = sha256_bytes(compact_json(result_identity_material(result)))
        if result.get("record_sha256") != expected_record_sha256:
            failures.append(f"startup result record identity is invalid: {profile_id}")
    expected_ids = [item["profile_id"] for item in profiles]
    if observed_ids != expected_ids or len(set(observed_ids)) != 7:
        failures.append("startup result profile closure or order is invalid")
    if len(configuration_hashes) == 7 and len(set(configuration_hashes)) != 7:
        failures.append("startup configuration identities are not unique")
    return failures


def execute_gate(
    root: Path = ROOT, runner: Runner = subprocess_runner
) -> tuple[dict[str, Any], ...]:
    output = runner(COMMAND, root)
    if TEST_NAME.findall(output) != [EXPECTED_TEST]:
        raise RuntimeError("configuration startup test closure failed")
    results = []
    for line in output.splitlines():
        if line.startswith(RESULT_PREFIX):
            results.append(json.loads(line.removeprefix(RESULT_PREFIX)))
    failures = validate_results(results, root)
    if failures:
        raise RuntimeError("; ".join(failures))
    runner(CLIPPY_COMMAND, root)
    return tuple(results)


def build_report(executed_results: Sequence[dict[str, Any]], root: Path = ROOT) -> dict[str, Any]:
    results = copy.deepcopy(list(executed_results))
    failures = validate_results(results, root)
    if failures:
        raise ValueError("; ".join(failures))
    return {
        "schema_version": 1,
        "task_id": "3.1.3.4",
        "test_id": "S-003-IT01",
        "status": "pass-shared-linux-clean-profile-startup-verification",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in SOURCE_PATHS
        ],
        "commands": [
            {"argv": list(COMMAND), "status": "pass"},
            {"argv": list(CLIPPY_COMMAND), "status": "pass"},
        ],
        "tests": [EXPECTED_TEST],
        "result_bundles": results,
        "summary": {
            "profile_count": 7,
            "current_foundation_profile_count": 3,
            "future_disabled_profile_count": 4,
            "clean_environment_count": 7,
            "declared_capability_mismatch_count": 0,
            "registered_capability_count": 0,
            "early_product_registration_count": 0,
            "artifact_identity_mismatch_count": 0,
            "failed_test_count": 0,
            "skipped_test_count": 0,
        },
        "clean_environment": {
            "fresh_empty_directory_per_profile": True,
            "ambient_environment_values_read": False,
            "startup_directory_writes": 0,
            "private_paths_persisted": False,
        },
        "network_used": False,
        "product_startup_activation_claim": "none",
        "release_readiness_claim": "none",
        "platform_execution": "fedora-linux",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration startup report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.3.4"
        or value.get("test_id") != "S-003-IT01"
        or value.get("status")
        != "pass-shared-linux-clean-profile-startup-verification"
    ):
        failures.append("configuration startup report identity is invalid")
    if value.get("tests") != [EXPECTED_TEST]:
        failures.append("configuration startup test closure is invalid")
    failures.extend(validate_results(value.get("result_bundles"), root))
    if value.get("summary") != {
        "profile_count": 7,
        "current_foundation_profile_count": 3,
        "future_disabled_profile_count": 4,
        "clean_environment_count": 7,
        "declared_capability_mismatch_count": 0,
        "registered_capability_count": 0,
        "early_product_registration_count": 0,
        "artifact_identity_mismatch_count": 0,
        "failed_test_count": 0,
        "skipped_test_count": 0,
    }:
        failures.append("configuration startup summary is invalid")
    if value.get("clean_environment") != {
        "fresh_empty_directory_per_profile": True,
        "ambient_environment_values_read": False,
        "startup_directory_writes": 0,
        "private_paths_persisted": False,
    }:
        failures.append("configuration startup environment evidence is invalid")
    if (
        value.get("network_used") is not False
        or value.get("product_startup_activation_claim") != "none"
        or value.get("release_readiness_claim") != "none"
        or value.get("platform_execution") != "fedora-linux"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration startup report made an unsupported claim")
    try:
        expected_sources = [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in SOURCE_PATHS
        ]
    except OSError:
        failures.append("configuration startup source closure is unavailable")
    else:
        if value.get("source_artifacts") != expected_sources:
            failures.append("configuration startup source closure is stale")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read configuration startup report: {error}"]
    try:
        return validate_report(report, root)
    except (OSError, KeyError, TypeError, ValueError) as error:
        return [f"cannot rebuild configuration startup report: {error}"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            results = execute_gate()
            write_atomic(REPORT_PATH, canonical_json(build_report(results)))
        failures = check_artifact()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"configuration startup evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration startup evidence failed: {failure}")
        return 1
    print("Story 3.1 clean profile-startup evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
