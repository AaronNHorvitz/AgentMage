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
    def test_untracked_source_inventory_binds_new_bytes_and_ignores_build_outputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            (root / ".gitignore").write_text("target/\n")
            (root / "tracked.rs").write_text("tracked\n")
            subprocess.run(["git", "add", ".gitignore", "tracked.rs"], cwd=root, check=True)
            (root / "target").mkdir()
            (root / "target/binary").write_text("ignored\n")
            source = root / "new module.rs"
            source.write_text("first\n")
            first = coding_harness.untracked_file_identities(root)
            self.assertEqual([row["path"] for row in first], ["new module.rs"])
            self.assertEqual(first[0]["bytes"], 6)
            source.write_text("other\n")
            second = coding_harness.untracked_file_identities(root)
            self.assertNotEqual(first[0]["sha256"], second[0]["sha256"])
            self.assertEqual(first[0]["path"], second[0]["path"])

    def test_untracked_source_inventory_refuses_aliases_special_files_and_large_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            outside = Path(temporary) / "private"
            outside.write_text("not a source input\n")
            path = root / "new.rs"
            path.symlink_to(outside)
            with self.assertRaisesRegex(coding_harness.HarnessError, "source-inventory-alias"):
                coding_harness.untracked_file_identities(root)
            path.unlink()
            os.mkfifo(path)
            # Git omits a FIFO that already exists. Simulate a regular enumerated
            # source replaced by a FIFO before the held-file observation.
            listed = subprocess.CompletedProcess([], 0, stdout=b"new.rs\0")
            with mock.patch.object(coding_harness.subprocess, "run", return_value=listed):
                with self.assertRaisesRegex(coding_harness.HarnessError, "identity-not-regular"):
                    coding_harness.untracked_file_identities(root)
            path.unlink()
            with path.open("wb") as stream:
                stream.truncate(16 * 1024 * 1024 + 1)
            with self.assertRaisesRegex(coding_harness.HarnessError, "source-inventory-limit"):
                coding_harness.untracked_file_identities(root)

    def test_read_rejection_requires_exact_bytes_no_authority_and_terminal_denial(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary) / "fixture"
            workspace = base / "disposable/worktree"
            (workspace / "src").mkdir(parents=True)
            (workspace / "src/calc.py").write_text("def add(left, right):\n    return left + right\n")
            logs = Path(temporary) / "logs"
            logs.mkdir()
            (logs / "stdout.jsonl").write_text("")
            (logs / "stderr.log").write_text("")
            requested = {"turn_id": "rejected", "kind": {"event": "tool_requested",
                "tool_call_id": "scripted-invalid-read-paths",
                "arguments_sha256": "aa960a422a134655e7cc9eeebc922e74d9d10883d3fdc1e5721693c9544b5028"}}
            rejected = {"turn_id": "rejected", "kind": {"event": "tool_rejected", "reason": "arguments_invalid"}}
            effects = [{"turn_id": f"turn-{index}", "kind": {"event": "tool_completed", "tool_call_id": tool}}
                for index, tool in enumerate([
                    "scripted-validation-failing", "scripted-inspect-preimage", "scripted-repair-patch",
                    "scripted-validation-passing", "scripted-git-diff", "scripted-git-status"])]
            observed = [requested, rejected, *effects, {"type": "runtime_artifact_verified"},
                        {"state": "SUCCESS", "turn_count": 8}]
            (logs / "result.json").write_text(json.dumps({"scenario": "read-arguments-correction"}))
            with mock.patch.object(coding_harness_acceptance, "git_status", return_value=[" M src/calc.py"]), \
                    mock.patch.object(coding_harness_acceptance, "rows", return_value=observed):
                verify = lambda: coding_harness_acceptance.verify_case("read-arguments-correction", base, logs, 0)
                self.assertTrue(verify()["passed"])
                requested["kind"]["arguments_sha256"] = "0" * 64
                self.assertFalse(verify()["checks"]["retained-muse-read-arguments"])
                observed.insert(2, {"turn_id": "rejected", "kind": {"event": "permission_requested"}})
                self.assertFalse(verify()["checks"]["no-rejection-authority-or-effects"])
            observed[:] = [requested, rejected, {"state": "FAILED", "turn_count": 1,
                                                "unresolved_codes": ["runtime.proposal.invalid"]}]
            (logs / "result.json").write_text(json.dumps({"scenario": "read-arguments-denied"}))
            with mock.patch.object(coding_harness_acceptance, "git_status", return_value=[]), \
                    mock.patch.object(coding_harness_acceptance, "rows", return_value=observed):
                verify = lambda: coding_harness_acceptance.verify_case("read-arguments-denied", base, logs, 8)
                self.assertTrue(verify()["passed"])
                observed.insert(2, effects[0])
                self.assertFalse(verify()["checks"]["bounded-tools"])
                observed.pop(2)
                observed[-1]["unresolved_codes"] = []
                self.assertFalse(verify()["checks"]["terminal-rejection-code"])

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
        self.assertEqual(identity["untracked_files_sha256"], coding_harness.hashlib.sha256(
            json.dumps(identity["untracked_files"], sort_keys=True, separators=(",", ":"),
                       ensure_ascii=True).encode("ascii")).hexdigest())
        self.assertTrue(all(not Path(row["path"]).is_absolute()
                            and len(row["sha256"]) == 64 for row in identity["untracked_files"]))
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
