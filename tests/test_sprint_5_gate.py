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
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_all_six_sprint_criteria_pass_current_linux_scope(self) -> None:
        criteria = self.report["acceptance_criteria"]
        self.assertEqual([item["criterion_id"] for item in criteria], [f"5.AC{i}" for i in range(1, 7)])
        self.assertEqual(criteria[0]["admitted_mutation_count"], 0)
        self.assertEqual(criteria[1]["replay_success_count"], 0)
        self.assertEqual(criteria[3]["admitted_authority_count"], 0)
        self.assertEqual(criteria[4]["grant_operation_count"], 22)
        self.assertEqual(criteria[5]["operation_failure_pair_count"], 308)
        self.assertFalse(criteria[5]["uncertain_outcome_retried"])

    def test_all_three_story_gates_preserve_dependencies_and_platform(self) -> None:
        stories = self.report["story_gates"]
        self.assertEqual([item["story_id"] for item in stories], ["5.1", "5.2", "5.3"])
        self.assertTrue(all(item["current_linux_scope_complete"] for item in stories))
        self.assertTrue(all(item["blocking_controls"] == ["G-DOD-10"] for item in stories))
        self.assertEqual(self.report["summary"]["blocking_story_ids"], ["5.1", "5.2", "5.3"])

    def test_reviewed_commit_and_artifact_closure_are_retained(self) -> None:
        review = self.report["independent_review"]
        self.assertEqual(self.report["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(len(review["reviewed_artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)

    def test_checklist_requires_all_story_and_sprint_criteria_and_open_headings(self) -> None:
        complete = "\n".join((
            *[f"- [x] **Story AC 5.1.AC{i}:**" for i in range(1, 3)],
            *[f"- [x] **Story AC 5.2.AC{i}:**" for i in range(1, 4)],
            *[f"- [x] **Story AC 5.3.AC{i}:**" for i in range(1, 4)],
            *[f"- [x] **Sprint AC 5.AC{i}:**" for i in range(1, 7)],
            "### [ ] Sprint 5 - Capability Grants and Policy Engine",
            "#### [ ] Story 5.1 - Capability Grants and Policy Engine",
            "#### [ ] Story 5.2 - Side-Effect, Idempotency, and Retry Policy",
            "#### [ ] Story 5.3 - Verified Workflow Definition and Completion Authority",
        ))
        self.assertEqual(checklist_failures(complete), [])
        self.assertTrue(checklist_failures(complete.replace("5.AC6", "5.ACX")))
        self.assertTrue(checklist_failures(complete.replace("5.3.AC3", "5.3.ACX")))
        self.assertTrue(checklist_failures(complete.replace("### [ ]", "### [x]")))

    def test_acceptance_story_and_blocker_mutations_fail_closed(self) -> None:
        criterion = copy.deepcopy(self.report)
        criterion["acceptance_criteria"][5]["uncertain_outcome_retried"] = True
        story = copy.deepcopy(self.report)
        story["story_gates"].pop()
        blocker = copy.deepcopy(self.report)
        blocker["summary"]["open_dependency_ids"] = []
        for changed in (criterion, story, blocker):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_platform_product_release_and_review_overclaims_fail_closed(self) -> None:
        changes = []
        for key, value in (
            ("installed_product_claim", "pass"),
            ("product_requirement_completion_claim", "complete"),
            ("product_acceptance_claim", "pass"),
            ("release_claim", "pass"),
        ):
            changed = copy.deepcopy(self.report)
            changed[key] = value
            changes.append(changed)
        platform = copy.deepcopy(self.report)
        platform["platform"]["status"] = "pass"
        changes.append(platform)
        external = copy.deepcopy(self.report)
        external["independent_review"]["external_human_review_status"] = "pass"
        changes.append(external)
        for changed in changes:
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_definition_of_done_closure_is_exact(self) -> None:
        dod = self.report["universal_definition_of_done"]
        self.assertEqual(dod["control_ids"], list(G_DOD_IDS))
        self.assertEqual(dod["story_count"], 3)
        self.assertEqual(dod["blocking_controls"], ["G-DOD-10"])


if __name__ == "__main__":
    unittest.main()
