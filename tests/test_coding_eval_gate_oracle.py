"""Independent fast-gate observations, without held-out test imports."""
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_gate_oracle as adapter
from scripts import coding_eval_oracle as oracle


class GateOracleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def task(self, identifier):
        return next(task for task in evaluation.collection() if task['id'] == identifier)

    def files(self, task, fixed=True):
        files = evaluation.projection(task)
        if fixed:
            files[task['source']] = evaluation.pinned_blob(task['reference'], task['source'], task['reference_sha256'])
        return files

    def probe(self, identifier, suffix, name):
        task = self.task(identifier)
        files = self.files(task)
        files[task['source']] += suffix
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            return oracle.grade(task, files, self.root / name)

    def test_complete_original_methods_discriminate_both_frozen_changes(self):
        for identifier in ('dev-003', 'dev-004'):
            task = self.task(identifier)
            for fixed in (False, True):
                with self.subTest(task=identifier, fixed=fixed):
                    report = oracle.grade(task, self.files(task, fixed), self.root / (identifier + str(fixed)))
                    self.assertEqual(report['passed'], fixed, report)
                    self.assertEqual(report['tests']['tests'], task['expected_tests'], report)
                    self.assertFalse(report['protocol_failed'], report)
                    self.assertFalse(report['adapter_policy_failed'], report)
                    self.assertTrue(report['inherited_lane_resources']['verified'])
                    self.assertEqual(report['model_attempts'], 0)
                    self.assertIsNone(report['model_score'])
                    self.assertEqual(report['gate_oracle_sha256'], evaluation.sha(Path(adapter.__file__).read_bytes()))
                    if not fixed:
                        self.assertTrue(oracle.expected_base(task, report), report)

    def test_candidate_cannot_patch_original_assertions_or_forge_a_summary(self):
        report = self.probe('dev-004', b'''
import unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertTrue = lambda *args, **kwargs: None
def plan(*args):
    print('Ran 9 tests; OK')
    return []
''', 'wrong-plan')
        self.assertFalse(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 9, report)
        self.assertGreater(report['tests']['failures'] + report['tests']['errors'], 0)

    def test_candidate_has_no_hidden_result_or_process_inspection_mount(self):
        report = self.probe('dev-004', b'''
import ctypes as boundary_ctypes
import errno as boundary_errno
for boundary_path in ('/adapter/test.py', '/output', '/candidate', '/proc/self', '/proc/1/root'):
    assert not Path(boundary_path).exists(), boundary_path
boundary_libc = boundary_ctypes.CDLL(None, use_errno=True)
class BoundaryHeader(boundary_ctypes.Structure):
    _fields_ = [('version', boundary_ctypes.c_uint), ('pid', boundary_ctypes.c_int)]
class BoundaryCapabilities(boundary_ctypes.Structure):
    _fields_ = [(name, boundary_ctypes.c_uint) for name in ('effective', 'permitted', 'inheritable')]
boundary_header = BoundaryHeader(0x20080522, 0)
boundary_caps = (BoundaryCapabilities * 2)()
assert boundary_libc.capget(boundary_ctypes.byref(boundary_header), boundary_caps) == 0
assert all(getattr(part, name) == 0 for part in boundary_caps for name in ('effective', 'permitted', 'inheritable'))
assert boundary_libc.unshare(0x10000000) == -1
assert boundary_ctypes.get_errno() == boundary_errno.ENOSPC
for boundary_call in (101, 310, 311, 438, 298, 0x40000065):
    boundary_ctypes.set_errno(0)
    assert boundary_libc.syscall(boundary_call, 0, 0, 0, 0, 0, 0) == -1
    assert boundary_ctypes.get_errno() == boundary_errno.EPERM
''', 'boundary')
        self.assertTrue(report['passed'], report)

    def test_missing_supervision_fails_real_process_and_reservation_assertions(self):
        report = self.probe('dev-004', b'''
def execute(root, commands, timeout):
    return 0
''', 'no-execute')
        self.assertFalse(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 9, report)
        self.assertGreater(report['tests']['failures'], 0, report)

    def test_malformed_wire_and_early_exit_never_supply_test_counts(self):
        for name, suffix in (
            ('early-exit', b'\nraise SystemExit(0)\n'),
            ('fake-json', b'\nprint(\'{"tests":{"tests":9,"failures":0}}\')\nraise SystemExit(0)\n'),
        ):
            with self.subTest(name=name):
                report = self.probe('dev-004', suffix, name)
                self.assertFalse(report['passed'], report)
                self.assertIsNone(report['tests'], report)
                self.assertEqual(report['cause'], 'infrastructure')

    def test_generated_input_refuses_before_any_worker_or_output(self):
        for identifier in ('dev-003', 'dev-004'):
            task = self.task(identifier)
            files = self.files(task)
            files[task['source']] += b'\n# unadmitted source\n'
            with self.subTest(task=identifier), mock.patch.object(adapter, 'grade_case') as grade:
                with self.assertRaisesRegex(evaluation.EvaluationError, 'untrusted-candidate-disabled'):
                    oracle.grade(task, files, self.root / identifier)
                grade.assert_not_called()
                self.assertFalse((self.root / identifier).exists())

    def test_closed_values_preserve_types_and_refuse_objects_and_deep_frames(self):
        values = (None, True, 1, 'data', b'bytes', Path('/tmp/data'), [1], (1,), {'kind': [True]})
        for value in values:
            with self.subTest(value=value):
                result = adapter.unpack(adapter.decoded(adapter.encoded(adapter.pack(value))))
                self.assertEqual(result, value)
                self.assertIs(type(result), type(value))
        for value in (object(), {1: 'value'}, set()):
            with self.subTest(value=value), self.assertRaises(adapter.GateProtocolError):
                adapter.pack(value)
        for value in ({'callable': 'exec'}, {'tuple': [1], 'list': [1]}, {'path': False}, {'bytes': 'invalid'}):
            with self.subTest(value=value), self.assertRaises(adapter.GateProtocolError):
                adapter.unpack(value)
        with self.assertRaises(adapter.GateProtocolError):
            adapter.decoded(b'{"a":1,"a":2}')
        with self.assertRaises(adapter.GateProtocolError):
            adapter.encoded('x' * adapter.MAX_BYTES)

    def test_replay_refuses_unowned_process_or_signal_and_unknown_operation(self):
        process = mock.Mock(pid=123)
        module = mock.Mock()
        cases = ([['exec', 'source']], [['killpg', adapter.pack((124, 9))]],
                 [['killpg', adapter.pack((123, 0))]], [['waitid', adapter.pack((1, 124, 0))]])
        for events in cases:
            with self.subTest(events=events), self.assertRaises(adapter.GateProtocolError):
                adapter.replay(events, process, module)
        process.wait.assert_not_called()
        process.poll.assert_not_called()
        module.os.killpg.assert_not_called()

    def test_filter_refuses_unknown_architecture(self):
        with mock.patch.object(adapter.platform, 'machine', return_value='unsupported'):
            with self.assertRaisesRegex(adapter.GateProtocolError, 'architecture-refused'):
                adapter.syscall_filter()

    def test_bootstrap_ancestry_requires_a_strict_child_and_refuses_foreign_or_cyclic_trees(self):
        def kernel(parents):
            def entry(path):
                pid = int(path.split('/')[2])
                return SimpleNamespace(read_text=lambda: f'{pid} (synthetic) S {parents[pid]}')
            return mock.patch.object(adapter, 'Path', side_effect=entry)

        with kernel({91: 81, 81: 71}):
            self.assertEqual(adapter.bootstrap_lineage(91, 71), [91, 81])
        for parents, child in (({91: 81, 81: 1, 1: 0}, 91), ({91: 81, 81: 91}, 91), ({}, 71)):
            with self.subTest(parents=parents), kernel(parents), \
                    mock.patch.object(adapter, 'KILL_GROUP') as group, \
                    mock.patch.object(adapter, 'KILL_PROCESS') as process:
                with self.assertRaisesRegex(adapter.GateProtocolError, 'ancestry-refused'):
                    adapter.bootstrap_lineage(child, 71)
                group.assert_not_called()
                process.assert_not_called()

    def test_git_configuration_cannot_execute_a_hook_in_the_assertion_namespace(self):
        task = self.task('dev-004')
        trusted = self.files(task, False)
        files = self.files(task)
        # Original synthetic fixture only: no frozen test bytes or selections
        # are replaced. The malicious hook reports visibility, never reading
        # hidden contents or attempting to change the result file.
        hidden = b'''
import subprocess
import tempfile
from pathlib import Path
import unittest
from scripts import local_agent_fast_gate as gate
class GitBoundaryTests(unittest.TestCase):
    def test_hook_visibility(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True)
            gate.untracked_whitespace(root)
            subprocess.run(['git', 'status', '--porcelain'], cwd=root, check=True, capture_output=True)
            self.assertEqual((root / 'hook-observation').read_text(), 'confined')
'''
        files[task['source']] += b'''
original_untracked_whitespace = untracked_whitespace
def untracked_whitespace(root):
    original_untracked_whitespace(root)
    hook = root / 'fixture-hook'
    program = ("#!/usr/bin/python3\\nfrom pathlib import Path\\nimport sys\\n"
               "Path(__file__).with_name('hook-observation').write_text("
               "'exposed' if Path('/adapter/test.py').exists() or Path('/output').exists() else 'confined')\\n"
               "sys.stdout.buffer.write(b'token\\\\0')\\n")
    hook.write_text(program)
    hook.chmod(0o700)
    subprocess.run(['git', 'config', 'core.fsmonitor', str(hook)], cwd=root, check=True)
'''
        config = {'function': 'plan', 'result_type': 'gate-result'}
        synthetic = {**task, 'id': 'synthetic-git-boundary', 'reference': 'synthetic-only',
                     'hidden_test': 'tests/synthetic_git_boundary.py', 'hidden_sha256': evaluation.sha(hidden),
                     'expected_tests': 1, 'test_names': ['GitBoundaryTests.test_hook_visibility']}
        source_path = Path(adapter.__file__).resolve()
        source = source_path.read_bytes()
        safe = b"        if command[0] in ('git', '/usr/bin/git'):\n            command = ['/usr/bin/git', *command[1:]]\n"
        unsafe = b"        if command[0] in ('git', '/usr/bin/git'):\n            return original(command, *args, **kwargs)\n"
        self.assertEqual(source.count(safe), 1)
        real_read = Path.read_bytes
        real_blob = evaluation.pinned_blob

        def blob(revision, name, digest):
            if name == synthetic['hidden_test']:
                self.assertEqual((revision, digest), ('synthetic-only', evaluation.sha(hidden)))
                return hidden
            return real_blob(revision, name, digest)

        for bypass in (True, False):
            def read(path):
                return source.replace(safe, unsafe, 1) if bypass and path.resolve() == source_path else real_read(path)

            with self.subTest(bypass=bypass), \
                    mock.patch.object(oracle, 'supported_task', return_value=config), \
                    mock.patch.dict(oracle.SUPPORTED, {synthetic['id']: config}), \
                    mock.patch.object(evaluation, 'projection', return_value=trusted), \
                    mock.patch.object(evaluation, 'pinned_blob', side_effect=blob), \
                    mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'), \
                    mock.patch.object(Path, 'read_bytes', autospec=True, side_effect=read):
                report = oracle.grade(synthetic, files, self.root / ('git' + str(bypass)))
                self.assertEqual(report['passed'], not bypass, report)
                self.assertEqual(report['tests']['tests'], 1, report)
                self.assertEqual(report['cause'], 'test_failure' if bypass else 'none', report)
                self.assertEqual(report['tests']['failures'], int(bypass), report)


if __name__ == '__main__':
    unittest.main()
