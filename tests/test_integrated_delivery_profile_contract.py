import json,unittest
from scripts.integrated_delivery_profile_contract import ATTACKS,FAILURES,LIFECYCLES,SERVICES,build
class IntegratedDeliveryProfileContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.value["lifecycles"]),LIFECYCLES);self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertEqual(tuple(self.value["failures"]),FAILURES);self.assertEqual(tuple(self.value["services"]),SERVICES);self.assertEqual(len(self.value["profile_cases"]),49)
 def test_fail_closed(self):self.assertTrue(all(not c["authority_crossing_count"]and not c["false_completion_count"]for c in self.value["attack_cases"]));self.assertTrue(all(not c["service_impersonation_count"]and not c["self_approval_count"]and c["dissent_visible"]for c in self.value["profile_cases"]))
 def test_no_qualification_claim(self):self.assertEqual(self.value["native_platform_count"],0);self.assertEqual(self.value["promoted_provider_path_count"],0);self.assertEqual(self.value["enabled_profile_count"],0);self.assertEqual(self.value["promotion_count"],0)
if __name__=="__main__":unittest.main()
