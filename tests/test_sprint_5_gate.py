from __future__ import annotations

import copy
import unittest

from scripts.sprint_5_gate import (
    G_DOD_IDS,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    build_report,
    checklist_failures,
    validate_report,
)


class Sprint5GateTests(unittest.TestCase):
    def test_all_five_sprint_criteria_pass_shared_linux_scopes(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["5.AC1", "5.AC2", "5.AC3", "5.AC4", "5.AC5"],
        )
        self.assertEqual(report["acceptance_criteria"][0]["admitted_mutation_count"], 0)
        self.assertEqual(report["acceptance_criteria"][1]["replay_success_count"], 0)
        self.assertEqual(report["acceptance_criteria"][3]["admitted_authority_count"], 0)

    def test_story_and_sprint_preserve_the_macos_blocker(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "blocked-macos")
        self.assertEqual(report["summary"]["blocking_controls"], ["G-DOD-10"])
        self.assertEqual(report["story_gates"][0]["status"], "blocked-macos")
        self.assertFalse(report["macos"]["execution_performed"])

    def test_reviewed_commit_and_artifact_closure_are_retained(self) -> None:
        report = build_report()
        review = report["independent_review"]
        self.assertEqual(report["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(len(review["reviewed_artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)

    def test_checklist_requires_story_criteria_and_open_blocked_headings(self) -> None:
        text = "\n".join(
            (
                "- [x] **Story AC 5.1.AC1:**",
                "- [x] **Story AC 5.1.AC2:**",
                *[
                    f"- [x] **Sprint AC 5.AC{index}:**"
                    for index in range(1, 6)
                ],
                "### [ ] Sprint 5 - Capability Grants and Policy Engine",
                "#### [ ] Story 5.1 - Capability Grants and Policy Engine",
            )
        )
        self.assertEqual(checklist_failures(text), [])
        self.assertTrue(checklist_failures(text.replace("AC2", "ACX")))
        self.assertTrue(checklist_failures(text.replace("5.AC5", "5.ACX")))
        self.assertTrue(checklist_failures(text.replace("### [ ]", "### [x]")))

    def test_acceptance_story_and_blocker_mutations_fail_closed(self) -> None:
        report = build_report()
        criterion = copy.deepcopy(report)
        criterion["acceptance_criteria"][0]["admitted_mutation_count"] = 1
        replay = copy.deepcopy(report)
        replay["acceptance_criteria"][1]["replay_success_count"] = 1
        story = copy.deepcopy(report)
        story["story_gates"][0]["status"] = "pass"
        blocker = copy.deepcopy(report)
        blocker["summary"]["blocking_controls"] = []
        for changed in (criterion, replay, story, blocker):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_macos_product_release_and_review_overclaims_fail_closed(self) -> None:
        report = build_report()
        macos = copy.deepcopy(report)
        macos["macos"]["status"] = "pass"
        product = copy.deepcopy(report)
        product["product_acceptance_claim"] = "pass"
        requirement = copy.deepcopy(report)
        requirement["product_requirement_completion_claim"] = "complete"
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        external = copy.deepcopy(report)
        external["independent_review"]["external_human_review_status"] = "pass"
        for changed in (macos, product, requirement, release, external):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_definition_of_done_closure_is_exact(self) -> None:
        dod = build_report()["universal_definition_of_done"]
        self.assertEqual(dod["control_ids"], list(G_DOD_IDS))
        self.assertEqual(dod["blocking_controls"], ["G-DOD-10"])


if __name__ == "__main__":
    unittest.main()
