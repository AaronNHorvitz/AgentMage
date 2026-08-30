#!/usr/bin/env python3
"""Build and verify Story 2.3.2.2 workflow crash-point fixtures."""

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

from scripts.engineering_artifact_admission_fixtures import ZERO_SHA256, canonical_json, sealed, sha256_bytes, write_atomic  # noqa: E402


FIXTURE_DIR: Final = ROOT / "fixtures" / "artifact-evaluation" / "v1"
PLAN_PATH: Final = FIXTURE_DIR / "workflow-plan-fixtures.json"
OUTPUT_PATH: Final = FIXTURE_DIR / "workflow-crash-points.json"
BOUNDARIES: Final = (
    "proposal",
    "validation",
    "approval",
    "dispatch",
    "effect_observation",
    "receipt",
    "verification",
    "checkpoint",
    "terminal_event_commit",
)
POSITIONS: Final = ("before", "after")


def durable_records(boundary: str, position: str) -> list[str]:
    ordered = ["proposal", "validation", "approval", "dispatch", "effect_observation", "receipt", "verification", "checkpoint", "terminal_event"]
    index = BOUNDARIES.index(boundary)
    count = index if position == "before" else index + 1
    return ordered[:count]


def expected(boundary: str, position: str) -> dict[str, Any]:
    effect_may_have_started = BOUNDARIES.index(boundary) > BOUNDARIES.index("dispatch") or (boundary == "dispatch" and position == "after")
    observation_durable = BOUNDARIES.index(boundary) > BOUNDARIES.index("effect_observation") or (boundary == "effect_observation" and position == "after")
    terminal_durable = boundary == "terminal_event_commit" and position == "after"
    if terminal_durable:
        action = "reopen_absorbing_terminal"
        state = "verified_success"
    elif effect_may_have_started and not observation_durable:
        action = "reconcile_uncertain_effect_before_any_retry"
        state = "uncertain"
    elif observation_durable:
        action = "rebuild_forward_from_durable_observation_without_effect_replay"
        state = "recoverable"
    else:
        action = "restart_from_last_durable_pre_effect_record"
        state = "recoverable"
    return {
        "recovery_state": state,
        "recovery_action": action,
        "effect_may_have_started": effect_may_have_started,
        "effect_observation_durable": observation_durable,
        "effect_replay_allowed": False,
        "approval_reuse_allowed": False,
        "terminal_event_durable": terminal_durable,
        "false_completion_allowed": False,
    }


def fixture(boundary: str, position: str, plan_sha256: str) -> dict[str, Any]:
    value = {
        "fixture_id": f"workflow-crash-{position}-{boundary.replace('_', '-')}-v1",
        "boundary": boundary,
        "position": position,
        "plan_fixture_id": "workflow-plan-success-v1",
        "plan_suite_sha256": plan_sha256,
        "durable_records": durable_records(boundary, position),
        "expected": expected(boundary, position),
        "synthetic_crash_only": True,
        "process_terminated": False,
        "effect_executed": False,
        "fixture_sha256": ZERO_SHA256,
    }
    return sealed(value, "fixture_sha256")


def build_suite() -> dict[str, Any]:
    plan_sha = sha256_bytes(PLAN_PATH.read_bytes())
    items = [fixture(boundary, position, plan_sha) for boundary in BOUNDARIES for position in POSITIONS]
    value = {
        "schema_version": 1,
        "suite_id": "artifact-evaluation-workflow-crash-points-v1",
        "task_id": "2.3.2.2",
        "generated_on": "2026-08-30",
        "status": "synthetic-crash-point-contract",
        "synthetic_only": True,
        "product_runtime_executed": False,
        "process_terminated": False,
        "effect_executed": False,
        "boundaries": list(BOUNDARIES),
        "positions": list(POSITIONS),
        "fixture_count": len(items),
        "plan_dependency": {"path": str(PLAN_PATH.relative_to(ROOT)), "sha256": plan_sha},
        "generator": {"path": "scripts/artifact_evaluation_crash_fixtures.py", "sha256": sha256_bytes(Path(__file__).read_bytes())},
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
        return ["workflow crash suite must be an object"]
    failures: list[str] = []
    if not valid_hash(value, "suite_sha256"):
        failures.append("workflow crash suite self-hash is invalid")
    items = value.get("fixtures", [])
    expected_pairs = [(boundary, position) for boundary in BOUNDARIES for position in POSITIONS]
    if [(item.get("boundary"), item.get("position")) for item in items if isinstance(item, dict)] != expected_pairs:
        failures.append("workflow crash matrix is incomplete or reordered")
    if value.get("fixture_count") != len(expected_pairs):
        failures.append("workflow crash fixture count is incomplete")
    for item in items if isinstance(items, list) else []:
        if not valid_hash(item, "fixture_sha256"):
            failures.append(f"workflow crash fixture hash is invalid: {item.get('fixture_id')}")
        result = item.get("expected", {})
        if result.get("effect_replay_allowed") is not False or result.get("approval_reuse_allowed") is not False or result.get("false_completion_allowed") is not False:
            failures.append(f"workflow crash fixture permits replay or false completion: {item.get('fixture_id')}")
        if item.get("process_terminated") is not False or item.get("effect_executed") is not False:
            failures.append(f"workflow crash fixture makes an execution claim: {item.get('fixture_id')}")
    if value.get("synthetic_only") is not True or value.get("product_runtime_executed") is not False or value.get("process_terminated") is not False or value.get("effect_executed") is not False:
        failures.append("workflow crash suite makes a runtime or effect overclaim")
    return failures


def check() -> list[str]:
    try:
        actual = json.loads(OUTPUT_PATH.read_text(encoding="utf-8"))
        expected_value = build_suite()
    except (OSError, ValueError, json.JSONDecodeError) as error:
        return [f"cannot validate workflow crash fixtures: {error}"]
    failures = validate_suite(actual)
    if actual != expected_value:
        failures.append("checked workflow crash suite is stale, incomplete, or widened")
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
            print(f"Workflow crash fixture validation failed: {failure}", file=sys.stderr)
        return 1
    print(f"Validated {len(BOUNDARIES) * len(POSITIONS)} workflow crash-point fixtures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
