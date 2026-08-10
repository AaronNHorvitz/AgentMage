from __future__ import annotations

import copy
import unittest

from scripts.sprint_3_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    build_report,
    check_report,
    read_json,
    validate_report,
)


class Sprint3GateTests(unittest.TestCase):
    def test_checked_sprint_gate_is_current_and_blocked_only_by_macos(self) -> None:
        self.assertEqual(check_report(), [])
        report = build_report()
        self.assertEqual(read_json(REPORT_PATH), report)
        self.assertEqual(report["status"], "blocked-macos")
        self.assertEqual(report["summary"]["blocking_controls"], ["G-DOD-10"])

    def test_all_five_sprint_acceptance_criteria_pass_shared_linux(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["3.AC1", "3.AC2", "3.AC3", "3.AC4", "3.AC5"],
        )
        self.assertTrue(
            all(
                item["status"] == "pass-shared-linux"
                for item in report["acceptance_criteria"]
            )
        )
        self.assertGreaterEqual(
            report["acceptance_criteria"][0]["executed_mutation_attempts"], 100
        )

    def test_both_story_gates_preserve_the_same_macos_blocker(self) -> None:
        stories = build_report()["story_gates"]
        self.assertEqual([item["story_id"] for item in stories], ["3.1", "3.2"])
        self.assertTrue(all(item["status"] == "blocked-macos" for item in stories))
        self.assertTrue(
            all(item["dod_control_ids"] == list(G_DOD_IDS) for item in stories)
        )
        self.assertTrue(
            all(item["dod_blocking_controls"] == ["G-DOD-10"] for item in stories)
        )

    def test_reviewed_commit_and_artifact_closure_are_retained(self) -> None:
        report = build_report()
        review = report["independent_review"]
        self.assertEqual(report["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(len(review["reviewed_artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)

    def test_acceptance_and_story_status_mutations_fail_closed(self) -> None:
        report = build_report()
        criterion = copy.deepcopy(report)
        criterion["acceptance_criteria"][0]["executed_mutation_attempts"] = 99
        secrets = copy.deepcopy(report)
        secrets["acceptance_criteria"][4]["raw_configuration_persisted"] = True
        story = copy.deepcopy(report)
        story["story_gates"][0]["status"] = "pass"
        for changed in (criterion, secrets, story):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed))

    def test_blocker_omission_and_external_review_overclaim_fail_closed(self) -> None:
        report = build_report()
        blocker = copy.deepcopy(report)
        blocker["summary"]["blocking_controls"] = []
        dod = copy.deepcopy(report)
        dod["universal_definition_of_done"]["blocking_controls"] = []
        external = copy.deepcopy(report)
        external["independent_review"]["external_human_review_claim"] = "complete"
        for changed in (blocker, dod, external):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed))

    def test_macos_product_startup_and_release_overclaims_fail_closed(self) -> None:
        report = build_report()
        macos = copy.deepcopy(report)
        macos["macos"]["status"] = "pass"
        startup = copy.deepcopy(report)
        startup["product_startup_claim"] = "pass"
        product = copy.deepcopy(report)
        product["product_acceptance_claim"] = "pass"
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        for changed in (macos, startup, product, release):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed))


if __name__ == "__main__":
    unittest.main()
