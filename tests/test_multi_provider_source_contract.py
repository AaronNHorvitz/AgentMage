import json,unittest
from scripts.multi_provider_source_contract import ATTACKS,EFFECTS,PROHIBITED,PROVIDERS,READS,TUPLES,build
class MultiProviderSourceContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_published_synthetic_matrix_is_truthful(self):self.assertEqual(tuple(self.value["providers"]),PROVIDERS);self.assertEqual(tuple(self.value["tuples"]),TUPLES);self.assertTrue(all(c["synthetic"] and not c["promoted"] and not c["live"] for c in self.value["matrix_cases"]))
 def test_reads_preserve_identity_and_checkouts(self):self.assertEqual(tuple(self.value["reads"]),READS);self.assertTrue(all(c["immutable_identity"] and c["namespaced_fields"] and c["isolated_worktree"] and not c["active_checkout_change_count"] for c in self.value["read_cases"]))
 def test_effects_are_inert_exact_and_separate(self):self.assertEqual(tuple(self.value["effects"]),EFFECTS);self.assertTrue(all(c["draft_only"] and c["separate_approval"] and not c["provider_request_count"] for c in self.value["effect_cases"]));self.assertEqual(tuple(self.value["prohibited"]),PROHIBITED)
 def test_attacks_and_recovery_fail_closed(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["wrong_domain_count"] and not c["stale_effect_count"] for c in self.value["attack_cases"]));self.assertTrue(all(not c["duplicate_effect_count"] and c["reconciled"] for c in self.value["recovery_cases"]));self.assertEqual(self.value["live_provider_count"],0)
if __name__=="__main__":unittest.main()
