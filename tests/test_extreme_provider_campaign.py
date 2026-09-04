import json,unittest
from scripts.extreme_provider_campaign import ATTACKS,BLOCKERS,FAILURES,PRESSURES,TRANSITIONS,VERSIONS,build
class ExtremeProviderCampaignTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions_are_complete(self):self.assertEqual(tuple(self.value["versions"]),VERSIONS);self.assertEqual(tuple(self.value["failures"]),FAILURES);self.assertEqual(tuple(self.value["pressures"]),PRESSURES);self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertEqual(tuple(self.value["transitions"]),TRANSITIONS);self.assertEqual(tuple(self.value["blockers"]),BLOCKERS)
 def test_attacks_and_failures_are_closed(self):self.assertTrue(all(not c["authority_leak_count"]and not c["stale_support_claim_count"]for c in self.value["conformance_cases"]));self.assertTrue(all(not c["canary_disclosure_count"]and not c["duplicate_effect_count"]and c["uncertainty_explicit"]for c in self.value["attack_cases"]));self.assertTrue(all(c["visible"]and not c["gate_closed"]for c in self.value["blocker_cases"]))
 def test_external_claims_are_zero(self):self.assertEqual(self.value["approved_live_environment_count"],0);self.assertEqual(self.value["native_fuzz_run_count"],0);self.assertEqual(self.value["review_vector_count"],0);self.assertEqual(self.value["promoted_provider_count"],0)
if __name__=="__main__":unittest.main()
