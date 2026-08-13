"""Mutation tests for Secret Service and encrypted-backup format evidence."""

import copy
import unittest

from scripts import secret_backup_format_evidence as evidence


class SecretBackupFormatEvidenceTests(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return (
            evidence.DOCUMENT_PATH.read_text(),
            evidence.STORE_PATH.read_text(),
            evidence.SECRET_PATH.read_text(),
            evidence.LIFECYCLE_PATH.read_text(),
        )

    def valid(self) -> dict:
        return {
            "artifact_id": "secret-store-encrypted-backup-format",
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-current-linux-secret-and-backup-format",
            "task_ids": ["11.1.2.3"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(*self.sources()), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_each_source_fragment_mutation_fails(self) -> None:
        sources = list(self.sources())
        fragment_groups = (
            evidence.DOCUMENT_FRAGMENTS,
            evidence.STORE_FRAGMENTS,
            evidence.SECRET_FRAGMENTS,
            evidence.LIFECYCLE_FRAGMENTS,
        )
        for source_index, fragments in enumerate(fragment_groups):
            for fragment in fragments:
                mutated = sources.copy()
                mutated[source_index] = mutated[source_index].replace(fragment, "removed", 1)
                self.assertTrue(evidence.validate_sources(*mutated))

    def test_claim_limit_and_command_mutations_fail(self) -> None:
        report = self.valid()
        report["claims"]["distinct_backup_key_type_enforced"] = True
        self.assertIn("secret-backup evidence claims changed", evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("secret-backup evidence limitations changed", evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn(
            "secret-backup evidence verification_commands changed",
            evidence.validate_report(report),
        )

    def test_revision_source_and_data_scope_mutations_fail(self) -> None:
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("secret-backup evidence revision is invalid", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("secret-backup evidence sources changed", evidence.validate_report(report))
        for key in ("private_user_data_used", "external_network_used"):
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
