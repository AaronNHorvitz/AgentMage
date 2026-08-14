#!/usr/bin/env python3
"""Build and validate S-003-RT01 configuration migration recovery evidence."""

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
    / "artifacts/sprints/sprint-3/story-3.1/configuration-migration-recovery-report.json"
)
EXPECTED_TEST = "migration_interruptions_select_valid_state_and_rollback_is_repeatable"
DURABLE_TRANSITIONS = (
    "backup-published",
    "candidate-published",
    "target-published",
)
INTERRUPTION_BOUNDARIES = ("before", "after")
COMMAND = (
    "cargo",
    "test",
    "--offline",
    "-p",
    "agentmage-platform-linux",
    f"configuration_store::tests::{EXPECTED_TEST}",
    "--locked",
    "--",
    "--exact",
)
CLIPPY_COMMAND = (
    "cargo",
    "clippy",
    "--offline",
    "-p",
    "agentmage-platform-linux",
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
    "platforms/linux/Cargo.toml",
    "platforms/linux/src/configuration_store.rs",
    "fixtures/configuration/migration/v0.valid.json",
    "fixtures/configuration/migration/v1.expected.json",
    "scripts/configuration_migration_recovery_evidence.py",
    "tests/test_configuration_migration_recovery_evidence.py",
)
TEST_NAME = re.compile(
    r"^test configuration_store::tests::([a-z0-9_]+) \.\.\. ok$", re.MULTILINE
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
        prefix=".agentmage-configuration-migration-recovery-", dir=path.parent
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
        raise RuntimeError(f"configuration migration recovery command failed: {command[0]}")
    return result.stdout + result.stderr


def execute_gate(root: Path = ROOT, runner: Runner = subprocess_runner) -> str:
    output = runner(COMMAND, root)
    if TEST_NAME.findall(output) != [EXPECTED_TEST]:
        raise RuntimeError("configuration migration recovery test closure failed")
    runner(CLIPPY_COMMAND, root)
    return EXPECTED_TEST


def build_report(executed_test: str, root: Path = ROOT) -> dict[str, Any]:
    if executed_test != EXPECTED_TEST:
        raise ValueError("migration recovery evidence requires the exact focused test")
    interruption_count = len(DURABLE_TRANSITIONS) * len(INTERRUPTION_BOUNDARIES)
    return {
        "schema_version": 1,
        "task_id": "3.1.3.3",
        "test_id": "S-003-RT01",
        "status": "pass-shared-linux-configuration-migration-recovery",
        "source_artifacts": [
            {"path": path, "sha256": sha256_file(root / path)}
            for path in SOURCE_PATHS
        ],
        "commands": [
            {"argv": list(COMMAND), "status": "pass"},
            {"argv": list(CLIPPY_COMMAND), "status": "pass"},
        ],
        "tests": [executed_test],
        "durable_transitions": list(DURABLE_TRANSITIONS),
        "interruption_boundaries": list(INTERRUPTION_BOUNDARIES),
        "summary": {
            "focused_test_count": 1,
            "durable_transition_count": len(DURABLE_TRANSITIONS),
            "injected_interruption_count": interruption_count,
            "concurrent_preimage_mutation_count": 1,
            "invalid_selected_state_count": 0,
            "failed_test_count": 0,
            "skipped_test_count": 0,
            "repeatable_rollback": "pass-idempotent-exact-preimage",
        },
        "state_selection": {
            "allowed": ["exact-prior-version-0", "complete-canonical-version-1"],
            "partial_configuration_allowed": False,
            "preimage_change_overwrite_allowed": False,
        },
        "durability": {
            "content_addressed_backup": True,
            "private_backup_and_candidate": True,
            "file_sync": True,
            "parent_directory_sync": True,
            "atomic_same_directory_publish": True,
            "prepared_candidate_retry": True,
        },
        "raw_configuration_persisted_in_report": False,
        "private_paths_persisted_in_report": False,
        "network_used": False,
        "product_startup_migration_claim": "none",
        "product_profile_activation_claim": "none",
        "platform_execution": "fedora-linux",
        "macos_execution_status": "blocked-macos",
        "macos_support_claim": "none",
    }


def validate_report(value: Any, root: Path = ROOT) -> list[str]:
    if not isinstance(value, dict):
        return ["configuration migration recovery report must be an object"]
    failures = []
    if (
        value.get("schema_version") != 1
        or value.get("task_id") != "3.1.3.3"
        or value.get("test_id") != "S-003-RT01"
        or value.get("status")
        != "pass-shared-linux-configuration-migration-recovery"
    ):
        failures.append("configuration migration recovery identity is invalid")
    if value.get("tests") != [EXPECTED_TEST]:
        failures.append("configuration migration recovery test closure is invalid")
    if value.get("durable_transitions") != list(DURABLE_TRANSITIONS) or value.get(
        "interruption_boundaries"
    ) != list(INTERRUPTION_BOUNDARIES):
        failures.append("configuration migration transition closure is invalid")
    summary = value.get("summary", {})
    if summary != {
        "focused_test_count": 1,
        "durable_transition_count": 3,
        "injected_interruption_count": 6,
        "concurrent_preimage_mutation_count": 1,
        "invalid_selected_state_count": 0,
        "failed_test_count": 0,
        "skipped_test_count": 0,
        "repeatable_rollback": "pass-idempotent-exact-preimage",
    }:
        failures.append("configuration migration recovery summary is invalid")
    if value.get("state_selection") != {
        "allowed": ["exact-prior-version-0", "complete-canonical-version-1"],
        "partial_configuration_allowed": False,
        "preimage_change_overwrite_allowed": False,
    }:
        failures.append("configuration migration state selection is invalid")
    durability = value.get("durability", {})
    if set(durability.values()) != {True} or set(durability) != {
        "content_addressed_backup",
        "private_backup_and_candidate",
        "file_sync",
        "parent_directory_sync",
        "atomic_same_directory_publish",
        "prepared_candidate_retry",
    }:
        failures.append("configuration migration durability evidence is invalid")
    if (
        value.get("raw_configuration_persisted_in_report") is not False
        or value.get("private_paths_persisted_in_report") is not False
        or value.get("network_used") is not False
        or value.get("product_startup_migration_claim") != "none"
        or value.get("product_profile_activation_claim") != "none"
        or value.get("platform_execution") != "fedora-linux"
        or value.get("macos_execution_status") != "blocked-macos"
        or value.get("macos_support_claim") != "none"
    ):
        failures.append("configuration migration report made an unsupported claim")
    try:
        expected = build_report(EXPECTED_TEST, root)
    except (OSError, ValueError) as error:
        failures.append(f"cannot rebuild configuration migration report: {error}")
    else:
        if value != expected:
            failures.append("configuration migration report is stale or non-deterministic")
    return failures


def check_artifact(root: Path = ROOT) -> list[str]:
    try:
        report = read_json(root / REPORT_PATH.relative_to(ROOT))
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read configuration migration report: {error}"]
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
        print(f"configuration migration recovery evidence failed: {error}")
        return 1
    if failures:
        for failure in failures:
            print(f"configuration migration recovery evidence failed: {failure}")
        return 1
    print("Story 3.1 configuration migration recovery evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
