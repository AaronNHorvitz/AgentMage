"""Mutation tests for S-011-RT01 seeded crash-recovery evidence."""

import copy
import unittest

from scripts import seeded_crash_recovery_evidence as evidence


class SeededCrashRecoveryEvidenceTests(unittest.TestCase):
    def sources(self) -> tuple[str, str, str]:
        return tuple((evidence.ROOT / path).read_text() for path in evidence.SOURCE_PATHS[:3])

    def valid(self) -> dict:
        return {
            "artifact_id": "s-011-rt01-seeded-crash-recovery-results",
            "boundaries": list(evidence.BOUNDARIES),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "positions": list(evidence.POSITIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [{"bytes": 1, "path": path, "sha256": "b" * 64} for path in evidence.SOURCE_PATHS],
            "status": "pass-current-encrypted-operational-store-boundary",
            "task_ids": ["11.1.3.3", "S-011-RT01"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(*self.sources()), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_boundary_and_source_mutations_fail(self) -> None:
        sources = list(self.sources())
        mutated = sources.copy()
        mutated[0] = mutated[0].replace("`RT-04`", "`RT-99`", 1)
        self.assertIn("S-011-RT01 boundary closure changed", evidence.validate_sources(*mutated))
        for index, fragments in enumerate((evidence.DOCUMENT_FRAGMENTS, evidence.AUTHORITY_FRAGMENTS, evidence.STORE_FRAGMENTS)):
            for fragment in fragments:
                mutated = sources.copy()
                mutated[index] = mutated[index].replace(fragment, "removed", 1)
                self.assertTrue(evidence.validate_sources(*mutated))

    def test_count_limit_and_command_mutations_fail(self) -> None:
        for key in ("seed_count", "boundary_family_count", "runs_per_boundary_position_pair"):
            report = self.valid()
            report["claims"][key] -= 1
            self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertTrue(evidence.validate_report(report))

    def test_revision_source_and_data_scope_mutations_fail(self) -> None:
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertTrue(evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertTrue(evidence.validate_report(report))
        for key in ("private_user_data_used", "external_network_used"):
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
