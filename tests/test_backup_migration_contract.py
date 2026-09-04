import json,unittest
from scripts.backup_migration_contract import CLASSES,DOMAINS,build
class BackupMigrationContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_exact_complete_domains(self):self.assertEqual(tuple(self.value["domains"]),DOMAINS);self.assertEqual(self.value["domain_count"],8)
 def test_attack_and_recovery_closure(self):self.assertEqual(self.value["case_count"],64);self.assertEqual({c["class"] for c in self.value["cases"]},set(CLASSES));self.assertTrue(all(not c["secret_export_count"] and not c["live_replacement_count"] and not c["network_count"] for c in self.value["cases"]))
 def test_native_claims_absent(self):self.assertEqual(self.value["native_cross_machine_migration_count"],0);self.assertEqual(self.value["platforms_verified"],[]);self.assertEqual(self.value["secret_export_count"],0)
if __name__=="__main__":unittest.main()
