import unittest
from scripts.sprint_168_evidence import BLOCKERS,IMPLEMENTED,SUMMARY,VERIFICATION
class T(unittest.TestCase):
 def test_matrix(self):self.assertEqual(IMPLEMENTED["fixture_case_count"],2080);self.assertEqual(sum(IMPLEMENTED[k]for k in ("ordinary_activation_count","direct_promotion_count","approved_state_change_count","canonical_damage_count")),0)
 def test_truth(self):self.assertFalse(SUMMARY["experimental_model_promoted"]);self.assertEqual(IMPLEMENTED["signed_support_matrix_count"],0);self.assertFalse(VERIFICATION["sprint_gate_closed"])
 def test_blockers(self):self.assertEqual([x["code"]for x in BLOCKERS],["UPSTREAM-SPRINTS-163-164-167-BLOCKED","BLOCKED_EXTERNAL"])
if __name__=="__main__":unittest.main()
