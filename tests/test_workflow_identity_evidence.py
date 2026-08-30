from __future__ import annotations

import copy
import unittest

from scripts.workflow_identity_evidence import expected_report, validate


class WorkflowIdentityEvidenceTests(unittest.TestCase):
    def test_current_report_is_exact_complete_and_bounded(self) -> None:
        report = expected_report()
        self.assertEqual(validate(report), [])
        self.assertEqual(len(report["identity_families"]), 7)
        self.assertEqual(report["race_contract"]["successful_identity_reservations"], 1)
        self.assertFalse(report["product_truth"]["runtime_effect_executed"])

    def test_freshness_or_retry_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["issuance_contract"]["all_identity_roles_globally_disjoint"] = False
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["retry_contract"]["uncertain_effect_automatic_retry"] = True
        self.assertTrue(validate(changed))

    def test_race_or_product_claim_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["race_contract"]["successful_identity_reservations"] = 2
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["capability_grant_issued"] = True
        self.assertTrue(validate(changed))


if __name__ == "__main__":
    unittest.main()
