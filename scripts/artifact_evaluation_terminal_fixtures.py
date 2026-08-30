#!/usr/bin/env python3
"""Build and verify Story 2.3.2.3 premature-terminal fixtures."""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.engineering_artifact_admission_fixtures import (  # noqa: E402
    ZERO_SHA256,
    canonical_json,
    sealed,
    sha256_bytes,
    write_atomic,
)


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
PLAN_PATH: Final = FIXTURE_DIR / "workflow-plan-fixtures.json"
OUTPUT_PATH: Final = FIXTURE_DIR / "workflow-terminal-outcomes.json"
CATEGORIES: Final = (
    "empty_output",
    "reasoning_only_output",
    "false_completion",
    "open_plan_completion",
    "post_error_stop",
    "repeated_state_loop",
    "budget_exhaustion",
    "cancellation",
    "provider_disconnect",
)
TERMINAL_RESULTS: Final = ("blocked", "cancelled", "exhausted", "failed", "stalled")
MAXIMA: Final = {"model_turns": 3, "tool_attempts": 2, "repeated_state_count": 3}


def case_specs() -> dict[str, dict[str, Any]]:
    return {
        "empty_output": {
            "observed_states": ["plan_validated", "provider_output_empty"],
            "terminal_result": "failed",
            "reason_code": "model_output_empty",
            "last_verified_state": "plan_validated",
            "safe_resume_action": "start_fresh_attempt_with_current_bindings",
        },
        "reasoning_only_output": {
            "observed_states": ["plan_validated", "reasoning_present", "proposal_absent"],
            "terminal_result": "failed",
            "reason_code": "model_proposal_absent",
            "last_verified_state": "plan_validated",
            "safe_resume_action": "start_fresh_attempt_with_current_bindings",
        },
        "false_completion": {
            "observed_states": ["plan_validated", "model_completion_claimed", "verification_evidence_absent"],
            "terminal_result": "blocked",
            "reason_code": "completion_evidence_missing",
            "last_verified_state": "plan_validated",
            "safe_resume_action": "supply_current_deterministic_verification",
            "model_completion_claimed": True,
        },
        "open_plan_completion": {
            "observed_states": ["plan_validated", "model_completion_claimed", "required_step_open"],
            "terminal_result": "blocked",
            "reason_code": "plan_step_incomplete",
            "last_verified_state": "plan_validated",
            "safe_resume_action": "resolve_open_required_step_under_fresh_attempt",
            "model_completion_claimed": True,
        },
        "post_error_stop": {
            "observed_states": ["plan_validated", "tool_error_observed", "provider_output_stopped"],
            "terminal_result": "failed",
            "reason_code": "step_error_terminal",
            "last_verified_state": "tool_error_observed",
            "safe_resume_action": "diagnose_error_before_fresh_attempt",
        },
        "repeated_state_loop": {
            "observed_states": ["state_fingerprint_01", "state_fingerprint_01", "state_fingerprint_01"],
            "terminal_result": "stalled",
            "reason_code": "repeated_state_limit_reached",
            "last_verified_state": "state_fingerprint_01",
            "safe_resume_action": "revise_plan_before_fresh_attempt",
            "repeated_state_count": 3,
        },
        "budget_exhaustion": {
            "observed_states": ["plan_validated", "model_turn_01", "model_turn_02", "model_turn_03"],
            "terminal_result": "exhausted",
            "reason_code": "model_turn_budget_exhausted",
            "last_verified_state": "plan_validated",
            "safe_resume_action": "revise_budget_or_plan_before_fresh_attempt",
            "model_turns_consumed": 3,
        },
        "cancellation": {
            "observed_states": ["plan_validated", "cancellation_observed"],
            "terminal_result": "cancelled",
            "reason_code": "user_cancelled",
            "last_verified_state": "plan_validated",
            "safe_resume_action": None,
        },
        "provider_disconnect": {
            "observed_states": ["plan_validated", "provider_stream_disconnected", "proposal_incomplete"],
            "terminal_result": "failed",
            "reason_code": "provider_disconnected",
            "last_verified_state": "plan_validated",
            "safe_resume_action": "await_provider_recovery_then_start_fresh_attempt",
        },
    }


def diagnostic(category: str, spec: dict[str, Any]) -> dict[str, Any] | None:
    if category == "cancellation":
        return None
    return {
        "failed_step": "inspect",
        "reason_code": spec["reason_code"],
        "last_verified_state": spec["last_verified_state"],
        "attempts": 1,
        "evidence_ids": [f"synthetic-evidence-{category.replace('_', '-')}-v1"],
        "exhausted_budgets": ["model_turns"] if category == "budget_exhaustion" else [],
        "blocked_retry_reason": "fresh_attempt_required",
        "approval_requirement": "none",
        "uncertainty": False,
        "safe_resume_action": spec["safe_resume_action"],
    }


