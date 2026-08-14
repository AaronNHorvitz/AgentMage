#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 37 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-37/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
IGNORED_TESTS: Final = re.compile(
    rb"test result: (?:ok|FAILED)\. \d+ passed; \d+ failed; (\d+) ignored;"
)
SOURCE_PATHS: Final = (
    "kernel/engine/src/filesystem_control.rs",
    "platforms/linux/src/filesystem_control.rs",
    "platforms/linux/src/write_transaction.rs",
    "platforms/linux/src/lib.rs",
    "docs/architecture/controlled-filesystem-mutations.md",
    "docs/verification/sprint-37-local-results.md",
    "docs/verification/sprint-37-protected-path-corpus.json",
    "scripts/sprint_37_evidence.py",
    "tests/test_sprint_37_evidence.py",
)
COMMANDS: Final = (
    (
        "kernel-filesystem-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "filesystem_control", "--locked",
        ),
    ),
    (
        "linux-filesystem-tests",
        (
            "cargo", "test", "-p", "agentmage-platform-linux",
            "filesystem_control", "--locked",
        ),
    ),
    (
        "kernel-linux-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine", "-p",
            "agentmage-platform-linux", "--locked",
        ),
    ),
    (
        "kernel-linux-clippy",
        (
            "cargo", "clippy", "-p", "agentmage-kernel-engine", "-p",
            "agentmage-platform-linux", "--all-targets", "--locked", "--",
            "-D", "warnings",
        ),
    ),
    ("product-gate", ("npm", "run", "product:check")),
    ("documentation-gate", ("npm", "run", "docs:check")),
    ("module-inventory", ("python3", "scripts/module_inventory.py")),
    ("dependency-rules", ("python3", "scripts/dependency_rules.py")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("build-contract", ("python3", "scripts/build_contract.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_37_evidence")),
)
FOCUSED_COMMANDS: Final = ("kernel-filesystem-tests", "linux-filesystem-tests")
SECURITY_REQUIREMENTS: Final = [
    "SR-PLT-004",
    "SR-ACC-002",
    "SR-ACC-003",
    "SR-ACC-004",
    "SR-ACC-005",
    "SR-ACC-006",
    "SR-OPS-001",
    "SR-TST-004",
    "SR-TST-005",
]
BLOCKERS: Final = [
    {"code": "UPSTREAM-SPRINT-36-BLOCKED", "owner": "36.1"},
    {"code": "SPRINT-37-NATIVE-RACE-MATRIX-INCOMPLETE", "owner": "37.1.3.3"},
    {"code": "SPRINT-37-DISKFULL-PROCESS-DEATH-MATRIX-INCOMPLETE", "owner": "37.1.3.4"},
    {"code": "SPRINT-37-NON-FEDORA-NATIVE-EVIDENCE-ABSENT", "owner": "37.1.AC2"},
    {"code": "SPRINT-37-ISOLATED-WRITE-WORKER-PROOF-ABSENT", "owner": "37.AC5"},
    {"code": "INDEPENDENT-SPRINT-37-REVIEW-ABSENT", "owner": "37.1.3.5"},
    {"code": "MANUAL-FUZZING-DEFERRED", "owner": "SR-TST-004"},
]
IMPLEMENTED: Final = {
    "authority_free_operation_plans": True,
    "closed_structured_patch_parser": True,
    "operation_specific_exact_previews": True,
    "write_and_delete_grant_separation": True,
    "separate_high_risk_delete_confirmation": True,
    "single_use_grant_consumption": True,
    "fresh_source_parent_sibling_destination_validation": True,
    "create_patch_copy_move_trash_operations": True,
    "hash_chained_operation_receipts": True,
    "known_partial_restoration": True,
    "terminal_uncertain_no_replay": True,
    "cancellation_before_consumption": True,
    "protected_path_and_unrelated_work_denial": True,
    "linux_native_driver": True,
    "fedora_native_fixture_evidence": True,
    "ubuntu_native_fixture_evidence": False,
    "macos_native_driver": False,
    "windows_native_driver": False,
    "isolated_write_worker_proven": False,
    "complete_native_race_matrix": False,
    "complete_crash_durability_matrix": False,
    "independent_review": False,
    "manual_fuzzing_complete": False,
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
        raise ValueError(f"committed Sprint 37 source is absent: {path}")
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
        "record_type": "sprint_37_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "environment": environment_manifest(),
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "kernel_operation_matrix": local_pass,
            "fedora_native_operation_matrix": local_pass,
            "focused_blocking_skip_count": 0 if local_pass else None,
            "upstream_sprint_36_gate": False,
            "complete_native_race_matrix": False,
            "complete_crash_durability_matrix": False,
            "non_fedora_native_evidence": False,
            "isolated_write_worker_proof": False,
            "independent_review": False,
            "manual_fuzzing": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_filesystem_contract_passed": local_pass,
            "fedora_native_driver_passed": local_pass,
            "sprint_status": "BLOCKED",
            "upstream_dependency_passed": False,
            "cross_platform_evidence_passed": False,
            "native_race_and_crash_evidence_passed": False,
            "isolated_write_worker_proven": False,
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
        "local_filesystem_contract_passed": True,
        "fedora_native_driver_passed": True,
        "sprint_status": "BLOCKED",
        "upstream_dependency_passed": False,
        "cross_platform_evidence_passed": False,
        "native_race_and_crash_evidence_passed": False,
        "isolated_write_worker_proven": False,
        "independent_review_passed": False,
        "manual_fuzzing_complete": False,
        "network_access_enabled": False,
        "release_approval": False,
    }
    if report.get("summary") != expected_summary:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    if verification.get("focused_blocking_skip_count") != 0:
        failures.append("focused blocking skip summary invalid")
    for field in (
        "upstream_sprint_36_gate",
        "complete_native_race_matrix",
        "complete_crash_durability_matrix",
        "non_fedora_native_evidence",
        "isolated_write_worker_proof",
        "independent_review",
        "manual_fuzzing",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    for field in (
        "ubuntu_native_fixture_evidence",
        "macos_native_driver",
        "windows_native_driver",
        "isolated_write_worker_proven",
        "complete_native_race_matrix",
        "complete_crash_durability_matrix",
        "independent_review",
        "manual_fuzzing_complete",
        "generic_shell",
        "network_access",
        "external_delivery",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"capability overclaim: {field}")
    environment = report.get("environment", {})
    if set(environment) != {
        "system", "release", "machine", "python", "rustc", "cargo", "node", "npm",
    }:
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
    corpus = json.loads(
        git_file(revision, "docs/verification/sprint-37-protected-path-corpus.json")
    ) if REVISION.fullmatch(revision) else {}
    cases = corpus.get("cases", []) if isinstance(corpus, dict) else []
    if len(cases) != 18 or len({item.get("id") for item in cases}) != 18:
        failures.append("protected-path corpus drift")
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
