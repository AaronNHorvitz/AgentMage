import json,unittest
from scripts.delivery_foundation_contract import FIXTURES,LEVELS,MODES,NODES,OPERATIONS,RELATIONS,build
class DeliveryFoundationContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_graph_contract(self):self.assertEqual(tuple(self.value["node_kinds"]),NODES);self.assertEqual(tuple(self.value["relationship_states"]),RELATIONS);self.assertEqual(self.value["graph_case_count"],len(NODES)*len(FIXTURES))
 def test_adapter_sdk_and_noninheritance(self):self.assertEqual(tuple(self.value["adapter_operations"]),OPERATIONS);self.assertEqual(tuple(self.value["conformance_levels"]),LEVELS);self.assertEqual(tuple(self.value["adapter_modes"]),MODES);self.assertEqual(self.value["matrix_case_count"],len(LEVELS)*len(OPERATIONS));self.assertTrue(all(not c["inherited"] for c in self.value["conformance_matrix"]))
 def test_authority_and_support_overclaims_absent(self):self.assertTrue(all(not c["authority_count"] for c in self.value["graph_cases"]));self.assertEqual(self.value["live_provider_count"],0);self.assertEqual(self.value["enabled_adapter_count"],0);self.assertEqual(self.value["reviewer"],"scripts.delivery_foundation_contract")
if __name__=="__main__":unittest.main()
