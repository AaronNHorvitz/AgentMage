import copy,unittest
from unittest.mock import patch
from scripts import sprint_98_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return [{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None} for i,a in evidence.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint98EvidenceTests(unittest.TestCase):
 def validate(self,v):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,v,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.validate(report()),[])
 def test_external_overclaims(self):
  for field in ("upstream_sprint_97_closed","native_maintenance_recovery_complete","cross_platform_recovery_complete","independent_review_present","release_approval"):
   v=copy.deepcopy(report());v["summary"][field]=True;self.assertTrue(self.validate(v))
 def test_observation_overclaims(self):
  for field in ("optional_capability_count_in_safe_mode","raw_diagnostic_content_count","secret_diagnostic_count","network_count","native_recovery_execution_count"):
   v=copy.deepcopy(report());v["verification_evidence"][field]=1;self.assertTrue(self.validate(v))
if __name__=="__main__":unittest.main()
