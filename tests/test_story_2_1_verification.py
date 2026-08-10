from __future__ import annotations

import copy
import unittest

from scripts.story_2_1_verification import (
    ADAPTER_REPORT_PATH,
    SUMMARY_REPORT_PATH,
    CORPUS_REPORT_PATH,
    EXPECTED_SOURCE_SEEDS,
    build_corpus_report,
    build_adapter_report,
    build_summary_report,
    check_adapter_report,
    check_summary_report,
    independent_reconciliation,
    independent_summary,
    check_corpus_report,
    read_json,
    validate_corpus_report,
    validate_adapter_report,
    validate_summary_report,
)
from scripts import test_result_bundle as result_bundles


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

    def test_adapter_matrix_report_is_current_and_complete(self) -> None:
        report = build_adapter_report()
        self.assertEqual(check_adapter_report(), [])
        self.assertEqual(read_json(ADAPTER_REPORT_PATH), report)
        self.assertEqual(validate_adapter_report(report), [])
        self.assertEqual(report["summary"]["adapter_count"], 7)
        self.assertEqual(report["summary"]["mode_count"], 8)
        self.assertEqual(report["summary"]["matrix_case_count"], 56)
        self.assertEqual(report["summary"]["post_close_rejection_count"], 56)
        self.assertTrue(report["summary"]["all_typed_outcomes_matched"])
        self.assertTrue(report["summary"]["all_cleanup_passed"])

    def test_each_adapter_covers_every_typed_mode_once(self) -> None:
        report = build_adapter_report()
        expected_statuses = {
            "succeeded",
            "denied",
            "malformed",
            "cancelled",
            "timed-out",
            "crashed",
            "uncertain",
        }
        for adapter in report["adapters"]:
            with self.subTest(adapter=adapter["adapter_id"]):
                self.assertEqual(adapter["mode_count"], 8)
                self.assertEqual(adapter["event_count"], 8)
                self.assertEqual(set(adapter["status_counts"]), expected_statuses)
                self.assertEqual(adapter["status_counts"]["crashed"], 2)
                self.assertTrue(adapter["cleanup_passed"])

    def test_adapter_report_rejects_omissions_side_effects_and_macos_claims(self) -> None:
        report = build_adapter_report()
        omitted = copy.deepcopy(report)
        omitted["summary"]["matrix_case_count"] = 55
        side_effect = copy.deepcopy(report)
        side_effect["side_effect_contract"]["network"] = True
        unblocked = copy.deepcopy(report)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_adapter_report(omitted))
        self.assertTrue(validate_adapter_report(side_effect))
        self.assertTrue(validate_adapter_report(unblocked))

    def test_summary_reconciliation_report_is_current_and_exact(self) -> None:
        report = build_summary_report()
        self.assertEqual(check_summary_report(), [])
        self.assertEqual(read_json(SUMMARY_REPORT_PATH), report)
        self.assertEqual(validate_summary_report(report), [])
        reconciliation = report["reconciliation"]
        self.assertTrue(reconciliation["exact_match"])
        self.assertEqual(reconciliation["test_count"], 9)
        self.assertEqual(reconciliation["non_pass_test_count"], 5)
        self.assertEqual(reconciliation["overall_status"], "fail")

    def test_independent_summary_matches_without_calling_producer_reducer(self) -> None:
        profile = result_bundles.read_json(result_bundles.PROFILE_PATH)
        bundle = result_bundles.build_bundle(profile)
        results = result_bundles.flattened_results(bundle)
        self.assertEqual(independent_summary(results), bundle["summary"])
        self.assertEqual(independent_reconciliation(bundle)["summary"], bundle["summary"])

    def test_independent_reconciliation_rejects_omission_duplication_and_pass_conversion(self) -> None:
        profile = result_bundles.read_json(result_bundles.PROFILE_PATH)
        bundle = result_bundles.build_bundle(profile)

        omitted = copy.deepcopy(bundle)
        omitted["shards"][0]["results"].pop()
        omitted["shards"][0]["result_count"] -= 1
        with self.assertRaises(ValueError):
            independent_reconciliation(omitted)

        duplicated = copy.deepcopy(bundle)
        duplicated["shards"][0]["results"].append(
            copy.deepcopy(duplicated["shards"][0]["results"][0])
        )
        duplicated["shards"][0]["result_count"] += 1
        with self.assertRaises(ValueError):
            independent_reconciliation(duplicated)

        converted = copy.deepcopy(bundle)
        target = next(
            result
            for result in result_bundles.flattened_results(converted)
            if result["final_status"] != "pass"
        )
        target["classification"] = "pass"
        with self.assertRaises(ValueError):
            independent_reconciliation(converted)

    def test_summary_report_rejects_mismatch_raw_retention_and_macos_claims(self) -> None:
        report = build_summary_report()
        mismatch = copy.deepcopy(report)
        mismatch["reconciliation"]["exact_match"] = False
        retained = copy.deepcopy(report)
        retained["raw_results_persisted_by_verifier"] = True
        unblocked = copy.deepcopy(report)
        unblocked["macos_execution_status"] = "pass"
        self.assertTrue(validate_summary_report(mismatch))
        self.assertTrue(validate_summary_report(retained))
        self.assertTrue(validate_summary_report(unblocked))


if __name__ == "__main__":
    unittest.main()
