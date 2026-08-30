from __future__ import annotations

import copy
import unittest

from scripts.story_1_3_ac1_evidence import (
    MARKERS, TRUTH, expected_report, validate_raw, validate_report, validate_upstream,
)


class Story13Ac1EvidenceTests(unittest.TestCase):
    def test_current_canonical_boundary_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["canonical_record_family_count"], 9)
        self.assertEqual(TRUTH["canonical_schema_version"], 2)

    def test_rust_owned_shape_and_version_truth_is_required(self) -> None:
        for field in (
            "rust_owned_meaning", "rust_and_json_field_sets_match",
            "current_canonical_record_boundary_scope_complete",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = False
            self.assertTrue(validate_report(changed), field)

    def test_fail_closed_boundary_cannot_be_widened(self) -> None:
        for field in (
            "required_nullable_fields_may_be_omitted", "unknown_fields_admitted",
            "missing_fields_admitted", "malformed_records_admitted", "oversized_records_admitted",
            "unsupported_versions_admitted", "invalid_records_reach_canonical_publication",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)

    def test_product_and_later_gate_overclaims_are_rejected(self) -> None:
        for field in (
            "model_inference_executed", "runtime_effect_executed", "installed_product_complete",
            "native_platform_complete", "independent_review_complete", "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "pass"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_all_schema_and_boundary_markers(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
