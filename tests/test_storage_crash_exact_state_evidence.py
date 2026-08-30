from __future__ import annotations

import copy
import unittest

from scripts.storage_crash_exact_state_evidence import (
    MARKERS,
    SOURCE_MARKERS,
    SOURCE_PATH,
    expected_report,
    validate_raw,
    validate_report,
    validate_source,
)


class StorageCrashExactStateEvidenceTests(unittest.TestCase):
    def test_current_source_and_report_are_exact(self) -> None:
        self.assertEqual(validate_source(SOURCE_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_every_exact_state_marker_is_required(self) -> None:
        source = SOURCE_PATH.read_text()
        for marker in SOURCE_MARKERS:
            self.assertTrue(validate_source(source.replace(marker, "REMOVED")), marker)

    def test_zero_or_one_and_no_replay_assertions_cannot_be_removed(self) -> None:
        source = SOURCE_PATH.read_text()
        for marker in (SOURCE_MARKERS[1], SOURCE_MARKERS[-2], SOURCE_MARKERS[-1]):
            self.assertTrue(validate_source(source.replace(marker, "REMOVED")), marker)

    def test_later_task_story_sprint_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "retained_trace_and_rv_mapping_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "ready"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_every_marker_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
