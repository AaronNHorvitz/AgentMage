"""Grade worker APIs with complete frozen assertions outside candidate namespaces.

Each original selected method has its own sealed oracle and confined candidate.
The admission command separately validates generated projections and quotas.
"""
from __future__ import annotations

import json
from pathlib import Path
import subprocess
import time

if __package__:
    from . import coding_eval as evaluation
    from . import coding_eval_gate_oracle as gate
    from . import coding_eval_oracle as engine
    from . import coding_eval_scratch as scratch
else:
    import coding_eval as evaluation
    import coding_eval_gate_oracle as gate
    import coding_eval_oracle as engine
    import coding_eval_scratch as scratch


VERSION = 'coding-worker-oracle-v1'
SOURCE = 'scripts/coding_eval_oracle_worker.py'
PARTS = {
    'graph.py': 'coding_eval_worker_graph.py',
    'sealed.py': 'coding_eval_worker_seal.py',
    'worker.py': 'coding_eval_worker_client.py',
    'runtime.py': 'coding_eval_worker_runtime.py',
    'facade.py': 'coding_eval_worker_facade.py',
}
TRANSCRIPT_BYTES = 16 * evaluation.MAX_FILE
MAX_METHODS = 52
BOOTSTRAP = b'''
import os,sys
sys.stdout.buffer.write(b'worker-bootstrap-v1\\n');sys.stdout.buffer.flush()
if sys.stdin.buffer.readline(64)!=b'worker-go-v1\\n':raise SystemExit(2)
os.execv('/usr/bin/python3',['/usr/bin/python3','-I','-B','/runner/worker.py'])
'''


def source_parts():
    parts = {name: Path(__file__).with_name(path).read_bytes() for name, path in PARTS.items()}
    # Bind whole helper files, including imports materialized by the scratch
    # holder. RPC delta encoding does not narrow these provenance inputs.
    parts.update({'source/' + path.name: path.read_bytes()
                  for path in Path(__file__).parent.glob('coding_eval*.py')})
    parts['native-gate.py'] = Path(gate.__file__).read_bytes()
    return parts


def method_record(raw):
    value = json.loads(raw)
    counts = {'tests', 'failures', 'errors', 'skipped', 'expected_failures', 'unexpected_successes'}
    if (type(value) is not dict or value.keys() != {
            'tests', 'calls', 'protocol_failed', 'adapter_policy_failed', 'candidate_exception_failed',
            'candidate_ready', 'function_found', 'methods', 'private_fs_sealed', 'proc_windows',
            'proc_window_open', 'module_global_continuity', 'fixture_assertion_failed'}
            or type(value['tests']) is not dict or value['tests'].keys() != counts
            or any(type(item) is not int or item < 0 for item in value['tests'].values())
            or value['tests']['tests'] != 1
            or type(value['calls']) is not int or not 0 < value['calls'] <= 4096
            or any(type(value[key]) is not bool for key in (
                'protocol_failed', 'adapter_policy_failed', 'candidate_exception_failed',
                'candidate_ready', 'function_found', 'private_fs_sealed', 'proc_window_open',
                'module_global_continuity', 'fixture_assertion_failed'))
            or value['private_fs_sealed'] is not True or value['proc_window_open'] is not False
            or value['module_global_continuity'] is not False
            or type(value['proc_windows']) is not int or value['proc_windows'] < 1
            or type(value['methods']) is not list or len(value['methods']) != 1):
        raise evaluation.EvaluationError('worker-oracle.invalid-result')
    return value


