from __future__ import annotations

import copy
import unittest

from scripts.story_5_3_ac1_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_upstream,
)


class Story53Ac1EvidenceTests(unittest.TestCase):
    def test_current_transition_authority_and_budget_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["workflow_lifecycle_state_count"], 18)
        self.assertEqual(TRUTH["independent_budget_dimension_count"], 6)

    def test_closed_transition_and_exact_boundary_claims_are_required(self) -> None:
        for field in (
            "closed_transition_table",
            "workflow_identity_and_sequence_immutable",
            "graph_and_step_policy_integrity_bound",
            "preflight_required",
            "effect_and_retry_policy_exact",
            "approval_policy_exact",
            "single_use_grant_required",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = False
            self.assertTrue(validate_report(changed), field)

    def test_dispatch_effect_and_network_overclaims_are_rejected(self) -> None:
        for field, value in (
            ("stale_preflight_dispatches", 1),
            ("approval_bypass_dispatches", 1),
            ("grant_reuse_dispatches", 1),
            ("native_tool_effect_executed", True),
            ("model_inference_executed", True),
            ("network_calls", 1),
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = value
            self.assertTrue(validate_report(changed), field)

    def test_later_gate_overclaims_are_rejected(self) -> None:
        for field in (
            "durable_cross_process_execution_complete",
            "installed_product_complete",
            "cross_platform_complete",
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

    def test_raw_results_require_each_runtime_boundary(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
