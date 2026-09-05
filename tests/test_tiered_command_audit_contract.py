import json,unittest
from scripts.tiered_command_audit_contract import ATTACKS,HAZARDS,LEVELS,SEMANTICS,STATES,TRIGGERS,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_counts(self):self.assertEqual((self.value["authority_case_count"],self.value["audit_case_count"]),(6300,792))
 def test_coverage(self):self.assertEqual((len(LEVELS),len(SEMANTICS),len(ATTACKS),len(TRIGGERS),len(STATES),len(HAZARDS)),(5,14,10,9,22,12))
 def test_zero_effects(self):self.assertTrue(all(x["fail_closed"] for x in self.value["authority_cases"]));self.assertTrue(all(x["disposition_count"]==1 for x in self.value["audit_cases"]));self.assertEqual(sum(self.value[k]for k in ("escape_count","hidden_launch_count","surviving_process_count","canonical_mutation_count","hosted_mutation_count","raw_secret_count","content_authority_count","silent_omission_count","parser_escape_count")),0)
if __name__=="__main__":unittest.main()
