from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scripts.story_0_3_macos_evidence import (
    Story03MacEvidenceError,
    adapter_disposition,
    build_bundle,
    sanitize_server_log,
    source_receipt,
    write_bundle,
)


ROOT = Path(__file__).resolve().parents[1]


def fixture_result() -> dict[str, object]:
    result = json.loads(
        (ROOT / "artifacts/sprints/sprint-0/story-0.3/native-linux-result.json").read_text(
            encoding="utf-8"
        )
    )
    result["adapter_id"] = "macos-native-metal"
    result["completed_at_epoch"] = 1786320000
    result["runtime_settings"]["hardware"] = {
        "platform": "macOS",
        "architecture": "arm64",
        "machine_name": "MacBook Pro",
        "chip": "Apple M5",
    }
    return result


class Story03MacEvidenceTests(unittest.TestCase):
    def test_server_log_sanitizer_replaces_only_evaluation_root(self) -> None:
        source = "load /Users/example/.local/share/agentmage/evaluation/models/model.gguf\n"
        self.assertEqual(
            sanitize_server_log(source),
            "load <EVALUATION_DATA_ROOT>/models/model.gguf\n",
        )
        with self.assertRaises(Story03MacEvidenceError):
            sanitize_server_log("load /Users/example/Documents/private.gguf\n")

    def test_disposition_records_result_without_reversing_rejection(self) -> None:
        disposition = adapter_disposition(fixture_result(), "0" * 40)
        self.assertEqual(disposition["task_0_3_2_2_status"], "COMPLETE")
        self.assertEqual(disposition["macos_native"]["quality_status"], "FAIL")
        self.assertEqual(disposition["controlling_profile_decision"], "REJECTED")
        self.assertFalse(disposition["candidate_enabled"])
        self.assertFalse(disposition["release_approval"])

    def test_source_receipt_binds_raw_and_admitted_files_without_a_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            (source / "manifest.json").write_bytes(b"manifest")
            (source / "results.json").write_bytes(b"result")
            (source / "server.log").write_bytes(b"raw")
            receipt = source_receipt(source, b"sanitized")
            self.assertNotIn(str(source), json.dumps(receipt))
            self.assertTrue(receipt["source_directory_retained_outside_repository"])
            self.assertFalse(
                receipt["admitted_files"]["macos-native-server.log"]["byte_identical_to_source"]
            )

    @patch("scripts.story_0_3_macos_evidence.validate_directory", return_value=[])
    def test_bundle_builds_from_a_valid_source_and_sanitizes_log(self, _validate) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            result_bytes = (json.dumps(fixture_result(), sort_keys=True) + "\n").encode()
            (source / "results.json").write_bytes(result_bytes)
            (source / "server.log").write_text(
                "load /Users/example/.local/share/agentmage/evaluation/model.gguf\n",
                encoding="utf-8",
            )
            (source / "manifest.json").write_text("{}\n", encoding="utf-8")
            bundle = build_bundle(source, "0" * 40)
            self.assertEqual(bundle["macos-native-result.json"], result_bytes)
            self.assertNotIn(b"/Users/example", bundle["macos-native-server.log"])
            self.assertIn(b'"controlling_profile_decision": "REJECTED"', bundle["adapter-disposition.json"])

    @patch(
        "scripts.story_0_3_macos_evidence.validate_directory",
        return_value=["fixture invalid"],
    )
    def test_invalid_source_is_rejected_before_admission(self, _validate) -> None:
        with self.assertRaisesRegex(Story03MacEvidenceError, "fixture invalid"):
            build_bundle(Path("unread"), "0" * 40)

    def test_writer_refuses_to_overwrite_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with self.assertRaises(Story03MacEvidenceError):
                write_bundle(output, output / "source", "HEAD")


if __name__ == "__main__":
    unittest.main()
