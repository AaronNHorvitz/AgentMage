#!/usr/bin/env python3
"""Generate and validate Task 12.1.2.2 synthetic runtime fixtures."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final

try:
    from scripts.evidence_core import atomic_write, canonical_json_bytes
except ModuleNotFoundError:
    from evidence_core import atomic_write, canonical_json_bytes

ROOT: Final = Path(__file__).resolve().parents[1]
FIXTURE_ROOT: Final = ROOT / "fixtures/runtime/v1"
FIXTURE_PATHS: Final = {
    "planning": FIXTURE_ROOT / "planning-fixtures.json",
    "reasoning": FIXTURE_ROOT / "reasoning-mode-fixtures.json",
    "completion": FIXTURE_ROOT / "completion-evidence-fixtures.json",
}


def planning_fixtures() -> dict[str, Any]:
    steps = [
        {
            "depends_on": [],
            "expected_evidence": ["observation"],
            "ordinal": 0,
            "plan_step_id": "plan-fixture:step:0",
            "state": "proposed",
        },
        {
            "depends_on": ["plan-fixture:step:0"],
            "expected_evidence": ["validation"],
            "ordinal": 1,
            "plan_step_id": "plan-fixture:step:1",
            "state": "proposed",
        },
    ]
    one_running = json.loads(json.dumps(steps))
    one_running[0]["state"] = "running"
    competing = json.loads(json.dumps(one_running))
    competing[1]["state"] = "running"
    dependency = json.loads(json.dumps(steps))
    dependency[1]["state"] = "ready"
    return {
        "schema_version": 1,
        "fixture_set_id": "agentmage-runtime-planning-v1",
        "cases": [
            {
                "case_id": "PLN-01",
                "plan_state": "proposed",
                "steps": steps,
                "expected": {"admitted": True, "code": "valid"},
            },
            {
                "case_id": "PLN-02",
                "plan_state": "in_progress",
                "steps": one_running,
                "expected": {"admitted": True, "code": "one-active-step"},
            },
            {
                "case_id": "PLN-03",
                "plan_state": "in_progress",
                "steps": competing,
                "expected": {"admitted": False, "code": "competing-running-step"},
            },
            {
                "case_id": "PLN-04",
                "plan_state": "current",
                "steps": dependency,
                "expected": {"admitted": False, "code": "dependency-incomplete"},
            },
        ],
        "private_user_data_used": False,
        "external_network_used": False,
    }


def reasoning_fixtures() -> dict[str, Any]:
    shared = {
        "authority": "descriptive-only",
        "clarification_gate_required": True,
        "contradiction_check_required": True,
        "independent_verification_required": True,
        "private_chain_of_thought_retained": False,
    }
    return {
        "schema_version": 1,
        "fixture_set_id": "agentmage-reasoning-modes-v1",
        "cases": [
            {
                "case_id": "RMD-01",
                "mode": "concise",
                "maximum_assumptions": 4,
                "maximum_hypotheses": 4,
                "maximum_clarification_questions": 2,
                **shared,
            },
            {
                "case_id": "RMD-02",
                "mode": "deep",
                "maximum_assumptions": 16,
                "maximum_hypotheses": 16,
                "maximum_clarification_questions": 8,
                **shared,
            },
        ],
        "mode_changes_authority": False,
        "mode_changes_evidence_standard": False,
        "private_user_data_used": False,
        "external_network_used": False,
    }


def completion_fixtures() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "fixture_set_id": "agentmage-completion-evidence-v1",
        "cases": [
            {
                "case_id": "CMP-01",
                "claim_kind": "complete",
                "expected_revision": "fixture-v1",
                "prerequisites": [
                    {"claim_id": "claim-read", "verified": True},
                    {"claim_id": "claim-test", "verified": True},
                ],
                "evidence_roles": ["acceptance_verification"],
                "evidence_revision": "fixture-v1",
                "expected": {"verified": True, "code": "complete"},
            },
            {
                "case_id": "CMP-02",
                "claim_kind": "complete",
                "expected_revision": "fixture-v1",
                "prerequisites": [],
                "evidence_roles": [],
                "evidence_revision": "fixture-v1",
                "expected": {"verified": False, "code": "missing-evidence"},
            },
            {
                "case_id": "CMP-03",
                "claim_kind": "complete",
                "expected_revision": "fixture-v2",
                "prerequisites": [],
                "evidence_roles": ["acceptance_verification"],
                "evidence_revision": "fixture-v1",
                "expected": {"verified": False, "code": "stale-evidence"},
            },
            {
                "case_id": "CMP-04",
                "claim_kind": "read",
                "expected_revision": "fixture-v1",
                "prerequisites": [],
                "evidence_roles": [],
                "evidence_revision": "fixture-v1",
                "expected": {"verified": False, "code": "unsupported-claim"},
            },
            {
                "case_id": "CMP-05",
                "claim_kind": "complete",
                "expected_revision": "fixture-v1",
                "prerequisites": [
                    {"claim_id": "claim-read", "verified": False}
                ],
                "evidence_roles": ["acceptance_verification"],
                "evidence_revision": "fixture-v1",
                "expected": {"verified": False, "code": "unverified-prerequisite"},
            },
        ],
        "private_user_data_used": False,
        "external_network_used": False,
    }


def expected_fixtures() -> dict[str, dict[str, Any]]:
    return {
        "planning": planning_fixtures(),
        "reasoning": reasoning_fixtures(),
        "completion": completion_fixtures(),
    }


def semantic_failures(fixtures: dict[str, dict[str, Any]]) -> list[str]:
    failures = []
    planning = fixtures["planning"]
    if [case["case_id"] for case in planning.get("cases", [])] != [
        "PLN-01",
        "PLN-02",
        "PLN-03",
        "PLN-04",
    ]:
        failures.append("planning case closure changed")
    for case in planning.get("cases", []):
        running = sum(step.get("state") == "running" for step in case.get("steps", []))
        if case["expected"]["admitted"] != (running <= 1 and case["case_id"] != "PLN-04"):
            failures.append(f"{case['case_id']}: planning disposition changed")

    reasoning = fixtures["reasoning"]
    if [case["case_id"] for case in reasoning.get("cases", [])] != ["RMD-01", "RMD-02"]:
        failures.append("reasoning case closure changed")
    for case in reasoning.get("cases", []):
        if (
            case.get("authority") != "descriptive-only"
            or not case.get("clarification_gate_required")
            or not case.get("contradiction_check_required")
            or not case.get("independent_verification_required")
            or case.get("private_chain_of_thought_retained")
        ):
            failures.append(f"{case['case_id']}: reasoning invariant changed")
    if reasoning.get("mode_changes_authority") or reasoning.get(
        "mode_changes_evidence_standard"
    ):
        failures.append("reasoning mode broadens authority or evidence")

    completion = fixtures["completion"]
    if [case["case_id"] for case in completion.get("cases", [])] != [
        "CMP-01",
        "CMP-02",
        "CMP-03",
        "CMP-04",
        "CMP-05",
    ]:
        failures.append("completion case closure changed")
    verified = [case["case_id"] for case in completion.get("cases", []) if case["expected"]["verified"]]
    if verified != ["CMP-01"]:
        failures.append("completion evidence disposition changed")

    for family, fixture in fixtures.items():
        if fixture.get("schema_version") != 1:
            failures.append(f"{family}: schema version changed")
        if fixture.get("private_user_data_used") is not False:
            failures.append(f"{family}: private data scope changed")
        if fixture.get("external_network_used") is not False:
            failures.append(f"{family}: network scope changed")
    return failures


def validate_current() -> list[str]:
    expected = expected_fixtures()
    actual = {}
    failures = []
    for family, path in FIXTURE_PATHS.items():
        try:
            actual[family] = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"{family}: cannot read fixture: {error}")
            continue
        if actual[family] != expected[family]:
            failures.append(f"{family}: fixture differs from deterministic source")
    if len(actual) == len(expected):
        failures.extend(semantic_failures(actual))
    return failures


def write_fixtures() -> None:
    for family, fixture in expected_fixtures().items():
        atomic_write(FIXTURE_PATHS[family], canonical_json_bytes(fixture))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_fixtures()
    failures = validate_current()
    if failures:
        for failure in failures:
            print(f"runtime fixture bundle: FAIL: {failure}")
        return 1
    print("runtime fixture bundle: pass (4 planning, 2 reasoning, 5 completion cases)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
