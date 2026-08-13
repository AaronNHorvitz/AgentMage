"""Tests for strict-local Linux state-root evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import strict_local_state_root_evidence as evidence


class StrictLocalStateRootEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "strict-local-linux-state-root-policy",
            "source_revision": "a" * 40,
            "task_ids": ["10.1.1.4"],
            "status": "pass-linux-detection-and-pre-io-rejection",
            "policy_profile": copy.deepcopy(evidence.POLICY_PROFILE),
            "verification_commands": evidence.expected_commands(),
            "integration_results": {
                "provider_and_sentinel_roots_rejected": True,
                "sentinel_added_after_inspection_rejected": True,
                "configuration_read_started": False,
                "configuration_preimage_preserved": True,
                "authority_database_created": False,
                "kernel_rejected_every_risky_or_unknown_observation": True,
            },
            "claims": {
                "linux_state_root_policy_implemented": True,
                "configuration_and_authority_pre_io_rejection_tested": True,
                "live_remote_mount_tested": False,
                "all_sync_clients_detected": False,
                "release_support": False,
            },
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

    def test_policy_or_command_mutation_fails(self) -> None:
        policy = self.valid_report()
        policy["policy_profile"]["rejected_filesystem_classes"].pop()
        self.assertIn(
            "strict-local state-root policy profile changed",
            evidence.validate_report(policy),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "strict-local state-root verification commands changed",
            evidence.validate_report(command),
        )

    def test_integration_or_claim_overstatement_fails(self) -> None:
        integration = self.valid_report()
        integration["integration_results"]["authority_database_created"] = True
        self.assertIn(
            "strict-local state-root integration results changed",
            evidence.validate_report(integration),
        )
        claim = self.valid_report()
        claim["claims"]["live_remote_mount_tested"] = True
        self.assertIn(
            "strict-local state-root evidence overclaimed",
            evidence.validate_report(claim),
        )

    def test_limitation_or_source_omission_fails(self) -> None:
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "strict-local state-root limitations changed",
            evidence.validate_report(limitation),
        )
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "strict-local state-root source evidence changed",
            evidence.validate_report(source),
        )

    def test_private_data_or_external_network_use_fails(self) -> None:
        for field in ("private_user_data_used", "external_network_used"):
            with self.subTest(field=field):
                changed = self.valid_report()
                changed[field] = True
                self.assertIn(
                    "strict-local state-root evidence used prohibited data or network",
                    evidence.validate_report(changed),
                )


if __name__ == "__main__":
    unittest.main()
