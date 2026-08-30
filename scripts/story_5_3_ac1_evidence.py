#!/usr/bin/env python3
"""Build and validate Story 5.3 AC1 runtime-owned transition evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "story-ac1-runtime-owned-transitions-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "story-ac1-runtime-owned-transitions-report.json"
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "engineering_records::tests::workflow_transition_table_is_closed_and_terminal_states_are_absorbing",
        "--",
        "--exact",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "retry_admission",
        "fresh_attempt_requires_current_preflight_remaining_budget_and_single_use_grant",
        "--",
        "--exact",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "--test",
        "workflow_budget_independence",
        "parser_model_attempt_replan_repeat_and_total_limits_are_independent",
        "--",
        "--exact",
    ),
    ("python3", "scripts/workflow_definition_evidence.py"),
    ("python3", "scripts/workflow_identity_evidence.py"),
    ("python3", "scripts/workflow_budget_independence_evidence.py"),
    ("python3", "scripts/workflow_adversarial_campaign.py"),
)
MARKERS: Final = (
    "test engineering_records::tests::workflow_transition_table_is_closed_and_terminal_states_are_absorbing ... ok",
    "test fresh_attempt_requires_current_preflight_remaining_budget_and_single_use_grant ... ok",
    "test parser_model_attempt_replan_repeat_and_total_limits_are_independent ... ok",
    "Workflow definition evidence validated through Sub-task 5.3.1.1",
    "Workflow identity evidence validated through Sub-task 5.3.1.2",
    "Workflow budget independence validated through Sub-task 5.3.1.3",
    "Eight adversarial workflow families validated through Sub-task 5.3.3.1",
)
RETAINED_PATHS: Final = (
    "kernel/engine/src/engineering_records.rs",
    "kernel/engine/src/workflow_definition.rs",
    "kernel/engine/src/workflow_identity.rs",
    "kernel/engine/src/retry_admission.rs",
    "kernel/engine/src/workflow_budget.rs",
    "kernel/engine/src/workflow_progress.rs",
    "artifacts/sprints/sprint-5/story-5.3/workflow-definition-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-identity-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-budget-independence-report.json",
    "artifacts/sprints/sprint-5/story-5.3/workflow-adversarial-report.json",
)
TRUTH: Final = {
    "current_contract_and_in_runtime_admission_scope_complete": True,
    "workflow_lifecycle_state_count": 18,
    "absorbing_terminal_state_count": 8,
    "closed_transition_table": True,
    "workflow_identity_and_sequence_immutable": True,
    "graph_and_step_policy_integrity_bound": True,
    "preflight_required": True,
    "effect_and_retry_policy_exact": True,
    "approval_policy_exact": True,
    "single_use_grant_required": True,
    "fresh_identity_family_count": 7,
    "independent_budget_dimension_count": 6,
    "stale_preflight_dispatches": 0,
    "approval_bypass_dispatches": 0,
    "grant_reuse_dispatches": 0,
    "native_tool_effect_executed": False,
    "model_inference_executed": False,
    "network_calls": 0,
    "synthetic_data_only": True,
    "durable_cross_process_execution_complete": False,
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
        "criterion_id": "5.3.AC1",
        "generated_on": "2026-08-30",
        "status": "pass-local-current-contract-and-in-runtime-admission-scope",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "retained_evidence": [artifact(path) for path in RETAINED_PATHS],
        "artifacts": [
            artifact("docs/verification/story-5-3-ac1-runtime-owned-transitions.md"),
            artifact("scripts/story_5_3_ac1_evidence.py"),
            artifact("tests/test_story_5_3_ac1_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "acceptance_truth": dict(TRUTH),
        "limitations": [
            "the criterion closes the current pure workflow contract and synchronized in-runtime admission scope",
            "no native tool, provider, model, or network effect executes",
            "durable cross-process execution, installed-product, and cross-platform evidence remain later gates",
            "independent review, Story, Sprint, packaging, and release completion are not claimed",
        ],
    }


def validate_upstream() -> list[str]:
    failures: list[str] = []
    definition = load("artifacts/sprints/sprint-5/story-5.3/workflow-definition-report.json")
    identity = load("artifacts/sprints/sprint-5/story-5.3/workflow-identity-report.json")
    budget = load("artifacts/sprints/sprint-5/story-5.3/workflow-budget-independence-report.json")
    adversarial = load("artifacts/sprints/sprint-5/story-5.3/workflow-adversarial-report.json")

    graph = definition.get("graph_contract", {})
    step_policy = definition.get("step_policy_contract", {})
    terminal = definition.get("terminal_contract", {})
    if not all(graph.values()):
        failures.append("workflow graph admission evidence is incomplete")
    required_step_truth = {
        "policy_sha256_verified": True,
        "preflights_required": True,
        "effect_and_retry_exactly_bound": True,
        "approval_fails_closed_for_high_effect_classes": True,
        "budgets_exactly_bound": True,
    }
    if any(step_policy.get(key) is not value for key, value in required_step_truth.items()):
        failures.append("exact step-policy boundary evidence is incomplete")
    if terminal.get("lifecycle_states") != 18 or terminal.get("absorbing_terminal_states") != 8:
        failures.append("closed workflow lifecycle evidence is incomplete")

    issuance = identity.get("issuance_contract", {})
    retry = identity.get("retry_contract", {})
    if identity.get("identity_families") != [
        "call", "tool_call", "attempt", "grant", "approval", "receipt", "verification"
    ]:
        failures.append("fresh identity-family evidence is incomplete")
    if issuance.get("workflow_scoped_synchronized_ledger") is not True or issuance.get("failed_issue_is_atomic") is not True:
        failures.append("runtime-owned identity issuance is incomplete")
    if retry.get("required_per_attempt_approval_is_fresh") is not True or retry.get("policy_digest_tampering_admitted") is not False:
        failures.append("approval and policy-binding evidence is incomplete")

    expected_dimensions = [
        "parser_repair", "model_repair", "step_attempt", "replan", "repeated_state", "workflow_work"
    ]
    accounting = budget.get("accounting_contract", {})
    if budget.get("independent_limits") != expected_dimensions or not all(
        accounting.get(key) is expected
        for key, expected in {
            "exhausted_dimension_mutates_any_counter": False,
            "policy_digest_binds_every_budget_limit": True,
            "primary_and_total_work_charge_atomically": True,
        }.items()
    ):
        failures.append("independent budget evidence is incomplete")

    attacks = {item.get("attack"): item for item in adversarial.get("attack_families", [])}
    for attack in ("missing_or_stale_preflight", "approval_bypass", "grant_reuse"):
        item = attacks.get(attack, {})
        if item.get("admitted_dispatches") != 0 or item.get("admitted_false_successes") != 0:
            failures.append(f"{attack} was admitted")
    for report in (definition, identity, budget, adversarial):
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
        "Story 5.3 AC1 report is stale, incomplete, reordered, or widened"
    ]


def capture() -> tuple[str, int]:
    chunks: list[str] = []
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
        print(f"Story 5.3 AC1 evidence failed: {error}", file=sys.stderr)
        return 1
    if failures:
        for failure in failures:
            print(f"Story 5.3 AC1 evidence failed: {failure}", file=sys.stderr)
        return 1
    print("Story acceptance criterion 5.3.AC1 runtime-owned workflow transitions validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
