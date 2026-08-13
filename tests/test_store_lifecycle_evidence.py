"""Mutation tests for encrypted-store lifecycle evidence."""

import copy
import unittest

from scripts import store_lifecycle_evidence as evidence


class StoreLifecycleEvidenceTests(unittest.TestCase):
    def valid(self) -> dict:
        return {
            "artifact_id": "encrypted-store-lifecycle",
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
            "status": "pass-bounded-store-lifecycle",
            "task_ids": ["11.1.1.6"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_valid(self) -> None:
        for path in evidence.FRAGMENTS:
            source = (evidence.ROOT / path).read_text()
            self.assertEqual(evidence.validate_source(path, source), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_source_and_claim_mutations_fail(self) -> None:
        for path, fragments in evidence.FRAGMENTS.items():
            source = (evidence.ROOT / path).read_text()
            for fragment in fragments:
                self.assertTrue(evidence.validate_source(path, source.replace(fragment, "removed", 1)))
        report = self.valid()
        report["claims"]["physical_overwrite_claim"] = True
        self.assertIn("lifecycle evidence claims changed", evidence.validate_report(report))

    def test_command_revision_and_source_mutations_fail(self) -> None:
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn("lifecycle evidence verification_commands changed", evidence.validate_report(report))
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("lifecycle evidence revision is invalid", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("lifecycle evidence sources changed", evidence.validate_report(report))

    def test_private_data_network_limits_and_status_mutations_fail(self) -> None:
        for key in ["private_user_data_used", "external_network_used"]:
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("lifecycle evidence limitations changed", evidence.validate_report(report))
        report = self.valid()
        report["status"] = "pass-release"
        self.assertIn("lifecycle evidence status changed", evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