def grade_method(task, files, trusted, hidden, method, output, parts, timeout):
    """One method, with all its original setup, body, assertions and cleanup."""
    evaluation.new_directory(output)
    started = time.monotonic()
    artifacts = {}
    with engine.case_workspace(output), scratch.scratch_mount(output) as mount:
        root = mount.root
        for name in ('trusted', 'candidate', 'candidate-runner', 'oracle-adapter', 'results'):
            (root / name).mkdir(mode=0o700)
        for name, tree in (('trusted', trusted), ('candidate', files)):
            for path, content in tree.items():
                evaluation.write_file(root / name / path, content)
        native = parts['native-gate.py']
        evaluation.write_file(root / 'candidate-runner/gate.py', native)
        evaluation.write_file(root / 'candidate-runner/worker.py', engine.limited_source(parts['worker.py']))
        evaluation.write_file(root / 'candidate-runner/graph.py', parts['graph.py'])
        evaluation.write_file(root / 'candidate-runner/bootstrap.py', BOOTSTRAP)
        for name in ('graph.py', 'sealed.py', 'facade.py'):
            evaluation.write_file(root / 'oracle-adapter' / name, parts[name])
        evaluation.write_file(root / 'oracle-adapter/gate.py', engine.limited_source(parts['runtime.py']))
        evaluation.write_file(root / 'oracle-adapter/test.py', hidden)
        evaluation.publish(root / 'oracle-adapter/config.json', {
            'test_names': [method], 'expected_tests': 1, 'source': SOURCE})
        arguments = (root / 'candidate', root / 'candidate-runner', root / 'results', 'candidate')
        selected = engine.sandbox(*arguments)
        refused = selected == ['/usr/bin/false']
        if not refused and selected != engine.STANDARD_SANDBOX(*arguments):
            raise evaluation.EvaluationError('worker-oracle.factory-refused')
        remaining = timeout - (time.monotonic() - started)
        if remaining <= 0:
            raise TimeoutError('worker-oracle.case-timeout')
        mount.used = True
        mount.control.settimeout(remaining)
        mount.control.send(gate.encoded({'mode': 'independent-gate', 'startup_refused': refused}))
        outcome = gate.decoded(mount.control.recv(gate.MAX_BYTES + 1))
        if (type(outcome) is not list or len(outcome) != 4
                or outcome[1] is not None or any(type(value) is not bool for value in outcome[2:])
                or (outcome[0] is not None and type(outcome[0]) is not int)):
            raise evaluation.EvaluationError('worker-oracle.invalid-outcome')
        for name, limit in (('method-000-tests.json', 16384), ('method-000-details.json', evaluation.MAX_FILE),
                            ('method-000-wire.log', TRANSCRIPT_BYTES)):
            path = root / 'results' / name
            if path.exists():
                artifacts[name] = evaluation.read_regular(path, limit)
    # Result promotion follows holder termination and namespace cleanup.
    for previous, current in (('stdout.log', 'candidate.log'), ('stderr.log', 'oracle.log')):
        if (output / previous).exists():
            (output / previous).rename(output / current)
    for name, content in artifacts.items():
        evaluation.write_file(output / name, content)
    record = method_record(artifacts['method-000-tests.json']) if artifacts.get('method-000-tests.json') else None
    return {'method': method, 'outcome': outcome, 'record': record,
            'logs': {name: evaluation.sha(evaluation.read_regular(output / name, engine.LOG_LIMIT))
                     for name in ('candidate.log', 'oracle.log') if (output / name).exists()},
            'artifacts': {name: evaluation.sha(content) for name, content in artifacts.items()}}


