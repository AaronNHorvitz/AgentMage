from __future__ import annotations

import copy
import unittest

from scripts.sprint_2_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    REVIEWED_TREE,
    build_report,
    check_report,
    read_json,
    validate_report,
)


class Sprint2GateTests(unittest.TestCase):
    def test_checked_sprint_gate_is_current_and_blocked_without_substitution(self) -> None:
        self.assertEqual(check_report(), [])
        report = build_report()
        self.assertEqual(read_json(REPORT_PATH), report)
        self.assertEqual(
            report["status"],
            "blocked-open-dependencies-full-protocol-and-platform",
        )
        self.assertEqual(report["summary"]["blocking_controls"], ["G-DOD-10"])

    def test_all_six_sprint_acceptance_criteria_pass(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["criterion_id"] for item in report["acceptance_criteria"]],
            ["2.AC1", "2.AC2", "2.AC3", "2.AC4", "2.AC5", "2.AC6"],
        )
        self.assertTrue(
            all(item["status"] == "pass" for item in report["acceptance_criteria"])
        )

    def test_all_four_story_gates_preserve_their_exact_blockers(self) -> None:
        report = build_report()
        self.assertEqual(
            [item["story_id"] for item in report["story_gates"]],
            ["2.1", "2.2", "2.3", "2.4"],
        )
        self.assertEqual(
            [item["status"] for item in report["story_gates"]],
            [
                "blocked-macos",
                "blocked-macos",
                "blocked-open-dependencies-and-platform",
                "blocked-open-dependencies-full-protocol-and-platform",
            ],
        )
        self.assertTrue(
            all(item["dod_control_ids"] == list(G_DOD_IDS) for item in report["story_gates"])
        )

    def test_reviewed_commit_and_result_identities_are_retained(self) -> None:
        report = build_report()
        platform = report["acceptance_criteria"][3]
        self.assertEqual(report["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(report["reviewed_tree"], REVIEWED_TREE)
        self.assertEqual(
            len(report["independent_review"]["reviewed_artifacts"]),
            len(REVIEWED_PATHS),
        )
        self.assertEqual(platform["result_identity_count"], 2)
        self.assertTrue(
            all(
                set(item)
                >= {
                    "fixture_sha256",
                    "build_sha256",
                    "model_sha256",
                    "runtime_sha256",
                    "policy_sha256",
                }
                for item in platform["result_identities"]
            )
        )

    def test_acceptance_and_story_status_mutations_fail_closed(self) -> None:
        report = build_report()
        criterion = copy.deepcopy(report)
        criterion["acceptance_criteria"][0]["status"] = "fail"
        story = copy.deepcopy(report)
        story["story_gates"][0]["status"] = "pass"
        self.assertTrue(validate_report(criterion))
        self.assertTrue(validate_report(story))

    def test_blocker_omission_and_external_review_overclaim_fail_closed(self) -> None:
        report = build_report()
        blocker = copy.deepcopy(report)
        blocker["summary"]["blocking_controls"] = []
        dependency = copy.deepcopy(report)
        dependency["blockers"] = []
        external = copy.deepcopy(report)
        external["independent_review"]["external_human_review_claim"] = "complete"
        self.assertTrue(validate_report(blocker))
        self.assertTrue(validate_report(dependency))
        self.assertTrue(validate_report(external))

    def test_macos_product_and_release_overclaims_fail_closed(self) -> None:
        report = build_report()
        macos = copy.deepcopy(report)
        macos["macos"]["status"] = "pass"
        product = copy.deepcopy(report)
        product["product_acceptance_claim"] = "pass"
        release = copy.deepcopy(report)
        release["release_claim"] = "pass"
        parser = copy.deepcopy(report)
        parser["product_runtime_claim"] = "pass"
        protocol = copy.deepcopy(report)
        protocol["full_protocol_substitution_permitted"] = True
        for changed in (macos, product, release, parser, protocol):
            self.assertTrue(validate_report(changed))

    def test_story_2_3_aggregate_criterion_retains_non_pass_and_no_effect_truth(self) -> None:
        criterion = build_report()["acceptance_criteria"][5]
        self.assertEqual(criterion["clean_reproduction_run_count"], 2)
        self.assertEqual(criterion["reproduced_output_count"], 11)
        self.assertEqual(criterion["golden_outcome_count"], 77)
        self.assertEqual(criterion["golden_non_success_terminal_count"], 60)
        self.assertEqual(criterion["duplicate_effect_count"], 0)
        self.assertEqual(criterion["approval_bypass_count"], 0)
        self.assertEqual(criterion["fixed_seed_observation_count"], 153)
        self.assertEqual(criterion["manual_fuzz_campaign_status"], "deferred-not-executed")


if __name__ == "__main__":
    unittest.main()
