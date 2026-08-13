"""Tests for encrypted operational-schema evidence."""

from __future__ import annotations

import copy
import unittest

from scripts import operational_schema_evidence as evidence


class OperationalSchemaEvidenceTests(unittest.TestCase):
    def valid_report(self) -> dict:
        return {
            "schema_version": 1,
            "artifact_id": "encrypted-operational-schema-v2",
            "source_revision": "a" * 40,
            "task_ids": ["11.1.1.1"],
            "status": "pass-normalized-schema-and-migration-boundary",
            "migration_sha256": "b" * 64,
            "schema_profile": copy.deepcopy(evidence.SCHEMA_PROFILE),
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

    def test_current_migration_and_exact_report_are_valid(self) -> None:
        migration = (evidence.ROOT / evidence.MIGRATION_PATH).read_text(encoding="utf-8")
        self.assertEqual(evidence.validate_migration(migration), [])
        self.assertEqual(evidence.validate_report(self.valid_report()), [])

    def test_table_index_or_relationship_mutation_fails(self) -> None:
        migration = (evidence.ROOT / evidence.MIGRATION_PATH).read_text(encoding="utf-8")
        for changed in (
            migration.replace("CREATE TABLE sessions", "CREATE TABLE runs", 1),
            migration.replace("CREATE INDEX tasks_plan_idx", "CREATE INDEX changed_idx", 1),
            migration.replace("CHECK(legal_hold IN (0, 1))", "CHECK(legal_hold >= 0)", 1),
        ):
            self.assertTrue(evidence.validate_migration(changed))

    def test_profile_command_or_claim_mutation_fails(self) -> None:
        profile = self.valid_report()
        profile["schema_profile"]["total_table_count"] = 18
        self.assertIn(
            "operational schema schema_profile changed",
            evidence.validate_report(profile),
        )
        command = self.valid_report()
        command["verification_commands"].pop()
        self.assertIn(
            "operational schema verification_commands changed",
            evidence.validate_report(command),
        )
        claim = self.valid_report()
        claim["claims"]["public_domain_write_api_implemented"] = True
        self.assertIn(
            "operational schema claims changed",
            evidence.validate_report(claim),
        )

    def test_source_migration_identity_or_revision_mutation_fails(self) -> None:
        source = self.valid_report()
        source["sources"].pop()
        self.assertIn(
            "operational schema source evidence changed",
            evidence.validate_report(source),
        )
        identity = self.valid_report()
        identity["migration_sha256"] = "c" * 64
        self.assertIn(
            "operational schema migration identity changed",
            evidence.validate_report(identity),
        )
        revision = self.valid_report()
        revision["source_revision"] = "HEAD"
        self.assertIn(
            "operational schema source revision is invalid",
            evidence.validate_report(revision),
        )

    def test_private_data_or_network_use_mutation_fails(self) -> None:
        private = self.valid_report()
        private["private_user_data_used"] = True
        self.assertIn(
            "operational schema private_user_data_used changed",
            evidence.validate_report(private),
        )
        network = self.valid_report()
        network["external_network_used"] = True
        self.assertIn(
            "operational schema external_network_used changed",
            evidence.validate_report(network),
        )


if __name__ == "__main__":
    unittest.main()
