"""Tests for the Sprint 64 presentation review contract."""

from __future__ import annotations

import unittest

from scripts.presentation_contract import EXPECTED_COUNTS, validate


class PresentationContractTests(unittest.TestCase):
    """Keep the committed corpus and capability boundary fail closed."""

    def test_contract_is_current(self) -> None:
        self.assertEqual(validate(), [])

    def test_corpus_size_is_explicit(self) -> None:
        self.assertEqual(sum(EXPECTED_COUNTS.values()), 56)


if __name__ == "__main__":
    unittest.main()
