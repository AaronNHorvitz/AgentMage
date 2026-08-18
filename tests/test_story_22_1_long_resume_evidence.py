from __future__ import annotations

import copy
import json
import unittest

from scripts.story_22_1_long_resume_evidence import (
    EXPECTED_METRIC,
    METRIC_PREFIX,
    REPORT_PATH,
    LongResumeEvidenceError,
    parse_metric,
    read_report,
    validate_report,
)


class Story221LongResumeEvidenceTests(unittest.TestCase):
    def test_metric_covers_long_history_exact_resume_and_drift(self) -> None:
        self.assertEqual(EXPECTED_METRIC["checkpoint_count"], 7)
        self.assertEqual(EXPECTED_METRIC["receipt_count"], 7)
        self.assertEqual(EXPECTED_METRIC["post_resume_total_worker_launches"], 7)
        self.assertTrue(EXPECTED_METRIC["exact_artifact_set_restored"])
        self.assertTrue(EXPECTED_METRIC["intent_preserved"])
        self.assertEqual(len(EXPECTED_METRIC["blocked_drift_classes"]), 7)

    def test_metric_parser_rejects_missing_duplicate_malformed_and_drift(self) -> None:
        encoded = f"{METRIC_PREFIX}{json.dumps(EXPECTED_METRIC, sort_keys=True)}"
        self.assertEqual(parse_metric(f"test case ... {encoded}"), EXPECTED_METRIC)
        for changed in (
            "",
            f"{encoded}\n{encoded}",
            f"{METRIC_PREFIX}{{bad-json}}",
            f"{METRIC_PREFIX}{json.dumps({**EXPECTED_METRIC, 'receipt_count': 6})}",
        ):
            with self.assertRaises(LongResumeEvidenceError):
                parse_metric(changed)

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])
        changed = copy.deepcopy(report)
        changed["metrics"]["exact_artifact_set_restored"] = False
        self.assertIn("runtime.long_resume.report_metrics", validate_report(changed))
        changed = copy.deepcopy(report)
        changed["sources"][0]["sha256"] = "0" * 64
        self.assertIn("runtime.long_resume.source_drift", validate_report(changed))


if __name__ == "__main__":
    unittest.main()
