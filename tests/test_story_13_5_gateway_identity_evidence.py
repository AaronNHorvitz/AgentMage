from __future__ import annotations
import copy, unittest
from scripts.story_13_5_gateway_identity_evidence import MARKERS, TRUTH, expected_report, validate_raw, validate_report, validate_sources

class Story135GatewayIdentityEvidenceTests(unittest.TestCase):
    def test_sources_and_expected_report_are_current(self) -> None:
        self.assertEqual(validate_sources(), []); self.assertEqual(validate_report(expected_report()), [])
    def test_every_execution_marker_is_required(self) -> None:
        valid = "\n".join(MARKERS); self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS: self.assertTrue(validate_raw(valid.replace(marker, "")), marker)
    def test_activation_authority_fallback_and_later_gate_overclaims_are_rejected(self) -> None:
        for field in ("model_proposal_has_authority", "live_endpoint_qualification_complete", "routing_rv54_slice_complete"):
            changed = copy.deepcopy(expected_report()); changed["product_truth"][field] = not TRUTH[field]; self.assertTrue(validate_report(changed), field)
        for field in ("enabled_candidate_count", "selected_route_count"):
            changed = copy.deepcopy(expected_report()); changed["product_truth"][field] = 1; self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report()); changed["product_truth"]["automatic_fallback_enabled"] = True; self.assertTrue(validate_report(changed))
    def test_release_overclaim_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report()); changed["product_truth"]["release_claim"] = "ready"; self.assertTrue(validate_report(changed))

if __name__ == "__main__": unittest.main()
