from __future__ import annotations

import copy
import unittest

from scripts.story_11_2_ac2_evidence import (
    IDENTITIES,
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_sources,
    validate_upstream_reports,
)


class Story112Ac2EvidenceTests(unittest.TestCase):
    def test_current_sources_and_upstream_reports_satisfy_acceptance(self) -> None:
        self.assertEqual(validate_sources(), [])
        self.assertEqual(validate_upstream_reports(), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_identity_dimension_set_is_exact(self) -> None:
        self.assertEqual(
            list(IDENTITIES),
            ["source", "parser", "policy", "model", "plan", "tool", "environment"],
        )
        self.assertEqual(TRUTH["identity_change_count"], 7)

    def test_every_identity_block_and_stale_invariant_is_required(self) -> None:
        for field in (
            "source_change_blocks_resume",
            "parser_change_blocks_resume",
            "policy_change_blocks_resume",
            "model_change_blocks_resume",
            "plan_change_blocks_resume",
            "tool_change_blocks_resume",
            "environment_change_blocks_resume",
            "stale_source_derivatives_excluded",
            "stale_attempt_replay_refused",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = False
            self.assertTrue(validate_report(changed), field)

    def test_later_integration_platform_review_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "all_later_active_parser_integrations_complete",
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

    def test_raw_results_require_every_validator_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nTraceback"))


if __name__ == "__main__":
    unittest.main()
