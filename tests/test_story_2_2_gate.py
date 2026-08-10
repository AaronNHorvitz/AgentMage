from __future__ import annotations

import copy
import unittest

from scripts.story_2_2_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REVIEWED_COMMIT,
    build_report,
    check_report,
    read_json,
    reviewed_artifacts,
    validate_report,
)


class Story22GateTests(unittest.TestCase):
    def test_checked_gate_report_is_current_and_blocked_only_by_macos(self) -> None:
        self.assertEqual(check_report(), [])
        report = build_report()
        self.assertEqual(read_json(REPORT_PATH), report)
        self.assertEqual(report["status"], "blocked-macos")
        self.assertEqual(report["summary"]["blocking_controls"], ["G-DOD-10"])

    def test_all_three_acceptance_criteria_pass(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["2.2.AC1", "2.2.AC2", "2.2.AC3"],
        )
        self.assertTrue(
            all(item["status"] == "pass" for item in report["acceptance_criteria"])
        )

    def test_reviewed_artifacts_bind_the_pushed_evidence_commit(self) -> None:
        report = build_report()
        artifacts = reviewed_artifacts()
        self.assertEqual(report["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(report["independent_review"]["reviewed_artifacts"], artifacts)
        self.assertGreater(len(artifacts), 20)

    def test_acceptance_failure_and_blocker_omission_fail_closed(self) -> None:
        report = build_report()
        failed = copy.deepcopy(report)
        failed["acceptance_criteria"][0]["status"] = "fail"
        omitted = copy.deepcopy(report)
        omitted["summary"]["blocking_controls"] = []
        self.assertTrue(validate_report(failed))
        self.assertTrue(validate_report(omitted))

    def test_definition_of_done_omission_and_wrong_blocker_fail_closed(self) -> None:
        report = build_report()
        omitted = copy.deepcopy(report)
        omitted["universal_definition_of_done"].pop()
        wrong = copy.deepcopy(report)
        wrong["universal_definition_of_done"][0]["status"] = "blocked-macos"
        self.assertTrue(validate_report(omitted))
        self.assertTrue(validate_report(wrong))
        self.assertEqual(
            [item["control_id"] for item in report["universal_definition_of_done"]],
            list(G_DOD_IDS),
        )

    def test_review_boundary_and_external_human_overclaims_fail_closed(self) -> None:
        report = build_report()
        commit = copy.deepcopy(report)
        commit["reviewed_commit"] = "0" * 40
        external = copy.deepcopy(report)
        external["independent_review"]["external_human_review_claim"] = "complete"
        self.assertTrue(validate_report(commit))
        self.assertTrue(validate_report(external))

    def test_macos_product_and_release_overclaims_fail_closed(self) -> None:
        report = build_report()
        macos = copy.deepcopy(report)
        macos["macos"]["status"] = "pass"
        product = copy.deepcopy(report)
        product["product_acceptance_claim"] = "pass"
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        for changed in (macos, product, release):
            self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
