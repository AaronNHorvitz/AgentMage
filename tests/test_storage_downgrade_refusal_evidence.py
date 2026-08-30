from __future__ import annotations

import copy
import unittest

from scripts.storage_downgrade_refusal_evidence import (
    MARKERS,
    SOURCE_MARKERS,
    SOURCE_PATH,
    expected_report,
    validate_raw,
    validate_report,
    validate_source,
)


class StorageDowngradeRefusalEvidenceTests(unittest.TestCase):
    def test_current_source_and_report_are_exact(self) -> None:
        self.assertEqual(validate_source(SOURCE_PATH.read_text()), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_each_refusal_and_preservation_marker_is_required(self) -> None:
        source = SOURCE_PATH.read_text()
        for marker in SOURCE_MARKERS:
            self.assertTrue(validate_source(source.replace(marker, "REMOVED")), marker)

    def test_writer_claim_cannot_move_before_compatibility_refusal(self) -> None:
        source = SOURCE_PATH.read_text()
        start = source.index("fn open_keyed(")
        claim = source.index("claim_exclusive_writer", start)
        compatibility = source.index("open_connection_for_supported_schema", start)
        changed = source[:compatibility] + source[claim:claim + len("claim_exclusive_writer")] + source[compatibility:claim] + source[claim + len("claim_exclusive_writer"):]
        self.assertTrue(validate_source(changed))

    def test_reverse_partial_or_product_overclaims_are_rejected(self) -> None:
        for field in (
            "automatic_reverse_migration_allowed",
            "partial_interpretation_allowed",
            "record_deletion_or_rewrite_allowed",
            "new_family_lifecycle_coverage_complete",
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
