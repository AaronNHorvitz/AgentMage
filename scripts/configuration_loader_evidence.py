#!/usr/bin/env python3
"""Build and validate bounded evidence for the configuration loader."""

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
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.1/configuration-loader-report.json"
TEST_COMMAND = (
    "cargo",
    "test",
    "--offline",
    "-p",
    "agentmage-kernel-engine",
    "configuration",
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
EXPECTED_TESTS = (
    "atomic_apply_retains_backup_and_rollback_restores_exact_identity",
    "diff_is_stable_redacted_and_classifies_authority_and_resource_changes",
    "invalid_candidate_and_stale_rollback_preimage_preserve_current_file",
    "loads_canonical_profile_deterministically",
    "migrates_only_version_zero_with_fixed_non_broadening_operations",
    "migration_rejects_wrong_source_ambiguous_version_and_missing_section",
    "mirrors_published_identifier_version_collection_and_numeric_bounds",
    "rejects_permission_network_logging_and_budget_broadening",
    "rejects_unknown_duplicate_missing_unsupported_and_oversized_input",
    "safe_defaults_are_explicit_read_only_and_minimum_authority",
)
SOURCE_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "kernel/engine/Cargo.toml",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/configuration.rs",
    "schemas/configuration/agent-configuration.schema.json",
    "configuration/profiles/strict-local-read-only.json",
    "configuration/profiles/synthetic-test.json",
    "scripts/configuration_loader_evidence.py",
    "tests/test_configuration_loader_evidence.py",
)
TEST_NAME = re.compile(r"^test configuration::tests::([a-z0-9_]+) \.\.\. ok$", re.MULTILINE)
Runner = Callable[[Sequence[str], Path], str]


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def write_atomic(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=".agentmage-configuration-loader-", dir=path.parent
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
        raise RuntimeError(f"evidence command failed: {command[1]}")
    return result.stdout + result.stderr


def execute_gate(root: Path = ROOT, runner: Runner = subprocess_runner) -> tuple[str, ...]:
    test_output = runner(TEST_COMMAND, root)
    observed = tuple(sorted(TEST_NAME.findall(test_output)))
    if observed != EXPECTED_TESTS:
        raise RuntimeError("configuration test identity closure failed")
    runner(CLIPPY_COMMAND, root)
    return observed


def build_report(executed_tests: Sequence[str], root: Path = ROOT) -> dict[str, Any]:
    if tuple(executed_tests) != EXPECTED_TESTS:
        raise ValueError("configuration evidence requires the exact focused test set")
    return {
        "schema_version": 1,
        "task_id": "3.1.1.3",
        "status": "pass-shared-linux-configuration-loader",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)} for path in SOURCE_PATHS
        ],
        "commands": [
            {"argv": list(TEST_COMMAND), "status": "pass"},
            {"argv": list(CLIPPY_COMMAND), "status": "pass"},
        ],
        "tests": list(executed_tests),
        "summary": {
            "focused_test_count": len(executed_tests),
            "failed_test_count": 0,
            "skipped_test_count": 0,
            "unknown_and_duplicate_field_rejection": "pass",
            "safe_defaults": "pass-explicit-read-only-minimum-authority",
            "migration": "pass-version-0-to-1-only",
            "redacted_diff": "pass-hash-only-values",
            "atomic_backup": "pass-content-addressed-private-file",
            "rollback": "pass-preimage-checked",
        },
        "configuration_content_persisted": False,
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
        return ["configuration loader report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.1.3"
        or value.get("status") != "pass-shared-linux-configuration-loader"
    ):
        failures.append("configuration loader report identity is invalid")
    if value.get("tests") != list(EXPECTED_TESTS):
        failures.append("configuration loader test identity closure is invalid")
    summary = value.get("summary", {})
    if (
        summary.get("focused_test_count") != len(EXPECTED_TESTS)
        or summary.get("failed_test_count") != 0
        or summary.get("skipped_test_count") != 0
    ):
        failures.append("configuration loader test summary is invalid")
    if (
        value.get("configuration_content_persisted") is not False
        or value.get("private_paths_persisted") is not False
        or value.get("network_used") is not False
        or value.get("product_loader_registration_claim") != "none"
        or value.get("product_profile_activation_claim") != "none"
        or value.get("platform_execution") != "fedora-linux"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration loader report made an unsupported claim")
    try:
        expected = build_report(EXPECTED_TESTS, root)
    except (OSError, ValueError) as error:
        failures.append(f"cannot rebuild configuration loader report: {error}")
    else:
        if value != expected:
            failures.append("configuration loader report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read configuration loader report: {error}"]
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
        print(f"configuration loader evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration loader evidence failed: {failure}")
        return 1
    print("Story 3.1 configuration loader evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
