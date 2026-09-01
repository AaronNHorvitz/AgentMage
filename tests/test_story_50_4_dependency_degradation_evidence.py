from __future__ import annotations

import copy
import unittest

from scripts.story_50_4_dependency_degradation_evidence import expected_report, validate_report


class Story504DependencyDegradationEvidenceTests(unittest.TestCase):
    def test_current_local_truth_is_exact(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertEqual(report["requirement_ids"], ["AM-DEG-001", "AT-DEG-001"])

    def test_safety_or_completion_widening_fails_closed(self) -> None:
        for field in (
            "authority_broadening_permitted", "security_or_verification_weakening_permitted",
            "silent_omission_or_route_change", "false_success_authority",
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(expected_report())
                changed["product_truth"][field] = True
                self.assertTrue(validate_report(changed))

    def test_external_or_release_overclaim_fails_closed(self) -> None:
        for field in (
            "live_dependency_removal_campaign_complete",
            "qualified_model_or_endpoint_campaign_complete", "installed_client_campaign_complete",
            "independent_review_complete", "windows_validation_complete", "macos_validation_complete",
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(expected_report())
                changed["product_truth"][field] = True
                self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "v0.4"
        self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
