import json,unittest
from scripts.v1_plus_assurance_contract import PRIVACY,RECOVERY,build
class V1PlusAssuranceContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_recovery_and_privacy_closure(self):self.assertEqual(set(self.value["recovery_classes"]),set(RECOVERY));self.assertEqual(set(self.value["privacy_paths"]),set(PRIVACY));self.assertEqual(self.value["recovery_case_count"],72);self.assertEqual(self.value["privacy_case_count"],64)
 def test_zero_escape_or_unattributed_effect(self):self.assertTrue(all(not c["unauthorized_effect_count"] and not c["unattributed_actor_count"] for c in self.value["recovery_cases"]));self.assertTrue(all(not c["unauthorized_persistence_count"] and not c["unauthorized_disclosure_count"] and c["cleanup_verified"] for c in self.value["privacy_cases"]))
 def test_external_claims_absent(self):self.assertFalse(self.value["strict_local_all_packs_disabled_observed"]);self.assertEqual(self.value["native_interface_count"],0);self.assertFalse(self.value["independent_review"])
if __name__=="__main__":unittest.main()
