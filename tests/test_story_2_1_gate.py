from __future__ import annotations

import copy
import unittest

from scripts.story_2_1_gate import (
    G_DOD_IDS,
    REPORT_PATH,
    REVIEWED_COMMIT,
    REVIEWED_PATHS,
    build_report,
    check_report,
    read_json,
    validate_report,
)


class Story21GateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_checked_gate_report_is_current(self) -> None:
        self.assertEqual(check_report(), [])
        self.assertEqual(read_json(REPORT_PATH), self.report)

    def test_story_acceptance_passes_while_gate_remains_blocked_macos(self) -> None:
        self.assertEqual(
            [item["status"] for item in self.report["acceptance_criteria"]],
            ["pass", "pass"],
        )
        self.assertEqual(self.report["status"], "blocked-macos")
        self.assertFalse(self.report["summary"]["story_checkbox_complete"])
        self.assertEqual(self.report["summary"]["blocking_controls"], ["G-DOD-10"])

    def test_portable_reproduction_uses_no_private_or_machine_data(self) -> None:
        criterion = self.report["acceptance_criteria"][1]
        self.assertEqual(criterion["corpus_reproduction_runs"], 2)
        self.assertTrue(criterion["corpus_runs_byte_identical"])
        self.assertTrue(criterion["summary_exact_match"])
        self.assertFalse(criterion["private_user_data_used"])
        self.assertFalse(criterion["original_development_machine_required"])

    def test_review_binds_the_immutable_commit_and_expected_artifacts(self) -> None:
        review = self.report["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(
            [item["path"] for item in review["reviewed_artifacts"]],
            list(REVIEWED_PATHS),
        )
        self.assertEqual(review["finding_count"], 0)
        self.assertEqual(review["external_human_review_claim"], "none")

    def test_definition_of_done_is_exact_and_preserves_macos_blocker(self) -> None:
        dod = self.report["universal_definition_of_done"]
        self.assertEqual([item["control_id"] for item in dod], list(G_DOD_IDS))
        self.assertEqual(
            [item["control_id"] for item in dod if item["status"] == "blocked-macos"],
            ["G-DOD-10"],
        )

    def test_failed_criterion_or_missing_dod_control_fails_closed(self) -> None:
        failed = copy.deepcopy(self.report)
        failed["acceptance_criteria"][0]["status"] = "fail"
        missing = copy.deepcopy(self.report)
        missing["universal_definition_of_done"].pop()
        self.assertTrue(validate_report(failed))
        self.assertTrue(validate_report(missing))

    def test_story_product_release_or_macos_overclaim_fails_closed(self) -> None:
        complete = copy.deepcopy(self.report)
        complete["summary"]["story_checkbox_complete"] = True
        product = copy.deepcopy(self.report)
        product["product_acceptance_claim"] = "pass"
        macos = copy.deepcopy(self.report)
        macos["macos"]["execution_performed"] = True
        self.assertTrue(validate_report(complete))
        self.assertTrue(validate_report(product))
        self.assertTrue(validate_report(macos))


if __name__ == "__main__":
    unittest.main()
