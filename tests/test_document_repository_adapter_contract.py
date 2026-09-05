import json,unittest
from scripts.document_repository_adapter_contract import build
class RepositoryAdapterTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(self.v["case_count"],6720)
 def test_mutations_and_unregistered_profiles_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["cases"]if c["mutation"]!="none"or c["profile"].endswith("_unregistered")))
 def test_content_never_creates_authority(self):self.assertTrue(all(not c["content_authority"]for c in self.v["cases"]))
 def test_no_disclosure_duplicate_false_or_residual(self):self.assertEqual(sum(self.v[k]for k in ("disclosure_count","duplicate_effect_count","false_completion_count","removal_authority_count")),0)
if __name__=="__main__":unittest.main()
