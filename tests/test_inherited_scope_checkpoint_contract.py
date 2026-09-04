import json,unittest
from scripts.inherited_scope_checkpoint_contract import BLOCKERS,DOCUMENTS,build
class InheritedScopeCheckpointContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_complete_local_inventory(self):self.assertEqual(self.value["requirement_count"],294);self.assertEqual(self.value["completion_record_count"],112);self.assertEqual(tuple(x["path"]for x in self.value["documents"]),DOCUMENTS)
 def test_every_failure_class_blocks_both_release_gates(self):self.assertEqual({x["blocker"]for x in self.value["blocker_cases"]},set(BLOCKERS));self.assertTrue(all(x["visible"]and not x["legacy_g_product_closed"]and not x["g_ga_closed"]and not x["release_authorized"]for x in self.value["blocker_cases"]))
 def test_checkpoint_is_truthfully_non_ga(self):self.assertTrue(self.value["decision_0008_non_ga_checkpoint"]);self.assertEqual(self.value["supported_platform_count"],0);self.assertEqual(self.value["enabled_model_count"],0);self.assertFalse(self.value["legacy_g_product_closed"]);self.assertFalse(self.value["g_ga_closed"])
if __name__=="__main__":unittest.main()
