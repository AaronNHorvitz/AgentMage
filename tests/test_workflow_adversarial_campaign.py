from __future__ import annotations

import copy
import unittest

from scripts.workflow_adversarial_campaign import ATTACK_MARKERS, expected_report, validate


class WorkflowAdversarialCampaignTests(unittest.TestCase):
    def test_current_report_has_all_eight_closed_attack_families(self) -> None:
        report = expected_report()
        self.assertEqual(validate(report), [])
        self.assertEqual(len(report["attack_families"]), 8)
        self.assertEqual({item["attack"] for item in report["attack_families"]}, set(ATTACK_MARKERS))

    def test_missing_attack_or_nonzero_admission_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["attack_families"].pop()
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["attack_families"][0]["admitted_dispatches"] = 1
        self.assertTrue(validate(changed))

    def test_uncertainty_or_product_claim_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["campaign_contract"]["uncertain_effects_become_success"] = True
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["rv52_completion_claim"] = True
        self.assertTrue(validate(changed))


if __name__ == "__main__":
    unittest.main()
