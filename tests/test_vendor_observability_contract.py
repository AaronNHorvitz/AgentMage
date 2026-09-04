import json,unittest
from scripts.vendor_observability_contract import ATTACKS,FAULTS,OBJECTS,PROVIDERS,build
class VendorObservabilityContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_queries(self):self.assertEqual(tuple(self.value["providers"]),PROVIDERS);self.assertEqual(tuple(self.value["objects"]),OBJECTS);self.assertTrue(all(c["tenant_bound"]and c["cost_bound"]and c["namespaced_extensions"]for c in self.value["query_cases"]))
 def test_attacks(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["tenant_crossover_count"]and not c["credential_disclosure_count"]and not c["hidden_write_count"]and not c["remediation_count"]for c in self.value["attack_cases"]))
 def test_faults(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["provider_difference_visible"]and c["cancellation_effective"]and not c["bounded_resource_count"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_provider_count"],0)
if __name__=="__main__":unittest.main()
