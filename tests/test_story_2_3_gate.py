from __future__ import annotations

import copy
import unittest

from scripts.story_2_3_gate import (
    BLOCKERS,
    G_DOD_IDS,
    REQUIRED_TASK_MARKERS,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    REVIEWED_TREE,
    build_report,
    task_completion,
    validate_raw,
    validate_report,
)


class Story23GateTests(unittest.TestCase):
    def test_all_three_bounded_acceptance_criteria_pass(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["2.3.AC1", "2.3.AC2", "2.3.AC3"],
        )
        self.assertTrue(all(item["status"].startswith("pass-local-") for item in report["acceptance_criteria"]))

    def test_dependency_and_platform_blockers_are_exact(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "blocked-open-dependencies-and-platform")
        self.assertEqual(report["blockers"], list(BLOCKERS))
        self.assertEqual(report["blocking_controls"], ["G-DOD-10"])
        self.assertFalse(report["story_checkbox_complete"])
        self.assertFalse(report["dependency_substitution_permitted"])
        self.assertFalse(report["platform_evidence_substituted"])

    def test_review_identity_and_artifact_closure_are_retained(self) -> None:
        review = build_report()["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(review["reviewed_tree"], REVIEWED_TREE)
        self.assertEqual(len(review["artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)

    def test_task_or_criterion_omission_is_detected(self) -> None:
        complete = "\n".join(REQUIRED_TASK_MARKERS)
        self.assertEqual(task_completion(complete), [])
        self.assertEqual(
            task_completion(complete.replace(REQUIRED_TASK_MARKERS[-1], "")),
            [REQUIRED_TASK_MARKERS[-1]],
        )

    def test_acceptance_review_and_blocker_mutations_fail_closed(self) -> None:
        report = build_report()
        changes = []
        for mutate in (
            lambda value: value["acceptance_criteria"][0].update(status="pass"),
            lambda value: value["independent_review"].update(finding_count=1),
            lambda value: value.update(blockers=[]),
            lambda value: value.update(blocking_controls=[]),
        ):
            changed = copy.deepcopy(report)
            mutate(changed)
            changes.append(changed)
        for changed in changes:
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_completion_and_product_overclaims_fail_closed(self) -> None:
        report = build_report()
        for key, value in (
            ("story_checkbox_complete", True),
            ("dependency_substitution_permitted", True),
            ("platform_evidence_substituted", True),
            ("product_runtime_claim", "pass"),
            ("installed_product_claim", "pass"),
            ("product_acceptance_claim", "pass"),
            ("sprint_completion_claim", True),
            ("release_claim", "pass"),
        ):
            changed = copy.deepcopy(report)
            changed[key] = value
            with self.subTest(key=key):
                self.assertTrue(validate_report(changed, verify_current=False))

    def test_universal_dod_closure_is_exact(self) -> None:
        dod = build_report()["universal_definition_of_done"]
        self.assertEqual([item["control_id"] for item in dod], list(G_DOD_IDS))
        self.assertEqual(
            [item["control_id"] for item in dod if item["status"].startswith("blocked-")],
            ["G-DOD-10"],
        )

    def test_validator_log_requires_every_success_marker(self) -> None:
        complete = "\n".join((
            "Validated 10 artifact golden metrics",
            "Validated 10 workflow golden metrics",
            "Validated versioned golden gate with 77 visible outcomes",
            "Validated 11 byte-identical outputs across two clean roots",
            "Sprint 2 fixtures and generated artifacts passed the security scan",
        ))
        self.assertEqual(validate_raw(complete), [])
        self.assertTrue(validate_raw(complete.replace("Validated 10 artifact golden metrics", "")))
        self.assertTrue(validate_raw(complete + "\nvalidation failed"))


if __name__ == "__main__":
    unittest.main()
