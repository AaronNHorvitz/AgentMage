"""Mutation tests for the classification and retention decision table."""

import unittest

from scripts import classification_retention_table as table


class ClassificationRetentionTableTests(unittest.TestCase):
    def sources(self) -> tuple[str, str, str]:
        return (
            table.TABLE_PATH.read_text(),
            table.PERSISTENCE_PATH.read_text(),
            table.STORE_PATH.read_text(),
        )

    def test_current_table_and_sources_are_closed(self) -> None:
        self.assertEqual(table.validate(*self.sources()), [])
        report = table.report()
        self.assertEqual(report["row_count"], 24)
        self.assertFalse(report["release_support"])

    def test_row_removal_reordering_or_duplication_fails(self) -> None:
        document, persistence, store = self.sources()
        self.assertTrue(table.validate(document.replace("`DC-01`", "`DC-99`", 1), persistence, store))
        self.assertTrue(table.validate(document + "\n`DC-01`\n", persistence, store))

    def test_limit_and_source_mutations_fail(self) -> None:
        document, persistence, store = self.sources()
        for fragment in table.TABLE_FRAGMENTS:
            self.assertTrue(table.validate(document.replace(fragment, "removed", 1), persistence, store))
        for fragment in table.PERSISTENCE_FRAGMENTS:
            self.assertTrue(table.validate(document, persistence.replace(fragment, "removed", 1), store))
        for fragment in table.STORE_FRAGMENTS:
            self.assertTrue(table.validate(document, persistence, store.replace(fragment, "removed", 1)))


if __name__ == "__main__":
    unittest.main()
