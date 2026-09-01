from __future__ import annotations

import copy
import unittest

from scripts.story_50_3_foundational_runtime_evidence import expected_report, validate_report


class Story503FoundationalRuntimeEvidenceTests(unittest.TestCase):
    def test_current_local_core_truth_is_exact(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertEqual(report["status"], "PASS_LOCAL_TEXT_LOG_WORKFLOW_CORE")
        self.assertFalse(report["product_truth"]["complete_foundational_runtime_claim"])
        self.assertTrue(report["product_truth"]["independent_feature_activation_complete"])
        self.assertTrue(report["product_truth"]["local_pressure_performance_campaign_complete"])
        self.assertTrue(report["pressure_metrics"]["runtime_cleanup_verified"])

    def test_external_later_or_release_overclaim_fails_closed(self) -> None:
        fields = (
            "structured_document_parser_qualification",
            "qualified_production_model_evaluation",
            "installed_client_campaign_complete",
            "independent_review_complete",
            "windows_validation_complete",
            "macos_validation_complete",
            "complete_foundational_runtime_claim",
        )
        for field in fields:
            with self.subTest(field=field):
                changed = copy.deepcopy(expected_report())
                changed["product_truth"][field] = True
                self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "v0.4"
        self.assertTrue(validate_report(changed))

    def test_local_authority_or_evidence_regression_fails_closed(self) -> None:
        for field in (
            "one_verified_workflow_supervisor",
            "one_common_runtime_coordinator_per_attempt",
            "verifier_owned_completion",
            "fresh_attempt_run_tool_grant_and_receipt_identities",
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(expected_report())
                changed["product_truth"][field] = False
                self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["false_completion_or_unsafe_retry"] = True
        self.assertTrue(validate_report(changed))

    def test_pressure_threshold_regressions_fail_closed(self) -> None:
        for measured, threshold in (
            ("source_extraction_latency_ms", "source_extraction_latency_ceiling_ms"),
            ("runtime_peak_memory_kib", "runtime_peak_memory_ceiling_kib"),
            ("runtime_store_growth_bytes", "runtime_store_growth_ceiling_bytes"),
            ("cancellation_latency_us", "cancellation_latency_ceiling_us"),
            ("terminal_diagnosis_latency_us", "terminal_diagnosis_latency_ceiling_us"),
        ):
            with self.subTest(measured=measured):
                changed = copy.deepcopy(expected_report())
                changed["pressure_metrics"][measured] = (
                    changed["pressure_metrics"][threshold] + 1
                )
                self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
