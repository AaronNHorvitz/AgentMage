"""Mutation tests for one-way JSON Lines export evidence."""

import copy
import unittest

from scripts import derived_export_evidence as evidence


class DerivedExportEvidenceTests(unittest.TestCase):
    def valid(self) -> dict:
        return {
            "artifact_id": "derived-json-lines-export",
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-one-way-derived-export",
            "task_ids": ["11.1.1.7"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_source_and_report_are_valid(self) -> None:
        source = (evidence.ROOT / "kernel/engine/src/operational_store.rs").read_text()
        self.assertEqual(evidence.validate_source(source), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_source_claim_and_forbidden_authority_mutations_fail(self) -> None:
        source = (evidence.ROOT / "kernel/engine/src/operational_store.rs").read_text()
        for fragment in evidence.FRAGMENTS:
            self.assertTrue(evidence.validate_source(source.replace(fragment, "removed", 1)))
        self.assertTrue(evidence.validate_source(source + "\npub fn import_json_lines() {}\n"))
        report = self.valid()
        report["claims"]["dual_write_present"] = True
        self.assertIn("derived export evidence claims changed", evidence.validate_report(report))

    def test_command_revision_and_source_mutations_fail(self) -> None:
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn("derived export evidence verification_commands changed", evidence.validate_report(report))
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("derived export evidence revision is invalid", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("derived export evidence sources changed", evidence.validate_report(report))

    def test_private_data_network_limits_and_status_mutations_fail(self) -> None:
        for key in ["private_user_data_used", "external_network_used"]:
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("derived export evidence limitations changed", evidence.validate_report(report))
        report = self.valid()
        report["status"] = "pass-release"
        self.assertIn("derived export evidence status changed", evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
