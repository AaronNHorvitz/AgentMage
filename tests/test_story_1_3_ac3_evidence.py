from __future__ import annotations

import copy
import unittest

from scripts.story_1_3_ac3_evidence import (
    MARKERS, TRUTH, expected_report, validate_raw, validate_report, validate_upstream,
)


class Story13Ac3EvidenceTests(unittest.TestCase):
    def test_current_predispatch_rejection_evidence_is_complete(self) -> None:
        self.assertEqual(validate_upstream(), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(TRUTH["schema_rejection_count"], 45)
        self.assertEqual(TRUTH["trusted_boundary_rejection_count"], 2)

    def test_any_rejected_input_reaching_dispatch_is_refused(self) -> None:
        for field in (
            "malformed_records_reach_publication", "stale_records_reach_dispatch",
            "unsupported_versions_reach_dispatch", "cyclic_workflows_reach_dispatch",
            "model_dispatch_after_rejection", "tool_dispatch_after_rejection",
            "effect_dispatch_after_rejection",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)

    def test_content_or_prose_diagnostics_are_refused(self) -> None:
        for field in ("diagnostics_include_candidate_content", "terminal_diagnostics_admit_free_prose"):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["diagnostics_are_stable_codes"] = False
        self.assertTrue(validate_report(changed))

    def test_execution_and_later_gate_overclaims_are_refused(self) -> None:
        for field in (
            "model_inference_executed", "tool_worker_executed", "runtime_effect_executed",
            "installed_product_complete", "native_platform_complete", "independent_review_complete",
            "story_completion_claim", "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "pass"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_every_boundary_and_no_failure_marker(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
