"""Mutation tests for bounded Sprint 11 crash/canary result evidence."""

import copy
import unittest

from scripts import crash_canary_result_evidence as evidence


class CrashCanaryResultEvidenceTests(unittest.TestCase):
    def sources(self) -> tuple[str, str, str, str]:
        return (
            evidence.DOCUMENT_PATH.read_text(),
            evidence.AUTHORITY_PATH.read_text(),
            evidence.STORE_PATH.read_text(),
            evidence.PERSISTENCE_PATH.read_text(),
        )

    def valid(self) -> dict:
        return {
            "artifact_id": "sprint-11-bounded-crash-canary-results",
            "canary_surfaces": list(evidence.CANARY_SURFACES),
            "claims": copy.deepcopy(evidence.CLAIMS),
            "crash_point_ids": list(evidence.CRASH_POINT_IDS),
            "external_network_used": False,
            "limitations": list(evidence.LIMITATIONS),
            "private_user_data_used": False,
            "schema_version": 1,
            "source_revision": "a" * 40,
            "sources": [
                {"bytes": 1, "path": path, "sha256": "b" * 64}
                for path in evidence.SOURCE_PATHS
            ],
            "status": "pass-current-partial-results-open-verification-gates",
            "task_ids": ["11.1.2.4"],
            "verification_commands": evidence.expected_commands(),
        }

    def test_current_sources_and_report_are_closed(self) -> None:
        self.assertEqual(evidence.validate_sources(*self.sources()), [])
        self.assertEqual(evidence.validate_report(self.valid()), [])

    def test_matrix_and_each_source_fragment_mutation_fail(self) -> None:
        sources = list(self.sources())
        mutated = sources.copy()
        mutated[0] = mutated[0].replace("`CP-03`", "`CP-99`", 1)
        self.assertIn("crash-point matrix closure changed", evidence.validate_sources(*mutated))
        for source_index, fragments in enumerate(
            (
                evidence.DOCUMENT_FRAGMENTS,
                evidence.AUTHORITY_FRAGMENTS,
                evidence.STORE_FRAGMENTS,
                evidence.PERSISTENCE_FRAGMENTS,
            )
        ):
            for fragment in fragments:
                mutated = sources.copy()
                mutated[source_index] = mutated[source_index].replace(fragment, "removed", 1)
                self.assertTrue(evidence.validate_sources(*mutated))

    def test_completion_claim_limit_and_command_mutations_fail(self) -> None:
        for claim in ("s011_st01_complete", "s011_rt01_complete", "manual_fuzzing_executed"):
            report = self.valid()
            report["claims"][claim] = True
            self.assertIn("crash-canary evidence claims changed", evidence.validate_report(report))
        report = self.valid()
        report["limitations"].pop()
        self.assertIn("crash-canary evidence limitations changed", evidence.validate_report(report))
        report = self.valid()
        report["verification_commands"].pop()
        self.assertIn(
            "crash-canary evidence verification_commands changed",
            evidence.validate_report(report),
        )

    def test_revision_source_and_data_scope_mutations_fail(self) -> None:
        report = self.valid()
        report["source_revision"] = "HEAD"
        self.assertIn("crash-canary evidence revision is invalid", evidence.validate_report(report))
        report = self.valid()
        report["sources"].pop()
        self.assertIn("crash-canary evidence sources changed", evidence.validate_report(report))
        for key in ("private_user_data_used", "external_network_used"):
            report = self.valid()
            report[key] = True
            self.assertTrue(evidence.validate_report(report))


if __name__ == "__main__":
    unittest.main()
