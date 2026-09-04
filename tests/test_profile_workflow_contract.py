import copy,json,unittest
from scripts.profile_workflow_contract import CASES,GRAPHS,build
class ProfileWorkflowContractTests(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_exact_graphs_and_profiles(self):
  self.assertEqual(self.value["workflow_count"],4);self.assertEqual({g["workflow_id"]:g["profiles"] for g in self.value["graphs"]},GRAPHS)
 def test_shared_runtime_parent_review_and_zero_effects(self):
  self.assertTrue(all(g["shared_runtime"] and g["parent_review_required"] and g["provider_effect_count"]==0 for g in self.value["graphs"]));self.assertTrue(all(c["real_effect_count"]==0 and c["approval_count"]==0 for c in self.value["cases"]))
 def test_adversarial_corpus_is_complete(self):
  self.assertEqual(self.value["case_count"],80);self.assertEqual({c["class"] for c in self.value["cases"]},set(CASES));self.assertTrue(self.value["dissent_preserved"])
 def test_native_and_independent_claims_remain_absent(self):
  self.assertEqual(self.value["native_descendant_cleanup_count"],0);self.assertEqual(self.value["native_worktree_pipeline_count"],0);self.assertFalse(self.value["independent_review"])
if __name__=="__main__":unittest.main()
