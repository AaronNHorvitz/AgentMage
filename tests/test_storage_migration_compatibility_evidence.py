from __future__ import annotations

import copy
import unittest

from scripts.storage_migration_compatibility_evidence import (
    FIXTURE_PATH,
    MARKERS,
    MIGRATION_SHA256,
    SOURCE_MARKERS,
    SOURCE_PATH,
    expected_report,
    validate_fixture,
    validate_raw,
    validate_report,
    validate_source,
)

import json


class StorageMigrationCompatibilityEvidenceTests(unittest.TestCase):
    def test_current_source_fixture_and_report_are_exact(self) -> None:
        self.assertEqual(validate_source(SOURCE_PATH.read_text()), [])
        self.assertEqual(validate_fixture(json.loads(FIXTURE_PATH.read_text())), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_each_source_boundary_is_required(self) -> None:
        source = SOURCE_PATH.read_text()
        for marker in SOURCE_MARKERS:
            self.assertTrue(validate_source(source.replace(marker, "REMOVED")), marker)

    def test_fixture_version_order_hashes_and_tables_are_closed(self) -> None:
        fixture = json.loads(FIXTURE_PATH.read_text())
        mutations = []
        changed = copy.deepcopy(fixture)
        changed["schema_version"] = 17
        mutations.append(changed)
        changed = copy.deepcopy(fixture)
        changed["migrations"][0]["sha256"] = "f" * 64
        mutations.append(changed)
        changed = copy.deepcopy(fixture)
        changed["migrations"].reverse()
        mutations.append(changed)
        changed = copy.deepcopy(fixture)
        changed["tables"].remove("workflow_attempts")
        mutations.append(changed)
        for mutation in mutations:
            self.assertTrue(validate_fixture(mutation))
        self.assertEqual(len(MIGRATION_SHA256), 16)

    def test_follow_on_or_product_completion_overclaim_is_rejected(self) -> None:
        for field in (
            "downgrade_record_behavior_complete",
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
