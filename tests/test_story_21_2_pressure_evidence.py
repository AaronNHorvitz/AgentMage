from __future__ import annotations

import copy
import json
import unittest

from scripts.story_21_2_pressure_evidence import (
    COVERAGE,
    METRIC_PREFIX,
    REPORT_PATH,
    PressureEvidenceError,
    metric_failures,
    parse_metrics,
    read_report,
    validate_report,
)


def valid_metrics() -> dict[str, object]:
    return {
        "cancellation_latency_us": 1_000,
        "cancellation_limit_ms": 250,
        "client_progress_while_store_delayed": True,
        "external_network_used": False,
        "fragment_count_before_cancellation": 17,
        "store_delay_ms": 18,
        "terminal_waited_for_correctness_durability": True,
    }


class Story212PressureEvidenceTests(unittest.TestCase):
    def test_metric_contract_accepts_only_the_closed_bounded_shape(self) -> None:
        metrics = valid_metrics()
        self.assertEqual(metric_failures(metrics), [])
        output = f"prefix\n{METRIC_PREFIX}{json.dumps(metrics)}\nsuffix\n"
        self.assertEqual(parse_metrics(output), metrics)

        for field, replacement in (
            ("cancellation_latency_us", 250_000),
            ("fragment_count_before_cancellation", 15),
            ("store_delay_ms", 0),
            ("external_network_used", True),
        ):
            changed = copy.deepcopy(metrics)
            changed[field] = replacement
            self.assertNotEqual(metric_failures(changed), [], field)

    def test_missing_duplicate_malformed_and_extra_metrics_fail_closed(self) -> None:
        with self.assertRaises(PressureEvidenceError):
            parse_metrics("no metric")
        record = f"{METRIC_PREFIX}{json.dumps(valid_metrics())}\n"
        with self.assertRaises(PressureEvidenceError):
            parse_metrics(record + record)
        with self.assertRaises(PressureEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{{not-json}}\n")
        changed = valid_metrics()
        changed["unexpected"] = 1
        with self.assertRaises(PressureEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{json.dumps(changed)}\n")

    def test_coverage_is_closed_and_does_not_claim_device_fault_injection(self) -> None:
        self.assertEqual(len(COVERAGE), 7)
        self.assertTrue(all(COVERAGE.values()))
        self.assertNotIn("physical_device_fault_injection", COVERAGE)

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])


if __name__ == "__main__":
    unittest.main()
