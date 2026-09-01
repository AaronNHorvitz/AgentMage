"""Mutation tests for source-bound Story 22.4 evidence."""

from __future__ import annotations

import copy
import json

from scripts import story_22_4_attempt_recovery_evidence as evidence


def test_story_22_4_retained_evidence_is_current() -> None:
    assert evidence.validate() == []


def test_story_22_4_golden_rejects_replay_missing_boundaries_and_hidden_content() -> None:
    value = json.loads(evidence.GOLDEN_PATH.read_text(encoding="utf-8"))
    replay = copy.deepcopy(value)
    replay["decisions"][0]["replay_allowed"] = True
    assert evidence.validate_golden(replay)
    incomplete = copy.deepcopy(value)
    incomplete["deterministic_seed_count"] = 99
    assert evidence.validate_golden(incomplete)
    hidden = copy.deepcopy(value)
    hidden["model_output"] = "untrusted transcript"
    assert evidence.validate_golden(hidden)
