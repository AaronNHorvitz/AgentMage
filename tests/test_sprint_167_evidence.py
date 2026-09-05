import unittest
from scripts.sprint_167_evidence import BLOCKERS,IMPLEMENTED,VERIFICATION
class T(unittest.TestCase):
 def test_matrix(self):self.assertEqual(IMPLEMENTED["fixture_case_count"],6912);self.assertEqual(sum(IMPLEMENTED[k]for k in ("authority_count","network_count","canonical_write_count","approved_store_count","hidden_limitation_count")),0)
 def test_truth(self):self.assertTrue(VERIFICATION["separate_package_complete"]);self.assertFalse(VERIFICATION["sprint_gate_closed"])
 def test_blockers(self):self.assertEqual([x["code"]for x in BLOCKERS],["UPSTREAM-SPRINTS-157-163-164-166-BLOCKED","BLOCKED_EXTERNAL"])
if __name__=="__main__":unittest.main()
