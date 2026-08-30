from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_terminal_fixtures as fixtures


class ArtifactEvaluationTerminalFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.suite = json.loads(fixtures.OUTPUT_PATH.read_text(encoding="utf-8"))
        cls.by_category = {item["category"]: item for item in cls.suite["fixtures"]}

    def test_every_required_premature_terminal_category_is_exact(self) -> None:
        self.assertEqual(tuple(self.by_category), fixtures.CATEGORIES)
        self.assertEqual(self.suite["fixture_count"], 9)

    def test_false_and_open_plan_completion_never_verify(self) -> None:
        for category in ("false_completion", "open_plan_completion"):
            item = self.by_category[category]
            self.assertTrue(item["model_completion_claimed"])
            self.assertEqual(item["expected"]["terminal_result"], "blocked")
            self.assertFalse(item["expected"]["verified_completion"])

    def test_repeat_and_budget_limits_terminate_exactly(self) -> None:
        repeated = self.by_category["repeated_state_loop"]
        exhausted = self.by_category["budget_exhaustion"]
        self.assertEqual(repeated["repeated_state_count"], repeated["limits"]["repeated_state_count"])
        self.assertEqual(repeated["expected"]["terminal_result"], "stalled")
        self.assertEqual(exhausted["model_turns_consumed"], exhausted["limits"]["model_turns"])
        self.assertEqual(exhausted["expected"]["terminal_result"], "exhausted")

    def test_every_non_cancelled_failure_has_one_actionable_diagnostic(self) -> None:
        for category, item in self.by_category.items():
            diagnostic = item["terminal_diagnostic"]
            if category == "cancellation":
                self.assertIsNone(diagnostic)
                self.assertEqual(item["expected"]["diagnostic_count"], 0)
            else:
                self.assertIsInstance(diagnostic, dict)
                self.assertEqual(item["expected"]["diagnostic_count"], 1)
                self.assertTrue(diagnostic["safe_resume_action"])

    def test_no_case_needs_continue_retries_replays_or_false_completes(self) -> None:
        for item in self.suite["fixtures"]:
            expected = item["expected"]
            self.assertTrue(expected["bounded_termination"])
            self.assertFalse(expected["artificial_continue_required"])
            self.assertFalse(expected["automatic_retry_allowed"])
            self.assertFalse(expected["effect_replay_allowed"])
            self.assertFalse(expected["false_completion_allowed"])

    def test_provider_disconnect_and_cancellation_remain_distinct(self) -> None:
        disconnected = self.by_category["provider_disconnect"]
        cancelled = self.by_category["cancellation"]
        self.assertEqual(disconnected["expected"]["terminal_result"], "failed")
        self.assertEqual(cancelled["expected"]["terminal_result"], "cancelled")
        self.assertFalse(disconnected["real_provider_contacted"])
        self.assertFalse(cancelled["real_provider_contacted"])

    def test_mutation_completion_and_execution_overclaims_fail(self) -> None:
        changed = copy.deepcopy(self.suite)
        changed["fixtures"].pop()
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["fixtures"][0]["expected"]["verified_completion"] = True
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["real_provider_contacted"] = True
        self.assertTrue(fixtures.validate_suite(changed))

    def test_checked_suite_matches_the_reproducible_builder(self) -> None:
        for item in self.suite["fixtures"]:
            self.assertTrue(fixtures.valid_hash(item, "fixture_sha256"))
        self.assertTrue(fixtures.valid_hash(self.suite, "suite_sha256"))
        self.assertEqual(fixtures.check(), [])


if __name__ == "__main__":
    unittest.main()
