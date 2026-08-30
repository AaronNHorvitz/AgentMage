#!/usr/bin/env python3
"""Build and validate evidence for independent workflow repair and supervision budgets."""

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
RAW_PATH: Final = EVIDENCE_DIR / "workflow-budget-independence-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-budget-independence-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "workflow_budget_independence",
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
    "parser_model_attempt_replan_repeat_and_total_limits_are_independent ... ok",
    "each_exhausted_repair_dimension_denies_without_touching_other_usage ... ok",
    "invalid_repairs_and_total_work_exhaustion_return_no_admission_and_no_partial_charge ... ok",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "model_inference_executed": False,
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
        "record_type": "agentmage-workflow-budget-independence-evidence",
        "story_id": "5.3",
        "task_id": "5.3.1.3",
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "requirements": ["AM-WKF-001", "AT-WKF-001"],
        "independent_limits": [
            "parser_repair",
            "model_repair",
            "step_attempt",
            "replan",
            "repeated_state",
            "workflow_work",
        ],
        "accounting_contract": {
            "parser_and_model_repair_use_distinct_counters": True,
            "valid_parser_repair_charged_before_return": True,
            "valid_model_repair_charged_before_admission": True,
            "invalid_repair_consumes_budget": False,
            "exhausted_dimension_mutates_any_counter": False,
            "primary_and_total_work_charge_atomically": True,
            "repeated_state_uses_separate_content_bound_policy": True,
            "repeated_state_stop_mutates_budget_ledger": False,
            "policy_digest_binds_every_budget_limit": True,
        },
        "test_contract": {
            "focused_tests": len(MARKERS),
            "required_markers": list(MARKERS),
            "clippy_warnings_allowed": 0,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [
            artifact("kernel/engine/src/workflow_budget.rs"),
            artifact("kernel/engine/src/workflow_progress.rs"),
            artifact("kernel/engine/src/tool_call_repair.rs"),
            artifact("kernel/engine/tests/workflow_budget_independence.rs"),
            artifact("docs/verification/story-5-3-workflow-budget-evidence.md"),
            artifact("scripts/workflow_budget_independence_evidence.py"),
            artifact("tests/test_workflow_budget_independence_evidence.py"),
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
        failures.append("workflow budget report is stale, incomplete, reordered, or widened")
    if isinstance(value, dict) and value.get("product_truth") != TRUTH:
        failures.append("workflow budget product truth was widened")
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
                print(f"workflow budget evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow budget evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate(report)
    if failures:
        for failure in failures:
            print(f"workflow budget evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Workflow budget independence validated through Sub-task 5.3.1.3")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
