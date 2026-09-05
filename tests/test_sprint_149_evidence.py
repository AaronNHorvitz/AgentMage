import copy,unittest
from unittest.mock import patch
from scripts import sprint_149_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return[{"id":item,"argv":list(argv),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if item in evidence.FOCUSED_COMMANDS else None}for item,argv in evidence.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint149EvidenceTests(unittest.TestCase):
 def validate(self,value):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,value,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.validate(report()),[])
 def test_external_claims_cannot_be_invented(self):
  for field in ("native_labeled_evaluation_complete","sprint_gate_closed"):
   value=copy.deepcopy(report());value["verification_evidence"][field]=True;self.assertTrue(self.validate(value))
 def test_upstream_or_promotion_cannot_close(self):
  value=copy.deepcopy(report());value["summary"]["upstream_dependencies_closed"]=True;self.assertTrue(self.validate(value));value=copy.deepcopy(report());value["summary"]["support_promotion_count"]=1;self.assertTrue(self.validate(value))
if __name__=="__main__":unittest.main()
