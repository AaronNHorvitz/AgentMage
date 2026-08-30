from __future__ import annotations

import copy
import unittest

from scripts.storage_new_family_lifecycle_evidence import (
    LIFECYCLE_MARKERS,
    LIFECYCLE_PATH,
    MARKERS,
    SOURCE_MARKERS,
    SOURCE_PATH,
    expected_report,
    validate_raw,
    validate_report,
    validate_source,
)


class StorageNewFamilyLifecycleEvidenceTests(unittest.TestCase):
    def test_current_sources_and_report_are_exact(self) -> None:
        self.assertEqual(
            validate_source(SOURCE_PATH.read_text(), LIFECYCLE_PATH.read_text()), []
        )
        self.assertEqual(validate_report(expected_report()), [])

    def test_every_source_and_lifecycle_marker_is_required(self) -> None:
        source = SOURCE_PATH.read_text()
        lifecycle = LIFECYCLE_PATH.read_text()
        for marker in SOURCE_MARKERS:
            self.assertTrue(validate_source(source.replace(marker, "REMOVED"), lifecycle), marker)
        for marker in LIFECYCLE_MARKERS:
            self.assertTrue(validate_source(source, lifecycle.replace(marker, "REMOVED")), marker)

    def test_family_omission_duplication_and_content_fields_are_rejected(self) -> None:
        source = SOURCE_PATH.read_text()
        lifecycle = LIFECYCLE_PATH.read_text()
        family = 'family: "source_released_payloads"'
        self.assertTrue(validate_source(source.replace(family, ""), lifecycle))
        self.assertTrue(validate_source(source.replace(family, f"{family}\n{family}"), lifecycle))
        insertion = 'record_json FROM source_origins'
        self.assertTrue(validate_source(source.replace("FROM source_origins", insertion), lifecycle))

    def test_crash_story_sprint_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "exhaustive_crash_campaign_complete",
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
