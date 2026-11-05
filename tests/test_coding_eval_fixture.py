"""Development-only independent fixture observations; no held-out imports."""
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


class CodingFixtureTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def task_files(self, identifier):
        task = next(task for task in evaluation.collection() if task['id'] == identifier)
        files = evaluation.projection(task)
        files[task['source']] = evaluation.pinned_blob(task['reference'], task['source'], task['reference_sha256'])
        return task, files

    def probe(self, identifier, suffix, name):
        task, files = self.task_files(identifier)
        files[task['source']] += suffix
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            return oracle.grade(task, files, self.root / name)

    def test_generated_fixture_candidates_are_refused_before_output_or_launch(self):
        for identifier in ('dev-008', 'dev-013', 'dev-025'):
            task, files = self.task_files(identifier)
            files[task['source']] += b'\n# synthetic unadmitted candidate\n'
            with self.subTest(task=identifier), mock.patch.object(oracle, 'exchange') as exchange:
                with self.assertRaisesRegex(evaluation.EvaluationError, 'untrusted-candidate-disabled'):
                    oracle.grade(task, files, self.root / identifier)
                exchange.assert_not_called()
                self.assertFalse((self.root / identifier).exists())

    def test_fixture_paths_refuse_alias_components_and_non_temporary_roots(self):
        for path in ('/output/tests.json', '/adapter/test.py', '/tmp/../output/a',
                     '/tmp/a/./b', '/tmp/a//b', '/tmp/a\0b', 'relative', '/tmp/'):
            with self.subTest(path=path), self.assertRaises(worker.ProtocolError):
                worker.fixture_path(path)
        self.assertEqual(worker.fixture_path('/tmp/synthetic/new'), Path('/tmp/synthetic/new'))

    def test_validation_never_runs_a_candidate_command_in_the_trusted_process(self):
        class Remote:
            fixture_function = 'setup'

            def __call__(self, request):
                self.request = request
                return {'returncode': 1}

        remote = Remote()
        subject = SimpleNamespace(subprocess=subprocess)
        worker.fixture_proxies(remote, subject)
        self.assertIsNot(subject.subprocess, subprocess)
        with mock.patch.object(subprocess, 'run', side_effect=AssertionError('trusted execution')):
            value = subject.subprocess.run(['python3', 'tests/run_validation.py'],
                                           cwd='/tmp/synthetic/disposable/worktree',
                                           check=False, stdout=subprocess.PIPE, text=True)
            self.assertEqual(value.returncode, 1)
            self.assertEqual(remote.request, {'operation': 'validation', 'cwd': '/tmp/synthetic/disposable/worktree'})
            with self.assertRaises(worker.ProtocolError):
                subject.subprocess.run(['python3', '/candidate-tmp/attack.py'], cwd='/tmp/synthetic',
                                       check=False, stdout=subprocess.PIPE, text=True)

    def test_validation_requires_a_workspace_previously_created_by_setup(self):
        class HarnessError(RuntimeError):
            pass

        with mock.patch.object(subprocess, 'run') as run:
            with self.assertRaises(worker.ProtocolError):
                worker.fixture_result(SimpleNamespace(HarnessError=HarnessError),
                                      {'operation': 'validation', 'cwd': '/tmp/unknown'}, set())
            run.assert_not_called()

    def test_catalog_observations_cannot_replay_into_a_real_process_launcher(self):
        subject = SimpleNamespace(subprocess=subprocess)
        remote = SimpleNamespace(fixture_function='doc_pack_arguments')
        worker.fixture_proxies(remote, subject)
        with mock.patch.object(subprocess, 'Popen') as launch:
            with self.assertRaises(worker.ProtocolError):
                subject.catalog(Path('/tmp/synthetic'), ['--doc-pack-list'])
            launch.assert_not_called()

    def test_frozen_fixture_controls_keep_complete_assertions(self):
        for identifier in ('dev-008', 'dev-013', 'dev-025'):
            task, fixed = self.task_files(identifier)
            for kind, files in (('base', evaluation.projection(task)), ('reference', fixed)):
                with self.subTest(task=identifier, kind=kind):
                    report = oracle.grade(task, files, self.root / (identifier + kind))
                    self.assertEqual(report['passed'], kind == 'reference', report)
                    self.assertEqual(report['tests']['tests'], task['expected_tests'])
                    self.assertFalse(report['protocol_failed'], report)
                    self.assertFalse(report['adapter_policy_failed'], report)
                    self.assertEqual(report['candidate_exit_code'], 0)
                    self.assertIsNone(report['model_score'])
                    self.assertEqual(report['model_attempts'], 0)
                    self.assertTrue(report['inherited_lane_resources']['verified'], report)
                    self.assertEqual(report['inherited_lane_resources']['scope'], 'entire-inherited-lane')
                    if kind == 'base':
                        self.assertTrue(oracle.expected_base(task, report), report)

    def test_native_diagnosis_observations_cannot_patch_the_trusted_assertions(self):
        variants = (
            ('trust', b'''\ndef native_executable_prerequisite(path): return 'available'
'''),
            ('timeout', b'''\noriginal_diagnosis = confinement_diagnosis
def confinement_diagnosis():
 result = original_diagnosis()
 result['user_manager'] = 'available'
 result['ready'] = True
 return result
'''),
            ('probe-arguments', b'''\noriginal_diagnosis = confinement_diagnosis
def confinement_diagnosis():
 result = original_diagnosis()
 if subprocess.run.call_args is not None:
  subprocess.run.call_args.kwargs['timeout'] = 10
 return result
'''),
        )
        for name, change in variants:
            change += b'''\nimport unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertFalse = lambda *args, **kwargs: None
unittest.TestCase.assertNotIn = lambda *args, **kwargs: None
'''
            with self.subTest(variant=name):
                report = self.probe('dev-013', change, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['tests']['tests'], 3)
                self.assertEqual(report['cause'], 'test_failure', report)

    def test_native_call_replay_requires_an_inert_manager_mock(self):
        subject = SimpleNamespace(subprocess=subprocess)
        remote = SimpleNamespace(failed=False)
        diagnose = worker.native_proxies(remote, subject)
        with mock.patch.object(subprocess, 'run') as run:
            subject.subprocess = SimpleNamespace(run=lambda *args, **kwargs: self.fail('real execution'))
            with self.assertRaises(worker.ProtocolError):
                diagnose()
            run.assert_not_called()
        self.assertTrue(remote.failed)

    def test_wrong_fixture_files_and_validation_outcomes_cannot_patch_assertions(self):
        variants = (
            ('missing-subtract', b'''\noriginal_setup = setup
def setup(*args, **kwargs):
 result = original_setup(*args, **kwargs)
 (args[0] / 'disposable/worktree/src/subtract.py').unlink(missing_ok=True)
 return result
'''),
            ('passing-validator', b'''\noriginal_setup = setup
def setup(*args, **kwargs):
 result = original_setup(*args, **kwargs)
 (args[0] / 'disposable/worktree/tests/run_validation.py').write_text('raise SystemExit(0)')
 return result
'''),
            ('unexpected-calc', b'''\noriginal_setup = setup
def setup(*args, **kwargs):
 result = original_setup(*args, **kwargs)
 (args[0] / 'disposable/worktree/src/calc.py').touch()
 return result
'''),
        )
        for name, change in variants:
            change += b'''\nimport unittest
unittest.TestCase.assertTrue = lambda *args, **kwargs: None
unittest.TestCase.assertFalse = lambda *args, **kwargs: None
unittest.TestCase.assertNotEqual = lambda *args, **kwargs: None
'''
            with self.subTest(variant=name):
                report = self.probe('dev-025', change, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['tests']['tests'], 1)
                self.assertEqual(report['cause'], 'test_failure', report)

    def test_sample_bytes_modes_seals_and_catalog_arguments_remain_trusted_assertions(self):
        variants = (
            ('bytes', b'''\noriginal_sample = write_sample_pack
def write_sample_pack(directory, *args):
 result = original_sample(directory, *args)
 (directory / 'notes.txt').write_text('changed bytes')
 return result
'''),
            ('mode', b'''\noriginal_sample = write_sample_pack
def write_sample_pack(directory, *args):
 result = original_sample(directory, *args)
 (directory / 'notes.txt').chmod(0o644)
 return result
'''),
            ('seal', b'''\noriginal_sample = write_sample_pack
def write_sample_pack(directory, *args):
 result = original_sample(directory, *args)
 result['manifest_sha256'] = '0' * 64
 (directory / 'manifest.json').write_text(json.dumps(result))
 return result
'''),
            ('catalog', b'''\noriginal_catalog = catalog
def catalog(base, arguments):
 return original_catalog(base, ['--wrong'])
'''),
        )
        for name, change in variants:
            change += b'''\nimport unittest
unittest.TestCase.assertEqual = lambda *args, **kwargs: None
unittest.TestCase.assertNotIn = lambda *args, **kwargs: None
unittest.TestCase.assertIn = lambda *args, **kwargs: None
'''
            with self.subTest(variant=name):
                report = self.probe('dev-008', change, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['tests']['tests'], 2)
                self.assertEqual(report['cause'], 'test_failure', report)


if __name__ == '__main__':
    unittest.main()
