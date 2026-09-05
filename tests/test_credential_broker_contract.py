import json,unittest
from scripts.credential_broker_contract import ATTACKS,FLOWS,LIFECYCLES,STORES,SURFACES,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_matrix(self):self.assertEqual(self.value["case_count"],6912);self.assertEqual(len(self.value["cases"]),6912)
 def test_coverage(self):self.assertEqual((len(STORES),len(FLOWS),len(ATTACKS),len(LIFECYCLES),len(SURFACES)),(4,6,12,8,13))
 def test_zero_disclosure_and_residue(self):self.assertTrue(all(item["fail_closed"]and item["worker_memory_cleared"] for item in self.value["cases"]));self.assertEqual(sum(self.value[k] for k in ("disclosure_count","wrong_account_count","stale_reference_count","canary_finding_count","restored_raw_credential_count","restored_usable_credential_count")),0);self.assertTrue(self.value["reauthentication_required"])
if __name__=="__main__":unittest.main()
