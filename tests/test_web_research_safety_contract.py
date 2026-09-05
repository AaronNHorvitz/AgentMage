import json,unittest
from scripts.web_research_safety_contract import HAZARDS,PRIVATE,STATES,build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_count(self):self.assertEqual(self.v["case_count"],2688)
 def test_coverage(self):self.assertEqual((len(STATES),len(HAZARDS),len(PRIVATE)),(7,12,8))
 def test_zero(self):self.assertTrue(all(x["citation_bound"]and x["freshness_visible"] for x in self.v["cases"]));self.assertEqual(sum(self.v[k]for k in ("authority_count","undeclared_egress_count","raw_private_count","unadmitted_download_count")),0)
if __name__=="__main__":unittest.main()
