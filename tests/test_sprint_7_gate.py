from __future__ import annotations

import copy
import json
import unittest

from scripts.sprint_7_gate import BLOCKERS, REPORT_PATH, REVIEWED_PATHS, build_report, validate_report


class Sprint7GateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_all_criteria_and_current_scope_are_exact(self) -> None:
        self.assertEqual(json.loads(REPORT_PATH.read_text(encoding="utf-8")), self.report)
        criteria = self.report["acceptance_criteria"]
        self.assertEqual([item["criterion_id"] for item in criteria], [f"7.AC{i}" for i in range(1, 6)])
        self.assertEqual(self.report["summary"]["acceptance_criteria_passed"], 1)
        self.assertEqual(self.report["summary"]["acceptance_criteria_partial"], 2)
        self.assertEqual(self.report["summary"]["acceptance_criteria_blocked"], 2)
        self.assertEqual(criteria[2]["operating_system_branches_in_kernel_selector"], 0)

    def test_story_platform_and_external_review_blockers_remain_open(self) -> None:
        self.assertEqual(self.report["summary"]["blockers"], list(BLOCKERS))
        self.assertFalse(self.report["summary"]["sprint_checkbox_complete"])
        self.assertEqual(
            self.report["automated_aggregate_review"]["external_human_review_status"],
            "not-performed",
        )

    def test_review_closure_is_exact(self) -> None:
        self.assertEqual(
            len(self.report["automated_aggregate_review"]["reviewed_artifacts"]),
            len(REVIEWED_PATHS),
        )

    def test_mutations_and_overclaims_fail_closed(self) -> None:
        evidence = copy.deepcopy(self.report)
        evidence["acceptance_criteria"][2]["operating_system_branches_in_kernel_selector"] = 1
        blockers = copy.deepcopy(self.report)
        blockers["summary"]["blockers"] = []
        external = copy.deepcopy(self.report)
        external["automated_aggregate_review"]["external_human_review_status"] = "pass"
        product = copy.deepcopy(self.report)
        product["installed_product_claim"] = "pass"
        macos = copy.deepcopy(self.report)
        macos["macos_evidence_substituted"] = True
        for changed in (evidence, blockers, external, product, macos):
            self.assertTrue(validate_report(changed, verify_current=False))


if __name__ == "__main__":
    unittest.main()