def grade_case(task, files, output, timeout, parts=None):
    if (task['source'] != SOURCE or type(task.get('test_names')) is not list
            or len(task['test_names']) != task['expected_tests']
            or not 1 <= task['expected_tests'] <= MAX_METHODS or timeout != evaluation.TIMEOUT):
        raise evaluation.EvaluationError('worker-oracle.task-refused')
    hidden = evaluation.pinned_blob(task['reference'], task['hidden_test'], task['hidden_sha256'])
    trusted = evaluation.projection(task)
    parts = source_parts() if parts is None else parts
    started = time.monotonic()
    rows = []
    for index, method in enumerate(task['test_names']):
        remaining = timeout - (time.monotonic() - started)
        row = grade_method(task, files, trusted, hidden, method, output / f'method-{index:03d}', parts, remaining)
        rows.append(row)
        if row['record'] is None or row['outcome'][2] or row['outcome'][3]:
            break
    evaluation.publish(output / 'methods.json', rows)
    # Combined diagnostic basenames preserve the supervising evaluator's
    # artifact contract. Individual complete logs also remain hash-bound.
    for name, limit in (('candidate.log', engine.LOG_LIMIT), ('oracle.log', engine.LOG_LIMIT),
                        ('wire.log', TRANSCRIPT_BYTES)):
        with (output / name).open('xb') as target:
            size = 0
            for index, row in enumerate(rows):
                filename = 'method-000-wire.log' if name == 'wire.log' else name
                path = output / f'method-{index:03d}' / filename
                if not path.exists():
                    continue
                content = evaluation.read_regular(path, limit)
                size += len(content)
                if size > limit:
                    raise evaluation.EvaluationError('worker-oracle.aggregate-log-limit')
                target.write(content)
    timed_out = any(row['outcome'][2] for row in rows) or time.monotonic() - started > timeout
    limited = any(row['outcome'][3] for row in rows)
    if len(rows) != task['expected_tests'] or any(row['record'] is None for row in rows):
        return None, None, timed_out, limited, None
    counts = {key: sum(row['record']['tests'][key] for row in rows) for key in rows[0]['record']['tests']}
    record = {
        'tests': counts, 'calls': sum(row['record']['calls'] for row in rows),
        'protocol_failed': any(row['record']['protocol_failed'] for row in rows),
        'adapter_policy_failed': any(row['record']['adapter_policy_failed'] for row in rows),
        'candidate_exception_failed': any(row['record']['candidate_exception_failed'] for row in rows),
        'fixture_assertion_failed': any(row['record']['fixture_assertion_failed'] for row in rows),
        'candidate_ready': all(row['record']['candidate_ready'] for row in rows),
        'function_found': all(row['record']['function_found'] for row in rows),
    }
    evaluation.publish(output / 'tests.json', record)
    code = int(any(row['outcome'][0] != 0 for row in rows))
    return code, None, timed_out, limited, record


def coverage():
    return {
        'graded': ['complete-original-selected-methods', 'in-method-retained-mock-scope',
                   'in-method-message-root-identity', 'native-subprocess-results',
                   'list-derived-invalid-input-category', 'closed-builtin-type-literals'],
        'adapter_policy': ['closed-typed-values', 'bounded-RPC', 'private-oracle-filesystem',
                           'immutable-verdict-objects-and-mock-assertion-methods'],
        'not_graded': ['cross-method-module-and-class-state', 'fixture-class-introspection',
                       'arbitrary-Python-object-identity', 'custom-list-subclass-behavior',
                       'unselected-statement-clauses', 'candidate-writable-temporary-fixtures'],
        'unsupported': ['fixture-class-inheritance', 'mock-class-specifications-and-delegates'],
    }


def historical_kind(task, files, held_out=None, workspace_roots=()):
    if held_out is None:
        return evaluation.require_historical_candidate(task, files)
    if __package__:
        from . import coding_eval_suite as suite
    else:
        import coding_eval_suite as suite
    descriptor = Path(held_out)
    registered = suite.held_out_tasks(descriptor, workspace_roots)
    if task not in registered:
        raise evaluation.InputIntegrityError('worker-oracle.task-definition-refused')
    expected = evaluation.projection(task)
    if files == expected:
        return 'frozen-base'
    expected[task['source']] = evaluation.pinned_blob(task['reference'], task['source'], task['reference_sha256'])
    if files == expected:
        return 'frozen-reference'
    raise evaluation.EvaluationError('evaluation.untrusted-candidate-disabled')


def grade(task, files, output, *, held_out=None, workspace_roots=()):
    kind = historical_kind(task, files, held_out, workspace_roots)
    return _grade(task, files, output, kind, False)


def grade_observations(task, files, output, *, candidate_kind, required_resources):
    """Trusted admission caller validates the private task and projection first."""
    if candidate_kind != 'generated' or required_resources is not True:
        raise evaluation.EvaluationError('worker-oracle.generated-policy-refused')
    return _grade(task, files, output, candidate_kind, required_resources)


