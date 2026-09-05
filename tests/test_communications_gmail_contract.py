import json,unittest
from scripts.communications_gmail_contract import FAULTS,MUTATIONS,PROFILES,build
class GmailContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.v["profiles"]),PROFILES);self.assertEqual(tuple(self.v["mutations"]),MUTATIONS);self.assertEqual(tuple(self.v["faults"]),FAULTS);self.assertEqual(self.v["case_count"],4320)
 def test_changed_fields_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["cases"]if c["mutation"]!="none"))
 def test_incomplete_is_visible(self):self.assertTrue(all(not c["presented_complete"]for c in self.v["cases"]if c["incomplete_visible"]))
 def test_no_duplicate_false_or_residual_authority(self):self.assertEqual(self.v["duplicate_delivery_count"]+self.v["false_completion_count"]+self.v["removal_authority_count"],0)
if __name__=="__main__":unittest.main()
