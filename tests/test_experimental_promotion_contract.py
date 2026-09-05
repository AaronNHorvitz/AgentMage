import json,unittest
from scripts.experimental_promotion_contract import MUTATIONS,ROUTES,STATES,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_matrix(self):self.assertEqual((len(ROUTES),len(MUTATIONS),len(STATES),self.value["case_count"]),(10,13,16,2080))
 def test_no_promotion(self):self.assertEqual(sum(self.value[k]for k in ("ordinary_activation_count","direct_promotion_count","approved_state_change_count","canonical_damage_count")),0)
if __name__=="__main__":unittest.main()
