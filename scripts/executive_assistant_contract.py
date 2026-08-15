#!/usr/bin/env python3
"""Validate the exact Sprint 54 executive-assistant acceptance corpus."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
CORPUS_PATH: Final = ROOT / "docs/verification/sprint-54-executive-corpus.json"
ZERO_EFFECT_FIELDS: Final = [
    "inbox_access_present",
    "network_access_present",
    "notification_effect_present",
    "send_effect_present",
    "schedule_effect_present",
    "source_mutation_present",
    "authority_grant_present",
    "sprint_gate_closed",
]
EXPECTED: Final = {
    "ranking_dimensions": [
        "consequence", "dependencies", "due_window", "effort", "evidence_state",
        "importance", "schedule", "urgency", "user_preference",
    ],
    "evidence_states": ["confirmed", "disputed", "historical", "inferred", "unknown"],
    "tracker_kinds": ["approvals", "commitments", "deadlines", "decisions", "reminders", "waiting"],
    "view_kinds": [
        "audit", "briefing_pack", "changes_since", "closeout", "decision_brief",
        "forgotten_items", "meeting_brief", "organization_brief", "person_brief",
        "portfolio", "recurring_review", "start_of_cycle", "status",
    ],
    "correspondence_checks": [
        "accidental_commitment", "missing_attachment", "sensitive_content",
        "uncertain_name", "unclear_date", "unanswered_question", "unsupported_claim",
    ],
    "message_triage_classes": [
        "action_required", "decision_required", "duplicate", "reference_only",
        "response_required", "uncertain", "waiting",
    ],
    "prohibited_actions": [
        "automatic_assignment", "automatic_file_mutation", "automatic_notification",
        "automatic_schedule_change", "automatic_send", "hidden_prioritization",
        "inferred_sensitive_traits", "unauthorized_memory",
    ],
    "reconciliation_events": ["conflict", "source_correction", "supersession", "task_completion"],
    "storage_grammars": ["obsidian", "plain_folder"],
    "privacy_operation_cases": [
        f"{privacy}:{operation}"
        for privacy in ("confidential", "highly_restricted", "ordinary", "private")
        for operation in ("export", "index", "retain", "retrieve")
    ],
}


def expected_corpus() -> dict[str, Any]:
    counts = {key: len(values) for key, values in EXPECTED.items()}
    counts["total"] = sum(counts.values())
    return {
        "schema_version": 1,
        "corpus_id": "sprint-54-executive-assistant-v1",
        **EXPECTED,
        "expanded_case_counts": dict(sorted(counts.items())),
        **{field: False for field in ZERO_EFFECT_FIELDS},
    }


def validate(value: Any) -> list[str]:
    failures = []
    expected = expected_corpus()
    if value != expected:
        failures.append("Sprint 54 executive corpus drifted")
    for field in ZERO_EFFECT_FIELDS:
        if not isinstance(value, dict) or value.get(field) is not False:
            failures.append(f"Sprint 54 executive effect overclaim: {field}")
    return failures


def main() -> int:
    value = json.loads(CORPUS_PATH.read_text(encoding="utf-8"))
    failures = validate(value)
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print("Sprint 54 executive-assistant corpus validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
