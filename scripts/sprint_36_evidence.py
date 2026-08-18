#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 36 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-36/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/write_transaction.rs",
    "kernel/engine/src/write_approval.rs",
    "kernel/engine/src/grants.rs",
    "kernel/engine/src/lib.rs",
    "platforms/linux/src/write_transaction.rs",
    "docs/architecture/atomic-write-transaction-and-rollback.md",
    "docs/verification/sprint-36-local-results.md",
    "scripts/sprint_36_evidence.py",
    "tests/test_sprint_36_evidence.py",
)
COMMANDS: Final = (
    (
        "write-transaction-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "write_transaction",
            "--locked",
        ),
    ),
    ("kernel-tests", ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked")),
    (
        "kernel-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
            "--locked", "--", "-D", "warnings",
        ),
    ),
    (
        "linux-write-transaction-tests",
        (
            "cargo", "test", "-p", "agentmage-platform-linux", "write_transaction",
            "--locked",
        ),
    ),
    (
        "linux-platform-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-platform-linux", "--all-targets",
            "--locked", "--", "-D", "warnings",
        ),
    ),
    ("contract-tests", ("cargo", "test", "-p", "agentmage-kernel-contracts", "--locked")),
    ("product-gate", ("npm", "run", "product:check")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("module-inventory", ("python3", "scripts/module_inventory.py")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_36_evidence")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-001", "SR-ACC-002", "SR-ACC-003", "SR-ACC-004", "SR-ACC-005",
    "SR-ACC-006", "SR-ACC-007", "SR-DAT-002", "SR-OPS-001", "SR-OPS-002",
    "SR-TST-005", "SR-TST-011", "SR-TST-012",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-35-BLOCKED", "owner": "35.1"},
    {"code": "SPRINT-36-ST01-NATIVE-RACE-MATRIX-INCOMPLETE", "owner": "36.1.3.3"},
    {"code": "SPRINT-36-RT01-CRASH-MATRIX-INCOMPLETE", "owner": "36.1.3.4"},
    {"code": "INDEPENDENT-SPRINT-36-TRANSACTION-REVIEW-ABSENT", "owner": "36.1.3.5"},
]
IMPLEMENTED: Final = {
    "grant_consuming_transaction_coordinator": True,
    "opaque_effect_authorization": True,
    "ordered_apply_and_restore_report_contract": True,
    "fresh_postimage_verification": True,
    "known_partial_preimage_restoration": True,
    "uncertain_terminal_outcome": True,
    "hash_chained_operation_receipts": True,
    "separately_granted_post_write_commands": True,
    "fresh_rollback_proposal": True,
    "later_user_change_rollback_refusal": True,
    "in_memory_driver_fixtures": True,
    "post_preview_mutation_matrix": True,
    "native_filesystem_driver": True,
    "native_linux_atomic_exchange_and_restore": True,
    "native_linux_descriptor_race_fixtures": True,
    "native_linux_parent_rename_boundary_matrix": True,
    "native_linux_process_stop_matrix": True,
    "native_atomicity_proven": False,
    "native_race_matrix_complete": False,
    "crash_durability_matrix_complete": False,
    "post_write_command_execution": False,
    "generic_shell": False,
    "network_access": False,
    "external_delivery": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 36 source is absent: {path}")
    return result.stdout


def version(executable: str, *arguments: str) -> str:
    resolved = shutil.which(executable)
    if resolved is None:
        return "unavailable"
    result = subprocess.run(
        (resolved, *arguments), cwd=ROOT, check=False, capture_output=True,
        text=True, timeout=30,
    )
    output = (result.stdout + result.stderr).strip().splitlines()
    return output[0][:256] if result.returncode == 0 and output else "unavailable"


def environment_manifest() -> dict[str, str]:
    return {
        "system": platform.system(), "release": platform.release(),
        "machine": platform.machine(), "python": platform.python_version(),
        "rustc": version("rustc", "--version"), "cargo": version("cargo", "--version"),
        "node": version("node", "--version"), "npm": version("npm", "--version"),
    }


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]), cwd=ROOT, check=False,
                capture_output=True, timeout=1800,
            )
            code, output = result.returncode, result.stdout + result.stderr
        blocking_skip_count = None
        if identifier in {"write-transaction-tests", "linux-write-transaction-tests"}:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        records.append({
            "id": identifier, "argv": list(argv), "exit_code": code,
            "output_sha256": digest(output), "blocking_skip_count": blocking_skip_count,
        })
    return records


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    focused = [
        item for item in commands
        if item["id"] in {"write-transaction-tests", "linux-write-transaction-tests"}
    ]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == 2
        and all(item.get("blocking_skip_count") == 0 for item in focused)
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_36_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "transaction_and_restoration_fixture_suite": local_pass,
            "receipt_integrity_and_rollback_suite": local_pass,
            "complete_local_product_and_docs_gates": local_pass,
            "focused_blocking_skip_count": 0 if local_pass else None,
            "upstream_sprint_35_gate": False,
            "post_preview_mutation_matrix": local_pass,
            "native_filesystem_driver_evidence": local_pass,
            "native_linux_atomic_exchange_and_restore": local_pass,
            "native_linux_descriptor_race_fixtures": local_pass,
            "native_linux_parent_rename_boundary_matrix": local_pass,
            "native_linux_process_stop_matrix": local_pass,
            "native_race_matrix": False,
            "native_mount_change_matrix": False,
            "crash_durability_matrix": False,
            "independent_transaction_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_atomic_transaction_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_dependency_passed": False,
            "native_filesystem_driver_proven": local_pass,
            "native_race_and_crash_evidence_passed": False,
            "independent_review_passed": False,
            "network_access_enabled": False,
            "release_approval": False,
        },
    }


