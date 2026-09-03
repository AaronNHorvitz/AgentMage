"""Mutation tests for the gate-owned Sprint 23 source-boundary review."""

from __future__ import annotations

import copy
import json

from scripts import sprint_23_boundary_review as review


def test_retained_review_is_current() -> None:
    value = json.loads(review.REPORT.read_text(encoding="utf-8"))
    assert review.validate(value) == []


def test_review_rejects_suppression_and_human_overclaim() -> None:
    value = review.build_report("HEAD")
    suppressed = copy.deepcopy(value)
    suppressed["checks"]["automatic_model_substitution_absent"] = False
    assert review.validate(suppressed)
    human = copy.deepcopy(value)
    human["independent_human_review_performed"] = True
    assert review.validate(human)


def test_review_rejects_source_or_requirement_drift() -> None:
    value = review.build_report("HEAD")
    source = copy.deepcopy(value)
    source["source_sha256"][review.SOURCES[0]] = "0" * 64
    assert review.validate(source)
    requirements = copy.deepcopy(value)
    requirements["security_requirement_ids"] = []
    assert review.validate(requirements)
