from __future__ import annotations

import copy
import unittest
from unittest.mock import patch

from scripts import knowledge_sprint_boundary_review as review


class KnowledgeSprintBoundaryReviewTests(unittest.TestCase):
    def fixture(self, sprint: int) -> dict[str, object]:
        with patch.object(review, "git_bytes", return_value=b"deterministic citation parity missing instruction-like prose are untrusted data ObsidianVaultSelection ObsidianEntryKind::SymbolicLink symlink_metadata apply_watch_batch derived_only source_files_mutated external_process_started network_accessed"):
            return review.expected(sprint, "a" * 40)

    def test_all_three_review_shapes_pass(self) -> None:
        for sprint in review.SOURCES:
            value = self.fixture(sprint)
            with patch.object(review, "expected", return_value=value):
                self.assertEqual(review.validate(value), [])

    def test_suppression_human_and_source_mutations_fail(self) -> None:
        original = self.fixture(28)
        for mutate in (
            lambda value: value["checks"].update({"watch_batch_is_atomic": False}),
            lambda value: value.update({"independent_human_review_performed": True}),
            lambda value: value["source_sha256"].pop(next(iter(value["source_sha256"]))),
        ):
            value = copy.deepcopy(original)
            mutate(value)
            with patch.object(review, "expected", return_value=original):
                self.assertTrue(review.validate(value))


if __name__ == "__main__":
    unittest.main()
