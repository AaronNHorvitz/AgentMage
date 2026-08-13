#!/usr/bin/env python3
"""Build and validate S-011-RT01 seeded crash-recovery evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/s-011-rt01.json"
SOURCE_PATHS: Final = (
    "docs/verification/s-011-rt01-seeded-crash-recovery-results.md",
    "kernel/engine/src/authority_transaction.rs",
    "kernel/engine/src/operational_store.rs",
    "scripts/seeded_crash_recovery_evidence.py",
    "tests/test_seeded_crash_recovery_evidence.py",
)
BOUNDARY_IDS: Final = tuple(f"RT-{index:02}" for index in range(1, 9))
BOUNDARIES: Final = (
    "transaction",
    "checkpoint",
    "migration",
    "key-retrieval",
    "backup",
    "restore",
    "expiry",
    "deletion",
)
POSITIONS: Final = ("before", "after")
DOCUMENT_FRAGMENTS: Final = (
    "executes 128 deterministic subprocess runs",
    "Every one of the 16 boundary-position pairs runs exactly eight",
    "Repeated completed durable transitions observed: **0**.",
    "Authority recovery/replay launches observed: **0**.",
    "deterministic `exit(86)` without unwinding",
    "Manual fuzzing remains deferred until the end of development",
)
AUTHORITY_FRAGMENTS: Final = (
    "fn encrypted_restart_recovery_never_replays_and_publishes_one_receipt()",
    "assert_eq!(runtime.receipts().len(), 1);",
    "assert_eq!(replay_driver.launches, 0);",
)
STORE_FRAGMENTS: Final = (
    "const SEEDED_CRASH_RUNS: u64 = 128;",
    "const SEEDED_CRASH_CHILD_EXIT: i32 = 86;",
    "enum SeededCrashBoundary {",
    "enum SeededCrashPosition {",
    "fn seeded_crash_recovery_child()",
    "fn seeded_crash_recovery_campaign_never_repeats_a_completed_transition()",
    "assert_eq!(coverage.len(), 16);",
    "assert_eq!(coverage.get(&(boundary, position)), Some(&8));",
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked", "operational_store::tests::seeded_crash_recovery_campaign_never_repeats_a_completed_transition", "--", "--exact"), "1 passed; 0 failed"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "--lib", "--locked", "authority_transaction::tests::encrypted_restart_recovery_never_replays_and_publishes_one_receipt", "--", "--exact"), "1 passed; 0 failed"),
    (("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_seeded_crash_recovery_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "seed_count": 128,
    "boundary_family_count": 8,
    "position_count": 2,
    "boundary_position_pair_count": 16,
    "runs_per_boundary_position_pair": 8,
    "abrupt_subprocess_stop_count": 128,
    "repeated_completed_durable_transitions": 0,
    "authority_recovery_or_replay_launches": 0,
    "hard_exit_without_unwinding": True,
    "kernel_sigkill_or_power_loss_tested": False,
    "live_secret_service_interruption_tested": False,
    "manual_fuzzing_executed": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The deterministic child exits with status 86 without unwinding; kernel SIGKILL, host power loss, torn sectors, storage-controller failure, and filesystem corruption are not claimed.",
    "Deletion uses a synthetic file-backed test key so key absence is observable across processes; live Linux Secret Service interruption remains S-011-IT01.",
    "Backup and restore cover fresh immutable candidates, not live-store swap, clean-device continuity, or remote backup providers.",
    "Execution is current-host Linux evidence only; clean distribution, macOS, Windows, packaging, release, support, and manually deferred fuzzing remain separate gates.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    pass


def validate_sources(document: str, authority: str, store: str) -> list[str]:
    failures = []
    if tuple(re.findall(r"`(RT-\d{2})`", document)) != BOUNDARY_IDS:
        failures.append("S-011-RT01 boundary closure changed")
    for label, value, fragments in (
        ("document", document, DOCUMENT_FRAGMENTS),
        ("authority", authority, AUTHORITY_FRAGMENTS),
        ("store", store, STORE_FRAGMENTS),
    ):
        failures.extend(
            f"S-011-RT01 {label} fragment changed: {index}"
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
        raise EvidenceError("source revision unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError("committed source unavailable")
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
        raise EvidenceError(f"verification failed: {arguments[0]}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    values = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    failures = validate_sources(values[SOURCE_PATHS[0]], values[SOURCE_PATHS[1]], values[SOURCE_PATHS[2]])
    if failures:
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "s-011-rt01-seeded-crash-recovery-results",
        "boundaries": list(BOUNDARIES),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "positions": list(POSITIONS),
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-current-encrypted-operational-store-boundary",
        "task_ids": ["11.1.3.3", "S-011-RT01"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    exact = {
        "artifact_id": "s-011-rt01-seeded-crash-recovery-results",
        "boundaries": list(BOUNDARIES),
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "positions": list(POSITIONS),
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-current-encrypted-operational-store-boundary",
        "task_ids": ["11.1.3.3", "S-011-RT01"],
        "verification_commands": expected_commands(),
    }
    failures = [f"S-011-RT01 evidence {key} changed" for key, value in exact.items() if report.get(key) != value]
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("S-011-RT01 evidence revision invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("S-011-RT01 evidence sources changed")
    elif any(not isinstance(item.get("bytes"), int) or item["bytes"] <= 0 or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources):
        failures.append("S-011-RT01 evidence source records invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError("committed source binding changed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    report = build_report(revision) if arguments.write else json.loads(REPORT_PATH.read_text())
    if arguments.write:
        atomic_write(REPORT_PATH, canonical_json_bytes(report))
    failures = validate_report(report)
    if failures:
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("S-011-RT01 evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
