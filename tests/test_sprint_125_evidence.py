import copy,unittest
from unittest.mock import patch
from scripts import sprint_125_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return[{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None}for i,a in evidence.COMMANDS]
def report():
 with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint125EvidenceTests(unittest.TestCase):
 def validate(self,value):
  with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,value,verify_ancestry=False)
 def test_valid(self):self.assertEqual(self.validate(report()),[])
 def test_overclaims(self):
  for field in ("native_provider_lifecycle_complete","native_platform_parity_complete","release_candidate_complete","all_profile_lifecycle_complete","rv01_30_complete","rv49_complete","at_xte_001_complete","at_win_001_complete","g_delivery_closed","g_windows_closed","independent_decisions_complete","sprint_gate_closed"):
   value=copy.deepcopy(report());value["verification_evidence"][field]=True;self.assertTrue(self.validate(value))
 def test_external(self):
  for field in ("upstream_sprint_124_closed","delivery_gate_closed","windows_gate_closed","release_approval"):
   value=copy.deepcopy(report());value["summary"][field]=True;self.assertTrue(self.validate(value))
if __name__=="__main__":unittest.main()
