import json,unittest
from scripts.model_catalog_semantic_contract import CHANNELS,COMPAT,MUTATIONS,PARSERS,PARTITIONS,STATES,VARIATIONS,build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_counts(self):self.assertEqual((self.v["catalog_case_count"],self.v["semantic_case_count"]),(3136,640))
 def test_coverage(self):self.assertEqual((len(STATES),len(COMPAT),len(MUTATIONS),len(CHANNELS),len(PARSERS),len(PARTITIONS),len(VARIATIONS)),(7,7,8,8,10,8,8))
 def test_zero(self):self.assertTrue(all(x["exact_identity"]for x in self.v["catalog_cases"]));self.assertTrue(all(x["source_bound"]for x in self.v["semantic_cases"]));self.assertEqual(sum(self.v[k]for k in ("unauthorized_usability_count","family_inheritance_count","omitted_negative_count","structural_gap_count","coverage_gap_count","authority_change_count","completion_change_count")),0)
if __name__=="__main__":unittest.main()
