import json,unittest
from scripts.cloud_observer_contract import build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.v["case_count"],10800);self.assertEqual(len(self.v["reads"]),10);self.assertEqual(len(self.v["prohibited"]),13)
 def test_truth(self):self.assertTrue(all(c["exact_scope"]and c["limits_visible"]and c["limitations_visible"]for c in self.v["cases"]))
 def test_zero(self):self.assertEqual(sum(self.v[k]for k in ("escape_count","provider_effect_count","authority_count","residue_count")),0)
if __name__=="__main__":unittest.main()
