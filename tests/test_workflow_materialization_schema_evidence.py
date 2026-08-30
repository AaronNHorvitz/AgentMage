from __future__ import annotations

import copy
import unittest

from scripts.workflow_materialization_schema_evidence import (
    MARKERS,
    MIGRATION_PATH,
    TABLES,
    expected_report,
    validate_migration,
    validate_raw,
    validate_report,
)


class WorkflowMaterializationSchemaEvidenceTests(unittest.TestCase):
    def test_current_migration_and_report_are_exact(self) -> None:
        self.assertEqual(validate_migration(MIGRATION_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(expected_report()["normalized_tables"], list(TABLES))

    def test_second_payload_or_journal_authority_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        self.assertTrue(
            validate_migration(f"{migration}\nCREATE TABLE workflow_payloads (payload BLOB);")
        )
        self.assertTrue(
            validate_migration(f"{migration}\nCREATE TABLE workflow_event_journal (event_id TEXT);")
        )

    def test_missing_table_or_existing_authority_binding_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        self.assertTrue(
            validate_migration(migration.replace("CREATE TABLE workflow_attempts (", ""))
        )
        self.assertTrue(
            validate_migration(
                migration.replace(
                    "REFERENCES runtime_events(run_id, sequence, event_id)",
                    "REFERENCES runtime_events(event_id)",
                    1,
                )
            )
        )

    def test_follow_on_or_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "typed_publication_api_complete",
            "attempt_and_idempotency_invariants_complete",
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
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
