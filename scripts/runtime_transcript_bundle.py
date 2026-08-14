#!/usr/bin/env python3
"""Generate and validate Task 12.1.2.3 synthetic runtime transcripts."""

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
FIXTURE_PATH: Final = ROOT / "fixtures/runtime/v1/session-transcripts.json"
CASE_IDS: Final = tuple(f"TRN-{index:02}" for index in range(1, 13))
FINAL_STATES: Final = ("completed", "failed", "blocked", "unknown", "not_run", "cancelled")
RECORD_TYPES: Final = {
    "cancellation_signal",
    "final_response",
    "message_disposition",
    "progress_event",
    "status_response",
    "user_message",
    "validation_refusal",
}
TASK_ID: Final = "transcript-task"
PLAN_ID: Final = "transcript-plan"
STEP_ID: Final = "transcript-step-1"


def status_response(
    revision: int,
    status: str,
    active_step_id: str | None,
    completed_steps: int,
) -> dict[str, Any]:
    return {
        "record_type": "status_response",
        "schema_version": 1,
        "task_id": TASK_ID,
        "plan_id": PLAN_ID,
        "plan_revision": revision,
        "status": status,
        "active_step_id": active_step_id,
        "completed_steps": completed_steps,
        "total_steps": 2,
    }


def user_message(intent: str, cancellation_supplied: bool) -> dict[str, Any]:
    return {
        "record_type": "user_message",
        "intent": intent,
        "cancellation_supplied": cancellation_supplied,
    }


def evidence_reference() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "evidence_id": "transcript-evidence-1",
        "kind": "validation",
        "source_id": "synthetic-runtime-fixture",
        "object_id": "acceptance-check-1",
        "fragment": None,
        "content_sha256": "1" * 64,
        "observed_revision": "fixture-v1",
    }


def final_response(state: str) -> dict[str, Any]:
    completed = state == "completed"
    unresolved = {
        "completed": [],
        "failed": ["A typed synthetic operation failed."],
        "blocked": ["A synthetic dependency requires a decision."],
        "unknown": ["Available synthetic evidence cannot establish the result."],
        "not_run": ["The synthetic operation was not attempted."],
        "cancelled": ["The synthetic operation was cancelled."],
    }[state]
    return {
        "record_type": "final_response",
        "schema_version": 1,
        "task_id": TASK_ID,
        "state": state,
        "summary": f"Synthetic terminal result: {state}.",
        "evidence": [evidence_reference()] if completed else [],
        "unresolved": unresolved,
        "claims_unfinished_work_succeeded": False,
    }


