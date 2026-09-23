import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest
from unittest import mock

from scripts import coding_harness
from scripts import coding_harness_acceptance


class CodingHarnessTests(unittest.TestCase):
    def test_multifile_regression_requires_nine_tools_and_recording_beyond_old_limit(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary) / "fixture"
            logs = Path(temporary) / "logs"
            logs.mkdir()
            (logs / "result.json").write_text(json.dumps({"scenario": "multi-file"}))
            (logs / "stdout.jsonl").write_text("")
            (logs / "stderr.log").write_text("")
            outcome = {"state": "SUCCESS", "tool_call_count": 9}
            records = [{"type": "runtime_artifact_verified", "artifact_id": f"a-{index}"}
                       for index in range(35)]
            rows = [*records, outcome]
            with mock.patch.object(coding_harness_acceptance, "git_status", return_value=[
                    " M src/calc.py", " M src/subtract.py"]), \
                    mock.patch.object(coding_harness_acceptance, "rows", return_value=rows), \
                    mock.patch.object(coding_harness_acceptance.subprocess, "run",
                                      return_value=subprocess.CompletedProcess([], 0)):
                self.assertTrue(coding_harness_acceptance.verify_case(
                    "multi-file", base, logs, 0)["passed"])
                del rows[:2]
                self.assertFalse(coding_harness_acceptance.verify_case(
                    "multi-file", base, logs, 0)["passed"])
                rows[:0] = records[:2]
                outcome["tool_call_count"] = 6
                self.assertFalse(coding_harness_acceptance.verify_case(
                    "multi-file", base, logs, 0)["passed"])

    def test_native_command_failure_artifact_must_belong_to_exact_command_turn(self):
        self.assertEqual(len(set(coding_harness_acceptance.CASE_ROOTS.values())),
                         len(coding_harness_acceptance.CASE_ROOTS))
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary) / "fixture"
            workspace = base / "disposable/worktree"
            (workspace / "src").mkdir(parents=True)
            (workspace / "src/calc.py").write_text("def broken_add(left, right):\n    return left + right\n")
            logs = Path(temporary) / "logs"
            logs.mkdir()
            (logs / "result.json").write_text(json.dumps({"scenario": "native-command-failure"}))
            (logs / "stdout.jsonl").write_text("")
            (logs / "stderr.log").write_text("")
            rows = [{"turn_id": "turn-0", "kind": {
                "event": "tool_failed", "tool_call_id": "scripted-native-ordered-command",
                "failure_code": "runtime.tool.failed", "receipt_id": "receipt-command"}},
                {"turn_id": "turn-0", "kind": {"event": "tool_requested",
                 "tool_call_id": "scripted-native-ordered-command",
                 "arguments_sha256": "87565c772c7ae8b4c39bfe07ac130422afadac4bb90b44e3ceaccc73a0aad922"}}]
            failed = (json.dumps({
                "schema_version": 1, "status": "assertion_failed", "passed": 0, "failed": 1,
                "skipped": 0, "duration_ms": 1, "failed_names": ["fixture::add"], "artifact_ids": [],
                "retry_count": 0, "initial_failure_sha256": None,
            }, sort_keys=True) + "\n").encode()
            artifact = {"turn_id": "turn-0", "kind": {"event": "artifact_created", "artifact_id": "failure"}}
            verified = {"type": "runtime_artifact_verified", "artifact_id": "failure",
                        "payload_sha256": coding_harness_acceptance.hashlib.sha256(failed).hexdigest(),
                        "byte_size": len(failed)}
            rows.extend([artifact, verified, {"state": "FAILED", "turn_count": 1,
                                             "evidence": [], "unresolved_codes": ["runtime.tool.failed"]}])
            with mock.patch.object(coding_harness_acceptance, "git_status", return_value=[]), \
                    mock.patch.object(coding_harness_acceptance, "rows", return_value=rows):
                self.assertTrue(coding_harness_acceptance.verify_case(
                    "native-command-failure", base, logs, 8)["passed"])
                artifact["turn_id"] = "turn-1"
                self.assertFalse(coding_harness_acceptance.verify_case(
                    "native-command-failure", base, logs, 8)["passed"])
                artifact["turn_id"] = "turn-0"
                verified["payload_sha256"] = "0" * 64
                self.assertFalse(coding_harness_acceptance.verify_case(
                    "native-command-failure", base, logs, 8)["passed"])

    def test_acceptance_retains_prelaunch_failure_without_outcome(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary) / "fixture"
            with mock.patch.object(coding_harness, "binary", return_value=Path("/bin/true")):
                coding_harness.setup(base)
            logs = Path(temporary) / "logs"
            logs.mkdir()
            (logs / "stdout.jsonl").write_text("")
            (logs / "stderr.log").write_text("client.protocol.value_invalid\n")
            (logs / "result.json").write_text(json.dumps({"scenario": "protocol-correction"}))
            report = coding_harness_acceptance.verify_case("protocol-correction", base, logs, 2)
            self.assertFalse(report["passed"])
            self.assertFalse(report["checks"]["exit"])
            self.assertFalse(report["checks"]["terminal"])
            self.assertEqual(report["event_count"], 0)

    def test_implementation_identity_pins_source_and_all_three_binaries(self):
        with mock.patch.object(coding_harness, "binary", return_value=Path("/usr/bin/true")):
            identity = coding_harness.implementation_identity()
        self.assertEqual(len(identity["head_commit"]), 40)
        self.assertEqual(len(identity["binaries"]), 3)
        self.assertTrue(all(len(item["sha256"]) == 64 and item["bytes"] > 0 for item in identity["binaries"]))
        self.assertEqual(len(identity["tracked_diff_sha256"]), 64)
        self.assertEqual(identity["wrapper"]["path"], str(Path(coding_harness.__file__).resolve()))

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
