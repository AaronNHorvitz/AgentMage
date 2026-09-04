import copy,json,unittest
from scripts.child_authority_contract import CLASSES,build
class ChildAuthorityContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_closed_class_and_case_counts(self):
  self.assertEqual(self.value["class_count"],24);self.assertEqual(self.value["case_count"],96);self.assertEqual(set(self.value["classes"]),set(CLASSES))
 def test_every_case_is_inert_and_results_mint_nothing(self):
  self.assertTrue(all(case["real_effect_count"]==0 and case["result_authority_count"]==0 for case in self.value["cases"]))
 def test_native_and_review_overclaims_are_absent(self):
  self.assertEqual(self.value["native_process_trace_count"],0);self.assertEqual(self.value["native_worktree_trace_count"],0);self.assertFalse(self.value["independent_review"])
 def test_mutated_case_identity_is_detectable(self):
  value=copy.deepcopy(self.value);value["cases"][1]["case_id"]=value["cases"][0]["case_id"];self.assertNotEqual(len({case["case_id"] for case in value["cases"]}),96)
if __name__=="__main__":unittest.main()
