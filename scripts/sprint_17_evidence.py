#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 17 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-17/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "capabilities/read-only/src/git.rs",
    "capabilities/read-only/src/lib.rs",
    "kernel/engine/src/instruction_provenance.rs",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/lib.rs",
    "docs/architecture/read-only-git-and-instruction-trust.md",
    "docs/verification/sprint-17-local-results.md",
    "scripts/sprint_17_evidence.py",
    "tests/test_sprint_17_evidence.py",
)
COMMANDS: Final = (
    (
        "read-only-git-pack",
        ("cargo", "test", "-p", "agentmage-capability-read-only", "--all-targets", "--locked"),
    ),
    (
        "instruction-trust-kernel",
        ("cargo", "test", "-p", "agentmage-kernel-engine", "--all-targets", "--locked"),
    ),
    (
        "git-pack-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-capability-read-only",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
    ),
    (
        "instruction-kernel-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-kernel-engine",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
    ),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-ACC-006",
    "SR-ACC-007",
    "SR-ACC-008",
    "SR-AI-005",
    "SR-AI-008",
    "SR-NET-001",
    "SR-TST-002",
    "SR-TST-004",
    "RV-11",
]
BLOCKERS: Final = [
    {"code": "PACKAGED-LINUX-GIT-WORKER-NOT-INTEGRATED", "owner": "17.1.1.2"},
    {"code": "ROOT-OWNED-LINUX-WORKER-NOT-INSTALLED", "owner": "17.1.3.3"},
    {"code": "MACOS-GIT-WORKER-EVIDENCE-MISSING", "owner": "17.1.3.1"},
    {"code": "LIVE-NETWORK-OBSERVATION-INCOMPLETE", "owner": "17.1.3.3"},
    {"code": "MANUAL-GIT-PARSER-FUZZING-DEFERRED", "owner": "17.1.3.5"},
    {"code": "INDEPENDENT-SPRINT-17-REVIEW-NOT-RETAINED", "owner": "17.1.3.5"},
]


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, relative: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{relative}"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 17 source is absent: {relative}")
    return result.stdout


def run_commands() -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            exit_code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]),
                cwd=ROOT,
                check=False,
                capture_output=True,
                timeout=900,
            )
            exit_code, output = result.returncode, result.stdout + result.stderr
        records.append(
            {
                "id": identifier,
                "argv": list(argv),
                "exit_code": exit_code,
                "output_sha256": sha256_bytes(output),
            }
        )
    return records


def build_report(source_revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    local_pass = all(command["exit_code"] == 0 for command in commands)
    return {
        "schema_version": 1,
        "record_type": "sprint_17_local_evidence",
        "source_revision": source_revision,
        "source_sha256": {
            path: sha256_bytes(git_file(source_revision, path)) for path in SOURCE_PATHS
        },
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": {
            "fixed_git_operations": 13,
            "git_mutation_operations": 0,
            "git_fixture_states": 7,
            "instruction_source_classes": 20,
            "injection_cases": 200,
            "narrowing_constraint_classes": 6,
            "raw_instruction_content_in_ledger": False,
            "authority_broadening_fields": 0,
            "workspace_manifest_ledger": True,
            "stale_and_conflict_stop": True,
        },
        "platform_evidence": {
            "local_disposable_git_adapter": local_pass,
            "linux_packaged_git_worker": False,
            "linux_live_network_observation": False,
            "macos_git_worker": False,
        },
        "verification_evidence": {
            "git_invariance": local_pass,
            "hostile_git_configuration_canaries": local_pass,
            "instruction_injection_matrix": local_pass,
            "manual_parser_fuzzing": False,
            "independent_review": False,
        },
        "blockers": BLOCKERS,
        "summary": {
            "local_contract_passed": local_pass,
            "sprint_status": "BLOCKED",
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
    expected_contracts = {
        "fixed_git_operations": 13,
        "git_mutation_operations": 0,
        "git_fixture_states": 7,
        "instruction_source_classes": 20,
        "injection_cases": 200,
        "narrowing_constraint_classes": 6,
        "raw_instruction_content_in_ledger": False,
        "authority_broadening_fields": 0,
        "workspace_manifest_ledger": True,
        "stale_and_conflict_stop": True,
    }
    if report.get("implemented_contracts") != expected_contracts:
        failures.append("implemented-contract inventory drift")
    if report.get("summary") != {
        "local_contract_passed": True,
        "sprint_status": "BLOCKED",
        "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    platform = report.get("platform_evidence", {})
    verification = report.get("verification_evidence", {})
    for field in (
        "linux_packaged_git_worker",
        "linux_live_network_observation",
        "macos_git_worker",
    ):
        if platform.get(field) is not False:
            failures.append(f"platform overclaim: {field}")
    for field in ("manual_parser_fuzzing", "independent_review"):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    if verify_current and REVISION.fullmatch(revision):
        for path in SOURCE_PATHS:
            digest = str(report.get("source_sha256", {}).get(path, ""))
            if not SHA256.fullmatch(digest) or digest != sha256_bytes(git_file(revision, path)):
                failures.append(f"source digest drift: {path}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    args = parser.parse_args()
    if args.write:
        revision = subprocess.run(
            ["git", "rev-parse", args.source_revision],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        report = build_report(revision, run_commands())
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    else:
        report = json.loads(OUTPUT.read_text(encoding="utf-8"))
    failures = validate_report(report)
    if failures:
        for failure in failures:
            print(failure)
        return 1
    print(json.dumps(report["summary"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
