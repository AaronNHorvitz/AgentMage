from __future__ import annotations

import copy
import json
import unittest

from scripts.kernel_dispatch_security import (
    EXPECTED_CASES,
    MARKER,
    TEST_NAME,
    build_report,
    decode_dispatch_trace,
    run_dispatch_trace,
    trace_failures,
    validate_report,
)


class KernelDispatchSecurityTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.traces = run_dispatch_trace()

    def test_actual_trace_with_single_thread_libtest_prefix(self) -> None:
        payload = MARKER + json.dumps(self.traces)
        for framed in (payload, f"test {TEST_NAME} ... {payload}"):
            with self.subTest(framed=framed[:100]):
                self.assertEqual(
                    decode_dispatch_trace(f"running 1 test\n{framed}\nok\n"), self.traces
                )

    def test_missing_duplicate_unexpected_prefix_and_malformed_traces_fail(self) -> None:
        payload = MARKER + json.dumps(self.traces)
        for stdout in (
            "running 0 tests\n", payload + "\n" + payload,
            payload + payload, "test another_test ... " + payload,
            "arbitrary " + payload, MARKER + "{", MARKER + "{}", MARKER + "[1]",
        ):
            with self.subTest(stdout=stdout[:100]):
                with self.assertRaises(ValueError):
                    decode_dispatch_trace(stdout)

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
