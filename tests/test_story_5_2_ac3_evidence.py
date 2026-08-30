from __future__ import annotations

import copy
import unittest

from scripts.story_5_2_ac3_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_upstream,
)


class Story52Ac3EvidenceTests(unittest.TestCase):
    def test_current_bounded_termination_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["budget_termination_case_count"], 19)

    def test_every_budget_failure_and_repeat_bound_is_required(self) -> None:
        for field, value in (
            ("budget_dimension_count", 5),
            ("failure_class_count", 13),
            ("primary_or_error_class_limit_cases", 17),
            ("independent_total_work_limit_cases", 0),
            ("budget_termination_case_count", 18),
            ("no_progress_repeat_limit", 3),
            ("preterminal_progress_decision_count", 1),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_diagnosis_mutation_continuation_retry_and_effect_broadening_is_rejected(self) -> None:
        for field, value in (
            ("one_actionable_diagnosis_per_termination", False),
            ("rejected_budget_charge_mutations", 1),
            ("terminal_diagnosis_sticky", False),
            ("automatic_continuation_allowed", True),
            ("automatic_retry_allowed", True),
            ("additional_effect_count", 1),
            ("runtime_effect_executed", True),
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

    def test_raw_results_require_product_test_and_both_upstream_validators(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nFAILED"))


if __name__ == "__main__":
    unittest.main()
