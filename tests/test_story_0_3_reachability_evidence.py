from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.story_0_3_reachability_evidence import (
    DEFAULT_OUTPUT,
    ReachabilityEvidenceError,
    check_bundle,
    sanitize_log,
    write_bundle,
)


class Story03ReachabilityEvidenceTests(unittest.TestCase):
    def test_committed_guarded_reachability_bundle_is_valid(self) -> None:
        self.assertEqual(check_bundle(DEFAULT_OUTPUT), [])

    def test_log_sanitizer_accepts_public_dmr_diagnostics(self) -> None:
        self.assertEqual(
            sanitize_log("time=now level=INFO msg=ready socket=/tmp/model-runner.sock\n"),
            "time=now level=INFO msg=ready socket=/tmp/model-runner.sock\n",
        )

    def test_log_sanitizer_rejects_paths_and_authentication_values(self) -> None:
        for value in (
            "open /home/example/private/file\n",
            "Authorization: Bearer value\n",
            "token=private\n",
        ):
            with self.subTest(value=value):
                with self.assertRaises(ReachabilityEvidenceError):
                    sanitize_log(value)

    def test_writer_refuses_to_overwrite_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)
            with self.assertRaises(ReachabilityEvidenceError):
                write_bundle(output, output / "result", "HEAD")


if __name__ == "__main__":
    unittest.main()
