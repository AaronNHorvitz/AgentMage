import json,unittest
from scripts.finance_privacy_contract import build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.v["case_count"],12285);self.assertEqual(len(self.v["families"]),13);self.assertEqual(len(self.v["scan_classes"]),12)
 def test_receipts(self):self.assertTrue(all(c["policy_receipt"]and c["canary_digest_only"]for c in self.v["cases"]))
 def test_zero(self):self.assertEqual(sum(self.v[k]for k in ("undeclared_disclosure_count","capability_shape_count","external_effect_count","canary_disclosure_count","residual_count")),0)
if __name__=="__main__":unittest.main()
