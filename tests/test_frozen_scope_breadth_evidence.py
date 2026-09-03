from __future__ import annotations

import copy
import unittest

from scripts.frozen_scope_breadth_evidence import expected_report, validate_inputs, validate_report


class FrozenScopeBreadthEvidenceTests(unittest.TestCase):
    def test_current_inputs_and_exact_report_pass(self) -> None:
        self.assertEqual(validate_inputs(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(len(expected_report()["criteria"]), 5)

    def test_every_promotion_or_substitution_overclaim_fails(self) -> None:
        for field in (
            "installed_product_complete",
            "native_cross_platform_complete",
            "story_10_complete",
            "sprint_10_complete",
            "story_11_1_complete",
            "sprint_11_complete",
            "external_evidence_substituted",
        ):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "ready"
        self.assertTrue(validate_report(changed))

    def test_criterion_or_artifact_mutation_fails(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["criteria"].pop()
        self.assertTrue(validate_report(changed))
        changed = copy.deepcopy(expected_report())
        changed["artifacts"][0]["sha256"] = "0" * 64
        self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
