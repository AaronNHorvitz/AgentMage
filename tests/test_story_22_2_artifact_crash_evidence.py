from __future__ import annotations

import copy
import json
import unittest

from scripts.story_22_2_artifact_crash_evidence import (
    BOUNDARIES,
    METRIC_PREFIX,
    REPORT_PATH,
    ArtifactCrashEvidenceError,
    expected_cases,
    expected_metrics,
    parse_metrics,
    read_report,
    validate_report,
)


class Story222ArtifactCrashEvidenceTests(unittest.TestCase):
    def test_closed_matrix_has_every_native_boundary_position_once(self) -> None:
        cases = expected_cases()
        self.assertEqual(len(cases), 14)
        self.assertEqual(
            [(case["boundary"], case["position"]) for case in cases],
            [
                (boundary, position)
                for boundary in BOUNDARIES
                for position in ("before", "after")
            ],
        )
        self.assertEqual(sum(case["recovery_cleaned_staging"] for case in cases), 2)
        self.assertEqual(sum(case["recovery_deleted_orphans"] for case in cases), 4)
        self.assertTrue(all(case["recovery_quarantined_payloads"] == 0 for case in cases))
        self.assertTrue(all(case["false_terminal_count"] == 0 for case in cases))

    def test_exact_metric_record_parses_and_drift_fails(self) -> None:
        metrics = expected_metrics()
        output = f"prefix\n{METRIC_PREFIX}{json.dumps(metrics, sort_keys=True)}\nsuffix\n"
        self.assertEqual(parse_metrics(output), metrics)
        changed = copy.deepcopy(metrics)
        changed["cases"][3]["inventory_after_recovery"] = 1
        with self.assertRaises(ArtifactCrashEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{json.dumps(changed)}\n")

    def test_missing_duplicate_and_malformed_metrics_fail_closed(self) -> None:
        with self.assertRaises(ArtifactCrashEvidenceError):
            parse_metrics("no metric")
        record = f"{METRIC_PREFIX}{json.dumps(expected_metrics())}\n"
        with self.assertRaises(ArtifactCrashEvidenceError):
            parse_metrics(record + record)
        with self.assertRaises(ArtifactCrashEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{{not-json}}\n")

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        self.assertEqual(validate_report(read_report()), [])


if __name__ == "__main__":
    unittest.main()
