import unittest
from scripts.sprint_164_evidence import BLOCKERS,IMPLEMENTED,VERIFICATION
class T(unittest.TestCase):
 def test_local_contracts(self):self.assertTrue(VERIFICATION["focused_local_contracts"]);self.assertEqual(VERIFICATION["focused_blocking_skip_count"],0)
 def test_corpus(self):self.assertEqual(IMPLEMENTED["fixture_case_count"],3120);self.assertEqual(sum(IMPLEMENTED[k]for k in ("silent_activation_count","partial_activation_count","substituted_profile_count","unconfirmed_effect_count")),0)
 def test_truth(self):self.assertEqual(IMPLEMENTED["approved_profile_execution_count"],0);self.assertFalse(VERIFICATION["sprint_gate_closed"])
 def test_blockers(self):self.assertEqual([x["code"] for x in BLOCKERS],["UPSTREAM-SPRINTS-14-15-23-157-158-163-BLOCKED","BLOCKED_EXTERNAL"])
if __name__=="__main__":unittest.main()
