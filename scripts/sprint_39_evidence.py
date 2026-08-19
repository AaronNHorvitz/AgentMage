#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 39 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-39/local-evidence-report.json"
CORPUS_PATH: Final = "docs/verification/sprint-39-recovery-corpus.json"
AUDIT_PATH: Final = "docs/verification/sprint-39-redacted-audit-fixture.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/write_recovery.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/runtime_loop.rs",
    "kernel/engine/tests/write_recovery_matrix.rs",
    "kernel/engine/migrations/operational-store/0010-write-checkpoints.sql",
    "platforms/linux/src/platform.rs",
    "platforms/linux/src/write_transaction.rs",
    "platforms/linux/src/filesystem_control.rs",
    "capabilities/knowledge/src/lib.rs",
    "capabilities/knowledge/src/obsidian_index.rs",
    "shells/host/src/knowledge_write.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "schemas/runtime/write-aware-checkpoint.schema.json",
    "schemas/runtime/examples/write-aware-checkpoint.valid.json",
    "scripts/validate_planning_schemas.mjs",
    "tests/test_planning_schemas.mjs",
    "docs/architecture/write-privacy-recovery-audit.md",
    CORPUS_PATH,
    AUDIT_PATH,
    "docs/verification/sprint-39-local-results.md",
    "scripts/sprint_39_evidence.py",
    "tests/test_sprint_39_evidence.py",
)
COMMANDS: Final = (
    (
        "write-recovery-unit-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "write_recovery",
            "--locked",
        ),
    ),
    (
        "write-recovery-integration-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "--test",
            "write_recovery_matrix", "--locked",
        ),
    ),
    (
        "native-write-checkpoint-process-test",
        (
            "cargo", "test", "-p", "agentmage-host",
            "story_39_1_native_write_checkpoint_process_matrix_recovers_without_replay",
            "--locked",
        ),
    ),
    (
        "native-derived-index-checkpoint-tests",
        (
            "cargo", "test", "-p", "agentmage-host", "index_publication",
            "--locked",
        ),
    ),
    (
        "native-write-producer-privacy-tests",
        (
            "cargo", "test", "-p", "agentmage-host",
            "story_39_1_native_write_producer_gate_is_bounded_and_catches_split_canaries",
            "--locked",
        ),
    ),
    (
        "native-atomic-write-internal-stop-matrix",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "s_029_rt01_process_stops_leave_only_reviewed_target_bytes", "--locked",
        ),
    ),
    (
        "native-filesystem-internal-stop-matrix",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "s_030_rt01_process_stops_leave_only_prestate_or_poststate_paths", "--locked",
        ),
    ),
    (
        "native-derived-index-process-stop-matrix",
        (
            "cargo", "test", "-p", "agentmage-host",
            "native_process_stops_rebuild_index_only_from_canonical_markdown", "--locked",
        ),
    ),
    (
        "native-atomic-write-race-matrix",
        (
            "cargo", "test", "-p", "agentmage-platform-linux", "s_029_st01", "--locked",
        ),
    ),
    (
        "native-filesystem-race-matrix",
        (
            "cargo", "test", "-p", "agentmage-platform-linux", "s_030_st01", "--locked",
        ),
    ),
    (
        "native-filesystem-restore-race-matrix",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "s_030_rt01_copy_restore_parent_renames_leave_no_transaction_effect", "--locked",
        ),
    ),
    (
        "kernel-tests",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--locked"),
    ),
    (
        "kernel-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
            "--locked", "--", "-D", "warnings",
        ),
    ),
    ("runtime-schema-tests", ("node", "--test", "tests/test_planning_schemas.mjs")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("dependency-classes", ("python3", "scripts/dependency_classes.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_39_evidence")),
)
FOCUSED_COMMANDS: Final = (
    "write-recovery-unit-tests",
    "write-recovery-integration-tests",
    "native-write-checkpoint-process-test",
    "native-derived-index-checkpoint-tests",
    "native-write-producer-privacy-tests",
    "native-atomic-write-internal-stop-matrix",
    "native-filesystem-internal-stop-matrix",
    "native-derived-index-process-stop-matrix",
    "native-atomic-write-race-matrix",
    "native-filesystem-race-matrix",
    "native-filesystem-restore-race-matrix",
)
SECURITY_REQUIREMENTS: Final = [
    "SR-DAT-002", "SR-DAT-003", "SR-DAT-004", "SR-DAT-010", "SR-DAT-011",
    "SR-DAT-012", "SR-OPS-001", "SR-OPS-002", "SR-OPS-003", "SR-OPS-004",
    "SR-OPS-005", "SR-OPS-006", "SR-OPS-007", "SR-TST-005",
]
CORPUS_CLASSES: Final = {
    "cancellation": 1,
    "cleanup": 1,
    "conflict": 2,
    "protected-storage": 1,
    "receipt-recovery": 1,
    "uncertain": 4,
    "workspace-drift": 2,
}
IMPLEMENTED: Final = {
    "authority_free_recovery_coordinator": True,
    "fifteen_phase_checkpoint_contract": True,
    "hash_chained_checkpoint_validation": True,
    "formal_runtime_checkpoint_schema": True,
    "eleven_boundary_privacy_gate": True,
    "removed_content_not_retained": True,
    "deterministic_recovery_precedence": True,
    "completed_write_replay_allowed": False,
    "concurrent_change_auto_merge": False,
    "content_free_staging_diagnostics": True,
    "separately_receipted_cleanup": True,
    "redacted_human_audit": True,
    "native_end_to_end_recovery_wiring": True,
    "native_sqlcipher_checkpoint_journal": True,
    "native_write_process_stop_matrix": True,
    "native_derived_index_checkpoint_publication": True,
    "native_write_producer_privacy_gate": True,
    "native_internal_process_stop_matrices": True,
    "complete_native_crash_concurrency_matrix": True,
    "physical_enospc_executed": False,
    "all_runtime_roots_scanned": False,
    "non_fedora_native_evidence": False,
    "independent_review": False,
    "manual_fuzzing_complete": False,
}
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-38-BLOCKED", "owner": "39.1"},
    {"code": "SPRINT-39-ALL-RUNTIME-ROOT-SCAN-ABSENT", "owner": "39.1.3.4"},
    {"code": "TRUSTED-PACKAGE-LAUNCHER-ENVIRONMENT-ABSENT", "owner": "39.1.3.5"},
    {"code": "SPRINT-39-NON-FEDORA-EVIDENCE-ABSENT", "owner": "39.1.3.5"},
    {"code": "INDEPENDENT-SPRINT-39-REVIEW-ABSENT", "owner": "39.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-005"},
]


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 39 source is absent: {path}")
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
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "rustc": version("rustc", "--version"),
        "cargo": version("cargo", "--version"),
        "node": version("node", "--version"),
        "npm": version("npm", "--version"),
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
        if identifier in FOCUSED_COMMANDS:
            matches = IGNORED_TESTS.findall(output)
            blocking_skip_count = sum(int(value) for value in matches) if matches else -1
        records.append({
            "id": identifier,
            "argv": list(argv),
            "exit_code": code,
            "output_sha256": digest(output),
            "blocking_skip_count": blocking_skip_count,
        })
    return records


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    focused = [item for item in commands if item["id"] in FOCUSED_COMMANDS]
    local_pass = (
        all(item["exit_code"] == 0 for item in commands)
        and len(focused) == len(FOCUSED_COMMANDS)
        and all(item.get("blocking_skip_count") == 0 for item in focused)
    )
    return {
        "schema_version": 1,
        "record_type": "sprint_39_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "focused_contracts": local_pass,
            "focused_blocking_skip_count": 0 if local_pass else None,
            "upstream_sprint_38_gate": False,
            "native_end_to_end_recovery": True,
            "native_derived_index_recovery": True,
            "native_write_producer_privacy": True,
            "native_internal_process_stop_matrices": True,
            "complete_native_crash_concurrency_matrix": True,
            "physical_enospc_executed": False,
            "all_runtime_roots_scanned": False,
            "trusted_package_launcher_environment": False,
            "non_fedora_native_evidence": False,
            "independent_review": False,
            "manual_fuzzing": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_write_recovery_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_dependency_passed": False,
            "native_end_to_end_passed": True,
            "native_derived_index_recovery_passed": True,
            "native_write_producer_privacy_passed": True,
            "native_internal_process_stops_passed": True,
            "native_crash_concurrency_passed": True,
            "physical_enospc_executed": False,
            "all_runtime_roots_scanned": False,
            "trusted_launcher_environment_passed": False,
            "cross_platform_evidence_passed": False,
            "independent_review_passed": False,
            "manual_fuzzing_complete": False,
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
    for identifier in FOCUSED_COMMANDS:
        focused = next((item for item in commands if item.get("id") == identifier), None)
        if focused is None or focused.get("blocking_skip_count") != 0:
            failures.append(f"focused skipped, suppressed, or unavailable check: {identifier}")
    if any(
        item.get("blocking_skip_count") is not None
        for item in commands
        if item.get("id") not in FOCUSED_COMMANDS
    ):
        failures.append("supporting command skip count must remain not-applicable")
    expected_summary = {
        "local_write_recovery_contract_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_dependency_passed": False,
        "native_end_to_end_passed": True,
        "native_derived_index_recovery_passed": True,
        "native_write_producer_privacy_passed": True,
        "native_internal_process_stops_passed": True,
        "native_crash_concurrency_passed": True,
        "physical_enospc_executed": False,
        "all_runtime_roots_scanned": False,
        "trusted_launcher_environment_passed": False,
        "cross_platform_evidence_passed": False,
        "independent_review_passed": False,
        "manual_fuzzing_complete": False,
        "network_access_enabled": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary or negative claim drift")
    verification = report.get("verification_evidence", {})
    if verification != {
        "focused_contracts": True,
        "focused_blocking_skip_count": 0,
        "upstream_sprint_38_gate": False,
        "native_end_to_end_recovery": True,
        "native_derived_index_recovery": True,
        "native_write_producer_privacy": True,
        "native_internal_process_stop_matrices": True,
        "complete_native_crash_concurrency_matrix": True,
        "physical_enospc_executed": False,
        "all_runtime_roots_scanned": False,
        "trusted_package_launcher_environment": False,
        "non_fedora_native_evidence": False,
        "independent_review": False,
        "manual_fuzzing": False,
    }:
        failures.append("verification boundary drift")
    source = report.get("source_sha256", {})
    if set(source) != set(SOURCE_PATHS) or any(
        not SHA256.fullmatch(str(value)) for value in source.values()
    ):
        failures.append("source inventory invalid")
    if verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            try:
                if source.get(path) != digest(git_file(revision, path)):
                    failures.append(f"source hash drift: {path}")
            except ValueError as error:
                failures.append(str(error))
    try:
        corpus = json.loads(git_file(revision, CORPUS_PATH))
        classes: dict[str, int] = {}
        for case in corpus.get("cases", []):
            category = case.get("class")
            classes[category] = classes.get(category, 0) + 1
            if case.get("repeat_completed_write") is not False:
                failures.append("recovery corpus permits replay")
        if classes != CORPUS_CLASSES or corpus.get("privacy") != "public-synthetic-only":
            failures.append("recovery corpus drift")
    except (ValueError, json.JSONDecodeError):
        failures.append("recovery corpus unavailable")
    try:
        audit_bytes = git_file(revision, AUDIT_PATH)
        audit = json.loads(audit_bytes)
        if (
            audit.get("redacted_fields") != 1
            or "[REDACTED]" not in audit.get("report", "")
            or any(key in audit_bytes.lower() for key in (b"password=", b"authorization:"))
        ):
            failures.append("redacted audit fixture drift")
    except (ValueError, json.JSONDecodeError):
        failures.append("redacted audit fixture unavailable")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--revision")
    parser.add_argument("--verify", type=Path)
    arguments = parser.parse_args()
    if arguments.verify:
        report = json.loads(arguments.verify.read_text(encoding="utf-8"))
        failures = validate_report(report)
        if failures:
            print("Sprint 39 evidence validation failed:")
            for failure in failures:
                print(f"- {failure}")
            return 1
        print("Sprint 39 evidence validated")
        return 0
    if not arguments.record or not arguments.revision:
        parser.error("--record requires --revision")
    report = build_report(arguments.revision, run_commands())
    failures = validate_report(report)
    if failures:
        print("Sprint 39 evidence recording failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(OUTPUT.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
