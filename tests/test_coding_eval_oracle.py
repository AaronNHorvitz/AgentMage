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


WIRE_DESCRIPTOR_PROBE = b'''
import os, fcntl, stat
def protocol_fd():
 for fd in range(3, 64):
  try:
   if stat.S_ISFIFO(os.fstat(fd).st_mode) and fcntl.fcntl(fd, fcntl.F_GETFL) & os.O_ACCMODE == os.O_WRONLY:
    return fd
  except OSError: pass
 raise AssertionError('missing protocol descriptor')
'''


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
        self.assertEqual(len(summary["tasks"]), 17)
        self.assertEqual(summary["control_attempts"], {"total": 34, "passed": 17, "failed": 17})
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

    def test_deadline_controls_preserve_mock_calls_and_timeout_without_writes(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-026')
        variants = [
            ('short-deadline', '''original_run = run_commands
def run_commands():
 real_run = subprocess.run
 def wrong(*args, **kwargs):
  kwargs['timeout'] = 1800
  return real_run(*args, **kwargs)
 with __import__('unittest.mock', fromlist=['patch']).patch.object(subprocess, 'run', side_effect=wrong):
  return original_run()''', False, 'test_failure'),
            ('wrong-container', '''original_run = run_commands
def run_commands():
 real_run = subprocess.run
 def wrong(command, **kwargs): return real_run(list(command), **kwargs)
 with __import__('unittest.mock', fromlist=['patch']).patch.object(subprocess, 'run', side_effect=wrong):
  return original_run()''', False, 'test_failure'),
            ('wrong-capture', '''original_run = run_commands
def run_commands():
 real_run = subprocess.run
 def wrong(*args, **kwargs):
  kwargs['capture_output'] = False
  return real_run(*args, **kwargs)
 with __import__('unittest.mock', fromlist=['patch']).patch.object(subprocess, 'run', side_effect=wrong):
  return original_run()''', False, 'test_failure'),
            ('wrong-count', '''def run_commands(): return []''', False, 'test_failure'),
            ('swallowed-timeout', '''def main():
 try: run_commands()
 except subprocess.TimeoutExpired: return 0''', False, 'test_failure'),
            ('write-before-timeout', '''def main():
 OUTPUT.write_text('candidate-result')
 run_commands()''', False, 'test_failure'),
            ('truthy-capture', '''original_run = run_commands
def run_commands():
 real_run = subprocess.run
 def changed(*args, **kwargs):
  kwargs['capture_output'] = 1
  return real_run(*args, **kwargs)
 with __import__('unittest.mock', fromlist=['patch']).patch.object(subprocess, 'run', side_effect=changed):
  return original_run()''', True, 'none'),
            ('inert-records', '''original_run = run_commands
def run_commands():
 records = original_run()
 return [{'passed': True, 'timeout': True, 'candidate_exception': True, 'calls': []} for _ in records]
''', True, 'none'),
            ('private-exception', '''def run_commands(): raise ValueError('private-collector-payload')''',
             False, 'candidate_exception'),
            ('unsupported-return', '''original_run = run_commands
def run_commands():
 original_run()
 return {1, 2}''', False, 'adapter_policy'),
        ]
        for name, source, passed, cause in variants:
            files = self.files(task)
            files[task['source']] += ('\n' + source + '\n').encode()
            with self.subTest(variant=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
                self.assertEqual((report['passed'], report['cause']), (passed, cause), report)
                self.assertEqual(report['tests']['tests'], 2)
                self.assertEqual(report['control_attempts'],
                                 {'total': 1, 'passed': int(passed), 'failed': int(not passed)})
                self.assertIn('real-command-execution', report['coverage']['not_graded'])
                if cause == 'candidate_exception':
                    self.assertNotIn(b'private-collector-payload', (self.root / name / 'wire.log').read_bytes())
                if cause not in {'adapter_policy', 'candidate_exception'}:
                    legacy = evaluation.grade(task, files, self.root / (name + '-legacy'))
                    self.assertEqual(legacy['passed'], passed, legacy)

    def test_deadline_adapter_refuses_generated_source_and_has_no_hidden_mount(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-026')
        files = self.files(task)
        files[task['source']] += b'\n# generated collector refused\n'
        output = self.root / 'refused-deadline'
        with self.assertRaises(evaluation.EvaluationError):
            oracle.grade(task, files, output)
        self.assertFalse(output.exists())
        files = self.files(task)
        files[task['source']] += b'''
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
assert not Path('/lane').exists()
assert not Path('/dev/nvidia0').exists()
import socket
try:
 socket.create_connection(('127.0.0.1', 9), timeout=0.1)
except OSError: pass
else: raise AssertionError('network reachable')
'''
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            report = oracle.grade(task, files, self.root / 'confined-deadline')
        self.assertTrue(report['passed'], report)

    def test_resource_controls_and_wrong_observations_keep_original_assertions(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-024')
        variants = [
            ('gpu-always-valid', 'def gpu_memory(): return (1234, 23000)', False, 'test_failure'),
            ('gpu-wrong-container', '''original_gpu = gpu_memory
def gpu_memory(): return list(original_gpu())''', False, 'test_failure'),
            ('gpu-wrong-value', '''original_gpu = gpu_memory
def gpu_memory():
 used, free = original_gpu()
 return (used + 1, free)''', False, 'test_failure'),
            ('guard-wrong-return', '''original_sample = CandidateResourceGuard.sample
def sample(self):
 original_sample(self)
 return True
CandidateResourceGuard.sample = sample''', False, 'test_failure'),
            ('guard-wrong-error', '''original_sample = CandidateResourceGuard.sample
def sample(self):
 value = original_sample(self)
 self.error = 'wrong'
 return value
CandidateResourceGuard.sample = sample''', False, 'test_failure'),
            ('guard-wrong-peak', '''original_sample = CandidateResourceGuard.sample
def sample(self):
 value = original_sample(self)
 self.peak_total_gpu_used_mib_sampled = 0
 return value
CandidateResourceGuard.sample = sample''', False, 'test_failure'),
            ('guard-falsey-value', '''original_sample = CandidateResourceGuard.sample
def sample(self):
 original_sample(self)
 return []
CandidateResourceGuard.sample = sample''', True, 'none'),
            ('guard-reserved-data', '''original_sample = CandidateResourceGuard.sample
def sample(self):
 original_sample(self)
 self.error = {'harness_error': True, 'candidate_exception': True, 'passed': True}
 return False
CandidateResourceGuard.sample = sample''', False, 'test_failure'),
            ('guard-object', '''original_sample = CandidateResourceGuard.sample
def sample(self):
 original_sample(self)
 return object()
CandidateResourceGuard.sample = sample''', False, 'adapter_policy'),
            ('guard-exception', '''def sample(self): raise ValueError('synthetic-private-payload')
CandidateResourceGuard.sample = sample''', False, 'candidate_exception'),
        ]
        for name, source, passed, cause in variants:
            files = self.files(task)
            files[task['source']] += ('\n' + source + '\n').encode()
            with self.subTest(variant=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
                self.assertEqual((report['passed'], report['cause']), (passed, cause), report)
                self.assertEqual(report['tests']['tests'], 2)
                self.assertEqual(report['control_attempts'],
                                 {'total': 1, 'passed': int(passed), 'failed': int(not passed)})
                self.assertIn('real-gpu-observation', report['coverage']['not_graded'])
                if cause == 'candidate_exception':
                    self.assertNotIn(b'synthetic-private-payload', (self.root / name / 'wire.log').read_bytes())
                if cause not in {'adapter_policy', 'candidate_exception'}:
                    legacy = evaluation.grade(task, files, self.root / (name + '-legacy'))
                    self.assertEqual(legacy['passed'], passed, legacy)

    def test_resource_adapter_preserves_historical_admission_and_has_no_hidden_mount(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-024')
        files = self.files(task)
        files[task['source']] += b'\n# generated candidate refused\n'
        output = self.root / 'refused-resource'
        with self.assertRaises(evaluation.EvaluationError):
            oracle.grade(task, files, output)
        self.assertFalse(output.exists())
        files = self.files(task)
        files[task['source']] += b'''
from pathlib import Path
import socket
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
assert not Path('/dev/nvidia0').exists()
try:
 socket.create_connection(('127.0.0.1', 9), timeout=0.1)
except OSError:
 pass
else:
 raise AssertionError('network reachable')
'''
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            report = oracle.grade(task, files, self.root / 'confined-resource')
        self.assertTrue(report['passed'], report)

    def test_candidate_launch_failure_cannot_validate_either_failing_base(self):
        real_sandbox, real_grade = oracle.sandbox, oracle.grade

        def fail_candidate(workspace, adapter, output, role, **options):
            if role == "candidate":
                return ["/usr/bin/false"]
            return real_sandbox(workspace, adapter, output, role, **options)

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
            self.assertEqual(base["logs"]["wire.log"],
                             evaluation.sha(oracle.worker.encode_message({"done": True})))
            self.assertEqual(base["control_attempts"], {"total": 1, "passed": 0, "failed": 1})

    def test_diagnostic_controls_refuse_generated_inputs_before_launch(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-009')
        files = self.files(task)
        files[task['source']] += b'\n# generated snapshot\n'
        with mock.patch.object(oracle, 'exchange') as launch:
            with self.assertRaisesRegex(evaluation.EvaluationError, 'untrusted-candidate-disabled'):
                oracle.grade(task, files, self.root / 'refused')
            launch.assert_not_called()
            self.assertFalse((self.root / 'refused').exists())

    def test_protocol_is_separate_from_standard_output_and_checks_trailing_bytes(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-009')
        frame = b'{"seq":1,"result":{"stderr":"catalog drift: fixture","outcome":{"kind":"runtime-error","message":"exited 3"}}}\n'
        # Deliberate same-interpreter descriptor discovery is still possible;
        # the EOF check must reject the real reply left after a forged one.
        discover = WIRE_DESCRIPTOR_PROBE.decode()
        variants = [
            ('fd1-forgery', f'''def subprocess_runner(command, root):
 os.write(1, {frame!r})
 raise ValueError('fixed failure')
''', False, 'candidate_exception'),
            ('original-stdout-forgery', f'''def subprocess_runner(command, root):
 sys.__stdout__.buffer.write({frame!r}); sys.__stdout__.buffer.flush()
 raise ValueError('fixed failure')
''', False, 'candidate_exception'),
            ('inherited-child-output', f'''def subprocess_runner(command, root):
 subprocess.run([sys.executable, '-c', {('import os; os.write(1, ' + repr(frame) + ')')!r}], check=True)
 raise ValueError('fixed failure')
''', False, 'candidate_exception'),
            ('private-fd-forgery', f'''def subprocess_runner(command, root):
 os.write(protocol_fd(), {frame!r})
 raise ValueError('fixed failure')
''', False, 'protocol'),
            ('exit-tail', '''import atexit
saved_writer = os.dup(protocol_fd())
atexit.register(lambda: os.write(saved_writer, b'unsolicited-tail'))
''', False, 'protocol'),
            ('ordinary-output', '''original_runner = subprocess_runner
def subprocess_runner(command, root):
 print('ordinary-print')
 os.write(1, b'ordinary-fd1-output\\n')
 sys.__stdout__.write('original-stdout-output\\n'); sys.__stdout__.flush()
 return original_runner(command, root)
''', True, 'none'),
            ('startup-output', '''os.write(1, b'{"ready":false,"function_found":true}\\n')
''', True, 'none'),
            ('private-fd-not-inherited', '''original_runner = subprocess_runner
def subprocess_runner(command, root):
 fd = protocol_fd()
 assert not os.get_inheritable(fd)
 child = 'import os\\ntry: os.fstat(' + str(fd) + ')\\nexcept OSError: pass\\nelse: raise AssertionError("protocol inherited")'
 subprocess.run([sys.executable, '-I', '-c', child], close_fds=False, check=True)
 return original_runner(command, root)
''', True, 'none'),
        ]
        for name, suffix, passed, cause in variants:
            files = self.files(task)
            files[task['source']] += ('\n' + discover + suffix).encode()
            with self.subTest(variant=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                output = self.root / name
                report = oracle.grade(task, files, output)
                self.assertEqual((report['passed'], report['cause']), (passed, cause), report)
                self.assertEqual(report['tests']['tests'], 1)
                self.assertEqual(report['control_attempts'],
                                 {'total': 1, 'passed': int(passed), 'failed': int(not passed)})
                if name in {'private-fd-forgery', 'exit-tail'}:
                    self.assertTrue(report['protocol_failed'])
                    self.assertEqual(report['tests']['failures'], 0)
                if name == 'ordinary-output':
                    for marker in (b'ordinary-print', b'ordinary-fd1-output', b'original-stdout-output'):
                        self.assertIn(marker, (output / 'candidate.log').read_bytes())
                        self.assertNotIn(marker, (output / 'wire.log').read_bytes())

    def test_launcher_arguments_and_descriptor_are_absent_inside_real_namespace(self):
        report = self.probe(b'''
import os
from pathlib import Path
cmd = Path('/proc/1/cmdline').read_bytes().split(b'\\0')
assert cmd[0] == b'/usr/bin/bwrap', cmd
assert cmd[1] == b'--args', cmd
assert cmd[3:] == [b'--', b'/usr/bin/python3', b'-I', b'-B', b'/adapter/run.py', b'candidate', b''], cmd
for path in Path('/proc').glob('[0-9]*/cmdline'):
 data = path.read_bytes()
 assert b'agentmage-eval-' not in data and b'candidate-adapter' not in data, data
assert not Path('/proc/self/fd/' + cmd[2].decode()).exists()
assert not Path('/proc/1/fd/' + cmd[2].decode()).exists()
''', 'private-launch-arguments')
        self.assertTrue(report['passed'], report)

    def test_launcher_and_workers_do_not_inherit_host_environment(self):
        # Synthetic-only parent environment keeps a failing regression from
        # printing real host data. Test both namespace roles, including PID 1;
        # --clearenv alone only clears the executed worker's environment.
        source = '''import os
from pathlib import Path
assert Path('/proc/1/environ').read_bytes() == b'', 'launcher environment not empty'
assert dict(os.environ) == {
 'PATH': '/usr/bin', 'HOME': '/tmp', 'LANG': 'C.UTF-8', 'PWD': '/workspace'
}, 'unexpected worker environment'
for entry in Path('/proc').glob('[0-9]*/environ'):
 try:
  data = entry.read_bytes()
 except FileNotFoundError:
  continue
 assert b'AGENTMAGE_TEST_SECRET' not in data, 'host sentinel inherited'
 assert b'/synthetic-host-home' not in data, 'host home inherited'
 assert b'synthetic-host-user' not in data, 'host user inherited'
print('environment-isolated')
'''
        host_environment = {'AGENTMAGE_TEST_SECRET': 'synthetic-credential',
                            'HOME': '/synthetic-host-home', 'USER': 'synthetic-host-user'}
        for role in ('candidate', 'oracle'):
            with self.subTest(role=role):
                command = oracle.sandbox(self.root, self.root, self.root, role)
                command[command.index('--') + 1:] = ['/usr/bin/python3', '-I', '-B', '-c', source]
                with mock.patch.dict(os.environ, host_environment, clear=True):
                    process = oracle.launch(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                            start_new_session=True, preexec_fn=evaluation.limits)
                    try:
                        stdout, stderr = process.communicate(timeout=10)
                    finally:
                        # communicate reaps on success. Never signal a group
                        # after its leader's PID can have been reused.
                        if process.returncode is None:
                            oracle.terminate(process)
                self.assertEqual(process.returncode, 0, stderr.decode(errors='replace'))
                self.assertEqual(stdout, b'environment-isolated\n')
                self.assertEqual(stderr, b'')

    def test_candidate_that_holds_eof_open_hits_host_deadline_and_counts_failure(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-009')
        files = self.files(task)
        files[task['source']] += b'''
import atexit, fcntl, os, stat, time
for fd in range(3, 64):
 try:
  if stat.S_ISFIFO(os.fstat(fd).st_mode) and fcntl.fcntl(fd, fcntl.F_GETFL) & os.O_ACCMODE == os.O_WRONLY:
   saved_writer = os.dup(fd)
   break
 except OSError: pass
atexit.register(lambda: time.sleep(20))
'''
        real_case = oracle.grade_case

        def bounded_case(task, files, output):
            return real_case(task, files, output, timeout=2)

        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'), \
                mock.patch.object(oracle, 'grade_case', side_effect=bounded_case):
            report = oracle.grade(task, files, self.root / 'held-eof')
        self.assertFalse(report['passed'])
        self.assertEqual(report['cause'], 'timeout')
        self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
        self.assertFalse(list(self.root.rglob('cleanup-recovery.json')))

    def test_diagnostic_assertions_and_exception_data_stay_independent(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-009')
        variants = [
            ('no-stderr', '''def subprocess_runner(command, root):
 raise RuntimeError('exited 3')
''', False, 'test_failure'),
            ('wrong-message', '''def subprocess_runner(command, root):
 print('catalog drift: fixture', file=sys.stderr)
 raise RuntimeError('wrong error')
''', False, 'test_failure'),
            ('no-error', '''def subprocess_runner(command, root):
 print('catalog drift: fixture', file=sys.stderr)
 return 'exited 3'
''', False, 'test_failure'),
            ('unexpected-error', '''def subprocess_runner(command, root):
 raise ValueError('unexpected-private-payload')
''', False, 'candidate_exception'),
            ('forgery', '''import unittest
unittest.TestCase.assertIn = lambda *args: None
def subprocess_runner(command, root): raise RuntimeError('wrong error')
''', False, 'test_failure'),
            ('real-child', '''original_runner = subprocess_runner
def subprocess_runner(command, root): return original_runner(command, root)
''', True, 'none'),
            ('separate-mounts', '''original_runner = subprocess_runner
def subprocess_runner(command, root):
 assert not Path('/adapter/test.py').exists()
 assert not Path('/output').exists()
 return original_runner(command, root)
''', True, 'none'),
            ('exception-subclass', '''class DiagnosticError(RuntimeError): pass
def subprocess_runner(command, root):
 print('catalog drift: fixture', file=sys.stderr)
 raise DiagnosticError('exited 3')
''', True, 'none'),
            ('tail-ungraded', '''def subprocess_runner(command, root):
 print('x' * 17000 + 'catalog drift: fixture', file=sys.stderr)
 raise RuntimeError('exited 3')
''', True, 'none'),
        ]
        for name, suffix, passed, cause in variants:
            files = self.files(task)
            files[task['source']] += ('\n' + suffix).encode()
            output = self.root / name
            with self.subTest(variant=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, output)
                legacy = evaluation.grade(task, files, self.root / (name + '-legacy'))
                self.assertEqual((report['passed'], report['cause']), (passed, cause), report)
                legacy_passed = True if name == 'forgery' else False if name == 'separate-mounts' else passed
                self.assertEqual(legacy['passed'], legacy_passed, legacy)
                self.assertEqual(report['tests']['tests'], 1)
                self.assertEqual(report['calls'], 1)
                self.assertEqual(report['control_attempts'],
                                 {'total': 1, 'passed': int(passed), 'failed': int(not passed)})
                for log in ('wire.log', 'oracle.log'):
                    self.assertNotIn(b'unexpected-private-payload', (output / log).read_bytes())
                self.assertIn('output-tail-bound-and-newline', report['coverage']['not_graded'])
                self.assertIn('exact-exception-type', report['coverage']['not_graded'])

    def test_capture_controls_refuse_generated_inputs_before_launch(self):
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-005", "dev-018"}):
            files = self.files(task)
            files[task["source"]] += b"\n# synthetic candidate\n"
            output = self.root / task["id"]
            with self.subTest(task=task["id"]), mock.patch.object(oracle, "exchange") as launch:
                with self.assertRaisesRegex(evaluation.EvaluationError, "untrusted-candidate-disabled"):
                    oracle.grade(task, files, output)
                launch.assert_not_called()
                self.assertFalse(output.exists())

    def test_mode_and_command_controls_keep_admission_closed(self):
        for task in (t for t in evaluation.collection() if t['id'] in {'dev-017', 'dev-019'}):
            files = self.files(task)
            files[task['source']] += b'\n# generated source stays refused\n'
            with self.subTest(task=task['id']), mock.patch.object(oracle, 'exchange') as launch:
                with self.assertRaisesRegex(evaluation.EvaluationError, 'untrusted-candidate-disabled'):
                    oracle.grade(task, files, self.root / task['id'])
                launch.assert_not_called()
                self.assertFalse((self.root / task['id']).exists())

    def test_mode_and_command_observations_match_frozen_assertions(self):
        variants = {
            'dev-017': [
                ('noop', 'def make_container_readable(root): pass', False, 'test_failure'),
                ('all-executable', '''def make_container_readable(root):
 for path in (root, *root.rglob('*')): path.chmod(0o755)
''', False, 'test_failure'),
                ('deleted', '''def make_container_readable(root):
 (root / 'nested/source.rs').unlink()
''', False, 'adapter_policy'),
                ('forgery', '''import unittest
unittest.TestCase.assertEqual = lambda *args: None
def make_container_readable(root): pass
''', False, 'test_failure'),
                ('content-ungraded', '''original_mode = make_container_readable
def make_container_readable(root):
 (root / 'nested/source.rs').write_text('different bytes')
 return original_mode(root)
''', True, 'none'),
            ],
            'dev-019': [
                ('noop', 'def extract_deb(package, root): pass', False, 'test_failure'),
                ('tuple', '''def extract_deb(package, root):
 command(('ar', 'x', str(package)), cwd=root)
 command(['tar', '--extract', '--gzip', '--file', 'data.tar.gz', '--no-same-owner', '--same-permissions'], cwd=root)
''', False, 'test_failure'),
                ('string-path', '''original_extract = extract_deb
def extract_deb(package, root): return original_extract(package, str(root))
''', False, 'test_failure'),
                ('extra-call', '''original_extract = extract_deb
def extract_deb(package, root):
 original_extract(package, root)
 command(['extra'], cwd=root)
''', False, 'test_failure'),
                ('forgery', '''import unittest
unittest.TestCase.assertEqual = lambda *args: None
def extract_deb(package, root): command(['wrong'], cwd=root)
''', False, 'test_failure'),
                ('object', 'def extract_deb(package, root): command(object())', False, 'adapter_policy'),
                ('exception', 'def extract_deb(package, root): raise ValueError("private-payload")',
                 False, 'candidate_exception'),
            ],
        }
        for task in (t for t in evaluation.collection() if t['id'] in variants):
            for name, suffix, passed, cause in variants[task['id']]:
                files = self.files(task)
                files[task['source']] += ('\n' + suffix + '\n').encode()
                output = self.root / (task['id'] + '-' + name)
                with self.subTest(task=task['id'], variant=name), mock.patch.object(
                        evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                    report = oracle.grade(task, files, output)
                    legacy = evaluation.grade(task, files, self.root / (task['id'] + '-' + name + '-legacy'))
                    self.assertEqual((report['passed'], report['cause']), (passed, cause), report)
                    # The legacy grader shares its assertions with candidate
                    # imports: these deliberate unittest patches can forge it.
                    self.assertEqual(legacy['passed'], True if name == 'forgery' else passed, legacy)
                    self.assertEqual(report['tests']['tests'], task['expected_tests'])
                    self.assertEqual(report['control_attempts'],
                                     {'total': 1, 'passed': int(passed), 'failed': int(not passed)})
                    for log in ('wire.log', 'oracle.log'):
                        self.assertNotIn(b'private-payload', (output / log).read_bytes())

    def test_mode_claims_cannot_replace_actual_confined_filesystem_observations(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-017')
        variants = [
            ('false-ack', '''import __main__
__main__.mode_result = lambda *args: {'observed': True}
''', 'adapter_policy'),
            ('forged-modes', '''import __main__
__main__.mode_result = lambda *args: {'modes': {'.': 493}}
''', 'protocol'),
            ('symlink', '''def make_container_readable(root):
 (root / 'nested/source.rs').unlink()
 (root / 'nested/source.rs').symlink_to('/workspace/scripts/path_platform_conformance.py')
''', 'adapter_policy'),
            ('forged-stat', '''from types import SimpleNamespace
def make_container_readable(root):
 Path.stat = lambda *a, **kw: SimpleNamespace(st_mode=0o755)
''', 'test_failure'),
        ]
        for name, suffix, cause in variants:
            files = self.files(task)
            files[task['source']] += ('\n' + suffix + '\n').encode()
            with self.subTest(variant=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
                self.assertEqual(report['cause'], cause, report)
                self.assertFalse(report['passed'])
                self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def mode_cleanup_probe(self, suffix, name):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-017')
        files = self.files(task)
        files[task['source']] += ('\n' + suffix + '\n').encode()
        output = self.root / name
        scratch = self.root / (name + '-scratch')
        scratch.mkdir()
        make_temporary = evaluation.tempfile.mkdtemp

        def make_case(*args, **kwargs):
            return make_temporary(*args, **kwargs, dir=scratch)

        with mock.patch.object(evaluation.tempfile, 'mkdtemp', side_effect=make_case), mock.patch.object(
                evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            report = oracle.grade(task, files, output)
        self.assertEqual(list(scratch.iterdir()), [], 'candidate workspace residue')
        self.assertFalse((output / 'cleanup.json').exists())
        self.assertIsNone(report['error'], report)
        self.assertIsNotNone(report['tests'], report)
        self.assertEqual(report['calls'], 1, report)
        passed = report['passed']
        self.assertEqual(report['control_attempts'],
                         {'total': 1, 'passed': int(passed), 'failed': int(not passed)})
        self.assertEqual(json.loads((output / 'tests.json').read_text())['tests'], report['tests'])
        return report

    def test_mode_cleanup_repairs_extra_locked_directories_without_changing_verdict(self):
        for parent in ('root', "Path('/tmp')"):
            for mode in (0o500, 0o000):
                suffix = f'''original_mode = make_container_readable
def make_container_readable(root):
 original_mode(root)
 extra = {parent} / 'extra'
 (extra / 'child').mkdir(parents=True)
 (extra / 'child/payload').write_text('candidate-only payload')
 (extra / 'child').chmod({mode})
 extra.chmod({mode})
'''
                name = 'locked-' + ('root' if parent == 'root' else 'tmp') + '-' + str(mode)
                with self.subTest(parent=parent, mode=mode):
                    report = self.mode_cleanup_probe(suffix, name)
                    self.assertTrue(report['passed'], report)
                    self.assertEqual(report['cause'], 'none', report)

    def test_mode_cleanup_preserves_wrong_answer_and_policy_failures(self):
        variants = [
            ('all-555', '''def make_container_readable(root):
 for path in reversed([root, *root.rglob('*')]):
  path.chmod(0o555 if path.is_dir() else 0o644)
''', 'test_failure'),
            ('all-644', '''def make_container_readable(root):
 for path in reversed([root, *root.rglob('*')]): path.chmod(0o644)
''', 'adapter_policy'),
            ('nested-000', "def make_container_readable(root): (root / 'nested').chmod(0)", 'adapter_policy'),
            ('root-000', 'def make_container_readable(root): root.chmod(0)', 'adapter_policy'),
            ('tmp-000', "def make_container_readable(root): Path('/tmp').chmod(0)", 'adapter_policy'),
            ('exception', '''def make_container_readable(root):
 extra = Path('/tmp/extra')
 extra.mkdir()
 extra.chmod(0)
 raise ValueError('candidate-only payload')
''', 'candidate_exception'),
        ]
        for name, suffix, cause in variants:
            with self.subTest(variant=name):
                report = self.mode_cleanup_probe(suffix, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['cause'], cause, report)

    def test_fixture_permission_repair_never_chmods_symlink_targets_or_files(self):
        fixture, outside = self.root / 'fixture', self.root / 'outside'
        fixture.mkdir()
        outside.mkdir(mode=0o700)
        child = fixture / 'child'
        child.mkdir()
        data = outside / 'data'
        data.write_text('unchanged')
        data.chmod(0o400)
        outside.chmod(0o500)
        (child / 'file-link').hardlink_to(data)
        (child / 'dir-alias').symlink_to(outside, target_is_directory=True)
        (child / 'file-alias').symlink_to(data)
        (child / 'dangling').symlink_to(self.root / 'absent')
        (child / 'cycle').symlink_to(fixture, target_is_directory=True)
        child.chmod(0)
        fixture.chmod(0)
        oracle.repair_fixture_permissions(fixture)
        self.assertEqual(fixture.stat().st_mode & 0o777, 0o700)
        self.assertEqual(child.stat().st_mode & 0o777, 0o700)
        self.assertEqual(outside.stat().st_mode & 0o777, 0o500)
        self.assertEqual(data.stat().st_mode & 0o777, 0o400)
        self.assertEqual(data.read_text(), 'unchanged')
        oracle.remove_fixture_tree(fixture)
        self.assertFalse(fixture.exists())
        self.assertEqual(outside.stat().st_mode & 0o777, 0o500)
        self.assertEqual(data.stat().st_mode & 0o777, 0o400)
        # Do not let this test's intentionally locked sentinel obstruct its own cleanup.
        outside.chmod(0o700)

    def test_deep_candidate_trees_preserve_pass_and_failure_without_residue(self):
        for wrong in (False, True):
            suffix = f'''import os as _os
original_mode = make_container_readable
def make_container_readable(root):
 original_mode(root)
 if {wrong!r}: (root / 'nested').chmod(0o555)
 _os.mkdir('/tmp/deep')
 _os.chdir('/tmp/deep')
 for _ in range(2500):
  _os.mkdir('a')
  child = _os.open('a', _os.O_RDONLY | _os.O_DIRECTORY)
  _os.chmod('.', 0)
  _os.fchdir(child)
  _os.close(child)
 _os.chmod('.', 0)
 _os.chdir('/workspace')
'''
            with self.subTest(wrong=wrong):
                report = self.mode_cleanup_probe(suffix, 'deep-' + str(wrong))
                self.assertEqual(report['passed'], not wrong, report)
                self.assertEqual(report['cause'], 'test_failure' if wrong else 'none', report)

    def test_deep_branched_repair_uses_bounded_descriptors(self):
        # A subprocess avoids changing the test runner's descriptor limit.
        source = '''import os, resource, sys
from pathlib import Path
from scripts.coding_eval_oracle import repair_fixture_permissions, remove_fixture_tree
root = Path(sys.argv[1])
root.mkdir()
resource.setrlimit(resource.RLIMIT_NOFILE, (32, 32))
fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY)
try:
 for _ in range(2500):
  os.mkdir('a', dir_fd=fd)
  os.mkdir('b', dir_fd=fd)
  child = os.open('a', os.O_RDONLY | os.O_DIRECTORY, dir_fd=fd)
  os.fchmod(fd, 0)
  os.close(fd)
  fd = child
 os.fchmod(fd, 0)
finally: os.close(fd)
repair_fixture_permissions(root)
fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY)
try:
 for _ in range(2500):
  assert os.stat('b', dir_fd=fd).st_mode & 0o777 == 0o700
  child = os.open('a', os.O_RDONLY | os.O_DIRECTORY, dir_fd=fd)
  os.close(fd)
  fd = child
 assert os.fstat(fd).st_mode & 0o777 == 0o700
finally: os.close(fd)
remove_fixture_tree(root)
assert not root.exists()
'''
        result = subprocess.run([sys.executable, '-B', '-c', source, str(self.root / 'deep-branched')],
                                capture_output=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors='replace'))

    def test_permission_repair_pins_inode_across_ancestor_symlink_replacement(self):
        fixture, outside = self.root / 'fixture', self.root / 'sentinel'
        (fixture / 'parent/child').mkdir(parents=True)
        (outside / 'child').mkdir(parents=True, mode=0o500)
        outside.chmod(0o500)
        chmod = os.chmod
        replaced = False

        def replace_then_chmod(path, mode, **kwargs):
            nonlocal replaced
            if not replaced and os.readlink(path).endswith('/parent/child'):
                (fixture / 'parent').rename(fixture / 'retired')
                (fixture / 'parent').symlink_to(outside, target_is_directory=True)
                replaced = True
            return chmod(path, mode, **kwargs)

        try:
            with mock.patch.object(oracle.os, 'chmod', side_effect=replace_then_chmod):
                oracle.repair_fixture_permissions(fixture)
            self.assertTrue(replaced)
            self.assertEqual(outside.stat().st_mode & 0o777, 0o500)
            self.assertEqual((outside / 'child').stat().st_mode & 0o777, 0o500)
            self.assertEqual((fixture / 'retired/child').stat().st_mode & 0o777, 0o700)
        finally:
            chmod(outside, 0o700)
            chmod(outside / 'child', 0o700)

    def test_permission_repair_resists_live_ancestor_symlink_swapping(self):
        fixture, outside = self.root / 'fixture', self.root / 'sentinel'
        (fixture / 'parent/child').mkdir(parents=True)
        (outside / 'child').mkdir(parents=True)
        outside.chmod(0o500)
        (outside / 'child').chmod(0o500)
        source = '''import os, sys
root, sentinel = sys.argv[1:]
os.chdir(root)
os.rename('parent', 'retired')
os.symlink(sentinel, 'parent')
os.unlink('parent')
os.rename('retired', 'parent')
print('swapped', flush=True)
while True:
 os.rename('parent', 'retired')
 os.symlink(sentinel, 'parent')
 os.unlink('parent')
 os.rename('retired', 'parent')
'''
        process = subprocess.Popen([sys.executable, '-B', '-c', source, str(fixture), str(outside)],
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            self.assertEqual(process.stdout.readline(), b'swapped\n')
            for _ in range(100):
                oracle.repair_fixture_permissions(fixture)
            self.assertIsNone(process.poll(), 'swapper must be live during repair')
            self.assertEqual(outside.stat().st_mode & 0o777, 0o500)
            self.assertEqual((outside / 'child').stat().st_mode & 0o777, 0o500)
        finally:
            process.kill()
            process.communicate(timeout=5)
            outside.chmod(0o700)
            (outside / 'child').chmod(0o700)

    def test_fixture_removal_refuses_changed_parent_identity(self):
        fixture, outside = self.root / 'fixture', self.root / 'sentinel'
        (fixture / 'parent/child').mkdir(parents=True)
        (outside / 'child').mkdir(parents=True)
        (outside / 'child/payload').write_text('sentinel')
        real_open = os.open
        moved = False

        def move_parent(path, flags, **kwargs):
            nonlocal moved
            if path == '..' and not moved:
                (fixture / 'parent').rename(self.root / 'retired')
                (fixture / 'parent').symlink_to(outside, target_is_directory=True)
                moved = True
            return real_open(path, flags, **kwargs)

        with mock.patch.object(oracle.os, 'open', side_effect=move_parent):
            with self.assertRaisesRegex(OSError, 'fixture-moved-during-cleanup'):
                oracle.remove_fixture_tree(fixture)
        self.assertTrue(moved)
        self.assertEqual((outside / 'child/payload').read_text(), 'sentinel')

    def test_mode_fixture_repairs_after_exchange_interruption(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-017')
        files = self.files(task)
        files[task['source']] += b'''\noriginal_mode = make_container_readable
def make_container_readable(root):
 original_mode(root)
 extra = Path('/tmp/extra')
 extra.mkdir()
 extra.chmod(0)
'''
        scratch = self.root / 'interrupted-scratch'
        scratch.mkdir()
        make_temporary, real_exchange = evaluation.tempfile.mkdtemp, oracle.exchange

        def stopped_then_interrupt(*args, **kwargs):
            real_exchange(*args, **kwargs)
            raise KeyboardInterrupt()

        output = self.root / 'interrupted-fixture'
        with mock.patch.object(evaluation.tempfile, 'mkdtemp', side_effect=lambda **kw:
                               make_temporary(**kw, dir=scratch)), mock.patch.object(
                evaluation, 'require_historical_candidate', return_value='synthetic-probe'), mock.patch.object(
                oracle, 'exchange', side_effect=stopped_then_interrupt):
            with self.assertRaises(KeyboardInterrupt):
                oracle.grade(task, files, output)
        self.assertEqual(list(scratch.iterdir()), [])
        self.assertFalse((output / 'cleanup.json').exists())
        self.assertFalse((output / 'tests.json').exists())

    def test_fixture_repair_error_preserves_original_interruption(self):
        root = self.root / 'case'
        root.mkdir()
        with mock.patch.object(oracle, 'repair_fixture_permissions', side_effect=OSError('repair refused')):
            with self.assertRaises(KeyboardInterrupt):
                with oracle.candidate_temporary(root, True):
                    raise KeyboardInterrupt()
            (root / 'candidate-tmp').rmdir()
            with self.assertRaisesRegex(OSError, 'repair refused'):
                with oracle.candidate_temporary(root, True):
                    pass

    def test_new_observation_workers_keep_files_network_and_assertions_confined(self):
        suffix = '''
from pathlib import Path
import socket
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
try:
 Path('/workspace/escape').write_text('escape')
except OSError: pass
else: raise AssertionError('workspace writable')
try:
 socket.create_connection(('127.0.0.1', 9), timeout=0.1)
except OSError: pass
else: raise AssertionError('network accessible')
'''
        for task in (t for t in evaluation.collection() if t['id'] in {'dev-017', 'dev-019'}):
            files = self.files(task)
            files[task['source']] += suffix.encode()
            with self.subTest(task=task['id']), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / task['id'])
                self.assertTrue(report['passed'], report)

    def test_capture_assertions_stay_trusted_and_wrong_values_fail(self):
        variants = (
            ("redaction", b"\ndef capture(): return ('unredacted output', 0)\n", "test_failure"),
            ("validation", b"\ndef validate_raw(text): return []\n", "test_failure"),
            ("forgery", b'''
import unittest
from pathlib import Path
assert not Path('/adapter/test.py').exists()
assert not Path('/output').exists()
unittest.TestCase.assertIn = lambda *a, **kw: None
unittest.TestCase.assertNotIn = lambda *a, **kw: None
def capture(): return ('unredacted output', 0)
def validate_raw(text): return []
''', "test_failure"),
            ("object", b"\ndef validate_raw(text): return object()\n", "adapter_policy"),
            ("exception", b"\ndef validate_raw(text): raise ValueError('private-candidate-payload')\n",
             "candidate_exception"),
        )
        for task in (t for t in evaluation.collection() if t["id"] in {"dev-005", "dev-018"}):
            for name, suffix, cause in variants:
                files = self.files(task)
                files[task["source"]] += suffix
                output = self.root / (task["id"] + "-" + name)
                with self.subTest(task=task["id"], variant=name), mock.patch.object(
                        evaluation, "require_historical_candidate", return_value="synthetic-probe"):
                    report = oracle.grade(task, files, output)
                self.assertFalse(report["passed"], report)
                self.assertEqual(report["cause"], cause, report)
                self.assertEqual(report["tests"]["tests"], 1)
                self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})
                self.assertIn("real-command-execution", report["coverage"]["not_graded"])
                self.assertNotIn(b"private-candidate-payload", (output / "wire.log").read_bytes())
                self.assertNotIn(b"private-candidate-payload", (output / "oracle.log").read_bytes())

    def test_response_memory_exhaustion_is_counted_adapter_policy(self):
        report = self.probe(b"\ndef case_passed(v): return 'x' * (150 * 1024 * 1024) if v else ''\n",
                            "memory-exhaustion")
        self.assertFalse(report["passed"])
        self.assertEqual(report["cause"], "adapter_policy", report)
        self.assertEqual(report["candidate_exit_code"], 0)
        self.assertEqual(report["control_attempts"], {"total": 1, "passed": 0, "failed": 1})

    def test_capture_stub_semantics_agree_with_complete_frozen_methods(self):
        variants = [
            ('stderr-merge', '''
original_capture = capture
def capture():
 result = subprocess.run([])
 text = result.stdout + (result.stderr or "")
 if not isinstance(text, str): return ("unredacted output", 0)
 return original_capture()
''', {'dev-005': (True, 'none'), 'dev-018': (False, 'test_failure')}),
            ('stderr-method', '''
original_capture = capture
def capture():
 result = subprocess.run([])
 str(result.stderr.replace("x", "y"))
 return original_capture()
''', {'dev-005': (False, 'candidate_exception'), 'dev-018': (True, 'none')}),
            ('completed-type', '''
original_capture = capture
def capture():
 if not isinstance(subprocess.run([]), subprocess.CompletedProcess):
  return ("unredacted output", 0)
 return original_capture()
''', {'dev-005': (True, 'none'), 'dev-018': (False, 'test_failure')}),
        ]
        for task in (t for t in evaluation.collection() if t['id'] in {'dev-005', 'dev-018'}):
            for name, suffix, outcomes in variants:
                expected = outcomes[task['id']]
                files = self.files(task)
                files[task['source']] += suffix.encode()
                output = self.root / (task['id'] + '-' + name)
                with self.subTest(task=task['id'], variant=name), mock.patch.object(
                        evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                    report = oracle.grade(task, files, output)
                    legacy = evaluation.grade(task, files, self.root / (task['id'] + '-' + name + '-legacy'))
                self.assertEqual((report['passed'], report['cause']), expected, report)
                self.assertEqual(legacy['passed'], expected[0], legacy)
                self.assertEqual(report['control_attempts'], {'total': 1, 'passed': int(expected[0]),
                                                              'failed': int(not expected[0])})
                self.assertEqual(report['tests']['tests'], task['expected_tests'])
                if task['id'] == 'dev-018':
                    self.assertIn('home-alias-variant', report['coverage']['not_graded'])
                else:
                    self.assertNotIn('home-alias-variant', report['coverage']['not_graded'])

    def test_import_failure_and_false_startup_do_not_run_hidden_assertions(self):
        for name, code in (
                ("import-failure", b'\nraise RuntimeError("synthetic import failure")\n'),
                ("false-startup", b'\nimport __main__\noriginal_encode = __main__.encode_message\n'
                 b'__main__.encode_message = lambda value: original_encode(\n'
                 b' {"ready": False, "function_found": True} if "ready" in value else value)\n')):
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

    def test_argument_containers_are_graded_and_integer_truthiness_reaches_assertions(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-006")
        files = self.files(task)
        files[task["source"]] += b'''
original_arguments = route_grant_arguments
def route_grant_arguments(arguments):
 return tuple(original_arguments(arguments))
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "container-observation")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["cause"], "test_failure")
        self.assertIn("argument-return-container-type", report["coverage"]["graded"])
        task = next(t for t in evaluation.collection() if t["id"] == "dev-022")
        files = self.files(task)
        files[task["source"]] += b'''
original_verify_case = verify_case
def verify_case(*args):
 result = original_verify_case(*args)
 return {'passed': int(result['passed']), 'checks': {key: int(value) for key, value in result['checks'].items()}}
'''
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "integer-observation")
        self.assertTrue(report["passed"], report)
        self.assertFalse(report["adapter_policy_failed"])
        self.assertFalse(report["protocol_failed"])
        self.assertIn("exact-boolean-result-type", report["coverage"]["not_graded"])
        self.assertIn("passed-truthiness", report["coverage"]["graded"])

    def test_unsupported_parser_namespace_is_a_policy_failure_without_reconstruction(self):
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
        self.assertEqual(report["cause"], "adapter_policy")
        self.assertEqual(report["tests"]["errors"], 1)

    def test_verification_preamble_grades_candidate_constants_with_original_assertion(self):
        task = next(t for t in evaluation.collection() if t["id"] == "dev-022")
        files = self.files(task)
        files[task["source"]] += b"\nCASE_ROOTS = {name: 'duplicate' for name in CASE_ROOTS}\n"
        with mock.patch.object(evaluation, "require_historical_candidate", return_value="synthetic-probe"):
            report = oracle.grade(task, files, self.root / "candidate-case-roots")
        self.assertFalse(report["passed"], report)
        self.assertEqual(report["cause"], "test_failure")
        self.assertEqual(report["calls"], 1)
        self.assertEqual(report["tests"]["failures"], 1)
        self.assertIn("candidate-case-root-uniqueness", report["coverage"]["graded"])

    def test_parser_policies_are_separate_from_malformed_wire_and_still_count_failures(self):
        for name, ident, source, cause in (
                ('callable-default', 'dev-007', b'''
original_parser = parser
def parser():
 result = original_parser()
 result.set_defaults(handler=print)
 return result
''', 'adapter_policy'),
                ('nonstandard-exit', 'dev-027', b'''
original_parser = parser
class ParserProxy:
 def parse_args(self, argv):
  try: return original_parser().parse_args(argv)
  except SystemExit: raise SystemExit(1)
def parser(): return ParserProxy()
''', 'adapter_policy'),
                ('forged-policy-payload', 'dev-006', b'''
import __main__
__main__.argument_result = lambda *a: {'adapter_policy': True, 'message': 'untrusted-payload'}
''', 'protocol'),
                ('forged-container', 'dev-006', b'''
import __main__
__main__.argument_result = lambda *a: {'arguments': [], 'container': '__reduce__'}
''', 'protocol')):
            task = next(t for t in evaluation.collection() if t['id'] == ident)
            files = self.files(task)
            files[task['source']] += source
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
            self.assertFalse(report['passed'], report)
            self.assertEqual(report['cause'], cause)
            self.assertEqual(report['adapter_policy_failed'], cause == 'adapter_policy')
            self.assertEqual(report['protocol_failed'], cause == 'protocol')
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
            self.assertNotIn('untrusted-payload', (self.root / name / 'oracle.log').read_text())

    def test_unsupported_case_roots_stop_before_hidden_assertions_and_count_the_attempt(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-022')
        for name, source, cause in (
                ('unsupported-roots', b"\nCASE_ROOTS = {'case': print}\n", 'adapter_policy'),
                ('malformed-roots', WIRE_DESCRIPTOR_PROBE + b'''
import os
os.write(protocol_fd(), b'{"ready":true,"function_found":true}\\n{"seq":1,"result":{"case_roots":{"a":1}}}\\n')
os._exit(0)
''', 'protocol')):
            files = self.files(task)
            files[task['source']] += source
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
            self.assertEqual(report['cause'], cause, report)
            self.assertTrue(report['candidate_ready'])
            self.assertEqual(report['calls'], 1)
            self.assertEqual(report['tests']['tests'], 0)
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def test_declared_coverage_matches_allowed_policy_types(self):
        for task in (t for t in evaluation.collection() if t['id'] in oracle.SUPPORTED):
            kind = oracle.SUPPORTED[task['id']]['result_type']
            self.assertEqual(bool(oracle.coverage(task).get('adapter_policy')),
                             kind in oracle.worker.POLICY_RESULT_TYPES)

    def predicate_contrast(self, ident, transform, name, expected):
        task = next(t for t in evaluation.collection() if t['id'] == ident)
        function = oracle.SUPPORTED[ident]['function']
        files = self.files(task)
        result = (transform if ident == 'dev-015' else
                  '{key: ' + transform + ' for key, value in original(*args).items()}')
        files[task['source']] += ('\noriginal = ' + function + '\ndef ' + function + '(*args):\n'
                                  + (' value = original(*args)\n' if ident == 'dev-015' else '')
                                  + ' return ' + result + '\n').encode()
        # Independent and legacy diagnostics use the same candidate projection.
        # The production admission guard and every hidden assertion stay unchanged.
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            report = oracle.grade(task, files, self.root / name)
            legacy = evaluation.grade(task, files, self.root / (name + '-legacy'))
        self.assertEqual(report['passed'], expected, report)
        self.assertEqual(legacy['passed'], expected, legacy)
        self.assertEqual(report['cause'], 'none' if expected else 'test_failure', report)
        self.assertEqual(report['tests']['tests'], task['expected_tests'])
        self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed']
                         or report['candidate_exception_failed'])
        self.assertEqual(report['control_attempts'], {'total': 1, 'passed': int(expected),
                                                    'failed': int(not expected)})
        self.assertIn('exact-boolean-result-type', report['coverage']['not_graded'])
        self.assertIn('selected-predicate-truthiness', report['coverage']['graded'])

    def test_boolean_predicate_truthiness_matches_frozen_assertions_and_legacy(self):
        for ident in ('dev-015', 'dev-029'):
            for name, transform in (('integer', 'int(value)'), ('null', 'value or None')):
                with self.subTest(task=ident, transform=name):
                    self.predicate_contrast(ident, transform, ident + '-' + name, True)
        for name, transform in (('text', "'yes' if value else ''"), ('float', 'float(value)')):
            with self.subTest(transform=name):
                self.predicate_contrast('dev-015', transform, name, True)
        self.predicate_contrast('dev-029', '[1] if value else []', 'map-containers', True)
        for name, transform in (('list', '[1] if value else []'),
                                ('dict', "{'ok': 1} if value else {}")):
            self.predicate_contrast('dev-015', transform, 'predicate-' + name, True)

    def test_wrong_boolean_predicate_values_fail_original_assertions_and_legacy(self):
        for ident in ('dev-015', 'dev-029'):
            with self.subTest(task=ident):
                self.predicate_contrast(ident, 'not value', ident + '-inverted', False)

    def test_case_root_exception_records_no_assertions_but_one_failed_attempt(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-022')
        files = self.files(task)
        files[task['source']] += b'\ndel CASE_ROOTS\ndef __getattr__(name): raise RuntimeError("private-payload")\n'
        output = self.root / 'case-root-exception'
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            report = oracle.grade(task, files, output)
        self.assertEqual(report['cause'], 'candidate_exception', report)
        self.assertEqual(report['calls'], 1)
        self.assertEqual(report['tests']['tests'], 0)
        self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
        self.assertNotIn('private-payload', (output / 'wire.log').read_text())

    def test_wrong_json_helper_values_fail_original_assertions_without_policy_or_protocol(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-006')
        for index, value in enumerate(('None', '"wrong"', '1', 'False', '[1]', '[None]', '{}', '[{}]')):
            files = self.files(task)
            files[task['source']] += ('\ndef route_grant_arguments(args): return ' + value + '\n').encode()
            with self.subTest(value=value), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / ('wrong-helper-' + str(index)))
            self.assertEqual(report['cause'], 'test_failure', report)
            self.assertEqual(report['tests']['failures'], 1)
            self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed']
                             or report['candidate_exception_failed'])

    def test_wrong_verification_values_reach_original_truthiness_assertions(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-020')
        for index, value in enumerate(('None', '"wrong"', '1', '0')):
            files = self.files(task)
            files[task['source']] += ("\ndef verify_case(*args): return {'passed': " + value
                                      + ", 'checks': {'validation': True}}\n").encode()
            with self.subTest(value=value), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / ('wrong-predicate-' + str(index)))
            self.assertEqual(report['cause'], 'test_failure', report)
            self.assertGreater(report['tests']['failures'], 0)
            self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed']
                             or report['candidate_exception_failed'])

    def test_candidate_exceptions_have_one_fixed_cause_without_payloads_and_still_count(self):
        probes = [
            ('helper-raises', 'dev-006', 'def route_grant_arguments(args): raise RuntimeError("private-payload")'),
            ('parser-raises', 'dev-007', 'def parser(): raise RuntimeError("private-payload")'),
            ('predicate-raises', 'dev-020', 'def verify_case(*args): raise RuntimeError("private-payload")'),
            ('missing-checks', 'dev-022', 'def verify_case(*args): return {"passed": True}'),
            ('bool-raises', 'dev-015', 'def case_passed(*args): raise RuntimeError("private-payload")'),
            ('path-raises', 'dev-010', 'def test_only_files(*args): raise RuntimeError("private-payload")'),
            ('map-raises', 'dev-029', 'def suspend_resume_checks(*args): raise RuntimeError("private-payload")'),
            ('bool-systemexit', 'dev-015', 'def case_passed(*args): raise SystemExit("private-payload")'),
        ]
        for name, ident, source in probes:
            task = next(t for t in evaluation.collection() if t['id'] == ident)
            files = self.files(task)
            files[task['source']] += ('\n' + source + '\n').encode()
            output = self.root / name
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, output)
            self.assertEqual(report['cause'], 'candidate_exception', report)
            self.assertTrue(report['candidate_exception_failed'])
            self.assertFalse(report['protocol_failed'] or report['adapter_policy_failed'])
            self.assertEqual(report['candidate_exit_code'], 0)
            self.assertEqual(report['tests']['tests'], task['expected_tests'])
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
            for log in ('wire.log', 'oracle.log'):
                self.assertNotIn('private-payload', (output / log).read_text())

    def test_predicate_data_keys_cannot_forge_control_tags_or_verdicts(self):
        for name, value in (('policy-data', '{"adapter_policy": True}'),
                            ('exception-data', '{"candidate_exception": True}'),
                            ('fake-verdict-data', '{"tests": 1, "passed": True}')):
            report = self.probe(('\ndef case_passed(case): return ' + value + '\n').encode(), name)
            self.assertEqual(report['cause'], 'test_failure', report)
            self.assertFalse(report['candidate_exception_failed'] or report['adapter_policy_failed']
                             or report['protocol_failed'])
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def test_raw_forged_control_tags_still_only_fail_and_are_counted(self):
        for name, result, cause in (('raw-policy', {'adapter_policy': True}, 'adapter_policy'),
                                   ('raw-exception', {'candidate_exception': True}, 'candidate_exception'),
                                   ('raw-payload', {'candidate_exception': True, 'message': 'untrusted'}, 'protocol')):
            frame = oracle.worker.encode_message({'seq': 1, 'result': result})
            source = WIRE_DESCRIPTOR_PROBE + ('\nimport os\ndef case_passed(case):\n os.write(protocol_fd(), ' + repr(frame)
                      + ')\n os._exit(0)\n').encode()
            report = self.probe(source, name)
            self.assertEqual(report['cause'], cause, report)
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})

    def test_reserved_map_keys_reach_unchanged_assertions_and_agree_with_legacy(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-029')
        for key in ('adapter_policy', 'candidate_exception', 'value', 'tests', 'passed'):
            files = self.files(task)
            files[task['source']] += ('\noriginal = suspend_resume_checks\n'
                'def suspend_resume_checks(*args):\n result = original(*args)\n'
                ' result[' + repr(key) + '] = True\n return result\n').encode()
            with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / ('key-' + key))
                legacy = evaluation.grade(task, files, self.root / ('legacy-key-' + key))
            self.assertTrue(report['passed'], report)
            self.assertTrue(legacy['passed'], legacy)
            self.assertEqual(report['cause'], 'none')

    def test_unrepresentable_observations_have_policy_cause_and_clean_candidate_exit(self):
        cases = [('scalar-oversize', 'dev-015', "def case_passed(*a): return 'x' * 70000"),
                 ('scalar-bigint', 'dev-015', 'def case_passed(*a): return 10**5000'),
                 ('scalar-nonfinite', 'dev-015', 'def case_passed(*a): return float("nan")'),
                 ('scalar-object', 'dev-015', 'def case_passed(*a): return object()'),
                 ('scalar-deep', 'dev-015', 'def case_passed(*a):\n v = []\n for _ in range(17): v = [v]\n return v'),
                 ('map-bigint', 'dev-029', 'def suspend_resume_checks(*a): return {"x": 10**5000}'),
                 ('map-oversize', 'dev-029', 'def suspend_resume_checks(*a): return {"x": "a" * 70000}'),
                 ('map-deep', 'dev-029', 'def suspend_resume_checks(*a):\n v = []\n for _ in range(17): v = [v]\n return {"x": v}'),
                 ('argument-bigint', 'dev-006', 'def route_grant_arguments(*a): return [10**5000]'),
                 ('argument-oversize', 'dev-007', 'def memory_arguments(*a): return ["x" * 70000]'),
                 ('verification-oversize', 'dev-020', 'def verify_case(*a): return {"passed": "x" * 70000, "checks": {}}')]
        for name, ident, source in cases:
            task = next(t for t in evaluation.collection() if t['id'] == ident)
            files = self.files(task)
            files[task['source']] += ('\n' + source + '\n').encode()
            with self.subTest(name=name), mock.patch.object(
                    evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
                report = oracle.grade(task, files, self.root / name)
            self.assertEqual(report['cause'], 'adapter_policy', report)
            self.assertTrue(report['adapter_policy_failed'])
            self.assertFalse(report['protocol_failed'] or report['candidate_exception_failed'])
            self.assertEqual(report['candidate_exit_code'], 0)
            self.assertEqual(report['control_attempts'], {'total': 1, 'passed': 0, 'failed': 1})
            self.assertLess((self.root / name / 'wire.log').stat().st_size, oracle.worker.MAX_TRANSCRIPT)

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
                           {"candidate_exception_failed": not base["candidate_exception_failed"]},
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
            ("duplicate-key", WIRE_DESCRIPTOR_PROBE + b'''
import os
def case_passed(case):
 os.write(protocol_fd(), b'{"seq":1,"seq":1,"result":true}\\n')
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
        report = self.probe(WIRE_DESCRIPTOR_PROBE + b'''
import json
import os
original_case_passed = case_passed
calls = 0
def case_passed(case):
 global calls
 calls += 1
 result = original_case_passed(case)
 if calls == 4:
  os.write(protocol_fd(), (json.dumps({'seq':calls,'result':{'value':result}})+'\\n').encode())
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

    def test_launch_uses_anonymous_arguments_and_closes_descriptor_on_success_and_failure(self):
        for fails in (False, True):
            captured = []

            def popen(command, **options):
                descriptor, = options['pass_fds']
                captured.append(descriptor)
                self.assertEqual(options['env'], {})
                self.assertEqual(command, ['/usr/bin/bwrap', '--args', str(descriptor), '--', '/usr/bin/true'])
                self.assertEqual(os.read(descriptor, 4096),
                                 b'--ro-bind\0private source with spaces\0/workspace\0')
                if fails:
                    raise OSError('synthetic exec failure')
                return 'process'

            with self.subTest(failure=fails), mock.patch.object(oracle.subprocess, 'Popen', side_effect=popen):
                command = ['/usr/bin/bwrap', '--ro-bind', 'private source with spaces', '/workspace', '--', '/usr/bin/true']
                if fails:
                    with self.assertRaises(OSError):
                        oracle.launch(command)
                else:
                    self.assertEqual(oracle.launch(command), 'process')
            self.assertEqual(len(captured), 1)
            with self.assertRaises(OSError):
                os.fstat(captured[0])

    def test_mode_fixture_is_writable_only_to_candidate_and_read_only_to_oracle(self):
        fixture = Path('disposable-candidate-tmp')
        candidate = oracle.sandbox(Path('candidate'), Path('candidate-adapter'), Path('result'),
                                   'candidate', candidate_tmp=fixture)
        trusted = oracle.sandbox(Path('trusted'), Path('oracle-adapter'), Path('result'),
                                 'oracle', candidate_tmp=fixture)
        index = candidate.index(str(fixture))
        self.assertEqual(candidate[index-1:index+2], ['--bind', str(fixture), '/tmp'])
        index = trusted.index(str(fixture))
        self.assertEqual(trusted[index-1:index+2], ['--ro-bind', str(fixture), '/candidate-tmp'])
        self.assertNotIn('/candidate-tmp', candidate)
        self.assertNotIn('/output', candidate)
        self.assertNotIn('oracle-adapter', candidate)

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
                 "calls": 4, "protocol_failed": False, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                 "candidate_ready": True, "function_found": True}
        for update in ({"candidate_exception_failed": 1}, {"adapter_policy_failed": 1}, {"tests": {}}, {"calls": True}, {"calls": 0},
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
        valid = {"tests": counts, "calls": 0, "protocol_failed": True, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                 "candidate_ready": False, "function_found": None}
        path = self.root / "startup.json"
        path.write_text(json.dumps(valid))
        self.assertEqual(oracle.result_record(path, self.task), valid)
        for update in ({"calls": 1}, {"protocol_failed": False}, {"function_found": False},
                       {"candidate_exception_failed": True},
                       {"tests": {**counts, "tests": 1}}, {"tests": {**counts, "errors": 1}}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)

    def test_zero_assertions_require_a_failed_case_root_observation(self):
        task = next(t for t in evaluation.collection() if t['id'] == 'dev-022')
        counts = {name: 0 for name in ('tests', 'failures', 'errors', 'skipped',
                                      'expected_failures', 'unexpected_successes')}
        valid = {'tests': counts, 'calls': 1, 'protocol_failed': False,
                 'adapter_policy_failed': True, 'candidate_exception_failed': False,
                 'candidate_ready': True, 'function_found': True}
        path = self.root / 'observation.json'
        for field in ('protocol_failed', 'adapter_policy_failed', 'candidate_exception_failed'):
            value = {**valid, 'protocol_failed': False, 'adapter_policy_failed': False,
                     'candidate_exception_failed': False, field: True}
            path.write_text(json.dumps(value))
            self.assertEqual(oracle.result_record(path, task), value)
            with self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, self.task)
        for update in ({'calls': 0}, {'calls': 2}, {'adapter_policy_failed': False},
                       {'tests': {**counts, 'errors': 1}}, {'candidate_ready': False}):
            path.write_text(json.dumps({**valid, **update}))
            with self.subTest(update=update), self.assertRaises(evaluation.EvaluationError):
                oracle.result_record(path, task)

    def test_every_reported_failure_stays_in_control_attempt_counts(self):
        counts = {name: 0 for name in ("failures", "errors", "skipped",
                                       "expected_failures", "unexpected_successes")}
        ready = {"tests": {**counts, "tests": 1}, "calls": 4, "protocol_failed": False, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                 "candidate_ready": True, "function_found": True}
        not_ready = {"tests": {**counts, "tests": 0}, "calls": 0, "protocol_failed": True, "adapter_policy_failed": False,
                 "candidate_exception_failed": False,
                     "candidate_ready": False, "function_found": None}
        for cause, result in (
                ("none", (0, 0, False, False, ready)),
                ("infrastructure", (1, 1, False, False, not_ready)),
                ("infrastructure", (1, 1, False, False, None)),
                ("timeout", (None, None, True, False, None)),
                ("output_limit", (None, None, False, True, None)),
                ("protocol", (1, 1, False, False, {**ready, "protocol_failed": True})),
                ("adapter_policy", (1, 0, False, False, {**ready, "adapter_policy_failed": True})),
                ("candidate_exception", (1, 0, False, False, {**ready, "candidate_exception_failed": True})),
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
