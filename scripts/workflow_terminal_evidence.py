#!/usr/bin/env python3
"""Build and validate closed workflow terminal-outcome evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "workflow-terminal-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-terminal-report.json"
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
        "-p",
        "agentmage-kernel-contracts",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ),
    ("npm", "run", "engineering-runtime:schemas:check"),
)
MARKERS: Final = (
    "changed_and_unchanged_verified_evidence_resolve_to_distinct_success_states ... ok",
    "all_seven_non_success_states_remain_exact_and_diagnostic ... ok",
    "malformed_non_success_cannot_collapse_into_a_success_or_generic_failure ... ok",
    "tool observations and terminal results preserve runtime authority",
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
        "record_type": "agentmage-workflow-terminal-evidence",
        "story_id": "5.3",
        "task_id": "5.3.2.2",
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "requirements": ["AM-WKF-001", "AM-VER-001", "AT-VER-001"],
        "terminal_outcomes": [
            "verified_success",
            "verified_no_op",
            "blocked",
            "denied",
            "failed",
            "cancelled",
            "timed_out",
            "resource_exhausted",
            "uncertain",
        ],
        "terminal_contract": {
            "success_requires_opaque_current_evidence_proof": True,
            "changed_evidence_maps_only_to_verified_success": True,
            "unchanged_evidence_maps_only_to_verified_no_op": True,
            "non_success_kind_is_closed": True,
            "non_success_diagnostic_is_required": True,
            "non_success_safe_next_action_is_required": True,
            "terminal_establisher_is_runtime_verifier": True,
            "denied_is_distinct_from_blocked_and_failed": True,
            "uncertain_is_distinct_from_failed": True,
            "terminal_result_is_integrity_bound": True,
        },
        "test_contract": {
            "focused_markers": len(MARKERS),
            "required_markers": list(MARKERS),
            "clippy_warnings_allowed": 0,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [
            artifact("kernel/contracts/src/engineering_records.rs"),
            artifact("kernel/engine/src/workflow_terminal.rs"),
            artifact("kernel/engine/src/workflow_verifier.rs"),
            artifact("kernel/engine/tests/workflow_verifier.rs"),
            artifact("schemas/engineering-runtime/terminal-result.schema.json"),
            artifact("schemas/engineering-runtime/workflow-state.schema.json"),
            artifact("docs/verification/story-5-3-workflow-terminal-evidence.md"),
            artifact("scripts/workflow_terminal_evidence.py"),
            artifact("tests/test_workflow_terminal_evidence.py"),
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
        failures.append("workflow terminal report is stale, incomplete, reordered, or widened")
    if isinstance(value, dict) and value.get("product_truth") != TRUTH:
        failures.append("workflow terminal product truth was widened")
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
                print(f"workflow terminal evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow terminal evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate(report)
    if failures:
        for failure in failures:
            print(f"workflow terminal evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Closed workflow terminal outcomes validated through Sub-task 5.3.2.2")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
