import copy,unittest
from unittest.mock import patch
from scripts import sprint_75_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands(): return [{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None} for i,a in evidence.COMMANDS]
def report():
    with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"): return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint75EvidenceTests(unittest.TestCase):
    def validate(self,value):
        with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"): return validate_report(evidence.DEFINITION,value,verify_ancestry=False)
    def test_valid(self): self.assertEqual(self.validate(report()),[])
    def test_overclaims(self):
        for field in ("upstream_sprints_closed","native_connector_acceptance_complete","strict_local_native_regression_complete","credential_cache_cleanup_complete","native_security_evidence_complete","signed_gate_decision_present","release_approval"):
            value=copy.deepcopy(report()); value["summary"][field]=True; self.assertTrue(self.validate(value))
    def test_effect_drift(self):
        for field in ("hosted_mutation_operation_count","network_request_count"):
            value=copy.deepcopy(report()); value["verification_evidence"][field]=1; self.assertTrue(self.validate(value))
if __name__=="__main__": unittest.main()
