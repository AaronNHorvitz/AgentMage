from __future__ import annotations

import copy
import unittest

from scripts.story_4_1_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REQUIRED_TASK_MARKERS,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    REVIEWED_TREE,
    build_report,
    check_report,
    read_json,
    task_completion,
    validate_report,
)


class Story41GateTests(unittest.TestCase):
    def test_checked_gate_is_current_and_blocked_only_by_macos(self) -> None:
        self.assertEqual(check_report(), [])
        report = read_json(REPORT_PATH)
        self.assertEqual(report, build_report())
        self.assertEqual(report["only_blocker"], "macos-execution-evidence-unavailable")

    def test_both_acceptance_criteria_pass_shared_linux(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["4.1.AC1", "4.1.AC2"],
        )
        self.assertTrue(
            all(
                item["status"] == "pass-shared-linux"
                for item in report["acceptance_criteria"]
            )
        )
        self.assertEqual(report["acceptance_criteria"][1]["valid_fixture_count"], 15)
        self.assertEqual(
            report["acceptance_criteria"][1]["persisted_invalid_fixture_count"], 8
        )

    def test_architecture_acceptance_is_bounded_to_the_contract_layer(self) -> None:
        acceptance = build_report()["architecture_acceptance"]
        self.assertEqual(acceptance["acceptance_test_id"], "AT-ARCH-001")
        self.assertEqual(acceptance["scope"], "implemented-contract-layer")
        self.assertEqual(acceptance["prohibited_observed_edge_count"], 0)
        self.assertEqual(acceptance["runtime_boundary_trace_count"], 5)
        self.assertEqual(acceptance["product_wide_acceptance_claim"], "none")

    def test_review_identity_and_artifact_closure_are_retained(self) -> None:
        review = build_report()["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(review["reviewed_tree"], REVIEWED_TREE)
        self.assertEqual(len(review["artifacts"]), len(REVIEWED_PATHS))
        self.assertEqual(review["finding_count"], 0)

    def test_task_omission_is_detected(self) -> None:
        complete = "\n".join(REQUIRED_TASK_MARKERS)
        self.assertEqual(task_completion(complete), [])
        self.assertEqual(
            task_completion(complete.replace(REQUIRED_TASK_MARKERS[-1], "")),
            [REQUIRED_TASK_MARKERS[-1]],
        )

    def test_acceptance_dod_security_and_architecture_mutations_fail_closed(self) -> None:
        report = build_report()
        acceptance = copy.deepcopy(report)
        acceptance["acceptance_criteria"][0]["status"] = "pass"
        dod = copy.deepcopy(report)
        dod["universal_definition_of_done"][0]["status"] = "blocked-macos"
        security = copy.deepcopy(report)
        security["security_mapping"]["product_requirements_complete"] = 7
        architecture = copy.deepcopy(report)
        architecture["architecture_acceptance"]["product_wide_acceptance_claim"] = "pass"
        for changed in (acceptance, dod, security, architecture):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed))

    def test_product_authority_release_and_macos_overclaims_fail_closed(self) -> None:
        report = build_report()
        changes = []
        for key, value in (
            ("product_acceptance_claim", "pass"),
            ("positive_authority_path_claim", "pass"),
            ("release_claim", "pass"),
            ("macos_execution_status", "pass"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(report)
            changed[key] = value
            changes.append(changed)
        for changed in changes:
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed))

    def test_universal_dod_closure_is_exact(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["control_id"] for item in report["universal_definition_of_done"]],
            list(G_DOD_IDS),
        )
        self.assertEqual(
            [
                item["control_id"]
                for item in report["universal_definition_of_done"]
                if item["status"] == "blocked-macos"
            ],
            ["G-DOD-10"],
        )


if __name__ == "__main__":
    unittest.main()
