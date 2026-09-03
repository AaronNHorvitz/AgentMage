"""Mutation tests for the Sprint 50 shared-runtime contract review."""

import copy
import subprocess
import unittest
from unittest.mock import patch

from scripts import sprint_50_runtime_contract_review as review


class Sprint50RuntimeContractReviewTests(unittest.TestCase):
    def test_contract_and_mutations(self) -> None:
        revision = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip()
        with patch.object(
            review,
            "git_bytes",
            side_effect=lambda _revision, path: (review.ROOT / path).read_bytes(),
        ):
            value = review.expected(revision)
        self.assertEqual(value["status"], "PASS_LOCAL_RUNTIME_CONTRACT_REVIEW")
        self.assertEqual(len(value["ceiling_inventory"]), 16)
        self.assertEqual(len(value["recovery_boundary_inventory"]), 10)
        with patch.object(review, "expected", return_value=value):
            self.assertEqual(review.validate(value), [])
            mutations = (
                lambda changed: changed["checks"].update(
                    {next(iter(changed["checks"])): False}
                ),
                lambda changed: changed["source_sha256"].pop(
                    next(iter(changed["source_sha256"]))
                ),
                lambda changed: changed["ceiling_inventory"].pop(),
                lambda changed: changed["recovery_boundary_inventory"].pop(),
                lambda changed: changed.update(
                    {"independent_human_review_performed": True}
                ),
                lambda changed: changed.update({"status": "FAIL"}),
            )
            for mutate in mutations:
                changed = copy.deepcopy(value)
                mutate(changed)
                self.assertTrue(review.validate(changed))


if __name__ == "__main__":
    unittest.main()
