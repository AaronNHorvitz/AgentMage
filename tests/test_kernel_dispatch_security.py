from __future__ import annotations

import copy
import unittest

from scripts.kernel_dispatch_security import (
    EXPECTED_CASES,
    build_report,
    run_dispatch_trace,
    trace_failures,
    validate_report,
)


class KernelDispatchSecurityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.traces = run_dispatch_trace()

    def test_rust_trace_has_the_exact_closed_case_set(self) -> None:
        self.assertEqual(
            tuple(item["case_id"] for item in self.traces), EXPECTED_CASES
        )
        self.assertEqual(trace_failures(self.traces), [])

    def test_state_change_output_and_evidence_are_rejected(self) -> None:
        for field, value in (
            ("state_change", "changed"),
            ("output_present", True),
            ("evidence_count", 1),
        ):
            with self.subTest(field=field):
                mutated = copy.deepcopy(self.traces)
                mutated[0][field] = value
                self.assertTrue(trace_failures(mutated))

    def test_stale_or_broadened_report_is_rejected(self) -> None:
        report = build_report("a" * 40)
        self.assertEqual(validate_report(report), [])
        mutated = copy.deepcopy(report)
        mutated["positive_dispatch_path_available"] = True
        self.assertTrue(validate_report(mutated))


if __name__ == "__main__":
    unittest.main()
