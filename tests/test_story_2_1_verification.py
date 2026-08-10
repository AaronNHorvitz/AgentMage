from __future__ import annotations

import copy
import unittest

from scripts.story_2_1_verification import (
    CORPUS_REPORT_PATH,
    EXPECTED_SOURCE_SEEDS,
    build_corpus_report,
    check_corpus_report,
    read_json,
    validate_corpus_report,
)


class Story21VerificationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_corpus_report()

    def test_checked_report_is_current(self) -> None:
        self.assertEqual(check_corpus_report(), [])
        self.assertEqual(read_json(CORPUS_REPORT_PATH), self.report)
        self.assertEqual(validate_corpus_report(self.report), [])

    def test_every_source_family_and_corpus_have_pinned_seeds(self) -> None:
        observed = {
            item["family"]: item["seed"] for item in self.report["source_seeds"]
        }
        self.assertEqual(observed, EXPECTED_SOURCE_SEEDS)
        self.assertEqual(
            self.report["corpus"]["seed"],
            "agentmage-versioned-corpus-synthetic-v1",
        )

    def test_two_clean_regenerations_match_checked_artifacts(self) -> None:
        reproduction = self.report["reproduction"]
        self.assertEqual(reproduction["run_count"], 2)
        self.assertTrue(reproduction["runs_byte_identical"])
        self.assertTrue(reproduction["checked_artifacts_match"])
        first, second = reproduction["runs"]
        self.assertEqual(first["archive_sha256"], second["archive_sha256"])
        self.assertEqual(
            first["manifest_file_sha256"], second["manifest_file_sha256"]
        )
        self.assertEqual(
            first["manifest_self_sha256"], second["manifest_self_sha256"]
        )

    def test_metadata_provenance_and_goldens_are_closed(self) -> None:
        metadata = self.report["metadata"]
        self.assertEqual(sum(metadata["family_counts"].values()), 77)
        self.assertEqual(metadata["source_family_count"], 5)
        self.assertEqual(metadata["provenance_record_count"], 5)
        self.assertEqual(metadata["golden_manifest_count"], 4)
        self.assertEqual(
            set(metadata["golden_evidence_states"]),
            {"Observed", "Derived", "Inferred", "Unknown/Blocked"},
        )

    def test_archive_and_manifest_corruption_are_detected_before_use(self) -> None:
        corruption = self.report["corruption"]
        self.assertTrue(corruption["archive_mutation_detected_before_use"])
        self.assertTrue(corruption["manifest_mutation_detected_before_use"])
        self.assertGreater(corruption["archive_failure_count"], 0)
        self.assertGreater(corruption["manifest_failure_count"], 0)
        self.assertFalse(corruption["corrupt_artifact_persisted"])

    def test_report_rejects_tamper_private_data_and_macos_claims(self) -> None:
        tampered = copy.deepcopy(self.report)
        tampered["reproduction"]["runs_byte_identical"] = False
        private = copy.deepcopy(self.report)
        private["reproduction"]["private_user_data_used"] = True
        unblocked = copy.deepcopy(self.report)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_corpus_report(tampered))
        self.assertTrue(validate_corpus_report(private))
        self.assertTrue(validate_corpus_report(unblocked))

    def test_report_is_deterministic_and_has_no_product_claim(self) -> None:
        self.assertEqual(build_corpus_report(), build_corpus_report())
        self.assertEqual(self.report["product_support_claim"], "none")
        self.assertEqual(self.report["macos_execution_status"], "blocked-macos")


if __name__ == "__main__":
    unittest.main()
