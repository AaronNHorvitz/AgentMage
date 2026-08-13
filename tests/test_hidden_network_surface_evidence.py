"""Tests for hidden network-surface evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import hidden_network_surface_evidence as evidence


class HiddenNetworkSurfaceEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-hidden-network-surfaces",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.1.6"],
            "status": "pass-static-source-artifact-manifest-and-dependency-gate",
            "policy_profile": copy.deepcopy(evidence.POLICY_PROFILE),
            "mutation_results": copy.deepcopy(evidence.MUTATION_RESULTS),
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

    def test_policy_mutation_or_missing_command_fails(self) -> None:
        policy = self.valid_report()
        policy["policy_profile"]["reviewed_cargo_package_identity_count"] = 61
        self.assertIn("hidden network policy_profile changed", evidence.validate_report(policy))
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "hidden network verification_commands changed",
            evidence.validate_report(command),
        )

    def test_mutation_result_or_claim_overstatement_fails(self) -> None:
        mutation = self.valid_report()
        mutation["mutation_results"]["compiled_artifact_telemetry_rejected"] = False
        self.assertIn(
            "hidden network mutation_results changed",
            evidence.validate_report(mutation),
        )
        claim = self.valid_report()
        claim["claims"]["packet_capture_performed"] = True
        self.assertIn("hidden network claims changed", evidence.validate_report(claim))

    def test_limitation_source_and_revision_mutation_fails(self) -> None:
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn("hidden network limitations changed", evidence.validate_report(limitation))
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn("hidden network source evidence changed", evidence.validate_report(source))
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "hidden network source revision is invalid",
            evidence.validate_report(revision),
        )

    def test_private_data_or_network_use_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "hidden network private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "hidden network external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
