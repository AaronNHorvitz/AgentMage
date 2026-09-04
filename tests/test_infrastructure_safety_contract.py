import json,unittest
from scripts.infrastructure_safety_contract import ATTACKS,EFFECTS,FAULTS,READS,TOOLS,build
class InfrastructureSafetyContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_reads_and_plans(self):self.assertEqual(tuple(self.value["tools"]),TOOLS);self.assertEqual(tuple(self.value["reads"]),READS);self.assertEqual(tuple(self.value["effects"]),EFFECTS);self.assertTrue(all(not c["secret_value_count"]and not c["effect_count"]for c in self.value["read_cases"]));self.assertTrue(all(not c["implicit_approval_count"]and c["saved_plan_bound"]for c in self.value["plan_cases"]))
 def test_attacks_fail_closed(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["provider_request_count"]and not c["secret_value_count"]and not c["unsafe_apply_count"]for c in self.value["attack_cases"]))
 def test_faults_reconcile(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["unknown_state_blocked"]and not c["automatic_retry_count"]and not c["state_surgery_count"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_tool_count"],0)
if __name__=="__main__":unittest.main()
