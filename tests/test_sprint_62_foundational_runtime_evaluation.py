"""Mutation tests for the Sprint 62 foundational-runtime evaluation."""

import copy
import subprocess
import unittest
from unittest.mock import patch

from scripts import sprint_62_foundational_runtime_evaluation as evaluation


class Sprint62FoundationalRuntimeEvaluationTests(unittest.TestCase):
    def test_blocked_evaluation_and_mutations(self) -> None:
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
        with patch.object(
            evaluation,
            "git_bytes",
            side_effect=lambda _revision, path: (evaluation.ROOT / path).read_bytes(),
        ):
            value = evaluation.expected(revision)
        self.assertEqual(value["status"], "BLOCKED_M_FOUNDATIONAL_RUNTIME")
        self.assertTrue(all(value["local_checks"].values()))
        self.assertEqual(value["enabled_model_profile_ids"], [])
        self.assertEqual(value["qualifying_installed_campaign_count"], 0)
        self.assertEqual(value["hidden_non_pass_state_count"], 0)
        self.assertEqual(value["blockers"], list(evaluation.BLOCKERS))
        with patch.object(evaluation, "expected", return_value=value):
            self.assertEqual(evaluation.validate(value), [])
            mutations = (
                lambda changed: changed.update({"status": "PASS"}),
                lambda changed: changed["rows"].update({"62.2.4.1": "PASS"}),
                lambda changed: changed.update({"enabled_model_profile_ids": ["invented"]}),
                lambda changed: changed.update({"hidden_non_pass_state_count": 1}),
                lambda changed: changed.update({"blockers": []}),
                lambda changed: changed.update({"limitations": []}),
            )
            for mutate in mutations:
                changed = copy.deepcopy(value)
                mutate(changed)
                self.assertTrue(evaluation.validate(changed))


if __name__ == "__main__":
    unittest.main()
