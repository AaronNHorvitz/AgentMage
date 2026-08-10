from __future__ import annotations

import copy
import unittest

from scripts.sprint_4_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    build_report,
    check_report,
    read_json,
    sprint_marker_failures,
    validate_report,
)


class Sprint4GateTests(unittest.TestCase):
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
            ["4.AC1", "4.AC2", "4.AC3", "4.AC4", "4.AC5"],
        )
        self.assertTrue(
            all(
                item["status"] == "pass-shared-linux"
                for item in report["acceptance_criteria"]
            )
        )

    def test_story_gate_preserves_the_macos_blocker(self) -> None:
        story = build_report()["story_gates"][0]
        self.assertEqual(story["story_id"], "4.1")
        self.assertEqual(story["status"], "blocked-macos")
        self.assertEqual(story["dod_control_ids"], list(G_DOD_IDS))
        self.assertEqual(story["dod_blocking_controls"], ["G-DOD-10"])

    def test_reviewed_commit_and_artifact_closure_are_retained(self) -> None:
        report = build_report()
        review = report["independent_review"]
        self.assertEqual(report["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(len(review["reviewed_artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)

    def test_sprint_markers_require_criteria_and_open_blocked_headings(self) -> None:
        text = "\n".join(
            [
                *[f"- [x] **Sprint AC 4.AC{index}:**" for index in range(1, 6)],
                "### [ ] Sprint 4 - Kernel Contracts and Typed Boundaries",
                "#### [ ] Story 4.1 - Kernel Contracts and Typed Boundaries",
            ]
        )
        self.assertEqual(sprint_marker_failures(text), [])
        self.assertTrue(sprint_marker_failures(text.replace("AC5", "ACX")))
        self.assertTrue(sprint_marker_failures(text.replace("### [ ]", "### [x]")))

    def test_acceptance_story_and_blocker_mutations_fail_closed(self) -> None:
        report = build_report()
        criterion = copy.deepcopy(report)
        criterion["acceptance_criteria"][0]["prohibited_observed_edge_count"] = 1
        dispatch = copy.deepcopy(report)
        dispatch["acceptance_criteria"][4]["positive_dispatch_path_available"] = True
        story = copy.deepcopy(report)
        story["story_gates"][0]["status"] = "pass"
        blocker = copy.deepcopy(report)
        blocker["summary"]["blocking_controls"] = []
        for changed in (criterion, dispatch, story, blocker):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_macos_product_authority_release_and_review_overclaims_fail_closed(self) -> None:
        report = build_report()
        macos = copy.deepcopy(report)
        macos["macos"]["status"] = "pass"
        product = copy.deepcopy(report)
        product["product_acceptance_claim"] = "pass"
        authority = copy.deepcopy(report)
        authority["positive_authority_path_claim"] = "pass"
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        external = copy.deepcopy(report)
        external["independent_review"]["external_human_review_claim"] = "complete"
        for changed in (macos, product, authority, release, external):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
