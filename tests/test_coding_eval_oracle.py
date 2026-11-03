"""Measure frozen controls and result-forgery probes in separate real sandboxes."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_oracle as oracle


class IndependentOracleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="agentmage-oracle-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.task = next(t for t in evaluation.collection() if t["id"] == "dev-015")

    def files(self, task=None, fixed=True):
        task = task or self.task
        files = evaluation.projection(task)
        if fixed:
            files[task["source"]] = evaluation.pinned_blob(
                task["reference"], task["source"], task["reference_sha256"])
        return files

    def probe(self, source, name):
        # Builder-authored diagnostics only. Real CLI/grade admission is tested
        # separately, and never accepts these modified snapshots.
        files = self.files()
        files[self.task["source"]] += source
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            return oracle.grade(self.task, files, self.root / name)

    def test_real_two_sandbox_controls_keep_original_hidden_tests_and_counts(self):
        summary = oracle.verify(self.root / "controls")
        self.assertTrue(summary["passed"], summary)
        self.assertEqual(len(summary["tasks"]), 2)
        self.assertFalse(summary["generated_scoring"])
        self.assertIsNone(summary["model_score"])
        self.assertEqual(summary["collection_sha256"],
                         "0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570")
        for row in summary["tasks"]:
            self.assertFalse(row["base"]["passed"])
            self.assertEqual(row["reference"]["tests"]["tests"], 1)
            self.assertEqual(row["reference"]["tests"]["errors"], 0)
            self.assertGreater(row["reference"]["calls"], 0)
            self.assertFalse(row["reference"]["aggregate_worker_quotas"])

    def test_candidate_cannot_patch_assertions_or_write_the_oracle_result(self):
        report = self.probe(b'''
import unittest
from pathlib import Path
unittest.TestCase.assertFalse = lambda *args, **kw: None
unittest.TestCase.assertTrue = lambda *args, **kw: None
try:
 Path('/output/tests.json').write_text('{"tests":1,"failures":0}')
except OSError:
 pass
print('Ran 1 test in 0s\\nOK')
def case_passed(case): return True
''', "forgery")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["cause"], "test_failure")
        self.assertEqual(report["tests"]["failures"], 1)
        self.assertIn("OK", (self.root / "forgery/candidate.log").read_text())
        self.assertIn("FAILED", (self.root / "forgery/oracle.log").read_text())

    def test_candidate_has_no_hidden_tests_result_mount_or_host_home(self):
        report = self.probe(b'''
import json
from pathlib import Path
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
assert not Path('/grader').exists()
assert not Path('/lane').exists()
assert not Path('/var/home').exists()
assert set(json.loads(Path('/adapter/config.json').read_text())) == {'module','function','result_type'}
original_case_passed = case_passed
def case_passed(case):
 return original_case_passed(case)
''', "mounts")
        self.assertTrue(report["passed"], report)
        self.assertIsNone(report["model_score"])

    def test_forged_frames_early_exit_and_oversize_results_cannot_pass(self):
        for name, source in (
            ("early-exit", b"\nimport os\nos._exit(0)\n"),
            ("forged-counts", b"\ndef case_passed(case): return {'tests':1,'failures':0}\n"),
            ("oversize", b"\ndef case_passed(case): return 'x' * (128 * 1024)\n"),
            ("duplicate-key", b'''
import os
def case_passed(case):
 os.write(1, b'{"seq":1,"seq":1,"result":true}\\n')
 return True
'''),
        ):
            with self.subTest(probe=name):
                report = self.probe(source, name)
                self.assertFalse(report["passed"], report)
                self.assertNotEqual(report["cause"], "none")
        self.assertIn(b'"seq":1,"seq":1', (self.root / "duplicate-key/wire.log").read_bytes())

    def test_generated_and_unsupported_tasks_are_refused_before_output_creation(self):
        files = self.files()
        files[self.task["source"]] += b"\n# generated snapshot\n"
        with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
            oracle.grade(self.task, files, self.root / "generated")
        self.assertFalse((self.root / "generated").exists())
        other = next(t for t in evaluation.collection() if t["id"] == "dev-001")
        with self.assertRaisesRegex(evaluation.EvaluationError, "oracle.task-refused"):
            oracle.grade(other, {}, self.root / "unsupported")
        self.assertFalse((self.root / "unsupported").exists())

    def test_correct_replies_followed_by_candidate_failure_cannot_pass(self):
        report = self.probe(b'''
import json
import os
original_case_passed = case_passed
calls = 0
def case_passed(case):
 global calls
 calls += 1
 result = original_case_passed(case)
 if calls == 4:
  os.write(1, (json.dumps({'seq':calls,'result':result})+'\\n').encode())
  os._exit(7)
 return result
''', "late-failure")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["exit_code"], 0)
        self.assertEqual(report["candidate_exit_code"], 7)
        self.assertEqual(report["cause"], "candidate_exit")
        self.assertEqual(report["tests"]["failures"], 0)

    def test_public_cli_exposes_no_generated_scoring_operation(self):
        result = subprocess.run([sys.executable, str(Path(oracle.__file__)), "score",
                                 "--output", str(self.root / "score")], capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 2)
        self.assertFalse((self.root / "score").exists())

    def test_sandboxes_have_disjoint_code_and_only_oracle_has_result_mount(self):
        candidate = oracle.sandbox(Path("candidate"), Path("candidate-adapter"), Path("result"), "candidate")
        trusted = oracle.sandbox(Path("trusted"), Path("oracle-adapter"), Path("result"), "oracle")
        self.assertNotIn("--bind", candidate)
        self.assertIn("--bind", trusted)
        for command in (candidate, trusted):
            self.assertIn("--unshare-all", command)
            self.assertIn("--disable-userns", command)
            self.assertIn("--clearenv", command)
        self.assertNotIn("oracle-adapter", candidate)
        self.assertNotIn("candidate", trusted)

    def test_timeout_and_log_budget_stop_both_owned_process_groups(self):
        for name, code, timeout in (
            ("timeout", "import time; time.sleep(20)", 0.2),
            ("output", "import os; os.write(2,b'x'*20000)", 5),
        ):
            output = self.root / name
            output.mkdir()
            commands = [sys.executable, "-c", code]
            spawned = []
            real_popen = subprocess.Popen

            def spawn(*args, **kwargs):
                process = real_popen(*args, **kwargs)
                spawned.append(process)
                return process

            with mock.patch.object(oracle, "LOG_LIMIT", 4096), mock.patch.object(
                    oracle.subprocess, "Popen", side_effect=spawn):
                _, _, timed_out, limited = oracle.exchange(commands, commands, output, timeout)
            self.assertEqual(timed_out, name == "timeout")
            self.assertEqual(limited, name == "output")
            self.assertEqual(len(spawned), 2)
            for process in spawned:
                self.assertIsNotNone(process.returncode)
                with self.assertRaises(ProcessLookupError):
                    os.killpg(process.pid, 0)
            for path in output.iterdir():
                self.assertLessEqual(path.stat().st_size, 4096)

    def test_second_launch_failure_and_interruption_clean_first_process(self):
        for name, error in (("launch", OSError("synthetic launch failure")),
                            ("interrupt", KeyboardInterrupt())):
            spawned = []
            output = self.root / name
            output.mkdir()
            real_popen = subprocess.Popen

            def spawn(*args, **kwargs):
                if spawned:
                    raise error
                process = real_popen(*args, **kwargs)
                spawned.append(process)
                return process

            with mock.patch.object(oracle.subprocess, "Popen", side_effect=spawn):
                with self.assertRaises(type(error)):
                    oracle.exchange([sys.executable, "-c", "import time; time.sleep(20)"],
                                    [sys.executable, "-c", "pass"], output, 5)
            self.assertIsNotNone(spawned[0].returncode)
            with self.assertRaises(ProcessLookupError):
                os.killpg(spawned[0].pid, 0)

    def test_interruption_during_exchange_cleans_both_processes(self):
        output = self.root / "active-interruption"
        output.mkdir()
        spawned = []
        real_popen = subprocess.Popen

        def spawn(*args, **kwargs):
            process = real_popen(*args, **kwargs)
            spawned.append(process)
            return process

        selector = oracle.selectors.DefaultSelector
        with mock.patch.object(oracle.subprocess, "Popen", side_effect=spawn), mock.patch.object(
                selector, "select", side_effect=KeyboardInterrupt()):
            with self.assertRaises(KeyboardInterrupt):
                command = [sys.executable, "-c", "import time; time.sleep(20)"]
                oracle.exchange(command, command, output, 5)
        self.assertEqual(len(spawned), 2)
        for process in spawned:
            self.assertIsNotNone(process.returncode)
            with self.assertRaises(ProcessLookupError):
                os.killpg(process.pid, 0)

    def test_cleanup_failure_never_promotes_counts_or_success(self):
        files = self.files()
        remove = evaluation.remove_case_workspace

        def cleanup_failure(root, output):
            remove(root, output)
            raise evaluation.EvaluationError("evaluation.cleanup-failed")

        with mock.patch.object(evaluation, "remove_case_workspace", side_effect=cleanup_failure):
            report = oracle.grade(self.task, files, self.root / "cleanup")
        self.assertFalse(report["passed"])
        self.assertIsNone(report["tests"])
        self.assertEqual(report["cause"], "infrastructure")
        self.assertNotIn("private failure", json.dumps(report))
        self.assertFalse((self.root / "cleanup/tests.json").exists())
        self.assertFalse((self.root / "cleanup/wire.log").exists())

    def test_input_integrity_failure_is_not_a_failed_test_control(self):
        files = self.files()
        with mock.patch.object(oracle, "grade_case", side_effect=evaluation.InputIntegrityError(
                "evaluation.input-hash-mismatch")):
            report = oracle.grade(self.task, files, self.root / "input")
        self.assertEqual(report["cause"], "input_integrity")
        self.assertIsNone(report["tests"])
        self.assertFalse(report["passed"])

    def test_missing_counts_skips_and_wrong_count_cannot_be_results(self):
        path = self.root / "tests.json"
        valid = {"tests": {"tests": 1, "failures": 0, "errors": 0, "skipped": 0,
                           "expected_failures": 0, "unexpected_successes": 0},
                 "calls": 4, "protocol_failed": False}
        for update in ({"tests": {}}, {"calls": True}, {"calls": 0},
                       {"protocol_failed": "false"}, {"tests": {**valid["tests"], "tests": 0}},
                       {"tests": {**valid["tests"], "failures": False}}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)
        for field in ("skipped", "expected_failures", "unexpected_successes"):
            record = {**valid, "tests": {**valid["tests"], field: 1}}
            with mock.patch.object(oracle, "grade_case", return_value=(0, 0, False, False, record)):
                report = oracle.grade(self.task, self.files(), self.root / field)
            self.assertFalse(report["passed"], report)


if __name__ == "__main__":
    unittest.main()
