from __future__ import annotations

import copy
import unittest

from scripts.story_5_2_ac2_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_upstream,
)


class Story52Ac2EvidenceTests(unittest.TestCase):
    def test_current_no_replay_and_uncertainty_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["prior_identity_family_count"], 7)

    def test_every_identity_unsafe_effect_and_reconciliation_count_is_required(self) -> None:
        for field, value in (
            ("prior_identity_family_count", 6),
            ("automatic_retry_denied_effect_class_count", 3),
            ("non_admitting_reconciliation_disposition_count", 1),
            ("concurrent_eligible_attempts", 15),
            ("execution_admissions", 2),
            ("synthetic_effect_callback_invocations", 2),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_replay_authority_uncertainty_and_dispatch_broadening_is_rejected(self) -> None:
        for field, value in (
            ("replayed_prior_identity_admitted", True),
            ("old_call_replayed", True),
            ("old_authority_object_reused", True),
            ("uncertain_recovery_admitted", True),
            ("uncertain_outcome_retained", False),
            ("uncertain_outcome_retried", True),
            ("dispatches_after_policy_mutation", 1),
            ("native_tool_effect_executed", True),
            ("model_inference_executed", True),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_platform_review_story_sprint_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "native_platform_complete",
            "independent_review_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "pass"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_both_product_tests_and_upstream_validators(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nFAILED"))


if __name__ == "__main__":
    unittest.main()
