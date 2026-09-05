import json,unittest
from scripts.trusted_audit_contract import build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_counts(self):self.assertEqual((self.v["trusted_case_count"],self.v["audit_case_count"]),(1650,1170))
 def test_truth(self):self.assertTrue(all(x["exact_class"]and x["fail_closed"] for x in self.v["trusted_cases"]));self.assertTrue(all(x["source_bound"]and x["explicit_disposition"] for x in self.v["audit_cases"]))
 def test_zero(self):self.assertEqual(sum(self.v[k] for k in ("union_count","effect_count","audit_authority_count","audit_completeness_count")),0)
if __name__=="__main__":unittest.main()
