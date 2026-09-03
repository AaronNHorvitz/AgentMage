"""Mutation tests for the Sprint 45 gate-owned boundary review."""

import copy
import subprocess
import unittest
from unittest.mock import patch

from scripts import sprint_45_boundary_review as review


class Sprint45BoundaryReviewTests(unittest.TestCase):
    def test_contract_and_mutations(self) -> None:
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        value = review.expected(revision)
        self.assertEqual(value["status"], "PASS_LOCAL_BOUNDARY_REVIEW")
        with patch.object(review, "expected", return_value=value):
            self.assertEqual(review.validate(value), [])
        mutations = (
            lambda changed: changed["checks"].update({next(iter(changed["checks"])): False}),
            lambda changed: changed["security_requirement_ids"].pop(),
            lambda changed: changed["source_sha256"].pop(next(iter(changed["source_sha256"]))),
            lambda changed: changed.update({"independent_human_review_performed": True}),
            lambda changed: changed.update({"status": "FAIL"}),
        )
        for mutate in mutations:
            changed = copy.deepcopy(value)
            mutate(changed)
            with patch.object(review, "expected", return_value=value):
                self.assertTrue(review.validate(changed))


if __name__ == "__main__":
    unittest.main()
