import json,unittest
from scripts.cloud_provider_observer_contract import build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],19200);self.assertEqual(len(self.value["providers"]),3);self.assertEqual(len(self.value["prohibited"]),11)
 def test_native_identity(self):self.assertTrue(all(len(self.value["native_identities"][provider])==8 for provider in self.value["providers"]));self.assertTrue(all(case["native_identity"] for case in self.value["cases"]))
 def test_truth(self):self.assertTrue(all(case["exact_scope"]and case["limitations_visible"] for case in self.value["cases"]))
 def test_zero(self):self.assertEqual(sum(self.value[key]for key in ("secret_disclosure_count","escape_count","provider_effect_count","residual_authority_count")),0)
if __name__=="__main__":unittest.main()
