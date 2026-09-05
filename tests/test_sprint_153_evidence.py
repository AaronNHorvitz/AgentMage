import copy,unittest
from unittest.mock import patch
from scripts import sprint_153_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return[{"id":identity,"argv":list(argv),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if identity in evidence.FOCUSED_COMMANDS else None}for identity,argv in evidence.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class T(unittest.TestCase):
 def validate(self,value):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,value,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.validate(report()),[])
 def test_external_claims_fail(self):
  for field in ("native_provider_campaigns_complete","sprint_gate_closed"):
   value=copy.deepcopy(report());value["verification_evidence"][field]=True;self.assertTrue(self.validate(value))
 def test_promotion_fails(self):value=copy.deepcopy(report());value["summary"]["support_promotion_count"]=1;self.assertTrue(self.validate(value))
if __name__=="__main__":unittest.main()
