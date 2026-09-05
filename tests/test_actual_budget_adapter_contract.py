import json,unittest
from scripts.actual_budget_adapter_contract import build
class ActualBudgetAdapterContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],4680);self.assertEqual(len(self.value["operations"]),9)
 def test_unsupported_profiles_have_no_effect(self):self.assertTrue(all(case["effect_count"]==0 for case in self.value["cases"]if case["profile"]!="supported"))
 def test_mutations_and_failures_have_no_effect(self):self.assertTrue(all(case["effect_count"]==0 for case in self.value["cases"]if case["mutation"]!="none"or case["failure"]!="success"))
 def test_money_movement_and_network_broadening_absent(self):self.assertEqual(self.value["money_movement_count"]+self.value["network_broadening_count"],0)
 def test_integrity_and_removal_zero(self):self.assertEqual(sum(self.value[key]for key in ("wrong_file_write_count","duplicate_effect_count","precision_loss_count","silent_rule_change_count","corrupt_recovery_count","removal_authority_count")),0)
if __name__=="__main__":unittest.main()
