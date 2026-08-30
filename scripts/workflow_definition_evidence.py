#!/usr/bin/env python3
"""Build and validate retained evidence for closed workflow-definition admission."""

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
RAW_PATH: Final = EVIDENCE_DIR / "workflow-definition-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-definition-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "workflow_definition",
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
    "admits_an_exact_closed_graph_and_exposes_no_execution_authority ... ok",
    "definition_digest_and_exact_entry_frontier_fail_closed ... ok",
    "dependency_order_and_non_executable_steps_are_denied ... ok",
    "policy_set_must_be_exact_and_identity_unique ... ok",
    "every_step_policy_surface_is_bound_to_the_definition ... ok",
    "preflight_postcondition_and_policy_integrity_mutations_are_denied ... ok",
    "effect_classes_require_conservative_approval_and_idempotency_policy ... ok",
    "terminal_state_family_is_closed_and_nonterminal_states_do_not_leak_into_it ... ok",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "execution_authority_created": False,
    "runtime_effect_executed": False,
    "network_calls": 0,
    "native_platform_claim": "none",
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
        "record_type": "agentmage-workflow-definition-evidence",
        "story_id": "5.3",
        "task_id": "5.3.1.1",
        "generated_on": "2026-08-30",
        "status": "pass-local-contract-evidence",
        "requirements": ["AM-WKF-001", "AM-VER-001", "AT-WKF-001", "AT-VER-001"],
        "graph_contract": {
            "definition_sha256_verified": True,
            "dependency_graph_acyclic": True,
            "dependency_order_closed": True,
            "entry_frontier_exact": True,
            "one_execution_surface_per_step_minimum": True,
            "one_policy_binding_per_step": True,
        },
        "step_policy_contract": {
            "policy_sha256_verified": True,
            "preflights_required": True,
            "postcondition_verifiers_required": True,
            "effect_and_retry_exactly_bound": True,
            "approval_fails_closed_for_high_effect_classes": True,
            "idempotency_or_desired_state_proof_required_for_effects": True,
            "budgets_exactly_bound": True,
            "policy_identity_reuse_allowed": False,
        },
        "terminal_contract": {
            "lifecycle_states": 18,
            "nonterminal_states": 10,
            "absorbing_terminal_states": 8,
            "unknown_terminal_state_admitted": False,
        },
        "test_contract": {
            "focused_tests": len(MARKERS),
            "required_markers": list(MARKERS),
            "clippy_warnings_allowed": 0,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "artifacts": [
            artifact("kernel/engine/src/workflow_definition.rs"),
            artifact("kernel/engine/tests/workflow_definition.rs"),
            artifact("docs/verification/story-5-3-workflow-definition-evidence.md"),
            artifact("scripts/workflow_definition_evidence.py"),
            artifact("tests/test_workflow_definition_evidence.py"),
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
        failures.append("workflow definition report is stale, incomplete, reordered, or widened")
    if isinstance(value, dict) and value.get("product_truth") != TRUTH:
        failures.append("workflow definition product truth was widened")
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
                print(f"workflow definition evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow definition evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_raw(raw) + validate(report)
    if failures:
        for failure in failures:
            print(f"workflow definition evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Workflow definition evidence validated through Sub-task 5.3.1.1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