def validate_report(report: dict[str, Any], verify_current: bool = True) -> list[str]:
    failures: list[str] = []
    revision = str(report.get("source_revision", ""))
    if not REVISION.fullmatch(revision):
        failures.append("source revision invalid")
    if report.get("security_requirement_ids") != SECURITY_REQUIREMENTS:
        failures.append("security mapping drift")
    if report.get("implemented_contracts") != IMPLEMENTED:
        failures.append("implemented contract drift")
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(
        item.get("exit_code") != 0
        or not SHA256.fullmatch(str(item.get("output_sha256", "")))
        for item in commands
    ):
        failures.append("command result invalid")
    focused = [
        item for item in commands
        if item.get("id") in {"write-transaction-tests", "linux-write-transaction-tests"}
    ]
    if len(focused) != 2 or any(
        item.get("blocking_skip_count") != 0 for item in focused
    ):
        failures.append("focused skipped, suppressed, or unavailable check")
    if any(
        item.get("blocking_skip_count") is not None
        for item in commands
        if item.get("id") not in {"write-transaction-tests", "linux-write-transaction-tests"}
    ):
        failures.append("supporting command skip count must remain not-applicable")
    expected_summary = {
        "local_atomic_transaction_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_dependency_passed": False,
        "native_filesystem_driver_proven": True,
        "native_race_and_crash_evidence_passed": False,
        "independent_review_passed": False,
        "network_access_enabled": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    if verification.get("focused_blocking_skip_count") != 0:
        failures.append("focused blocking skip summary invalid")
    for field in (
        "upstream_sprint_35_gate", "native_race_matrix", "native_mount_change_matrix",
        "crash_durability_matrix", "independent_transaction_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "post_preview_mutation_matrix", "native_filesystem_driver_evidence",
        "native_linux_atomic_exchange_and_restore", "native_linux_descriptor_race_fixtures",
        "native_linux_parent_rename_boundary_matrix", "native_linux_process_stop_matrix",
    ):
        if verification.get(field) is not True:
            failures.append(f"missing local verification: {field}")
    for field in (
        "native_atomicity_proven", "native_race_matrix_complete", "crash_durability_matrix_complete",
        "post_write_command_execution", "generic_shell", "network_access", "external_delivery",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"capability overclaim: {field}")
    environment = report.get("environment", {})
    expected_environment = {
        "system", "release", "machine", "python", "rustc", "cargo", "node", "npm",
    }
    if set(environment) != expected_environment:
        failures.append("environment manifest drift")
    elif any(
        not isinstance(value, str) or not value or len(value) > 256
        for value in environment.values()
    ):
        failures.append("environment manifest invalid")
    sources = report.get("source_sha256", {})
    if set(sources) != set(SOURCE_PATHS):
        failures.append("source inventory drift")
    elif verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            if sources[path] != digest(git_file(revision, path)):
                failures.append(f"source digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision], cwd=ROOT, check=True,
            capture_output=True, text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8",
        )
    else:
        report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    failures = validate_report(report)
    if failures:
        print("\n".join(failures))
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
