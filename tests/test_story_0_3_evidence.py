import shutil
import tempfile
import unittest
from pathlib import Path

from scripts.story_0_3_evidence import (
    DEFAULT_OUTPUT,
    Story03EvidenceError,
    check_bundle,
    sanitize_server_log,
)


class Story03EvidenceTests(unittest.TestCase):
    def test_committed_partial_evidence_reconciles(self):
        self.assertEqual(check_bundle(), [])

    def test_server_log_sanitizer_removes_local_home_identity(self):
        source = "loading /var/home/example/.local/share/agentmage/evaluation/models/model.gguf\n"
        sanitized = sanitize_server_log(source)
        self.assertEqual(
            sanitized,
            "loading <EVALUATION_DATA_ROOT>/models/model.gguf\n",
        )

    def test_server_log_sanitizer_rejects_other_sensitive_identifiers(self):
        with self.assertRaises(Story03EvidenceError):
            sanitize_server_log("contact person@example.test\n")

    def test_mutated_result_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            copied = Path(temporary) / "story-0.3"
            shutil.copytree(DEFAULT_OUTPUT, copied)
            result = copied / "native-linux-result.json"
            result.write_bytes(result.read_bytes() + b"\n")
            self.assertTrue(any("hash mismatch" in failure for failure in check_bundle(copied)))


if __name__ == "__main__":
    unittest.main()
