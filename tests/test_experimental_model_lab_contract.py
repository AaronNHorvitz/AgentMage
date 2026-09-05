import json,unittest
from scripts.experimental_model_lab_contract import ARTIFACTS,FAULTS,ROUTES,STAGES,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual((len(ARTIFACTS),len(STAGES),len(ROUTES),len(FAULTS)),(9,8,12,8));self.assertEqual(self.value["case_count"],6912)
 def test_isolation(self):self.assertTrue(all(x["experimental_visible"]and x["provenance_gaps_visible"]for x in self.value["cases"]));self.assertEqual(sum(self.value[k]for k in ("authority_count","network_count","canonical_write_count","approved_store_count","hidden_limitation_count")),0)
if __name__=="__main__":unittest.main()
