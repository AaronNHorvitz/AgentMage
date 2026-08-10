from __future__ import annotations

import copy
import unittest

from scripts.emergency_disable_policy import (
    FIXTURE_PATH,
    REPORT_PATH,
    SUBJECT_KINDS,
    build_report,
    check_artifact,
    evaluate_subject,
    read_json,
    validate_policy,
    validate_report,
)


class EmergencyDisablePolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = read_json(FIXTURE_PATH)

    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_all_five_subject_classes_block_exact_matches(self) -> None:
        self.assertEqual(validate_policy(self.fixture), [])
        self.assertEqual(
            tuple(entry["subject_kind"] for entry in self.fixture["entries"]),
            SUBJECT_KINDS,
        )
        for entry in self.fixture["entries"]:
            result = evaluate_subject(
                self.fixture,
                entry["subject_kind"],
                entry["subject_id"],
                entry["subject_sha256"],
            )
            self.assertEqual(result["outcome"], "blocked")
            self.assertEqual(result["matched_entry_id"], entry["entry_id"])

    def test_hash_mismatch_and_unknown_subject_do_not_false_match(self) -> None:
        mismatch = evaluate_subject(
            self.fixture,
            "model-artifact",
            self.fixture["entries"][0]["subject_id"],
            "f" * 64,
        )
        unknown = evaluate_subject(self.fixture, "component", "unknown", None)
        for result in (mismatch, unknown):
            self.assertEqual(result["outcome"], "continue-without-authority")
            self.assertIsNone(result["matched_entry_id"])

    def test_signer_sequence_and_validity_mutations_fail_closed(self) -> None:
        signer = copy.deepcopy(self.fixture)
        signer["signing"]["detached_signatures"][0]["signer_key_id"] = "unknown"
        threshold = copy.deepcopy(self.fixture)
        threshold["signing"]["threshold"] = 2
        sequence = copy.deepcopy(self.fixture)
        sequence["installation"]["sequence_must_increase"] = False
        validity = copy.deepcopy(self.fixture)
        validity["policy"]["expires_at"] = validity["policy"]["not_before"]
        for changed in (signer, threshold, sequence, validity):
            self.assertTrue(validate_policy(changed))

    def test_network_remote_receipt_and_recovery_mutations_fail_closed(self) -> None:
        network = copy.deepcopy(self.fixture)
        network["installation"]["network_allowed"] = True
        remote = copy.deepcopy(self.fixture)
        remote["installation"]["remote_control"] = True
        receipt = copy.deepcopy(self.fixture)
        receipt["receipt_policy"]["prohibited_fields"].remove("credential")
        recovery = copy.deepcopy(self.fixture)
        recovery["recovery"]["deletion_silently_unblocks"] = True
        for changed in (network, remote, receipt, recovery):
            self.assertTrue(validate_policy(changed))

    def test_authority_product_release_and_macos_overclaims_fail_closed(self) -> None:
        authority = copy.deepcopy(self.fixture)
        authority["evaluation"]["may_create_authority"] = True
        product = copy.deepcopy(self.fixture)
        product["claims"]["implemented_installer"] = True
        activation = copy.deepcopy(self.fixture)
        activation["claims"]["product_activation"] = True
        macos = copy.deepcopy(self.fixture)
        macos["claims"]["macos_support"] = True
        for changed in (authority, product, activation, macos):
            self.assertTrue(validate_policy(changed))

    def test_report_staleness_data_network_and_macos_claims_fail_closed(self) -> None:
        report = build_report()
        stale = copy.deepcopy(report)
        stale["source_artifacts"][0]["sha256"] = "0" * 64
        private = copy.deepcopy(report)
        private["private_user_data_used"] = True
        network = copy.deepcopy(report)
        network["network_used"] = True
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (stale, private, network, macos):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
