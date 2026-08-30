from __future__ import annotations

import copy
import unittest

from scripts.story_5_1_gate import (
    G_DOD_IDS,
    REQUIRED_TASK_MARKERS,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    REVIEWED_TREE,
    build_report,
    task_completion,
    validate_report,
)


class Story51GateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_both_acceptance_criteria_pass_their_bounded_linux_scopes(self) -> None:
        report = self.report
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["5.1.AC1", "5.1.AC2"],
        )
        self.assertEqual(report["acceptance_criteria"][0]["mutation_seed_count"], 560)
        self.assertEqual(report["acceptance_criteria"][0]["replay_success_count"], 0)
        self.assertEqual(report["acceptance_criteria"][1]["admitted_authority_count"], 0)

    def test_gate_is_blocked_only_by_macos_without_substitution(self) -> None:
        report = self.report
        self.assertEqual(report["status"], "blocked-macos")
        self.assertTrue(report["shared_linux_story_work_complete"])
        self.assertFalse(report["story_checkbox_complete"])
        self.assertEqual(report["only_blocker"], "macos-execution-evidence-unavailable")
        self.assertFalse(report["macos_evidence_substituted"])

    def test_review_identity_and_artifact_closure_are_retained(self) -> None:
        review = self.report["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(review["reviewed_tree"], REVIEWED_TREE)
        self.assertEqual(len(review["artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)
        self.assertEqual(review["external_human_review_status"], "not-performed")

    def test_task_omission_is_detected(self) -> None:
        complete = "\n".join(REQUIRED_TASK_MARKERS)
        self.assertEqual(task_completion(complete), [])
        self.assertEqual(
            task_completion(complete.replace(REQUIRED_TASK_MARKERS[-1], "")),
            [REQUIRED_TASK_MARKERS[-1]],
        )

    def test_acceptance_security_and_review_mutations_fail_closed(self) -> None:
        report = self.report
        acceptance = copy.deepcopy(report)
        acceptance["acceptance_criteria"][0]["admitted_mutation_count"] = 1
        authority = copy.deepcopy(report)
        authority["acceptance_criteria"][1]["admitted_authority_count"] = 1
        security = copy.deepcopy(report)
        security["security_mapping"]["product_requirements_complete"] = 8
        review = copy.deepcopy(report)
        review["independent_review"]["finding_count"] = 1
        for changed in (acceptance, authority, security, review):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_product_release_external_review_and_macos_overclaims_fail_closed(self) -> None:
        report = self.report
        changes = []
        for key, value in (
            ("product_requirement_completion_claim", "complete"),
            ("product_acceptance_claim", "pass"),
            ("release_claim", "pass"),
            ("macos_execution_status", "pass"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(report)
            changed[key] = value
            changes.append(changed)
        external = copy.deepcopy(report)
        external["independent_review"]["external_human_review_status"] = "pass"
        changes.append(external)
        for changed in changes:
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_universal_dod_closure_is_exact(self) -> None:
        dod = self.report["universal_definition_of_done"]
        self.assertEqual([item["control_id"] for item in dod], list(G_DOD_IDS))
        self.assertEqual(
            [item["control_id"] for item in dod if item["status"] == "blocked-macos"],
            ["G-DOD-10"],
        )


if __name__ == "__main__":
    unittest.main()
