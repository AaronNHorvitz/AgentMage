#!/usr/bin/env python3
"""Build and validate the Story 5.3 adversarial workflow campaign."""

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
RAW_PATH: Final = EVIDENCE_DIR / "workflow-adversarial-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-adversarial-report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_definition", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_identity", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "retry_admission", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_verifier", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "tool_call_repair::tests", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "workflow_progress::tests", "--locked"),
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
ATTACK_MARKERS: Final = {
    "missing_or_stale_preflight": [
        "preflight_postcondition_and_policy_integrity_mutations_are_denied ... ok",
        "every_preflight_and_reconciliation_field_mutation_denies_before_dispatch ... ok",
    ],
    "malformed_call": [
        "normalization_is_ordered_bounded_and_does_not_invent_values ... ok",
        "model_repair_is_single_profile_bound_and_never_an_effect_attempt ... ok",
    ],
    "approval_bypass": [
        "approvals_receipts_and_verifications_are_fresh_single_issue_identities ... ok",
        "every_approval_field_mutation_denies_before_dispatch ... ok",
    ],
    "grant_reuse": [
        "every_pre_effect_identity_is_globally_fresh_and_failed_issue_is_atomic ... ok",
        "complete_prior_use_ledger_denies_every_replayed_identity_and_any_receipt ... ok",
    ],
    "retry_loop": [
        "non_adjacent_repeated_state_stops_at_the_exact_policy_limit ... ok",
        "unsafe_and_uncertain_effects_never_receive_automatic_new_attempts ... ok",
    ],
    "false_completion": [
        "exit_zero_persuasive_output_and_tampered_results_never_establish_completion ... ok",
    ],
    "uncertain_effect": [
        "uncertain_effects_cannot_open_a_successor_until_safely_reconciled ... ok",
        "racing_eligible_attempts_execute_once_and_uncertainty_is_sticky ... ok",
    ],
    "contradictory_evidence": [
        "every_postcondition_invariant_and_prohibited_effect_is_fail_closed ... ok",
        "expected_output_state_observation_receipt_and_evidence_bind_independently ... ok",
    ],
}
TRUTH: Final = {
    "synthetic_data_only": True,
    "model_inference_executed": False,
    "runtime_effect_executed": False,
    "network_calls": 0,
    "native_platform_campaign": False,
    "rv52_completion_claim": False,
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
        "record_type": "agentmage-workflow-adversarial-campaign",
        "story_id": "5.3",
        "task_id": "5.3.3.1",
        "generated_on": "2026-08-30",
        "status": "pass-local-synthetic-campaign",
        "requirements": ["AM-WKF-001", "AM-VER-001", "AT-WKF-001", "AT-VER-001"],
        "attack_families": [
            {
                "attack": attack,
                "required_markers": markers,
                "admitted_dispatches": 0,
                "admitted_false_successes": 0,
            }
            for attack, markers in ATTACK_MARKERS.items()
        ],
        "campaign_contract": {
            "attack_family_count": len(ATTACK_MARKERS),
            "all_attacks_are_synthetic": True,
            "all_boundaries_execute_deterministically": True,
            "denied_candidates_mutate_authority": False,
            "denied_candidates_execute_effects": False,
            "uncertain_effects_become_success": False,
            "model_text_or_exit_zero_becomes_success": False,
            "reused_identity_becomes_fresh": False,
            "repeated_state_can_exceed_stop_limit": False,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [
            artifact("kernel/engine/src/workflow_definition.rs"),
            artifact("kernel/engine/src/workflow_identity.rs"),
            artifact("kernel/engine/src/retry_admission.rs"),
            artifact("kernel/engine/src/tool_call_repair.rs"),
            artifact("kernel/engine/src/workflow_progress.rs"),
            artifact("kernel/engine/src/workflow_verifier.rs"),
            artifact("kernel/engine/tests/workflow_definition.rs"),
            artifact("kernel/engine/tests/workflow_identity.rs"),
            artifact("kernel/engine/tests/retry_admission.rs"),
            artifact("kernel/engine/tests/workflow_verifier.rs"),
            artifact("docs/verification/story-5-3-workflow-adversarial-evidence.md"),
            artifact("scripts/workflow_adversarial_campaign.py"),
            artifact("tests/test_workflow_adversarial_campaign.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_raw(value: str) -> list[str]:
    failures = [
        f"raw results missing {attack} marker: {marker}"
        for attack, markers in ATTACK_MARKERS.items()
        for marker in markers
        if marker not in value
    ]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if value != expected_report():
        failures.append("workflow adversarial report is stale, incomplete, reordered, or widened")
    if isinstance(value, dict) and value.get("product_truth") != TRUTH:
        failures.append("workflow adversarial product truth was widened")
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
                print(f"workflow adversarial build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow adversarial validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate(report)
    if failures:
        for failure in failures:
            print(f"workflow adversarial validation failed: {failure}", file=sys.stderr)
        return 1
    print("Eight adversarial workflow families validated through Sub-task 5.3.3.1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
