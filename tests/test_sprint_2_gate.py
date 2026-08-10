from __future__ import annotations

import copy
import unittest

from scripts.sprint_2_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REVIEWED_COMMIT,
    build_report,
    check_report,
    read_json,
    validate_report,
)


class Sprint2GateTests(unittest.TestCase):
    def test_checked_sprint_gate_is_current_and_blocked_only_by_macos(self) -> None:
        self.assertEqual(check_report(), [])
        report = build_report()
        self.assertEqual(read_json(REPORT_PATH), report)
        self.assertEqual(report["status"], "blocked-macos")
        self.assertEqual(report["summary"]["blocking_controls"], ["G-DOD-10"])

    def test_all_five_sprint_acceptance_criteria_pass(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["2.AC1", "2.AC2", "2.AC3", "2.AC4", "2.AC5"],
        )
        self.assertTrue(
            all(item["status"] == "pass" for item in report["acceptance_criteria"])
        )

    def test_both_story_gates_preserve_the_same_macos_blocker(self) -> None:
        report = build_report()
        self.assertEqual([item["story_id"] for item in report["story_gates"]], ["2.1", "2.2"])
        self.assertTrue(
            all(item["status"] == "blocked-macos" for item in report["story_gates"])
        )
        self.assertTrue(
            all(item["dod_control_ids"] == list(G_DOD_IDS) for item in report["story_gates"])
        )

    def test_reviewed_commit_and_result_identities_are_retained(self) -> None:
        report = build_report()
        platform = report["acceptance_criteria"][3]
        self.assertEqual(report["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(platform["result_identity_count"], 2)
        self.assertTrue(
            all(
                set(item)
                >= {
                    "fixture_sha256",
                    "build_sha256",
                    "model_sha256",
                    "runtime_sha256",
                    "policy_sha256",
                }
                for item in platform["result_identities"]
            )
        )

    def test_acceptance_and_story_status_mutations_fail_closed(self) -> None:
        report = build_report()
        criterion = copy.deepcopy(report)
        criterion["acceptance_criteria"][0]["status"] = "fail"
        story = copy.deepcopy(report)
        story["story_gates"][0]["status"] = "pass"
        self.assertTrue(validate_report(criterion))
        self.assertTrue(validate_report(story))

    def test_blocker_omission_and_external_review_overclaim_fail_closed(self) -> None:
        report = build_report()
        blocker = copy.deepcopy(report)
        blocker["summary"]["blocking_controls"] = []
        external = copy.deepcopy(report)
        external["independent_review"]["external_human_review_claim"] = "complete"
        self.assertTrue(validate_report(blocker))
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
