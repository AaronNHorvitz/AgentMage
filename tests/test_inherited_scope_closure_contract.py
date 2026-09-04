import json,unittest
from scripts.inherited_scope_closure_contract import EXPECTED,build
class InheritedScopeClosureContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_requirement_and_inventory_closure(self):self.assertEqual(self.value["registry_requirement_count"],294);self.assertEqual(self.value["unique_requirement_count"],294);self.assertEqual(self.value["planning_mapped_requirement_count"],294);self.assertEqual(self.value["category_counts"],EXPECTED)
 def test_every_defer_is_an_inert_tested_exclusion(self):self.assertEqual(self.value["deferred_exclusion_count"],30);self.assertTrue(all(x["explicit_exclusion"]and x["negative_registration_test"]and not x["authority_added"]and not x["promoted"]for x in self.value["deferred_exclusions"]))
 def test_unmet_release_evidence_cannot_close_checkpoint(self):self.assertFalse(self.value["promoted_mapping_gate_closed"]);self.assertFalse(self.value["inherited_release_approved"]);self.assertGreater(self.value["missing_required_evidence_count"],0)
if __name__=="__main__":unittest.main()
