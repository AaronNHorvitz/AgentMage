import json,unittest
from scripts.productivity_identity_graph_contract import ATTACKS,KINDS,STATES,build
class ProductivityIdentityGraphContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_dimensions(self):self.assertEqual(tuple(self.value["identity_kinds"]),KINDS);self.assertEqual(tuple(self.value["link_states"]),STATES);self.assertEqual(tuple(self.value["attack_families"]),ATTACKS);self.assertEqual(self.value["case_count"],936)
 def test_non_authoritative_links_deny(self):self.assertTrue(all(not c["authorized"]for c in self.value["cases"]if c["link_state"]in {"proposed","conflicting","stale","removed"}))
 def test_lineage_and_truth_are_visible(self):self.assertTrue(all(c["lineage_preserved"]and c["native_identity_visible"]and c["source_visible"]and c["freshness_visible"]and c["classification_visible"]for c in self.value["cases"]))
 def test_display_alias_stale_never_authorize(self):self.assertEqual([self.value[k]for k in ("display_authorization_count","alias_authorization_count","stale_authorization_count")],[0,0,0])
if __name__=="__main__":unittest.main()
