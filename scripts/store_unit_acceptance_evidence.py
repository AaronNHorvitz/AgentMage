#!/usr/bin/env python3
"""Build and validate S-011-UT01 operational-store unit evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/s-011-ut01.json"
DOCUMENT_PATH: Final = ROOT / "docs/verification/s-011-ut01-store-unit-results.md"
STORE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
SOURCE_PATHS: Final = (
    "docs/verification/s-011-ut01-store-unit-results.md",
    "kernel/engine/src/operational_store.rs",
    "scripts/store_unit_acceptance_evidence.py",
    "tests/test_store_unit_acceptance_evidence.py",
)
MATRIX_IDS: Final = ("UT-01", "UT-02", "UT-03", "UT-04", "UT-05", "UT-06", "UT-07", "UT-08")
EXPECTED_TESTS: Final = (
    "encrypted_store_requires_key_and_hides_sqlite_header",
    "encrypted_store_opens_through_exact_held_linux_directory_descriptor",
    "only_exact_numeric_proc_self_fd_parent_is_a_held_descriptor_path",
    "wrong_key_and_wrong_storage_class_fail_closed",
    "exclusive_writer_and_encrypted_backup_are_verified",
    "encrypted_backup_restores_only_to_a_verified_fresh_candidate",
    "synthetic_canary_is_absent_from_encrypted_and_derived_artifacts",
    "corrupted_or_wrongly_keyed_backup_leaves_no_restore_candidate",
    "whole_store_cryptographic_erasure_consumes_key_scope_without_overwrite_claim",
    "json_lines_export_is_deterministic_content_free_and_export_only",
    "json_lines_export_rejects_occupied_or_ineligible_destinations_without_change",
    "schema_constraints_and_atomic_rollback_reject_partial_authority",
    "required_writer_and_database_configuration_is_verified",
    "version_eighteen_schema_matches_fixture_snapshot_and_is_relational",
    "version_one_upgrades_through_eighteen_with_exact_history",
    "version_two_retention_rows_upgrade_to_three_with_initial_event",
    "failed_version_three_migration_rolls_back_all_alterations",
    "failed_version_two_migration_rolls_back_without_partial_schema",
    "future_schema_and_page_corruption_are_refused",
    "retention_holds_expiration_and_stale_revisions_are_atomic",
    "retention_assignment_and_event_tampering_fail_closed",
)
DOCUMENT_FRAGMENTS: Final = (
    "**Status:** Pass at the deterministic unit boundary",
    "The complete focused suite contains 21 tests.",
    "Failed v3 migration retains exact v2 schema",
    "Writer contention uses independent SQLCipher connections in one process",
    "Rollback uses deterministic database failures; it is not an operating-system",
    "Manual fuzzing is not part of this unit result and remains deferred.",
)
STORE_FRAGMENTS: Final = (
    "BEGIN EXCLUSIVE; COMMIT;",
    "PRAGMA foreign_key_check",
    "fn persist_snapshot(",
    "fn append_retention_event(",
    "fn verify_schema_history(",
    "fn verify_integrity(",
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "operational_store", "--locked"), "23 passed; 0 failed"),
    (("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_store_unit_acceptance_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "matrix_class_count": 8,
    "focused_test_count": 21,
    "schema_constraints_pass": True,
    "foreign_keys_and_relationships_pass": True,
    "single_writer_runtime_policy_pass": True,
    "retention_transitions_pass": True,
    "migration_versions_pass": True,
    "migration_rollback_pass": True,
    "invalid_states_fail_closed": True,
    "partial_or_orphaned_state_observed": False,
    "multiprocess_writer_evidence": False,
    "operating_system_process_kill_used": False,
    "s011_st01_complete": False,
    "s011_rt01_complete": False,
    "manual_fuzzing_executed": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Writer contention uses independent SQLCipher connections in one process rather than a retained multiprocess fixture.",
    "Rollback uses deterministic database failures rather than operating-system process kill or power-loss injection.",
    "The every-field secret-canary and at-least-100-seed crash campaigns remain Sub-tasks 11.1.3.2 and 11.1.3.3.",
    "Live Secret Service substitution, macOS, Windows, packaging, release acceptance, and manually deferred fuzzing are not claimed.",
]
TEST_PATTERN = re.compile(r"#\[test\]\s+fn ([a-z0-9_]+)\(\)")
MATRIX_PATTERN = re.compile(r"`(UT-\d{2})`")
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when S-011-UT01 evidence is incomplete or overstated."""


def validate_sources(document: str, store: str) -> list[str]:
    failures = []
    if tuple(MATRIX_PATTERN.findall(document)) != MATRIX_IDS:
        failures.append("S-011-UT01 acceptance matrix changed")
    observed_tests = TEST_PATTERN.findall(store)
    if len(observed_tests) != len(set(observed_tests)) or not set(EXPECTED_TESTS).issubset(
        observed_tests
    ):
        failures.append("S-011-UT01 focused test closure changed")
    for label, value, fragments in (
        ("document", document, DOCUMENT_FRAGMENTS),
        ("store", store, STORE_FRAGMENTS),
    ):
        failures.extend(
            f"S-011-UT01 {label} fragment changed: {index}"
            for index, fragment in enumerate(fragments, 1)
            if value.count(fragment) != 1
        )
    return failures


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError("committed source is unavailable")
    return result.stdout


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "exit_code": 0,
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(
        list(arguments), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        timeout=300, check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification command failed: {Path(arguments[0]).name}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values: dict[str, str] = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    failures = validate_sources(
        values[str(DOCUMENT_PATH.relative_to(ROOT))],
        values[str(STORE_PATH.relative_to(ROOT))],
    )
    if failures:
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "acceptance_matrix_ids": list(MATRIX_IDS),
        "artifact_id": "s-011-ut01-operational-store-unit-results",
        "claims": CLAIMS,
        "external_network_used": False,
        "focused_tests": list(EXPECTED_TESTS),
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-deterministic-store-unit-boundary",
        "task_ids": ["11.1.3.1", "S-011-UT01"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "acceptance_matrix_ids": list(MATRIX_IDS),
        "artifact_id": "s-011-ut01-operational-store-unit-results",
        "claims": CLAIMS,
        "external_network_used": False,
        "focused_tests": list(EXPECTED_TESTS),
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-deterministic-store-unit-boundary",
        "task_ids": ["11.1.3.1", "S-011-UT01"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"S-011-UT01 evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-011-UT01 evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("S-011-UT01 evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("S-011-UT01 evidence source records are invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError("committed source binding changed")


def write_atomic(report: dict[str, Any]) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{REPORT_PATH.name}.", dir=REPORT_PATH.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, REPORT_PATH)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    if arguments.write:
        report = build_report(revision)
        write_atomic(report)
    else:
        report = json.loads(REPORT_PATH.read_text())
    failures = validate_report(report)
    if failures:
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("S-011-UT01 evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
