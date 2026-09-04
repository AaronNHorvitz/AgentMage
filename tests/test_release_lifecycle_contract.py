import json,unittest
from scripts.release_lifecycle_contract import ATTACKS,CHANGES,FAULTS,TOOLS,build
class ReleaseLifecycleContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_manifests(self):self.assertEqual(tuple(self.value["tools"]),TOOLS);self.assertEqual(tuple(self.value["changes"]),CHANGES);self.assertTrue(all(c["source_bound"]and c["rollback_bound"]for c in self.value["manifest_cases"]))
 def test_attacks(self):self.assertEqual(tuple(self.value["attacks"]),ATTACKS);self.assertTrue(all(not c["provider_contact_count"]and not c["authority_escape_count"]and not c["automatic_advance_count"]for c in self.value["attack_cases"]))
 def test_faults(self):self.assertEqual(tuple(self.value["faults"]),FAULTS);self.assertTrue(all(c["later_state_preserved"]and not c["automatic_retry_count"]and c["fresh_compensation_required"]for c in self.value["fault_cases"]));self.assertEqual(self.value["promoted_tool_count"],0)
if __name__=="__main__":unittest.main()
