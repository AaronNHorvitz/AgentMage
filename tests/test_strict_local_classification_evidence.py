"""Tests for strict-local cross-layer classification evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import strict_local_classification_evidence as evidence


class StrictLocalClassificationEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        sources = [
            {"path": path, "bytes": 1, "sha256": "b" * 64}
            for path in evidence.SOURCE_PATHS
        ]
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-cross-layer-classification",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.3.1", "S-010-UT01"],
            "status": "pass-strict-local-cross-layer-fixture-matrix",
            "boundary_fixture_sha256": "b" * 64,
            "fixture_profile": copy.deepcopy(evidence.FIXTURE_PROFILE),
            "verification_commands": evidence.expected_commands(),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "external_network_used": False,
            "sources": sources,
        }

    def test_exact_fixture_and_report_are_valid(self) -> None:
        self.assertEqual(
            evidence.validate_boundary_fixture(evidence.expected_boundary_fixture()), []
        )
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_fixture_ip_or_policy_mutation_fails(self) -> None:
        for family in ("ip_cases", "policy_cases"):
            changed = copy.deepcopy(evidence.expected_boundary_fixture())
            changed[family].pop()
            self.assertEqual(
                evidence.validate_boundary_fixture(changed), ["boundary fixture changed"]
            )

    def test_profile_command_or_claim_mutation_fails(self) -> None:
        profile = self.valid_report()
        profile["fixture_profile"]["allowed_policy_case_count"] = 3
        self.assertIn(
            "classification evidence fixture_profile changed",
            evidence.validate_report(profile),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "classification evidence verification_commands changed",
            evidence.validate_report(command),
        )
        claim = self.valid_report()
        claim["claims"]["live_remote_mount_tested"] = True
        self.assertIn(
            "classification evidence claims changed",
            evidence.validate_report(claim),
        )

    def test_source_fixture_identity_or_revision_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "classification source evidence changed",
            evidence.validate_report(source),
        )
        identity = self.valid_report()
        identity["boundary_fixture_sha256"] = "c" * 64
        self.assertIn(
            "classification fixture identity changed",
            evidence.validate_report(identity),
        )
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "classification source revision is invalid",
            evidence.validate_report(revision),
        )

    def test_private_data_or_network_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "classification evidence private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "classification evidence external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
