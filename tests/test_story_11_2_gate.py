from __future__ import annotations

import copy
import unittest

from scripts.story_11_2_gate import (
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


class Story112GateTests(unittest.TestCase):
    def test_all_criteria_pass_their_bounded_current_scopes(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["11.2.AC1", "11.2.AC2", "11.2.AC3"],
        )
        self.assertTrue(all(item["status"].startswith("pass-local-current-") for item in report["acceptance_criteria"]))

    def test_gate_preserves_exact_dependencies_and_blockers(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "blocked-open-dependencies-and-platform-integration")
        self.assertEqual(report["blockers"], list(BLOCKERS))
        self.assertFalse(report["story_checkbox_complete"])
        self.assertFalse(report["dependency_substitution_permitted"])

    def test_review_identity_and_artifact_closure_are_exact(self) -> None:
        review = build_report()["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(review["reviewed_tree"], REVIEWED_TREE)
        self.assertEqual(len(review["artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)

    def test_task_or_criterion_omission_is_detected(self) -> None:
        complete = "\n".join(REQUIRED_TASK_MARKERS)
        self.assertEqual(task_completion(complete), [])
        self.assertEqual(task_completion(complete.replace(REQUIRED_TASK_MARKERS[-1], "")), [REQUIRED_TASK_MARKERS[-1]])

    def test_validator_results_require_all_three_criteria(self) -> None:
        from scripts.story_11_2_gate import VALIDATOR_MARKERS

        valid = "\n".join(VALIDATOR_MARKERS)
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(validate_raw(valid.replace(VALIDATOR_MARKERS[0], "")))
        self.assertTrue(validate_raw(f"{valid}\nTraceback"))

    def test_acceptance_dod_review_and_blocker_mutations_fail_closed(self) -> None:
        report = build_report()
        changes = []
        for path, value in (
            (("acceptance_criteria", 0, "status"), "pass"),
            (("universal_definition_of_done", 9, "status"), "pass"),
            (("independent_review", "finding_count"), 1),
            (("blockers",), []),
        ):
            changed = copy.deepcopy(report)
            target = changed
            for key in path[:-1]:
                target = target[key]
            target[path[-1]] = value
            changes.append(changed)
        for changed in changes:
            self.assertTrue(validate_report(changed, verify_current=False))

    def test_product_platform_sprint_and_release_overclaims_fail_closed(self) -> None:
        report = build_report()
        changes = []
        for key, value in (
            ("story_checkbox_complete", True),
            ("dependency_substitution_permitted", True),
            ("platform_evidence_substituted", True),
            ("installed_product_claim", "pass"),
            ("product_acceptance_claim", "pass"),
            ("sprint_completion_claim", True),
            ("release_claim", "pass"),
        ):
            changed = copy.deepcopy(report)
            changed[key] = value
            changes.append(changed)
        for changed in changes:
            self.assertTrue(validate_report(changed, verify_current=False))

    def test_applicable_definition_of_done_control_set_is_exact(self) -> None:
        report = build_report()
        dod = report["universal_definition_of_done"]
        self.assertEqual([item["control_id"] for item in dod], list(G_DOD_IDS))
        self.assertEqual(report["blocking_controls"], ["G-DOD-10", "G-DOD-22"])


if __name__ == "__main__":
    unittest.main()
