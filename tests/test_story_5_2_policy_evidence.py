from __future__ import annotations

import copy
import unittest

from scripts.story_5_2_policy_evidence import expected_report, validate


class Story52PolicyEvidenceTests(unittest.TestCase):
    def test_current_map_is_exact_complete_and_limited(self) -> None:
        report = expected_report()
        self.assertEqual(validate(report), [])
        self.assertEqual(report["decision_table"]["registered_operation_count"], 22)
        self.assertEqual([item["protocol"] for item in report["review_protocols"]], ["RV-12", "RV-17", "RV-25"])
        self.assertTrue(all(not item["product_complete"] for item in report["review_protocols"]))

    def test_coverage_mutation_or_race_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["decision_table"]["operation_effect_mappings"].pop()
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["mutation_results"]["dispatches_after_mutation"] = 1
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["race_results"]["effect_callback_invocations"] = 2
        self.assertTrue(validate(changed))

    def test_protocol_and_product_claim_widening_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["review_protocols"][2]["status"] = "complete"
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["external_effect_executed"] = True
        self.assertTrue(validate(changed))
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["story_completion_claim"] = True
        self.assertTrue(validate(changed))


if __name__ == "__main__":
    unittest.main()
