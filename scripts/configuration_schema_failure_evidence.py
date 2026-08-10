#!/usr/bin/env python3
"""Build and validate S-003-UT01 configuration failure-matrix evidence."""

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
REPORT_PATH = (
    ROOT
    / "artifacts/sprints/sprint-3/story-3.1/configuration-schema-failure-report.json"
)
EXPECTED_TEST = (
    "every_configuration_schema_failure_class_is_stable_and_side_effect_free"
)
SCHEMA_IDS = (
    "agent-configuration",
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
FAILURE_CLASSES = (
    "missing",
    "extra",
    "wrong-type",
    "oversized",
    "unsupported-version",
    "ambiguous",
)
TEST_COMMAND = (
    "cargo",
    "test",
    "--offline",
    "-p",
    "agentmage-kernel-engine",
    "--locked",
    f"configuration::tests::{EXPECTED_TEST}",
    "--",
    "--exact",
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
    "configuration/profiles/strict-local-read-only.json",
    "configuration/profiles/synthetic-test.json",
    "schemas/configuration/common.schema.json",
    "schemas/configuration/agent-configuration.schema.json",
    *(
        f"schemas/configuration/{schema_id}.schema.json"
        for schema_id in SCHEMA_IDS
        if schema_id != "agent-configuration"
    ),
    "scripts/configuration_schema_failure_evidence.py",
    "tests/test_configuration_schema_failure_evidence.py",
)
TEST_NAME = re.compile(
    r"^test configuration::tests::([a-z0-9_]+) \.\.\. ok$", re.MULTILINE
)
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
        prefix=".agentmage-configuration-schema-failure-", dir=path.parent
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
        raise RuntimeError(f"configuration failure evidence command failed: {command[0]}")
    return result.stdout + result.stderr


def execute_gate(root: Path = ROOT, runner: Runner = subprocess_runner) -> str:
    output = runner(TEST_COMMAND, root)
    if TEST_NAME.findall(output) != [EXPECTED_TEST]:
        raise RuntimeError("configuration failure test identity closure failed")
    runner(CLIPPY_COMMAND, root)
    return EXPECTED_TEST


def build_report(executed_test: str, root: Path = ROOT) -> dict[str, Any]:
    if executed_test != EXPECTED_TEST:
        raise ValueError("configuration failure evidence requires the exact focused test")
    case_count = len(SCHEMA_IDS) * len(FAILURE_CLASSES)
    return {
        "schema_version": 1,
        "task_id": "3.1.3.1",
        "test_id": "S-003-UT01",
        "status": "pass-shared-linux-configuration-schema-failures",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in SOURCE_PATHS
        ],
        "commands": [
            {"argv": list(TEST_COMMAND), "status": "pass"},
            {"argv": list(CLIPPY_COMMAND), "status": "pass"},
        ],
        "test": executed_test,
        "schema_ids": list(SCHEMA_IDS),
        "failure_classes": list(FAILURE_CLASSES),
        "stable_diagnostic_codes": [
            "configuration-contract-violation",
            "configuration-malformed-json",
            "configuration-too-large",
            "configuration-unsupported-version",
        ],
        "summary": {
            "schema_count": len(SCHEMA_IDS),
            "failure_class_count": len(FAILURE_CLASSES),
            "matrix_case_count": case_count,
            "rejected_case_count": case_count,
            "stable_diagnostic_case_count": case_count,
            "partial_startup_case_count": 0,
            "filesystem_mutation_case_count": 0,
            "failed_test_count": 0,
            "skipped_test_count": 0,
        },
        "raw_configuration_persisted": False,
        "private_paths_persisted": False,
        "network_used": False,
        "product_startup_registration_claim": "none",
        "product_profile_activation_claim": "none",
        "platform_execution": "fedora-linux",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration failure report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.3.1"
        or value.get("test_id") != "S-003-UT01"
        or value.get("status")
        != "pass-shared-linux-configuration-schema-failures"
    ):
        failures.append("configuration failure report identity is invalid")
    if value.get("test") != EXPECTED_TEST:
        failures.append("configuration failure test identity is invalid")
    if value.get("schema_ids") != list(SCHEMA_IDS):
        failures.append("configuration failure schema closure is invalid")
    if value.get("failure_classes") != list(FAILURE_CLASSES):
        failures.append("configuration failure class closure is invalid")
    case_count = len(SCHEMA_IDS) * len(FAILURE_CLASSES)
    summary = value.get("summary", {})
    if (
        summary.get("schema_count") != len(SCHEMA_IDS)
        or summary.get("failure_class_count") != len(FAILURE_CLASSES)
        or summary.get("matrix_case_count") != case_count
        or summary.get("rejected_case_count") != case_count
        or summary.get("stable_diagnostic_case_count") != case_count
        or summary.get("partial_startup_case_count") != 0
        or summary.get("filesystem_mutation_case_count") != 0
        or summary.get("failed_test_count") != 0
        or summary.get("skipped_test_count") != 0
    ):
        failures.append("configuration failure summary is invalid")
    if (
        value.get("raw_configuration_persisted") is not False
        or value.get("private_paths_persisted") is not False
        or value.get("network_used") is not False
        or value.get("product_startup_registration_claim") != "none"
        or value.get("product_profile_activation_claim") != "none"
        or value.get("platform_execution") != "fedora-linux"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration failure report made an unsupported claim")
    try:
        expected = build_report(EXPECTED_TEST, root)
    except (OSError, ValueError) as error:
        failures.append(f"cannot rebuild configuration failure report: {error}")
    else:
        if value != expected:
            failures.append("configuration failure report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read configuration failure report: {error}"]
    return validate_report(report, root)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            executed_test = execute_gate()
            write_atomic(REPORT_PATH, canonical_json(build_report(executed_test)))
        failures = check_artifact()
    except (OSError, RuntimeError, ValueError) as error:
        print(f"configuration failure evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration failure evidence failed: {failure}")
        return 1
    print("Story 3.1 configuration schema failure matrix validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
