import json,unittest
from scripts.telemetry_correlation_contract import ATTACKS,FAULTS,IDENTITIES,build
class TelemetryCorrelationContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_queries_and_correlations(self):self.assertEqual(tuple(self.value["identities"]),IDENTITIES);self.assertTrue(all(c["range_bound"]and c["source_cited"]for c in self.value["query_cases"]));self.assertTrue(all(c["inference_labeled"]and not c["causation_claim_count"]for c in self.value["correlation_cases"]))
 def test_attacks(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["sensitive_disclosure_count"]and not c["operation_authority_count"]and not c["durable_memory_count"]for c in self.value["attack_cases"]))
 def test_faults(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["uncertainty_preserved"]and not c["unbounded_resource_count"]and not c["automatic_effect_count"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_backend_count"],0)
if __name__=="__main__":unittest.main()
