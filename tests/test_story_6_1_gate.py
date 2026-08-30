from __future__ import annotations

import copy
import unittest

from scripts.story_6_1_gate import BLOCKERS, REVIEWED_COMMIT, REVIEWED_PATHS, build_report, validate_report


class Story61GateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_current_non_macos_criteria_are_exact(self) -> None:
        criteria = self.report["acceptance_criteria"]
        self.assertEqual([item["criterion_id"] for item in criteria], ["6.1.AC1", "6.1.AC2"])
        self.assertEqual(criteria[0]["generated_case_count"], 640)
        self.assertEqual(criteria[0]["out_of_root_access_count"], 0)
        self.assertEqual(criteria[1]["display_link_rejection_count"], 1280)

    def test_gate_preserves_every_macos_blocker(self) -> None:
        self.assertEqual(self.report["status"], "blocked-macos")
        self.assertEqual(self.report["blockers"], list(BLOCKERS))
        self.assertFalse(self.report["story_checkbox_complete"])
        self.assertFalse(self.report["macos_evidence_substituted"])

    def test_review_boundary_is_exact(self) -> None:
        review = self.report["independent_review"]
        self.assertEqual(review["reviewed_commit"], REVIEWED_COMMIT)
        self.assertEqual(len(review["artifacts"]), len(REVIEWED_PATHS))

    def test_evidence_and_blocker_mutations_fail_closed(self) -> None:
        count = copy.deepcopy(self.report)
        count["acceptance_criteria"][0]["admitted_escape_count"] = 1
        blocker = copy.deepcopy(self.report)
        blocker["blockers"] = []
        review = copy.deepcopy(self.report)
        review["independent_review"]["finding_count"] = 1
        for changed in (count, blocker, review):
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
