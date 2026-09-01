"""Mutation tests for source-bound Story 23.6 evidence."""

from __future__ import annotations

import copy
import json

from scripts import story_23_6_workflow_supervisor_evidence as evidence


def test_story_23_6_retained_evidence_is_current() -> None:
    assert evidence.validate() == []


def test_story_23_6_report_rejects_second_loop_replay_authority_and_external_overclaim() -> None:
    value = json.loads(evidence.REPORT_PATH.read_text(encoding="utf-8"))
    for field in (
        "second_model_or_tool_loop",
        "uncertain_effect_replay",
        "client_or_model_completion_authority",
        "presentation_changes_canonical_result",
        "qualified_production_model_complete",
        "independent_review_complete",
        "windows_validation_complete",
        "macos_validation_complete",
    ):
        mutated = copy.deepcopy(value)
        mutated["product_truth"][field] = True
        assert evidence.validate_report(mutated)
    release = copy.deepcopy(value)
    release["product_truth"]["release_claim"] = "pass"
    assert evidence.validate_report(release)
