from __future__ import annotations

import copy
import json
import unittest

from scripts.story_21_2_crash_evidence import (
    METRIC_PREFIX,
    REPORT_PATH,
    CrashEvidenceError,
    expected_cases,
    expected_metrics,
    parse_metrics,
    read_report,
    validate_report,
)


class Story212CrashEvidenceTests(unittest.TestCase):
    def test_closed_matrix_has_every_boundary_position_once(self) -> None:
        cases = expected_cases()
        self.assertEqual(len(cases), 8)
        self.assertEqual(
            [(case["boundary"], case["position"]) for case in cases],
            [
                ("queue-admission", "before"),
                ("queue-admission", "after"),
                ("batch-flush", "before"),
                ("batch-flush", "after"),
                ("correctness-transaction", "before"),
                ("correctness-transaction", "after"),
                ("subscriber-publication", "before"),
                ("subscriber-publication", "after"),
            ],
        )
        self.assertEqual(sum(case["bounded_progress_loss"] for case in cases), 2)
        self.assertTrue(all(not case["false_terminal_before_recovery"] for case in cases))
        self.assertTrue(all(case["second_reopen_verified"] for case in cases))

    def test_exact_metric_record_parses_and_drift_fails(self) -> None:
        metrics = expected_metrics()
        output = f"prefix\n{METRIC_PREFIX}{json.dumps(metrics, sort_keys=True)}\nsuffix\n"
        self.assertEqual(parse_metrics(output), metrics)
        changed = copy.deepcopy(metrics)
        changed["cases"][0]["middle_was_durable"] = True
        with self.assertRaises(CrashEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{json.dumps(changed)}\n")

    def test_missing_duplicate_and_malformed_metrics_fail_closed(self) -> None:
        with self.assertRaises(CrashEvidenceError):
            parse_metrics("no metric")
        record = f"{METRIC_PREFIX}{json.dumps(expected_metrics())}\n"
        with self.assertRaises(CrashEvidenceError):
            parse_metrics(record + record)
        with self.assertRaises(CrashEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{{not-json}}\n")

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])


if __name__ == "__main__":
    unittest.main()
