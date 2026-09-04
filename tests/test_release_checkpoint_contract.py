import json,unittest
from scripts.release_checkpoint_contract import BLOCKERS,DOCUMENTS,STATES,build
class ReleaseCheckpointContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_dimensions(self):self.assertEqual(tuple(self.value["states"]),STATES);self.assertEqual(tuple(self.value["blockers"]),BLOCKERS);self.assertEqual(tuple(d["path"]for d in self.value["documents"]),DOCUMENTS)
 def test_every_blocker_prevents_release(self):self.assertTrue(all(c["visible"]and not c["g_ga_closed"]and not c["package_produced"]and not c["support_promoted"]for c in self.value["claim_cases"]))
 def test_release_claims_are_zero(self):self.assertEqual(self.value["native_reproduction_count"],0);self.assertEqual(self.value["reviewer_signature_count"],0);self.assertEqual(self.value["release_package_count"],0);self.assertEqual(self.value["promotion_count"],0)
if __name__=="__main__":unittest.main()
