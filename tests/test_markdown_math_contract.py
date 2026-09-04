import json,unittest
from scripts.markdown_math_contract import ATTACKS,FAULTS,SYNTAX,build
class MarkdownMathContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_round_trips(self):self.assertEqual(tuple(self.value["syntax"]),SYNTAX);self.assertTrue(all(c["source_span_exact"]and c["round_trip_exact"]and c["code_fence_inert"]for c in self.value["round_trip_cases"]))
 def test_attacks(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["file_access_count"]and not c["network_access_count"]and not c["process_count"]and not c["authority_grant_count"]for c in self.value["attack_cases"]))
 def test_faults(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["cleanup_complete"]and not c["stale_preview_count"]and c["cache_identity_bound"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_platform_count"],0)
if __name__=="__main__":unittest.main()
