from __future__ import annotations

import copy
import unittest

from scripts.manual_patch_verifier import (
    CASES_PATH,
    EXPECTED_CASES,
    FIXTURE_ROOT,
    REPORT_PATH,
    build_report,
    check_artifact,
    read_json,
    run_case,
    validate_report,
)


class ManualPatchVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = read_json(CASES_PATH)
        self.trust = {
            "agentmage-release-key-1-fixture": FIXTURE_ROOT
            / "trust/authorized-public.pem"
        }

    def test_checked_report_is_current_and_exact(self) -> None:
        self.assertEqual(check_artifact(), [])
        self.assertEqual(read_json(REPORT_PATH), build_report())

    def test_all_required_cases_reach_their_exact_outcomes(self) -> None:
        self.assertEqual(
            tuple(
                (case["case_id"], case["expected_outcome"])
                for case in self.manifest["cases"]
            ),
            EXPECTED_CASES,
        )
        for case in self.manifest["cases"]:
            result = run_case(case, self.trust)
            self.assertEqual(result["status"], "pass", case["case_id"])
            self.assertEqual(result["actual_outcome"], case["expected_outcome"])

    def test_valid_case_uses_real_signature_and_all_artifact_hashes(self) -> None:
        result = run_case(self.manifest["cases"][0], self.trust)
        self.assertEqual(result["actual_outcome"], "verified-can-proceed")

    def test_interruption_and_rollback_preserve_exact_prior_state(self) -> None:
        by_id = {case["case_id"]: case for case in self.manifest["cases"]}
        self.assertEqual(
            run_case(by_id["interrupted"], self.trust)["actual_outcome"],
            "interrupted-prior-preserved",
        )
        self.assertEqual(
            run_case(by_id["rollback"], self.trust)["actual_outcome"],
            "rollback-complete-prior-restored",
        )

    def test_case_reordering_omission_and_expected_outcome_drift_fail_closed(self) -> None:
        report = build_report()
        reordered = copy.deepcopy(report)
        reordered["cases"].reverse()
        omitted = copy.deepcopy(report)
        omitted["cases"].pop()
        drifted = copy.deepcopy(report)
        drifted["cases"][0]["expected_outcome"] = "pass"
        for changed in (reordered, omitted, drifted):
            self.assertTrue(validate_report(changed))

    def test_source_signature_and_verification_engine_drift_fail_closed(self) -> None:
        report = build_report()
        source = copy.deepcopy(report)
        source["source_artifacts"][0]["sha256"] = "0" * 64
        engine = copy.deepcopy(report)
        engine["verification_engine"]["executable_sha256"] = "0" * 64
        for changed in (source, engine):
            self.assertTrue(validate_report(changed))

    def test_product_release_network_and_macos_overclaims_fail_closed(self) -> None:
        report = build_report()
        product = copy.deepcopy(report)
        product["product_verifier_claim"] = "pass"
        activation = copy.deepcopy(report)
        activation["product_activation_claim"] = "pass"
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        network = copy.deepcopy(report)
        network["network_used"] = True
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        for changed in (product, activation, release, network, macos):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
