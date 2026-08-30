from __future__ import annotations

import copy
import unittest

from scripts.story_2_4_gate import (
    BLOCKERS, G_DOD_IDS, LATER_STORY_IDS, REQUIRED_TASK_MARKERS, REVIEWED_COMMIT,
    REVIEWED_PATHS, REVIEWED_TREE, build_report, task_completion, validate_raw,
    validate_report,
)


class Story24GateTests(unittest.TestCase):
    def test_all_three_bounded_acceptance_criteria_pass(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["2.4.AC1", "2.4.AC2", "2.4.AC3"],
        )
        self.assertTrue(all(
            item["status"].startswith("pass-local-public-synthetic-")
            for item in report["acceptance_criteria"]
        ))

    def test_dependency_full_protocol_and_platform_blockers_are_exact(self) -> None:
        report = build_report()
        self.assertEqual(report["status"], "blocked-open-dependencies-full-protocol-and-platform")
        self.assertEqual(report["blockers"], list(BLOCKERS))
        self.assertEqual(report["blocking_controls"], ["G-DOD-10"])
        self.assertFalse(report["story_checkbox_complete"])
        self.assertFalse(report["dependency_substitution_permitted"])
        self.assertFalse(report["platform_evidence_substituted"])

    def test_rv51_partial_evidence_cannot_be_substituted(self) -> None:
        rv51 = build_report()["rv51"]
        self.assertFalse(rv51["protocol_complete"])
        self.assertEqual(rv51["source_identity_count"], 26)
        self.assertEqual(rv51["derivative_range_identity_count"], 21)
        self.assertEqual(rv51["context_receipt_count"], 8)
        self.assertEqual(rv51["resource_observation_count"], 10)
        self.assertEqual(rv51["remaining_protocol_scenario_count"], 4)
        self.assertFalse(rv51["evidence_substitution_permitted"])

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

    def test_acceptance_review_rv51_and_blocker_mutations_fail_closed(self) -> None:
        report = build_report()
        for mutate in (
            lambda value: value["acceptance_criteria"][0].update(status="pass"),
            lambda value: value["independent_review"].update(finding_count=1),
            lambda value: value["rv51"].update(protocol_complete=True),
            lambda value: value.update(blockers=[]),
            lambda value: value.update(blocking_controls=[]),
            lambda value: value.update(later_story_dependencies=[]),
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
            ("product_parser_claim", "pass"),
            ("product_context_delivery_claim", "pass"),
            ("installed_product_claim", "pass"),
            ("product_acceptance_claim", "pass"),
            ("sprint_completion_claim", True),
            ("release_claim", "pass"),
        ):
            changed = copy.deepcopy(report)
            changed[key] = value
            self.assertTrue(validate_report(changed, verify_current=False), key)

    def test_universal_dod_later_owners_and_validator_log_are_exact(self) -> None:
        report = build_report()
        dod = report["universal_definition_of_done"]
        self.assertEqual([item["control_id"] for item in dod], list(G_DOD_IDS))
        self.assertEqual(
            [item["control_id"] for item in dod if item["status"].startswith("blocked-")],
            ["G-DOD-10"],
        )
        self.assertEqual(
            [item["story_id"] for item in report["later_story_dependencies"]],
            list(LATER_STORY_IDS),
        )
        self.assertEqual(validate_raw("\n".join((
            "Validated 16 identity-bound artifact admission fixtures",
            "Validated 10 inert hostile artifact fixtures",
            "Validated lineage for 26 sources and 21 ranges",
            "Validated 8 complete context-accounting manifests",
            "Validated 8 reconstructible context-delivery receipts",
            "Validated 10 resource ceilings and 4 cleanup boundaries",
            "Task 2.4.3.2 applicable RV-51 evidence validated with full protocol blockers open",
        ))), [])
        self.assertTrue(report["validator_results_sha256"])
        self.assertTrue(validate_raw("validation failed"))


if __name__ == "__main__":
    unittest.main()
