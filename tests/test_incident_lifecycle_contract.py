import json,unittest
from scripts.incident_lifecycle_contract import ATTACKS,CLASSES,EFFECTS,FAULTS,PROVIDERS,build
class IncidentLifecycleContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_records(self):self.assertEqual(tuple(self.value["providers"]),PROVIDERS);self.assertEqual(tuple(self.value["effects"]),EFFECTS);self.assertEqual(tuple(self.value["evidence_classes"]),CLASSES);self.assertTrue(all(c["source_bound"]and c["fact_inference_separate"]for c in self.value["record_cases"]))
 def test_attacks(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["hidden_recipient_count"]and not c["cross_tenant_count"]and not c["autonomous_effect_count"]for c in self.value["attack_cases"]))
 def test_faults(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["uncertain_reconciled"]and c["correction_is_fresh_effect"]and not c["duplicate_effect_count"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_provider_count"],0)
if __name__=="__main__":unittest.main()
