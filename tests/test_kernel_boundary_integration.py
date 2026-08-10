from __future__ import annotations

import copy
import unittest

from scripts.kernel_boundary_integration import (
    EXPECTED_CASES,
    build_report,
    run_boundary_trace,
    trace_failures,
    validate_report,
)


class KernelBoundaryIntegrationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.traces = run_boundary_trace()

    def test_rust_trace_has_the_exact_closed_outcome_set(self) -> None:
        self.assertEqual(
            tuple(item["case_id"] for item in self.traces), EXPECTED_CASES
        )
        self.assertEqual(trace_failures(self.traces), [])

    def test_context_route_and_error_mutations_are_rejected(self) -> None:
        for field, value in (
            ("correlation_id", "correlation-elsewhere"),
            ("route", ["tool", "shell"]),
            ("caused_by", None),
        ):
            with self.subTest(field=field):
                mutated = copy.deepcopy(self.traces)
                mutated[-1][field] = value
                self.assertTrue(trace_failures(mutated))

    def test_stale_or_broadened_report_is_rejected(self) -> None:
        report = build_report("a" * 40)
        self.assertEqual(validate_report(report), [])
        mutated = copy.deepcopy(report)
        mutated["platform_status"]["macos_test"] = "verified"
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
