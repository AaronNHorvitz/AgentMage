from __future__ import annotations

import copy
import json
import unittest

from scripts.story_23_4_runtime_evidence import (
    COMMANDS,
    COVERAGE,
    MAXIMUM_COMMAND_ELAPSED_MS,
    MAXIMUM_COMMAND_RSS_KIB,
    PERFORMANCE_PREFIX,
    REPORT_PATH,
    RuntimeEvidenceError,
    parse_performance,
    parse_test_result,
    performance_failures,
    read_report,
    validate_report,
)


def valid_performance() -> dict[str, object]:
    return {
        "schema_version": 1,
        "maximum_scenario_us": 250_000,
        "direct": scenario("success", 6, 1, 0, 1, 23),
        "nominal": scenario("success", 15, 2, 1, 2, 23),
        "maximum": scenario("success", 33, 4, 3, 4, 23),
        "over_limit": scenario("exhausted", 33, 4, 3, 4, 0),
        "cancellation": scenario("cancelled", 4, 0, 0, 0, 0),
    }


def scenario(
    state: str,
    events: int,
    model_calls: int,
    tool_calls: int,
    turns: int,
    output_bytes: int,
) -> dict[str, object]:
    return {
        "state": state,
        "event_count": events,
        "model_calls": model_calls,
        "tool_calls": tool_calls,
        "turns": turns,
        "output_bytes": output_bytes,
        "elapsed_us": 1_000,
    }


class Story234RuntimeEvidenceTests(unittest.TestCase):
    def test_actual_single_threaded_libtest_metric_keeps_exact_limits(self) -> None:
        # Retained 2026-09-27 campaign: all 18 tests passed, but column-zero
        # extraction rejected this actual serial --nocapture output framing.
        heading = "test runtime_loop::tests::story_23_4_ephemeral_runtime_profile_is_bounded ... "
        metrics = valid_performance()
        for name, elapsed in (("cancellation", 598), ("direct", 2077), ("maximum", 5687),
                              ("nominal", 4626), ("over_limit", 4871)):
            metrics[name]["elapsed_us"] = elapsed
        record = PERFORMANCE_PREFIX + json.dumps(metrics, separators=(",", ":"))
        trace = heading + record + "\nok\ntest result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 1155 filtered out; finished in 0.14s\n"
        self.assertEqual(parse_performance(trace), metrics)
        for invalid in (
            "unrelated " + record,
            heading.replace("ephemeral_runtime_profile", "another_profile") + record,
            trace + record,
            trace + "unrelated " + PERFORMANCE_PREFIX,
            heading + record + " trailing data",
        ):
            with self.assertRaises(RuntimeEvidenceError):
                parse_performance(invalid)
        metrics["maximum"]["elapsed_us"] = 250_001
        with self.assertRaises(RuntimeEvidenceError):
            parse_performance(heading + PERFORMANCE_PREFIX + json.dumps(metrics))

    def test_closed_performance_record_accepts_only_expected_results(self) -> None:
        metrics = valid_performance()
        self.assertEqual(performance_failures(metrics), [])
        output = f"prefix\n{PERFORMANCE_PREFIX}{json.dumps(metrics)}\nsuffix\n"
        self.assertEqual(parse_performance(output), metrics)

        for scenario_name, field, value in (
            ("direct", "state", "failed"),
            ("maximum", "tool_calls", 4),
            ("over_limit", "output_bytes", 1),
            ("cancellation", "elapsed_us", 250_001),
        ):
            changed = copy.deepcopy(metrics)
            changed[scenario_name][field] = value  # type: ignore[index]
            self.assertTrue(performance_failures(changed))

    def test_missing_duplicate_malformed_and_extra_metrics_fail_closed(self) -> None:
        metrics = valid_performance()
        record = f"{PERFORMANCE_PREFIX}{json.dumps(metrics)}\n"
        with self.assertRaises(RuntimeEvidenceError):
            parse_performance("no metric")
        with self.assertRaises(RuntimeEvidenceError):
            parse_performance(record + record)
        with self.assertRaises(RuntimeEvidenceError):
            parse_performance(f"{PERFORMANCE_PREFIX}{{not-json}}\n")
        changed = copy.deepcopy(metrics)
        changed["unexpected"] = True
        self.assertTrue(performance_failures(changed))

    def test_rust_test_summary_is_aggregated_and_failure_sensitive(self) -> None:
        output = (
            "test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 4 filtered out\n"
            "test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n"
        )
        self.assertEqual(parse_test_result(output, 16)["passed"], 16)
        with self.assertRaises(RuntimeEvidenceError):
            parse_test_result(output, 17)
        with self.assertRaises(RuntimeEvidenceError):
            parse_test_result(
                "test result: ok. 16 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out",
                16,
            )

    def test_campaign_shape_is_closed_and_bounded(self) -> None:
        self.assertEqual(len(COMMANDS), 9)
        self.assertEqual(len({command_id for command_id, _, _ in COMMANDS}), 9)
        self.assertEqual(len(COVERAGE), 24)
        self.assertTrue(all(COVERAGE.values()))
        prepared = [(arguments, count) for name, arguments, count in COMMANDS
                    if name == "prepared-artifact-matrix"]
        self.assertEqual(len(prepared), 1)
        self.assertEqual(prepared[0][1], 12)
        self.assertIn("runtime_loop::tests::artifact_preparation_tests::", prepared[0][0])
        self.assertEqual(MAXIMUM_COMMAND_ELAPSED_MS, 120_000)
        self.assertEqual(MAXIMUM_COMMAND_RSS_KIB, 1_048_576)

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])


if __name__ == "__main__":
    unittest.main()
