from __future__ import annotations

import copy
import unittest

from scripts.story_3_1_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REQUIRED_TASK_MARKERS,
    REVIEWED_COMMIT,
    REVIEWED_TREE,
    build_report,
    check_report,
    read_json,
    task_completion,
    validate_report,
)


class Story31GateTests(unittest.TestCase):
    def test_checked_gate_is_current_and_blocked_only_by_macos(self) -> None:
        self.assertEqual(check_report(), [])
        report = read_json(REPORT_PATH)
        self.assertEqual(report, build_report())
        self.assertEqual(report["only_blocker"], "macos-execution-evidence-unavailable")

    def test_both_acceptance_criteria_pass_shared_linux(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["3.1.AC1", "3.1.AC2"],
        )
        self.assertTrue(
            all(item["status"] == "pass-shared-linux" for item in report["acceptance_criteria"])
        )

    def test_review_identity_and_artifact_closure_are_retained(self) -> None:
        report = build_report()
        review = report["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(review["reviewed_tree"], REVIEWED_TREE)
        self.assertEqual(len(review["artifacts"]), 16)

    def test_task_omission_is_detected(self) -> None:
        complete = "\n".join(REQUIRED_TASK_MARKERS)
        self.assertEqual(task_completion(complete), [])
        self.assertEqual(
            task_completion(complete.replace(REQUIRED_TASK_MARKERS[-1], "")),
            [REQUIRED_TASK_MARKERS[-1]],
        )

    def test_acceptance_dod_and_security_mutations_fail_closed(self) -> None:
        report = build_report()
        acceptance = copy.deepcopy(report)
        acceptance["acceptance_criteria"][0]["status"] = "pass"
        dod = copy.deepcopy(report)
        dod["universal_definition_of_done"][0]["status"] = "blocked-macos"
        security = copy.deepcopy(report)
        security["security_mapping"]["product_requirements_complete"] = 9
        for changed in (acceptance, dod, security):
            self.assertTrue(validate_report(changed))

    def test_product_release_and_macos_overclaims_fail_closed(self) -> None:
        report = build_report()
        product = copy.deepcopy(report)
        product["product_acceptance_claim"] = "pass"
        activation = copy.deepcopy(report)
        activation["product_startup_activation_claim"] = "pass"
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        substitution = copy.deepcopy(report)
        substitution["macos_evidence_substituted"] = True
        for changed in (product, activation, release, macos, substitution):
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
