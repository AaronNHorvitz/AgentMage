#!/usr/bin/env python3
"""Build and validate deterministic workflow-verifier evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "workflow-verifier-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-verifier-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "workflow_verifier",
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
    "exact_current_deterministic_evidence_is_admitted_without_authority ... ok",
    "expected_output_state_observation_receipt_and_evidence_bind_independently ... ok",
    "every_postcondition_invariant_and_prohibited_effect_is_fail_closed ... ok",
    "exit_zero_persuasive_output_and_tampered_results_never_establish_completion ... ok",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "model_inference_executed": False,
    "runtime_effect_executed": False,
    "terminal_result_issued": False,
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
        "record_type": "agentmage-workflow-verifier-evidence",
        "story_id": "5.3",
        "task_id": "5.3.2.1",
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "requirements": ["AM-VER-001", "AT-VER-001"],
        "evaluated_surfaces": [
            "expected_output",
            "postconditions",
            "preserved_invariants",
            "prohibited_effects",
            "terminal_observations",
            "receipt_integrity",
            "current_evidence",
            "current_state",
        ],
        "completion_contract": {
            "all_required_deterministic_verifiers_must_pass": True,
            "policy_and_records_are_integrity_bound": True,
            "observation_and_receipt_sets_are_exact": True,
            "evidence_must_be_complete_current_and_ordered": True,
            "expected_output_and_state_are_exact": True,
            "required_invariants_must_be_preserved": True,
            "prohibited_effects_must_be_absent": True,
            "exit_zero_has_completion_authority": False,
            "model_or_tool_prose_has_completion_authority": False,
            "opaque_proof_has_execution_authority": False,
            "terminal_result_issued_by_this_increment": False,
        },
        "test_contract": {
            "focused_tests": len(MARKERS),
            "required_markers": list(MARKERS),
            "clippy_warnings_allowed": 0,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [
            artifact("kernel/engine/src/workflow_verifier.rs"),
            artifact("kernel/engine/tests/workflow_verifier.rs"),
            artifact("docs/verification/story-5-3-workflow-verifier-evidence.md"),
            artifact("scripts/workflow_verifier_evidence.py"),
            artifact("tests/test_workflow_verifier_evidence.py"),
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
        failures.append("workflow verifier report is stale, incomplete, reordered, or widened")
    if isinstance(value, dict) and value.get("product_truth") != TRUTH:
        failures.append("workflow verifier product truth was widened")
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
                print(f"workflow verifier evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow verifier evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate(report)
    if failures:
        for failure in failures:
            print(f"workflow verifier evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Deterministic workflow evidence validated through Sub-task 5.3.2.1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
