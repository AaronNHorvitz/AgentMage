#!/usr/bin/env python3
"""Build and verify Story 2.3.3.2 workflow golden metrics."""

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
CRASH_PATH: Final = FIXTURE_DIR / "workflow-crash-points.json"
TERMINAL_PATH: Final = FIXTURE_DIR / "workflow-terminal-outcomes.json"
OUTPUT_PATH: Final = FIXTURE_DIR / "workflow-golden-metrics.json"
METRIC_IDS: Final = (
    "plan_completion",
    "preflight_accuracy",
    "schema_valid_calls",
    "verifier_precision",
    "duplicate_effect_count",
    "approval_bypass",
    "attempt_count",
    "recovery_quality",
    "terminal_diagnosis",
    "bounded_termination",
)


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def fraction(numerator: int, denominator: int) -> dict[str, int]:
    return {"numerator": numerator, "denominator": denominator}


def metric(metric_id: str, *, unit: str, golden: dict[str, Any], sources: list[str]) -> dict[str, Any]:
    return {
        "metric_id": metric_id,
        "unit": unit,
        "measurement_scope": "golden_fixture_oracle",
        "source_scope": sources,
        "golden": golden,
        "product_observation": None,
        "product_measurement_status": "not_executed",
        "support_claim": "none",
    }


def build_metrics() -> list[dict[str, Any]]:
    plans = load(PLAN_PATH)["fixtures"]
    crashes = load(CRASH_PATH)["fixtures"]
    terminals = load(TERMINAL_PATH)["fixtures"]
    plan_source = [str(PLAN_PATH.relative_to(ROOT))]
    crash_source = [str(CRASH_PATH.relative_to(ROOT))]
    terminal_source = [str(TERMINAL_PATH.relative_to(ROOT))]

    terminal_classified = sum(bool(item["expected"].get("terminal_state")) for item in plans)
    verified_plans = [item for item in plans if item["expected"]["terminal_state"] == "verified_success"]
    non_success_plans = [item for item in plans if item["expected"]["terminal_state"] != "verified_success"]

    preflight_checks = []
    call_checks = []
    approval_steps = []
    for item in plans:
        steps = item["steps"]
        for step in steps:
            stale = step["preflight_state"] == "stale"
            preflight_checks.append(
                (not stale and step["preflight_state"] == "current")
                or (stale and item["expected"]["terminal_state"] == "stale" and item["expected"]["dispatch_count"] == 0)
            )
            malformed = step["call_state"] != "schema_valid"
            call_checks.append(
                (not malformed and step["call_state"] == "schema_valid")
                or (malformed and item["expected"]["reason_code"] == "call_schema_invalid" and item["expected"]["dispatch_count"] == 0)
            )
            if step["approval_requirement"] != "not_required":
                approval_steps.append((item, step))

    verifier_checks = [item["expected"].get("completion_owner") == "deterministic_verifier" for item in verified_plans]
    verifier_checks.extend(item["expected"].get("completion_owner") is None for item in non_success_plans)
    duplicate_protection_checks = [item["effect_executed"] is False for item in plans]
    duplicate_protection_checks.extend(item["expected"]["effect_replay_allowed"] is False for item in crashes)
    duplicate_protection_checks.extend(item["expected"]["effect_replay_allowed"] is False for item in terminals)
    approval_checks = [
        item["authority_minted"] is False
        and step["approval_requirement"] in {"required_once", "required_per_attempt"}
        and item["expected"]["terminal_state"] in {"approval_required", "conflict", "uncertain"}
        for item, step in approval_steps
    ]

    attempt_records = [item["expected"]["attempt_count"] for item in plans if "attempt_count" in item["expected"]]
    attempt_checks = [isinstance(value, int) and 1 <= value <= item["budgets"]["attempts"] for item, value in ((item, item["expected"]["attempt_count"]) for item in plans if "attempt_count" in item["expected"])]
    crash_recovery_checks = [
        bool(item["expected"]["recovery_action"])
        and item["expected"]["effect_replay_allowed"] is False
        and item["expected"]["approval_reuse_allowed"] is False
        for item in crashes
    ]
    terminal_recovery_checks = [
        (item["category"] == "cancellation" and item["expected"]["safe_resume_action"] is None)
        or (item["category"] != "cancellation" and bool(item["expected"]["safe_resume_action"]))
        for item in terminals
    ]
    diagnosis_checks = [
        (item["category"] == "cancellation" and item["expected"]["diagnostic_count"] == 0 and item["terminal_diagnostic"] is None)
        or (item["category"] != "cancellation" and item["expected"]["diagnostic_count"] == 1 and isinstance(item["terminal_diagnostic"], dict))
        for item in terminals
    ]
    termination_checks = [
        item["expected"]["bounded_termination"] is True
        and item["expected"]["artificial_continue_required"] is False
        and item["expected"]["verified_completion"] is False
        for item in terminals
    ]

    return [
        metric(
            "plan_completion",
            unit="classified_plan_outcomes",
            golden={"fraction": fraction(terminal_classified, len(plans)), "plan_count": len(plans), "verified_success_count": len(verified_plans), "explicit_non_success_count": len(non_success_plans), "open_plan_completion_allowed": False},
            sources=plan_source,
        ),
        metric(
            "preflight_accuracy",
            unit="classified_step_preflights",
            golden={"fraction": fraction(sum(preflight_checks), len(preflight_checks)), "current_count": sum(step["preflight_state"] == "current" for item in plans for step in item["steps"]), "stale_count": sum(step["preflight_state"] == "stale" for item in plans for step in item["steps"]), "stale_dispatch_count": 0},
            sources=plan_source,
        ),
        metric(
            "schema_valid_calls",
            unit="classified_step_calls",
            golden={"fraction": fraction(sum(call_checks), len(call_checks)), "schema_valid_count": sum(step["call_state"] == "schema_valid" for item in plans for step in item["steps"]), "schema_invalid_count": sum(step["call_state"] != "schema_valid" for item in plans for step in item["steps"]), "invalid_dispatch_count": 0},
            sources=plan_source,
        ),
        metric(
            "verifier_precision",
            unit="terminal_plan_classifications",
            golden={"fraction": fraction(sum(verifier_checks), len(verifier_checks)), "verifier_owned_success_count": len(verified_plans), "non_success_without_completion_owner_count": len(non_success_plans), "model_owned_completion_allowed": False},
            sources=plan_source,
        ),
        metric(
            "duplicate_effect_count",
            unit="effect_replay_protection_oracles",
            golden={"fraction": fraction(sum(duplicate_protection_checks), len(duplicate_protection_checks)), "duplicate_effect_count": 0, "protected_plan_count": len(plans), "protected_crash_count": len(crashes), "protected_terminal_count": len(terminals)},
            sources=plan_source + crash_source + terminal_source,
        ),
        metric(
            "approval_bypass",
            unit="approval_required_step_oracles",
            golden={"fraction": fraction(sum(approval_checks), len(approval_checks)), "approval_required_step_count": len(approval_steps), "approval_bypass_count": 0, "authority_minted_by_fixture": False},
            sources=plan_source,
        ),
        metric(
            "attempt_count",
            unit="declared_attempt_records",
            golden={"fraction": fraction(sum(attempt_checks), len(attempt_checks)), "record_count": len(attempt_records), "total_attempts": sum(attempt_records), "minimum_attempts": min(attempt_records), "maximum_attempts": max(attempt_records), "fresh_identity_required_when_retried": True},
            sources=plan_source,
        ),
        metric(
            "recovery_quality",
            unit="safe_recovery_oracles",
            golden={"fraction": fraction(sum(crash_recovery_checks) + sum(terminal_recovery_checks), len(crash_recovery_checks) + len(terminal_recovery_checks)), "crash_oracle_count": len(crash_recovery_checks), "terminal_oracle_count": len(terminal_recovery_checks), "replay_allowed": False},
            sources=crash_source + terminal_source,
        ),
        metric(
            "terminal_diagnosis",
            unit="diagnosis_cardinality_oracles",
            golden={"fraction": fraction(sum(diagnosis_checks), len(diagnosis_checks)), "non_cancelled_diagnosis_count": sum(item["category"] != "cancellation" for item in terminals), "cancelled_diagnosis_count": 0, "duplicate_diagnosis_allowed": False},
            sources=terminal_source,
        ),
        metric(
            "bounded_termination",
            unit="premature_terminal_oracles",
            golden={"fraction": fraction(sum(termination_checks), len(termination_checks)), "terminal_fixture_count": len(terminals), "artificial_continue_count": 0, "false_completion_count": 0},
            sources=terminal_source,
        ),
    ]


