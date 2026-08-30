from __future__ import annotations

import copy
import unittest

from scripts.workflow_atomic_publication_evidence import (
    MARKERS,
    SOURCE_MARKERS,
    SOURCE_PATH,
    expected_report,
    validate_raw,
    validate_report,
    validate_source,
)


class WorkflowAtomicPublicationEvidenceTests(unittest.TestCase):
    def test_current_source_and_report_are_exact(self) -> None:
        self.assertEqual(validate_source(SOURCE_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_each_atomic_member_is_required(self) -> None:
        source = SOURCE_PATH.read_text()
        for marker in SOURCE_MARKERS:
            self.assertTrue(validate_source(source.replace(marker, "REMOVED")), marker)

    def test_commit_order_cannot_be_widened(self) -> None:
        source = SOURCE_PATH.read_text()
        self.assertTrue(
            validate_source(source.replace("runtime_resume_binding_after_events: true", "false", 1))
        )

    def test_follow_on_or_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "full_workflow_crash_campaign_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = True
            self.assertTrue(validate_report(changed), field)

    def test_raw_results_require_every_marker_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
