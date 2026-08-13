"""Tests for the reviewable Sprint 10 strict-local policy profile."""

from __future__ import annotations

import copy
import unittest

from scripts import strict_local_policy_profile as profile


class StrictLocalPolicyProfileTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-policy-profile",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.2.1"],
            "status": "pass-reviewable-current-foundation-profile",
            "configuration_sha256": "b" * 64,
            "policy_profile": copy.deepcopy(profile.POLICY_PROFILE),
            "verification_commands": profile.expected_commands(),
            "claims": copy.deepcopy(profile.CLAIMS),
            "limitations": list(profile.LIMITATIONS),
            "private_user_data_used": False,
            "external_network_used": False,
            "sources": [
                {"path": path, "bytes": 1, "sha256": "b" * 64}
                for path in profile.SOURCE_PATHS
            ],
            "dependency_evidence": [
                {"path": path, "bytes": 1, "sha256": "c" * 64}
                for path in profile.DEPENDENCY_EVIDENCE_PATHS
            ],
        }

    def test_exact_profile_is_valid(self) -> None:
        self.assertEqual(profile.validate_report(self.valid_report()), [])

    def test_authority_network_or_storage_mutation_fails(self) -> None:
        for family, key, value in [
            ("authority", "default_effect", "allow"),
            ("network", "configuration_endpoint_count", 1),
            ("storage", "remote_filesystem", "allow"),
        ]:
            changed = self.valid_report()
            changed["policy_profile"][family][key] = value
            self.assertIn(
                "strict-local profile policy_profile changed",
                profile.validate_report(changed),
            )

    def test_claim_command_or_limitation_overstatement_fails(self) -> None:
        claim = self.valid_report()
        claim["claims"]["product_profile_registered"] = True
        self.assertIn("strict-local profile claims changed", profile.validate_report(claim))
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "strict-local profile verification_commands changed",
            profile.validate_report(command),
        )
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "strict-local profile limitations changed",
            profile.validate_report(limitation),
        )

    def test_configuration_and_source_bindings_fail_closed(self) -> None:
        identity = self.valid_report()
        identity["configuration_sha256"] = "d" * 64
        self.assertIn(
            "strict-local profile configuration identity changed",
            profile.validate_report(identity),
        )
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "strict-local profile source evidence changed",
            profile.validate_report(source),
        )
        dependency = self.valid_report()
        dependency["dependency_evidence"].pop()
        self.assertIn(
            "strict-local profile dependency evidence changed",
            profile.validate_report(dependency),
        )

    def test_revision_private_data_and_network_mutations_fail(self) -> None:
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "strict-local profile source revision is invalid",
            profile.validate_report(revision),
        )
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "strict-local profile private_user_data_used changed",
            profile.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "strict-local profile external_network_used changed",
            profile.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
