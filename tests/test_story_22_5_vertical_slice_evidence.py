"""Mutation tests for source-bound Story 22.5 evidence."""

from __future__ import annotations

import copy
import json

from scripts import story_22_5_vertical_slice_evidence as evidence


def test_story_22_5_retained_evidence_is_current() -> None:
    assert evidence.validate() == []


def test_story_22_5_golden_rejects_replay_missing_source_and_false_success() -> None:
    value = json.loads(evidence.GOLDEN_PATH.read_text(encoding="utf-8"))
    replay = copy.deepcopy(value)
    replay["replay_count"] = 1
    assert evidence.validate_golden(replay)
    omitted = copy.deepcopy(value)
    omitted["context_manifests"][0]["records"] = []
    assert evidence.validate_golden(omitted)
    false_success = copy.deepcopy(value)
    false_success["outcome"]["answer_evidence"] = None
    assert evidence.validate_golden(false_success)
