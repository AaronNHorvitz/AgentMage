from __future__ import annotations

import copy
import unittest

from scripts.workflow_definition_evidence import expected_report, validate


class WorkflowDefinitionEvidenceTests(unittest.TestCase):
    def test_current_report_is_exact_and_bounded(self) -> None:
        report = expected_report()
        self.assertEqual(validate(report), [])
        self.assertEqual(report["terminal_contract"]["lifecycle_states"], 18)
        self.assertEqual(report["test_contract"]["focused_tests"], 8)
        self.assertFalse(report["product_truth"]["story_completion_claim"])

    def test_graph_or_policy_coverage_mutation_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["graph_contract"]["entry_frontier_exact"] = False
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["step_policy_contract"]["policy_identity_reuse_allowed"] = True
        self.assertTrue(validate(changed))

    def test_product_claim_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["runtime_effect_executed"] = True
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "complete"
        self.assertTrue(validate(changed))


if __name__ == "__main__":
    unittest.main()
