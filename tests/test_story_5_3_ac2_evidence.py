from __future__ import annotations

import copy
import unittest

from scripts.story_5_3_ac2_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_upstream,
)


class Story53Ac2EvidenceTests(unittest.TestCase):
    def test_current_verifier_and_terminal_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["evaluated_surface_count"], 8)
        self.assertEqual(TRUTH["non_success_outcome_count"], 7)

    def test_required_verifier_truth_cannot_be_weakened(self) -> None:
        for field in (
            "all_required_deterministic_verifiers_must_pass",
            "expected_output_and_current_state_exact",
            "observations_receipts_and_evidence_current_complete_ordered",
            "required_postconditions_pass",
            "preserved_invariants_pass",
            "prohibited_effects_absent",
            "verified_success_requires_opaque_current_evidence_proof",
            "verified_no_op_is_distinct",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = False
            self.assertTrue(validate_report(changed), field)

    def test_false_success_and_effect_overclaims_are_rejected(self) -> None:
        for field, value in (
            ("exit_zero_has_completion_authority", True),
            ("persuasive_model_or_tool_text_has_completion_authority", True),
            ("stale_or_contradictory_evidence_establishes_completion", True),
            ("native_tool_effect_executed", True),
            ("model_inference_executed", True),
            ("network_calls", 1),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_later_gate_overclaims_are_rejected(self) -> None:
        for field in (
            "installed_product_complete", "cross_platform_complete", "independent_review_complete",
            "story_completion_claim", "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "pass"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_all_verifier_and_terminal_boundaries(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