def transcript_fixture() -> dict[str, Any]:
    cases: list[dict[str, Any]] = [
        {
            "case_id": "TRN-01",
            "category": "status",
            "initial_status": status_response(3, "running", STEP_ID, 0),
            "transcript": [
                user_message("status_query", False),
                status_response(3, "running", STEP_ID, 0),
            ],
            "expected": {
                "admitted": True,
                "code": "status_returned",
                "plan_revision_delta": 0,
                "descendant_cancellation_count": 0,
                "claims_unfinished_work_succeeded": False,
            },
        },
        {
            "case_id": "TRN-02",
            "category": "interruption",
            "initial_status": status_response(3, "running", STEP_ID, 0),
            "transcript": [
                user_message("replace_task", True),
                {
                    "record_type": "cancellation_signal",
                    "signal": {
                        "schema_version": 1,
                        "cancellation_id": "replace-signal",
                        "correlation_id": "replace-correlation",
                        "task_id": TASK_ID,
                        "reason": "user_requested",
                        "requested_by": "shell",
                    },
                    "propagated_to": ["kernel", "platform_adapter", "tool"],
                },
                {
                    "record_type": "message_disposition",
                    "disposition": "interrupted_for_replacement",
                },
                {
                    "record_type": "progress_event",
                    "schema_version": 1,
                    "sequence": 4,
                    "task_id": TASK_ID,
                    "plan_id": PLAN_ID,
                    "plan_revision": 4,
                    "plan_step_id": None,
                    "kind": "cancelled",
                },
                status_response(4, "cancelled", None, 0),
                final_response("cancelled"),
            ],
            "expected": {
                "admitted": True,
                "code": "interrupted_for_replacement",
                "plan_revision_delta": 1,
                "descendant_cancellation_count": 3,
                "claims_unfinished_work_succeeded": False,
            },
        },
        {
            "case_id": "TRN-03",
            "category": "interruption",
            "initial_status": status_response(3, "running", STEP_ID, 0),
            "transcript": [
                user_message("extend_task", True),
                {
                    "record_type": "cancellation_signal",
                    "signal": {
                        "schema_version": 1,
                        "cancellation_id": "extend-signal",
                        "correlation_id": "extend-correlation",
                        "task_id": TASK_ID,
                        "reason": "user_requested",
                        "requested_by": "shell",
                    },
                    "propagated_to": ["kernel", "model"],
                },
                {
                    "record_type": "message_disposition",
                    "disposition": "interrupted_for_extension",
                },
                {
                    "record_type": "progress_event",
                    "schema_version": 1,
                    "sequence": 4,
                    "task_id": TASK_ID,
                    "plan_id": PLAN_ID,
                    "plan_revision": 4,
                    "plan_step_id": None,
                    "kind": "cancelled",
                },
                status_response(4, "cancelled", None, 0),
                final_response("cancelled"),
            ],
            "expected": {
                "admitted": True,
                "code": "interrupted_for_extension",
                "plan_revision_delta": 1,
                "descendant_cancellation_count": 2,
                "claims_unfinished_work_succeeded": False,
            },
        },
        {
            "case_id": "TRN-04",
            "category": "refusal",
            "initial_status": status_response(3, "running", STEP_ID, 0),
            "transcript": [
                user_message("replace_task", False),
                {"record_type": "validation_refusal", "code": "cancellation_required"},
                status_response(3, "running", STEP_ID, 0),
            ],
            "expected": {
                "admitted": False,
                "code": "cancellation_required",
                "plan_revision_delta": 0,
                "descendant_cancellation_count": 0,
                "claims_unfinished_work_succeeded": False,
            },
        },
        {
            "case_id": "TRN-05",
            "category": "refusal",
            "initial_status": status_response(3, "running", STEP_ID, 0),
            "transcript": [
                user_message("status_query", True),
                {"record_type": "validation_refusal", "code": "unexpected_cancellation"},
                status_response(3, "running", STEP_ID, 0),
            ],
            "expected": {
                "admitted": False,
                "code": "unexpected_cancellation",
                "plan_revision_delta": 0,
                "descendant_cancellation_count": 0,
                "claims_unfinished_work_succeeded": False,
            },
        },
        {
            "case_id": "TRN-06",
            "category": "refusal",
            "initial_status": status_response(5, "completed", None, 2),
            "transcript": [
                user_message("replace_task", True),
                {"record_type": "validation_refusal", "code": "plan_not_interruptible"},
                status_response(5, "completed", None, 2),
            ],
            "expected": {
                "admitted": False,
                "code": "plan_not_interruptible",
                "plan_revision_delta": 0,
                "descendant_cancellation_count": 0,
                "claims_unfinished_work_succeeded": False,
            },
        },
    ]
    for offset, state in enumerate(FINAL_STATES, start=7):
        cases.append(
            {
                "case_id": f"TRN-{offset:02}",
                "category": "final_response",
                "initial_status": None,
                "transcript": [final_response(state)],
                "expected": {
                    "admitted": True,
                    "code": state,
                    "plan_revision_delta": 0,
                    "descendant_cancellation_count": 0,
                    "claims_unfinished_work_succeeded": False,
                },
            }
        )
    return {
        "schema_version": 1,
        "fixture_set_id": "agentmage-runtime-transcripts-v1",
        "contract_authority": "descriptive-only",
        "case_count": len(cases),
        "cases": cases,
        "private_user_data_used": False,
        "external_network_used": False,
        "live_model_tool_or_platform_execution": False,
    }


def _records(case: dict[str, Any], record_type: str) -> list[dict[str, Any]]:
    return [record for record in case.get("transcript", []) if record.get("record_type") == record_type]