def fixture(category: str, spec: dict[str, Any], plan_sha256: str) -> dict[str, Any]:
    expected = {
        "terminal_result": spec["terminal_result"],
        "reason_code": spec["reason_code"],
        "bounded_termination": True,
        "artificial_continue_required": False,
        "verified_completion": False,
        "false_completion_allowed": False,
        "automatic_retry_allowed": False,
        "effect_replay_allowed": False,
        "diagnostic_count": 0 if category == "cancellation" else 1,
        "safe_resume_action": spec["safe_resume_action"],
    }
    value = {
        "fixture_id": f"workflow-terminal-{category.replace('_', '-')}-v1",
        "category": category,
        "plan_fixture_id": "workflow-plan-success-v1",
        "plan_suite_sha256": plan_sha256,
        "limits": MAXIMA,
        "observed_states": spec["observed_states"],
        "model_completion_claimed": spec.get("model_completion_claimed", False),
        "repeated_state_count": spec.get("repeated_state_count", 0),
        "model_turns_consumed": spec.get("model_turns_consumed", 1),
        "expected": expected,
        "terminal_diagnostic": diagnostic(category, spec),
        "synthetic_only": True,
        "real_provider_contacted": False,
        "product_runtime_executed": False,
        "authority_minted": False,
        "effect_executed": False,
        "fixture_sha256": ZERO_SHA256,
    }
    return sealed(value, "fixture_sha256")


def build_suite() -> dict[str, Any]:
    plan_sha = sha256_bytes(PLAN_PATH.read_bytes())
    specs = case_specs()
    items = [fixture(category, specs[category], plan_sha) for category in CATEGORIES]
    value = {
        "schema_version": 1,
        "suite_id": "artifact-evaluation-workflow-terminal-outcomes-v1",
        "task_id": "2.3.2.3",
        "generated_on": "2026-08-30",
        "status": "synthetic-premature-terminal-contract",
        "synthetic_only": True,
        "product_runtime_executed": False,
        "real_provider_contacted": False,
        "authority_minted": False,
        "effect_executed": False,
        "required_categories": list(CATEGORIES),
        "terminal_result_vocabulary": list(TERMINAL_RESULTS),
        "fixture_count": len(items),
        "plan_dependency": {"path": str(PLAN_PATH.relative_to(ROOT)), "sha256": plan_sha},
        "generator": {"path": "scripts/artifact_evaluation_terminal_fixtures.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
        "fixtures": items,
        "suite_sha256": ZERO_SHA256,
    }
    return sealed(value, "suite_sha256")


def valid_hash(record: dict[str, Any], field: str) -> bool:
    unhashed = copy.deepcopy(record)
    recorded = unhashed.get(field)
    unhashed[field] = ZERO_SHA256
    return recorded == sha256_bytes(canonical_json(unhashed))


def validate_suite(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return ["workflow terminal suite must be an object"]
    failures: list[str] = []
    if not valid_hash(value, "suite_sha256"):
        failures.append("workflow terminal suite self-hash is invalid")
    items = value.get("fixtures", [])
    if not isinstance(items, list):
        return failures + ["workflow terminal fixtures must be a list"]
    if [item.get("category") for item in items if isinstance(item, dict)] != list(CATEGORIES):
        failures.append("workflow terminal categories are incomplete or reordered")
    if value.get("fixture_count") != len(CATEGORIES):
        failures.append("workflow terminal fixture count is incomplete")
    for item in items:
        if not isinstance(item, dict):
            failures.append("workflow terminal fixture must be an object")
            continue
        category = item.get("category")
        expected = item.get("expected", {})
        if not valid_hash(item, "fixture_sha256"):
            failures.append(f"workflow terminal fixture hash is invalid: {category}")
        if expected.get("terminal_result") not in TERMINAL_RESULTS:
            failures.append(f"workflow terminal result is outside the closed vocabulary: {category}")
        if expected.get("bounded_termination") is not True or expected.get("artificial_continue_required") is not False:
            failures.append(f"workflow terminal fixture is unbounded or needs an artificial continue: {category}")
        denied = ("verified_completion", "false_completion_allowed", "automatic_retry_allowed", "effect_replay_allowed")
        if any(expected.get(field) is not False for field in denied):
            failures.append(f"workflow terminal fixture permits completion, retry, or replay: {category}")
        diagnostic_count = expected.get("diagnostic_count")
        diagnostic_value = item.get("terminal_diagnostic")
        if category == "cancellation":
            if diagnostic_count != 0 or diagnostic_value is not None or expected.get("terminal_result") != "cancelled":
                failures.append("cancelled workflow terminal fixture invented a failure diagnostic")
        elif diagnostic_count != 1 or not isinstance(diagnostic_value, dict):
            failures.append(f"non-cancelled workflow terminal fixture lacks one diagnostic: {category}")
        if item.get("real_provider_contacted") is not False or item.get("product_runtime_executed") is not False or item.get("authority_minted") is not False or item.get("effect_executed") is not False:
            failures.append(f"workflow terminal fixture makes an execution or authority claim: {category}")
    suite_denied = ("real_provider_contacted", "product_runtime_executed", "authority_minted", "effect_executed")
    if value.get("synthetic_only") is not True or any(value.get(field) is not False for field in suite_denied):
        failures.append("workflow terminal suite makes a provider, runtime, authority, or effect overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = json.loads(OUTPUT_PATH.read_text(encoding="utf-8"))
        expected = build_suite()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate workflow terminal fixtures: {error}"]
    failures = validate_suite(actual)
    if actual != expected:
        failures.append("checked workflow terminal suite is stale, incomplete, or widened")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_atomic(OUTPUT_PATH, canonical_json(build_suite()))
    failures = check()
    if failures:
        for failure in failures:
            print(f"Workflow terminal fixture validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(CATEGORIES)} workflow terminal-outcome fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
