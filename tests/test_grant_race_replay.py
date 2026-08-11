import copy
import json
import unittest

from scripts import grant_race_replay as race


class GrantRaceReplayTests(unittest.TestCase):
    def test_typed_races_cover_every_result_without_replay(self) -> None:
        coverage = race.validate_traces(race.run_traces())
        self.assertEqual(coverage["scenario_count"], 5)
        self.assertEqual(coverage["attempt_scenarios_with_one_consumption"], 4)
        self.assertEqual(coverage["policy_denial_scenarios_with_zero_consumption"], 1)
        self.assertEqual(coverage["maximum_worker_start_count"], 1)
        self.assertEqual(coverage["maximum_effect_count"], 1)
        self.assertEqual(coverage["replay_success_count"], 0)
        self.assertEqual(coverage["uncertain_terminal_scenario_count"], 3)

    def test_second_consumption_worker_start_and_effect_are_rejected(self) -> None:
        traces = race.run_traces()
        for field, value in (
            ("atomic_consumption_count", 2),
            ("worker_start_count", 2),
            ("effect_count_after_replay", 2),
        ):
            changed = copy.deepcopy(traces)
            changed[0][field] = value
            with self.assertRaises(race.GrantRaceReplayError):
                race.validate_traces(changed)

    def test_missing_result_and_nonterminal_uncertainty_are_rejected(self) -> None:
        traces = race.run_traces()
        with self.assertRaises(race.GrantRaceReplayError):
            race.validate_traces(traces[:-1])
        changed = json.loads(json.dumps(traces))
        changed[2]["terminal_status"] = "consumed"
        with self.assertRaises(race.GrantRaceReplayError):
            race.validate_traces(changed)


if __name__ == "__main__":
    unittest.main()
