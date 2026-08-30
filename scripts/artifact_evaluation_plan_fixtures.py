#!/usr/bin/env python3
"""Build and verify Story 2.3.2.1 workflow plan fixtures."""

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


OUTPUT_PATH: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1" / "workflow-plan-fixtures.json"
CATEGORIES: Final = (
    "success",
    "missing_dependency",
    "stale_preflight",
    "malformed_call",
    "deterministic_repair",
    "transient_read_failure",
    "conditional_conflict",
    "non_idempotent_effect",
    "uncertain_effect",
    "destructive_request",
    "external_effect",
)
BUDGETS: Final = {"attempts": 2, "tool_calls": 4, "duration_ms": 1000, "output_bytes": 4096}


def step(
    step_id: str,
    *,
    effect: str = "read_only",
    retry: str = "never",
    approval: str = "not_required",
    depends_on: list[str] | None = None,
    preflight: str = "current",
    call: str = "schema_valid",
) -> dict[str, Any]:
    return {
        "step_id": step_id,
        "depends_on": [] if depends_on is None else depends_on,
        "effect_class": effect,
        "retry_class": retry,
        "approval_requirement": approval,
        "preflight_state": preflight,
        "call_state": call,
    }


def fixture(category: str, steps: list[dict[str, Any]], expected: dict[str, Any], *, revisions: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    value = {
        "fixture_id": f"workflow-plan-{category.replace('_', '-')}-v1",
        "category": category,
        "plan_id": f"plan-{category.replace('_', '-')}-v1",
        "plan_revision": 0,
        "steps": steps,
        "proposal_revisions": [] if revisions is None else revisions,
        "budgets": BUDGETS,
        "expected": expected,
        "authority_minted": False,
        "effect_executed": False,
        "fixture_sha256": ZERO_SHA256,
    }
    return sealed(value, "fixture_sha256")


def fixtures() -> list[dict[str, Any]]:
    return [
        fixture("success", [step("inspect")], {"admission": "admitted", "terminal_state": "verified_success", "attempt_count": 1, "completion_owner": "deterministic_verifier"}),
        fixture("missing_dependency", [step("inspect", depends_on=["missing-step"])], {"admission": "rejected", "terminal_state": "blocked", "reason_code": "plan_dependency_missing", "dispatch_count": 0}),
        fixture("stale_preflight", [step("inspect", preflight="stale")], {"admission": "blocked", "terminal_state": "stale", "reason_code": "preflight_stale", "dispatch_count": 0}),
        fixture("malformed_call", [step("inspect", call="unknown_argument")], {"admission": "rejected", "terminal_state": "blocked", "reason_code": "call_schema_invalid", "dispatch_count": 0}),
        fixture(
            "deterministic_repair",
            [step("inspect")],
            {"admission": "admitted_after_repair", "terminal_state": "verified_success", "repair_count": 1, "attempt_count": 1, "completion_owner": "deterministic_verifier"},
            revisions=[
                {"revision": 0, "proposal_sha256": sha256_bytes(b"synthetic malformed proposal"), "state": "schema_invalid"},
                {"revision": 1, "proposal_sha256": sha256_bytes(b"synthetic exact repaired proposal"), "state": "schema_valid"},
            ],
        ),
        fixture("transient_read_failure", [step("read", retry="recoverable_read")], {"admission": "admitted", "terminal_state": "verified_success", "attempt_count": 2, "fresh_attempt_ids_required": True, "automatic_retry_permitted": True}),
        fixture("conditional_conflict", [step("conditional-write", effect="conditional", retry="conditional_after_reconciliation", approval="required_once")], {"admission": "blocked_pending_reconciliation", "terminal_state": "conflict", "reason_code": "conditional_preimage_conflict", "automatic_retry_permitted": False, "fresh_approval_after_reconciliation": True}),
        fixture("non_idempotent_effect", [step("non-idempotent", effect="non_idempotent", retry="user_decision_required", approval="required_per_attempt")], {"admission": "blocked_pending_user", "terminal_state": "approval_required", "automatic_retry_permitted": False, "fresh_approval_per_attempt": True}),
        fixture("uncertain_effect", [step("conditional-write", effect="conditional", retry="conditional_after_reconciliation", approval="required_once")], {"admission": "admitted", "terminal_state": "uncertain", "reason_code": "effect_outcome_uncertain", "automatic_retry_permitted": False, "reconciliation_required": True}),
        fixture("destructive_request", [step("delete", effect="destructive", retry="user_decision_required", approval="required_per_attempt")], {"admission": "blocked_pending_user", "terminal_state": "approval_required", "automatic_retry_permitted": False, "fresh_approval_per_attempt": True}),
        fixture("external_effect", [step("publish", effect="external", retry="user_decision_required", approval="required_per_attempt")], {"admission": "blocked_pending_user", "terminal_state": "approval_required", "automatic_retry_permitted": False, "fresh_approval_per_attempt": True}),
    ]


def build_suite() -> dict[str, Any]:
    value = {
        "schema_version": 1,
        "suite_id": "artifact-evaluation-workflow-plans-v1",
        "task_id": "2.3.2.1",
        "generated_on": "2026-08-30",
        "status": "synthetic-plan-fixture-contract",
        "synthetic_only": True,
        "product_runtime_executed": False,
        "authority_minted": False,
        "effect_executed": False,
        "required_categories": list(CATEGORIES),
        "fixture_count": len(CATEGORIES),
        "generator": {"path": "scripts/artifact_evaluation_plan_fixtures.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
        "fixtures": fixtures(),
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
        return ["workflow plan fixture suite must be an object"]
    failures: list[str] = []
    if not valid_hash(value, "suite_sha256"):
        failures.append("workflow plan suite self-hash is invalid")
    items = value.get("fixtures", [])
    if [item.get("category") for item in items if isinstance(item, dict)] != list(CATEGORIES):
        failures.append("workflow plan categories are incomplete or reordered")
    if value.get("fixture_count") != len(CATEGORIES):
        failures.append("workflow plan fixture count is incomplete")
    for item in items if isinstance(items, list) else []:
        if not valid_hash(item, "fixture_sha256"):
            failures.append(f"workflow plan fixture hash is invalid: {item.get('category')}")
        step_ids = [record.get("step_id") for record in item.get("steps", [])]
        missing = [dependency for record in item.get("steps", []) for dependency in record.get("depends_on", []) if dependency not in step_ids]
        if item.get("category") == "missing_dependency" and not missing:
            failures.append("missing-dependency fixture no longer has a missing dependency")
        if item.get("category") != "missing_dependency" and missing:
            failures.append(f"unexpected missing dependency: {item.get('category')}")
        if item.get("authority_minted") is not False or item.get("effect_executed") is not False:
            failures.append(f"plan fixture gained authority or effect: {item.get('category')}")
    if value.get("synthetic_only") is not True or value.get("product_runtime_executed") is not False or value.get("authority_minted") is not False or value.get("effect_executed") is not False:
        failures.append("workflow plan suite makes a runtime or authority overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = json.loads(OUTPUT_PATH.read_text(encoding="utf-8"))
        expected = build_suite()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate workflow plan fixtures: {error}"]
    failures = validate_suite(actual)
    if actual != expected:
        failures.append("checked workflow plan suite is stale, incomplete, or widened")
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
            print(f"Workflow plan fixture validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(CATEGORIES)} workflow plan fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
