import json,unittest
from scripts.cloud_delivery_correlation_contract import build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.value["case_count"],6048);self.assertEqual(len(self.value["association_classes"]),6)
 def test_attribution(self):self.assertTrue(all(case["native_identity_visible"]and case["source_cited"]and case["time_visible"]and case["cost_assumptions_visible"] for case in self.value["cases"]))
 def test_zero(self):self.assertEqual(self.value["causal_claim_count"]+self.value["inherited_authority_count"],0)
if __name__=="__main__":unittest.main()
