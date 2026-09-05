import json,unittest
from scripts.financial_domain_contract import build
class FinancialDomainContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_exact_matrix(self):self.assertEqual(self.value["case_count"],8400);self.assertEqual(self.value["attack_count"],48)
 def test_no_binary_float_or_order_dependence(self):self.assertTrue(all(not case["binary_float"]and case["exact"]and case["deterministic"]for case in self.value["cases"]));self.assertEqual(self.value["order_dependent_count"],0)
 def test_currency_scale_rounding_coverage(self):self.assertEqual(len(self.value["currencies"]),7);self.assertEqual(self.value["scales"],[0,2,3,6,18]);self.assertEqual(len(self.value["rounding_modes"]),5)
 def test_closed_financial_object_model(self):self.assertEqual(len(self.value["object_kinds"]),20)
 def test_mutations_fail_before_storage_with_lineage(self):self.assertTrue(all(not attack["stored"]and attack["visible_failure"]and attack["lineage_preserved"]for attack in self.value["attacks"]))
 def test_zero_integrity_failures(self):self.assertEqual(sum(self.value[key]for key in ("binary_float_count","inexact_result_count","order_dependent_count","silent_failure_count","lineage_loss_count")),0)
if __name__=="__main__":unittest.main()
