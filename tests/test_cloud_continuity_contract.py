import json,unittest
from scripts.cloud_continuity_contract import ATTACKS,FAILURES,PROVIDERS,build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_count(self):self.assertEqual(self.v["case_count"],2160)
 def test_coverage(self):self.assertEqual((len(PROVIDERS),len(ATTACKS),len(FAILURES)),(6,10,9))
 def test_zero(self):self.assertTrue(all(x["fail_closed"]for x in self.v["cases"]));self.assertEqual(sum(self.v[k]for k in ("out_of_scope_count","duplicate_effect_count","plaintext_count","raw_credential_count","observer_crossover_count")),0)
if __name__=="__main__":unittest.main()
