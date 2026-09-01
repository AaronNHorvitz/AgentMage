"""Mutation tests for source-bound Story 23.8 evidence."""

from __future__ import annotations

import copy
import json

from scripts import story_23_8_native_chat_compatibility_evidence as evidence


def test_story_23_8_retained_evidence_is_current() -> None:
    assert evidence.validate() == []


def test_story_23_8_rejects_silent_weakening_and_external_overclaim() -> None:
    value = json.loads(evidence.REPORT_PATH.read_text(encoding="utf-8"))
    for field in (
        "unresolved_reference_model_execution",
        "external_tools_advertised_as_supported",
        "exact_token_usage_claimed",
        "private_or_proposed_production_api",
        "native_agent_host_claim",
        "installed_vsix_version_campaign_complete",
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
