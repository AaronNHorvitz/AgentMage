from __future__ import annotations

import copy
import unittest

from scripts.effect_class_taxonomy_evidence import (
    EFFECT_CLASSES,
    FAILURE_CLASSES,
    MARKERS,
    OPERATION_EFFECT_MAPPINGS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
)


class EffectClassTaxonomyEvidenceTests(unittest.TestCase):
    def test_current_report_is_exact_and_closed(self) -> None:
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(len(EFFECT_CLASSES), 7)

    def test_class_or_matrix_mutation_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["effect_classes"][0]["class"] = "custom"
        self.assertIn("stale", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["independent_dimensions"]["authority_or_risk_can_infer_effect"] = True
        self.assertIn("widened", validate_report(changed)[0])

    def test_unsafe_retry_or_approval_mutation_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["effect_classes"][-1]["automatic_retry"] = True
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["effect_classes"][-1]["approval_required"] = False
        self.assertIn("widened", validate_report(changed)[0])

    def test_failure_class_or_default_disposition_mutation_is_rejected(self) -> None:
        self.assertEqual(len(FAILURE_CLASSES), 14)
        changed = copy.deepcopy(expected_report())
        changed["failure_classes"][0]["class"] = "custom"
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["failure_classes"][10]["default_disposition"] = "eligible_fresh_attempt"
        self.assertIn("widened", validate_report(changed)[0])

    def test_operation_mapping_omission_duplication_and_override_are_rejected(self) -> None:
        self.assertEqual(len(OPERATION_EFFECT_MAPPINGS), 22)
        changed = copy.deepcopy(expected_report())
        changed["operation_effect_mappings"].pop()
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["operation_effect_mappings"].append(changed["operation_effect_mappings"][0])
        self.assertIn("widened", validate_report(changed)[0])
        changed = copy.deepcopy(expected_report())
        changed["registration_contract"]["caller_supplied_effect_class"] = True
        self.assertIn("widened", validate_report(changed)[0])

    def test_product_truth_cannot_be_promoted(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["product_runtime_executed"] = True
        self.assertIn("widened", validate_report(changed)[0])
        self.assertEqual(TRUTH["release_claim"], "none")

    def test_raw_results_require_every_marker_and_reject_failure_output(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
