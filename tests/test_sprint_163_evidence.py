import copy,unittest
from unittest.mock import patch
from scripts import sprint_163_evidence as e
from scripts.sprint_evidence_recorder import build_report,validate_report
def cmds():return[{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in e.FOCUSED_COMMANDS else None}for i,a in e.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(e.DEFINITION,"c"*40,cmds(),environment={})
class T(unittest.TestCase):
 def val(self,v):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(e.DEFINITION,v,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.val(report()),[])
 def test_external_overclaims_fail(self):
  for f in ("signed_catalog_complete","manifest_reconciliation_complete","decision_0027_reconciliation_complete","native_accessibility_removal_complete","rv40_41_46_complete","sprint_gate_closed"):
   v=copy.deepcopy(report());v["verification_evidence"][f]=True;self.assertTrue(self.val(v))
 def test_promotion_fails(self):v=copy.deepcopy(report());v["summary"]["approved_model_count"]=1;self.assertTrue(self.val(v))
if __name__=="__main__":unittest.main()
