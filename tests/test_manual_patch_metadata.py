from __future__ import annotations

import copy
import unittest

from scripts.manual_patch_metadata import (
    ARTIFACT_IDS,
    FIXTURE_PATH,
    REPORT_PATH,
    build_report,
    check_artifact,
    read_json,
    validate_metadata,
    validate_report,
)


class ManualPatchMetadataTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = read_json(FIXTURE_PATH)

    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_canonical_fixture_is_closed_synthetic_and_non_releasable(self) -> None:
        self.assertEqual(validate_metadata(self.fixture), [])
        self.assertEqual(
            self.fixture["fixture_status"],
            "synthetic-contract-fixture-not-releasable",
        )
        self.assertEqual(
            tuple(item["artifact_id"] for item in self.fixture["artifacts"]),
            ARTIFACT_IDS,
        )

    def test_downgrade_signer_and_checksum_mutations_fail_closed(self) -> None:
        downgrade = copy.deepcopy(self.fixture)
        downgrade["release"]["release_sequence"] = 1
        signer = copy.deepcopy(self.fixture)
        signer["signing"]["detached_signatures"][0]["signer_key_id"] = "unknown"
        threshold = copy.deepcopy(self.fixture)
        threshold["signing"]["threshold"] = 2
        checksum = copy.deepcopy(self.fixture)
        checksum["artifacts"][0]["sha256"] = checksum["artifacts"][1]["sha256"]
        for changed in (downgrade, signer, threshold, checksum):
            self.assertTrue(validate_metadata(changed))

    def test_provenance_schema_and_rollback_mutations_fail_closed(self) -> None:
        provenance = copy.deepcopy(self.fixture)
        provenance["provenance"]["source_commit"] = "2" * 40
        schema = copy.deepcopy(self.fixture)
        schema["schema_impact"]["later_user_data_overwrite_allowed"] = True
        rollback = copy.deepcopy(self.fixture)
        rollback["rollback"]["target_revoked"] = True
        precondition = copy.deepcopy(self.fixture)
        precondition["rollback"]["exact_active_precondition_required"] = False
        for changed in (provenance, schema, rollback, precondition):
            self.assertTrue(validate_metadata(changed))

    def test_revocation_support_and_delivery_mutations_fail_closed(self) -> None:
        revoked = copy.deepcopy(self.fixture)
        revoked["revocation"]["revoked_release_ids"] = [
            revoked["release"]["release_id"]
        ]
        support = copy.deepcopy(self.fixture)
        support["support"]["target_release_state"] = "end-of-support"
        network = copy.deepcopy(self.fixture)
        network["delivery"]["transaction_network_allowed"] = True
        remote = copy.deepcopy(self.fixture)
        remote["delivery"]["remote_trigger"] = True
        for changed in (revoked, support, network, remote):
            self.assertTrue(validate_metadata(changed))

    def test_activation_product_release_and_macos_overclaims_fail_closed(self) -> None:
        activation = copy.deepcopy(self.fixture)
        activation["activation"]["explicit_user_approval_required"] = False
        product = copy.deepcopy(self.fixture)
        product["claims"]["implemented_verifier"] = True
        release = copy.deepcopy(self.fixture)
        release["claims"]["releasable_package"] = True
        macos = copy.deepcopy(self.fixture)
        macos["claims"]["macos_support"] = True
        for changed in (activation, product, release, macos):
            self.assertTrue(validate_metadata(changed))

    def test_report_staleness_network_release_and_macos_claims_fail_closed(self) -> None:
        report = build_report()
        stale = copy.deepcopy(report)
        stale["source_artifacts"][0]["sha256"] = "0" * 64
        network = copy.deepcopy(report)
        network["network_used"] = True
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, network, release, macos):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
