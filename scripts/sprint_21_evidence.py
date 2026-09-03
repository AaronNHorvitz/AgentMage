#!/usr/bin/env python3
"""Build and validate truthful locally executable Sprint 21 evidence."""

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
OUTPUT: Final = ROOT / "artifacts/sprints/sprint-21/local-evidence-report.json"
REVISION: Final = re.compile(r"^[0-9a-f]{40}$")
SHA256: Final = re.compile(r"^[0-9a-f]{64}$")
SOURCE_PATHS: Final = (
    "docs/architecture/citation-and-receipt-integrity.md",
    "docs/verification/sprint-21-local-results.md",
    "kernel/engine/src/authority.rs",
    "kernel/engine/src/evidence_reconciliation.rs",
    "kernel/engine/src/evidence_store.rs",
    "kernel/engine/src/evidence_state.rs",
    "kernel/engine/src/lib.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/runtime_answer.rs",
    "kernel/engine/migrations/operational-store/0009-evidence-integrity.sql",
    "shells/host/src/linux_evidence_reconciliation.rs",
    "scripts/sprint_21_evidence.py",
    "tests/test_sprint_21_evidence.py",
)
COMMANDS: Final = (
    (
        "reconciliation-tests",
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "evidence_reconciliation::tests",
            "--locked",
        ),
    ),
    (
        "held-linux-citation-tests",
        (
            "cargo", "test", "-p", "agentmage-host",
            "linux_evidence_reconciliation", "--locked",
        ),
    ),
    (
        "encrypted-evidence-store-tests",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "evidence_store", "--locked",
        ),
    ),
    (
        "runtime-answer-ledger-test",
        (
            "cargo", "test", "-p", "agentmage-kernel-engine",
            "story_23_4_direct_answer_is_verifier_backed_and_streamed_in_exact_order",
            "--locked",
        ),
    ),
    (
        "evidence-host-clippy",
        (
            "cargo",
            "clippy",
            "-p",
            "agentmage-kernel-engine",
            "-p",
            "agentmage-host",
            "--all-targets",
            "--locked",
            "--",
            "-D",
            "warnings",
        ),
    ),
    ("evidence-tests", ("python3", "-m", "unittest", "tests.test_sprint_21_evidence")),
    ("effect-boundary", ("python3", "scripts/effect_boundary.py")),
    ("strict-local-source", ("python3", "scripts/strict_local_source_audit.py")),
    ("supply-chain", ("python3", "scripts/supply_chain.py")),
)
SECURITY_REQUIREMENTS: Final = [
    "SR-AI-003", "SR-AI-007", "SR-AI-010", "SR-AI-011",
    "SR-OPS-001", "SR-OPS-002", "SR-OPS-003", "SR-OPS-004", "SR-OPS-005",
    "SR-TST-010",
]
BLOCKERS: Final = [
    {"code": "INDEPENDENT-SPRINT-21-REVIEW-NOT-RETAINED", "owner": "21.1.3.5"},
]
IMPLEMENTED: Final = {
    "citation_freshness_states": 3,
    "exact_source_identity_dimensions": 6,
    "complete_answer_claim_coverage": True,
    "safe_content_free_projection": True,
    "claim_level_audit_rendering": True,
    "receipt_chain": True,
    "external_hmac_sha256_anchor": True,
    "authority_owned_receipt_checkpoint": True,
    "encrypted_answer_ledger_restart": True,
    "encrypted_durable_anchor_history": True,
    "held_linux_citation_resolution": True,
    "held_source_byte_recomputation": True,
    "runtime_answer_ledger_composition": True,
    "clock_regression_rejected": True,
    "integrity_key_stored_in_ledger": False,
    "ambient_filesystem_access": False,
    "network_authority": False,
    "completion_authority": False,
}


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def git_file(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, check=False,
        capture_output=True, timeout=30,
    )
    if result.returncode != 0:
        raise ValueError(f"committed Sprint 21 source is absent: {path}")
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
        "record_type": "sprint_21_local_evidence",
        "source_revision": revision,
        "source_sha256": {path: digest(git_file(revision, path)) for path in SOURCE_PATHS},
        "commands": commands,
        "security_requirement_ids": SECURITY_REQUIREMENTS,
        "implemented_contracts": IMPLEMENTED,
        "verification_evidence": {
            "citation_resolution_matrix": local_pass,
            "answer_ledger_matrix": local_pass,
            "receipt_tamper_matrix": local_pass,
            "safe_projection_scan": local_pass,
            "production_held_file_integration": local_pass,
            "durable_anchor_integration": local_pass,
            "clock_anomaly_matrix": local_pass,
            "native_linux_source_execution": local_pass,
            "runtime_answer_integration": local_pass,
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
        "production_held_file_integration", "durable_anchor_integration",
        "clock_anomaly_matrix", "native_linux_source_execution",
        "runtime_answer_integration",
    ):
        if verification.get(field) is not True:
            failures.append(f"verified integration drift: {field}")
    if verification.get("independent_review") is not False:
        failures.append("verification overclaim: independent_review")
    for field in (
        "integrity_key_stored_in_ledger", "ambient_filesystem_access",
        "network_authority", "completion_authority",
    ):
        if report.get("implemented_contracts", {}).get(field) is not False:
            failures.append(f"authority or key overclaim: {field}")
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
