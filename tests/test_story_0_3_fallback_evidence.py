from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from scripts.story_0_3_fallback_evidence import (
    DEFAULT_OUTPUT,
    FallbackEvidenceError,
    check_bundle,
    failed_cases,
    failed_thresholds,
    sanitize_server_log,
    validate_expected_result,
    write_bundle,
)


class Story03FallbackEvidenceTests(unittest.TestCase):
    def test_committed_fallback_bundle_is_valid_and_disabled(self) -> None:
        self.assertEqual(check_bundle(DEFAULT_OUTPUT), [])

    def test_server_log_sanitizer_replaces_evaluation_root(self) -> None:
        source = (
            "loading /var/home/example/.local/share/agentmage/evaluation/models/model.gguf\n"
        )
        self.assertEqual(
            sanitize_server_log(source),
            "loading <EVALUATION_DATA_ROOT>/models/model.gguf\n",
        )

    def test_server_log_sanitizer_rejects_other_home_paths(self) -> None:
        with self.assertRaises(FallbackEvidenceError):
            sanitize_server_log("loading /home/example/private/model.gguf\n")

    def test_expected_result_rejects_changed_outcome(self) -> None:
        result = {
            "adapter_id": "linux-native-vulkan",
            "status": "PASS",
            "cases": [
                {
                    "case_id": "CITE-001",
                    "passed": False,
                    "trials_completed": 82,
                }
            ],
            "threshold_results": {
                "citation_recall": {"passed": False},
                "tool_call_valid_rate": {"passed": False},
            },
        }
        failures = validate_expected_result(result)
        self.assertTrue(any("mandatory failure" in item for item in failures))
        self.assertTrue(any("failed-case set changed" in item for item in failures))

    def test_failed_sets_are_derived_from_raw_results(self) -> None:
        result = {
            "cases": [
                {"case_id": "A", "passed": True},
                {"case_id": "B", "passed": False},
            ],
            "threshold_results": {
                "first": {"passed": True},
                "second": {"passed": False},
            },
        }
        self.assertEqual(failed_cases(result), ["B"])
        self.assertEqual(failed_thresholds(result), ["second"])

    def test_writer_refuses_to_overwrite_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with self.assertRaises(FallbackEvidenceError):
                write_bundle(output, output / "native", output / "dmr", "HEAD")


if __name__ == "__main__":
    unittest.main()
