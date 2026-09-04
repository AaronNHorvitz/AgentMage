import json,unittest
from scripts.ci_control_contract import ATTACKS,EFFECTS,IDENTITIES,PROVIDERS,build
class CiControlContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_observations_are_exact_and_secret_free(self):self.assertEqual(tuple(self.value["providers"]),PROVIDERS);self.assertEqual(tuple(self.value["identities"]),IDENTITIES);self.assertTrue(all(c["source_bound"] and c["worker_bound"] and not c["secret_value_count"] for c in self.value["observation_cases"]))
 def test_effects_are_inert_and_non_inheriting(self):self.assertEqual(tuple(self.value["effects"]),EFFECTS);self.assertTrue(all(c["separate_grant"] and not c["provider_request_count"] and not c["deployment_authority_count"] and not c["secret_authority_count"] for c in self.value["effect_cases"]))
 def test_faults_have_zero_duplicate_runs(self):self.assertEqual(self.value["fault_case_count"],1024);self.assertTrue(all(not c["duplicate_run_count"] and not c["unsafe_retry_count"] and c["unknown_until_reconciled"] for c in self.value["fault_cases"]))
 def test_hostile_outputs_remain_bounded(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["secret_disclosure_count"] and not c["stronger_capability_count"] and not c["unsafe_archive_count"] and not c["unbounded_output_count"] for c in self.value["attack_cases"]));self.assertEqual(self.value["live_provider_count"],0)
if __name__=="__main__":unittest.main()
