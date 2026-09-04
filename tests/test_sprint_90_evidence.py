import copy,unittest
from unittest.mock import patch
from scripts import sprint_90_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return [{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None} for i,a in evidence.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint90EvidenceTests(unittest.TestCase):
 def validate(self,value):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,value,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.validate(report()),[])
 def test_external_overclaims(self):
  for field in ("upstream_sprint_89_closed","native_read_only_schedule_complete","native_notification_complete","multi_runner_recovery_complete","independent_review_present","release_approval"):
   value=copy.deepcopy(report());value["summary"][field]=True;self.assertTrue(self.validate(value))
 def test_runtime_overclaims(self):
  for field in ("native_schedule_executed","local_notification_delivered","multi_runner_restart_executed","post_run_receipt_captured","independent_review"):
   value=copy.deepcopy(report());value["verification_evidence"][field]=True;self.assertTrue(self.validate(value))
if __name__=="__main__":unittest.main()
