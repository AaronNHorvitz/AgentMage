import json,unittest
from scripts.financial_planning_contract import build
class FinancialPlanningContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],4320);self.assertEqual(len(self.value["kinds"]),6)
 def test_results_are_exact_and_order_stable(self):self.assertTrue(all(case["exact"]for case in self.value["cases"]));self.assertEqual(self.value["order_drift_count"],0)
 def test_evidence_and_uncertainty_stay_visible(self):self.assertTrue(all(case["source_visible"]and case["assumption_visible"]and case["version_visible"]and case["confidence_visible"]and case["limitation_visible"]for case in self.value["cases"]))
 def test_scenarios_are_labeled_and_not_guaranteed(self):self.assertTrue(all(case["scenario_labeled"]and not case["guaranteed_claim"]for case in self.value["cases"]))
 def test_no_execution_authority(self):self.assertEqual(self.value["execution_attempt_count"],0);self.assertEqual(len(self.value["prohibited"]),6)
if __name__=="__main__":unittest.main()
