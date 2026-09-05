import json,unittest
from scripts.local_continuity_contract import CHANGES,FAILURES,RECORDS,ROOTS,STAGES,TRANSITIONS,build
class T(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_counts(self):self.assertEqual((self.v["backup_case_count"],self.v["checkpoint_case_count"]),(936,720))
 def test_coverage(self):self.assertEqual((len(ROOTS),len(FAILURES),len(STAGES),len(TRANSITIONS),len(CHANGES),len(RECORDS)),(9,13,8,6,10,12))
 def test_zero(self):self.assertTrue(all(x["fail_closed"]for x in self.v["backup_cases"]));self.assertTrue(all(x["deterministic"]and x["replacement_scheduled"]and x["broader_rescan_on_uncertainty"]for x in self.v["checkpoint_cases"]));self.assertEqual(sum(self.v[k]for k in ("plaintext_count","raw_credential_count","false_complete_count","canonical_preconfirm_mutation_count","stale_current_count")),0)
if __name__=="__main__":unittest.main()
