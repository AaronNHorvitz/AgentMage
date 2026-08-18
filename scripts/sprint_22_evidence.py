#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 22 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-22/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "docs/architecture/context-and-crash-safe-resume.md",
    "docs/verification/sprint-22-local-results.md",
    "kernel/contracts/src/context.rs",
    "kernel/engine/migrations/operational-store/0004-session-checkpoints.sql",
    "kernel/engine/src/authority_transaction.rs",
    "kernel/engine/src/context_management.rs",
    "kernel/engine/src/operational_store.rs",
    "shells/host/src/linux_coding_runtime.rs",
    "scripts/sprint_22_evidence.py",
    "scripts/story_22_1_long_resume_evidence.py",
    "scripts/story_22_1_native_resume_evidence.py",
    "tests/test_sprint_22_evidence.py",
    "tests/test_story_22_1_long_resume_evidence.py",
    "tests/test_story_22_1_native_resume_evidence.py",
)
COMMANDS: Final = (
    ("context-tests", ("cargo", "test", "-p", "agentmage-kernel-engine",
                       "context_management::tests", "--locked")),
    ("checkpoint-tests", ("cargo", "test", "-p", "agentmage-kernel-engine",
                          "checkpoint", "--locked")),
    ("crash-campaign", ("cargo", "test", "-p", "agentmage-kernel-engine",
                        "seeded_crash_recovery_campaign_never_repeats_a_completed_transition",
                        "--locked")),
    ("native-resume-evidence", ("python3", "scripts/story_22_1_native_resume_evidence.py")),
    ("long-resume-evidence", ("python3", "scripts/story_22_1_long_resume_evidence.py")),
    ("engine-clippy", ("cargo", "clippy", "-p", "agentmage-kernel-engine",
                       "--all-targets", "--locked", "--", "-D", "warnings")),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_22_evidence")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-DAT-002", "SR-DAT-003", "SR-AI-008", "SR-AI-009", "SR-AI-010",
    "SR-OPS-003", "SR-TST-005", "SR-TST-006",
]
BLOCKERS: Final = [
    {"code": "PRODUCTION-CONTEXT-PIPELINE-NOT-INTEGRATED", "owner": "22.1.3.1"},
    {"code": "PRODUCT-WIDE-CANARY-SWEEP-NOT-RETAINED", "owner": "22.1.3.5"},
    {"code": "INDEPENDENT-SPRINT-22-REVIEW-NOT-RETAINED", "owner": "22.1.3.5"},
]
IMPLEMENTED: Final = {
    "context_item_kinds": 11,
    "resume_drift_dimensions": 9,
    "explicit_drift_decisions": 3,
    "subprocess_crash_runs": 126,
    "native_tool_terminal_resume_runs": 100,
    "native_long_session_checkpoints": 7,
    "native_long_session_drift_classes": 7,
    "authoritative_evidence_first_dedupe": True,
    "complete_content_free_accounting": True,
    "checked_summary_source_separation": True,
    "encrypted_atomic_checkpoint_publication": True,
    "terminal_receipt_checkpoint_binding": True,
    "ephemeral_checkpoint_persistence": False,
    "ambient_filesystem_access": False,
    "network_authority": False,
    "resume_authority": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 22 source is absent: {path}")
    return result.stdout


def run_commands() -> list[dict[str, Any]]:
    records = []
    for identifier, argv in COMMANDS:
        executable = shutil.which(argv[0])
        if executable is None:
            code, output = 127, b"executable-unavailable"
        else:
            result = subprocess.run(
                (executable, *argv[1:]), cwd=ROOT, check=False,
                capture_output=True, timeout=900,
            )
            code, output = result.returncode, result.stdout + result.stderr
        records.append({
            "id": identifier, "argv": list(argv), "exit_code": code,
            "output_sha256": digest(output),
        })
    return records


def build_report(revision: str, commands: list[dict[str, Any]]) -> dict[str, Any]:
    local_pass = all(item["exit_code"] == 0 for item in commands)
    return {
        "schema_version": 1,
        "record_type": "sprint_22_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "context_composition_matrix": local_pass,
            "checked_summary_matrix": local_pass,
            "checkpoint_schema_and_tamper_matrix": local_pass,
            "atomic_terminal_publication": local_pass,
            "subprocess_crash_campaign": local_pass,
            "drift_decision_matrix": local_pass,
            "ephemeral_no_persistence": local_pass,
            "production_context_integration": False,
            "native_platform_crash_matrix": local_pass,
            "native_long_session_resume": local_pass,
            "product_wide_canary_sweep": False,
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
    if report.get("implemented_contracts") != IMPLEMENTED:
        failures.append("implemented contract drift")
    if report.get("blockers") != BLOCKERS:
        failures.append("blocker drift")
    commands = report.get("commands", [])
    if [item.get("id") for item in commands] != [item[0] for item in COMMANDS]:
        failures.append("command inventory drift")
    if any(item.get("exit_code") != 0 or not SHA256.fullmatch(
            str(item.get("output_sha256", ""))) for item in commands):
        failures.append("command result invalid")
    if report.get("summary") != {
        "local_contract_passed": True, "sprint_status": "BLOCKED", "release_approval": False,
    }:
        failures.append("summary overclaim or local failure")
    verification = report.get("verification_evidence", {})
    for field in (
        "production_context_integration", "product_wide_canary_sweep", "independent_review",
    ):
        if verification.get(field) is not False:
            failures.append(f"verification overclaim: {field}")
    if verification.get("native_platform_crash_matrix") is not True:
        failures.append("native platform crash matrix missing")
    if verification.get("native_long_session_resume") is not True:
        failures.append("native long-session resume missing")
    for field in (
        "ephemeral_checkpoint_persistence", "ambient_filesystem_access",
        "network_authority", "resume_authority",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"authority or persistence overclaim: {field}")
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
        OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
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
