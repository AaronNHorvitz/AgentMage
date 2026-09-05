import json,unittest
from scripts.cross_pack_verification_contract import build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],15360);self.assertEqual(len(self.value["prohibited"]),8)
 def test_truth(self):self.assertTrue(all(case["attributable"]and case["receipted"]and case["evidence_current"] for case in self.value["cases"]))
 def test_zero(self):self.assertEqual(sum(self.value[key]for key in ("unauthorized_disclosure_count","unauthorized_effect_count","money_movement_count","cloud_mutation_count","duplicate_count","false_completion_count","hidden_blocker_count","residue_count")),0)
if __name__=="__main__":unittest.main()
