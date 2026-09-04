import json,unittest
from scripts.productivity_sync_contract import FAULTS,TRANSITIONS,build
class ProductivitySyncContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.v["faults"]),FAULTS);self.assertEqual(tuple(self.v["transitions"]),TRANSITIONS);self.assertEqual(self.v["case_count"],1120)
 def test_incomplete_is_visible(self):self.assertTrue(all(c["gap_visible"]for c in self.v["cases"]if not c["coverage_complete"]))
 def test_events_create_no_authority(self):self.assertEqual(sum(self.v[k]for k in ("event_created_grant_count","event_created_schedule_count","event_created_workflow_count","event_created_effect_count")),0)
 def test_removal_clears_authority(self):self.assertTrue(all(c["remaining_authority_count"]==0 for c in self.v["cases"]if c["transition"]=="remove"))
if __name__=="__main__":unittest.main()
