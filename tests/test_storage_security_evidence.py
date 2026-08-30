from __future__ import annotations

import copy
import unittest

from scripts.storage_security_evidence import (
    MAPPINGS,
    MARKERS,
    RETAINED_PATHS,
    expected_report,
    validate_raw,
    validate_report,
    validate_retained,
)


class StorageSecurityEvidenceTests(unittest.TestCase):
    def test_current_inventory_and_report_are_exact(self) -> None:
        self.assertEqual(validate_retained(), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_every_retained_artifact_is_hash_bound(self) -> None:
        report = expected_report()
        self.assertEqual([item["path"] for item in report["retained_evidence"]], list(RETAINED_PATHS))
        self.assertTrue(all(len(item["sha256"]) == 64 for item in report["retained_evidence"]))

    def test_review_mapping_set_is_exact(self) -> None:
        self.assertEqual([item["review_id"] for item in MAPPINGS], ["RV-08", "RV-09", "RV-10", "RV-17"])
        changed = copy.deepcopy(expected_report())
        changed["review_mappings"].pop()
        self.assertTrue(validate_report(changed))

    def test_independent_story_sprint_and_release_overclaims_are_rejected(self) -> None:
        for field in ("independent_review_complete", "story_completion_claim", "sprint_completion_claim"):
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
        self.assertTrue(validate_raw(f"{valid}\nTraceback"))


if __name__ == "__main__":
    unittest.main()
