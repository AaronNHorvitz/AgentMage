import json,unittest
from scripts.productivity_workflow_contract import build
class ProductivityWorkflowTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_case_count_and_determinism(self):self.assertEqual(self.v["case_count"],5600);self.assertTrue(all(c["deterministic"]for c in self.v["cases"]))
 def test_attacks_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["cases"]if c["attack"]!="none"))
 def test_no_inferred_consent_hidden_or_content_authority(self):self.assertEqual(sum(self.v[k]for k in ("inferred_consent_count","hidden_destination_count","content_authority_count")),0)
 def test_no_duplicate_or_residual_authority(self):self.assertEqual(self.v["duplicate_effect_count"]+self.v["removal_authority_count"],0)
if __name__=="__main__":unittest.main()
