#!/usr/bin/env python3
"""Build and validate evidence for configuration-bound result identities."""

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
REPORT_PATH = ROOT / "artifacts/sprints/sprint-3/story-3.1/configuration-result-report.json"
EXPECTED_TESTS = (
    "configuration_bound_results_are_deterministic_and_minimized",
    "configuration_bound_results_reject_invalid_identifiers_and_hashes",
    "configuration_or_result_changes_produce_distinct_binding_hashes",
    "session_and_release_results_bind_the_exact_configuration_identity",
)
RESULT_KINDS = ("session", "release")
CONFIGURATION_IDENTITY_FIELDS = ("profile_id", "sha256")
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
SCHEMA_COMMAND = ("npm", "run", "schemas:check")
SOURCE_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "kernel/engine/Cargo.toml",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/configuration.rs",
    "schemas/configuration/common.schema.json",
    "schemas/configuration/configuration-result.schema.json",
    "schemas/configuration/examples/configuration-session-result.valid.json",
    "schemas/configuration/examples/configuration-release-result.valid.json",
    "schemas/planning/common.schema.json",
    "schemas/planning/release-manifest.schema.json",
    "schemas/planning/examples/release-manifest.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "scripts/configuration_result_evidence.py",
    "tests/test_configuration_result_evidence.py",
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
        prefix=".agentmage-configuration-result-", dir=path.parent
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
        raise RuntimeError(f"configuration result evidence command failed: {command[0]}")
    return result.stdout + result.stderr


def execute_gate(root: Path = ROOT, runner: Runner = subprocess_runner) -> tuple[str, ...]:
    observed = []
    for name in EXPECTED_TESTS:
        output = runner(test_command(name), root)
        names = TEST_NAME.findall(output)
        if names != [name]:
            raise RuntimeError("configuration result test identity closure failed")
        observed.append(name)
    runner(CLIPPY_COMMAND, root)
    runner(SCHEMA_COMMAND, root)
    return tuple(observed)


def build_report(executed_tests: Sequence[str], root: Path = ROOT) -> dict[str, Any]:
    if tuple(executed_tests) != EXPECTED_TESTS:
        raise ValueError("configuration result evidence requires the exact focused test set")
    return {
        "schema_version": 1,
        "task_id": "3.1.1.5",
        "status": "pass-shared-linux-configuration-result-binding",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)} for path in SOURCE_PATHS
        ],
        "commands": [
            *[
                {"argv": list(test_command(name)), "status": "pass"}
                for name in executed_tests
            ],
            {"argv": list(CLIPPY_COMMAND), "status": "pass"},
            {"argv": list(SCHEMA_COMMAND), "status": "pass"},
        ],
        "tests": list(executed_tests),
        "result_kinds": list(RESULT_KINDS),
        "configuration_identity_fields": list(CONFIGURATION_IDENTITY_FIELDS),
        "summary": {
            "focused_test_count": len(executed_tests),
            "failed_test_count": 0,
            "skipped_test_count": 0,
            "configuration_hash_algorithm": "sha256",
            "record_hash_algorithm": "sha256",
            "release_manifest_configuration_identity_required": True,
            "deterministic_record_encoding": "canonical-json",
        },
        "raw_configuration_persisted": False,
        "private_paths_persisted": False,
        "network_used": False,
        "product_session_execution_claim": "none",
        "product_release_execution_claim": "none",
        "product_profile_activation_claim": "none",
        "platform_execution": "fedora-linux",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration result report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.1.5"
        or value.get("status")
        != "pass-shared-linux-configuration-result-binding"
    ):
        failures.append("configuration result report identity is invalid")
    if value.get("tests") != list(EXPECTED_TESTS):
        failures.append("configuration result test closure is invalid")
    if value.get("result_kinds") != list(RESULT_KINDS):
        failures.append("configuration result-kind closure is invalid")
    if value.get("configuration_identity_fields") != list(
        CONFIGURATION_IDENTITY_FIELDS
    ):
        failures.append("configuration result identity-field closure is invalid")
    summary = value.get("summary", {})
    if (
        summary.get("focused_test_count") != len(EXPECTED_TESTS)
        or summary.get("failed_test_count") != 0
        or summary.get("skipped_test_count") != 0
        or summary.get("configuration_hash_algorithm") != "sha256"
        or summary.get("record_hash_algorithm") != "sha256"
        or summary.get("release_manifest_configuration_identity_required") is not True
        or summary.get("deterministic_record_encoding") != "canonical-json"
    ):
        failures.append("configuration result summary is invalid")
    if (
        value.get("raw_configuration_persisted") is not False
        or value.get("private_paths_persisted") is not False
        or value.get("network_used") is not False
        or value.get("product_session_execution_claim") != "none"
        or value.get("product_release_execution_claim") != "none"
        or value.get("product_profile_activation_claim") != "none"
        or value.get("platform_execution") != "fedora-linux"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration result report made an unsupported claim")
    try:
        expected = build_report(EXPECTED_TESTS, root)
    except (OSError, ValueError) as error:
        failures.append(f"cannot rebuild configuration result report: {error}")
    else:
        if value != expected:
            failures.append("configuration result report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read configuration result report: {error}"]
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
        print(f"configuration result evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration result evidence failed: {failure}")
        return 1
    print("Story 3.1 configuration-bound result evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