def semantic_failures(fixture: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    cases = fixture.get("cases", [])
    if [case.get("case_id") for case in cases] != list(CASE_IDS):
        failures.append("transcript case closure changed")
        return failures
    if fixture.get("case_count") != len(CASE_IDS):
        failures.append("transcript case count changed")
    for case in cases:
        case_id = case["case_id"]
        transcript = case.get("transcript", [])
        if not transcript or any(record.get("record_type") not in RECORD_TYPES for record in transcript):
            failures.append(f"{case_id}: transcript record closure changed")
        if case.get("expected", {}).get("claims_unfinished_work_succeeded") is not False:
            failures.append(f"{case_id}: expected success claim changed")
        if any(record.get("claims_unfinished_work_succeeded") for record in transcript):
            failures.append(f"{case_id}: transcript claims unfinished success")

    status_case = cases[0]
    if (
        [record["record_type"] for record in status_case["transcript"]]
        != ["user_message", "status_response"]
        or status_case["transcript"][0].get("intent") != "status_query"
        or status_case["expected"].get("plan_revision_delta") != 0
        or _records(status_case, "cancellation_signal")
    ):
        failures.append("TRN-01: status query interrupted work")

    for case in cases[1:3]:
        record_types = [record["record_type"] for record in case["transcript"]]
        cancellations = _records(case, "cancellation_signal")
        progress = _records(case, "progress_event")
        statuses = _records(case, "status_response")
        if record_types != [
            "user_message",
            "cancellation_signal",
            "message_disposition",
            "progress_event",
            "status_response",
            "final_response",
        ]:
            failures.append(f"{case['case_id']}: interruption ordering changed")
        signal = cancellations[0].get("signal", {}) if len(cancellations) == 1 else {}
        if (
            len(cancellations) != 1
            or set(signal) != {
                "schema_version",
                "cancellation_id",
                "correlation_id",
                "task_id",
                "reason",
                "requested_by",
            }
            or signal.get("schema_version") != 1
            or signal.get("task_id") != TASK_ID
            or signal.get("reason") != "user_requested"
            or signal.get("requested_by") != "shell"
            or len(cancellations[0].get("propagated_to", []))
            != case["expected"].get("descendant_cancellation_count")
            or case["expected"].get("plan_revision_delta") != 1
            or len(progress) != 1
            or progress[0].get("kind") != "cancelled"
            or len(statuses) != 1
            or statuses[0].get("status") != "cancelled"
            or statuses[0].get("plan_revision") != case["initial_status"].get("plan_revision") + 1
        ):
            failures.append(f"{case['case_id']}: cancellation result changed")

    for case in cases[3:6]:
        statuses = _records(case, "status_response")
        if (
            [record["record_type"] for record in case["transcript"]]
            != ["user_message", "validation_refusal", "status_response"]
            or len(statuses) != 1
            or statuses[0] != case.get("initial_status")
            or _records(case, "cancellation_signal")
            or _records(case, "progress_event")
            or case["expected"].get("plan_revision_delta") != 0
        ):
            failures.append(f"{case['case_id']}: refusal changed state")

    final_cases = cases[6:]
    if [case["transcript"][0].get("state") for case in final_cases] != list(FINAL_STATES):
        failures.append("final-response state closure changed")
    for case in final_cases:
        response = case["transcript"][0]
        if response.get("record_type") != "final_response":
            failures.append(f"{case['case_id']}: final response missing")
            continue
        state = response.get("state")
        if state == "completed" and (not response.get("evidence") or response.get("unresolved")):
            failures.append(f"{case['case_id']}: completed evidence contract changed")
        if state == "not_run" and response.get("evidence"):
            failures.append(f"{case['case_id']}: not-run evidence contract changed")
        if state != "completed" and not response.get("unresolved"):
            failures.append(f"{case['case_id']}: non-complete uncertainty hidden")

    if fixture.get("schema_version") != 1:
        failures.append("transcript schema version changed")
    if fixture.get("contract_authority") != "descriptive-only":
        failures.append("transcript authority changed")
    for key in (
        "private_user_data_used",
        "external_network_used",
        "live_model_tool_or_platform_execution",
    ):
        if fixture.get(key) is not False:
            failures.append(f"transcript {key} changed")
    return failures


def validate_current() -> list[str]:
    try:
        actual = json.loads(FIXTURE_PATH.read_text())
    except (OSError, json.JSONDecodeError) as error:
        return [f"cannot read transcript fixture: {error}"]
    failures = semantic_failures(actual)
    if actual != transcript_fixture():
        failures.append("transcript fixture differs from deterministic source")
    return failures


def write_fixture() -> None:
    atomic_write(FIXTURE_PATH, canonical_json_bytes(transcript_fixture()))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        write_fixture()
    failures = validate_current()
    if failures:
        for failure in failures:
            print(f"runtime transcript bundle: FAIL: {failure}")
        return 1
    print("runtime transcript bundle: pass (12 transcripts, 6 final states)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
