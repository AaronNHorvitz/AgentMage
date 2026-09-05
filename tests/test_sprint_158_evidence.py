import copy,unittest
from unittest.mock import patch
from scripts import sprint_158_evidence as e
from scripts.sprint_evidence_recorder import build_report,validate_report
def cmds():return[{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in e.FOCUSED_COMMANDS else None}for i,a in e.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(e.DEFINITION,"c"*40,cmds(),environment={})
class T(unittest.TestCase):
 def val(self,value):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(e.DEFINITION,value,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.val(report()),[])
 def test_native_overclaims_fail(self):
  for field in ("native_platform_campaign_complete","rv38_complete","sprint_gate_closed"):
   value=copy.deepcopy(report());value["verification_evidence"][field]=True;self.assertTrue(self.val(value))
 def test_promotion_fails(self):value=copy.deepcopy(report());value["summary"]["support_promotion_count"]=1;self.assertTrue(self.val(value))
if __name__=="__main__":unittest.main()
