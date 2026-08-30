from __future__ import annotations

import copy
import unittest

from scripts.workflow_verifier_evidence import expected_report, validate


class WorkflowVerifierEvidenceTests(unittest.TestCase):
    def test_current_report_is_exact_complete_and_bounded(self) -> None:
        report = expected_report()
        self.assertEqual(validate(report), [])
        self.assertEqual(len(report["evaluated_surfaces"]), 8)
        self.assertFalse(report["product_truth"]["terminal_result_issued"])

    def test_missing_surface_or_false_authority_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["evaluated_surfaces"].remove("receipt_integrity")
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["completion_contract"]["exit_zero_has_completion_authority"] = True
        self.assertTrue(validate(changed))

    def test_terminal_or_product_claim_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["completion_contract"]["terminal_result_issued_by_this_increment"] = True
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["story_completion_claim"] = True
        self.assertTrue(validate(changed))


if __name__ == "__main__":
    unittest.main()
