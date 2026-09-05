import json,unittest
from scripts.communications_mail_contract import ATTACKS,FAULTS,PROFILES,build
class MailContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.v["profiles"]),PROFILES);self.assertEqual(tuple(self.v["attacks"]),ATTACKS);self.assertEqual(tuple(self.v["faults"]),FAULTS);self.assertEqual(self.v["case_count"],5040)
 def test_attacks_disclose_no_credential_and_have_no_effect(self):self.assertTrue(all(c["credential_disclosure"]==0 and c["effect_count"]==0 for c in self.v["cases"]if c["attack"]!="none"))
 def test_uncertain_is_visible(self):self.assertTrue(all(not c["presented_complete"]for c in self.v["cases"]if c["uncertain_visible"]))
 def test_no_duplicate_false_or_residual_authority(self):self.assertEqual(sum(self.v[k]for k in ("duplicate_delivery_count","false_completion_count","removal_authority_count")),0)
if __name__=="__main__":unittest.main()
