#!/usr/bin/env python3
"""Build and validate Story 5.2 AC3 bounded-termination evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-5/story-5.2"
RAW_PATH: Final = EVIDENCE_DIR / "story-ac3-bounded-termination-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac3-bounded-termination-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--lib",
        "workflow_termination::tests::story_5_2_declared_budget_or_no_progress_terminates_once_without_effect",
        "--",
        "--exact",
    ),
    ("python3", "scripts/workflow_supervision_evidence.py"),
    ("python3", "scripts/story_5_2_policy_evidence.py"),
)
MARKERS: Final = (
    "test workflow_termination::tests::story_5_2_declared_budget_or_no_progress_terminates_once_without_effect ... ok",
    "Workflow supervision evidence validated through Sub-task 5.2.3.3",
    "Story 5.2 policy evidence validated through Sub-task 5.2.4.3",
)
RETAINED_PATHS: Final = (
    "kernel/engine/src/workflow_budget.rs",
    "kernel/engine/src/workflow_progress.rs",
    "kernel/engine/src/workflow_termination.rs",
    "artifacts/sprints/sprint-5/story-5.2/workflow-supervision-report.json",
    "artifacts/sprints/sprint-5/story-5.2/workflow-supervision-results.log",
    "artifacts/sprints/sprint-5/story-5.2/policy-verification-report.json",
)
TRUTH: Final = {
    "current_supervision_scope_complete": True,
    "budget_dimension_count": 6,
    "failure_class_count": 14,
    "primary_or_error_class_limit_cases": 18,
    "independent_total_work_limit_cases": 1,
    "budget_termination_case_count": 19,
    "no_progress_repeat_limit": 2,
    "preterminal_progress_decision_count": 2,
    "one_actionable_diagnosis_per_termination": True,
    "rejected_budget_charge_mutations": 0,
    "terminal_diagnosis_sticky": True,
    "automatic_continuation_allowed": False,
    "automatic_retry_allowed": False,
    "additional_effect_count": 0,
    "runtime_effect_executed": False,
    "model_inference_executed": False,
    "synthetic_data_only": True,
    "native_platform_complete": False,
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
        "story_id": "5.2",
        "criterion_id": "5.2.AC3",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-supervision-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-5-2-ac3-bounded-termination.md"),
            artifact("scripts/story_5_2_ac3_evidence.py"),
            artifact("tests/test_story_5_2_ac3_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current deterministic budget, repeated-state, and terminal-diagnosis scope",
            "the campaign uses synthetic state and executes no workflow, model, tool, or provider effect",
            "installed-product and cross-platform supervision evidence remain later gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    supervision = load("artifacts/sprints/sprint-5/story-5.2/workflow-supervision-report.json")
    policy = load("artifacts/sprints/sprint-5/story-5.2/policy-verification-report.json")
    expected_dimensions = [
        "parser_repair",
        "model_repair",
        "step_attempt",
        "per_error_class",
        "workflow_work",
        "replan",
    ]
    if supervision.get("budget_dimensions") != expected_dimensions:
        failures.append("budget dimension evidence is incomplete or reordered")
    if len(supervision.get("failure_classes", [])) != 14:
        failures.append("failure-class budget evidence is incomplete")
    accounting = supervision.get("accounting_contract", {})
    for field in (
        "checked_addition",
        "primary_and_workflow_charge_atomic",
        "inclusive_limits",
        "ledger_bound_to_policy_id_and_sha256",
    ):
        if accounting.get(field) is not True:
            failures.append(f"accounting contract missing {field}")
    if accounting.get("rejected_charge_mutates_usage") is not False:
        failures.append("rejected budget charge mutates usage")
    repeated = supervision.get("repeated_state_contract", {})
    for field in ("exact_repeat_limit", "terminal_stop_is_sticky", "non_adjacent_cycles_detected"):
        if repeated.get(field) is not True:
            failures.append(f"repeated-state contract missing {field}")
    termination = supervision.get("termination_contract", {})
    if termination.get("safe_next_actions_are_descriptive") is not True:
        failures.append("terminal diagnosis is not actionable and descriptive")
    for field in ("authority_consumed", "automatic_continuation_allowed", "automatic_retry_allowed"):
        if termination.get(field) is not False:
            failures.append(f"termination contract widened {field}")
    if policy.get("supervision_results") != {
        "budget_dimensions": supervision.get("budget_dimensions"),
        "repeated_state_contract": supervision.get("repeated_state_contract"),
        "termination_contract": supervision.get("termination_contract"),
    }:
        failures.append("consolidated Story 5.2 supervision evidence differs")
    truth = supervision.get("product_truth", {})
    if truth.get("runtime_effect_executed") is not False or truth.get("authority_consumed") is not False:
        failures.append("upstream supervision evidence executed an effect or consumed authority")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("Traceback", "FAILED", "validation failed"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else ["Story 5.2 AC3 report is stale, incomplete, reordered, or widened"]


def capture() -> tuple[str, int]:
    chunks = []
    for command in COMMANDS:
        result = subprocess.run(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
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
                json.dumps(expected_report(), indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        failures = validate_upstream() + validate_raw(raw) + validate_report(report)
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"Story 5.2 AC3 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.2 AC3 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 5.2.AC3 bounded termination validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
