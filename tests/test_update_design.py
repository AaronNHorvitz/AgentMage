from __future__ import annotations

import copy
import unittest

from scripts.update_design import (
    REPORT_PATH,
    ROLLBACK_PATH,
    UPDATE_PATH,
    build_report,
    check_artifact,
    read_json,
    validate_report,
    validate_rollback,
    validate_update,
)


class UpdateDesignTests(unittest.TestCase):
    def setUp(self) -> None:
        self.update = read_json(UPDATE_PATH)
        self.rollback = read_json(ROLLBACK_PATH)

    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_canonical_design_records_are_closed_and_valid(self) -> None:
        self.assertEqual(validate_update(self.update), [])
        self.assertEqual(validate_rollback(self.rollback), [])

    def test_update_network_background_and_remote_authority_are_rejected(self) -> None:
        for key in (
            "automatic_update_checks",
            "background_downloads",
            "remote_triggers",
            "remote_control",
        ):
            changed = copy.deepcopy(self.update)
            changed["delivery"][key] = True
            self.assertTrue(validate_update(changed))

    def test_update_identity_signature_and_downgrade_controls_are_required(self) -> None:
        identity = copy.deepcopy(self.update)
        identity["required_identities"].pop()
        signature = copy.deepcopy(self.update)
        signature["trusted_metadata"]["signature_threshold_required"] = False
        downgrade = copy.deepcopy(self.update)
        downgrade["version_policy"]["downgrade_rejected"] = False
        for changed in (identity, signature, downgrade):
            self.assertTrue(validate_update(changed))

    def test_rollback_rejects_stale_revoked_and_unsafe_data_paths(self) -> None:
        stale = copy.deepcopy(self.rollback)
        stale["eligibility"]["exact_active_release_precondition"] = False
        revoked = copy.deepcopy(self.rollback)
        revoked["eligibility"]["target_not_revoked"] = False
        overwrite = copy.deepcopy(self.rollback)
        overwrite["data_policy"]["later_user_data_overwrite_allowed"] = True
        blind = copy.deepcopy(self.rollback)
        blind["data_policy"]["blind_database_restore_allowed"] = True
        for changed in (stale, revoked, overwrite, blind):
            self.assertTrue(validate_rollback(changed))

    def test_unknown_fields_implementation_and_macos_overclaims_fail_closed(self) -> None:
        unknown = copy.deepcopy(self.update)
        unknown["silent_channel"] = True
        implementation = copy.deepcopy(self.update)
        implementation["implementation_status"]["metadata_verifier"] = "implemented"
        macos = copy.deepcopy(self.rollback)
        macos["platform_status"]["macos_execution"] = "pass"
        self.assertTrue(validate_update(unknown))
        self.assertTrue(validate_update(implementation))
        self.assertTrue(validate_rollback(macos))

    def test_stale_report_and_product_or_macos_claims_fail_closed(self) -> None:
        report = build_report()
        stale = copy.deepcopy(report)
        stale["source_artifacts"][0]["sha256"] = "0" * 64
        product = copy.deepcopy(report)
        product["implementation_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, product, macos):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
