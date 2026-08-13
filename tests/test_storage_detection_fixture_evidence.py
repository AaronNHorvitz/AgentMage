"""Tests for strict-local storage-detection fixture evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import storage_detection_fixture_evidence as evidence


class StorageDetectionFixtureEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        sources = [
            {"path": path, "bytes": 1, "sha256": "b" * 64}
            for path in evidence.SOURCE_PATHS
        ]
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-storage-detection-fixtures",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.2.4"],
            "status": "pass-versioned-storage-detection-fixture-corpus",
            "fixture_sha256": "b" * 64,
            "fixture_profile": copy.deepcopy(evidence.FIXTURE_PROFILE),
            "verification_commands": evidence.expected_commands(),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "external_network_used": False,
            "sources": sources,
        }

    def test_exact_fixture_and_report_are_valid(self) -> None:
        self.assertEqual(evidence.validate_fixture(evidence.expected_fixture()), [])
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_fixture_case_mutations_fail(self) -> None:
        for family in ("filesystem_magic", "provider_components", "root_sentinels"):
            changed = copy.deepcopy(evidence.expected_fixture())
            changed[family].pop()
            self.assertEqual(
                evidence.validate_fixture(changed), ["storage detection fixture changed"]
            )

    def test_profile_command_or_claim_mutation_fails(self) -> None:
        profile = self.valid_report()
        profile["fixture_profile"]["known_remote_count"] = 6
        self.assertIn(
            "storage fixture fixture_profile changed",
            evidence.validate_report(profile),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "storage fixture verification_commands changed",
            evidence.validate_report(command),
        )
        claim = self.valid_report()
        claim["claims"]["live_remote_mount_tested"] = True
        self.assertIn("storage fixture claims changed", evidence.validate_report(claim))

    def test_source_fixture_identity_or_revision_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "storage fixture source evidence changed",
            evidence.validate_report(source),
        )
        identity = self.valid_report()
        identity["fixture_sha256"] = "c" * 64
        self.assertIn("storage fixture identity changed", evidence.validate_report(identity))
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "storage fixture source revision is invalid",
            evidence.validate_report(revision),
        )

    def test_private_data_or_network_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "storage fixture private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "storage fixture external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
