"""Mutation tests for source-bound Story 23.5 evidence."""

from __future__ import annotations

import copy
import json

from scripts import story_23_5_participant_ingress_evidence as evidence


def test_story_23_5_retained_evidence_is_current() -> None:
    assert evidence.validate() == []


def test_story_23_5_report_rejects_silent_drop_authority_and_external_overclaim() -> None:
    value = json.loads(evidence.REPORT_PATH.read_text(encoding="utf-8"))
    silent = copy.deepcopy(value)
    silent["product_truth"]["silent_provider_part_drop"] = True
    assert evidence.validate_report(silent)
    authority = copy.deepcopy(value)
    authority["product_truth"]["typescript_parser_or_runtime_authority"] = True
    assert evidence.validate_report(authority)
    external = copy.deepcopy(value)
    external["product_truth"]["installed_vsix_campaign_complete"] = True
    assert evidence.validate_report(external)
    release = copy.deepcopy(value)
    release["product_truth"]["release_claim"] = "pass"
    assert evidence.validate_report(release)
