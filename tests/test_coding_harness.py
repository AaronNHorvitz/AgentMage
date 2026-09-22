import os
from pathlib import Path
import stat
import subprocess
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

    def test_gpu_observation_requires_one_exact_numeric_device(self):
        completed = subprocess.CompletedProcess([], 0, stdout="1234, 23000\n", stderr="")
        with mock.patch.object(coding_harness.subprocess, "run", return_value=completed):
            self.assertEqual(coding_harness.gpu_memory(), (1234, 23000))
        for output in ("", "1, 2\n3, 4\n", "used, free\n"):
            completed = subprocess.CompletedProcess([], 0, stdout=output, stderr="")
            with mock.patch.object(coding_harness.subprocess, "run", return_value=completed):
                with self.assertRaises(coding_harness.HarnessError):
                    coding_harness.gpu_memory()

    def test_candidate_guard_cancels_only_its_run_on_sampled_pressure(self):
        guard = coding_harness.CandidateResourceGuard(
            -1,
            {"maximum_total_vram_used_mib": 22528},
            {"memory.peak": "0"},
            1200,
            23000,
        )
        with mock.patch.object(coding_harness, "gpu_memory", return_value=(22529, 1000)):
            self.assertFalse(guard.sample())
        self.assertEqual(
            guard.error,
            "coding.harness.candidate.gpu-headroom-exceeded",
        )
        self.assertEqual(guard.peak_total_gpu_used_mib_sampled, 22529)


if __name__ == "__main__":
    unittest.main()
