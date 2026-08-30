from __future__ import annotations

import copy
import unittest

from scripts.source_materialization_schema_evidence import (
    MARKERS,
    MIGRATION_PATH,
    TABLES,
    expected_report,
    validate_migration,
    validate_raw,
    validate_report,
)


class SourceMaterializationSchemaEvidenceTests(unittest.TestCase):
    def test_current_migration_and_report_are_exact(self) -> None:
        self.assertEqual(validate_migration(MIGRATION_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])
        self.assertEqual(expected_report()["normalized_tables"], list(TABLES))

    def test_second_payload_store_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        self.assertTrue(validate_migration(f"{migration}\nCREATE TABLE source_payloads (payload BLOB);"))

    def test_missing_table_or_existing_artifact_binding_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        self.assertTrue(validate_migration(migration.replace("CREATE TABLE source_manifests (", "")))
        self.assertTrue(
            validate_migration(
                migration.replace(
                    "REFERENCES runtime_artifacts(artifact_id, payload_sha256, byte_size)",
                    "REFERENCES runtime_payloads(payload_sha256)",
                )
            )
        )

    def test_follow_on_or_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "typed_publication_api_complete",
            "content_deduplication_complete",
            "refresh_invalidation_lifecycle_complete",
            "full_crash_campaign_complete",
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
