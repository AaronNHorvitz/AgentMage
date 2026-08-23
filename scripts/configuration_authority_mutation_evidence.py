#!/usr/bin/env python3
"""Build and validate S-003-UT02 signed-parent mutation evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from collections import Counter
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
REPORT_PATH = (
    ROOT
    / "artifacts/sprints/sprint-3/story-3.1/configuration-authority-mutation-report.json"
)
REGISTRY_PATH = ROOT / "configuration/permission-bearing-values.json"
EXPECTED_TESTS = (
    "every_permission_bearing_value_is_rejected_through_every_untrusted_channel",
    "parent_profile_signature_verification_rejects_tampering_and_wrong_keys",
)
SOURCES = (
    "configuration-file",
    "environment",
    "repository",
    "child-profile",
    "model-output",
)
EXPECTED_PATH_COUNT = 97
EXPECTED_DIMENSION_COUNT = 67
EXPECTED_REJECTIONS = {
    "configuration-authority-broadening": 59,
    "configuration-contract-violation": 38,
}
EXPECTED_PLANNED_ENTRY_POLICY = {
    "activation": "requires-versioned-schema-implementation-and-evidence",
    "audit_required": True,
    "credential_values_prohibited": True,
    "default_effect": "deny",
    "exact_bound_required": True,
    "expiry_required": True,
}
EXPECTED_PLANNED_ENTRIES = (
    ("/permission/inference_egress", "inference-egress"),
    ("/permission/remote_endpoint", "remote-endpoint"),
    ("/permission/managed_endpoint", "managed-endpoint"),
    ("/permission/context_disclosure", "context-disclosure"),
    ("/permission/persistent_execution", "persistent-execution"),
    ("/permission/unattended_execution", "unattended-execution"),
    ("/permission/multi_agent_fanout", "multi-agent-fanout"),
    ("/permission/model_route", "model-route"),
    ("/permission/endpoint_credential", "endpoint-credential-reference"),
    ("/permission/cost_budget", "cost-budget"),
)
COMMAND_PREFIX = (
    "cargo",
    "test",
    "--offline",
    "-p",
    "agentmage-kernel-engine",
    "--locked",
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
SOURCE_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "kernel/engine/Cargo.toml",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/configuration.rs",
    "configuration/permission-bearing-values.json",
    "configuration/profiles/strict-local-read-only.json",
    "configuration/profiles/synthetic-test.json",
    "scripts/configuration_authority_mutation_evidence.py",
    "tests/test_configuration_authority_mutation_evidence.py",
)
TEST_NAME = re.compile(
    r"^test configuration::tests::([a-z0-9_]+) \.\.\. ok$", re.MULTILINE
)
Runner = Callable[[Sequence[str], Path], str]


def test_command(name: str) -> tuple[str, ...]:
    return (*COMMAND_PREFIX, f"configuration::tests::{name}", "--", "--exact")


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-configuration-authority-mutation-", dir=path.parent
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


def validate_registry(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["permission-bearing registry must be an object"]
    failures = []
    if set(value) != {
        "schema_version",
        "status",
        "planned_entries",
        "planned_entry_policy",
        "entries",
        "excluded_non_authority_fields",
        "exclusion_rationale",
    }:
        failures.append("permission-bearing registry fields are not closed")
    if value.get("schema_version") != 1 or value.get("status") != "enforced-test-closure":
        failures.append("permission-bearing registry identity is invalid")
    planned_entries = value.get("planned_entries")
    if not isinstance(planned_entries, list):
        failures.append("planned permission-bearing entries must be an array")
    else:
        observed_planned_entries = []
        for entry in planned_entries:
            if not isinstance(entry, dict) or set(entry) != {
                "path",
                "dimension",
                "status",
                "expected_rejection",
            }:
                failures.append("planned permission-bearing entry fields are not closed")
                continue
            if entry.get("status") != "planned-not-active":
                failures.append("planned permission-bearing entries must remain inactive")
            if entry.get("expected_rejection") != "configuration-authority-broadening":
                failures.append("planned permission-bearing entries must fail closed")
            observed_planned_entries.append(
                (entry.get("path"), entry.get("dimension"))
            )
        if tuple(observed_planned_entries) != EXPECTED_PLANNED_ENTRIES:
            failures.append("planned permission-bearing entry closure is invalid")
    if value.get("planned_entry_policy") != EXPECTED_PLANNED_ENTRY_POLICY:
        failures.append("planned permission-bearing policy is invalid")
    entries = value.get("entries")
    if not isinstance(entries, list):
        return [*failures, "permission-bearing entries must be an array"]
    paths = []
    dimensions = []
    rejections = Counter()
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {
            "path",
            "dimension",
            "expected_rejection",
        }:
            failures.append("permission-bearing entry fields are not closed")
            continue
        path = entry.get("path")
        dimension = entry.get("dimension")
        rejection = entry.get("expected_rejection")
        if not isinstance(path, str) or not path.startswith("/") or len(path) > 160:
            failures.append("permission-bearing path is invalid")
        else:
            paths.append(path)
        if not isinstance(dimension, str) or not dimension:
            failures.append("permission-bearing dimension is invalid")
        else:
            dimensions.append(dimension)
        if rejection not in EXPECTED_REJECTIONS:
            failures.append("permission-bearing rejection class is invalid")
        else:
            rejections[rejection] += 1
    if len(paths) != EXPECTED_PATH_COUNT or len(set(paths)) != EXPECTED_PATH_COUNT:
        failures.append("permission-bearing path closure is invalid")
    if len(set(dimensions)) != EXPECTED_DIMENSION_COUNT:
        failures.append("permission-bearing dimension closure is invalid")
    if dict(rejections) != EXPECTED_REJECTIONS:
        failures.append("permission-bearing rejection closure is invalid")
    exclusions = value.get("excluded_non_authority_fields")
    if exclusions != [
        "/schema_version",
        "/*/schema_version",
        "/core/product_id",
        "/core/profile_id",
    ]:
        failures.append("non-authority exclusion closure is invalid")
    if not isinstance(value.get("exclusion_rationale"), str) or not value[
        "exclusion_rationale"
    ]:
        failures.append("non-authority exclusion rationale is missing")
    return failures


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
        raise RuntimeError(f"configuration authority mutation command failed: {command[0]}")
    return result.stdout + result.stderr


def execute_gate(root: Path = ROOT, runner: Runner = subprocess_runner) -> tuple[str, ...]:
    observed = []
    for name in EXPECTED_TESTS:
        output = runner(test_command(name), root)
        if TEST_NAME.findall(output) != [name]:
            raise RuntimeError("configuration authority mutation test closure failed")
        observed.append(name)
    runner(CLIPPY_COMMAND, root)
    return tuple(observed)


def build_report(executed_tests: Sequence[str], root: Path = ROOT) -> dict[str, Any]:
    if tuple(executed_tests) != EXPECTED_TESTS:
        raise ValueError("authority mutation evidence requires the exact focused tests")
    registry = read_json(root / REGISTRY_PATH.relative_to(ROOT))
    failures = validate_registry(registry)
    if failures:
        raise ValueError("; ".join(failures))
    attempt_count = EXPECTED_PATH_COUNT * len(SOURCES)
    return {
        "schema_version": 1,
        "task_id": "3.1.3.2",
        "test_id": "S-003-UT02",
        "status": "pass-shared-linux-signed-parent-authority-mutations",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in SOURCE_PATHS
        ],
        "commands": [
            *[
                {"argv": list(test_command(name)), "status": "pass"}
                for name in executed_tests
            ],
            {"argv": list(CLIPPY_COMMAND), "status": "pass"},
        ],
        "tests": list(executed_tests),
        "untrusted_sources": list(SOURCES),
        "permission_registry": {
            "path": REGISTRY_PATH.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(root / REGISTRY_PATH.relative_to(ROOT)),
            "permission_bearing_value_count": EXPECTED_PATH_COUNT,
            "authority_dimension_count": EXPECTED_DIMENSION_COUNT,
            "expected_rejection_counts": EXPECTED_REJECTIONS,
        },
        "parent_signature": {
            "algorithm": "ed25519",
            "implementation": "ed25519-dalek-3.0.0",
            "verified_before_authority_comparison": True,
            "configuration_identity_algorithm": "sha256",
            "tamper_wrong_key_and_cross_configuration_replay_rejected": True,
            "production_trust_root_claim": "none",
            "test_key_class": "synthetic-non-production",
        },
        "summary": {
            "focused_test_count": len(EXPECTED_TESTS),
            "failed_test_count": 0,
            "skipped_test_count": 0,
            "untrusted_source_count": len(SOURCES),
            "permission_bearing_value_count": EXPECTED_PATH_COUNT,
            "cross_channel_mutation_attempt_count": attempt_count,
            "accepted_broadening_count": 0,
            "parent_signature_failure_count": 0,
        },
        "configuration_file_channel_exercised": True,
        "environment_values_read": False,
        "raw_candidate_content_persisted": False,
        "private_paths_persisted": False,
        "network_used": False,
        "product_loader_registration_claim": "none",
        "product_profile_activation_claim": "none",
        "platform_execution": "fedora-linux",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration authority mutation report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.3.2"
        or value.get("test_id") != "S-003-UT02"
        or value.get("status")
        != "pass-shared-linux-signed-parent-authority-mutations"
    ):
        failures.append("configuration authority mutation identity is invalid")
    if value.get("tests") != list(EXPECTED_TESTS):
        failures.append("configuration authority mutation test closure is invalid")
    if value.get("untrusted_sources") != list(SOURCES):
        failures.append("configuration authority mutation source closure is invalid")
    registry = value.get("permission_registry", {})
    if (
        registry.get("permission_bearing_value_count") != EXPECTED_PATH_COUNT
        or registry.get("authority_dimension_count") != EXPECTED_DIMENSION_COUNT
        or registry.get("expected_rejection_counts") != EXPECTED_REJECTIONS
    ):
        failures.append("configuration authority mutation registry summary is invalid")
    signature = value.get("parent_signature", {})
    if (
        signature.get("algorithm") != "ed25519"
        or signature.get("implementation") != "ed25519-dalek-3.0.0"
        or signature.get("verified_before_authority_comparison") is not True
        or signature.get("tamper_wrong_key_and_cross_configuration_replay_rejected")
        is not True
        or signature.get("production_trust_root_claim") != "none"
        or signature.get("test_key_class") != "synthetic-non-production"
    ):
        failures.append("configuration parent-signature evidence is invalid")
    summary = value.get("summary", {})
    if (
        summary.get("focused_test_count") != len(EXPECTED_TESTS)
        or summary.get("failed_test_count") != 0
        or summary.get("skipped_test_count") != 0
        or summary.get("untrusted_source_count") != len(SOURCES)
        or summary.get("permission_bearing_value_count") != EXPECTED_PATH_COUNT
        or summary.get("cross_channel_mutation_attempt_count")
        != EXPECTED_PATH_COUNT * len(SOURCES)
        or summary.get("accepted_broadening_count") != 0
        or summary.get("parent_signature_failure_count") != 0
    ):
        failures.append("configuration authority mutation summary is invalid")
    if (
        value.get("configuration_file_channel_exercised") is not True
        or value.get("environment_values_read") is not False
        or value.get("raw_candidate_content_persisted") is not False
        or value.get("private_paths_persisted") is not False
        or value.get("network_used") is not False
        or value.get("product_loader_registration_claim") != "none"
        or value.get("product_profile_activation_claim") != "none"
        or value.get("platform_execution") != "fedora-linux"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration authority mutation report made an unsupported claim")
    try:
        expected = build_report(EXPECTED_TESTS, root)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        failures.append(f"cannot rebuild configuration authority mutation report: {error}")
    else:
        if value != expected:
            failures.append("configuration authority mutation report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read configuration authority mutation report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            tests = execute_gate()
            write_atomic(REPORT_PATH, canonical_json(build_report(tests)))
        failures = check_artifact()
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"configuration authority mutation evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration authority mutation evidence failed: {failure}")
        return 1
    print("Story 3.1 signed-parent authority mutation matrix validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
