"""Tests for encrypted operational-store invariant evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import operational_store_invariants_evidence as evidence


class OperationalStoreInvariantEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "encrypted-operational-store-invariants",
            "source_revision": "a" * 40,
            "task_ids": ["11.1.1.2"],
            "status": "pass-writer-transaction-wal-checkpoint-migration-invariants",
            "store_source_sha256": "b" * 64,
            "invariant_profile": copy.deepcopy(evidence.INVARIANT_PROFILE),
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

    def test_current_source_and_exact_report_are_valid(self) -> None:
        source = (
            evidence.ROOT / "kernel/engine/src/operational_store.rs"
        ).read_text(encoding="utf-8")
        self.assertEqual(evidence.validate_source(source), [])
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_required_source_fragment_mutations_fail(self) -> None:
        source = (
            evidence.ROOT / "kernel/engine/src/operational_store.rs"
        ).read_text(encoding="utf-8")
        for fragment in evidence.REQUIRED_SOURCE_FRAGMENTS:
            changed = source.replace(fragment, "removed-invariant", 1)
            self.assertTrue(evidence.validate_source(changed))

    def test_profile_command_or_claim_mutation_fails(self) -> None:
        profile = self.valid_report()
        profile["invariant_profile"]["required_pragmas"]["foreign_keys"] = 0
        self.assertIn(
            "store invariant invariant_profile changed",
            evidence.validate_report(profile),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "store invariant verification_commands changed",
            evidence.validate_report(command),
        )
        claim = self.valid_report()
        claim["claims"]["cross_process_stress_campaign_complete"] = True
        self.assertIn(
            "store invariant claims changed",
            evidence.validate_report(claim),
        )

    def test_source_identity_revision_or_limit_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "store invariant source evidence changed",
            evidence.validate_report(source),
        )
        identity = self.valid_report()
        identity["store_source_sha256"] = "c" * 64
        self.assertIn(
            "store invariant source identity changed",
            evidence.validate_report(identity),
        )
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "store invariant source revision is invalid",
            evidence.validate_report(revision),
        )
        limitation = self.valid_report()
        limitation["limitations"].pop()
        self.assertIn(
            "store invariant limitations changed",
            evidence.validate_report(limitation),
        )

    def test_private_data_or_network_use_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "store invariant private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "store invariant external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
