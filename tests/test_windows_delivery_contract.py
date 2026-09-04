import json,unittest
from scripts.windows_delivery_contract import ATTACKS,FAULTS,LIFECYCLES,build
class WindowsDeliveryContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_lifecycles(self):self.assertEqual(tuple(self.value["lifecycles"]),LIFECYCLES);self.assertTrue(all(c["synthetic"]and c["per_user"]and not c["external_effect_count"]for c in self.value["lifecycle_cases"]))
 def test_attacks(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["peer_admitted_count"]and not c["authority_count"]and not c["handle_leak_count"]for c in self.value["attack_cases"]))
 def test_faults(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["safe_recovery"]and not c["duplicate_effect_count"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_platform_count"],0)
if __name__=="__main__":unittest.main()
