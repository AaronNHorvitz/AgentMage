"""Tests for the Sprint 68 bounded local database contract."""

import unittest

from scripts.local_database_contract import COUNTS, validate


class LocalDatabaseContractTests(unittest.TestCase):
    def test_contract_is_current(self) -> None:
        self.assertEqual(validate(), [])

    def test_corpus_size_is_explicit(self) -> None:
        self.assertEqual(sum(COUNTS.values()), 58)


if __name__ == "__main__":
    unittest.main()
