"""Mutation tests for current operational-schema v3 evidence."""

import copy
import unittest

from scripts import operational_schema_v3_evidence as evidence


class OperationalSchemaV3EvidenceTests(unittest.TestCase):
    def valid(self) -> dict:
        return {
            "artifact_id": "encrypted-operational-schema-v3",
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "migration_sha256": {path.split("/")[-1]: "b" * 64 for path in evidence.MIGRATION_PATHS},
            "private_user_data_used": False,
            "schema_profile": copy.deepcopy(evidence.SCHEMA_PROFILE),
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-versioned-schema-and-migration-chain",
            "task_ids": ["11.1.2.1"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_migrations_source_and_report_are_valid(self) -> None:
        migrations = [(evidence.ROOT / path).read_text() for path in evidence.MIGRATION_PATHS]
        source = (evidence.ROOT / "kernel/engine/src/operational_store.rs").read_text()
        self.assertEqual(evidence.validate_migrations(*migrations), [])
        self.assertEqual(evidence.validate_source(source), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_migration_and_execution_mutations_fail(self) -> None:
        v2, v3 = [(evidence.ROOT / path).read_text() for path in evidence.MIGRATION_PATHS]
        self.assertTrue(evidence.validate_migrations(v2.replace("CREATE TABLE sessions", "CREATE TABLE runs", 1), v3))
        self.assertTrue(evidence.validate_migrations(v2, v3.replace("CREATE TABLE retention_events", "CREATE TABLE changed", 1)))
        source = (evidence.ROOT / "kernel/engine/src/operational_store.rs").read_text()
        for fragment in evidence.SOURCE_FRAGMENTS:
            self.assertTrue(evidence.validate_source(source.replace(fragment, "removed", 1)))

    def test_profile_claim_and_command_mutations_fail(self) -> None:
        report = self.valid()
        report["schema_profile"]["total_table_count"] = 19
        self.assertIn("schema v3 evidence schema_profile changed", evidence.validate_report(report))
        report = self.valid()
        report["claims"]["historical_v2_artifact_rewritten"] = True
        self.assertIn("schema v3 evidence claims changed", evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn("schema v3 evidence verification_commands changed", evidence.validate_report(report))

    def test_revision_migration_identity_and_source_mutations_fail(self) -> None:
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("schema v3 evidence revision is invalid", evidence.validate_report(report))
        report = self.valid()
        report["migration_sha256"].pop(next(iter(report["migration_sha256"])))
        self.assertIn("schema v3 migration identities changed", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("schema v3 sources changed", evidence.validate_report(report))

    def test_private_data_network_limit_and_status_mutations_fail(self) -> None:
        for key in ["private_user_data_used", "external_network_used"]:
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("schema v3 evidence limitations changed", evidence.validate_report(report))
        report = self.valid()
        report["status"] = "pass-release"
        self.assertIn("schema v3 evidence status changed", evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
