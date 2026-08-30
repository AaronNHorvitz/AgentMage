from __future__ import annotations

import copy
import unittest

from scripts.story_11_2_ac1_evidence import (
    BOUNDARIES,
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_sources,
    validate_upstream_reports,
)


class Story112Ac1EvidenceTests(unittest.TestCase):
    def test_current_sources_and_upstream_reports_satisfy_acceptance(self) -> None:
        self.assertEqual(validate_sources(), [])
        self.assertEqual(validate_upstream_reports(), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_boundary_matrix_is_exact_and_complete(self) -> None:
        self.assertEqual(len(BOUNDARIES), 16)
        self.assertEqual(TRUTH["forced_stop_case_count"], 224)
        self.assertEqual(TRUTH["boundary_position_cell_count"], 32)
        self.assertEqual(TRUTH["seeds_per_boundary_position"], 7)

    def test_every_acceptance_invariant_is_required(self) -> None:
        for field in (
            "exact_pre_or_post_state",
            "orphan_state_refused",
            "duplicate_state_refused",
            "authority_bearing_partial_state_refused",
            "stale_current_state_refused",
            "replay_driver_launch_refused",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = False
            self.assertTrue(validate_report(changed), field)

    def test_platform_review_story_sprint_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "physical_power_loss_complete",
            "cross_platform_complete",
            "independent_review_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "ready"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_both_upstream_validators_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nTraceback"))


if __name__ == "__main__":
    unittest.main()
