"""Mutation tests for classification and retention decision-table evidence."""

import copy
import unittest

from scripts import classification_retention_table as table


class ClassificationRetentionTableTests(unittest.TestCase):
    def sources(self) -> tuple[str, str, str]:
        return (
            table.TABLE_PATH.read_text(),
            table.PERSISTENCE_PATH.read_text(),
            table.STORE_PATH.read_text(),
        )

    def valid(self) -> dict:
        return {
            "artifact_id": "classification-retention-decision-table",
            "claims": copy.deepcopy(table.CLAIMS),
            "external_network_used": False,
            "limitations": list(table.LIMITATIONS),
            "private_user_data_used": False,
            "row_count": 24,
            "row_ids": list(table.EXPECTED_IDS),
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in table.SOURCE_PATHS
            ],
            "status": "pass-current-kernel-decision-table",
            "task_ids": ["11.1.2.2"],
            "verification_commands": table.expected_commands(),
        }

    def test_current_table_sources_and_report_are_closed(self) -> None:
        self.assertEqual(table.validate(*self.sources()), [])
        self.assertEqual(table.validate_report(self.valid()), [])

    def test_row_removal_reordering_or_duplication_fails(self) -> None:
        document, persistence, store = self.sources()
        self.assertTrue(table.validate(document.replace("`DC-01`", "`DC-99`", 1), persistence, store))
        self.assertTrue(table.validate(document + "\n`DC-01`\n", persistence, store))
        report = self.valid()
        report["row_ids"].pop()
        self.assertIn("classification-retention evidence row_ids changed", table.validate_report(report))

    def test_limit_and_source_mutations_fail(self) -> None:
        document, persistence, store = self.sources()
        for fragment in table.TABLE_FRAGMENTS:
            self.assertTrue(table.validate(document.replace(fragment, "removed", 1), persistence, store))
        for fragment in table.PERSISTENCE_FRAGMENTS:
            self.assertTrue(table.validate(document, persistence.replace(fragment, "removed", 1), store))
        for fragment in table.STORE_FRAGMENTS:
            self.assertTrue(table.validate(document, persistence, store.replace(fragment, "removed", 1)))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("classification-retention evidence sources changed", table.validate_report(report))

    def test_claim_command_revision_and_scope_mutations_fail(self) -> None:
        report = self.valid()
        report["claims"]["release_support"] = True
        self.assertIn("classification-retention evidence claims changed", table.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn(
            "classification-retention evidence verification_commands changed",
            table.validate_report(report),
        )
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("classification-retention evidence revision is invalid", table.validate_report(report))
        for key in ("private_user_data_used", "external_network_used"):
            report = self.valid()
            report[key] = True
            self.assertTrue(table.validate_report(report))


if __name__ == "__main__":
    unittest.main()
