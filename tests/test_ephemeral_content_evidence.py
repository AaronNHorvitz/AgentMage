"""Mutation tests for structural ephemeral-content evidence."""

import copy
import unittest

from scripts import ephemeral_content_evidence as evidence


class EphemeralContentEvidenceTests(unittest.TestCase):
    def valid(self) -> dict:
        return {
            "artifact_id": "structural-ephemeral-content",
            "claims": copy.deepcopy(evidence.CLAIMS),
            "content_classes": list(evidence.CONTENT_CLASSES),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-structural-ephemeral-defaults",
            "task_ids": ["11.1.1.5"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_source_and_report_are_valid(self) -> None:
        source = (evidence.ROOT / "kernel/engine/src/persistence.rs").read_text()
        self.assertEqual(evidence.validate_source(source), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_source_and_content_class_mutations_fail(self) -> None:
        source = (evidence.ROOT / "kernel/engine/src/persistence.rs").read_text()
        for fragment in evidence.FRAGMENTS:
            self.assertTrue(evidence.validate_source(source.replace(fragment, "removed", 1)))
        report = self.valid()
        report["content_classes"].pop()
        self.assertIn("ephemeral evidence content_classes changed", evidence.validate_report(report))

    def test_claim_command_revision_and_source_mutations_fail(self) -> None:
        report = self.valid()
        report["claims"]["forged_handling_refused"] = False
        self.assertIn("ephemeral evidence claims changed", evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn(
            "ephemeral evidence verification_commands changed",
            evidence.validate_report(report),
        )
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("ephemeral evidence revision is invalid", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("ephemeral evidence sources changed", evidence.validate_report(report))

    def test_private_data_network_limit_and_status_mutations_fail(self) -> None:
        for key in ["private_user_data_used", "external_network_used"]:
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("ephemeral evidence limitations changed", evidence.validate_report(report))
        report = self.valid()
        report["status"] = "pass-release"
        self.assertIn("ephemeral evidence status changed", evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
