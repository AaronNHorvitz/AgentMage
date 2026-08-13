"""Tests for content-free offline-proof workflow evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import offline_proof_evidence as evidence


class OfflineProofEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-offline-proof",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.1.7"],
            "status": "pass-kernel-ledger-and-offline-proof-contract",
            "policy_profile": copy.deepcopy(evidence.POLICY_PROFILE),
            "verification_results": copy.deepcopy(evidence.VERIFICATION_RESULTS),
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

    def test_exact_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_policy_result_or_command_mutation_fails(self) -> None:
        policy = self.valid_report()
        policy["policy_profile"]["ledger_record_limit"] = 4095
        self.assertIn(
            "offline proof policy_profile changed", evidence.validate_report(policy)
        )
        result = self.valid_report()
        result["verification_results"]["lifecycle_and_replay_fail_closed"] = False
        self.assertIn(
            "offline proof verification_results changed",
            evidence.validate_report(result),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "offline proof verification_commands changed",
            evidence.validate_report(command),
        )

    def test_claim_overstatement_and_limitation_mutation_fail(self) -> None:
        claim = self.valid_report()
        claim["claims"]["live_packet_capture_performed"] = True
        self.assertIn("offline proof claims changed", evidence.validate_report(claim))
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "offline proof limitations changed", evidence.validate_report(limitation)
        )

    def test_source_revision_and_source_record_mutation_fail(self) -> None:
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "offline proof source revision is invalid",
            evidence.validate_report(revision),
        )
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "offline proof source evidence changed", evidence.validate_report(source)
        )
        digest = self.valid_report()
        digest["sources"][0]["sha256"] = "invalid"
        self.assertIn(
            "offline proof source records are invalid",
            evidence.validate_report(digest),
        )

    def test_private_data_or_network_use_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "offline proof private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "offline proof external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