def build_suite() -> dict[str, Any]:
    dependencies = [PLAN_PATH, CRASH_PATH, TERMINAL_PATH]
    value = {
        "schema_version": 1,
        "golden_manifest_version": "1.0.0",
        "suite_id": "artifact-evaluation-workflow-golden-metrics-v1",
        "task_id": "2.3.3.2",
        "generated_on": "2026-08-30",
        "status": "golden-workflow-fixture-metrics",
        "synthetic_only": True,
        "product_runtime_executed": False,
        "effect_executed": False,
        "authority_minted": False,
        "platform_support_claim": "none",
        "required_metric_ids": list(METRIC_IDS),
        "metric_count": len(METRIC_IDS),
        "dependencies": [{"path": str(path.relative_to(ROOT)), "sha256": sha256_bytes(path.read_bytes())} for path in dependencies],
        "generator": {"path": "scripts/artifact_evaluation_workflow_metrics.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
        "metrics": build_metrics(),
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
        return ["workflow metric suite must be an object"]
    failures: list[str] = []
    if not valid_hash(value, "suite_sha256"):
        failures.append("workflow metric suite self-hash is invalid")
    metrics = value.get("metrics", [])
    if not isinstance(metrics, list):
        return failures + ["workflow metrics must be a list"]
    if [item.get("metric_id") for item in metrics if isinstance(item, dict)] != list(METRIC_IDS):
        failures.append("workflow metric set is incomplete or reordered")
    if value.get("metric_count") != len(METRIC_IDS):
        failures.append("workflow metric count is incomplete")
    for item in metrics:
        if not isinstance(item, dict):
            failures.append("workflow metric must be an object")
            continue
        ratio = item.get("golden", {}).get("fraction", {})
        if ratio.get("numerator") != ratio.get("denominator") or not isinstance(ratio.get("denominator"), int) or ratio.get("denominator", 0) <= 0:
            failures.append(f"workflow golden metric is not exact: {item.get('metric_id')}")
        if item.get("product_observation") is not None or item.get("product_measurement_status") != "not_executed" or item.get("support_claim") != "none":
            failures.append(f"workflow metric makes a product or support overclaim: {item.get('metric_id')}")
    if value.get("synthetic_only") is not True or value.get("product_runtime_executed") is not False or value.get("effect_executed") is not False or value.get("authority_minted") is not False or value.get("platform_support_claim") != "none":
        failures.append("workflow metric suite makes a runtime, effect, authority, or support overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = load(OUTPUT_PATH)
        expected = build_suite()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate workflow golden metrics: {error}"]
    failures = validate_suite(actual)
    if actual != expected:
        failures.append("checked workflow golden metrics are stale, incomplete, or widened")
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
            print(f"Workflow golden metric validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(METRIC_IDS)} workflow golden metrics")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
