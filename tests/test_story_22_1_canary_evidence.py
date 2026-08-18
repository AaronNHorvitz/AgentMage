from __future__ import annotations

import copy
import unittest

from scripts.story_22_1_canary_evidence import (
    COMMANDS,
    COVERAGE,
    REPORT_PATH,
    TASK_IDS,
    expected_command_records,
    read_report,
    validate_report,
)


class Story221CanaryEvidenceTests(unittest.TestCase):
    def test_coverage_closes_every_current_named_surface_family(self) -> None:
        self.assertEqual(len(COMMANDS), 8)
        self.assertEqual(set(COVERAGE), {
            "context-and-checkpoint-input",
            "classification-minimization-and-secret-detection",
            "sqlcipher-backup-export-and-crash-diagnostics",
            "transcript-event-diagnostic-and-metric-projections",
            "owner-bound-private-artifact-content",
            "model-and-tool-event-content",
            "native-staging-and-object-ciphertext",
            "external-telemetry-dependency",
        })
        self.assertEqual(
            {identifier for values in COVERAGE.values() for identifier in values},
            {record["id"] for record in expected_command_records()},
        )

    def test_command_records_bind_exact_invocations(self) -> None:
        records = expected_command_records()
        self.assertEqual(len({record["command_id"] for record in records}), len(records))
        self.assertTrue(all(record["required_test"] for record in records))

    @unittest.skipUnless(REPORT_PATH.is_file(), "retained report is generated after source commit")
    def test_current_report_and_raw_trace_are_hash_bound(self) -> None:
        report = read_report()
        self.assertEqual(validate_report(report), [])
        changed = copy.deepcopy(report)
        changed["task_ids"] = TASK_IDS[:-1]
        self.assertIn("runtime.canary.report_task_ids", validate_report(changed))
        changed = copy.deepcopy(report)
        changed["coverage"]["context-and-checkpoint-input"] = []
        self.assertIn("runtime.canary.report_disposition", validate_report(changed))
        changed = copy.deepcopy(report)
        changed["commands"][0]["exit_code"] = 1
        self.assertIn("runtime.canary.command.context-admission", validate_report(changed))
        changed = copy.deepcopy(report)
        changed["raw_canary_values_retained_in_report"] = 1
        self.assertIn("runtime.canary.raw_value_claim", validate_report(changed))


if __name__ == "__main__":
    unittest.main()
