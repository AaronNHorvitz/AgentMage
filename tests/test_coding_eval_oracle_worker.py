"""Hostile wire data must remain data, never test results or executable objects."""
import argparse
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

from scripts import coding_eval_oracle_worker as worker


class OracleProtocolTests(unittest.TestCase):
    def remote(self, raw, result_type="bool"):
        return worker.RemoteFunction(io.BytesIO(raw), io.BytesIO(), io.BytesIO(), result_type)

    def test_diagnostic_observations_preserve_text_as_inert_data_and_fixed_runtime_error(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        sentinel = Path(temporary.name) / 'must-not-exist'
        marker = f"__import__('pathlib').Path({str(sentinel)!r}).touch()"
        stderr = io.StringIO()

        def failing(command, root):
            self.assertIs(type(command), tuple)
            self.assertEqual(command, ('synthetic', 'argument'))
            self.assertIs(type(root), type(Path()))
            self.assertEqual(root, Path('/workspace'))
            print(marker, file=worker.sys.stderr)
            raise RuntimeError(marker)

        result = worker.diagnostic_result(failing, {'command': ['synthetic', 'argument'],
                                                   'root': '/workspace'})
        self.assertEqual(result, {'stderr': marker + '\n',
                                  'outcome': {'kind': 'runtime-error', 'message': marker}})
        remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'diagnostic-result')
        with worker.contextlib.redirect_stderr(stderr), self.assertRaises(RuntimeError) as caught:
            worker.diagnostic_proxy(remote)(('synthetic', 'argument'), Path('/workspace'))
        self.assertIs(type(caught.exception), RuntimeError)
        self.assertEqual(str(caught.exception), marker)
        self.assertEqual(stderr.getvalue(), marker + '\n')
        self.assertFalse(sentinel.exists())
        self.assertFalse(remote.failed)
        self.assertFalse(remote.exception_failed)
        # A successful value is not transported: only this failure path is graded.
        result = worker.diagnostic_result(lambda *a: object(), {'command': ['synthetic'], 'root': '/'})
        self.assertEqual(result, {'stderr': '', 'outcome': {'kind': 'return'}})
        remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'diagnostic-result')
        self.assertIsNone(worker.diagnostic_proxy(remote)(('synthetic',), Path('/')))

    def test_diagnostic_wire_refuses_classes_tracebacks_objects_and_malformed_fields(self):
        valid = {'stderr': 'text', 'outcome': {'kind': 'runtime-error', 'message': 'message'}}
        invalid = [
            {**valid, 'class': 'RuntimeError'}, {**valid, 'stderr': []},
            {**valid, 'outcome': {'kind': 'runtime-error', 'message': {}}},
            {**valid, 'outcome': {'kind': 'ValueError', 'message': 'payload'}},
            {**valid, 'outcome': {'kind': 'runtime-error', 'message': 'x', 'traceback': 'code'}},
            {**valid, 'outcome': {'kind': 'return', 'value': 'ungraded'}},
            {**valid, 'outcome': {'kind': 'runtime-error'}},
            {**valid, 'outcome': None},
        ]
        for result in invalid:
            remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'diagnostic-result')
            stderr = io.StringIO()
            with self.subTest(result=result), worker.contextlib.redirect_stderr(stderr):
                with self.assertRaises(worker.ProtocolError):
                    worker.diagnostic_proxy(remote)(('synthetic',), Path('/'))
            self.assertTrue(remote.failed)
            self.assertEqual(stderr.getvalue(), '')
        function = mock.Mock()
        for request in ({'command': [], 'root': '/'}, {'command': ['x'], 'root': []},
                        {'command': ['x'] * 129, 'root': '/'}, {'command': [1], 'root': '/'},
                        {'command': ('x',), 'root': '/'}, {'command': ['x'], 'root': '/', 'code': 'x'}):
            with self.subTest(request=request), self.assertRaises(worker.ProtocolError):
                worker.diagnostic_result(function, request)
        function.assert_not_called()

    def test_diagnostic_response_bound_is_a_counted_sticky_policy_refusal(self):
        result = {'stderr': 'x' * worker.MAX_MESSAGE, 'outcome': {'kind': 'return'}}
        remote = self.remote(worker.response_message(1, result), 'diagnostic-result')
        with self.assertRaises(worker.AdapterPolicyError):
            worker.diagnostic_proxy(remote)(('synthetic',), Path('/'))
        self.assertTrue(remote.policy_failed)
        with self.assertRaises(worker.AdapterPolicyError):
            worker.diagnostic_proxy(remote)(('synthetic',), Path('/'))
        self.assertEqual(remote.calls, 2)

    def test_command_observations_preserve_builtin_types_and_only_replay_into_a_mock(self):
        original = mock.Mock(side_effect=AssertionError('real helper must not run'))
        module = SimpleNamespace(command=original)
        def extract(package, root):
            module.command(['ar', 'x', str(package)], cwd=root)
            module.command(('wrong-container',), cwd=str(root), other={'nested': [None, True, 2, 1.5]})
        result = worker.command_result(extract, module, {'package': '/tmp/p.deb', 'root': '/tmp/root'})
        original.assert_not_called()
        self.assertIs(module.command, original)
        raw = worker.encode_message({'seq': 1, 'result': result})
        remote = self.remote(raw, 'command-result')
        with mock.patch.object(module, 'command') as stub:
            worker.command_proxy(remote, module)(Path('/tmp/p.deb'), Path('/tmp/root'))
        self.assertEqual(stub.call_args_list, [
            mock.call(['ar', 'x', '/tmp/p.deb'], cwd=Path('/tmp/root')),
            mock.call(('wrong-container',), cwd='/tmp/root', other={'nested': [None, True, 2, 1.5]}),
        ])
        self.assertIs(type(stub.call_args_list[0].kwargs['cwd']), type(Path()))
        self.assertIs(type(stub.call_args_list[1].args[0]), tuple)
        remote = self.remote(b'', 'command-result')
        module.command = lambda *a: self.fail('real helper invoked')
        with self.assertRaises(worker.ProtocolError):
            worker.command_proxy(remote, module)(Path('/tmp/p'), Path('/tmp/r'))
        self.assertTrue(remote.failed)
        self.assertEqual(remote.writer.getvalue(), b'')

    def test_helper_call_codec_refuses_objects_depth_unknown_tags_and_malformed_calls(self):
        for value in (object(), {1: 'wrong'}, float('nan')):
            with self.subTest(value=type(value).__name__), self.assertRaises(worker.AdapterPolicyError):
                worker.pack_call_value(value)
        nested = []
        for _ in range(17):
            nested = [nested]
        with self.assertRaises(worker.AdapterPolicyError):
            worker.pack_call_value(nested)
        valid = {'args': worker.pack_call_value((['arg'],)), 'kwargs': worker.pack_call_value({})}
        invalid = [
            {'calls': [valid] * 129}, {'calls': [], 'execute': 'payload'}, {'calls': [None]},
            {'calls': [{**valid, 'kwargs': worker.pack_call_value([])}]},
            {'calls': [{**valid, 'args': worker.pack_call_value(['arg'])}]},
            {'calls': [{**valid, 'args': {'kind': '__reduce__', 'value': 'payload'}}]},
            {'calls': [{**valid, 'kwargs': {'kind': 'dict', 'value': {'x': {
                'kind': 'scalar', 'value': []}}}}]},
            {'calls': [{**valid, 'args': {'kind': 'tuple', 'value': [
                {'kind': 'path', 'value': 1}]}}]},
        ]
        for result in invalid:
            remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'command-result')
            module = SimpleNamespace(command=mock.MagicMock())
            with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                worker.command_proxy(remote, module)(Path('/tmp/p'), Path('/tmp/r'))
            self.assertTrue(remote.failed)
            module.command.assert_not_called()
        module = SimpleNamespace(command=lambda: None)
        for function in (lambda p, r: module.command(object()),
                         lambda p, r: [module.command([]) for _ in range(129)]):
            self.assertEqual(worker.command_result(function, module, {'package': 'p', 'root': 'r'}),
                             {'adapter_policy': True})

    def test_mode_fixture_preserves_text_paths_and_modes_and_only_returns_permission_bits(self):
        with tempfile.TemporaryDirectory(dir='/tmp') as directory:
            root = Path(directory)
            root.rmdir()
            request = {'root': directory, 'entries': {
                '.': {'kind': 'directory', 'mode': 0o700},
                'nested': {'kind': 'directory', 'mode': 0o700},
                'nested/file.txt': {'kind': 'file', 'mode': 0o600, 'text': 'fixture text'},
            }}
            def change(path):
                self.assertEqual(path, root)
                self.assertEqual((path / 'nested/file.txt').read_text(), 'fixture text')
                self.assertEqual(path.stat().st_mode & 0o777, 0o700)
                for item in (path, *path.rglob('*')):
                    item.chmod(0o755 if item.is_dir() else 0o644)
            result = worker.mode_result(change, request)
            self.assertEqual(result, {'observed': True})
            self.assertEqual(worker.fixture_modes(root.parent, root.name, request['entries']),
                             {'.': 0o755, 'nested': 0o755, 'nested/file.txt': 0o644})
            with tempfile.TemporaryDirectory(dir='/tmp') as observed:
                shutil.copytree(root, Path(observed) / root.name)
                # Recreate initial trusted bits; inspect the distinct effect tree.
                for name, entry in request['entries'].items():
                    (root / name).chmod(entry['mode'])
                remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'mode-result')
                worker.mode_proxy(remote, Path(observed))(root)
            self.assertEqual(json.loads(remote.writer.getvalue())['args'], [request])
            self.assertEqual((root / 'nested/file.txt').stat().st_mode & 0o777, 0o644)
            self.assertEqual((root / 'nested/file.txt').read_text(), 'fixture text')

    def test_mode_requests_refuse_aliases_parent_traversal_and_special_entries_before_invocation(self):
        valid = {'root': '/tmp/oracle-fixture', 'entries': {'.': {'kind': 'directory', 'mode': 0o700}}}
        variants = [{**valid, 'root': '/workspace'}, {**valid, 'root': '/tmp/a/../b'},
                    {**valid, 'root': '/tmp/..'}, {**valid, 'extra': True},
                    {**valid, 'entries': {'.': {'kind': 'directory', 'mode': True}}},
                    {**valid, 'entries': {'.': {'kind': 'directory', 'mode': 0o1777}}}]
        for name, entry in (
                ('../escape', {'kind': 'directory', 'mode': 0o700}),
                ('/output', {'kind': 'directory', 'mode': 0o700}),
                ('a/b', {'kind': 'directory', 'mode': 0o700}),
                ('a', {'kind': 'symlink', 'mode': 0o700}), ('a', [])):
            variants.append({**valid, 'entries': {**valid['entries'], name: entry}})
        for request in variants:
            called = mock.Mock()
            with self.subTest(request=request), self.assertRaises(worker.ProtocolError):
                worker.mode_result(called, request)
            called.assert_not_called()
        with tempfile.TemporaryDirectory(dir='/tmp') as directory:
            root = Path(directory)
            (root / 'alias').symlink_to('/etc')
            remote = self.remote(b'', 'mode-result')
            with self.assertRaises(worker.ProtocolError):
                worker.mode_proxy(remote)(root)
            self.assertTrue(remote.failed)
            self.assertEqual(remote.writer.getvalue(), b'')

    def test_malformed_mode_replies_do_not_change_any_trusted_file(self):
        with tempfile.TemporaryDirectory(dir='/tmp') as directory:
            root = Path(directory)
            initial = root.stat().st_mode & 0o777
            for result in ({'modes': {'.': 0o755, '../escape': 0o777}}, {'modes': {}},
                           {'modes': {'.': True}}, {'modes': {'.': 0o1777}},
                           {'modes': {'.': '0755'}}, {'modes': {'.': 0o755}, 'execute': 'payload'},
                           {'observed': 1}, {'observed': True, 'modes': {'.': 0o755}}):
                remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'mode-result')
                with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                    worker.mode_proxy(remote)(root)
                self.assertTrue(remote.failed)
                self.assertEqual(root.stat().st_mode & 0o777, initial)

    def test_actual_mode_inspection_refuses_symlink_leaf_and_ancestor_without_following(self):
        entries = {'.': {'kind': 'directory', 'mode': 0o700},
                   'nested': {'kind': 'directory', 'mode': 0o700},
                   'nested/file': {'kind': 'file', 'mode': 0o600, 'text': 'fixture'}}
        with tempfile.TemporaryDirectory(dir='/tmp') as directory:
            mount = Path(directory)
            root = mount / 'fixture'
            root.mkdir()
            (root / 'nested').symlink_to('/tmp', target_is_directory=True)
            with self.assertRaises(worker.AdapterPolicyError):
                worker.fixture_modes(mount, 'fixture', entries)
            # Opening an aliased ancestor directly is refused by O_NOFOLLOW.
            with self.assertRaises(OSError):
                worker.fixture_modes(mount, 'fixture', {'nested/file': entries['nested/file']})
            (root / 'nested').unlink()
            (root / 'nested').mkdir()
            (root / 'nested/file').symlink_to('/etc/passwd')
            with self.assertRaises(worker.AdapterPolicyError):
                worker.fixture_modes(mount, 'fixture', entries)

    def test_argument_proxy_preserves_namespace_paths_and_order_without_object_deserialization(self):
        values = {'root': {'path': '/expected'}, 'grant': {'path': '/s/grant.json'},
                  'workspace': 'workspace-a', 'list': False, 'count': 3, 'unset': None}
        replies = [{'namespace': values}, {'arguments': ['--route-grant', '/s/grant.json'], 'container': 'list'},
                   {'harness_error': True}, {'parser_exit': True}, {'missing_function': True}]
        raw = b''.join(worker.encode_message({'seq': index, 'result': reply})
                       for index, reply in enumerate(replies, 1))
        remote = self.remote(raw, 'argument-result')
        module = SimpleNamespace(HarnessError=ValueError)
        parser, arguments = worker.argument_proxies(remote, module)
        parsed = parser().parse_args(['route-grant', '--root', '/expected'])
        self.assertEqual(parsed.root, Path('/expected'))
        self.assertEqual(worker.pack_namespace(parsed), values)
        self.assertEqual(arguments(parsed), ['--route-grant', '/s/grant.json'])
        with self.assertRaisesRegex(ValueError, 'oracle.argument-refused'):
            arguments(parsed)
        with self.assertRaises(SystemExit) as exit_error:
            parser().parse_args([])
        self.assertEqual(exit_error.exception.code, 2)
        with self.assertRaisesRegex(AttributeError, 'oracle.missing-function'):
            arguments(parsed)
        self.assertFalse(remote.failed)
        self.assertNotIn(b'__reduce__', remote.writer.getvalue())
        self.assertEqual(remote.calls, 5)

    def test_argument_tags_namespaces_and_replies_fail_closed(self):
        request = {'operation': 'parse', 'argv': []}
        for reply in ({'parser_exit': 2}, {'harness_error': 'ValueError'},
                      {'missing_function': True, 'message': 'payload'}, {'exception': 'SystemExit'},
                      {'namespace': {'__class__': 'payload'}},
                      {'namespace': {'root': {'path': '/tmp', 'execute': 'payload'}}},
                      {'namespace': {'root': {'__reduce__': 'payload'}}},
                      {'namespace': {'root': []}}, {'namespace': {'root': 1.5}},
                      {'namespace': {'x' * 65: None}},
                      {'namespace': {f'key{i}': i for i in range(129)}},
                      {'namespace': {'root': {'path': 1}}},
                      {'arguments': ['should-be-namespace']}, {'tests': {'failures': 0}},
                      {'namespace': {}, 'passed': True}, [], True):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': reply}), 'argument-result')
            with self.subTest(reply=reply), self.assertRaises(worker.ProtocolError):
                remote(request)
            self.assertTrue(remote.failed)
        for reply in ({'arguments': [1], 'container': 'invalid'}, {'namespace': {}},
                      {'arguments': [], 'container': '__reduce__'},
                      {'arguments': [], 'container': ['list']}, {'arguments': []},
                      {'arguments': [], 'container': 'tuple', 'message': 'payload'}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': reply}), 'argument-result')
            with self.assertRaises(worker.ProtocolError):
                remote({'operation': 'arguments', 'namespace': {}})
            self.assertTrue(remote.failed)

    def test_fixed_policy_tag_is_sticky_and_never_a_successful_observation(self):
        for kind in sorted(worker.POLICY_RESULT_TYPES):
            remote = self.remote(b'{"seq":1,"result":{"adapter_policy":true}}\n', kind)
            with self.subTest(kind=kind), self.assertRaises(worker.AdapterPolicyError):
                remote({})
            self.assertTrue(remote.policy_failed)
            self.assertFalse(remote.failed)
            remote.reader = io.BytesIO(b'{"seq":2,"result":true}\n')
            with self.assertRaises(worker.AdapterPolicyError):
                remote({})
            self.assertEqual(remote.calls, 2)
            self.assertEqual(remote.reader.tell(), 0)
        for result in ({'adapter_policy': 1}, {'adapter_policy': 'payload'},
                       {'adapter_policy': True, 'message': 'payload'}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': result}))
            with self.assertRaises(worker.ProtocolError):
                remote({})
            self.assertTrue(remote.failed)
            self.assertFalse(remote.policy_failed)

    def test_policy_tags_are_refused_for_unknown_result_types(self):
        for kind in ('unknown', '', '__reduce__'):
            remote = self.remote(b'{"seq":1,"result":{"adapter_policy":true}}\n', kind)
            with self.subTest(kind=kind), self.assertRaises(worker.ProtocolError):
                remote({})
            self.assertTrue(remote.failed)
            self.assertFalse(remote.policy_failed)

    def test_candidate_exception_tag_is_fixed_sticky_and_never_reconstructed(self):
        for kind in sorted(worker.POLICY_RESULT_TYPES):
            remote = self.remote(b'{"seq":1,"result":{"candidate_exception":true}}\n', kind)
            with self.subTest(kind=kind), self.assertRaisesRegex(
                    worker.CandidateExecutionError, '^oracle.candidate-exception$'):
                remote({})
            self.assertTrue(remote.exception_failed)
            self.assertFalse(remote.failed)
            self.assertFalse(remote.policy_failed)
            remote.reader = io.BytesIO(b'{"seq":2,"result":true}\n')
            with self.assertRaises(worker.CandidateExecutionError):
                remote({})
            self.assertEqual(remote.reader.tell(), 0)
        for result in ({'candidate_exception': 1}, {'candidate_exception': False},
                       {'candidate_exception': 'payload'},
                       {'candidate_exception': True, 'message': 'payload'}):
            for kind in sorted(worker.POLICY_RESULT_TYPES):
                remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), kind)
                with self.subTest(kind=kind, result=result), self.assertRaises(worker.ProtocolError):
                    remote({})
                self.assertFalse(remote.exception_failed)
                self.assertTrue(remote.failed)

    def test_wrong_json_arguments_are_data_for_original_assertions(self):
        helper = mock.Mock()
        module = SimpleNamespace(HarnessError=ValueError, helper=helper)
        request = {'operation': 'arguments', 'namespace': {}}
        for value in (None, False, 1, 1.5, 'wrong', {'wrong': 'shape'}):
            helper.return_value = value
            packed = worker.argument_result(module, 'helper', request)
            self.assertEqual(packed, {'value': value})
            remote = self.remote(worker.encode_message({'seq': 1, 'result': packed}), 'argument-result')
            _, arguments = worker.argument_proxies(remote, module)
            self.assertEqual(arguments(argparse.Namespace()), value)
            self.assertFalse(remote.failed or remote.policy_failed or remote.exception_failed)
        helper.return_value = [None, 1, False, 'wrong']
        packed = worker.argument_result(module, 'helper', request)
        self.assertEqual(packed, {'arguments': helper.return_value, 'container': 'list'})
        for value in (object(), [object()], float('nan')):
            helper.return_value = value
            self.assertEqual(worker.argument_result(module, 'helper', request), {'adapter_policy': True})

    def test_capture_fixture_transports_data_and_preserves_tuple_results(self):
        module = SimpleNamespace(subprocess=subprocess, COMMANDS=(("fake", "arg"),))
        def capture():
            completed = module.subprocess.run(["must-never-execute"], check=False)
            return completed.stdout, completed.returncode
        module.capture = capture
        module.validate_raw = lambda text: [text]
        request = {'operation': 'capture', 'commands': [['fake', 'arg']],
                   'stdout': 'fixture output', 'returncode': 3, 'stub_kind': 'completed-process'}
        original_run = subprocess.run
        result = worker.capture_result(module, request)
        self.assertIs(subprocess.run, original_run)
        self.assertEqual(module.COMMANDS, (("fake", "arg"),))
        self.assertEqual(result, {'value': ['fixture output', 3], 'container': 'tuple'})
        raw = worker.encode_message({'seq': 1, 'result': result})
        remote = self.remote(raw, 'capture-result')
        fixture = subprocess.CompletedProcess([], 3, stdout='fixture output')
        trusted = SimpleNamespace(subprocess=SimpleNamespace(run=SimpleNamespace(return_value=fixture)),
                                  COMMANDS=(("fake", "arg"),))
        _, proxy = worker.capture_proxies(remote, trusted)
        self.assertEqual(proxy(), ('fixture output', 3))
        self.assertEqual(json.loads(remote.writer.getvalue())['args'], [request])
        self.assertEqual(worker.capture_result(module, {'operation': 'validate', 'text': 'text'}),
                         {'value': ['text'], 'container': 'value'})

    def test_capture_stub_kinds_preserve_original_mock_and_completed_process_observations(self):
        module = SimpleNamespace(subprocess=subprocess, COMMANDS=(("fake",),))
        def capture():
            result = module.subprocess.run(["must-never-execute"])
            return (isinstance(result, subprocess.CompletedProcess), result.stdout,
                    result.returncode, result.stderr is None, isinstance(result.args, list),
                    isinstance(result.stderr.replace("x", "y"), mock.MagicMock)
                    if result.stderr is not None else False)
        module.capture = capture
        for kind, expected in (("completed-process", [True, "output", 2, True, True, False]),
                               ("mock", [False, "output", 2, False, False, True])):
            request = {'operation': 'capture', 'commands': [['fake']], 'stdout': 'output',
                       'returncode': 2, 'stub_kind': kind}
            remote = self.remote(worker.encode_message({'seq': 1, 'result': {
                'value': expected, 'container': 'tuple'}}), 'capture-result')
            trusted = SimpleNamespace(subprocess=subprocess, COMMANDS=(("fake",),))
            _, proxy = worker.capture_proxies(remote, trusted)
            with mock.patch.object(subprocess, 'run') as run:
                if kind == 'completed-process':
                    run.return_value = subprocess.CompletedProcess([], 2, stdout='output')
                else:
                    run.return_value.stdout = 'output'
                    run.return_value.returncode = 2
                self.assertEqual(proxy(), tuple(expected))
                run.assert_not_called()
            self.assertEqual(json.loads(remote.writer.getvalue())['args'], [request])
            self.assertEqual(worker.capture_result(module, request), {'value': expected, 'container': 'tuple'})

    def test_capture_trusted_stub_refuses_unknown_shapes_before_sending(self):
        fixtures = [object(), SimpleNamespace(stdout='output', returncode=0),
                    subprocess.CompletedProcess(['unrepresented'], 0, stdout='output'),
                    subprocess.CompletedProcess([], 0, stdout='output', stderr='unrepresented')]
        for fixture in fixtures:
            remote = self.remote(b'', 'capture-result')
            module = SimpleNamespace(subprocess=SimpleNamespace(run=SimpleNamespace(return_value=fixture)))
            _, proxy = worker.capture_proxies(remote, module)
            with self.subTest(fixture=type(fixture).__name__), self.assertRaises(worker.ProtocolError):
                proxy()
            self.assertTrue(remote.failed)
            self.assertEqual(remote.writer.getvalue(), b'')
            self.assertEqual(remote.calls, 0)

    def test_capture_observation_keys_are_data_and_malformed_tags_fail(self):
        for value in (None, False, 1, 'wrong', [], {'adapter_policy': True, 'candidate_exception': True}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': {
                'value': value, 'container': 'value'}}), 'capture-result')
            validate, _ = worker.capture_proxies(remote, SimpleNamespace())
            self.assertEqual(validate('input'), value)
            self.assertFalse(remote.failed or remote.policy_failed or remote.exception_failed)
        for value in ({'value': True, 'container': 'tuple'}, {'value': [], 'container': ['tuple']},
                      {'value': [], 'container': '__reduce__'}, {'value': [], 'container': 'list'},
                      {'value': [], 'container': 'value', 'tests': 0}, {'value': []}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': value}), 'capture-result')
            with self.subTest(value=value), self.assertRaises(worker.ProtocolError):
                remote({'operation': 'validate', 'text': 'input'})
            self.assertTrue(remote.failed)

    def test_capture_requests_cannot_supply_callbacks_or_arbitrary_attributes(self):
        module = SimpleNamespace(capture=mock.Mock(), validate_raw=mock.Mock())
        valid = {'operation': 'capture', 'commands': [['fake']], 'stdout': 'x',
                 'returncode': 0, 'stub_kind': 'mock'}
        for request in ({'operation': 'other'}, {'operation': 'validate', 'text': 1},
                        {'operation': 'validate', 'text': 'x', 'callback': 'run'},
                        {k: v for k, v in valid.items() if k != 'stub_kind'},
                        {**valid, 'returncode': True}, {**valid, 'commands': ['fake']},
                        {**valid, 'commands': [[]]}, {**valid, 'commands': [['fake']] * 129},
                        {**valid, 'stub_kind': '__reduce__'}, {**valid, 'stub_kind': []},
                        {**valid, 'stub_kind': {'mock': True}}, {**valid, 'stderr': 'payload'}):
            with self.subTest(request=request), self.assertRaises(worker.ProtocolError):
                worker.capture_result(module, request)
        module.capture.assert_not_called()
        module.validate_raw.assert_not_called()

    def test_malformed_adapter_requests_do_not_become_candidate_exceptions_in_worker(self):
        module = SimpleNamespace(capture=mock.Mock(), validate_raw=mock.Mock(),
                                 verify_case=mock.Mock(), helper=mock.Mock(), HarnessError=ValueError)
        requests = [('capture-result', 'capture', {'operation': 'validate', 'text': 1}),
                    ('capture-result', 'capture', {'operation': 'capture', 'stdout': 'x', 'returncode': 0,
                                                  'commands': [['fake']], 'stub_kind': 'unknown'}),
                    ('argument-result', 'helper', {'operation': 'other'}),
                    ('verification-result', 'verify_case', {'case': 'unsupported'}),
                    ('mode-result', 'helper', {'root': '/workspace', 'entries': {}}),
                    ('command-result', 'helper', {'package': 1, 'root': '/tmp/root'})]
        for kind, function, request in requests:
            config = {'module': 'synthetic', 'function': function, 'result_type': kind}
            reader = io.BufferedReader(io.BytesIO(worker.encode_message({'seq': 1, 'args': [request]})))
            writer = io.BytesIO()
            with (mock.patch.object(worker.sys, 'stdin', SimpleNamespace(buffer=reader)),
                  mock.patch.object(worker.sys, 'stdout', SimpleNamespace(buffer=writer)),
                  mock.patch.object(worker.sys, 'path', list(worker.sys.path)),
                  mock.patch.object(worker.importlib, 'import_module', return_value=module),
                  self.subTest(request=request), self.assertRaises(worker.ProtocolError)):
                worker.candidate(config)
            self.assertEqual(writer.getvalue(), b'{"ready":true,"function_found":true}\n')
        module.capture.assert_not_called()
        module.validate_raw.assert_not_called()
        module.verify_case.assert_not_called()
        module.helper.assert_not_called()

    def test_response_encoding_memory_refusal_uses_fixed_policy_tag(self):
        original = worker.json.dumps
        def bounded(value, **kwargs):
            if value.get('result') == {'value': 'large'}:
                raise MemoryError('untrusted memory payload')
            return original(value, **kwargs)
        with mock.patch.object(worker.json, 'dumps', side_effect=bounded):
            result = worker.response_message(7, {'value': 'large'})
        self.assertEqual(result, b'{"seq":7,"result":{"adapter_policy":true}}\n')

    def test_verification_observations_reject_objects_and_bound_nesting(self):
        for value in (None, 'text', 0, 1, True, 0.5, [], {}, {'x': [None, False]}):
            self.assertTrue(worker.observation_value(value))
        for value in (object(), print, float('nan'), float('inf'), {1: True}, (1,)):
            self.assertFalse(worker.observation_value(value))
        nested = []
        nested.append(nested)
        self.assertFalse(worker.observation_value(nested))
        self.assertFalse(worker.observation_value([[[None]]], depth=16))

    def test_argument_tuple_reconstruction_preserves_original_equality_failure(self):
        remote = self.remote(b'{"seq":1,"result":{"arguments":["a"],"container":"tuple"}}\n',
                             'argument-result')
        _, arguments = worker.argument_proxies(remote, SimpleNamespace(HarnessError=ValueError))
        self.assertEqual(arguments(argparse.Namespace()), ('a',))

    def test_case_roots_are_only_bounded_string_observations(self):
        roots = {'case-a': 'same', 'case-b': 'same'}
        remote = self.remote(worker.encode_message({'seq': 1, 'result': {'case_roots': roots}}),
                             'verification-result')
        self.assertEqual(remote({'operation': 'case-roots'}), {'case_roots': roots})
        # Duplicates are valid data: the original assertion must reject them.
        for roots in ([], {'a': {'__reduce__': 'payload'}}, {'a': 1},
                      {'a': 'x' * 257}, {'x' * 257: 'a'},
                      {str(i): 'a' for i in range(129)}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': {'case_roots': roots}}),
                                 'verification-result')
            with self.subTest(roots=roots), self.assertRaises(worker.ProtocolError):
                remote({'operation': 'case-roots'})
            self.assertTrue(remote.failed)

    def test_argument_candidate_runs_only_the_requested_parser_or_helper_and_fixed_exceptions(self):
        parsed = argparse.Namespace(root=Path('/expected'), list=True)
        parse = mock.Mock(return_value=parsed)
        helper = mock.Mock(return_value=['--list'])
        class HarnessError(RuntimeError):
            pass

        module = SimpleNamespace(HarnessError=HarnessError, parser=lambda: SimpleNamespace(parse_args=parse),
                                 helper=helper)
        request = {'operation': 'parse', 'argv': ['--list']}
        self.assertEqual(worker.argument_result(module, 'helper', request),
                         {'namespace': {'root': {'path': '/expected'}, 'list': True}})
        parse.assert_called_once_with(['--list'])
        helper.assert_not_called()
        request = {'operation': 'arguments', 'namespace': worker.pack_namespace(parsed)}
        self.assertEqual(worker.argument_result(module, 'helper', request), {'arguments': ['--list'], 'container': 'list'})
        helper.assert_called_once_with(parsed)
        helper.side_effect = HarnessError('untrusted payload')
        self.assertEqual(worker.argument_result(module, 'helper', request), {'harness_error': True})
        self.assertEqual(worker.argument_result(module, 'absent', request), {'missing_function': True})
        parse.side_effect = SystemExit(2)
        self.assertEqual(worker.argument_result(module, 'helper', {'operation': 'parse', 'argv': []}),
                         {'parser_exit': True})
        for code in (0, True, 'payload'):
            parse.side_effect = SystemExit(code)
            self.assertEqual(worker.argument_result(module, 'helper', {'operation': 'parse', 'argv': []}),
                             {'adapter_policy': True})
        parse.side_effect = RuntimeError('not a registered exception')
        with self.assertRaises(RuntimeError):
            worker.argument_result(module, 'helper', {'operation': 'parse', 'argv': []})
        for invalid in ({'operation': '__reduce__', 'argv': []},
                        {'operation': 'parse', 'argv': [], 'code': 'payload'},
                        {'operation': 'parse', 'argv': [1]}, {'operation': 'parse', 'argv': 'text'},
                        {'operation': 'arguments', 'namespace': {'root': object()}}, []):
            with self.subTest(request=invalid), self.assertRaises(worker.ProtocolError):
                worker.argument_result(module, 'helper', invalid)

    def test_startup_requires_one_exact_frame_before_assertions(self):
        for found in (True, False):
            raw = worker.encode_message({"ready": True, "function_found": found})
            remote = self.remote(raw)
            remote.start()
            self.assertTrue(remote.candidate_ready)
            self.assertIs(remote.function_found, found)
            self.assertEqual(remote.calls, 0)
            self.assertEqual(remote.writer.getvalue(), b'')
            self.assertEqual(remote.transcript.getvalue(), raw)
            with self.assertRaises(worker.ProtocolError):
                remote.start()
            self.assertTrue(remote.failed)

    def test_missing_malformed_or_forged_startup_is_sticky_failure(self):
        for raw in (b'', b'{"ready":true}\n', b'{"ready":1,"function_found":true}\n',
                    b'{"ready":true,"function_found":1}\n',
                    b'{"ready":true,"function_found":true,"tests":1}\n',
                    b'{"ready":true,"ready":true,"function_found":true}\n',
                    b'{"seq":1,"result":true}\n', b'x' * (worker.MAX_MESSAGE + 2)):
            remote = self.remote(raw)
            with self.subTest(raw=raw[:80]), self.assertRaises(worker.ProtocolError):
                remote.start()
            self.assertFalse(remote.candidate_ready)
            self.assertIsNone(remote.function_found)
            self.assertTrue(remote.failed)
            self.assertEqual(remote.calls, 0)
            remote.reader = io.BytesIO(b'{"ready":true,"function_found":true}\n')
            with self.assertRaises(worker.ProtocolError):
                remote.start()
            self.assertFalse(remote.candidate_ready)

    def test_path_calls_rebuild_values_and_only_the_fixed_trusted_exception(self):
        class Ambiguous(ValueError):
            pass

        raw = (b'{"seq":1,"result":{"ambiguous":true}}\n'
               b'{"seq":2,"result":{"paths":["crate/src/test.rs"]}}\n')
        remote = self.remote(raw, "path-result")
        call = worker.path_proxy(remote, Ambiguous)
        sources = {Path("crate/src/test.rs"): "fn test() {}"}
        with self.assertRaisesRegex(Ambiguous, "oracle.remote-ambiguity"):
            call(sources)
        self.assertFalse(remote.failed)
        self.assertEqual(call(sources), frozenset(sources))
        self.assertEqual(remote.calls, 2)
        requests = [json.loads(line) for line in remote.writer.getvalue().splitlines()]
        self.assertEqual(requests[0]["args"], [{"crate/src/test.rs": "fn test() {}"}])
        self.assertEqual(requests[1]["args"], requests[0]["args"])

    def test_path_results_refuse_foreign_paths_duplicates_and_exception_payloads(self):
        for result in ({"paths": ["../outside.rs"]}, {"paths": ["/output/tests.json"]},
                       {"paths": ["x.rs", "x.rs"]}, {"paths": [1]}, {"paths": {}},
                       {"ambiguous": False}, {"ambiguous": "ValueError"},
                       {"ambiguous": True, "message": "executable expression"},
                       {"exception": "AmbiguousReference", "args": []}, {"__reduce__": []}):
            remote = self.remote(worker.encode_message({"seq": 1, "result": result}), "path-result")
            with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                remote({"x.rs": "source"})
            self.assertTrue(remote.failed)

    def test_candidate_path_adapter_transports_no_other_exception_or_message(self):
        class Ambiguous(ValueError):
            pass

        def uncertain(sources):
            raise Ambiguous("candidate-selected payload")

        def failed(sources):
            raise RuntimeError("candidate-selected payload")

        self.assertEqual(worker.path_result(lambda sources: frozenset(sources), Ambiguous,
                                            {"a.rs": "text"}), {"paths": ["a.rs"]})
        self.assertEqual(worker.path_result(uncertain, Ambiguous, {}), {"ambiguous": True})
        with self.assertRaises(RuntimeError):
            worker.path_result(failed, Ambiguous, {})

    def verification_request(self):
        return {"case": "read-arguments-correction", "exit_code": 0,
                "files": {"result.json": '{"scenario":"read-arguments-correction"}',
                          "stdout.jsonl": "", "stderr.log": "", "calc.py": "fixture text\n"},
                "rows": [{"state": "SUCCESS"}], "status": [" M src/calc.py"],
                "validation_returncode": None}

    def test_verification_fixture_is_rebuilt_locally_and_removed_after_success_or_error(self):
        request = self.verification_request()
        module = SimpleNamespace(rows=lambda *a: None, git_status=lambda *a: None, subprocess=subprocess)
        original_rows, original_status, original_run = module.rows, module.git_status, subprocess.run
        roots = []

        def check(case, base, logs, code):
            roots.append(base.parent)
            self.assertEqual((base / 'disposable/worktree/src/calc.py').read_text(), 'fixture text\n')
            self.assertEqual((logs / 'result.json').read_text(), request['files']['result.json'])
            self.assertEqual(module.rows(logs), request['rows'])
            self.assertEqual(module.git_status(base), request['status'])
            self.assertIsNone(module.subprocess.run(['not-executed']).returncode)
            self.assertEqual((case, code), (request['case'], 0))
            return {'passed': True, 'checks': {'fixture': True}, 'other': 'not graded'}

        self.assertEqual(worker.verification_result(check, module, request),
                         {'passed': True, 'checks': {'fixture': True}})

        def fail(*args):
            check(*args)
            raise RuntimeError('fixture failure')

        with self.assertRaises(RuntimeError):
            worker.verification_result(fail, module, request)
        self.assertEqual(len(roots), 2)
        self.assertTrue(all(not root.exists() for root in roots))
        self.assertIs(module.rows, original_rows)
        self.assertIs(module.git_status, original_status)
        self.assertIs(subprocess.run, original_run)

    def test_verification_fixture_rejects_arbitrary_paths_and_nondata_values(self):
        request = self.verification_request()
        for update in ({'files': {**request['files'], '../escape': 'text'}},
                       {'files': {**request['files'], '/output/tests.json': 'text'}},
                       {'files': {'result.json': 'missing files'}},
                       {'files': {**request['files'], 'calc.py': 1}},
                       {'case': 'unregistered'}, {'case': []}, {'exit_code': True}, {'rows': {}},
                       {'status': [1]}, {'validation_returncode': True}, {'extra': 'data'}):
            with self.subTest(update=update), mock.patch.object(worker.tempfile, 'TemporaryDirectory') as create:
                with self.assertRaises(worker.ProtocolError):
                    worker.verification_result(None, None, {**request, **update})
                create.assert_not_called()

    def test_verification_proxy_sends_fixture_data_without_paths_or_assertions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            logs = root / 'logs'
            logs.mkdir()
            for name in ('result.json', 'stdout.jsonl', 'stderr.log'):
                (logs / name).write_text('fixture')
            workspace = root / 'worktree'
            (workspace / 'src').mkdir(parents=True)
            (workspace / 'src/calc.py').write_text('code')
            run = mock.Mock(return_value=subprocess.CompletedProcess([], 7))
            module = SimpleNamespace(
                coding_harness=SimpleNamespace(paths=lambda base: (None, None, workspace)),
                rows=lambda logs: [{'type': 'record'}], git_status=lambda workspace: [' M src/calc.py'],
                subprocess=SimpleNamespace(run=run, PIPE=subprocess.PIPE))
            remote = mock.Mock(return_value={'passed': False, 'checks': {'validation': False}})
            proxy = worker.verification_proxy(remote, module)
            result = proxy('multi-file', root, logs, 0)
            self.assertFalse(result['passed'])
            payload = remote.call_args.args[0]
            self.assertNotIn(directory, json.dumps(payload))
            self.assertEqual(payload['validation_returncode'], 7)
            self.assertEqual(payload['files']['calc.py'], 'code')
            run.assert_called_once()
            remote.return_value = {'missing_case': True}
            with self.assertRaisesRegex(KeyError, 'oracle.missing-case'):
                proxy('read-arguments-denied', root, logs, 8)
            run.assert_called_once()

    def test_verification_replies_accept_only_check_booleans_or_a_fixed_missing_case_tag(self):
        for result in ({'passed': True, 'checks': {'exact': False}}, {'missing_case': True},
                       {'passed': 1, 'checks': {'x': 0}}, {'passed': None, 'checks': {}},
                       {'passed': 'wrong', 'checks': {'x': 'true'}},
                       {'passed': [], 'checks': None}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'verification-result')
            self.assertEqual(remote({}), result)
        for result in ({'passed': True},
                       {'passed': True, 'checks': {'x': True}, 'tests': {'failures': 0}},
                       {'missing_case': False}, {'missing_case': True, 'message': 'payload'},
                       {'exception': 'KeyError'}, [], True):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': result}), 'verification-result')
            with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                remote({})
            self.assertTrue(remote.failed)

    def test_json_call_preserves_arguments_and_requires_exact_sequence_and_type(self):
        remote = self.remote(b'{"seq":1,"result":{"value":false}}\n')
        self.assertIs(remote({"measured": [0, True, "text"]}), False)
        self.assertEqual(json.loads(remote.writer.getvalue()),
                         {"seq": 1, "args": [{"measured": [0, True, "text"]}]})
        self.assertEqual(remote.calls, 1)
        self.assertFalse(remote.failed)
        self.assertIn(b'"result":{"value":false}', remote.transcript.getvalue())
        for reply in ({"seq": True, "result": True}, {"seq": 2, "result": True},
                      {"seq": 1, "result": {}}, {"seq": 1, "result": []},
                      {"seq": 1, "result": {"value": True, "tests": 1}},
                      {"seq": 1, "result": True, "tests": {"failures": 0}},
                      {"seq": 1, "error": "forged"}):
            with self.subTest(reply=reply), self.assertRaises(worker.ProtocolError):
                self.remote(worker.encode_message(reply))({})

    def test_rejects_duplicate_keys_nonfinite_deep_truncated_and_non_json_frames(self):
        for raw in (b'{"seq":1,"seq":1,"result":true}\n',
                    b'{"seq":1,"result":NaN}\n', b'{"seq":1,"result":Infinity}\n',
                    b'{"seq":1,"result":1e999}\n',
                    b'\xff\n', b'{"seq":1,"result":true}', b'', b'Ran 1 test: OK\n',
                    b'[' * 2000 + b']' * 2000 + b'\n', b'{}\n' * 2):
            remote = self.remote(raw)
            with self.subTest(raw=raw[:50]), self.assertRaises(worker.ProtocolError):
                remote({})
            self.assertTrue(remote.failed)
            with self.assertRaises(worker.ProtocolError):
                remote({})

    def test_oversize_input_is_bounded_and_retained_before_refusal(self):
        remote = self.remote(b'x' * (worker.MAX_MESSAGE * 3) + b'\n')
        with self.assertRaises(worker.ProtocolError):
            remote({})
        self.assertLessEqual(len(remote.transcript.getvalue()), worker.MAX_MESSAGE + 100)
        self.assertIn(b'x' * 100, remote.transcript.getvalue())
        self.assertEqual(remote.reader.tell(), worker.MAX_MESSAGE + 1)

    def test_predicate_observations_preserve_json_data_and_types_for_original_assertions(self):
        for value in (None, False, True, 0, 1, -2, 0.5, '', 'wrong', [], [1], {},
                      {'adapter_policy': True}, {'candidate_exception': True}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': {'value': value}}))
            with self.subTest(value=value):
                result = remote({})
                self.assertEqual(result, value)
                self.assertIs(type(result), type(value))
                self.assertFalse(remote.failed or remote.policy_failed or remote.exception_failed)

    def test_bool_map_preserves_bounded_json_values_for_original_assertions(self):
        for value in ({}, {'answers': True, 'filesystem': False}, {'answer': 1},
                      {'answer': None}, {'answer': [1, {'nested': False}]}, {'answer': {}},
                      {'adapter_policy': True}, {'candidate_exception': True},
                      {'value': {'tests': 3, 'passed': True}}):
            remote = self.remote(worker.encode_message({'seq': 1, 'result': {'value': value}}), 'bool-map')
            with self.subTest(value=value):
                self.assertEqual(remote(0, [], []), value)
                self.assertFalse(remote.failed or remote.policy_failed or remote.exception_failed)
        nested = []
        for _ in range(17):
            nested = [nested]
        for result in ([], True, None, {'answer': nested}, {'answer': float('inf')}):
            # Write a valid JSON exponent that decodes to infinity for that case.
            raw = (b'{"seq":1,"result":{"value":{"answer":1e999}}}\n' if result == {'answer': float('inf')}
                   else worker.encode_message({'seq': 1, 'result': {'value': result}}))
            with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                self.remote(raw, 'bool-map')()

    def test_candidate_predicates_wrap_keys_and_classify_unrepresentable_values(self):
        for kind in ('bool', 'bool-map'):
            for value in ({'adapter_policy': True}, {'candidate_exception': True}, {},
                          {'value': [None, True]}):
                self.assertEqual(worker.predicate_result(value, kind), {'value': value})
            cyclic = []
            cyclic.append(cyclic)
            for value in (object(), float('nan'), 1j, {1: True}, {'x': cyclic}):
                self.assertEqual(worker.predicate_result(value, kind), {'adapter_policy': True})
        for value in (None, [], True):
            self.assertEqual(worker.predicate_result(value, 'bool-map'), {'adapter_policy': True})
        self.assertEqual(worker.predicate_result({}, 'unknown'), {'adapter_policy': True})
        self.assertEqual(worker.predicate_result([1], 'bool'), {'value': [1]})

    def test_response_encoding_refusal_is_bounded_policy_not_a_crashed_worker(self):
        for value in ('x' * worker.MAX_MESSAGE, 10 ** 5000, float('nan'), object()):
            frame = worker.response_message(1, {'value': value})
            self.assertLessEqual(len(frame), worker.MAX_MESSAGE)
            for kind in sorted(worker.POLICY_RESULT_TYPES):
                remote = self.remote(frame, kind)
                with self.subTest(kind=kind, value=type(value).__name__), self.assertRaises(worker.AdapterPolicyError):
                    remote({})
                self.assertTrue(remote.policy_failed)
                self.assertFalse(remote.failed or remote.exception_failed)
        overhead = len(worker.response_message(1, {'value': ''}))
        exact = {'value': 'x' * (worker.MAX_MESSAGE - overhead)}
        self.assertEqual(len(worker.response_message(1, exact)), worker.MAX_MESSAGE)
        exact['value'] += 'x'
        self.assertEqual(json.loads(worker.response_message(1, exact)),
                         {'seq': 1, 'result': {'adapter_policy': True}})
        self.assertEqual(worker.response_message(1, {'value': True}),
                         b'{"seq":1,"result":{"value":true}}\n')

    def test_call_and_transcript_budgets_fail_sticky_without_unbounded_logs(self):
        remote = self.remote(b'{"seq":1,"result":true}\n')
        remote.calls = worker.MAX_CALLS
        with self.assertRaises(worker.ProtocolError):
            remote()
        self.assertTrue(remote.failed)
        remote = self.remote(b'{"seq":1,"result":true}\n')
        with mock.patch.object(worker, "MAX_TRANSCRIPT", 8), self.assertRaises(worker.ProtocolError):
            remote({})
        self.assertEqual(len(remote.transcript.getvalue()), 8)
        self.assertEqual(remote.writer.getvalue(), b'')
        self.assertTrue(remote.failed)

    def test_outgoing_objects_and_nonfinite_values_cannot_escape_json(self):
        for value in (object(), float("nan"), float("inf"), "x" * worker.MAX_MESSAGE):
            remote = self.remote(b'{"seq":1,"result":true}\n')
            with self.subTest(value=type(value).__name__), self.assertRaises(worker.ProtocolError):
                remote(value)
            self.assertTrue(remote.failed)
            self.assertEqual(remote.writer.getvalue(), b'')


if __name__ == "__main__":
    unittest.main()
