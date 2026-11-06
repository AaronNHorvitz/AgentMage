"""Synthetic collection contracts; no frozen hidden tests or model are run."""
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import coding_baseline as baseline
from scripts import coding_eval as evaluation


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.task = {'id': 'synthetic', 'source': 'scripts/source.py',
                     'statement': 'Repair the arithmetic.', 'reference': 'NEVER IN PROMPT',
                     'hidden_test': 'PRIVATE ASSERTIONS'}
        self.files = {'scripts/__init__.py': b'', 'scripts/source.py': b'result = 3 + 4\n'}

    def test_prompt_whitelists_only_statement_path_and_complete_base_source(self):
        messages = baseline.messages(self.task, self.files)
        self.assertEqual(messages[0], {'role': 'system', 'content': baseline.INSTRUCTION})
        self.assertEqual(json.loads(messages[1]['content']), {
            'task': self.task['statement'], 'path': self.task['source'],
            'source': self.files[self.task['source']].decode()})
        self.assertNotIn('NEVER IN PROMPT', str(messages))
        self.assertNotIn('PRIVATE ASSERTIONS', str(messages))
        self.assertEqual(baseline.POLICY['attempts_per_task'], 1)
        self.assertFalse(baseline.POLICY['test_feedback'])

    def test_replacements_are_exact_unique_ordered_data(self):
        answer = json.dumps({'edits': [{'old': '3 + 4', 'new': '3 * 4'},
                                       {'old': 'result =', 'new': 'product ='}]})
        self.assertEqual(baseline.apply_edits(self.files[self.task['source']], answer), b'product = 3 * 4\n')
        for invalid in ('{}', '{"edits":[]}', '{"edits":[],"edits":[]}',
                        '{"edits":[{"old":"", "new":"x"}]}',
                        '{"edits":[{"old":"missing", "new":"x"}]}',
                        '{"edits":[{"old":"3", "new":"3"}]}',
                        '{"edits":[{"old":"3", "new":false}]}',
                        '```json\n{"edits":[]}\n```'):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                baseline.apply_edits(self.files[self.task['source']], invalid)
        with self.assertRaises(ValueError):
            baseline.apply_edits(b'x x', '{"edits":[{"old":"x","new":"y"}]}')

    def response(self, events):
        response = io.BytesIO(b''.join(json.dumps(event).encode() + b'\n' for event in events))
        response.getheader = lambda name: 'application/x-ndjson'
        return response

    def collect(self, name, events, **options):
        auth = self.root / ('auth-' + name)
        auth.mkdir(mode=0o700)
        (auth / 'client.token').write_text('a' * 64)
        (auth / 'client.token').chmod(0o600)
        output, candidates = self.root / name, self.root / (name + '-candidates')
        candidates.mkdir(mode=0o700)
        connection = mock.Mock()
        with mock.patch.object(baseline.suite, 'development_tasks', return_value=[self.task]), \
                mock.patch.object(evaluation, 'projection', return_value=dict(self.files)), \
                mock.patch.object(baseline.chat, 'request', return_value=(connection, self.response(events))) as request:
            result = baseline.collect(('127.0.0.1', 1), auth, 'synthetic-model', output, candidates,
                                      count=1, **options)
        request.assert_called_once()
        self.assertEqual(request.call_args.args[4]['max_tokens'], 4096)
        connection.close.assert_called_once()
        return result, output, candidates

    def test_normal_stream_materializes_only_the_frozen_projection_and_binds_raw_records(self):
        answer = '{"edits":[{"old":"3 + 4","new":"3 * 4"}]}'
        result, output, candidates = self.collect('valid', [
            {'type': 'text', 'text': answer}, {'type': 'complete', 'finish': 'stop'}])
        self.assertEqual(result['generation_succeeded'], 1)
        row = result['tasks'][0]
        self.assertFalse(row['candidate_is_base_fallback'])
        self.assertIsNone(row['tokens'])
        self.assertEqual((candidates / 'synthetic/scripts/source.py').read_bytes(), b'result = 3 * 4\n')
        self.assertEqual(row['record_sha256'], evaluation.sha((output / 'synthetic/record.json').read_bytes()))
        for name, digest in row['logs'].items():
            self.assertEqual(digest, evaluation.sha((output / 'synthetic' / name).read_bytes()))

    def test_partial_length_and_malformed_answers_keep_the_task_in_the_denominator(self):
        for name, events in (
                ('partial', [{'type': 'text', 'text': 'partial'}]),
                ('length', [{'type': 'text', 'text': '{}'}, {'type': 'complete', 'finish': 'length'}]),
                ('format', [{'type': 'text', 'text': '{}'}, {'type': 'complete', 'finish': 'stop'}])):
            with self.subTest(name=name):
                result, _, candidates = self.collect(name, events)
                self.assertEqual(result['count'], 1)
                self.assertEqual(result['generation_succeeded'], 0)
                self.assertTrue(result['tasks'][0]['candidate_is_base_fallback'])
                self.assertEqual((candidates / 'synthetic/scripts/source.py').read_bytes(), self.files['scripts/source.py'])

    def test_held_out_collection_refuses_before_loading_without_row_close_purpose(self):
        with mock.patch.object(baseline.suite, 'held_out_tasks') as load:
            with self.assertRaises(ValueError):
                baseline.collect(('127.0.0.1', 1), self.root, 'synthetic-model',
                                 self.root / 'output', self.root / 'candidates',
                                 split='held-out', held_out=self.root / 'descriptor')
        load.assert_not_called()
        self.assertFalse((self.root / 'output').exists())

    def test_aggregate_binds_complete_tasks_generations_and_independent_results(self):
        result, collection, _ = self.collect('aggregate', [
            {'type': 'text', 'text': '{"edits":[{"old":"3 + 4","new":"3 * 4"}]}'},
            {'type': 'complete', 'finish': 'stop'}])
        grading = self.root / 'grading'
        grading.mkdir()
        report = {'candidate_sha256': result['tasks'][0]['candidate_sha256'], 'passed': True}
        evaluation.publish(grading / 'synthetic/report.json', report)
        attempts = {'tasks': [{'task': 'synthetic', 'passed': True, 'cause': 'none', 'known_races': [],
                               'report_sha256': evaluation.sha((grading / 'synthetic/report.json').read_bytes())}]}
        evaluation.publish(grading / 'attempts.json', attempts)
        summary = {'kind': 'generated-candidate-observations', 'split': 'development',
                   'source_hashes': baseline.suite.source_hashes(), 'attempts': 1, 'excluded_attempts': 0,
                   'inherited_lane_resources': {'verified': True}, 'wall_seconds': 0.1,
                   'attempts_sha256': evaluation.sha((grading / 'attempts.json').read_bytes())}
        evaluation.publish(grading / 'summary.json', summary)
        with mock.patch.object(baseline.suite, 'development_tasks', return_value=[self.task]):
            value = baseline.scoreboard([collection], grading, self.root / 'aggregate.json')
            self.assertEqual(value['tasks'], 1)
            self.assertEqual(value['first_attempt_passed'], 1)
            self.assertEqual(value['within_budget_passed'], 1)
            self.assertFalse(value['retries_supported'])
            self.assertFalse(value['native_coding_harness'])
            with self.assertRaisesRegex(ValueError, 'duplicate-task'):
                baseline.scoreboard([collection, collection], grading, self.root / 'duplicate.json')
            with mock.patch.object(baseline.suite, 'development_tasks', return_value=[self.task, {'id': 'missing'}]):
                with self.assertRaisesRegex(ValueError, 'incomplete-split'):
                    baseline.scoreboard([collection], grading, self.root / 'incomplete.json')
            (collection / 'synthetic/answer.txt').write_text('altered')
            with self.assertRaisesRegex(ValueError, 'log-changed'):
                baseline.scoreboard([collection], grading, self.root / 'changed.json')
