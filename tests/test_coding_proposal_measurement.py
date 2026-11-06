"""Synthetic snapshot-API measurement contracts; no model or hidden test runs."""
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

from scripts import coding_proposal_measurement as measurement


class ProposalMeasurementTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = b'value = 1\n'
        self.new = b'value = 2\n'
        self.context = {'type': 'context', 'authority': 'proposal-only', 'message_bytes': 1200}
        self.proposal = {'type': 'proposal', 'authority': 'proposal-only',
                         'applied': False, 'tested': False, 'changes': [{
                             'path': 'source.py', 'content': self.new.decode(),
                             'before_sha256': measurement.evaluation.sha(self.source),
                             'after_sha256': measurement.evaluation.sha(self.new)}]}

    def response(self, events):
        response = io.BytesIO(b''.join(json.dumps(e).encode() + b'\n' for e in events))
        response.getheader = lambda _: 'application/x-ndjson'
        return response

    def test_valid_proposal_is_data_with_no_effect_or_test_claim(self):
        result, stream = measurement.consume(self.response([
            self.context, {'type': 'text', 'text': 'untrusted data'}, self.proposal]),
            io.StringIO(), 'source.py', self.source)
        self.assertEqual(result, self.new)
        self.assertEqual(stream['message_bytes'], 1200)
        self.assertGreater(stream['bytes'], 0)

    def test_refuses_partial_reordered_duplicate_nested_oversized_and_effectful_events(self):
        for events in [[], [self.context], [self.proposal], [self.context, self.context],
                       [self.context, self.proposal, {'type': 'text', 'text': 'late'}],
                       [self.context, {'type': 'error', 'code': 'code.proposal.format'}],
                       [dict(self.context, message_bytes=6001), self.proposal],
                       [dict(self.context, message_bytes=True), self.proposal],
                       [self.context, dict(self.proposal, applied=True)],
                       [self.context, dict(self.proposal, tested=True)],
                       [self.context, dict(self.proposal, changes=[])],
                       [self.context, dict(self.proposal, changes=[dict(self.proposal['changes'][0], path='../outside')])],
                       [self.context, dict(self.proposal, changes=[dict(self.proposal['changes'][0], before_sha256='bad')])],
                       [self.context, dict(self.proposal, changes=[dict(self.proposal['changes'][0], after_sha256='bad')])]]:
            with self.subTest(events=events), self.assertRaises(ValueError):
                measurement.consume(self.response(events), io.StringIO(), 'source.py', self.source)
        for raw in [b'{"type":"context","type":"proposal"}\n', b'[' * 2000 + b'0' + b']' * 2000,
                    b' ' * (256 * 1024 + 1)]:
            response = io.BytesIO(raw)
            response.getheader = lambda _: 'application/x-ndjson'
            with self.subTest(raw=raw[:20]), self.assertRaises(ValueError):
                measurement.consume(response, io.StringIO(), 'source.py', self.source)

    def test_collection_whitelists_prompt_fields_and_retains_refused_base_in_denominator(self):
        task = {'id': 'synthetic', 'source': 'source.py', 'statement': 'Repair value.',
                'reference': 'NEVER SEND', 'hidden_test': 'PRIVATE ASSERTIONS'}
        files = {'source.py': self.source, 'helper.py': b'helper'}
        auth = self.root / 'auth'
        auth.mkdir(mode=0o700)
        (auth / 'client.token').write_text('a' * 64)
        (auth / 'client.token').chmod(0o600)
        for name, events, expected in [('valid', [self.context, self.proposal], self.new),
                                       ('refused', [self.context, {'type': 'error', 'code': 'refused'}], self.source)]:
            output, candidates = self.root / name, self.root / (name + '-candidates')
            candidates.mkdir(mode=0o700)
            connection = mock.Mock()
            with mock.patch.object(measurement.suite, 'development_tasks', return_value=[task]), \
                    mock.patch.object(measurement.evaluation, 'projection', return_value=dict(files)), \
                    mock.patch.object(measurement.chat, 'request', return_value=(connection, self.response(events))) as request:
                result = measurement.collect(('127.0.0.1', 1), auth, 'fixture', output, candidates, count=1)
            payload = request.call_args.args[4]
            self.assertEqual(payload, {'model': 'fixture', 'task': 'Repair value.',
                                      'files': [{'path': 'source.py', 'content': self.source.decode()}],
                                      'max_message_bytes': 6000, 'max_tokens': 2048})
            self.assertEqual(request.call_args.args[3], '/v1/code/proposals')
            self.assertNotIn('NEVER SEND', str(payload))
            self.assertNotIn('PRIVATE ASSERTIONS', str(payload))
            self.assertEqual((candidates / 'synthetic/source.py').read_bytes(), expected)
            self.assertEqual((candidates / 'synthetic/helper.py').read_bytes(), b'helper')
            self.assertEqual(result['count'], 1)
            self.assertEqual(result['generation_succeeded'], int(name == 'valid'))
            self.assertEqual(result['tasks'][0]['candidate_is_base_fallback'], name == 'refused')
            for log, digest in result['tasks'][0]['logs'].items():
                self.assertEqual(digest, measurement.evaluation.sha((output / 'synthetic' / log).read_bytes()))
            connection.close.assert_called_once()

    def test_no_held_out_collection_surface_and_fixed_budget(self):
        with self.assertRaises(TypeError):
            measurement.collect(None, None, None, None, None, split='held-out')
        self.assertEqual(measurement.POLICY['attempts_per_task'], 1)
        self.assertFalse(measurement.POLICY['test_feedback'])
        self.assertFalse(measurement.POLICY['filesystem_effects'])


if __name__ == '__main__':
    unittest.main()
