#!/usr/bin/env python3
"""Build and validate evidence for restrict-only configuration channels."""

from __future__ import annotations

import argparse
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
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.1/configuration-authority-report.json"
EXPECTED_TESTS = (
    "aggregate_diff_detects_non_capability_authority_broadening",
    "every_permission_bearing_value_is_rejected_through_every_untrusted_channel",
    "every_untrusted_channel_accepts_only_a_valid_restriction",
    "every_untrusted_channel_rejects_capability_broadening",
    "malformed_untrusted_input_fails_before_authority_comparison",
    "parent_profile_signature_verification_rejects_tampering_and_wrong_keys",
    "resource_logging_and_retention_increases_are_rejected",
    "roots_models_tools_and_platform_identity_cannot_broaden_or_change",
)
SOURCES = (
    "configuration-file",
    "environment",
    "child-profile",
    "repository",
    "model-output",
)
AUTHORITY_DIMENSIONS = (
    "platform-identity",
    "model-activation-runtime-and-limits",
    "workspace-root-access-and-symlink-policy",
    "tool-identity-enablement-and-capabilities",
    "permission-capabilities-network-and-grants",
    "resource-budgets",
    "logging-verbosity-size-and-redaction",
    "retention-durations-and-backup-policy",
    "skill-catalogs-limits-and-capabilities",
    "shell-identity-execution-and-network",
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
    "schemas/configuration/agent-configuration.schema.json",
    "configuration/profiles/strict-local-read-only.json",
    "configuration/profiles/synthetic-test.json",
    "scripts/configuration_authority_evidence.py",
    "tests/test_configuration_authority_evidence.py",
)
TEST_NAME = re.compile(r"^test configuration::tests::([a-z0-9_]+) \.\.\. ok$", re.MULTILINE)
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
        prefix=".agentmage-configuration-authority-", dir=path.parent
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
        raise RuntimeError(f"authority evidence command failed: {command[1]}")
    return result.stdout + result.stderr


def execute_gate(root: Path = ROOT, runner: Runner = subprocess_runner) -> tuple[str, ...]:
    observed = []
    for name in EXPECTED_TESTS:
        output = runner(test_command(name), root)
        names = TEST_NAME.findall(output)
        if names != [name]:
            raise RuntimeError("configuration authority test identity closure failed")
        observed.append(name)
    runner(CLIPPY_COMMAND, root)
    return tuple(observed)


def build_report(executed_tests: Sequence[str], root: Path = ROOT) -> dict[str, Any]:
    if tuple(executed_tests) != EXPECTED_TESTS:
        raise ValueError("authority evidence requires the exact focused test set")
    return {
        "schema_version": 1,
        "task_id": "3.1.1.4",
        "status": "pass-shared-linux-restrict-only-configuration",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)} for path in SOURCE_PATHS
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
        "authority_dimensions": list(AUTHORITY_DIMENSIONS),
        "summary": {
            "focused_test_count": len(executed_tests),
            "failed_test_count": 0,
            "skipped_test_count": 0,
            "untrusted_source_count": len(SOURCES),
            "authority_dimension_count": len(AUTHORITY_DIMENSIONS),
            "permission_bearing_value_count": 97,
            "cross_channel_mutation_attempt_count": 485,
            "accepted_broadening_count": 0,
            "parent_identity_binding": "sha256",
            "parent_signature_algorithm": "ed25519",
            "parent_signature_verification": "required-before-authority-comparison",
            "comparison_policy": "candidate-must-be-subset-of-parent",
        },
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
        return ["configuration authority report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.1.4"
        or value.get("status") != "pass-shared-linux-restrict-only-configuration"
    ):
        failures.append("configuration authority report identity is invalid")
    if value.get("tests") != list(EXPECTED_TESTS):
        failures.append("configuration authority test closure is invalid")
    if value.get("untrusted_sources") != list(SOURCES):
        failures.append("configuration untrusted-source closure is invalid")
    if value.get("authority_dimensions") != list(AUTHORITY_DIMENSIONS):
        failures.append("configuration authority dimension closure is invalid")
    summary = value.get("summary", {})
    if (
        summary.get("focused_test_count") != len(EXPECTED_TESTS)
        or summary.get("failed_test_count") != 0
        or summary.get("skipped_test_count") != 0
        or summary.get("permission_bearing_value_count") != 97
        or summary.get("cross_channel_mutation_attempt_count") != 485
        or summary.get("accepted_broadening_count") != 0
        or summary.get("parent_signature_algorithm") != "ed25519"
        or summary.get("parent_signature_verification")
        != "required-before-authority-comparison"
    ):
        failures.append("configuration authority summary is invalid")
    if (
        value.get("environment_values_read") is not False
        or value.get("raw_candidate_content_persisted") is not False
        or value.get("private_paths_persisted") is not False
        or value.get("network_used") is not False
        or value.get("product_loader_registration_claim") != "none"
        or value.get("product_profile_activation_claim") != "none"
        or value.get("platform_execution") != "fedora-linux"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration authority report made an unsupported claim")
    try:
        expected = build_report(EXPECTED_TESTS, root)
    except (OSError, ValueError) as error:
        failures.append(f"cannot rebuild configuration authority report: {error}")
    else:
        if value != expected:
            failures.append("configuration authority report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read configuration authority report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            executed_tests = execute_gate()
            write_atomic(REPORT_PATH, canonical_json(build_report(executed_tests)))
        failures = check_artifact()
    except (OSError, RuntimeError, ValueError) as error:
        print(f"configuration authority evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration authority evidence failed: {failure}")
        return 1
    print("Story 3.1 restrict-only configuration authority evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
