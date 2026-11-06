#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Measure the snapshot proposal API on development tasks, without executing edits.

No held-out loader, tests, references, Git history, commands or retry feedback
enter this collector. Independent grading remains a separate confined operation.
The original chat-only baseline and its frozen policy remain unchanged.
"""
from __future__ import annotations

import argparse
import http.client
import json
from pathlib import Path
import time

from scripts import coding_eval as evaluation
from scripts import coding_eval_suite as suite
from scripts import local_chat_smoke as chat


VERSION = 'coding-snapshot-proposal-measurement-v1'
POLICY = {'version': VERSION, 'route': '/v1/code/proposals',
          'max_message_bytes': 6000, 'max_tokens': 2048,
          'attempts_per_task': 1, 'retries': 0, 'test_feedback': False,
          'repository_tools': False, 'filesystem_effects': False,
          'native_coding_harness': False, 'split': 'development',
          'context': 'parser-and-lexical-excerpts-from-frozen-editable-source',
          'token_measurement': 'API-does-not-expose-usage'}
MAX_RECORD = 2 * 1024 * 1024


def consume(response, target, source_path, original):
    """Bound event parsing; nested JSON is refused, never an uncaught recursion."""
    if response.getheader('Content-Type') != 'application/x-ndjson':
        raise ValueError('proposal-measurement.media-type')
    size, context, proposal, terminal = 0, None, None, False
    started = time.monotonic()
    while line := response.readline(256 * 1024 + 1):
        size += len(line)
        if size > MAX_RECORD or len(line) > 256 * 1024 or time.monotonic() - started > 125:
            raise ValueError('proposal-measurement.stream-limit')
        try:
            event = json.loads(line, object_pairs_hook=suite.unique_object)
        except RecursionError:
            raise ValueError('proposal-measurement.nested-json') from None
        if type(event) is not dict or terminal:
            raise ValueError('proposal-measurement.stream-order')
        target.write(json.dumps(event, ensure_ascii=True) + '\n')
        kind = event.get('type')
        if kind == 'context' and context is None and proposal is None:
            if event.get('authority') != 'proposal-only' or type(event.get('message_bytes')) is not int \
                    or not 0 < event['message_bytes'] <= POLICY['max_message_bytes']:
                raise ValueError('proposal-measurement.context')
            context = event
        elif kind == 'text' and context is not None and type(event.get('text')) is str:
            pass
        elif kind == 'proposal' and context is not None:
            terminal = True
            if event.get('applied') is not False or event.get('tested') is not False \
                    or event.get('authority') != 'proposal-only':
                raise ValueError('proposal-measurement.effects')
            changes = event.get('changes')
            if type(changes) is not list or len(changes) != 1 or type(changes[0]) is not dict:
                raise ValueError('proposal-measurement.scope')
            change = changes[0]
            if change.get('path') != source_path or type(change.get('content')) is not str:
                raise ValueError('proposal-measurement.scope')
            proposal = change['content'].encode('utf-8')
            if len(proposal) > evaluation.MAX_FILE or proposal == original \
                    or change.get('before_sha256') != evaluation.sha(original) \
                    or change.get('after_sha256') != evaluation.sha(proposal):
                raise ValueError('proposal-measurement.digest')
        elif kind == 'error':
            terminal = True
            # The recorded stream retains its exact content-free terminal code.
        else:
            raise ValueError('proposal-measurement.event')
    if not terminal or proposal is None:
        raise ValueError('proposal-measurement.no-proposal')
    return proposal, {'message_bytes': context['message_bytes'], 'bytes': size}


def one(task, files, address, token, model, output):
    evaluation.new_directory(output)
    started = time.monotonic()
    request = {'model': model, 'task': task['statement'],
               'files': [{'path': task['source'], 'content': files[task['source']].decode('utf-8')}],
               'max_message_bytes': POLICY['max_message_bytes'], 'max_tokens': POLICY['max_tokens']}
    evaluation.publish(output / 'request.json', request)
    connection, proposed, stream, error_type = None, None, None, None
    try:
        with chat.private_output(output / 'events.ndjson') as events:
            connection, response = chat.request(address, token, 'POST', POLICY['route'], request)
            proposed, stream = consume(response, events, task['source'], files[task['source']])
    except (OSError, ValueError, RecursionError, chat.SmokeError, http.client.HTTPException) as error:
        error_type = type(error).__name__
    finally:
        if connection is not None:
            connection.close()
    succeeded = proposed is not None
    record = {'version': VERSION, 'task': task['id'], 'model': model,
              'task_sha256': suite.canonical_digest(task), 'policy_sha256': suite.canonical_digest(POLICY),
              'collector_sha256': evaluation.sha(Path(__file__).read_bytes()),
              'generation_succeeded': succeeded, 'cause': 'none' if succeeded else 'proposal-refused',
              'error_type': error_type, 'wall_seconds': round(time.monotonic() - started, 6),
              'stream': stream, 'tokens': None, 'tokens_reason': POLICY['token_measurement'],
              'native_coding_harness': False, 'retries': 0,
              'logs': {path.name: evaluation.sha(path.read_bytes()) for path in output.iterdir()
                       if path.is_file()}}
    evaluation.publish(output / 'record.json', record)
    return proposed, record


def collect(address, auth, model, output, candidates, *, first=0, count=5):
    for path in (output, candidates):
        suite.private_location(path)
    tasks = suite.development_tasks()
    if not 0 <= first < len(tasks) or not 1 <= count <= 5 or first + count > len(tasks):
        raise ValueError('proposal-measurement.batch')
    token = chat.private_text(auth / 'client.token', 64)
    if len(token) != 64 or any(c not in '0123456789abcdef' for c in token):
        raise ValueError('proposal-measurement.credential')
    evaluation.new_directory(output)
    source = evaluation.sha(Path(__file__).read_bytes())
    rows = []
    for task in tasks[first:first + count]:
        target = candidates / task['id']
        if target.exists() or target.is_symlink():
            raise ValueError('proposal-measurement.candidate-exists')
        files = evaluation.projection(task)
        proposed, record = one(task, files, address, token, model, output / task['id'])
        if proposed is not None:
            files[task['source']] = proposed
        evaluation.new_directory(target)
        for name, content in files.items():
            evaluation.write_file(target / name, content)
        record['candidate_sha256'] = evaluation.sha(files[task['source']])
        record['candidate_is_base_fallback'] = proposed is None
        record['record_sha256'] = evaluation.sha((output / task['id'] / 'record.json').read_bytes())
        rows.append(record)
    if source != evaluation.sha(Path(__file__).read_bytes()):
        suite.refuse('proposal-measurement-source-changed')
    summary = {'version': VERSION, 'split': 'development', 'first': first, 'count': count,
               'policy': POLICY, 'collector_sha256': source, 'tasks': rows,
               'generation_succeeded': sum(row['generation_succeeded'] for row in rows),
               'model': model, 'native_coding_harness': False, 'held_out_feedback_used': False}
    evaluation.publish(output / 'batch.json', summary)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api-address', required=True)
    parser.add_argument('--auth-directory', type=Path, required=True)
    parser.add_argument('--model', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--candidates', type=Path, required=True)
    parser.add_argument('--first', type=int, default=0)
    parser.add_argument('--count', type=int, default=5)
    parser.add_argument('--approve-task-context', action='store_true', required=True)
    args = parser.parse_args()
    try:
        result = collect(chat.api_address(args.api_address), args.auth_directory, args.model,
                         args.output, args.candidates, first=args.first, count=args.count)
        print(json.dumps({'version': VERSION, 'count': result['count'],
                          'generation_succeeded': result['generation_succeeded']}))
        return 0
    except (OSError, ValueError, chat.SmokeError) as error:
        print('proposal-measurement.refused ' + type(error).__name__)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
