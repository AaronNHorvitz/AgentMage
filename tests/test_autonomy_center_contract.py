import json,unittest
from scripts.autonomy_center_contract import LEVELS,MUTATIONS,ORIGINS,build
class AutonomyCenterContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_dimensions_and_floor(self):self.assertEqual(tuple(self.value["levels"]),LEVELS);self.assertEqual(tuple(self.value["origins"]),ORIGINS);self.assertEqual(tuple(self.value["mutations"]),MUTATIONS);self.assertGreaterEqual(self.value["case_count"],2000)
 def test_no_broadening_or_shell_authority(self):self.assertEqual(self.value["authority_broadening_count"],0);self.assertEqual(self.value["shell_authority_count"],0)
 def test_mutations_and_emergency_deny_before_effect(self):self.assertTrue(all(not c["effect_started"]for c in self.value["cases"]if c["mutation"]!="none"or c["emergency_disabled"]))
 def test_prohibited_operations_absent(self):self.assertEqual(self.value["prohibited_operations"],["money_movement","cloud_mutation"]);self.assertFalse(any(c["operation"]in self.value["prohibited_operations"]for c in self.value["cases"]))
if __name__=="__main__":unittest.main()
