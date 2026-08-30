from __future__ import annotations

import copy
import unittest

from scripts.story_11_2_ac3_evidence import (
    MARKERS,
    SURFACES,
    TRUTH,
    expected_report,
    validate_raw,
    validate_report,
    validate_retained,
    validate_upstream_reports,
)


class Story112Ac3EvidenceTests(unittest.TestCase):
    def test_current_inventory_and_upstream_reports_satisfy_acceptance(self) -> None:
        self.assertEqual(validate_retained(), [])
        self.assertEqual(validate_upstream_reports(), [])
        self.assertEqual(validate_report(expected_report()), [])

    def test_lifecycle_and_surface_sets_are_complete(self) -> None:
        self.assertEqual(TRUTH["source_workflow_family_count"], 31)
        self.assertEqual(TRUTH["unauthorized_raw_restricted_data_matches"], 0)
        self.assertEqual(
            list(SURFACES),
            [
                "sqlcipher-main",
                "sqlcipher-wal",
                "sqlcipher-shm",
                "encrypted-backup",
                "content-free-export",
                "receipt-and-diagnostic-strings",
            ],
        )

    def test_every_reconciliation_and_canary_invariant_is_required(self) -> None:
        for field in (
            "whole_store_backup_complete",
            "fresh_candidate_restore_complete",
            "source_and_payload_reference_reconciliation_complete",
            "shared_payload_preservation_complete",
            "retention_hold_expiry_release_delete_complete",
            "content_free_export_coverage_complete",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = False
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["unauthorized_raw_restricted_data_matches"] = 1
        self.assertTrue(validate_report(changed))

    def test_uninstall_remanence_platform_review_and_release_overclaims_are_rejected(self) -> None:
        for field in (
            "complete_product_uninstall_and_external_copy_cleanup",
            "physical_remanence_complete",
            "all_later_active_surfaces_complete",
            "cross_platform_complete",
            "independent_review_complete",
            "story_completion_claim",
            "sprint_completion_claim",
        ):
            changed = copy.deepcopy(expected_report())
            changed["acceptance_truth"][field] = True
            self.assertTrue(validate_report(changed), field)
        changed = copy.deepcopy(expected_report())
        changed["acceptance_truth"]["release_claim"] = "ready"
        self.assertTrue(validate_report(changed))

    def test_raw_results_require_every_validator_and_no_failure(self) -> None:
        valid = "\n".join(MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nTraceback"))


if __name__ == "__main__":
    unittest.main()
