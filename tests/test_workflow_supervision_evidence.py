from __future__ import annotations

import copy
import unittest

from scripts.workflow_supervision_evidence import FAILURE_CLASSES, MARKERS, TRUTH, expected_report, validate_raw, validate_report


class WorkflowSupervisionEvidenceTests(unittest.TestCase):
    def test_current_report_is_exact_and_closed(self) -> None:
        report = expected_report()
        self.assertEqual(validate_report(report), [])
        self.assertEqual(len(FAILURE_CLASSES), 14)
        self.assertEqual(len(report["budget_dimensions"]), 5)

    def test_dimension_or_failure_class_mutation_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["budget_dimensions"].remove("replan")
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["failure_classes"].pop()
        self.assertIn("widened", validate_report(changed)[0])

    def test_arithmetic_or_atomicity_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["accounting_contract"]["checked_addition"] = False
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["accounting_contract"]["rejected_charge_mutates_usage"] = True
        self.assertIn("widened", validate_report(changed)[0])

    def test_policy_identity_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["accounting_contract"]["ledger_bound_to_policy_id_and_sha256"] = False
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["accounting_contract"]["caller_supplied_policy_sha256"] = True
        self.assertIn("widened", validate_report(changed)[0])

    def test_fingerprint_or_repeated_state_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["repeated_state_contract"]["fingerprint_dimensions"].remove("verifier_state")
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["repeated_state_contract"]["non_adjacent_cycles_detected"] = False
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["repeated_state_contract"]["decision_contains_authority"] = True
        self.assertIn("widened", validate_report(changed)[0])

    def test_termination_reason_action_and_authority_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["termination_contract"]["caller_text_in_reason"] = True
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["termination_contract"]["nonterminal_input_can_terminate"] = True
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["termination_contract"]["authority_consumed"] = True
        self.assertIn("widened", validate_report(changed)[0])

    def test_product_truth_and_raw_results_fail_closed(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["runtime_effect_executed"] = True
        self.assertIn("widened", validate_report(changed)[0])
        self.assertEqual(TRUTH["release_claim"], "none")
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
