import json,unittest
from scripts.pod_composition_contract import CLASSES,build
class PodCompositionContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_attack_and_recovery_classes(self):self.assertEqual(self.value["case_count"],60);self.assertEqual(set(self.value["classes"]),set(CLASSES))
 def test_bounds_and_single_agent_path(self):self.assertEqual(self.value["max_workers"],5);self.assertTrue(self.value["single_agent_mode_preserved"]);self.assertTrue(self.value["serialized_integration"])
 def test_no_enablement_publication_or_pooling(self):self.assertEqual(self.value["enabled_pod_count"],0);self.assertEqual(self.value["native_worker_count"],0);self.assertTrue(all(not c["enabled"] and c["publication_count"]==0 and c["authority_pool_count"]==0 for c in self.value["cases"]))
if __name__=="__main__":unittest.main()
