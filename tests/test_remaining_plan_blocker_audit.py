import json,unittest
from scripts.remaining_plan_blocker_audit import build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_every_row_blocked(self):self.assertEqual(self.value["unchecked_row_count"],self.value["blocked_row_count"]);self.assertEqual(self.value["unmapped_row_count"],0)
 def test_empty_substitutions(self):self.assertEqual(self.value["nonempty_substitution_count"],0);self.assertTrue(all(not row["substitution_set"]for row in self.value["rows"]))
 def test_exact_sources(self):self.assertTrue(all(self.value["blockers"][key]for row in self.value["rows"]for key in row["blocker_ids"]))
if __name__=="__main__":unittest.main()