def _grade(task, files, output, kind, required_resources):
    evaluation.new_directory(output)
    started = time.monotonic()
    error = None
    input_error = None
    parts = source_parts()
    try:
        with evaluation.coding_eval_resources.measurement(required=required_resources) as resources:
            code, candidate_code, timed_out, limited, record = grade_case(task, files, output, evaluation.TIMEOUT, parts)
    except (OSError, ValueError, subprocess.SubprocessError) as exception:
        error = evaluation.refusal_detail(exception)
        if isinstance(exception, evaluation.InputIntegrityError):
            input_error = error
        code, candidate_code, timed_out, limited, record = (
            None, None, isinstance(exception, (TimeoutError, subprocess.TimeoutExpired)), False, None)
    tests = record['tests'] if record else None
    passed = (error is None and code == 0 and not timed_out and not limited and record is not None
              and record['candidate_ready'] and record['function_found']
              and not any(record[key] for key in ('protocol_failed', 'adapter_policy_failed', 'candidate_exception_failed'))
              and not record['fixture_assertion_failed']
              and tests['tests'] == task['expected_tests']
              and all(value == 0 for key, value in tests.items() if key != 'tests'))
    cause = ('none' if passed else 'input_integrity' if input_error else 'timeout' if timed_out else 'output_limit' if limited else
             'infrastructure' if error or record is None or not record['candidate_ready'] else
             'protocol' if record['protocol_failed'] else
             'adapter_policy' if record['adapter_policy_failed'] else
             'candidate_exception' if record['candidate_exception_failed'] else 'test_failure')
    report = {
        'version': VERSION, 'task': task['id'], 'passed': passed, 'cause': cause,
        'exit_code': code, 'candidate_exit_code': candidate_code,
        'candidate_exit_scope': 'no-single-candidate-process', 'tests': tests,
        'calls': record['calls'] if record else None, 'calls_scope': 'outer-worker-API-RPC',
        **{key: record[key] if record else None for key in ('protocol_failed', 'adapter_policy_failed',
            'candidate_exception_failed', 'candidate_ready', 'function_found', 'fixture_assertion_failed')},
        'error': error, 'input_error': input_error,
        'candidate_kind': kind, 'candidate_sha256': evaluation.sha(files[task['source']]),
        'hidden_sha256': task['hidden_sha256'], 'test_names': task['test_names'],
        'task_sha256': evaluation.sha(json.dumps(task, sort_keys=True, separators=(',', ':')).encode()),
        'source_hashes': {path: evaluation.sha(content) for path, content in parts.items()},
        'oracle_sha256': evaluation.sha(parts['source/' + Path(__file__).name]),
        'gate_sha256': evaluation.sha(parts['native-gate.py']),
        'evaluator_sha256': evaluation.sha(parts['source/' + Path(evaluation.__file__).name]),
        'scratch_sha256': evaluation.sha(parts['source/' + Path(scratch.__file__).name]),
        'resource_adapter_sha256': evaluation.sha(parts['source/' + Path(evaluation.coding_eval_resources.__file__).name]),
        'bootstrap_sha256': evaluation.sha(BOOTSTRAP),
        'materialized_client_sha256': evaluation.sha(engine.limited_source(parts['worker.py'])),
        'materialized_runtime_sha256': evaluation.sha(engine.limited_source(parts['runtime.py'])),
        'inherited_lane_resources': resources, 'wall_seconds': round(time.monotonic() - started, 6),
        'output_truncated': limited, 'coverage': coverage(), 'aggregate_worker_quotas': False,
        'worker_limits': {'processes': engine.WORKER_PROCESSES, 'address_bytes': 512 * evaluation.MAX_FILE,
                          'cpu_seconds': 60, 'task_wall_seconds': evaluation.TIMEOUT,
                          'frame_bytes': evaluation.MAX_FILE, 'transcript_bytes': TRANSCRIPT_BYTES},
        'model_attempts': 0, 'model_score': None, 'tokens': None, 'gpu_minutes': 0,
        'score_scope': 'independent-generated-observations' if required_resources else 'independent-worker-controls-only',
        'required_inherited_lane_resources': required_resources,
        'control_attempts': {'total': int(not required_resources), 'passed': int(passed and not required_resources),
                             'failed': int(not passed and not required_resources)},
        'candidate_attempts': {'total': int(required_resources), 'passed': int(passed and required_resources),
                               'failed': int(not passed and required_resources)},
        'logs': {name: evaluation.sha(evaluation.read_regular(output / name, limit))
                 for name, limit in (('candidate.log', engine.LOG_LIMIT), ('oracle.log', engine.LOG_LIMIT),
                                     ('wire.log', TRANSCRIPT_BYTES), ('methods.json', 4 * evaluation.MAX_FILE))
                 if (output / name).exists()},
    }
    evaluation.publish(output / 'report.json', report)
    return report
