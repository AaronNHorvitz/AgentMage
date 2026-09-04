import json,unittest
from scripts.productivity_inbox_contract import FAULTS,SOURCES,STATES,build
class ProductivityInboxContractTests(unittest.TestCase):
 def setUp(self):self.v=json.loads(build())
 def test_dimensions(self):self.assertEqual(tuple(self.v["states"]),STATES);self.assertEqual(tuple(self.v["sources"]),SOURCES);self.assertEqual(tuple(self.v["faults"]),FAULTS);self.assertEqual(self.v["case_count"],1014)
 def test_gaps_never_appear_complete(self):self.assertTrue(all(not c["presented_complete"]for c in self.v["cases"]if c["coverage_gap_visible"]))
 def test_source_truth_preserved(self):self.assertTrue(all(c["native_identity_preserved"]and c["citation_visible"]and c["freshness_visible"]and c["classification_visible"]for c in self.v["cases"]))
 def test_no_provider_or_model_authority(self):self.assertEqual(self.v["provider_write_count"]+self.v["model_authority_count"],0)
 def test_accessibility_matrix(self):self.assertTrue(all(c["keyboard_pass"]and c["screen_reader_pass"]and c["focus_pass"]and c["dynamic_announcement_pass"]for c in self.v["cases"]))
if __name__=="__main__":unittest.main()
