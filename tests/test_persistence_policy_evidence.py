"""Mutation tests for pre-persistence policy evidence."""

import copy
import unittest

from scripts import persistence_policy_evidence as evidence


class PersistencePolicyEvidenceTests(unittest.TestCase):
    def valid(self) -> dict:
        return {"schema_version": 1, "artifact_id": "pre-persistence-policy", "source_revision": "a" * 40, "task_ids": ["11.1.1.4"], "status": "pass-classification-minimization-encryption-retention-receipt", "profile": copy.deepcopy(evidence.PROFILE), "verification_commands": evidence.expected_commands(), "claims": copy.deepcopy(evidence.CLAIMS), "limitations": list(evidence.LIMITATIONS), "private_user_data_used": False, "external_network_used": False, "sources": [{"path": path, "bytes": 1, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS]}

    def test_current_source_and_report_are_valid(self) -> None:
        source = (evidence.ROOT / "kernel/engine/src/persistence.rs").read_text()
        self.assertEqual(evidence.validate_source(source), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_source_and_profile_mutations_fail(self) -> None:
        source = (evidence.ROOT / "kernel/engine/src/persistence.rs").read_text()
        for fragment in evidence.FRAGMENTS:
            self.assertTrue(evidence.validate_source(source.replace(fragment, "removed", 1)))
        report = self.valid()
        report["profile"]["restricted_persistence"] = "allowed"
        self.assertIn("persistence evidence profile changed", evidence.validate_report(report))

    def test_claim_command_revision_and_source_mutations_fail(self) -> None:
        report = self.valid()
        report["claims"].pop("complete_secret_discovery_claimed")
        self.assertIn("persistence evidence claims changed", evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn("persistence evidence verification_commands changed", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("persistence evidence sources changed", evidence.validate_report(report))
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("persistence evidence revision is invalid", evidence.validate_report(report))

    def test_private_data_network_and_limits_are_exact(self) -> None:
        for key in ["private_user_data_used", "external_network_used"]:
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("persistence evidence limitations changed", evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
