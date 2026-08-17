"""Mutation tests for the Story 50.2 reference-load evidence contract."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

from scripts import runtime_hardening_load as MODULE

ROOT = Path(__file__).resolve().parents[1]


class RuntimeHardeningLoadTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = json.loads(
            (ROOT / MODULE.PROFILE).read_text(encoding="utf-8")
        )
        self.metrics = {
            "cleanup_verified": True,
            "event_count": 8196,
            "fast_deliveries": 8196,
            "journal_elapsed_ms": 2000,
            "journal_events_per_second": 4098,
            "lagged_subscribers": 1,
            "maximum_queued_bytes": 100000,
            "maximum_queued_events": 127,
            "publish_elapsed_ms": 250,
            "publish_events_per_second": 32784,
            "resident_memory_kib": 50000,
            "restart_cycles": 16,
            "restart_elapsed_ms": 12000,
            "retained_disk_bytes": 12000000,
        }

    def test_current_profile_and_exact_metrics_pass(self) -> None:
        MODULE.validate_profile(self.profile)
        self.assertEqual(MODULE.metric_failures(self.metrics, self.profile), [])

    def test_test_summary_is_aggregated_without_accepting_failure(self) -> None:
        output = (
            "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out\n"
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out\n"
        )
        self.assertEqual(
            MODULE.parse_test_results(output),
            {
                "passed": 3,
                "failed": 0,
                "ignored": 0,
                "measured": 0,
                "filtered_out": 7,
            },
        )
        with self.assertRaises(MODULE.CampaignError):
            MODULE.parse_test_results("no test result")

    def test_metric_and_process_records_parse_only_one_closed_shape(self) -> None:
        output = MODULE.METRIC_PREFIX + json.dumps(self.metrics)
        self.assertEqual(MODULE.parse_load_metrics(output), self.metrics)
        with self.assertRaises(MODULE.CampaignError):
            MODULE.parse_load_metrics(output + "\n" + output)

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "time.txt"
            path.write_text(
                "elapsed_seconds=1.25\nmaximum_rss_kib=4096\n",
                encoding="ascii",
            )
            self.assertEqual(MODULE._parse_time_output(path), (1250, 4096))

    def test_every_metric_boundary_fails_independently(self) -> None:
        mutations = {
            "event_count": 8195,
            "fast_deliveries": 8195,
            "restart_cycles": 15,
            "cleanup_verified": False,
            "lagged_subscribers": 0,
            "journal_elapsed_ms": 30001,
            "journal_events_per_second": 249,
            "publish_elapsed_ms": 10001,
            "publish_events_per_second": 999,
            "restart_elapsed_ms": 60001,
            "resident_memory_kib": 1048577,
            "retained_disk_bytes": 134217729,
            "maximum_queued_events": 1025,
            "maximum_queued_bytes": 4194305,
        }
        for field, value in mutations.items():
            with self.subTest(field=field):
                candidate = copy.deepcopy(self.metrics)
                candidate[field] = value
                self.assertEqual(len(MODULE.metric_failures(candidate, self.profile)), 1)

    def test_command_substitution_duplicate_and_metric_owner_fail_closed(self) -> None:
        substituted = copy.deepcopy(self.profile)
        substituted["commands"][0]["argv"][0] = "sh"
        with self.assertRaises(MODULE.CampaignError):
            MODULE.validate_profile(substituted)

        duplicate = copy.deepcopy(self.profile)
        duplicate["commands"][1]["id"] = duplicate["commands"][0]["id"]
        with self.assertRaises(MODULE.CampaignError):
            MODULE.validate_profile(duplicate)

        hidden_metric_owner = copy.deepcopy(self.profile)
        hidden_metric_owner["commands"][1]["requires_load_metrics"] = True
        with self.assertRaises(MODULE.CampaignError):
            MODULE.validate_profile(hidden_metric_owner)


if __name__ == "__main__":
    unittest.main()
