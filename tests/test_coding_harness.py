import os
from pathlib import Path
import stat
import tempfile
import unittest
from unittest import mock

from scripts import coding_harness


class CodingHarnessTests(unittest.TestCase):
    def test_setup_creates_one_private_clean_path_bound_fixture(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary) / "fixture"
            with mock.patch.object(coding_harness, "binary", side_effect=lambda name: Path("/bin/true")):
                result = coding_harness.setup(base)
            workspace = base / "disposable/worktree"
            self.assertTrue(result["activation"])
            self.assertTrue(result["git_clean"])
            self.assertEqual(result["git_branch"], f"refs/heads/{coding_harness.BRANCH}")
            self.assertEqual(result["qualification"], "executable-scripted-only")
            for path in (base, base / "state", base / "disposable", workspace):
                self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE((workspace / coding_harness.MARKER).stat().st_mode), 0o600)
            with self.assertRaises(coding_harness.HarnessError):
                coding_harness.setup(base)

    def test_pid_reuse_and_substitution_cannot_receive_stop_signal(self):
        record = {"pid": os.getpid(), "base": "/different", "binary": "/bin/true", "started_at_epoch_ms": 1}
        self.assertFalse(coding_harness.exact_running_process(record, Path("/expected")))


if __name__ == "__main__":
    unittest.main()
