"""Tests for hostile network-surface injection evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import hostile_network_injection_evidence as evidence


class HostileNetworkInjectionEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-hostile-network-injection",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.3.3", "S-010-ST02"],
            "status": "pass-hostile-surfaces-blocked-before-execution",
            "fixture_sha256": "b" * 64,
            "mutation_profile": copy.deepcopy(evidence.MUTATION_PROFILE),
            "gate_result": evidence.gate.expected_result(),
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

    def test_profile_gate_result_or_command_mutation_fails(self) -> None:
        profile = self.valid_report()
        profile["mutation_profile"]["case_count"] = 7
        self.assertIn(
            "hostile injection mutation_profile changed",
            evidence.validate_report(profile),
        )
        result = self.valid_report()
        result["gate_result"]["injected_code_executed"] = True
        self.assertIn(
            "hostile injection gate_result changed",
            evidence.validate_report(result),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "hostile injection verification_commands changed",
            evidence.validate_report(command),
        )

    def test_claim_or_limitation_overstatement_fails(self) -> None:
        claim = self.valid_report()
        claim["claims"]["runtime_startup_behavior_proven"] = True
        self.assertIn(
            "hostile injection claims changed",
            evidence.validate_report(claim),
        )
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "hostile injection limitations changed",
            evidence.validate_report(limitation),
        )

    def test_source_fixture_identity_or_revision_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "hostile injection source evidence changed",
            evidence.validate_report(source),
        )
        identity = self.valid_report()
        identity["fixture_sha256"] = "c" * 64
        self.assertIn(
            "hostile injection fixture identity changed",
            evidence.validate_report(identity),
        )
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "hostile injection source revision is invalid",
            evidence.validate_report(revision),
        )

    def test_private_data_or_network_use_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "hostile injection private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "hostile injection external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
