from __future__ import annotations

import copy
import unittest

from scripts.workflow_attempt_invariants_evidence import (
    MARKERS,
    MIGRATION_PATH,
    TRIGGERS,
    UNIQUE_INDEXES,
    expected_report,
    validate_migration,
    validate_raw,
    validate_report,
)


class WorkflowAttemptInvariantsEvidenceTests(unittest.TestCase):
    def test_current_migration_and_report_are_exact(self) -> None:
        self.assertEqual(validate_migration(MIGRATION_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_removed_index_or_trigger_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        for name in (*UNIQUE_INDEXES, *TRIGGERS):
            self.assertTrue(validate_migration(migration.replace(name, f"removed_{name}")), name)

    def test_receipt_or_terminal_clause_widening_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        for clause in (
            "prior.state <> 'started'",
            "terminal_receipt.receipt_sha256 = NEW.receipt_sha256",
            "terminal_receipt.outcome = NEW.state",
            "OLD.state <> 'started'",
        ):
            self.assertTrue(validate_migration(migration.replace(clause, "1 = 1")), clause)

    def test_follow_on_or_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "atomic_event_projection_checkpoint_commit_complete",
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
        self.assertTrue(validate_raw(f"{valid}\nwarning: widened"))


if __name__ == "__main__":
    unittest.main()
