#!/usr/bin/env python3
"""Generate and validate the closed Sprint 55 meeting acceptance corpus."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "docs/verification/sprint-55-meeting-corpus.json"


def case(
    identifier: str,
    category: str,
    expected: str,
    requirement: str,
) -> dict[str, str]:
    return {
        "case_id": identifier,
        "category": category,
        "expected": expected,
        "requirement": requirement,
    }


def expected_cases() -> list[dict[str, str]]:
    cases: list[dict[str, str]] = []
    preparation = [
        ("agenda", "purpose"),
        ("agenda", "participants"),
        ("agenda", "topics"),
        ("agenda", "decision_needs"),
        ("agenda", "preparation"),
        ("agenda", "expected_outputs"),
        ("meeting_request", "purpose"),
        ("meeting_request", "participants"),
        ("meeting_request", "topics"),
        ("meeting_request", "decision_needs"),
        ("meeting_request", "preparation"),
        ("meeting_request", "expected_outputs"),
    ]
    for index, (draft_kind, field) in enumerate(preparation, 1):
        cases.append(case(
            f"meeting-plan-{index:02d}",
            "preparation",
            f"sealed {draft_kind} retains source-backed {field}",
            "S-048-I01",
        ))
    invitation_states = ["not_observed", "invited", "accepted", "declined", "tentative", "unknown"]
    attendance_states = ["not_observed", "attended", "partially_attended", "absent", "unknown"]
    for index, state in enumerate(invitation_states, 1):
        cases.append(case(
            f"meeting-invitation-{index:02d}",
            "attendee_state",
            f"invitation state remains {state} with exact evidence state",
            "S-048-I02",
        ))
    for index, state in enumerate(attendance_states, 1):
        cases.append(case(
            f"meeting-attendance-{index:02d}",
            "attendee_state",
            f"attendance state remains {state} with exact evidence state",
            "S-048-I02",
        ))
    transcript_expectations = [
        "verbatim text retained byte-for-byte",
        "cleaned text remains a separate proposal",
        "supplied UTC timestamps retained",
        "missing timestamps remain absent",
        "speaker attribution exposes confidence and evidence state",
        "missing speaker remains unknown with zero confidence",
        "unclear marker matches exact UTF-8 byte range",
        "overlapping unclear marker rejected",
        "changed unclear fragment rejected",
        "source mutation remains false",
    ]
    for index, expected in enumerate(transcript_expectations, 1):
        cases.append(case(
            f"meeting-transcript-{index:02d}",
            "transcript_cleanup",
            expected,
            "S-048-I03",
        ))
    minute_expectations = [
        "confirmed decision requires confirmed evidence",
        "proposed decision cannot carry confirmed evidence",
        "action retains exact source evidence",
        "question retains exact source evidence",
        "risk retains exact source evidence",
        "next meeting remains confirmed proposed or unknown",
        "known owner exposes confirmed or proposed state",
        "known date exposes confirmed or proposed state",
        "missing owner remains absent and unknown",
        "missing date remains absent and unknown",
        "duplicate minutes item identity rejected",
        "assignment effect remains false",
        "communication effect remains false",
        "calendar effect remains false",
    ]
    for index, expected in enumerate(minute_expectations, 1):
        requirement = "S-048-I05" if "owner" in expected or "date" in expected else "S-048-I04"
        cases.append(case(
            f"meeting-minutes-{index:02d}",
            "minutes",
            expected,
            requirement,
        ))
    continuity_expectations = [
        "closeout counts exact sealed minutes",
        "closeout identifies unknown action owners",
        "closeout identifies unknown action dates",
        "open action carries into recurring continuity",
        "open question carries into recurring continuity",
        "open risk carries into recurring continuity",
        "absence from later minutes never implies completion",
        "exact later source may mark completion",
        "exact later source may mark cancellation",
        "exact later source may mark supersession",
        "exact later source may mark state unknown",
        "carry-forward preserves immutable item history",
        "follow-up remains a local draft",
        "follow-up is never sent",
        "continuity never mutates a source",
    ]
    for index, expected in enumerate(continuity_expectations, 1):
        cases.append(case(
            f"meeting-continuity-{index:02d}",
            "continuity",
            expected,
            "S-048-I06",
        ))
    prohibited = [
        "infer invitation response from silence",
        "infer attendance from invitation response",
        "infer invitation response from attendance",
        "invent a speaker",
        "invent a timestamp",
        "invent an owner",
        "invent a due date",
        "invent a confirmed decision",
        "invite a participant",
        "assign work",
        "notify a participant",
        "send a follow-up",
        "schedule a meeting",
        "change a calendar",
        "access an inbox",
        "use a network",
        "mutate a canonical source",
        "broaden privacy or workspace scope",
    ]
    for index, denied in enumerate(prohibited, 1):
        cases.append(case(
            f"meeting-denial-{index:02d}",
            "prohibited_action",
            f"deny {denied}",
            "S-048-ST01",
        ))
    boundary_cases = [
        "ready dependencies and no cancellation admit projection",
        "missing dependency rejects projection before output",
        "cancellation rejects projection before output",
        "sticky cancellation takes precedence over dependency failure",
        "stale sealed minutes reject dependent continuity",
        "wrong recurring series rejects dependent continuity",
    ]
    for index, expected in enumerate(boundary_cases, 1):
        cases.append(case(
            f"meeting-boundary-{index:02d}",
            "dependency_and_cancellation",
            expected,
            "S-048-IT01",
        ))
    return cases


def expected_document() -> dict[str, Any]:
    cases = expected_cases()
    return {
        "schema_version": 1,
        "record_type": "sprint_55_meeting_acceptance_corpus",
        "story_id": "55.1",
        "legacy_story_id": "S-048",
        "case_count": len(cases),
        "cases": cases,
        "authority_effects_enabled": False,
        "network_enabled": False,
        "source_mutation_enabled": False,
        "product_integration_claimed": False,
    }


def validate(value: Any) -> list[str]:
    failures: list[str] = []
    if value != expected_document():
        failures.append("meeting acceptance corpus drifted from the closed inventory")
    encoded = json.dumps(value, sort_keys=True).lower()
    for prohibited in (
        "credential_value",
        "secret_value",
        "private_key",
        "access_token",
        "raw_transcript",
        "raw_message",
        "repository_path",
    ):
        if prohibited in encoded:
            failures.append(f"prohibited corpus field: {prohibited}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    if arguments.write:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(
            json.dumps(expected_document(), indent=2) + "\n", encoding="utf-8"
        )
    if not OUTPUT.is_file():
        print(f"error: missing {OUTPUT.relative_to(ROOT)}")
        return 1
    failures = validate(json.loads(OUTPUT.read_text(encoding="utf-8")))
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print(f"validated {len(expected_cases())} Sprint 55 meeting cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
