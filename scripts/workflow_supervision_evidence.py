#!/usr/bin/env python3
"""Build and validate retained evidence for workflow supervision policy."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts" / "sprints" / "sprint-5" / "story-5.2"
RAW_PATH: Final = EVIDENCE_DIR / "workflow-supervision-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-supervision-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "workflow_budget::tests",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "workflow_progress::tests",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "workflow_termination::tests",
        "--locked",
    ),
)
MARKERS: Final = (
    "every_dimension_is_separate_and_every_error_class_is_exact ... ok",
    "missing_or_duplicate_error_classes_cannot_form_a_policy ... ok",
    "checked_arithmetic_and_total_budget_fail_without_partial_consumption ... ok",
    "content_identity_is_immutable_and_substitution_is_denied ... ok",
    "fingerprint_is_deterministic_complete_and_boundary_preserving ... ok",
    "incomplete_malformed_or_oversized_state_fails_closed ... ok",
    "non_adjacent_repeated_state_stops_at_the_exact_policy_limit ... ok",
    "policy_identity_is_immutable_and_substitution_changes_nothing ... ok",
    "exhausted_and_saturated_budgets_are_inert_and_do_not_change_usage ... ok",
    "every_policy_denial_has_one_exact_safe_nonexecuting_action ... ok",
    "only_exact_repeated_state_stop_terminates_and_detector_remains_unchanged ... ok",
    "terminal_output_has_no_caller_text_identity_or_authority_surface ... ok",
)
FAILURE_CLASSES: Final = (
    "malformed_input",
    "preflight",
    "policy",
    "approval",
    "dependency",
    "transient",
    "conflict",
    "timeout",
    "cancellation",
    "crash",
    "uncertain_effect",
    "verification",
    "resource",
    "internal",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "runtime_effect_executed": False,
    "authority_consumed": False,
    "network_calls": 0,
    "native_platform_claim": "none",
    "product_support_claim": "none",
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
        "record_type": "agentmage-workflow-supervision-evidence",
        "story_id": "5.2",
        "task_id": "5.2.3.3",
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "budget_dimensions": [
            "parser_repair",
            "model_repair",
            "step_attempt",
            "per_error_class",
            "workflow_work",
            "replan",
        ],
        "failure_classes": list(FAILURE_CLASSES),
        "accounting_contract": {
            "all_failure_classes_required_exactly_once": True,
            "checked_addition": True,
            "primary_and_workflow_charge_atomic": True,
            "rejected_charge_mutates_usage": False,
            "inclusive_limits": True,
            "ledger_bound_to_policy_id_and_sha256": True,
            "policy_sha256_content_derived": True,
            "caller_supplied_policy_sha256": False,
        },
        "repeated_state_contract": {
            "fingerprint_dimensions": [
                "plan",
                "step",
                "observations",
                "proposal",
                "tool",
                "policy",
                "receipts",
                "artifacts",
                "verifier_state",
            ],
            "all_dimensions_required": True,
            "ordered_sequences_boundary_separated": True,
            "non_adjacent_cycles_detected": True,
            "exact_repeat_limit": True,
            "terminal_stop_is_sticky": True,
            "detector_bound_to_policy_id_and_sha256": True,
            "decision_contains_authority": False,
        },
        "termination_contract": {
            "terminal_sources": [
                "budget_exhaustion",
                "counter_saturation",
                "policy_denial",
                "repeated_state_limit",
            ],
            "caller_text_in_reason": False,
            "stable_reason_codes": True,
            "safe_next_actions_are_descriptive": True,
            "nonterminal_input_can_terminate": False,
            "authority_consumed": False,
            "automatic_continuation_allowed": False,
            "automatic_retry_allowed": False,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/workflow_budget.rs"),
            artifact("kernel/engine/src/workflow_progress.rs"),
            artifact("kernel/engine/src/workflow_termination.rs"),
            artifact("scripts/workflow_supervision_evidence.py"),
            artifact("tests/test_workflow_supervision_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_raw(text: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in text]
    for prohibited in ("FAILED (", "test result: FAILED", "not ok ", "Traceback (most recent call last)"):
        if prohibited in text:
            failures.append(f"raw results contain failure marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["workflow supervision report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["workflow supervision product truth was widened"]
    return []


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
                print(f"workflow supervision evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow supervision evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"workflow supervision evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Workflow supervision evidence validated through Sub-task 5.2.3.3")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
