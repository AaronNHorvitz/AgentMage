#!/usr/bin/env python3
"""Build and validate Story 5.3 AC2 verifier-owned completion evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "story-ac2-verifier-owned-completion-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac2-verifier-owned-completion-report.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_verifier",
        "exit_zero_persuasive_output_and_tampered_results_never_establish_completion", "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_verifier",
        "every_postcondition_invariant_and_prohibited_effect_is_fail_closed", "--", "--exact",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine", "--test", "workflow_verifier",
        "changed_and_unchanged_verified_evidence_resolve_to_distinct_success_states", "--", "--exact",
    ),
    ("python3", "scripts/workflow_verifier_evidence.py"),
    ("python3", "scripts/workflow_terminal_evidence.py"),
    ("python3", "scripts/workflow_adversarial_campaign.py"),
)
MARKERS: Final = (
    "test exit_zero_persuasive_output_and_tampered_results_never_establish_completion ... ok",
    "test every_postcondition_invariant_and_prohibited_effect_is_fail_closed ... ok",
    "test changed_and_unchanged_verified_evidence_resolve_to_distinct_success_states ... ok",
    "Deterministic workflow evidence validated through Sub-task 5.3.2.1",
    "Closed workflow terminal outcomes validated through Sub-task 5.3.2.2",
    "Eight adversarial workflow families validated through Sub-task 5.3.3.1",
)
RETAINED_PATHS: Final = (
    "kernel/engine/src/workflow_verifier.rs",
    "kernel/engine/src/workflow_terminal.rs",
    "kernel/engine/tests/workflow_verifier.rs",
    "schemas/engineering-runtime/terminal-result.schema.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-verifier-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-terminal-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-adversarial-report.json",
)
TRUTH: Final = {
    "current_verifier_and_terminal_scope_complete": True,
    "evaluated_surface_count": 8,
    "all_required_deterministic_verifiers_must_pass": True,
    "expected_output_and_current_state_exact": True,
    "observations_receipts_and_evidence_current_complete_ordered": True,
    "required_postconditions_pass": True,
    "preserved_invariants_pass": True,
    "prohibited_effects_absent": True,
    "exit_zero_has_completion_authority": False,
    "persuasive_model_or_tool_text_has_completion_authority": False,
    "stale_or_contradictory_evidence_establishes_completion": False,
    "verified_success_requires_opaque_current_evidence_proof": True,
    "verified_no_op_is_distinct": True,
    "non_success_outcome_count": 7,
    "native_tool_effect_executed": False,
    "model_inference_executed": False,
    "network_calls": 0,
    "synthetic_data_only": True,
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
        "criterion_id": "5.3.AC2",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-verifier-and-terminal-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-5-3-ac2-verifier-owned-completion.md"),
            artifact("scripts/story_5_3_ac2_evidence.py"),
            artifact("tests/test_story_5_3_ac2_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current pure deterministic verifier and terminal-result scope",
            "no native tool, provider, model, or network effect executes",
            "installed-product and cross-platform evidence remain later gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    verifier = load("artifacts/sprints/sprint-5/story-5.3/workflow-verifier-report.json")
    terminal = load("artifacts/sprints/sprint-5/story-5.3/workflow-terminal-report.json")
    adversarial = load("artifacts/sprints/sprint-5/story-5.3/workflow-adversarial-report.json")

    if verifier.get("evaluated_surfaces") != [
        "expected_output", "postconditions", "preserved_invariants", "prohibited_effects",
        "terminal_observations", "receipt_integrity", "current_evidence", "current_state",
    ]:
        failures.append("deterministic verifier surface evidence is incomplete")
    completion = verifier.get("completion_contract", {})
    expected_completion = {
        "all_required_deterministic_verifiers_must_pass": True,
        "evidence_must_be_complete_current_and_ordered": True,
        "expected_output_and_state_are_exact": True,
        "required_invariants_must_be_preserved": True,
        "prohibited_effects_must_be_absent": True,
        "exit_zero_has_completion_authority": False,
        "model_or_tool_prose_has_completion_authority": False,
    }
    if any(completion.get(key) is not value for key, value in expected_completion.items()):
        failures.append("verifier completion authority was widened or made incomplete")

    outcomes = terminal.get("terminal_outcomes", [])
    if outcomes != [
        "verified_success", "verified_no_op", "blocked", "denied", "failed", "cancelled",
        "timed_out", "resource_exhausted", "uncertain",
    ]:
        failures.append("terminal outcome family is incomplete")
    terminal_contract = terminal.get("terminal_contract", {})
    if not all(
        terminal_contract.get(key) is True
        for key in (
            "success_requires_opaque_current_evidence_proof",
            "changed_evidence_maps_only_to_verified_success",
            "unchanged_evidence_maps_only_to_verified_no_op",
            "terminal_establisher_is_runtime_verifier",
            "terminal_result_is_integrity_bound",
        )
    ):
        failures.append("verifier-owned terminal result evidence is incomplete")

    attacks = {item.get("attack"): item for item in adversarial.get("attack_families", [])}
    for attack in ("false_completion", "contradictory_evidence"):
        item = attacks.get(attack, {})
        if item.get("admitted_false_successes") != 0 or item.get("admitted_dispatches") != 0:
            failures.append(f"{attack} established false completion")
    for report in (verifier, terminal, adversarial):
        truth = report.get("product_truth", {})
        if truth.get("runtime_effect_executed") is not False or truth.get("network_calls") != 0:
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
        "Story 5.3 AC2 report is stale, incomplete, reordered, or widened"
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
        print(f"Story 5.3 AC2 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.3 AC2 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 5.3.AC2 verifier-owned completion validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
