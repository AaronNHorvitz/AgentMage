from __future__ import annotations

import copy
import unittest

from scripts.dependency_injection import build_report, check_report, validate_report
from scripts.dependency_rules import prohibited_compile_edges


class DependencyInjectionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.report = build_report()

    def test_checked_in_report_is_current(self) -> None:
        self.assertEqual(check_report(), [])

    def test_all_prohibited_compile_edges_are_injected(self) -> None:
        self.assertEqual(len(prohibited_compile_edges()), 58)
        observed = {(item["source"], item["target"]) for item in self.report["cases"]}
        self.assertEqual(observed, set(prohibited_compile_edges()))

    def test_every_case_has_precise_source_target_diagnostic(self) -> None:
        for case in self.report["cases"]:
            with self.subTest(case=case["case_id"]):
                self.assertIn(case["source"], case["expected_diagnostic"])
                self.assertIn(case["target"], case["expected_diagnostic"])
                self.assertIn(case["expected_diagnostic"], case["observed_diagnostics"])
                self.assertEqual(case["status"], "pass")

    def test_missing_case_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["cases"].pop()
        self.assertTrue(validate_report(mutated))

    def test_imprecise_diagnostic_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["cases"][0]["expected_diagnostic"] = "dependency failed"
        self.assertTrue(validate_report(mutated))

    def test_static_check_cannot_claim_macos_support(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["macos_support_claim"] = "verified"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
