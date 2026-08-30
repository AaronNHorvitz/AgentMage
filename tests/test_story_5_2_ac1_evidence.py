from __future__ import annotations

import copy
import unittest

from scripts.story_5_2_ac1_evidence import MARKERS, TRUTH, expected_report, validate_raw, validate_report, validate_upstream


class Story52Ac1EvidenceTests(unittest.TestCase):
    def test_current_matrix_and_upstream_evidence_are_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["operation_failure_pair_count"], 308)

    def test_every_cardinality_and_determinism_claim_is_required(self) -> None:
        for field, value in (
            ("registered_operation_count", 21),
            ("effect_class_count", 6),
            ("failure_class_count", 13),
            ("operation_failure_pair_count", 307),
            ("closed_failure_disposition_count", 8),
            ("every_operation_failure_pair_deterministic", False),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_model_override_dispatch_and_effect_overclaims_are_rejected(self) -> None:
        for field, value in (
            ("model_created_classification_admitted", True),
            ("caller_effect_override_admitted", True),
            ("dispatches_after_policy_mutation", 1),
            ("tool_effect_executed", True),
            ("model_inference_executed", True),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_platform_review_story_sprint_and_release_overclaims_are_rejected(self) -> None:
        for field in ("native_platform_complete", "independent_review_complete", "story_completion_claim", "sprint_completion_claim"):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "pass"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_product_matrix_and_all_upstream_validators(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nFAILED"))


if __name__ == "__main__":
    unittest.main()
