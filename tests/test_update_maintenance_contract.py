import copy,json,unittest
from scripts.update_maintenance_contract import CLASSES,build
class UpdateMaintenanceContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_complete_dependency_license_hash_and_removal_closure(self):
  i=self.value["dependency_inventory"];self.assertGreater(i["component_count"],0);self.assertEqual(i["component_count"],i["sbom_component_count"]);self.assertEqual(i["component_count"],i["licensed_component_count"]);self.assertEqual(i["component_count"],i["hashed_component_count"]);self.assertEqual(i["component_count"],self.value["removal_plan"]["component_count"])
 def test_attack_corpus_is_closed(self):self.assertEqual(self.value["case_count"],72);self.assertEqual({c["class"] for c in self.value["cases"]},set(CLASSES));self.assertTrue(all(not c["activation_count"] and not c["remote_check_count"] and not c["suppression_count"] for c in self.value["cases"]))
 def test_update_contract_is_offline_and_nonactivating(self):
  u=self.value["update"];self.assertFalse(u["automatic_remote_check"]);self.assertTrue(u["preview_contract_present"] and u["compatibility_contract_present"] and u["integrity_contract_present"]);self.assertEqual(u["signed_package_count"],0);self.assertEqual(u["staged_activation_count"],0);self.assertEqual(u["rollback_execution_count"],0)
 def test_vulnerability_review_absence_remains_visible(self):self.assertEqual(self.value["vulnerability_review"],{"status":"BLOCKED_EXTERNAL","current_feed_present":False,"reviewed_component_count":0,"suppression_count":0})
if __name__=="__main__":unittest.main()
