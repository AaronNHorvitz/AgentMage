from __future__ import annotations

import copy
import json
import unittest

from scripts import artifact_evaluation_artifact_metrics as metrics


class ArtifactEvaluationArtifactMetricTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.suite = json.loads(metrics.OUTPUT_PATH.read_text(encoding="utf-8"))
        cls.by_id = {item["metric_id"]: item for item in cls.suite["metrics"]}

    def test_required_metric_set_is_exact(self) -> None:
        self.assertEqual(self.suite["golden_manifest_version"], "1.0.0")
        self.assertEqual(tuple(self.by_id), metrics.METRIC_IDS)
        self.assertEqual(self.suite["metric_count"], 10)

    def test_extraction_provenance_and_section_goldens_are_complete(self) -> None:
        self.assertEqual(self.by_id["extraction_coverage"]["golden"]["fraction"], {"numerator": 30, "denominator": 30})
        self.assertEqual(self.by_id["provenance_accuracy"]["golden"]["fraction"], {"numerator": 14, "denominator": 14})
        self.assertEqual(self.by_id["section_fidelity"]["golden"]["fraction"], {"numerator": 6, "denominator": 6})

    def test_context_inclusion_omission_and_tokens_reconcile_exactly(self) -> None:
        self.assertEqual(self.by_id["context_inclusion"]["golden"]["fraction"], {"numerator": 3, "denominator": 3})
        self.assertEqual(self.by_id["context_omission"]["golden"]["fraction"], {"numerator": 205, "denominator": 205})
        token = self.by_id["token_budget_reconciliation"]["golden"]
        self.assertEqual(token["fraction"], {"numerator": 8, "denominator": 8})
        self.assertEqual(token["accounted_input_tokens"], 32)

    def test_latency_and_memory_are_logical_boundaries_not_product_results(self) -> None:
        latency = self.by_id["latency_boundary"]
        memory = self.by_id["peak_memory_boundary"]
        self.assertEqual((latency["golden"]["inclusive_ceiling"], latency["golden"]["first_denied_attempt"]), (5, 6))
        self.assertEqual((memory["golden"]["inclusive_ceiling"], memory["golden"]["first_denied_attempt"]), (8192, 12288))
        self.assertIn("not_product", latency["measurement_scope"])
        self.assertIn("not_product", memory["measurement_scope"])

    def test_cancellation_and_cleanup_goldens_are_zero_residue(self) -> None:
        self.assertEqual(self.by_id["cancellation"]["golden"]["fraction"], {"numerator": 2, "denominator": 2})
        self.assertEqual(self.by_id["cleanup"]["golden"]["fraction"], {"numerator": 7, "denominator": 7})
        self.assertEqual(self.by_id["cleanup"]["golden"]["expected_residue_count"], 0)

    def test_every_product_observation_remains_explicitly_unexecuted(self) -> None:
        for item in self.suite["metrics"]:
            self.assertIsNone(item["product_observation"])
            self.assertEqual(item["product_measurement_status"], "not_executed")
            self.assertEqual(item["support_claim"], "none")

    def test_mutation_false_metric_and_performance_overclaims_fail(self) -> None:
        changed = copy.deepcopy(self.suite)
        changed["metrics"].pop()
        self.assertTrue(metrics.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["metrics"][0]["golden"]["fraction"]["numerator"] -= 1
        self.assertTrue(metrics.validate_suite(changed))
        changed = copy.deepcopy(self.suite)
        changed["product_performance_measured"] = True
        self.assertTrue(metrics.validate_suite(changed))

    def test_checked_metrics_match_the_reproducible_builder(self) -> None:
        self.assertTrue(metrics.valid_hash(self.suite, "suite_sha256"))
        self.assertEqual(metrics.check(), [])


if __name__ == "__main__":
    unittest.main()
