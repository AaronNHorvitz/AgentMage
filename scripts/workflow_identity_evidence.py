#!/usr/bin/env python3
"""Build and validate retained evidence for workflow execution identity freshness."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-5/story-5.3"
RAW_PATH: Final = EVIDENCE_DIR / "workflow-identity-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-identity-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "workflow_identity",
        "--locked",
    ),
    (
        "cargo",
        "clippy",
        "-p",
        "agentmage-kernel-engine",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ),
)
MARKERS: Final = (
    "issues_the_complete_identity_chain_without_execution_authority ... ok",
    "every_pre_effect_identity_is_globally_fresh_and_failed_issue_is_atomic ... ok",
    "ordinals_predecessors_and_terminal_order_are_exact ... ok",
    "automatic_retry_is_forbidden_for_every_high_or_unknown_effect ... ok",
    "uncertain_effects_cannot_open_a_successor_until_safely_reconciled ... ok",
    "approvals_receipts_and_verifications_are_fresh_single_issue_identities ... ok",
    "policy_tampering_cannot_enable_a_retry ... ok",
    "required_once_approval_is_fresh_initially_and_not_reissued_automatically ... ok",
    "concurrent_duplicate_issue_has_exactly_one_winner ... ok",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "identity_authority_only": False,
    "capability_grant_issued": False,
    "runtime_effect_executed": False,
    "network_calls": 0,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-workflow-identity-evidence",
        "story_id": "5.3",
        "task_id": "5.3.1.2",
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "requirements": ["AM-WKF-001", "AM-VER-001", "AT-WKF-001", "AT-VER-001"],
        "identity_families": [
            "call",
            "tool_call",
            "attempt",
            "grant",
            "approval",
            "receipt",
            "verification",
        ],
        "issuance_contract": {
            "workflow_scoped_synchronized_ledger": True,
            "all_identity_roles_globally_disjoint": True,
            "failed_issue_is_atomic": True,
            "attempt_chain_exact": True,
            "prior_attempt_must_be_terminal": True,
            "one_receipt_per_attempt": True,
            "one_verification_per_verifier_policy": True,
            "identity_proof_contains_execution_authority": False,
        },
        "retry_contract": {
            "automatic_retry_forbidden_effects": [
                "non_idempotent",
                "destructive",
                "external",
                "unknown",
            ],
            "uncertain_effect_automatic_retry": False,
            "uncertain_effect_requires_safe_reconciliation_and_user_approval": True,
            "conditional_retry_requires_safe_reconciliation": True,
            "required_per_attempt_approval_is_fresh": True,
            "required_once_approval_is_initial_only": True,
            "policy_digest_tampering_admitted": False,
        },
        "race_contract": {
            "contenders": 16,
            "successful_identity_reservations": 1,
            "duplicate_identity_reservations": 0,
        },
        "test_contract": {
            "focused_tests": len(MARKERS),
            "required_markers": list(MARKERS),
            "clippy_warnings_allowed": 0,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [
            artifact("kernel/engine/src/workflow_identity.rs"),
            artifact("kernel/engine/tests/workflow_identity.rs"),
            artifact("docs/verification/story-5-3-workflow-identity-evidence.md"),
            artifact("scripts/workflow_identity_evidence.py"),
            artifact("tests/test_workflow_identity_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if value != expected_report():
        failures.append("workflow identity report is stale, incomplete, reordered, or widened")
    if isinstance(value, dict) and value.get("product_truth") != TRUTH:
        failures.append("workflow identity product truth was widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        outputs: list[str] = []
        for command in COMMANDS:
            result = subprocess.run(
                command,
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                check=False,
            )
            outputs.append(f"$ {' '.join(command)}\n{result.stdout.rstrip(chr(10))}")
            if result.returncode != 0:
                sys.stderr.write(outputs[-1])
                return 1
        raw = "\n".join(outputs) + "\n"
        failures = validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"workflow identity evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow identity evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate(report)
    if failures:
        for failure in failures:
            print(f"workflow identity evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Workflow identity evidence validated through Sub-task 5.3.1.2")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
