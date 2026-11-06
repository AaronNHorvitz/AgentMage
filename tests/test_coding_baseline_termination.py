"""Synthetic retained-log checks. No model, frozen assertions or held-out input."""
import json
from pathlib import Path
import tempfile
import unittest

from scripts import coding_baseline_termination as termination


def log(prompt=100, output=10, truncated=0):
    return (f'prompt eval time = 1 ms / {prompt} tokens\n'
            f'       eval time = 2 ms / {output} tokens\n'
            f'stop processing: n_tokens = {prompt + output - 1}, truncated = {truncated}\n').encode()


class ClassificationTests(unittest.TestCase):
    def test_four_backend_outcomes_are_independent_of_api_status(self):
        cases = (
            (b'request (9000 tokens) exceeds the available context size (8192 tokens)\n'
             b'stop processing: n_tokens = 100, truncated = 0\n', None, None, 'prompt-exceeds-context'),
            (log(8000, 192, 1), 8000, 192, 'context-exhausted'),
            (log(100, 4096), 100, 4096, 'output-cap'),
            (log(), 100, 10, 'stopped'))
        for raw, prompt, output, expected in cases:
            with self.subTest(expected=expected):
                self.assertEqual(termination.classify(raw, {
                    'prompt_tokens': prompt, 'completion_tokens': output}), expected)

    def test_unknown_duplicate_and_contradictory_logs_refuse(self):
        timing = {'prompt_tokens': 100, 'completion_tokens': 10}
        for raw in (b'', b'chat.transport.status', log() + log(), log(output=11),
                    log() + b'request (9000 tokens) exceeds the available context size (8192 tokens)'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                termination.classify(raw, timing)


class ReportTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.files = {}
        for model in termination.MODELS:
            for split, count in (('dev', 30), ('held', 10)):
                split_name = 'development' if split == 'dev' else 'held-out'
                prefix = f'{model}-{split}'
                results, launchers, timings = [], {}, []
                for first in range(0, count, 5):
                    directory = f'{prefix}-batch-{first:02d}'
                    observed = []
                    server = b''
                    for index in range(first, first + 5):
                        task = f'dev-{index:03d}' if split == 'dev' else f'PRIVATE-HELD-{index:03d}'
                        raw = log()
                        timing = {'task': task, 'server_log_start': len(server),
                                  'server_log_bytes': len(raw), 'server_log_sha256': termination.digest(raw),
                                  'prompt_tokens': 100, 'completion_tokens': 10}
                        server += raw
                        observed.append(timing)
                        results.append({'task': task})
                        self.write(f'{directory}/collection/{task}/backend-timing.json', timing)
                        self.write(f'{directory}/collection/{task}/request.json', {'max_tokens': 4096})
                    self.write(directory + '/server/server.log', server)
                    launcher = {'model_id': model, 'split': split_name, 'count': 5,
                                'api_reaped': True, 'settings_revoked': True,
                                'state_directory': str(self.root / directory / 'server'),
                                'backend_timings': observed}
                    launchers[directory] = self.write(directory + '/launcher.json', launcher)
                    self.write(directory + '/properties.json', {'default_generation_settings': {'n_ctx': 8192}})
                    timings.extend(observed)
                aggregate = {'model': model, 'split': split_name, 'tasks': count,
                             'results': results, 'attempts_per_task': 1}
                aggregate_sha = self.write(prefix + '-aggregate.json', aggregate)
                self.write(prefix + '-resources.json', {'model': model, 'split': split_name,
                           'aggregate_sha256': aggregate_sha, 'launchers': launchers, 'backend_timings': timings})
        self.freeze()

    def write(self, name, value):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        raw = value if isinstance(value, bytes) else json.dumps(value).encode()
        path.write_bytes(raw)
        self.files[name] = termination.digest(raw)
        return self.files[name]

    def freeze(self):
        raw = json.dumps({'files': self.files}).encode()
        (self.root / 'retained-manifest.json').write_bytes(raw)
        self.manifest_sha = termination.digest(raw)

    def test_complete_report_preserves_counts_but_never_exports_held_identities(self):
        result = termination.report(self.root, self.manifest_sha)
        self.assertEqual(len(result['development']), 60)
        self.assertEqual([row['classes']['stopped'] for row in result['summaries']], [30, 10, 30, 10])
        serialized = json.dumps(result)
        for private in ('PRIVATE-HELD', str(self.root), 'state_directory', 'server.log'):
            self.assertNotIn(private, serialized)

    def test_changed_manifest_and_bound_files_refuse(self):
        with self.assertRaisesRegex(ValueError, 'manifest-changed'):
            termination.report(self.root, '0' * 64)
        (self.root / 'muse-dev-batch-00/server/server.log').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'input-changed'):
            termination.report(self.root, self.manifest_sha)

    def test_rebound_log_still_must_match_the_frozen_slice(self):
        name = 'muse-dev-batch-00/server/server.log'
        self.write(name, (self.root / name).read_bytes().replace(b'1 ms', b'3 ms', 1))
        self.freeze()
        with self.assertRaisesRegex(ValueError, 'slice-changed'):
            termination.report(self.root, self.manifest_sha)

    def test_changed_request_budget_refuses(self):
        self.write('muse-dev-batch-00/collection/dev-000/request.json', {'max_tokens': 2048})
        self.freeze()
        with self.assertRaisesRegex(ValueError, 'budget-changed'):
            termination.report(self.root, self.manifest_sha)

    def test_rebound_resource_omission_refuses(self):
        name = 'muse-dev-resources.json'
        value = json.loads((self.root / name).read_bytes())
        value['launchers'].pop('muse-dev-batch-00')
        self.write(name, value)
        self.freeze()
        with self.assertRaisesRegex(ValueError, 'batch-coverage'):
            termination.report(self.root, self.manifest_sha)

    def test_external_and_symlink_paths_refuse(self):
        campaign = termination.BoundCampaign(self.root, self.manifest_sha)
        with self.assertRaises(ValueError):
            campaign.relative(self.root.parent)
        (self.root / 'alias').symlink_to(self.root / 'muse-dev-resources.json')
        with self.assertRaisesRegex(ValueError, 'path-refused'):
            campaign.bound('alias')


if __name__ == '__main__':
    unittest.main()
