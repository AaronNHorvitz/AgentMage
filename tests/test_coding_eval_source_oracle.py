"""Development source-fixture adapters, without held-out test imports."""
import contextlib
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_oracle as oracle
from scripts import coding_eval_oracle_worker as worker
from scripts import coding_eval_source_oracle as adapter


class SourceOracleTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def task(self, identifier):
        return next(task for task in evaluation.collection() if task['id'] == identifier)

    def files(self, task, fixed=True):
        files = evaluation.projection(task)
        if fixed:
            files[task['source']] = evaluation.pinned_blob(task['reference'], task['source'], task['reference_sha256'])
        return files

    def probe(self, identifier, change, name):
        task = self.task(identifier)
        files = self.files(task)
        files[task['source']] += change
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            return oracle.grade(task, files, self.root / name)

    def test_generated_source_candidates_refuse_before_creating_output_or_launching(self):
        for identifier in ('dev-001', 'dev-002', 'dev-011', 'dev-014'):
            task = self.task(identifier)
            files = self.files(task)
            files[task['source']] += b'\n# unadmitted synthetic source\n'
            with self.subTest(task=identifier), mock.patch.object(oracle, 'exchange') as exchange:
                with self.assertRaisesRegex(evaluation.EvaluationError, 'untrusted-candidate-disabled'):
                    oracle.grade(task, files, self.root / identifier)
                self.assertFalse((self.root / identifier).exists())
                exchange.assert_not_called()

    def test_all_original_methods_discriminate_bases_and_references(self):
        for identifier in ('dev-001', 'dev-002', 'dev-011', 'dev-014'):
            task = self.task(identifier)
            for fixed in (False, True):
                with self.subTest(task=identifier, fixed=fixed):
                    report = oracle.grade(task, self.files(task, fixed), self.root / (identifier + str(fixed)))
                    self.assertEqual(report['passed'], fixed, report)
                    self.assertEqual(report['tests']['tests'], task['expected_tests'])
                    self.assertFalse(report['protocol_failed'], report)
                    self.assertFalse(report['adapter_policy_failed'], report)
                    self.assertTrue(report['candidate_ready'], report)
                    self.assertEqual(report['candidate_exit_code'], 0, report)
                    self.assertEqual(report['source_adapter_sha256'], evaluation.sha(Path(adapter.__file__).read_bytes()))
                    self.assertTrue(report['inherited_lane_resources']['verified'], report)
                    self.assertIsNone(report['model_score'])
                    self.assertEqual(report['model_attempts'], 0)
                    self.assertFalse(report['aggregate_worker_quotas'])
                    if not fixed:
                        self.assertTrue(oracle.expected_base(task, report), report)

    def test_wrong_source_identities_cannot_patch_the_trusted_assertions(self):
        change = b'''\ndef untracked_file_identities(root): return []
original_identity = implementation_identity
def implementation_identity():
 value = original_identity()
 value['untracked_files_sha256'] = '0' * 64
 return value
import unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertTrue = lambda *args, **kwargs: None
'''
        report = self.probe('dev-001', change, 'wrong-identities')
        self.assertFalse(report['passed'], report)
        self.assertEqual(report['tests']['tests'], 12)

    def test_wrong_closures_and_validation_cannot_patch_the_trusted_assertions(self):
        changes = (
            ('constant', b'''\ndef complete_source_closure(revision):
 return {'schema_version': 1, 'scope': 'complete-committed-source-tree',
         'file_count': 1, 'total_bytes': 0, 'sha256': '0' * 64}
'''),
            ('accept-all', b'''\ndef validate_report(value): return []
def validate_current_source(value, revision): return []
'''),
        )
        for name, change in changes:
            change += b'''\nimport unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertTrue = lambda *args, **kwargs: None
unittest.TestCase.assertNotEqual = lambda *args, **kwargs: None
'''
            with self.subTest(variant=name):
                report = self.probe('dev-014', change, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['tests']['tests'], 7)

    def test_wrong_retention_artifacts_and_values_cannot_patch_assertions(self):
        changes = (
            ('bad-stage', b'''\noriginal_retain = retain
def retain(*args, **kwargs):
 value = original_retain(*args, **kwargs)
 value['schema_version'] = 1
 return value
'''),
            ('missing-artifact', b'''\noriginal_retain = retain
def retain(*args, **kwargs):
 value = original_retain(*args, **kwargs)
 for path in args[2].rglob('stage.json'): path.unlink()
 return value
'''),
            ('ignore-load', b'''\ndef load_stage(directory): return {}
'''),
        )
        for name, change in changes:
            change += b'''\nimport unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertFalse = lambda *args, **kwargs: None
'''
            with self.subTest(variant=name):
                report = self.probe('dev-011', change, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['tests']['tests'], 2)

    def test_wrong_model_request_and_schema_cannot_patch_assertions(self):
        changes = (
            ('claim-serving', b'''\noriginal_request = _model_request_diagnosis
def _model_request_diagnosis(arguments):
 result = original_request(arguments)
 result['serving'] = 'running'
 return result
'''),
            ('schema', b'''\noriginal_diagnose = diagnose
def diagnose(base):
 result = original_diagnose(base)
 result['schema_version'] = 1
 return result
'''),
            ('record-mutation', b'''\noriginal_diagnose = diagnose
def diagnose(base):
 result = original_diagnose(base)
 state, _, _ = paths(base)
 record = state / RUN_RECORD
 if record.exists(): record.write_text('{}')
 return result
'''),
        )
        for name, change in changes:
            change += b'''\nimport unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertNotIn = lambda *args, **kwargs: None
'''
            with self.subTest(variant=name):
                report = self.probe('dev-002', change, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['tests']['tests'], 3)

    def test_protocol_is_separate_from_trusted_fixture_child_standard_output(self):
        # The full diagnosis method launches Git children with inherited stdout.
        # Each retained frame must still be exactly one protocol JSON object.
        task = self.task('dev-002')
        output = self.root / 'stdout'
        report = oracle.grade(task, self.files(task), output)
        self.assertTrue(report['passed'], report)
        frames = [json.loads(line) for line in (output / 'wire.log').read_bytes().splitlines()]
        self.assertTrue(all(type(frame) is dict for frame in frames))
        self.assertIn(b'Initialized empty Git repository', (output / 'oracle.log').read_bytes())

    def test_output_replay_refuses_paths_aliases_and_oversize_before_writes(self):
        target = self.root / 'output'
        cases = (
            [{'path': '.', 'data': None}, {'path': '../outside', 'data': '61'}],
            [{'path': '.', 'data': None}, {'path': '/outside', 'data': '61'}],
            [{'path': '.', 'data': None}, {'path': 'stage/.git', 'data': None}],
            [{'path': '.', 'data': None}, {'path': 'stage/evil.py', 'data': '61'}],
            [{'path': '.', 'data': None}, {'path': 'stage/01.log', 'data': '61'}],
            [{'path': '.', 'data': None}, {'path': '.', 'data': None}],
            [{'path': '.', 'data': None}, {'path': 'stage', 'data': None},
             {'path': 'stage/stage.json', 'data': '61' * (adapter.MAX_ARTIFACT_BYTES + 1)}],
        )
        for entries in cases:
            with self.subTest(paths=[entry['path'] for entry in entries]):
                with self.assertRaises(worker.ProtocolError):
                    adapter.restore_output(target, entries, worker)
                self.assertFalse(target.exists())
        target.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(worker.ProtocolError):
            adapter.restore_output(target, [{'path': '.', 'data': None}], worker)

    def test_output_replay_refuses_an_existing_child_directory_alias(self):
        target = self.root / 'output'
        target.mkdir()
        outside = self.root / 'outside'
        outside.mkdir()
        (target / 'stage').symlink_to(outside, target_is_directory=True)
        with self.assertRaises(worker.ProtocolError):
            adapter.restore_output(target, [{'path': '.', 'data': None},
                                           {'path': 'stage', 'data': None},
                                           {'path': 'stage/stage.json', 'data': '61'}], worker)
        self.assertEqual(list(outside.iterdir()), [])

    def test_only_finite_mocks_can_be_replayed(self):
        with self.assertRaises(worker.AdapterPolicyError):
            adapter.mock_snapshot(lambda: None, worker)
        for value in (mock.Mock(side_effect=lambda: 'side effect'),
                      mock.Mock(wraps=lambda: 'wrapped effect'),
                      mock.Mock(side_effect=AssertionError('unrecognised exception')),
                      mock.Mock(side_effect=iter(range(130)))):
            with self.subTest(value=type(value)), self.assertRaises(worker.AdapterPolicyError):
                adapter.mock_snapshot(value, worker)
        value = mock.Mock(side_effect=[b'a', ValueError('fixture'), b'b'])
        observed = adapter.mock_snapshot(value, worker)
        self.assertEqual(observed['mode'], 'sequence')
        self.assertEqual(value(), b'a')
        with self.assertRaisesRegex(ValueError, 'fixture'):
            value()
        self.assertEqual(value(), b'b')

    def test_typed_mock_data_keeps_binary_git_input(self):
        value = mock.Mock(return_value=subprocess.CompletedProcess([], 0, stdout=b'a\0b', stderr=b''))
        observed = adapter.mock_snapshot(value, worker)
        with contextlib.ExitStack() as stack:
            subject = SimpleNamespace(run=None)
            installed = adapter.install_mock(stack, subject, 'run', observed, worker)
            self.assertEqual(installed(['git']).stdout, b'a\0b')

    def test_record_observation_does_not_follow_candidate_aliases_or_block_on_fifo(self):
        import os
        outside = self.root / 'outside'
        outside.write_bytes(b'not a fixture')
        fixture = self.root / 'fixture'
        fixture.mkdir()
        record = fixture / 'record.json'
        record.symlink_to(outside)
        with self.assertRaises(OSError):
            adapter.record_bytes(fixture, Path('record.json'), worker)
        record.unlink()
        os.mkfifo(record)
        with self.assertRaises(worker.AdapterPolicyError):
            adapter.record_bytes(fixture, Path('record.json'), worker)
        record.unlink()
        (fixture / 'alias').symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(OSError):
            adapter.record_bytes(fixture, Path('alias/outside'), worker)

    def test_retention_reads_real_files_without_following_aliases_or_fifos(self):
        import os
        mount = self.root / 'mount'
        stage = mount / 'fixture/output/fixture-core'
        stage.mkdir(parents=True)
        path = stage / 'stage.json'
        path.write_bytes(b'{"schema_version":2}')
        entries = adapter.output_snapshot(Path('/tmp/fixture/output'), worker, root=mount)
        self.assertEqual(next(row['data'] for row in entries if row['path'] == 'fixture-core/stage.json'),
                         path.read_bytes().hex())
        path.unlink()
        outside = self.root / 'outside'
        outside.write_bytes(b'not an output')
        path.symlink_to(outside)
        with self.assertRaises(worker.AdapterPolicyError):
            adapter.output_snapshot(Path('/tmp/fixture/output'), worker, root=mount)
        path.unlink()
        os.mkfifo(path)
        with self.assertRaises(worker.AdapterPolicyError):
            adapter.output_snapshot(Path('/tmp/fixture/output'), worker, root=mount)
        self.assertIsNone(adapter.output_snapshot(Path('/tmp/absent/output'), worker, root=mount))

    def test_fixture_roots_refuse_non_temporary_and_aliased_paths(self):
        for name in ('/workspace', '/adapter/test.py', '/tmp', '/tmp/../workspace', '/tmp/a/./b', '/tmp/a//b'):
            with self.subTest(path=name), self.assertRaises(worker.AdapterPolicyError):
                adapter.temporary_root(Path(name) if '//' not in name and '/./' not in name else name, worker)

    def test_tool_metadata_cannot_redirect_trusted_fixture_writes(self):
        for target in ('/output/tests.json', '/tmp/other', '../outside', 'scripts/other.py'):
            class Remote:
                def __call__(self, request):
                    return {'observation': {'kind': 'value', 'value': worker.pack_usage_value({'TOOL': target})},
                            'calls': {}, 'artifacts': None}
            with self.subTest(target=target), self.assertRaisesRegex(worker.ProtocolError, 'tool-constant-refused'):
                adapter.proxies(Remote(), SimpleNamespace(TOOL='scripts/verification_batch.py'), worker, 'retention')
        target = 'scripts/verification_batch.py'
        subject = SimpleNamespace()
        adapter.proxies(Remote(), subject, worker, 'retention')
        self.assertEqual(subject.TOOL, target)

    def test_malformed_source_observation_is_sticky_even_if_value_error_is_caught(self):
        class Remote:
            failed = policy_failed = False
            calls = 0

            def __call__(self, request):
                self.calls += 1
                if request == {'operation': 'constants'}:
                    return {'observation': {'kind': 'value', 'value': worker.pack_usage_value({})},
                            'calls': {}, 'artifacts': None}
                return {'observation': {'kind': 'value-error', 'message': 'inventory invalid'},
                        'calls': {'not-an-admitted-mock': []}, 'artifacts': None}

        remote = Remote()
        subject = SimpleNamespace(ROOT=Path('/workspace'), source_records=None,
                                  subprocess=SimpleNamespace(run=None))
        adapter.proxies(remote, subject, worker, 'closure')
        with self.assertRaises(ValueError):
            subject.complete_source_closure('a' * 40)
        self.assertTrue(remote.failed)
        self.assertFalse(remote.policy_failed)


if __name__ == '__main__':
    unittest.main()
