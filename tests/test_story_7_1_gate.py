from __future__ import annotations

import copy
import json
import unittest

from scripts.story_7_1_gate import BLOCKERS, REPORT_PATH, REVIEWED_COMMIT, REVIEWED_PATHS, build_report, validate_report


class Story71GateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_current_criteria_are_exact_and_bounded(self) -> None:
        self.assertEqual(json.loads(REPORT_PATH.read_text(encoding="utf-8")), self.report)
        criteria = self.report["acceptance_criteria"]
        self.assertEqual([item["criterion_id"] for item in criteria], ["7.1.AC1", "7.1.AC2"])
        self.assertEqual(criteria[0]["operating_system_branches_in_kernel_selector"], 0)
        self.assertEqual(criteria[0]["installed_runtime_component_manifest_count"], 0)
        self.assertEqual(criteria[1]["equivalent_available_contract_result_count"], 3)

    def test_gate_preserves_platform_and_review_blockers(self) -> None:
        self.assertEqual(self.report["blockers"], list(BLOCKERS))
        self.assertFalse(self.report["story_checkbox_complete"])
        self.assertFalse(self.report["macos_evidence_substituted"])
        self.assertEqual(
            self.report["independent_review"]["external_human_review_status"],
            "not-performed",
        )

    def test_review_boundary_is_exact(self) -> None:
        review = self.report["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(len(review["artifacts"]), len(REVIEWED_PATHS))

    def test_evidence_blocker_and_review_mutations_fail_closed(self) -> None:
        branch = copy.deepcopy(self.report)
        branch["acceptance_criteria"][0]["operating_system_branches_in_kernel_selector"] = 1
        blocker = copy.deepcopy(self.report)
        blocker["blockers"] = []
        review = copy.deepcopy(self.report)
        review["independent_review"]["external_human_review_status"] = "pass"
        for changed in (branch, blocker, review):
            self.assertTrue(validate_report(changed, verify_current=False))

    def test_product_platform_and_release_overclaims_fail_closed(self) -> None:
        for key, value in (
            ("installed_product_claim", "pass"),
            ("product_acceptance_claim", "pass"),
            ("sprint_completion_claim", True),
            ("release_claim", "pass"),
            ("macos_evidence_substituted", True),
        ):
            changed = copy.deepcopy(self.report)
            changed[key] = value
            self.assertTrue(validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
