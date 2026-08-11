import copy
import unittest

from scripts import grant_stale_dispatch as stale


class GrantStaleDispatchTests(unittest.TestCase):
    def test_every_post_approval_change_fails_before_worker_start(self) -> None:
        coverage = stale.validate_traces(stale.run_traces())
        self.assertEqual(coverage["mutation_count"], 4)
        self.assertEqual(coverage["atomic_consumption_count"], 0)
        self.assertEqual(coverage["worker_start_count"], 0)
        self.assertEqual(coverage["effect_count"], 0)
        self.assertEqual(coverage["exact_replay_success_count"], 0)
        self.assertEqual(coverage["terminal_invalidated_count"], 4)

    def test_missing_mutation_and_wrong_denial_scope_are_rejected(self) -> None:
        traces = stale.run_traces()
        with self.assertRaises(stale.GrantStaleDispatchError):
            stale.validate_traces(traces[:-1])
        changed = copy.deepcopy(traces)
        changed[1]["denial_scope"] = "grant"
        with self.assertRaises(stale.GrantStaleDispatchError):
            stale.validate_traces(changed)

    def test_consumption_worker_effect_and_replay_success_are_rejected(self) -> None:
        traces = stale.run_traces()
        for field, value in (
            ("atomic_consumption_count", 1),
            ("worker_start_count", 1),
            ("effect_count", 1),
            ("exact_replay_allowed", True),
        ):
            changed = copy.deepcopy(traces)
            changed[0][field] = value
            with self.assertRaises(stale.GrantStaleDispatchError):
                stale.validate_traces(changed)


if __name__ == "__main__":
    unittest.main()
