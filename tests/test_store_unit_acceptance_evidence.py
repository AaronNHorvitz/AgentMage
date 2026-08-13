"""Mutation tests for S-011-UT01 operational-store unit evidence."""

import copy
import unittest

from scripts import store_unit_acceptance_evidence as evidence


class StoreUnitAcceptanceEvidenceTests(unittest.TestCase):
    def sources(self) -> tuple[str, str]:
        return evidence.DOCUMENT_PATH.read_text(), evidence.STORE_PATH.read_text()

    def valid(self) -> dict:
        return {
            "acceptance_matrix_ids": list(evidence.MATRIX_IDS),
            "artifact_id": "s-011-ut01-operational-store-unit-results",
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "focused_tests": list(evidence.EXPECTED_TESTS),
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-deterministic-store-unit-boundary",
            "task_ids": ["11.1.3.1", "S-011-UT01"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(*self.sources()), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_matrix_test_set_and_fragment_mutations_fail(self) -> None:
        document, store = self.sources()
        self.assertIn(
            "S-011-UT01 acceptance matrix changed",
            evidence.validate_sources(document.replace("`UT-03`", "`UT-99`", 1), store),
        )
        self.assertIn(
            "S-011-UT01 focused test closure changed",
            evidence.validate_sources(
                document,
                store.replace("#[test]\n    fn encrypted_store_requires_key", "fn encrypted_store_requires_key", 1),
            ),
        )
        for fragment in evidence.DOCUMENT_FRAGMENTS:
            self.assertTrue(evidence.validate_sources(document.replace(fragment, "removed", 1), store))
        for fragment in evidence.STORE_FRAGMENTS:
            self.assertTrue(evidence.validate_sources(document, store.replace(fragment, "removed", 1)))

    def test_completion_limit_and_command_mutations_fail(self) -> None:
        report = self.valid()
        report["claims"]["multiprocess_writer_evidence"] = True
        self.assertIn("S-011-UT01 evidence claims changed", evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("S-011-UT01 evidence limitations changed", evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn(
            "S-011-UT01 evidence verification_commands changed",
            evidence.validate_report(report),
        )

    def test_revision_source_and_data_scope_mutations_fail(self) -> None:
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("S-011-UT01 evidence revision is invalid", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("S-011-UT01 evidence sources changed", evidence.validate_report(report))
        for key in ("private_user_data_used", "external_network_used"):
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
