import json,unittest
from scripts.gitops_control_contract import ATTACKS,EFFECTS,FAULTS,PROVIDERS,READS,build
class GitOpsControlContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_reads_and_plans(self):self.assertEqual(tuple(self.value["providers"]),PROVIDERS);self.assertEqual(tuple(self.value["reads"]),READS);self.assertEqual(tuple(self.value["effects"]),EFFECTS);self.assertTrue(all(not c["effect_count"] and not c["secret_value_count"] for c in self.value["read_cases"]));self.assertTrue(all(not c["strong_effect_enabled_count"] and not c["desired_state_write_count"] for c in self.value["plan_cases"]))
 def test_attacks_fail_closed(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["controller_contact_count"] and not c["authority_escape_count"] and not c["hidden_effect_count"] for c in self.value["attack_cases"]))
 def test_faults_reconcile(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["exact_truth"] and c["reconciliation_required"] and not c["duplicate_effect_count"] and not c["unsafe_retry_count"] for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_provider_count"],0)
if __name__=="__main__":unittest.main()
