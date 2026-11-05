"""Independent Rust observations, without importing any held-out tests."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_oracle as oracle
from scripts import coding_eval_rust_oracle as adapter


class RustOracleTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.task = evaluation.coding_eval_rust.TASKS[0]

    def files(self, fixed=True):
        files = evaluation.projection(self.task)
        if fixed:
            files[self.task['source']] = evaluation.pinned_blob(
                self.task['reference'], self.task['source'], self.task['reference_sha256'])
        return files

    def probe(self, change, name):
        files = self.files()
        files[self.task['source']] = change(files[self.task['source']])
        with mock.patch.object(evaluation, 'require_historical_candidate', return_value='synthetic-probe'):
            return oracle.grade(self.task, files, self.root / name)

    def test_original_corpus_discriminates_without_candidate_assertions(self):
        for fixed in (False, True):
            with self.subTest(fixed=fixed):
                report = oracle.grade(self.task, self.files(fixed), self.root / str(fixed))
                self.assertEqual(report['passed'], fixed, report)
                self.assertEqual(report['tests']['tests'], 1, report)
                self.assertEqual(report['tests']['failures'], int(not fixed), report)
                self.assertEqual(report['calls'], 576 if fixed else 1)
                self.assertFalse(report['protocol_failed'], report)
                self.assertFalse(report['adapter_policy_failed'], report)
                self.assertTrue(report['inherited_lane_resources']['verified'], report)
                self.assertEqual(report['worker_limits']['processes_per_namespace'], 16)
                self.assertEqual(report['worker_limits']['address_bytes_per_process'], 768 * evaluation.MAX_FILE)
                self.assertEqual(report['scratch_limits']['runtime_snapshot_bytes'], 1536 * evaluation.MAX_FILE)
                self.assertEqual(report['materialized_worker_sha256'], evaluation.sha(Path(adapter.__file__).read_bytes()))
                self.assertEqual(report['model_attempts'], 0)
                self.assertIsNone(report['model_score'])
                if not fixed:
                    self.assertTrue(oracle.expected_base(self.task, report), report)
                transcript = (self.root / str(fixed) / 'wire.log').read_bytes()
                self.assertNotIn(b'expected_outcome', transcript)
                self.assertNotIn(b'expected_code', transcript)

    def test_candidate_exit_and_forged_summary_cannot_write_a_trusted_result(self):
        probes = (
            ('early-exit', b'std::process::exit(0);'),
            ('fake-summary', b'println!("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"); std::process::exit(0);'),
            ('hidden-files', b'assert!(!std::path::Path::new("/output/tests.json").exists()); assert!(!std::path::Path::new("/workspace/kernel/contracts/tests/eval_hidden.rs").exists()); assert!(!std::path::Path::new("/workspace/fixtures/paths/v1/corpus.json").exists()); std::process::exit(0);'),
        )
        for name, insertion in probes:
            with self.subTest(name=name):
                report = self.probe(lambda source: source.replace(
                    b'        validate_workspace_id(&workspace_id)?;',
                    b'        ' + insertion + b'\n        validate_workspace_id(&workspace_id)?;', 1), name)
                self.assertFalse(report['passed'], report)
                self.assertTrue(report['protocol_failed'], report)
                self.assertEqual(report['tests']['tests'], 1, report)
                self.assertEqual(report['control_attempts']['failed'], 1)
                if name == 'hidden-files':
                    self.assertNotIn(b'assertion failed', (self.root / name / 'candidate.log').read_bytes())

    def test_wrong_error_indices_and_case_spelling_fail_original_assertions(self):
        variants = (
            ('index', lambda source: source.replace(b'        self.component_index\n', b'        Some(255)\n', 1)),
            ('case', lambda source: source.replace(
                b'WorkspacePathComponent(candidate.to_owned())', b'WorkspacePathComponent(candidate.to_lowercase())')),
        )
        for name, change in variants:
            with self.subTest(name=name):
                report = self.probe(change, name)
                self.assertFalse(report['passed'], report)
                self.assertEqual(report['cause'], 'test_failure', report)
                self.assertEqual(report['tests']['failures'], 1, report)

    def test_generated_rust_refuses_before_runtime_lookup_or_output(self):
        files = self.files()
        files[self.task['source']] += b'\n// unadmitted synthetic source\n'
        with mock.patch.object(evaluation.coding_eval_rust, 'runtime_paths') as runtime:
            with self.assertRaisesRegex(evaluation.EvaluationError, 'untrusted-candidate-disabled'):
                oracle.grade(self.task, files, self.root / 'refused')
        runtime.assert_not_called()
        self.assertFalse((self.root / 'refused').exists())

    def test_observations_and_requests_have_closed_shapes_and_bounds(self):
        self.assertTrue(adapter.valid_request({'workspace': 'test', 'components': ['src']}))
        self.assertTrue(adapter.valid_observation({'components': ['Src', 'src']}))
        self.assertTrue(adapter.valid_observation({'code': 'path.component.colon', 'index': 0}))
        for value in ({'components': ['a'] * 257}, {'components': [False]}, {'components': ['\ud800']},
                      {'code': 'private path text', 'index': 0}, {'code': 'x', 'index': True},
                      {'code': 'x', 'index': -1}, {'code': 'x', 'index': 257},
                      {'code': 'x', 'index': 0, 'passed': True}, {'passed': True}):
            with self.subTest(value=value):
                self.assertFalse(adapter.valid_observation(value))
        for value in ({'workspace': 'x', 'components': [], 'passed': True},
                      {'workspace': 'x' * 129, 'components': []},
                      {'workspace': 'x', 'components': ['x' * 4097]}):
            self.assertFalse(adapter.valid_request(value))

    def test_trusted_constructor_has_no_original_validation_fallback(self):
        source = evaluation.projection(self.task)[self.task['source']]
        shim = adapter.trusted_source(source)
        self.assertNotIn(b'validate_workspace_id(&workspace_id)?;', shim)
        self.assertIn(adapter.CONSTRUCTOR, shim)
        # The selected test and corpus remain exact pinned bytes. The trusted
        # program owns all 640 cases and assertions, not candidate replies.
        corpus = json.loads(evaluation.pinned_blob(self.task['hidden_revision'], self.task['fixture'],
                                                  self.task['fixture_sha256']))
        self.assertEqual(corpus['case_count'], 640)


if __name__ == '__main__':
    unittest.main()
