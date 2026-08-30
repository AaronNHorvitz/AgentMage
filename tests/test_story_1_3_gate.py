from __future__ import annotations

import copy
import unittest

from scripts.story_1_3_gate import (
    BLOCKERS, G_DOD_IDS, REQUIRED_TASK_MARKERS, REVIEWED_COMMIT, REVIEWED_PATHS,
    REVIEWED_TREE, build_report, task_completion, validate_raw, validate_report,
)


class Story13GateTests(unittest.TestCase):
    def test_all_three_bounded_acceptance_criteria_pass(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["1.3.AC1", "1.3.AC2", "1.3.AC3"],
        )
        self.assertTrue(all(item["status"].startswith("pass-local-current-") for item in report["acceptance_criteria"]))

    def test_later_story_and_platform_blockers_are_exact(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "blocked-open-later-stories-and-platform")
        self.assertEqual(report["blockers"], list(BLOCKERS))
        self.assertEqual(report["blocking_controls"], ["G-DOD-10"])
        self.assertFalse(report["story_checkbox_complete"])
        self.assertFalse(report["dependency_substitution_permitted"])
        self.assertFalse(report["platform_evidence_substituted"])

    def test_rv50_partial_evidence_cannot_be_substituted(self) -> None:
        rv50 = build_report()["rv50"]
        self.assertFalse(rv50["protocol_complete"])
        self.assertEqual(rv50["later_runtime_scenario_count"], 5)
        self.assertEqual(rv50["blocked_external_tuple_count"], 5)
        self.assertFalse(rv50["evidence_substitution_permitted"])

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

    def test_acceptance_review_rv50_and_blocker_mutations_fail_closed(self) -> None:
        report = build_report()
        for mutate in (
            lambda value: value["acceptance_criteria"][0].update(status="pass"),
            lambda value: value["independent_review"].update(finding_count=1),
            lambda value: value["rv50"].update(protocol_complete=True),
            lambda value: value.update(blockers=[]),
            lambda value: value.update(blocking_controls=[]),
        ):
            changed = copy.deepcopy(report)
            mutate(changed)
            self.assertTrue(validate_report(changed, verify_current=False))

    def test_completion_and_product_overclaims_fail_closed(self) -> None:
        report = build_report()
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
            self.assertTrue(validate_report(changed, verify_current=False), key)

    def test_universal_dod_and_validator_log_are_exact(self) -> None:
        report = build_report()
        dod = report["universal_definition_of_done"]
        self.assertEqual([item["control_id"] for item in dod], list(G_DOD_IDS))
        self.assertEqual(
            [item["control_id"] for item in dod if item["status"].startswith("blocked-")],
            ["G-DOD-10"],
        )
        valid = "\n".join((
            "Story acceptance criterion 1.3.AC1 Rust-owned boundary meaning validated",
            "Story acceptance criterion 1.3.AC2 cross-client identity parity validated",
            "Story acceptance criterion 1.3.AC3 pre-dispatch rejection validated",
            "Task 1.3.3.2 applicable RV-50 evidence validated with external blockers open",
        ))
        self.assertEqual(validate_raw(valid), [])
        self.assertTrue(report["validator_results_sha256"])
        self.assertTrue(validate_raw("validation failed"))


if __name__ == "__main__":
    unittest.main()
