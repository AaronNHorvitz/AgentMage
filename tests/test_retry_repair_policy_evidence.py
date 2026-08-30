from __future__ import annotations

import copy
import unittest

from scripts.retry_repair_policy_evidence import MARKERS, TRUTH, expected_report, validate_raw, validate_report


class RetryRepairPolicyEvidenceTests(unittest.TestCase):
    def test_current_report_is_exact_and_closed(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertEqual(report["model_repair_contract"]["maximum_repairs"], 1)
        self.assertFalse(report["normalization_contract"]["value_invention_permitted"])

    def test_normalization_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["normalization_contract"]["value_invention_permitted"] = True
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["normalization_contract"]["maximum_assembled_bytes"] += 1
        self.assertIn("widened", validate_report(changed)[0])

    def test_repair_limit_or_binding_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["model_repair_contract"]["maximum_repairs"] = 2
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["model_repair_contract"]["exact_model_profile_bound"] = False
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["model_repair_contract"]["prior_effect_attempts_permitted"] = 1
        self.assertIn("widened", validate_report(changed)[0])

    def test_fresh_attempt_prerequisite_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["current_complete_preflight_required"] = False
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["remaining_attempt_budget_required"] = False
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["successor_grant_use_limit"] = 2
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["fresh_per_attempt_approval_required_when_configured"] = False
        self.assertIn("widened", validate_report(changed)[0])

    def test_unsafe_effect_or_replay_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["automatic_retry_denied_effect_classes"].remove("unknown")
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["complete_prior_use_ledger"].remove("receipt")
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["any_successor_receipt_before_execution_admitted"] = True
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["fresh_attempt_contract"]["required_idempotency_key_must_be_fresh"] = False
        self.assertIn("widened", validate_report(changed)[0])

    def test_product_truth_cannot_be_promoted(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["model_inference_executed"] = True
        self.assertIn("widened", validate_report(changed)[0])
        self.assertEqual(TRUTH["release_claim"], "none")

    def test_raw_results_require_every_marker_and_reject_failure_output(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
