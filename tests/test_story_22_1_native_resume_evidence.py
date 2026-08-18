from __future__ import annotations

import copy
import json
import unittest

from scripts.story_22_1_native_resume_evidence import (
    EXPECTED_METRIC,
    METRIC_PREFIX,
    REPORT_PATH,
    NativeResumeEvidenceError,
    parse_metric,
    read_report,
    validate_report,
)


class Story221NativeResumeEvidenceTests(unittest.TestCase):
    def test_metric_closes_four_boundaries_and_one_hundred_resumes(self) -> None:
        self.assertEqual(len(EXPECTED_METRIC["boundaries"]), 4)
        self.assertEqual(EXPECTED_METRIC["repetitions_per_boundary"], 25)
        self.assertEqual(EXPECTED_METRIC["case_count"], 100)
        self.assertEqual(EXPECTED_METRIC["total_worker_launches_per_case"], 1)
        self.assertEqual(EXPECTED_METRIC["false_terminal_count"], 0)
        self.assertEqual(EXPECTED_METRIC["false_checkpoint_count"], 0)

    def test_metric_parser_rejects_missing_duplicate_malformed_and_drift(self) -> None:
        encoded = f"{METRIC_PREFIX}{json.dumps(EXPECTED_METRIC, sort_keys=True)}"
        self.assertEqual(parse_metric(encoded), EXPECTED_METRIC)
        for changed in (
            "",
            f"{encoded}\n{encoded}",
            f"{METRIC_PREFIX}{{bad-json}}",
            f"{METRIC_PREFIX}{json.dumps({**EXPECTED_METRIC, 'case_count': 99})}",
        ):
            with self.assertRaises(NativeResumeEvidenceError):
                parse_metric(changed)

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])
        changed = copy.deepcopy(report)
        changed["metrics"]["total_worker_launches_per_case"] = 2
        self.assertIn("runtime.native_resume.report_metrics", validate_report(changed))
        changed = copy.deepcopy(report)
        changed["verification"]["command_id"] = "0" * 64
        self.assertIn("runtime.native_resume.report_command", validate_report(changed))


if __name__ == "__main__":
    unittest.main()
