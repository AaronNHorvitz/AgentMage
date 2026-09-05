import json,unittest
from scripts.communications_slack_contract import FAILURES,MUTATIONS,PROFILES,STATES,build
class SlackContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.v["profiles"]),PROFILES);self.assertEqual(tuple(self.v["mutations"]),MUTATIONS);self.assertEqual(tuple(self.v["failures"]),FAILURES);self.assertEqual(tuple(self.v["lifecycle_states"]),STATES);self.assertEqual(self.v["case_count"],4800)
 def test_mutations_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["cases"]if c["mutation"]!="none"))
 def test_uncertainty_is_visible(self):self.assertTrue(all(not c["presented_complete"]for c in self.v["cases"]if c["uncertain_visible"]))
 def test_no_duplicate_cross_workspace_false_or_residual(self):self.assertEqual(sum(self.v[k]for k in ("duplicate_post_count","cross_workspace_disclosure_count","false_completion_count","removal_authority_count")),0)
if __name__=="__main__":unittest.main()
