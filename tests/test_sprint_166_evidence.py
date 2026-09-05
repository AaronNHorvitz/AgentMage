import unittest
from scripts.sprint_166_evidence import BLOCKERS,IMPLEMENTED,SUMMARY,VERIFICATION
class T(unittest.TestCase):
 def test_matrix(self):self.assertEqual(IMPLEMENTED["fixture_case_count"],180);self.assertEqual(sum(IMPLEMENTED[k]for k in ("false_publication_count","false_ga_closure_count","substitution_count")),0)
 def test_truth(self):self.assertEqual(SUMMARY["release_decision"],"BLOCKED");self.assertEqual(SUMMARY["release_approval_count"],0);self.assertFalse(VERIFICATION["sprint_gate_closed"])
 def test_blockers(self):self.assertEqual([x["code"]for x in BLOCKERS],["UPSTREAM-SPRINTS-0-165-BLOCKED","BLOCKED_EXTERNAL"])
if __name__=="__main__":unittest.main()
