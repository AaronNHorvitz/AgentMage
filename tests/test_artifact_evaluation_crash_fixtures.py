from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_crash_fixtures as fixtures


class ArtifactEvaluationCrashFixtureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.suite = json.loads(fixtures.OUTPUT_PATH.read_text(encoding="utf-8"))
        cls.by_pair = {(item["boundary"], item["position"]): item for item in cls.suite["fixtures"]}

    def test_every_boundary_has_exact_before_and_after_cases(self) -> None:
        self.assertEqual(list(self.by_pair), [(boundary, position) for boundary in fixtures.BOUNDARIES for position in fixtures.POSITIONS])
        self.assertEqual(self.suite["fixture_count"], 18)

    def test_durable_record_prefix_grows_exactly_across_boundaries(self) -> None:
        for boundary in fixtures.BOUNDARIES:
            before = self.by_pair[(boundary, "before")]["durable_records"]
            after = self.by_pair[(boundary, "after")]["durable_records"]
            self.assertEqual(len(after), len(before) + 1)
            self.assertEqual(after[:-1], before)

    def test_dispatch_without_observation_is_uncertain(self) -> None:
        self.assertEqual(self.by_pair[("dispatch", "after")]["expected"]["recovery_state"], "uncertain")
        self.assertEqual(self.by_pair[("effect_observation", "before")]["expected"]["recovery_state"], "uncertain")

    def test_durable_observation_rebuilds_forward_without_effect_replay(self) -> None:
        for boundary in fixtures.BOUNDARIES[4:]:
            item = self.by_pair[(boundary, "after")]
            self.assertTrue(item["expected"]["effect_observation_durable"])
            self.assertFalse(item["expected"]["effect_replay_allowed"])

    def test_terminal_commit_is_the_only_absorbing_terminal(self) -> None:
        terminal = [item for item in self.suite["fixtures"] if item["expected"]["terminal_event_durable"]]
        self.assertEqual([(item["boundary"], item["position"]) for item in terminal], [("terminal_event_commit", "after")])
        self.assertEqual(terminal[0]["expected"]["recovery_action"], "reopen_absorbing_terminal")

    def test_no_case_reuses_approval_replays_effect_or_false_completes(self) -> None:
        for item in self.suite["fixtures"]:
            self.assertFalse(item["expected"]["approval_reuse_allowed"])
            self.assertFalse(item["expected"]["effect_replay_allowed"])
            self.assertFalse(item["expected"]["false_completion_allowed"])
            self.assertTrue(fixtures.valid_hash(item, "fixture_sha256"))

    def test_mutation_execution_and_replay_overclaims_fail(self) -> None:
        changed = copy.deepcopy(self.suite)
        changed["fixtures"].pop()
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["fixtures"][0]["expected"]["effect_replay_allowed"] = True
        self.assertTrue(fixtures.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["product_runtime_executed"] = True
        self.assertTrue(fixtures.validate_suite(changed))

    def test_checked_suite_matches_the_reproducible_builder(self) -> None:
        self.assertEqual(fixtures.check(), [])


if __name__ == "__main__":
    unittest.main()
