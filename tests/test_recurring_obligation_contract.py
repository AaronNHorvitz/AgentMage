import json,unittest
from scripts.recurring_obligation_contract import build
class RecurringObligationContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],5760);self.assertEqual(len(self.value["kinds"]),8)
 def test_citations_and_method_remain_visible(self):self.assertTrue(all(case["financial_citation"]and case["communication_citation"]and case["method_visible"]for case in self.value["cases"]))
 def test_uncertainty_never_disappears(self):self.assertTrue(all(case["uncertainty_visible"]for case in self.value["cases"]));self.assertEqual(self.value["hidden_uncertainty_count"],0)
 def test_mutations_and_failures_do_not_create_detection(self):self.assertTrue(all(case["detection_count"]==0 for case in self.value["cases"]if case["mutation"]!="none"or case["failure"]!="success"))
 def test_no_authority_false_completion_or_financial_effect(self):self.assertEqual(self.value["authority_count"]+self.value["false_completion_count"]+self.value["financial_effect_count"],0)
if __name__=="__main__":unittest.main()
