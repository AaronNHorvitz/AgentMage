from __future__ import annotations

import copy
import json
import unittest

from scripts.sprint_6_gate import BLOCKERS, REPORT_PATH, REVIEWED_PATHS, build_report, validate_report


class Sprint6GateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_all_criteria_and_available_scope_are_exact(self) -> None:
        self.assertEqual(json.loads(REPORT_PATH.read_text(encoding="utf-8")), self.report)
        criteria = self.report["acceptance_criteria"]
        self.assertEqual([item["criterion_id"] for item in criteria], [f"6.AC{i}" for i in range(1, 6)])
        self.assertEqual(self.report["summary"]["acceptance_criteria_passed"], 1)
        self.assertEqual(self.report["summary"]["acceptance_criteria_blocked_macos"], 4)
        self.assertEqual(criteria[3]["display_link_rejection_count"], 1280)

    def test_story_and_macos_blockers_remain_open(self) -> None:
        self.assertEqual(self.report["summary"]["blockers"], list(BLOCKERS))
        self.assertEqual(self.report["story_gates"][0]["status"], "blocked-macos")
        self.assertFalse(self.report["summary"]["sprint_checkbox_complete"])

    def test_review_closure_is_exact(self) -> None:
        self.assertEqual(len(self.report["independent_review"]["reviewed_artifacts"]), len(REVIEWED_PATHS))

    def test_mutations_and_overclaims_fail_closed(self) -> None:
        evidence = copy.deepcopy(self.report)
        evidence["acceptance_criteria"][3]["filesystem_observation_count"] = 1
        blockers = copy.deepcopy(self.report)
        blockers["summary"]["blockers"] = []
        product = copy.deepcopy(self.report)
        product["installed_product_claim"] = "pass"
        macos = copy.deepcopy(self.report)
        macos["macos_evidence_substituted"] = True
        for changed in (evidence, blockers, product, macos):
            self.assertTrue(validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
