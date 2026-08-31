from __future__ import annotations

import copy
import unittest

from scripts.story_11_3_resume_evidence import (
    MARKERS,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_sources,
)


class Story113ResumeEvidenceTests(unittest.TestCase):
    def test_sources_and_expected_report_are_current(self) -> None:
        self.assertEqual(validate_sources(), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_every_runtime_marker_is_required(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        for marker in MARKERS:
            self.assertTrue(validate_raw(valid.replace(marker, "")), marker)

    def test_safety_truth_cannot_be_widened(self) -> None:
        for field in (
            "uncertain_or_completed_effect_replayed",
            "exhausted_budget_dispatch_permitted",
            "terminal_workflow_dispatch_permitted",
            "client_presence_owns_task_lifetime",
            "native_cross_platform_complete",
            "installed_product_complete",
            "full_rv52_protocol_complete",
        ):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = not TRUTH[field]
            self.assertTrue(validate_report(changed), field)

    def test_release_overclaim_is_rejected(self) -> None:
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "ready"
        self.assertTrue(validate_report(changed))

    def test_failure_output_is_rejected(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))
        self.assertTrue(validate_raw(f"{valid}\npanicked at source"))


if __name__ == "__main__":
    unittest.main()
