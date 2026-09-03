"""Mutation tests for Sprint 24 gate-owned boundary review."""

from __future__ import annotations

import copy
import json

from scripts import sprint_24_boundary_review as review


def test_retained_review_is_current() -> None:
    assert review.validate(json.loads(review.REPORT.read_text(encoding="utf-8"))) == []


def test_review_rejects_failed_live_observation_and_suppression() -> None:
    value = json.loads(review.REPORT.read_text(encoding="utf-8"))
    failed = copy.deepcopy(value)
    failed["command_exit_code"] = 1
    assert review.validate(failed)
    suppressed = copy.deepcopy(value)
    suppressed["checks"]["host_exposes_denial_not_delivery"] = False
    assert review.validate(suppressed)


def test_review_rejects_human_and_installed_overclaim() -> None:
    value = json.loads(review.REPORT.read_text(encoding="utf-8"))
    human = copy.deepcopy(value)
    human["independent_human_review_performed"] = True
    assert review.validate(human)
    widened = copy.deepcopy(value)
    widened["limitations"] = []
    assert review.validate(widened)
