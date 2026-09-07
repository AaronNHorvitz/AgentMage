import json,unittest
from scripts.expanded_release_checkpoint_contract import BLOCKERS,DOCUMENTS,DOMAINS,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_inventory(self):self.assertEqual(self.value["local_evidence_report_count"],154);self.assertEqual(len(self.value["documents"]),len(DOCUMENTS));self.assertEqual(tuple(self.value["blockers"]),BLOCKERS);self.assertEqual(tuple(self.value["domains"]),DOMAINS)
 def test_every_blocker_prevents_release(self):self.assertTrue(all(case["visible"]and not case["checkpoint_closed"]and not case["g_ga_closed"]and not case["package_produced"]and not case["support_promoted"] for case in self.value["cases"]))
 def test_release_claims_zero(self):self.assertEqual(sum(self.value[key] for key in ("signed_manifest_count","native_reproduction_count","reviewer_signature_count","user_approval_count","release_package_count","checkpoint_closure_count","promotion_count")),0)
if __name__=="__main__":unittest.main()
