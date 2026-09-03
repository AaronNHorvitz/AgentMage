"""Mutation tests for the Sprint 62 spreadsheet-source review."""

import copy
import subprocess
import unittest
from unittest.mock import patch

from scripts import sprint_62_spreadsheet_source_review as review


class Sprint62SpreadsheetSourceReviewTests(unittest.TestCase):
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
        self.assertEqual(value["status"], "PASS_LOCAL_SPREADSHEET_SOURCE_REVIEW")
        self.assertTrue(all(value["checks"].values()))
        with patch.object(review, "expected", return_value=value):
            self.assertEqual(review.validate(value), [])
            mutations = (
                lambda changed: changed["checks"].update(
                    {next(iter(changed["checks"])): False}
                ),
                lambda changed: changed["source_sha256"].pop(
                    next(iter(changed["source_sha256"]))
                ),
                lambda changed: changed.update(
                    {"independent_human_review_performed": True}
                ),
                lambda changed: changed.update({"limitations": []}),
                lambda changed: changed.update({"status": "FAIL"}),
            )
            for mutate in mutations:
                changed = copy.deepcopy(value)
                mutate(changed)
                self.assertTrue(review.validate(changed))


if __name__ == "__main__":
    unittest.main()
