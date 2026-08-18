from __future__ import annotations

import copy
import json
import unittest

from scripts.story_22_2_artifact_resume_evidence import (
    METRIC_PREFIX,
    REPORT_PATH,
    ArtifactResumeEvidenceError,
    expected_metrics,
    parse_metrics,
    read_report,
    validate_report,
)


class Story222ArtifactResumeEvidenceTests(unittest.TestCase):
    def test_closed_metrics_cover_exact_resume_drift_and_payload_loss(self) -> None:
        metrics = expected_metrics()
        exact = metrics["exact-checkpoint-resume"]
        lost = metrics["continuation-integrity-loss"]
        self.assertEqual(
            [
                exact["repository_drift"],
                exact["policy_drift"],
                exact["model_runtime_drift"],
            ],
            ["blocked", "blocked", "blocked"],
        )
        self.assertEqual(exact["pre_restart_tool_executions"], 1)
        self.assertEqual(exact["post_resume_total_tool_executions"], 1)
        self.assertEqual(lost["cases"], ["missing", "corrupt"])
        self.assertEqual(lost["cases_blocked"], 2)
        self.assertEqual(lost["post_failure_total_tool_executions_per_case"], 1)

    def test_exact_metric_records_parse_in_any_order_and_drift_fails(self) -> None:
        metrics = expected_metrics()
        output = "\n".join(
            f"{METRIC_PREFIX}{json.dumps(metric, sort_keys=True)}"
            for metric in reversed(list(metrics.values()))
        )
        self.assertEqual(parse_metrics(output), metrics)
        changed = copy.deepcopy(metrics)
        changed["exact-checkpoint-resume"]["post_resume_total_tool_executions"] = 2
        changed_output = "\n".join(
            f"{METRIC_PREFIX}{json.dumps(metric, sort_keys=True)}"
            for metric in changed.values()
        )
        with self.assertRaises(ArtifactResumeEvidenceError):
            parse_metrics(changed_output)

    def test_missing_duplicate_and_malformed_metrics_fail_closed(self) -> None:
        with self.assertRaises(ArtifactResumeEvidenceError):
            parse_metrics("no metric")
        exact = expected_metrics()["exact-checkpoint-resume"]
        duplicate = f"{METRIC_PREFIX}{json.dumps(exact)}\n" * 2
        with self.assertRaises(ArtifactResumeEvidenceError):
            parse_metrics(duplicate)
        with self.assertRaises(ArtifactResumeEvidenceError):
            parse_metrics(f"{METRIC_PREFIX}{{not-json}}\n")

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        self.assertEqual(validate_report(read_report()), [])


if __name__ == "__main__":
    unittest.main()
