#!/usr/bin/env python3
"""Build and validate Story 5.3 AC3 uncertain-effect blocking evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "story-ac3-uncertain-effect-blocking-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac3-uncertain-effect-blocking-report.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_identity",
        "uncertain_effects_cannot_open_a_successor_until_safely_reconciled", "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test", "retry_admission",
        "unsafe_and_uncertain_effects_never_receive_automatic_new_attempts", "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test", "retry_admission",
        "racing_eligible_attempts_execute_once_and_uncertainty_is_sticky", "--", "--exact",
    ),
    ("python3", "scripts/workflow_identity_evidence.py"),
    ("python3", "scripts/retry_repair_policy_evidence.py"),
    ("python3", "scripts/workflow_terminal_evidence.py"),
    ("python3", "scripts/workflow_adversarial_campaign.py"),
)
MARKERS: Final = (
    "test uncertain_effects_cannot_open_a_successor_until_safely_reconciled ... ok",
    "test unsafe_and_uncertain_effects_never_receive_automatic_new_attempts ... ok",
    "test racing_eligible_attempts_execute_once_and_uncertainty_is_sticky ... ok",
    "Workflow identity evidence validated through Sub-task 5.3.1.2",
    "Retry, repair, mutation, and race evidence validated through Sub-task 5.2.4.2",
    "Closed workflow terminal outcomes validated through Sub-task 5.3.2.2",
    "Eight adversarial workflow families validated through Sub-task 5.3.3.1",
)
RETAINED_PATHS: Final = (
    "kernel/engine/src/workflow_identity.rs",
    "kernel/engine/src/retry_admission.rs",
    "kernel/engine/src/workflow_terminal.rs",
    "kernel/engine/tests/workflow_identity.rs",
    "kernel/engine/tests/retry_admission.rs",
    "artifacts/sprints/sprint-5/story-5.3/workflow-identity-report.json",
    "artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-terminal-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-adversarial-report.json",
)
TRUTH: Final = {
    "current_retry_identity_and_in_runtime_gate_scope_complete": True,
    "fresh_identity_family_count": 7,
    "replayed_prior_identity_admitted": False,
    "old_call_or_authority_replayed": False,
    "automatic_retry_forbidden_effect_class_count": 4,
    "automatic_retry_forbidden_effect_classes": [
        "non_idempotent", "destructive", "external", "unknown"
    ],
    "uncertain_effect_automatic_retry": False,
    "uncertain_effect_requires_safe_reconciliation": True,
    "uncertain_effect_requires_separate_user_approval": True,
    "concurrent_eligible_attempts": 16,
    "execution_admissions": 1,
    "synthetic_effect_callback_invocations": 1,
    "uncertain_outcome_retained": True,
    "uncertain_outcome_converted_to_success": False,
    "uncertain_outcome_retried": False,
    "terminal_uncertain_distinct_from_failed": True,
    "native_tool_effect_executed": False,
    "model_inference_executed": False,
    "network_calls": 0,
    "synthetic_data_only": True,
    "cross_process_crash_durability_complete": False,
    "installed_product_complete": False,
    "cross_platform_complete": False,
    "independent_review_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    value = ROOT / path
    return {"path": path, "byte_length": value.stat().st_size, "sha256": sha256(value)}


def load(path: str) -> dict[str, Any]:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-acceptance-evidence",
        "story_id": "5.3",
        "criterion_id": "5.3.AC3",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-retry-identity-and-in-runtime-gate-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-5-3-ac3-uncertain-effect-blocking.md"),
            artifact("scripts/story_5_3_ac3_evidence.py"),
            artifact("tests/test_story_5_3_ac3_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current retry identity and synchronized in-runtime gate scope",
            "the single effect callback is synthetic and content free; no native tool or provider executes",
            "cross-process crash durability, installed-product, and cross-platform evidence remain later gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    identity = load("artifacts/sprints/sprint-5/story-5.3/workflow-identity-report.json")
    retry_report = load("artifacts/sprints/sprint-5/story-5.2/retry-repair-policy-report.json")
    terminal = load("artifacts/sprints/sprint-5/story-5.3/workflow-terminal-report.json")
    adversarial = load("artifacts/sprints/sprint-5/story-5.3/workflow-adversarial-report.json")

    if identity.get("identity_families") != [
        "call", "tool_call", "attempt", "grant", "approval", "receipt", "verification"
    ]:
        failures.append("fresh identity-family evidence is incomplete")
    retry = identity.get("retry_contract", {})
    if retry.get("automatic_retry_forbidden_effects") != [
        "non_idempotent", "destructive", "external", "unknown"
    ]:
        failures.append("ineligible effect retry refusal is incomplete")
    if any(
        retry.get(key) is not value
        for key, value in {
            "uncertain_effect_automatic_retry": False,
            "uncertain_effect_requires_safe_reconciliation_and_user_approval": True,
            "required_per_attempt_approval_is_fresh": True,
        }.items()
    ):
        failures.append("uncertain effect reconciliation or approval evidence is incomplete")

    race = retry_report.get("race_contract", {})
    if race != {
        "concurrent_eligible_attempts": 16,
        "execution_admissions": 1,
        "effect_callback_invocations": 1,
        "predecessor_step_attempt_claim_atomic": True,
        "execution_permit_cloneable": False,
        "uncertain_outcome_retained": True,
        "uncertain_outcome_converted_to_success": False,
        "uncertain_outcome_retried": False,
    }:
        failures.append("sticky uncertain race evidence is incomplete")
    terminal_contract = terminal.get("terminal_contract", {})
    if terminal_contract.get("uncertain_is_distinct_from_failed") is not True:
        failures.append("terminal uncertainty was collapsed into failure")
    attacks = {item.get("attack"): item for item in adversarial.get("attack_families", [])}
    uncertain = attacks.get("uncertain_effect", {})
    if uncertain.get("admitted_dispatches") != 0 or uncertain.get("admitted_false_successes") != 0:
        failures.append("uncertain effect attack was replayed or promoted")
    for report in (identity, retry_report, terminal, adversarial):
        truth = report.get("product_truth", {})
        executed = truth.get("runtime_effect_executed", truth.get("tool_effect_executed"))
        if executed is not False or truth.get("network_calls") != 0:
            failures.append("upstream evidence executed a native effect or network call")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("Traceback", "test result: FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else [
        "Story 5.3 AC3 report is stale, incomplete, reordered, or widened"
    ]


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(
            command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip()}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    try:
        if args.write:
            raw, returncode = capture()
            if returncode != 0:
                sys.stderr.write(raw)
                return 1
            failures = validate_upstream() + validate_raw(raw)
            if failures:
                raise ValueError("; ".join(failures))
            EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
            RAW_PATH.write_text(raw, encoding="utf-8")
            REPORT_PATH.write_text(
                json.dumps(expected_report(), indent=2, sort_keys=True) + "\n", encoding="utf-8"
            )
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_upstream() + validate_raw(raw) + validate_report(report)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"Story 5.3 AC3 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.3 AC3 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 5.3.AC3 uncertain and ineligible effects validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
