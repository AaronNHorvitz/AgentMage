from __future__ import annotations

import copy
import unittest

from scripts.source_content_deduplication_evidence import (
    MARKERS,
    MIGRATION_PATH,
    REQUIRED_MIGRATION_FRAGMENTS,
    expected_report,
    validate_migration,
    validate_raw,
    validate_report,
)


class SourceContentDeduplicationEvidenceTests(unittest.TestCase):
    def test_current_migration_and_report_are_exact(self) -> None:
        self.assertEqual(validate_migration(MIGRATION_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_missing_identity_constraint_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        for fragment in REQUIRED_MIGRATION_FRAGMENTS:
            self.assertTrue(validate_migration(migration.replace(fragment, "removed", 1)), fragment)

    def test_second_payload_authority_is_rejected(self) -> None:
        migration = MIGRATION_PATH.read_text()
        self.assertTrue(validate_migration(f"{migration}\nCREATE TABLE source_payloads (payload BLOB);"))

    def test_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "refresh_invalidation_lifecycle_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["product_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["product_truth"]["release_claim"] = "ga"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_every_marker_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\ntest result: FAILED"))


if __name__ == "__main__":
    unittest.main()
