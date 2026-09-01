"""Mutation tests for source-bound Story 23.7 evidence."""

from __future__ import annotations

import copy
import json

from scripts import story_23_7_verified_chat_evidence as evidence


def test_story_23_7_retained_evidence_is_current() -> None:
    assert evidence.validate() == []


def test_story_23_7_rejects_view_authority_and_external_overclaim() -> None:
    value = json.loads(evidence.REPORT_PATH.read_text(encoding="utf-8"))
    for field in (
        "webview_policy_tool_or_completion_authority",
        "private_vscode_api",
        "hidden_reasoning_persisted_or_rendered",
        "installed_vsix_campaign_complete",
        "assistive_technology_audit_complete",
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
