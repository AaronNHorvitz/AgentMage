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
        self.assertEqual(len(summary["tasks"]), 10)
        self.assertEqual(summary["control_attempts"], {"total": 20, "passed": 10, "failed": 10})
        self.assertFalse(summary["generated_scoring"])
        self.assertIsNone(summary["model_score"])
        self.assertEqual(summary["collection_sha256"],
                         "0c2a0624df93085540f61c4677260490a9810a0325efd6bcb75db0dce1461570")
        for row in summary["tasks"]:
            self.assertFalse(row["base"]["passed"])
            task = next(t for t in evaluation.collection() if t["id"] == row["task"])
            self.assertEqual(row["reference"]["tests"]["tests"], task["expected_tests"])
            self.assertEqual(row["reference"]["tests"]["errors"], 0)
            self.assertGreater(row["reference"]["calls"], 0)
            self.assertTrue(row["reference"]["candidate_ready"])
            self.assertTrue(row["reference"]["function_found"])
            self.assertTrue(oracle.expected_base(
                next(t for t in evaluation.collection() if t["id"] == row["task"]), row["base"]))
            self.assertFalse(row["reference"]["aggregate_worker_quotas"])

    def test_candidate_launch_failure_cannot_validate_either_failing_base(self):
        real_sandbox, real_grade = oracle.sandbox, oracle.grade

        def fail_candidate(workspace, adapter, output, role):
            if role == "candidate":
                return ["/usr/bin/false"]
            return real_sandbox(workspace, adapter, output, role)

        def fail_base(task, files, output):
            if evaluation.require_historical_candidate(task, files) == "frozen-base":
                with mock.patch.object(oracle, "sandbox", side_effect=fail_candidate):
                    return real_grade(task, files, output)
            return real_grade(task, files, output)

        with mock.patch.object(oracle, "grade", side_effect=fail_base):
            summary = oracle.verify(self.root / "failed-launches")
        self.assertFalse(summary["passed"])
        self.assertFalse(summary["input_refused"])
        for row in summary["tasks"]:
            self.assertFalse(row["valid"])
            self.assertTrue(row["reference"]["passed"], row)
            base = row["base"]
            self.assertEqual(base["cause"], "infrastructure")
            self.assertFalse(base["candidate_ready"])
            self.assertIsNone(base["function_found"])
            self.assertEqual(base["calls"], 0)
            self.assertEqual(base["tests"]["tests"], 0)
            self.assertEqual(base["logs"]["wire.log"], evaluation.sha(b""))
            self.assertEqual(base["control_attempts"], {"total": 1, "passed": 0, "failed": 1})

    def test_import_failure_and_false_startup_do_not_run_hidden_assertions(self):
        for name, code in (
                ("import-failure", b'\nraise RuntimeError("synthetic import failure")\n'),
                ("false-startup", b'\nimport os\nos.write(1, b\'{"ready":false,"function_found":true}\\n\')\n')):
            with self.subTest(name=name):
                report = self.probe(code, name)
                self.assertFalse(report["passed"])
                self.assertEqual(report["cause"], "infrastructure")
                self.assertFalse(report["candidate_ready"])
                self.assertEqual(report["calls"], 0)
                self.assertEqual(report["tests"]["tests"], 0)
                self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})

    def test_verification_fixtures_cannot_patch_or_write_trusted_assertions(self):
        for ident in ("dev-020", "dev-021", "dev-022"):
            task = next(t for t in evaluation.collection() if t["id"] == ident)
            files = self.files(task)
            files[task["source"]] += b'''
import unittest
from pathlib import Path
unittest.TestCase.assertFalse = lambda *a, **kw: None
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
assert not Path('/lane').exists()
def verify_case(*args):
 return {'passed': True, 'checks': {'retained-muse-read-arguments': True,
  'no-rejection-authority-or-effects': True, 'bounded-tools': True,
  'terminal-rejection-code': True}, 'tests': {'failures': 0}}
'''
            with self.subTest(task=ident), mock.patch.object(
                    evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, self.root / ident)
            self.assertFalse(report["passed"], report)
            self.assertEqual(report["cause"], "test_failure")
            self.assertTrue(report["candidate_ready"])
            self.assertEqual(report["tests"]["tests"], 1)
            self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})
            self.assertIn("other-report-fields", report["coverage"]["not_graded"])

    def test_new_verification_controls_refuse_generated_snapshots_and_hide_exception_data(self):
        for ident in ("dev-020", "dev-021", "dev-022"):
            task = next(t for t in evaluation.collection() if t["id"] == ident)
            files = self.files(task)
            files[task["source"]] += b"\ndef verify_case(*args): raise KeyError('untrusted exception payload')\n"
            output = self.root / ident
            with self.subTest(task=ident), self.assertRaisesRegex(
                    evaluation.EvaluationError, "untrusted-candidate-disabled"):
                oracle.grade(task, files, output)
            self.assertFalse(output.exists())
            with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, output)
            self.assertFalse(report["passed"])
            self.assertEqual(report["tests"]["errors"], 1)
            self.assertFalse(report["protocol_failed"])
            self.assertNotIn("untrusted exception payload", (output / "wire.log").read_text())
            self.assertNotIn("untrusted exception payload", (output / "oracle.log").read_text())

    def test_argument_controls_refuse_generated_sources_before_output_or_launch(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-006", "dev-007", "dev-027"}):
            files = self.files(task)
            files[task["source"]] += b"\n# synthetic candidate\n"
            output = self.root / task["id"]
            with self.subTest(task=task["id"]), mock.patch.object(oracle, "exchange") as launch:
                with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                    oracle.grade(task, files, output)
                launch.assert_not_called()
                self.assertFalse(output.exists())

    def test_argument_assertions_survive_candidate_patching_and_wrong_order(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-006", "dev-007", "dev-027"}):
            name = oracle.SUPPORTED[task["id"]]["function"]
            files = self.files(task)
            files[task["source"]] += ('''
import unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertRaises = lambda *args, **kwargs: None
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
original_arguments = %s
def %s(arguments):
 return list(reversed(original_arguments(arguments)))
''' % (name, name)).encode()
            with self.subTest(task=task["id"]), mock.patch.object(
                    evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, self.root / task["id"])
            self.assertFalse(report["passed"], report)
            self.assertEqual(report["cause"], "test_failure")
            self.assertEqual(report["tests"]["failures"], 1)
            self.assertFalse(report["protocol_failed"])

    def test_argument_exception_messages_stay_out_of_trusted_logs(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-006")
        files = self.files(task)
        files[task["source"]] += b'''
original_arguments = route_grant_arguments
def route_grant_arguments(arguments):
 try:
  return original_arguments(arguments)
 except HarnessError:
  raise HarnessError('synthetic-private-exception')
'''
        output = self.root / "exception-tag"
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, output)
        self.assertTrue(report["passed"], report)
        for name in ("wire.log", "oracle.log"):
            self.assertNotIn("synthetic-private-exception", (output / name).read_text())

    def test_argument_type_erasure_and_boolean_policy_are_explicit(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-006")
        files = self.files(task)
        files[task["source"]] += b'''
original_arguments = route_grant_arguments
def route_grant_arguments(arguments):
 return tuple(original_arguments(arguments))
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "container-erasure")
        self.assertTrue(report["passed"], report)
        self.assertIn("argument-return-container-type", report["coverage"]["not_graded"])
        task = next(t for t in evaluation.collection() if t["id"] == "dev-022")
        files = self.files(task)
        files[task["source"]] += b'''
original_verify_case = verify_case
def verify_case(*args):
 result = original_verify_case(*args)
 return {'passed': int(result['passed']), 'checks': {key: int(value) for key, value in result['checks'].items()}}
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "strict-policy")
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "protocol")
        self.assertEqual(report["coverage"]["adapter_policy"], ["strict-boolean-result-type"])

    def test_malformed_parser_namespace_cannot_reconstruct_objects_or_forge_results(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-006")
        files = self.files(task)
        files[task["source"]] += b'''
class HostileParser:
 def parse_args(self, arguments):
  return argparse.Namespace(root={'__reduce__': 'payload'})
def parser(): return HostileParser()
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "hostile-namespace")
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "protocol")
        self.assertEqual(report["tests"]["errors"], 1)

    def test_verification_preamble_on_trusted_constants_is_explicitly_ungraded(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-022")
        files = self.files(task)
        files[task["source"]] += b"\nCASE_ROOTS = {name: 'duplicate' for name in CASE_ROOTS}\n"
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "ungraded-case-roots")
        self.assertTrue(report["passed"], report)
        self.assertIn("candidate-case-root-uniqueness", report["coverage"]["not_graded"])

    def test_path_return_types_are_explicitly_outside_oracle_coverage(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-010")
        files = self.files(task)
        files[task["source"]] += b'''
original_test_only_files = test_only_files
def test_only_files(sources):
 return [str(path) for path in original_test_only_files(sources)]
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "ungraded-return-types")
        self.assertTrue(report["passed"], report)
        self.assertEqual(report["coverage"]["not_graded"], [
            "return-container-type", "return-element-type", "unselected-statement-clauses"])

    def test_only_the_expected_regression_outcome_validates_a_base(self):
        for task in (t for t in evaluation.collection() if t["id"] in oracle.SUPPORTED):
            base = oracle.grade(task, evaluation.projection(task), self.root / task["id"])
            self.assertTrue(oracle.expected_base(task, base), base)
            for update in ({"cause": "infrastructure"}, {"cause": "candidate_exit"},
                           {"candidate_ready": False}, {"function_found": None},
                           {"calls": base["calls"] + 1}, {"exit_code": -9},
                           {"candidate_exit_code": -9}, {"candidate_kind": "frozen-reference"},
                           {"tests": {**base["tests"], "errors": 5}}, {"passed": True}):
                with self.subTest(task=task["id"], update=update):
                    self.assertFalse(oracle.expected_base(task, {**base, **update}))

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

    def test_path_function_assertions_and_exception_checks_stay_in_the_oracle(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-010")
        for name, source in (
                ("path-assertion-patch", b'''
import unittest
unittest.TestCase.assertEqual = lambda *args, **kw: None
def test_only_files(sources): return frozenset()
'''),
                ("foreign-path", b"\ndef test_only_files(sources): return frozenset([Path('/output/tests.json')])\n"),
                ("always-ambiguous", b"\ndef test_only_files(sources): raise AmbiguousReference('forged payload')\n")):
            files = self.files(task)
            files[task["source"]] += source
            with self.subTest(probe=name), mock.patch.object(
                    evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                report = oracle.grade(task, files, self.root / name)
            self.assertFalse(report["passed"], report)
            self.assertTrue(report["candidate_ready"])
            self.assertEqual(report["tests"]["tests"], 1)
            self.assertGreater(report["tests"]["failures"] + report["tests"]["errors"], 0)
            self.assertNotIn("forged payload", (self.root / name / "oracle.log").read_text())

    def test_new_path_controls_still_refuse_generated_source_before_launch(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-010", "dev-012"}):
            files = self.files(task)
            files[task["source"]] += b"\n# generated\n"
            output = self.root / task["id"]
            with self.subTest(task=task["id"]), self.assertRaisesRegex(
                    evaluation.EvaluationError, "untrusted-candidate-disabled"):
                oracle.grade(task, files, output)
            self.assertFalse(output.exists())

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
                 "calls": 4, "protocol_failed": False,
                 "candidate_ready": True, "function_found": True}
        for update in ({"tests": {}}, {"calls": True}, {"calls": 0},
                       {"protocol_failed": "false"}, {"tests": {**valid["tests"], "tests": 0}},
                       {"tests": {**valid["tests"], "failures": False}},
                       {"candidate_ready": 1}, {"function_found": 1},
                       {"candidate_ready": False}, {"function_found": None}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)
        for field in ("skipped", "expected_failures", "unexpected_successes"):
            record = {**valid, "tests": {**valid["tests"], field: 1}}
            with mock.patch.object(oracle, "grade_case", return_value=(0, 0, False, False, record)):
                report = oracle.grade(self.task, self.files(), self.root / field)
            self.assertFalse(report["passed"], report)

    def test_startup_failure_record_requires_zero_assertions_and_no_function(self):
        counts = {name: 0 for name in ("tests", "failures", "errors", "skipped",
                                       "expected_failures", "unexpected_successes")}
        valid = {"tests": counts, "calls": 0, "protocol_failed": True,
                 "candidate_ready": False, "function_found": None}
        path = self.root / "startup.json"
        path.write_text(json.dumps(valid))
        self.assertEqual(oracle.result_record(path, self.task), valid)
        for update in ({"calls": 1}, {"protocol_failed": False}, {"function_found": False},
                       {"tests": {**counts, "tests": 1}}, {"tests": {**counts, "errors": 1}}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)

    def test_every_reported_failure_stays_in_control_attempt_counts(self):
        counts = {name: 0 for name in ("failures", "errors", "skipped",
                                       "expected_failures", "unexpected_successes")}
        ready = {"tests": {**counts, "tests": 1}, "calls": 4, "protocol_failed": False,
                 "candidate_ready": True, "function_found": True}
        not_ready = {"tests": {**counts, "tests": 0}, "calls": 0, "protocol_failed": True,
                     "candidate_ready": False, "function_found": None}
        for cause, result in (
                ("none", (0, 0, False, False, ready)),
                ("infrastructure", (1, 1, False, False, not_ready)),
                ("infrastructure", (1, 1, False, False, None)),
                ("timeout", (None, None, True, False, None)),
                ("output_limit", (None, None, False, True, None)),
                ("protocol", (1, 1, False, False, {**ready, "protocol_failed": True})),
                ("candidate_exit", (0, 7, False, False, ready)),
                ("test_failure", (1, 0, False, False, {
                    **ready, "tests": {**ready["tests"], "failures": 1}})),
                ("input_integrity", evaluation.InputIntegrityError("evaluation.input-hash-mismatch")),
                ("infrastructure", OSError("synthetic launch failure"))):
            path = self.root / str(len(list(self.root.iterdir())))
            with self.subTest(cause=cause), mock.patch.object(oracle, "grade_case") as grade_case:
                if isinstance(result, Exception):
                    grade_case.side_effect = result
                else:
                    grade_case.return_value = result
                report = oracle.grade(self.task, self.files(), path)
            self.assertEqual(report["cause"], cause)
            self.assertEqual(report["control_attempts"], {
                "total": 1, "passed": int(cause == "none"), "failed": int(cause != "none")})


if __name__ == "__main__":
    unittest.main()
