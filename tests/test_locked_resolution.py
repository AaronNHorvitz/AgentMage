from __future__ import annotations

import copy
import unittest

from scripts.locked_resolution import (
    EXPECTED_MUTATIONS,
    build_report,
    validate_report,
)


class LockedResolutionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = build_report()

    def test_two_clean_resolutions_are_identical(self) -> None:
        self.assertEqual(len(self.report["clean_runs"]), 2)
        self.assertTrue(self.report["clean_graphs_identical"])
        self.assertEqual(
            self.report["clean_runs"][0]["graph_sha256"],
            self.report["clean_runs"][1]["graph_sha256"],
        )

    def test_clean_resolvers_are_offline_and_non_mutating(self) -> None:
        for run in self.report["clean_runs"]:
            with self.subTest(run=run["run_id"]):
                self.assertEqual(run["status"], "pass")
                self.assertTrue(run["locks_unchanged"])
                self.assertIn("--offline", run["cargo"]["command"])
                self.assertIn("--offline", run["npm"]["command"])
                self.assertIn("--ignore-scripts", run["npm"]["command"])

    def test_each_dependency_mutation_fails_closed_precisely(self) -> None:
        self.assertEqual(
            {item["scenario"] for item in self.report["mutation_cases"]},
            set(EXPECTED_MUTATIONS),
        )
        for case in self.report["mutation_cases"]:
            with self.subTest(scenario=case["scenario"]):
                self.assertEqual(case["status"], "pass")
                self.assertIn(case["expected_diagnostic"], case["observed_diagnostics"])

    def test_missing_mutation_case_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["mutation_cases"].pop()
        self.assertTrue(validate_report(mutated))

    def test_nonidentical_clean_graphs_are_rejected(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["clean_graphs_identical"] = False
        self.assertTrue(validate_report(mutated))

    def test_linux_resolution_cannot_claim_macos_support(self) -> None:
        mutated = copy.deepcopy(self.report)
        mutated["macos_support_claim"] = "verified"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
