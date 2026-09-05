import json,unittest
from scripts.model_management_contract import INTENTS,MUTATIONS,OUTCOMES,TRANSITIONS,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_dimensions(self):self.assertEqual((len(INTENTS),len(MUTATIONS),len(TRANSITIONS),len(OUTCOMES)),(13,24,10,13))
 def test_required_scale(self):self.assertEqual(self.value["case_count"],3120);self.assertGreaterEqual(self.value["case_count"],3000)
 def test_no_unsafe_activation(self):
  self.assertTrue(all(x["prior_profile_preserved"]and x["quarantine_isolated"]for x in self.value["cases"]))
  self.assertEqual(sum(self.value[k]for k in ("silent_activation_count","partial_activation_count","substituted_profile_count","unconfirmed_effect_count")),0)
if __name__=="__main__":unittest.main()
