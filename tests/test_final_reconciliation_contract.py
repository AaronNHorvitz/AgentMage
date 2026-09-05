import json,unittest
from scripts.final_reconciliation_contract import ATTACKS,CAPABILITIES,FIXTURES,PROFILES,RISKS,STATES,VARIATIONS,build
class T(unittest.TestCase):
 def setUp(self):self.value=json.loads(build())
 def test_dimensions(self):self.assertEqual((len(CAPABILITIES),len(STATES),len(ATTACKS),len(VARIATIONS),len(FIXTURES),len(RISKS),len(PROFILES)),(10,8,13,10,11,8,3))
 def test_scale(self):self.assertEqual((self.value["composed_case_count"],self.value["audit_case_count"]),(10400,264))
 def test_composed_truth(self):self.assertTrue(all(x["raw_reconciled"]for x in self.value["composed_cases"]));self.assertEqual(sum(x["unauthorized_effect_count"]for x in self.value["composed_cases"]),0)
 def test_audit_truth(self):self.assertTrue(all(x["relationships_complete"]and x["contradictions_retained"]and x["finding_current"]for x in self.value["audit_cases"]));self.assertEqual(sum(x["false_comprehensive_count"]for x in self.value["audit_cases"]),0)
 def test_candidate_truth(self):self.assertEqual(self.value["muse_disposition"],"BLOCKED");self.assertEqual(self.value["approved_model_count"],0)
if __name__=="__main__":unittest.main()
