import json,unittest
from scripts.github_full_negative_contract import ATTACKS,PROHIBITED,build
class GithubFullNegativeContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_complete_negative_campaign(self):self.assertEqual(tuple(self.value["attack_classes"]),ATTACKS);self.assertEqual(self.value["case_count"],10000);self.assertTrue(all(not c["unauthorized_effect_count"] and not c["user_work_loss_count"] for c in self.value["cases"]))
 def test_prohibited_registry_is_closed(self):self.assertEqual(tuple(self.value["prohibited_operations"]),PROHIBITED);self.assertTrue(all(not c["provider_request_count"] and c["receipt_count"]==1 for c in self.value["cases"]))
 def test_support_and_rv49_overclaims_absent(self):self.assertEqual(self.value["supported_github_com_tuple_count"],0);self.assertEqual(self.value["supported_ghes_tuple_count"],0);self.assertEqual(self.value["native_request_count"],0);self.assertFalse(self.value["complete_rv49"])
if __name__=="__main__":unittest.main()
