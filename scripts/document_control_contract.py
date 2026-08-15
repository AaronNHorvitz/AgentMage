#!/usr/bin/env python3
"""Generate and validate the closed Sprint 56 document-control corpus."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
OUTPUT: Final = ROOT / "docs/verification/sprint-56-document-control-corpus.json"


def case(identifier: str, category: str, expected: str, requirement: str) -> dict[str, str]:
    return {
        "case_id": identifier,
        "category": category,
        "expected": expected,
        "requirement": requirement,
    }


def expected_cases() -> list[dict[str, str]]:
    cases: list[dict[str, str]] = []
    register_fields = [
        "version", "approval", "attachments", "commitments", "deadlines", "content_hash",
        "source_path", "record_category", "retention_schedule", "supersession_links",
    ]
    for index, field in enumerate(register_fields, 1):
        cases.append(case(
            f"records-register-{index:02d}", "register",
            f"register preserves exact {field}", "S-048-I07",
        ))
    statement_classes = [
        "verbatim_source", "observed_fact", "derived_action", "inferred_summary",
        "unresolved_conflict", "user_approved_final_language",
    ]
    for index, statement_class in enumerate(statement_classes, 1):
        cases.append(case(
            f"records-truth-{index:02d}", "truth_class",
            f"statement remains {statement_class} with its exact evidence state", "S-048-UT01",
        ))
    workflows = [
        "naming", "duplicate", "superseded", "final_copy", "quality", "deadline",
        "routing_slip", "mail_merge_preview", "calendar_file_draft", "filing_suggestion",
    ]
    for index, workflow in enumerate(workflows, 1):
        cases.append(case(
            f"records-workflow-{index:02d}", "workflow",
            f"{workflow} produces a local-only report", "S-048-I08",
        ))
    validation_fields = [
        "name", "date", "owner", "quorum_or_status", "attachment", "version",
        "record_category", "filing_destination", "retention",
    ]
    for index, field in enumerate(validation_fields, 1):
        cases.append(case(
            f"records-validation-{index:02d}", "field_validation",
            f"empty boundary and conflicting {field} remains visible", "S-048-UT02",
        ))
    for index, action in enumerate(["save_draft", "rename", "move", "file"], 1):
        cases.append(case(
            f"records-action-{index:02d}", "action_preview",
            f"{action} requires an exact sealed preview and separate approval", "S-048-I09",
        ))
    for index, field in enumerate(
        ["destination", "content", "metadata", "record_category", "retention", "final_decision"],
        1,
    ):
        cases.append(case(
            f"records-approval-{index:02d}", "approval",
            f"approval binds and confirms exact {field}", "S-048-I09",
        ))
    adversarial = [
        "unsupported identity claim remains unresolved",
        "hidden recipient is rejected",
        "malicious attachment remains untrusted data",
        "embedded prompt instruction creates no authority",
        "sensitive content cannot broaden disclosure",
        "record-disposition request remains records-owner decision",
        "invented attribution is rejected",
    ]
    for index, expected in enumerate(adversarial, 1):
        cases.append(case(
            f"records-adversarial-{index:02d}", "adversarial", expected, "S-048-ST01",
        ))
    integration = [
        "draft retains exact source links",
        "review records corrections without source mutation",
        "corrected version receives a new digest",
        "final copy binds exact approval and final language",
        "filing remains an unexecuted preview",
    ]
    for index, expected in enumerate(integration, 1):
        cases.append(case(
            f"records-integration-{index:02d}", "integration", expected, "S-048-IT01",
        ))
    prohibited = [
        "recipient selection", "sending", "scheduling", "notification", "saving", "renaming",
        "moving", "deleting", "filing", "records disposition", "silent final-language change",
    ]
    for index, effect in enumerate(prohibited, 1):
        cases.append(case(
            f"records-denial-{index:02d}", "prohibited_effect", f"deny {effect}", "S-048-ST01",
        ))
    return cases


def expected_document() -> dict[str, Any]:
    cases = expected_cases()
    return {
        "schema_version": 1,
        "record_type": "sprint_56_document_control_acceptance_corpus",
        "story_id": "56.1",
        "legacy_story_id": "S-048",
        "case_count": len(cases),
        "cases": cases,
        "external_effects_enabled": False,
        "records_disposition_enabled": False,
        "network_enabled": False,
        "product_integration_claimed": False,
    }


def validate(value: Any) -> list[str]:
    failures = []
    if value != expected_document():
        failures.append("document-control corpus drifted from the closed inventory")
    encoded = json.dumps(value, sort_keys=True).lower()
    for prohibited in (
        "credential_value", "secret_value", "private_key", "access_token", "raw_document",
        "raw_message", "repository_path",
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
        OUTPUT.write_text(json.dumps(expected_document(), indent=2) + "\n", encoding="utf-8")
    if not OUTPUT.is_file():
        print(f"error: missing {OUTPUT.relative_to(ROOT)}")
        return 1
    failures = validate(json.loads(OUTPUT.read_text(encoding="utf-8")))
    if failures:
        for failure in failures:
            print(f"error: {failure}")
        return 1
    print(f"validated {len(expected_cases())} Sprint 56 document-control cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
