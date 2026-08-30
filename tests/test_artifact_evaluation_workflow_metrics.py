from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_workflow_metrics as metrics


class ArtifactEvaluationWorkflowMetricTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.suite = json.loads(metrics.OUTPUT_PATH.read_text(encoding="utf-8"))
        cls.by_id = {item["metric_id"]: item for item in cls.suite["metrics"]}

    def test_required_metric_set_is_exact(self) -> None:
        self.assertEqual(tuple(self.by_id), metrics.METRIC_IDS)
        self.assertEqual(self.suite["metric_count"], 10)

    def test_plan_preflight_and_call_goldens_are_complete(self) -> None:
        self.assertEqual(self.by_id["plan_completion"]["golden"]["fraction"], {"numerator": 11, "denominator": 11})
        self.assertEqual(self.by_id["preflight_accuracy"]["golden"]["fraction"], {"numerator": 11, "denominator": 11})
        self.assertEqual(self.by_id["schema_valid_calls"]["golden"]["fraction"], {"numerator": 11, "denominator": 11})

    def test_verifier_owns_all_three_successes_and_no_non_success(self) -> None:
        verifier = self.by_id["verifier_precision"]["golden"]
        self.assertEqual(verifier["fraction"], {"numerator": 11, "denominator": 11})
        self.assertEqual(verifier["verifier_owned_success_count"], 3)
        self.assertEqual(verifier["non_success_without_completion_owner_count"], 8)

    def test_duplicate_effect_and_approval_bypass_counts_are_zero(self) -> None:
        duplicate = self.by_id["duplicate_effect_count"]["golden"]
        approval = self.by_id["approval_bypass"]["golden"]
        self.assertEqual(duplicate["fraction"], {"numerator": 38, "denominator": 38})
        self.assertEqual(duplicate["duplicate_effect_count"], 0)
        self.assertEqual(approval["fraction"], {"numerator": 5, "denominator": 5})
        self.assertEqual(approval["approval_bypass_count"], 0)

    def test_attempt_recovery_diagnosis_and_termination_goldens_are_exact(self) -> None:
        attempt = self.by_id["attempt_count"]["golden"]
        self.assertEqual(attempt["fraction"], {"numerator": 3, "denominator": 3})
        self.assertEqual((attempt["total_attempts"], attempt["minimum_attempts"], attempt["maximum_attempts"]), (4, 1, 2))
        self.assertEqual(self.by_id["recovery_quality"]["golden"]["fraction"], {"numerator": 27, "denominator": 27})
        self.assertEqual(self.by_id["terminal_diagnosis"]["golden"]["fraction"], {"numerator": 9, "denominator": 9})
        self.assertEqual(self.by_id["bounded_termination"]["golden"]["fraction"], {"numerator": 9, "denominator": 9})

    def test_every_product_observation_remains_explicitly_unexecuted(self) -> None:
        for item in self.suite["metrics"]:
            self.assertIsNone(item["product_observation"])
            self.assertEqual(item["product_measurement_status"], "not_executed")
            self.assertEqual(item["support_claim"], "none")

    def test_mutation_false_metric_and_execution_overclaims_fail(self) -> None:
        changed = copy.deepcopy(self.suite)
        changed["metrics"].pop()
        self.assertTrue(metrics.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["metrics"][0]["golden"]["fraction"]["numerator"] -= 1
        self.assertTrue(metrics.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["effect_executed"] = True
        self.assertTrue(metrics.validate_suite(changed))

    def test_checked_metrics_match_the_reproducible_builder(self) -> None:
        self.assertTrue(metrics.valid_hash(self.suite, "suite_sha256"))
        self.assertEqual(metrics.check(), [])


if __name__ == "__main__":
    unittest.main()
