import json,unittest
from scripts.communications_outlook_contract import FAILURES,MUTATIONS,PROFILES,build
class OutlookContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions_and_floors(self):self.assertEqual(tuple(self.v["profiles"]),PROFILES);self.assertEqual(tuple(self.v["mutations"]),MUTATIONS);self.assertEqual(tuple(self.v["failures"]),FAILURES);self.assertGreaterEqual(self.v["mutation_case_count"],2000);self.assertGreaterEqual(self.v["retry_case_count"],1000)
 def test_mutations_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["cases"]if c["mutation"]!="none"))
 def test_no_duplicate_or_false_completion(self):self.assertEqual(self.v["duplicate_delivery_count"]+self.v["false_completion_count"],0)
 def test_removal_zero_authority(self):self.assertEqual(self.v["removal_authority_count"],0)
if __name__=="__main__":unittest.main()
