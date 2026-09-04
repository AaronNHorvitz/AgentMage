import json,unittest
from scripts.connected_identity_contract import AXES,CAPABILITIES,CREDENTIAL_STATES,ORIGINS,SURFACES,build
class ConnectedIdentityContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_complete_identity_campaign(self):self.assertEqual(tuple(self.value["identity_axes"]),AXES);self.assertGreaterEqual(self.value["confusion_case_count"],2000);self.assertTrue(all(c["request_count"]==0 and c["disclosure_count"]==0 and c["receipt_count"]==1 for c in self.value["confusion_cases"]))
 def test_non_inheriting_capability_matrix(self):self.assertEqual(tuple(self.value["capability_classes"]),CAPABILITIES);self.assertEqual(tuple(self.value["escalation_origins"]),ORIGINS);self.assertEqual(self.value["escalation_case_count"],len(CAPABILITIES)*(len(CAPABILITIES)-1)*len(ORIGINS));self.assertTrue(all(not c["inherited"] and not c["provider_contact_count"] for c in self.value["escalation_cases"]))
 def test_credentials_canaries_and_workers(self):self.assertEqual(tuple(self.value["credential_states"]),CREDENTIAL_STATES);self.assertEqual(tuple(self.value["canary_surfaces"]),SURFACES);self.assertTrue(all(not c["secret_serialization_count"] for c in self.value["credential_cases"]));self.assertTrue(all(not c["canary_occurrences"] for c in self.value["canary_cases"]));self.assertEqual(self.value["concurrency_case_count"],64)
 def test_external_claims_absent(self):self.assertEqual(self.value["native_identity_count"],0);self.assertEqual(self.value["live_provider_count"],0);self.assertEqual(self.value["independent_review_count"],0);self.assertEqual(self.value["reviewer"],"scripts.connected_identity_contract")
if __name__=="__main__":unittest.main()
