"""Admission policy tests use synthetic receipts and development cases only."""
from contextlib import contextmanager
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import coding_eval as evaluation
from scripts import coding_eval_admission as admission
from scripts import coding_eval_oracle as oracle
from scripts import coding_eval_suite as suite


class AdmissionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    @contextmanager
    def resources(self, *, required=False):
        self.assertTrue(required)
        yield {'verified': True, 'scope': 'synthetic-test-only'}

    def receipt(self):
        root = self.root / 'controls'
        root.mkdir()
        names = ('development-coverage.json', 'development/summary.json',
                 'independent-development/summary.json', 'held-out/controls.json',
                 'independent-held-out/controls.json')
        logs = {}
        for name in names:
            evaluation.write_file(root / name, b'{}')
            logs[name] = evaluation.sha(b'{}')
        value = {'version': suite.VERSION, 'kind': 'complete-suite-historical-controls-only',
                 'passed': True, 'development_tasks': 30, 'held_out_tasks': 10,
                 'independent_development_tasks': 30, 'independent_held_out_tasks': 10,
                 'development_controls_valid': True, 'independent_development_controls_valid': True,
                 'held_out_controls_valid': True, 'independent_held_out_controls_valid': True,
                 'development_sha256': suite.DEVELOPMENT_SHA256, 'held_out_sha256': suite.HELD_OUT_SHA256,
                 'control_attempts': 160, 'input_refused': False,
                 'held_out_methods': 52, 'held_out_test_methods_run': 104,
                 'independent_held_out_test_methods_run': 104, 'source_changed': False,
                 'generated_scoring': False,
                 'source_hashes': admission.source_hashes(), 'logs': logs}
        evaluation.publish(root / 'summary.json', value)
        return root / 'summary.json', value

    def test_control_receipt_requires_complete_matching_sources_and_bound_logs(self):
        path, value = self.receipt()
        self.assertEqual(admission.control_receipt(path), evaluation.sha(path.read_bytes()))
        for key, wrong in (('control_attempts', 159), ('passed', 1),
                           ('source_hashes', {}), ('version', 'old')):
            with self.subTest(key=key):
                path.unlink()
                evaluation.publish(path, {**value, key: wrong})
                with self.assertRaises(evaluation.EvaluationError):
                    admission.control_receipt(path)
        path.unlink()
        evaluation.publish(path, value)
        (path.parent / 'held-out/controls.json').unlink()
        evaluation.write_file(path.parent / 'held-out/controls.json', b'{"changed":true}')
        with self.assertRaises(evaluation.EvaluationError):
            admission.control_receipt(path)

    def test_held_out_scoring_requires_row_close_purpose_before_reading_descriptor(self):
        output = self.root / 'refused'
        with mock.patch.object(suite, 'held_out_tasks') as load, mock.patch.object(admission, 'control_receipt') as receipt:
            with self.assertRaises(evaluation.EvaluationError):
                admission.score(self.root, output, self.root / 'receipt', split='held-out',
                                held_out=self.root / 'private')
        load.assert_not_called()
        receipt.assert_not_called()
        self.assertFalse(output.exists())

    def test_missing_receipt_and_missing_required_quotas_launch_no_candidate(self):
        output = self.root / 'refused'
        with mock.patch.object(oracle, 'grade_observations') as grade:
            with self.assertRaises(OSError):
                admission.score(self.root, output, self.root / 'missing')
            with mock.patch.object(admission, 'control_receipt', return_value='synthetic'), \
                    mock.patch.object(evaluation.coding_eval_resources, 'snapshot', side_effect=OSError):
                with self.assertRaises(ValueError):
                    admission.score(self.root, output, self.root / 'unused')
        grade.assert_not_called()
        self.assertFalse(output.exists())

    def test_complete_set_keeps_missing_and_policy_refused_attempts_in_denominator(self):
        tasks = [{'id': 'synthetic-one', 'source': 'scripts/fixture.py'},
                 {'id': 'synthetic-two', 'source': 'scripts/fixture.py'}]
        candidates = self.root / 'candidates'
        evaluation.write_file(candidates / tasks[0]['id'] / 'scripts/fixture.py', b'proposed')
        def grade(task, files, output, **options):
            self.assertEqual(options, {'candidate_kind': 'generated', 'required_resources': True})
            self.assertEqual(files, {'scripts/fixture.py': b'proposed'})
            evaluation.new_directory(output)
            report = {'passed': False, 'cause': 'adapter_policy', 'adapter_policy_failed': True}
            evaluation.publish(output / 'report.json', report)
            return report
        output = self.root / 'scored'
        with mock.patch.object(admission, 'control_receipt', return_value='synthetic'), \
                mock.patch.object(suite, 'development_tasks', return_value=tasks), \
                mock.patch.object(evaluation, 'projection', return_value={'scripts/fixture.py': b'base'}), \
                mock.patch.object(evaluation.coding_eval_resources, 'measurement', self.resources), \
                mock.patch.object(oracle, 'grade_observations', side_effect=grade) as graded:
            result = admission.score(candidates, output, self.root / 'unused')
        self.assertEqual(graded.call_count, 1)
        self.assertEqual(result['tasks'], 2)
        self.assertEqual(result['attempts'], 2)
        self.assertEqual(result['passed_tasks'], 0)
        self.assertEqual(result['ungradable_attempts'], 2)
        self.assertEqual(result['excluded_attempts'], 0)
        self.assertEqual(result['model_attempts'], 0)
        self.assertIsNone(result['model_score'])
        rows = json.loads((output / 'attempts.json').read_bytes())['tasks']
        self.assertEqual([row['task'] for row in rows], [task['id'] for task in tasks])

    def test_extra_hidden_file_is_refused_without_executing_candidate(self):
        tasks = [{'id': 'synthetic', 'source': 'scripts/fixture.py'}]
        candidates = self.root / 'candidates'
        evaluation.write_file(candidates / 'synthetic/scripts/fixture.py', b'proposed')
        evaluation.write_file(candidates / 'synthetic/tests/hidden.py', b'forged')
        with mock.patch.object(admission, 'control_receipt', return_value='synthetic'), \
                mock.patch.object(suite, 'development_tasks', return_value=tasks), \
                mock.patch.object(evaluation, 'projection', return_value={'scripts/fixture.py': b'base'}), \
                mock.patch.object(evaluation.coding_eval_resources, 'measurement', self.resources), \
                mock.patch.object(oracle, 'grade_observations') as grade:
            result = admission.score(candidates, self.root / 'scored', self.root / 'unused')
        grade.assert_not_called()
        self.assertEqual(result['attempts'], 1)
        self.assertEqual(result['passed_tasks'], 0)
        self.assertEqual(result['ungradable_attempts'], 1)

    def test_required_final_observation_failure_publishes_no_verified_summary(self):
        @contextmanager
        def fail_after(*, required=False):
            self.assertTrue(required)
            yield {'verified': False}
            raise ValueError('synthetic-resource-change')
        output = self.root / 'failed'
        with mock.patch.object(admission, 'control_receipt', return_value='synthetic'), \
                mock.patch.object(suite, 'development_tasks', return_value=[]), \
                mock.patch.object(evaluation.coding_eval_resources, 'measurement', fail_after):
            with self.assertRaisesRegex(ValueError, 'synthetic-resource-change'):
                admission.score(self.root, output, self.root / 'unused')
        self.assertTrue((output / 'attempts.json').exists())
        self.assertFalse((output / 'summary.json').exists())

    def test_generated_development_function_cannot_disable_trusted_assertions(self):
        task = next(task for task in suite.development_tasks() if task['id'] == 'dev-015')
        files = evaluation.projection(task)
        reference = evaluation.pinned_blob(task['reference'], task['source'], task['reference_sha256'])
        for name, suffix, expected in (
                ('correct', b'\n# synthetic generated-candidate admission probe\n', True),
                ('wrong', b'\nimport unittest\nunittest.TestCase.assertTrue=lambda *a,**k:None\n'
                          b'unittest.TestCase.assertFalse=lambda *a,**k:None\n'
                          b'def case_passed(*a,**k):return False\n', False)):
            with self.subTest(name=name):
                files[task['source']] = reference + suffix
                report = oracle.grade_observations(task, files, self.root / name,
                                                    candidate_kind='generated', required_resources=True)
                self.assertEqual(report['passed'], expected, report)
                self.assertEqual(report['candidate_kind'], 'generated')
                self.assertTrue(report['inherited_lane_resources']['verified'])
                self.assertEqual(report['tests']['tests'], task['expected_tests'])
                self.assertEqual(report['control_attempts']['total'], 0)
                self.assertEqual(report['candidate_attempts']['total'], 1)
