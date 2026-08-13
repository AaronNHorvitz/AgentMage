"""Tests for Linux operational-store key-boundary evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import secret_store_key_boundary_evidence as evidence


class SecretStoreKeyBoundaryEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "linux-secret-store-key-boundary",
            "source_revision": "a" * 40,
            "task_ids": ["11.1.1.3"],
            "status": "pass-linux-production-key-boundary",
            "boundary_profile": copy.deepcopy(evidence.BOUNDARY_PROFILE),
            "verification_commands": evidence.expected_commands(),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "external_network_used": False,
            "sources": [
                {"path": path, "bytes": 1, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }

    def test_current_sources_and_exact_report_are_valid(self) -> None:
        for path in evidence.SOURCE_PATHS:
            source = (evidence.ROOT / path).read_text(encoding="utf-8")
            self.assertEqual(evidence.validate_source(path, source), [])
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_required_source_fragment_mutations_fail(self) -> None:
        for path, fragments in evidence.REQUIRED_SOURCE_FRAGMENTS.items():
            source = (evidence.ROOT / path).read_text(encoding="utf-8")
            for fragment in fragments:
                changed = source.replace(fragment, "removed-key-boundary", 1)
                self.assertTrue(evidence.validate_source(path, changed))

    def test_profile_command_or_claim_mutation_fails(self) -> None:
        profile = self.valid_report()
        profile["boundary_profile"]["decoded_key_bytes"] = 16
        self.assertIn(
            "secret-store key boundary_profile changed",
            evidence.validate_report(profile),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "secret-store key verification_commands changed",
            evidence.validate_report(command),
        )
        claim = self.valid_report()
        claim["claims"]["live_secret_service_round_trip_executed"] = True
        self.assertIn("secret-store key claims changed", evidence.validate_report(claim))

    def test_revision_source_or_limitation_mutation_fails(self) -> None:
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "secret-store key source revision is invalid",
            evidence.validate_report(revision),
        )
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "secret-store key source evidence changed",
            evidence.validate_report(source),
        )
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "secret-store key limitations changed",
            evidence.validate_report(limitation),
        )

    def test_private_data_or_network_use_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "secret-store key private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "secret-store key external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
