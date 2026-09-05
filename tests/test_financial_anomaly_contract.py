import json,unittest
from scripts.financial_anomaly_contract import build
class FinancialAnomalyContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],7560);self.assertEqual(len(self.value["kinds"]),10);self.assertEqual(len(self.value["methods"]),3)
 def test_replay_and_explanation_are_complete(self):self.assertTrue(all(case["replay_digest_visible"]and case["features_visible"]and case["baseline_visible"]and case["threshold_visible"]and case["source_visible"]for case in self.value["cases"]))
 def test_uncertainty_is_visible(self):self.assertTrue(all(case["confidence_visible"]and case["limitations_visible"]and case["potential_signal"]for case in self.value["cases"]));self.assertEqual(self.value["hidden_limitation_count"],0)
 def test_metrics_are_exact_bounded_and_limited(self):self.assertTrue(all(isinstance(value,int)and 0<=value<=10000 for key,value in self.value["metrics"].items()if key.endswith("basis_points")));self.assertTrue(self.value["metrics"]["subgroup_limitations"])
 def test_no_claim_authority_rewrite_or_drift(self):self.assertEqual(sum(self.value[key]for key in ("definitive_claim_count","authority_count","evidence_rewrite_count","replay_drift_count")),0)
if __name__=="__main__":unittest.main()
