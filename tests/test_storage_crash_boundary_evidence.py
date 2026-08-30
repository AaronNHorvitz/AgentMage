from __future__ import annotations

import copy
import unittest

from scripts.storage_crash_boundary_evidence import (
    BOUNDARY_MARKERS,
    MARKERS,
    SOURCE_MARKERS,
    SOURCE_PATH,
    expected_report,
    validate_raw,
    validate_report,
    validate_source,
)


class StorageCrashBoundaryEvidenceTests(unittest.TestCase):
    def test_current_source_and_report_are_exact(self) -> None:
        self.assertEqual(validate_source(SOURCE_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_every_named_boundary_and_campaign_marker_is_required(self) -> None:
        source = SOURCE_PATH.read_text()
        for marker in (*BOUNDARY_MARKERS, *SOURCE_MARKERS):
            self.assertTrue(validate_source(source.replace(marker, "REMOVED")), marker)

    def test_boundary_count_cannot_be_reduced(self) -> None:
        source = SOURCE_PATH.read_text()
        self.assertTrue(validate_source(source.replace("const ALL: [Self; 16]", "const ALL: [Self; 15]")))

    def test_later_task_story_sprint_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "cross_table_old_or_new_matrix_complete",
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
