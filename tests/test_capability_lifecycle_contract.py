import json,unittest
from scripts.capability_lifecycle_contract import CASES,CANDIDATES,build
class CapabilityLifecycleContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_exact_disabled_candidates(self):self.assertEqual([c["capability_id"] for c in self.value["candidates"]],[f"agentmage.{n}" for n in CANDIDATES]);self.assertTrue(all(c["lifecycle"]=="disabled" for c in self.value["candidates"]))
 def test_lifecycle_and_attack_matrix(self):self.assertEqual(len(self.value["lifecycle_states"]),7);self.assertEqual(self.value["case_count"],66);self.assertEqual({c["class"] for c in self.value["cases"]},set(CASES))
 def test_removal_has_no_residue(self):self.assertTrue(all(c["residual_registration_count"]==0 and c["residual_authority_count"]==0 for c in self.value["cases"]))
 def test_native_strict_local_claim_absent(self):self.assertEqual(self.value["native_strict_local_restoration_count"],0)
if __name__=="__main__":unittest.main()
