import copy,unittest
from unittest.mock import patch
from scripts import sprint_131_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return[{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None}for i,a in evidence.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint131EvidenceTests(unittest.TestCase):
 def validate(self,v):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,v,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.validate(report()),[])
 def test_external_claims_cannot_be_invented(self):
  for f in ("native_connected_campaign_complete","sprint_gate_closed"):
   v=copy.deepcopy(report());v["verification_evidence"][f]=True;self.assertTrue(self.validate(v))
 def test_upstream_or_promotion_cannot_close(self):
  v=copy.deepcopy(report());v["summary"]["upstream_sprint_130_closed"]=True;self.assertTrue(self.validate(v));v=copy.deepcopy(report());v["summary"]["support_promotion_count"]=1;self.assertTrue(self.validate(v))
if __name__=="__main__":unittest.main()
