import json,unittest
from scripts.communication_write_contract import FIELDS,FAILURES,PHASES,PROVIDERS,build
class CommunicationWriteTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions_and_floors(self):self.assertEqual(tuple(self.v["providers"]),PROVIDERS);self.assertEqual(tuple(self.v["fields"]),FIELDS);self.assertEqual(tuple(self.v["failures"]),FAILURES);self.assertEqual(tuple(self.v["phases"]),PHASES);self.assertEqual(self.v["case_count"],12000);self.assertGreaterEqual(self.v["field_mutation_case_count"],2000);self.assertGreaterEqual(self.v["uncertainty_schedule_count"],1000)
 def test_changed_fields_have_zero_effect(self):self.assertTrue(all(c["effect_count"]==0 for c in self.v["cases"]if c["mutated_field"]!="none"))
 def test_uncertainty_is_visible(self):self.assertTrue(all(c["uncertain_visible"]for c in self.v["cases"]if c["failure"]in {"timeout_before","timeout_after","partial","unknown","receipt_crash"}))
 def test_no_unauthorized_duplicate_false_or_emergency_effect(self):self.assertEqual(sum(self.v[k]for k in ("unauthorized_effect_count","duplicate_effect_count","false_completion_count","emergency_effect_count")),0)
if __name__=="__main__":unittest.main()
