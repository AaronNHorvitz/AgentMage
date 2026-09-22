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

    def test_setup_builds_exact_new_and_multi_file_fixtures(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with mock.patch.object(coding_harness, "binary", side_effect=lambda name: Path("/bin/true")):
                new_file = root / "new"
                coding_harness.setup(new_file, "new-file")
                self.assertFalse((new_file / "disposable/worktree/src/calc.py").exists())
                multi_file = root / "multi"
                coding_harness.setup(multi_file, "multi-file")
            workspace = multi_file / "disposable/worktree"
            self.assertTrue((workspace / "src/calc.py").is_file())
            self.assertTrue((workspace / "src/subtract.py").is_file())
            validation = coding_harness.subprocess.run(
                ["python3", "tests/run_validation.py"], cwd=workspace, check=False,
                stdout=coding_harness.subprocess.PIPE, text=True,
            )
            self.assertNotEqual(validation.returncode, 0)


if __name__ == "__main__":
    unittest.main()
