from __future__ import annotations

import copy
import unittest

from scripts.story_50_4_core_milestone_evidence import expected_report, validate_report


class Story504CoreMilestoneEvidenceTests(unittest.TestCase):
    def test_local_core_closes_without_widening(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertTrue(report["gate_closed"])
        self.assertFalse(report["product_truth"]["complete_foundational_runtime_milestone"])

    def test_external_later_or_release_overclaim_fails_closed(self) -> None:
        for field in (
            "structured_parser_milestone_complete", "capability_registry_protocol_complete",
            "multi_agent_protocol_complete", "qualified_model_endpoint_or_route",
            "remote_endpoint_protocol_complete", "installed_client_protocol_complete",
            "independent_review_complete", "windows_validation_complete", "macos_validation_complete",
            "complete_foundational_runtime_milestone",
        ):
            with self.subTest(field=field):
                changed = copy.deepcopy(expected_report())
                changed["product_truth"][field] = True
                self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "v0.4"
        self.assertTrue(validate_report(changed))

    def test_later_protocol_pass_claim_fails_closed(self) -> None:
        for protocol in ("RV-56", "RV-57"):
            changed = copy.deepcopy(expected_report())
            next(item for item in changed["protocols"] if item["id"] == protocol)["status"] = "PASS"
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
