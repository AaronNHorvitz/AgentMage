import json,unittest
from scripts.deployment_safety_contract import ACTIONS,ATTACKS,FAULTS,READS,RENDERERS,build
class DeploymentSafetyContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_offline_reads_and_renders(self):self.assertEqual(tuple(self.value["reads"]),READS);self.assertEqual(tuple(self.value["renderers"]),RENDERERS);self.assertTrue(all(c["offline"] and c["deterministic"] and c["source_preserved"] for c in self.value["render_cases"]))
 def test_exact_plans_and_attacks(self):self.assertEqual(tuple(self.value["actions"]),ACTIONS);self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(c["resource_pre_state_bound"] and c["resource_post_state_bound"] and not c["secret_or_admin_authority_count"] for c in self.value["plan_cases"]));self.assertTrue(all(not c["cluster_contact_count"] and not c["authority_escape_count"] for c in self.value["attack_cases"]))
 def test_faults_require_reconciliation(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["exact_state"] and c["reconciliation_required"] and not c["automatic_retry_count"] and c["fresh_rollback_approval_required"] for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_environment_count"],0)
if __name__=="__main__":unittest.main()
