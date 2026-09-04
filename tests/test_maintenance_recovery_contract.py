import json,unittest
from scripts.maintenance_recovery_contract import DISABLED,FAILURES,SUBSYSTEMS,build
class MaintenanceRecoveryContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_safe_mode_closure(self):self.assertEqual(tuple(self.value["safe_mode_disabled"]),DISABLED);self.assertEqual(self.value["safe_mode_disabled_count"],8)
 def test_failure_and_diagnostic_closure(self):self.assertEqual(set(self.value["failure_classes"]),set(FAILURES));self.assertEqual(set(self.value["diagnostic_subsystems"]),set(SUBSYSTEMS));self.assertEqual(self.value["case_count"],72)
 def test_no_effect_content_secret_network_or_review_overclaim(self):self.assertTrue(all(not c["optional_authority_count"] and not c["raw_content_count"] and not c["secret_count"] and not c["network_count"] for c in self.value["cases"]));self.assertEqual(self.value["native_recovery_execution_count"],0);self.assertFalse(self.value["independent_review"])
if __name__=="__main__":unittest.main()
