"""Mutation tests for truthful Sprint 70 local evidence."""
import copy,unittest
from unittest.mock import patch
from scripts import sprint_70_evidence as evidence
from scripts.sprint_evidence_recorder import build_report,validate_report
def commands(): return [{"id":i,"argv":list(a),"exit_code":0,"output_sha256":"a"*64,"blocking_skip_count":0 if i in evidence.FOCUSED_COMMANDS else None} for i,a in evidence.COMMANDS]
def report():
    with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"): return build_report(evidence.DEFINITION,"c"*40,commands(),environment={})
class Sprint70EvidenceTests(unittest.TestCase):
    def validate(self,value):
        with patch("scripts.sprint_evidence_recorder.git_file",return_value=b"source"): return validate_report(evidence.DEFINITION,value,verify_ancestry=False)
    def test_valid_blocked(self):
        value=report(); self.assertEqual(self.validate(value),[]); self.assertEqual(value["summary"]["sprint_status"],"BLOCKED")
    def test_overclaims_fail(self):
        for field in ("upstream_sprint_69_closed","actual_connector_read_complete","credential_derivation_complete","encrypted_cache_lifecycle_complete","native_network_evidence_complete","independent_review_present","release_approval"):
            value=copy.deepcopy(report()); value["summary"][field]=True; self.assertTrue(self.validate(value),field)
    def test_authority_drift_fails(self):
        for mutate in (lambda v:v["commands"].pop(),lambda v:v["source_sha256"].pop(next(iter(v["source_sha256"]))),lambda v:v["blockers"].pop(),lambda v:v["verification_evidence"].update({"network_client_count":1}),lambda v:v["verification_evidence"].update({"credential_material_count":1}),lambda v:v["verification_evidence"].update({"accepted_network_effect_count":1})):
            value=copy.deepcopy(report()); mutate(value); self.assertTrue(self.validate(value))
if __name__=="__main__": unittest.main()
