from __future__ import annotations

import copy
import unittest

from scripts.story_3_2_gate import (
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


class Story32GateTests(unittest.TestCase):
    def test_checked_gate_is_current_and_blocked_only_by_macos(self) -> None:
        self.assertEqual(check_report(), [])
        report = read_json(REPORT_PATH)
        self.assertEqual(report, build_report())
        self.assertEqual(report["only_blocker"], "macos-execution-evidence-unavailable")

    def test_all_three_acceptance_criteria_pass_at_declared_scope(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["3.2.AC1", "3.2.AC2", "3.2.AC3"],
        )
        self.assertTrue(
            all(item["status"].startswith("pass-shared-linux") for item in report["acceptance_criteria"])
        )

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

    def test_acceptance_dod_security_and_review_mutations_fail_closed(self) -> None:
        report = build_report()
        acceptance = copy.deepcopy(report)
        acceptance["acceptance_criteria"][0]["receipt_count"] = 6
        startup = copy.deepcopy(report)
        startup["acceptance_criteria"][2]["implemented_product_startup_hook_claim"] = (
            "implemented"
        )
        dod = copy.deepcopy(report)
        dod["universal_definition_of_done"][0]["status"] = "blocked-macos"
        security = copy.deepcopy(report)
        security["security_mapping"]["rv_22_complete"] = True
        review = copy.deepcopy(report)
        review["independent_review"]["finding_count"] = 1
        for changed in (acceptance, startup, dod, security, review):
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed))

    def test_actual_product_release_network_and_macos_overclaims_fail_closed(self) -> None:
        report = build_report()
        mutations = []
        for field in (
            "actual_incident_claim",
            "product_startup_integration_claim",
            "product_support_claim",
            "product_patch_claim",
            "product_acceptance_claim",
            "release_claim",
        ):
            changed = copy.deepcopy(report)
            changed[field] = "pass"
            mutations.append(changed)
        network = copy.deepcopy(report)
        network["network_used"] = True
        mutations.append(network)
        macos = copy.deepcopy(report)
        macos["macos_execution_status"] = "pass"
        mutations.append(macos)
        substitution = copy.deepcopy(report)
        substitution["macos_evidence_substituted"] = True
        mutations.append(substitution)
        for changed in mutations:
            with self.subTest(changed=changed):
                self.assertTrue(validate_report(changed))

    def test_universal_dod_and_startup_contract_boundary_are_exact(self) -> None:
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
        startup = report["acceptance_criteria"][2]
        self.assertTrue(startup["startup_checkpoint_declared"])
        self.assertFalse(startup["remote_kill_switch"])
        self.assertEqual(startup["implemented_product_startup_hook_claim"], "none")


if __name__ == "__main__":
    unittest.main()
