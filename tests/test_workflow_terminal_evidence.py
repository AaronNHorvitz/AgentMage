from __future__ import annotations

import copy
import unittest

from scripts.workflow_terminal_evidence import expected_report, validate


class WorkflowTerminalEvidenceTests(unittest.TestCase):
    def test_current_report_has_the_exact_nine_outcomes(self) -> None:
        report = expected_report()
        self.assertEqual(validate(report), [])
        self.assertEqual(len(report["terminal_outcomes"]), 9)
        self.assertIn("denied", report["terminal_outcomes"])

    def test_outcome_collapse_or_success_bypass_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["terminal_outcomes"].remove("uncertain")
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["terminal_contract"]["success_requires_opaque_current_evidence_proof"] = False
        self.assertTrue(validate(changed))

    def test_diagnostic_or_product_claim_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["terminal_contract"]["non_success_diagnostic_is_required"] = False
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "complete"
        self.assertTrue(validate(changed))


if __name__ == "__main__":
    unittest.main()
