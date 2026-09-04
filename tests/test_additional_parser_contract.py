"""Tests for the Sprint 66 additional-parser contract."""

from __future__ import annotations

import unittest

from scripts.additional_parser_contract import EXPECTED_COUNTS, validate


class AdditionalParserContractTests(unittest.TestCase):
    def test_contract_is_current(self) -> None:
        self.assertEqual(validate(), [])

    def test_corpus_size_is_explicit(self) -> None:
        self.assertEqual(sum(EXPECTED_COUNTS.values()), 140)


if __name__ == "__main__":
    unittest.main()
