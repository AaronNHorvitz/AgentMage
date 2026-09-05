import json,unittest
from scripts.communications_teams_contract import FAILURES,MUTATIONS,PROFILES,STATES,build
class TeamsContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.v["profiles"]),PROFILES);self.assertEqual(tuple(self.v["mutations"]),MUTATIONS);self.assertEqual(tuple(self.v["failures"]),FAILURES);self.assertEqual(tuple(self.v["lifecycle_states"]),STATES);self.assertEqual(self.v["case_count"],4590)
 def test_mutations_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["cases"]if c["mutation"]!="none"))
 def test_no_disclosure_duplicate_or_false_completion(self):self.assertEqual(self.v["cross_chat_disclosure_count"]+self.v["duplicate_effect_count"]+self.v["false_completion_count"],0)
 def test_removal_zero_authority(self):self.assertEqual(self.v["removal_authority_count"],0)
if __name__=="__main__":unittest.main()
