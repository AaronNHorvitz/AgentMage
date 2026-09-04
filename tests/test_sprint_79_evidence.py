import copy,unittest
from unittest.mock import patch
from scripts import sprint_79_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands():return [{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None} for i,a in evidence.COMMANDS]
def report():
    with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint79EvidenceTests(unittest.TestCase):
    def validate(self,value):
        with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"):return validate_report(evidence.DEFINITION,value,verify_ancestry=False)
    def test_valid(self):self.assertEqual(self.validate(report()),[])
    def test_overclaims(self):
        for field in ("upstream_sprint_78_closed","native_package_recovery_complete","native_security_evidence_complete","independent_review_present","release_approval"):
            value=copy.deepcopy(report());value["summary"][field]=True;self.assertTrue(self.validate(value))
    def test_runtime_drift(self):
        for field,value in (("installed_package_count",1),("enabled_package_count",1),("safe_mode_startup_observed",True)):
            report_value=copy.deepcopy(report());report_value["verification_evidence"][field]=value;self.assertTrue(self.validate(report_value))
if __name__=="__main__":unittest.main()
