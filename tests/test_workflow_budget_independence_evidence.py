from __future__ import annotations

import copy
import unittest

from scripts.workflow_budget_independence_evidence import expected_report, validate


class WorkflowBudgetIndependenceEvidenceTests(unittest.TestCase):
    def test_current_report_is_exact_complete_and_bounded(self) -> None:
        report = expected_report()
        self.assertEqual(validate(report), [])
        self.assertEqual(len(report["independent_limits"]), 6)
        self.assertFalse(report["product_truth"]["model_inference_executed"])

    def test_dimension_or_atomicity_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["independent_limits"].remove("model_repair")
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["accounting_contract"]["exhausted_dimension_mutates_any_counter"] = True
        self.assertTrue(validate(changed))

    def test_repeat_or_product_claim_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["accounting_contract"]["repeated_state_stop_mutates_budget_ledger"] = True
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["model_inference_executed"] = True
        self.assertTrue(validate(changed))


if __name__ == "__main__":
    unittest.main()
