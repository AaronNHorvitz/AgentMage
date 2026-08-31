from __future__ import annotations
import copy, unittest
from scripts.story_13_6_gateway_routing_evidence import MARKERS, TRUTH, expected_report, validate_raw, validate_report, validate_sources
class Story136GatewayRoutingEvidenceTests(unittest.TestCase):
    def test_sources_and_expected_report_are_current(self) -> None: self.assertEqual(validate_sources(), []); self.assertEqual(validate_report(expected_report()), [])
    def test_every_execution_marker_is_required(self) -> None:
        valid="\n".join(MARKERS); self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS: self.assertTrue(validate_raw(valid.replace(marker,"")),marker)
    def test_silent_fallback_live_execution_and_release_overclaims_are_rejected(self) -> None:
        for field in ("fallback_enabled_by_default","live_remote_endpoint_executed","credential_value_accessed"):
            changed=copy.deepcopy(expected_report()); changed["product_truth"][field]=not TRUTH[field]; self.assertTrue(validate_report(changed),field)
        for field in ("silent_local_to_remote_transition_count","silent_cross_remote_transition_count"):
            changed=copy.deepcopy(expected_report()); changed["product_truth"][field]=1; self.assertTrue(validate_report(changed),field)
        changed=copy.deepcopy(expected_report()); changed["product_truth"]["release_claim"]="ready"; self.assertTrue(validate_report(changed))
if __name__=="__main__": unittest.main()
