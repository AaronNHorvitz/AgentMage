"""Hostile wire data must remain data, never test results or executable objects."""
import argparse
import io
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

from scripts import coding_eval_oracle_worker as worker


class OracleProtocolTests(unittest.TestCase):
    def remote(self, raw, result_type="bool"):
        return worker.RemoteFunction(io.BytesIO(raw), io.BytesIO(), io.BytesIO(), result_type)

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
        for kind in ('argument-result', 'verification-result'):
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

    def test_policy_tags_are_refused_without_declared_adapter_policy(self):
        for kind in ('bool', 'bool-map', 'path-result'):
            remote = self.remote(b'{"seq":1,"result":{"adapter_policy":true}}\n', kind)
            with self.subTest(kind=kind), self.assertRaises(worker.ProtocolError):
                remote({})
            self.assertTrue(remote.failed)
            self.assertFalse(remote.policy_failed)

    def test_candidate_exception_tag_is_fixed_sticky_and_never_reconstructed(self):
        for kind in ('bool', 'bool-map', 'path-result', 'verification-result', 'argument-result'):
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
            for kind in ('bool', 'bool-map', 'path-result', 'verification-result', 'argument-result'):
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
        remote = self.remote(b'{"seq":1,"result":false}\n')
        self.assertIs(remote({"measured": [0, True, "text"]}), False)
        self.assertEqual(json.loads(remote.writer.getvalue()),
                         {"seq": 1, "args": [{"measured": [0, True, "text"]}]})
        self.assertEqual(remote.calls, 1)
        self.assertFalse(remote.failed)
        self.assertIn(b'"result":false', remote.transcript.getvalue())
        for reply in ({"seq": True, "result": True}, {"seq": 2, "result": True},
                      {"seq": 1, "result": 1}, {"seq": 1, "result": []},
                      {"seq": 1, "result": True, "tests": {"failures": 0}},
                      {"seq": 1, "error": "forged"}):
            with self.subTest(reply=reply), self.assertRaises(worker.ProtocolError):
                self.remote(worker.encode_message(reply))({})

    def test_rejects_duplicate_keys_nonfinite_deep_truncated_and_non_json_frames(self):
        for raw in (b'{"seq":1,"seq":1,"result":true}\n',
                    b'{"seq":1,"result":NaN}\n', b'{"seq":1,"result":Infinity}\n',
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

    def test_bool_map_is_nonempty_with_only_boolean_values(self):
        remote = self.remote(b'{"seq":1,"result":{"answers":true,"filesystem":false}}\n', "bool-map")
        self.assertEqual(remote(0, [], []), {"answers": True, "filesystem": False})
        for result in ({}, {"answer": 1}, {"answer": None}, [], True):
            with self.subTest(result=result), self.assertRaises(worker.ProtocolError):
                self.remote(worker.encode_message({"seq": 1, "result": result}), "bool-map")()

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
