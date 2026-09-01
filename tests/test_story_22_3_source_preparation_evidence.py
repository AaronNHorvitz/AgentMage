"""Mutation tests for the source-bound Story 22.3 evidence."""

from __future__ import annotations

import copy
import json

from scripts import story_22_3_source_preparation_evidence as evidence


def test_story_22_3_retained_evidence_is_current() -> None:
    assert evidence.validate() == []


def test_story_22_3_golden_rejects_widening_and_missing_native_results() -> None:
    value = json.loads(evidence.GOLDEN_PATH.read_text(encoding="utf-8"))
    widened = copy.deepcopy(value)
    widened["canary_scan"]["network_access_count"] = 1
    assert evidence.validate_golden(widened)
    missing = copy.deepcopy(value)
    missing["native_tool_results"].pop()
    assert evidence.validate_golden(missing)
    path_leak = copy.deepcopy(value)
    path_leak["protected_path"] = "/home/private/source.log"
    assert evidence.validate_golden(path_leak)
