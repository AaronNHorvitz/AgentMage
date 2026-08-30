from __future__ import annotations

import copy
import unittest

from scripts.story_5_3_ac3_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_upstream,
)


class Story53Ac3EvidenceTests(unittest.TestCase):
    def test_current_retry_identity_and_uncertainty_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["fresh_identity_family_count"], 7)
        self.assertEqual(TRUTH["automatic_retry_forbidden_effect_class_count"], 4)

    def test_replay_reconciliation_and_approval_claims_are_required(self) -> None:
        for field, value in (
            ("replayed_prior_identity_admitted", True),
            ("old_call_or_authority_replayed", True),
            ("uncertain_effect_automatic_retry", True),
            ("uncertain_effect_requires_safe_reconciliation", False),
            ("uncertain_effect_requires_separate_user_approval", False),
            ("uncertain_outcome_retained", False),
            ("uncertain_outcome_converted_to_success", True),
            ("uncertain_outcome_retried", True),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_race_effect_and_network_overclaims_are_rejected(self) -> None:
        for field, value in (
            ("execution_admissions", 2),
            ("synthetic_effect_callback_invocations", 2),
            ("native_tool_effect_executed", True),
            ("model_inference_executed", True),
            ("network_calls", 1),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_later_gate_overclaims_are_rejected(self) -> None:
        for field in (
            "cross_process_crash_durability_complete", "installed_product_complete",
            "cross_platform_complete", "independent_review_complete", "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "pass"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_all_retry_and_terminal_boundaries(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
