"""Mutation tests for the gate-owned Sprint 26 boundary review."""

from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import sprint_26_boundary_review as review


def report() -> dict[str, object]:
    fixtures = {
        "capabilities/knowledge/src/authority.rs": b"KnowledgeDataOwner KnowledgeStorageRule recovery_source CanonicalMarkdown UserOwnedMarkdown DisposableSqlite",
        "capabilities/knowledge/src/index.rs": b"derived_only",
        "capabilities/knowledge/src/lifecycle.rs": b"KnowledgeRestorePlan KnowledgeMigrationPlan",
        "capabilities/knowledge/src/operations.rs": b"preview only",
        "capabilities/knowledge/src/store.rs": b"KnowledgeStore no create, update, delete, move, filesystem, or operational-store method",
        "docs/architecture/knowledge-authority-boundary.md": b"no filesystem handle Operational sessions v0.3 grant path",
    }
    with patch.object(review, "git_bytes", side_effect=lambda _revision, path: fixtures[path]):
        return review.expected("a" * 40)


class Sprint26BoundaryReviewTests(unittest.TestCase):
    def test_complete_review_passes(self) -> None:
        with patch.object(review, "expected", return_value=report()):
            self.assertEqual(review.validate(report()), [])

    def test_suppression_human_claim_and_source_drift_fail(self) -> None:
        for mutate in (
            lambda value: value["checks"].update({"canonical_markdown_is_user_owned": False}),
            lambda value: value.update({"independent_human_review_performed": True}),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        ):
            value = copy.deepcopy(report())
            mutate(value)
            with patch.object(review, "expected", return_value=report()):
                self.assertTrue(review.validate(value))


if __name__ == "__main__":
    unittest.main()
