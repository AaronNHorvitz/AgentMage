#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Reclassify retained baseline log slices without inference or task execution.

Reads only manifest-bound campaign records. Output includes development task
identifiers and held-out aggregates, never held-out identifiers or raw text.
The frozen collector, its policy and original aggregates remain unchanged.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re


CLASSES = ('prompt-exceeds-context', 'context-exhausted', 'output-cap', 'stopped')
MODELS = ('muse', 'gpt-oss')
CONTEXT = 8192
OUTPUT = 4096
MAX_BYTES = 32 * 1024 * 1024


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError('termination.duplicate-key')
        value[key] = item
    return value


def load(raw):
    return json.loads(raw, object_pairs_hook=unique_object)


def classify(raw, timing):
    """Refuse absent/conflicting diagnostics; never infer a cause from API errors."""
    rejected = re.findall(
        rb'request \((\d+) tokens\) exceeds the available context size \((\d+) tokens\)', raw)
    stops = re.findall(rb'stop processing: n_tokens =\s*(\d+), truncated = ([01])', raw)
    prompts = re.findall(rb'\bprompt eval time\s*=.*?/\s*(\d+) tokens\b', raw)
    outputs = re.findall(rb'(?<!prompt )\beval time\s*=.*?/\s*(\d+) tokens\b', raw)
    if len(stops) != 1:
        raise ValueError('termination.stop-count')
    if rejected:
        if (len(rejected) != 1 or int(rejected[0][0]) <= CONTEXT
                or int(rejected[0][1]) != CONTEXT or stops[0][1] != b'0'
                or prompts or outputs or timing['prompt_tokens'] is not None
                or timing['completion_tokens'] is not None):
            raise ValueError('termination.rejection-conflict')
        return 'prompt-exceeds-context'
    if (len(prompts) != 1 or len(outputs) != 1
            or int(prompts[0]) != timing['prompt_tokens']
            or int(outputs[0]) != timing['completion_tokens']
            or not 0 < int(prompts[0]) <= CONTEXT
            or not 0 < int(outputs[0]) <= OUTPUT):
        raise ValueError('termination.timing-conflict')
    if stops[0][1] == b'1':
        return 'context-exhausted'
    if int(outputs[0]) == OUTPUT:
        return 'output-cap'
    return 'stopped'


class BoundCampaign:
    def __init__(self, root, manifest_sha256):
        self.root = root.resolve(strict=True)
        raw = self.read('retained-manifest.json')
        if digest(raw) != manifest_sha256:
            raise ValueError('termination.manifest-changed')
        self.manifest = load(raw)

    def relative(self, path):
        path = Path(path)
        full = path if path.is_absolute() else self.root / path
        relative = full.resolve(strict=True).relative_to(self.root)
        if full.is_symlink() or full != self.root / relative:
            raise ValueError('termination.path-refused')
        return relative.as_posix()

    def read(self, name):
        path = self.root / self.relative(name)
        with path.open('rb') as source:
            raw = source.read(MAX_BYTES + 1)
        if len(raw) > MAX_BYTES:
            raise ValueError('termination.file-limit')
        return raw

    def bound(self, name, expected=None):
        name = self.relative(name)
        raw = self.read(name)
        actual = digest(raw)
        if self.manifest['files'].get(name) != actual or (expected is not None and expected != actual):
            raise ValueError('termination.input-changed')
        return raw


def report(root, manifest_sha256):
    campaign = BoundCampaign(root, manifest_sha256)
    summaries, development = [], []
    bindings = {}
    for model in MODELS:
        for split, expected in (('dev', 30), ('held', 10)):
            prefix = f'{model}-{split}'
            resource_raw = campaign.bound(prefix + '-resources.json')
            resource = load(resource_raw)
            aggregate_raw = campaign.bound(prefix + '-aggregate.json', resource['aggregate_sha256'])
            aggregate = load(aggregate_raw)
            split_name = 'development' if split == 'dev' else 'held-out'
            if (resource['model'] != model or aggregate['model'] != model
                    or resource['split'] != split_name or aggregate['split'] != split_name
                    or aggregate['tasks'] != expected or len(aggregate['results']) != expected
                    or aggregate['attempts_per_task'] != 1):
                raise ValueError('termination.aggregate-mismatch')
            results = {row['task']: row for row in aggregate['results']}
            if len(results) != expected:
                raise ValueError('termination.duplicate-task')
            expected_launchers = {f'{prefix}-batch-{i:02d}' for i in range(0, expected, 5)}
            if resource['launchers'].keys() != expected_launchers:
                raise ValueError('termination.batch-coverage')
            counts = Counter({name: 0 for name in CLASSES})
            observed, seen = [], set()
            for directory in sorted(expected_launchers):
                launcher = load(campaign.bound(directory + '/launcher.json', resource['launchers'][directory]))
                if (launcher['model_id'] != model or launcher['split'] != split_name
                        or launcher['count'] != 5 or not launcher['api_reaped']
                        or not launcher['settings_revoked']):
                    raise ValueError('termination.launcher-mismatch')
                # Require the actually observed settings, not a remembered profile.
                properties = load(campaign.bound(directory + '/properties.json'))
                settings = properties['default_generation_settings']
                if settings['n_ctx'] != CONTEXT:
                    raise ValueError('termination.budget-changed')
                server = campaign.bound(Path(launcher['state_directory']) / 'server.log')
                for timing in launcher['backend_timings']:
                    task = timing['task']
                    if task not in results or task in seen:
                        raise ValueError('termination.task-coverage')
                    seen.add(task)
                    original = load(campaign.bound(f'{directory}/collection/{task}/backend-timing.json'))
                    if original != timing:
                        raise ValueError('termination.timing-changed')
                    request = load(campaign.bound(f'{directory}/collection/{task}/request.json'))
                    if request['max_tokens'] != OUTPUT:
                        raise ValueError('termination.budget-changed')
                    start, size = timing['server_log_start'], timing['server_log_bytes']
                    if (type(start) is not int or type(size) is not int or start < 0
                            or not 0 < size <= 4 * 1024 * 1024 or start + size > len(server)):
                        raise ValueError('termination.slice-range')
                    raw = server[start:start + size]
                    if digest(raw) != timing['server_log_sha256']:
                        raise ValueError('termination.slice-changed')
                    kind = classify(raw, timing)
                    counts[kind] += 1
                    observed.append(timing)
                    if split == 'dev':
                        if not re.fullmatch(r'dev-(?:\d{3}|rust-\d{3})', task):
                            raise ValueError('termination.development-id')
                        development.append({'model': model, 'task': task, 'backend_termination': kind})
            if seen != results.keys() or observed != resource['backend_timings']:
                raise ValueError('termination.incomplete-split')
            summaries.append({'model': model, 'split': split, 'tasks': expected, 'classes': dict(counts)})
            bindings[prefix] = {'resources_sha256': digest(resource_raw), 'aggregate_sha256': digest(aggregate_raw)}
    return {'version': 'coding-baseline-termination-v1', 'manifest_sha256': manifest_sha256,
            'context_tokens': CONTEXT, 'output_tokens': OUTPUT,
            'summaries': summaries, 'development': development, 'bindings': bindings}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--manifest-sha256', required=True)
    args = parser.parse_args()
    try:
        value = report(args.root, args.manifest_sha256)
    except (OSError, ValueError, KeyError, TypeError, RecursionError):
        print('termination.records-refused')
        return 2
    print(json.dumps(value, sort_keys=True, indent=2))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
