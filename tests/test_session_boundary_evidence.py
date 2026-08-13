"""Tests for strict-local Linux session-boundary evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import session_boundary_evidence as evidence


class SessionBoundaryEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-linux-session-boundary",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.1.5"],
            "status": "pass-linux-point-in-time-session-reconciliation",
            "policy_profile": copy.deepcopy(evidence.POLICY_PROFILE),
            "integration_results": copy.deepcopy(evidence.RESULTS),
            "verification_commands": evidence.expected_commands(),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "external_network_used": False,
            "local_disposable_process_used": True,
            "sources": [
                {"path": path, "bytes": 1, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
        }

    def test_exact_report_is_valid(self) -> None:
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_policy_command_or_result_mutation_fails(self) -> None:
        policy = self.valid_report()
        policy["policy_profile"]["inventory_scope"] = "caller-selected"
        self.assertIn("session boundary policy_profile changed", evidence.validate_report(policy))
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "session boundary verification_commands changed",
            evidence.validate_report(command),
        )
        result = self.valid_report()
        result["integration_results"]["extra_process_rejected"] = False
        self.assertIn(
            "session boundary integration_results changed",
            evidence.validate_report(result),
        )

    def test_claim_or_limitation_overstatement_fails(self) -> None:
        claim = self.valid_report()
        claim["claims"]["packet_capture_performed"] = True
        self.assertIn("session boundary claims changed", evidence.validate_report(claim))
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "session boundary limitations changed",
            evidence.validate_report(limitation),
        )

    def test_source_identity_and_hash_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "session boundary source evidence changed",
            evidence.validate_report(source),
        )
        digest = self.valid_report()
        digest["sources"][0]["sha256"] = "invalid"
        self.assertIn(
            "session boundary source records are invalid",
            evidence.validate_report(digest),
        )
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "session boundary source revision is invalid",
            evidence.validate_report(revision),
        )

    def test_execution_classification_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "session boundary private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "session boundary external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
