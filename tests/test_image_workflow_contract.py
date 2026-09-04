"""Tests for the Sprint 65 image and visual-workflow contract."""

from __future__ import annotations

import unittest

from scripts.image_workflow_contract import EXPECTED_COUNTS, validate


class ImageWorkflowContractTests(unittest.TestCase):
    """Keep the committed corpus and capability boundary fail closed."""

    def test_contract_is_current(self) -> None:
        self.assertEqual(validate(), [])

    def test_corpus_size_is_explicit(self) -> None:
        self.assertEqual(sum(EXPECTED_COUNTS.values()), 102)


if __name__ == "__main__":
    unittest.main()
