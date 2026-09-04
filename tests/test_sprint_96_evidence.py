import copy,unittest
from unittest.mock import patch
from scripts import sprint_96_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return [{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None} for i,a in evidence.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint96EvidenceTests(unittest.TestCase):
 def validate(self,v):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,v,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.validate(report()),[])
 def test_external_overclaims(self):
  for field in ("upstream_sprint_95_closed","signed_update_complete","vulnerability_review_complete","independent_review_present","release_approval"):
   v=copy.deepcopy(report());v["summary"][field]=True;self.assertTrue(self.validate(v))
 def test_observation_overclaims(self):
  for field in ("current_vulnerability_feed_present","native_update_observed","independent_review","sprint_gate_closed"):
   v=copy.deepcopy(report());v["verification_evidence"][field]=True;self.assertTrue(self.validate(v))
if __name__=="__main__":unittest.main()
