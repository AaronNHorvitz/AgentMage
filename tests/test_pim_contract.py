import json,unittest
from scripts.pim_contract import build
class PimContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_case_counts(self):self.assertEqual((self.v["pim_case_count"],self.v["proton_case_count"]),(9000,8400))
 def test_mutations_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["pim_cases"]+self.v["proton_cases"]if c["mutation"]!="none"))
 def test_proton_never_exposes_credentials_or_duplicates(self):self.assertTrue(all(not c["credential_exposure"]and not c["duplicate_event"]and not c["duplicate_invitation"]for c in self.v["proton_cases"]))
 def test_no_false_completion_or_residual_authority(self):self.assertEqual(sum(self.v[k]for k in ("false_completion_count","removal_authority_count")),0)
if __name__=="__main__":unittest.main()
