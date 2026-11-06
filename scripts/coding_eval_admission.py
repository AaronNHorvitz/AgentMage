"""Score complete frozen sets through independent assertions with required quotas.

A matching successful control receipt is required. Legacy in-process graders
remain historical-controls-only. No model is launched by this command.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

if __package__:
    from . import coding_eval as evaluation
    from . import coding_eval_oracle as oracle
    from . import coding_eval_suite as suite
    from . import coding_eval_worker_oracle as worker_oracle
else:
    import coding_eval as evaluation
    import coding_eval_oracle as oracle
    import coding_eval_suite as suite
    import coding_eval_worker_oracle as worker_oracle


VERSION = 'coding-independent-admission-v1'
POLICY = {
    'score': 'complete-original-assertions-over-declared-finite-observations',
    'candidate_scope': 'one-source-file-in-an-exact-frozen-projection',
    'required_resources': 'verified-entire-inherited-lane-quotas',
    'candidate_only_resource_peaks': False,
    'adapter_policy_result': 'ungradable-attempt-included-in-denominator',
    'excluded_attempts': 0,
    'automatic_retries': 0,
    'whole_statement_correctness': False,
    'limitations': ['unasserted-statement-clauses', 'candidate-self-reported-observation-fidelity',
                    'deliberate-candidate-self-modification', 'adapter-specific-identity-and-state-limits'],
}


def source_hashes():
    return suite.source_hashes()


def control_receipt(path, workspace_roots=()):
    raw = suite.private_bytes(path, workspace_roots)
    value = json.loads(raw, object_pairs_hook=suite.unique_object)
    expected = {
        'version': suite.VERSION, 'kind': 'complete-suite-historical-controls-only',
        'passed': True, 'development_tasks': 30, 'held_out_tasks': 10,
        'independent_development_tasks': 30, 'independent_held_out_tasks': 10,
        'development_controls_valid': True, 'independent_development_controls_valid': True,
        'held_out_controls_valid': True, 'independent_held_out_controls_valid': True,
        'development_sha256': suite.DEVELOPMENT_SHA256, 'held_out_sha256': suite.HELD_OUT_SHA256,
        'control_attempts': 160, 'input_refused': False, 'source_hashes': source_hashes(),
        'held_out_methods': 52, 'held_out_test_methods_run': 104,
        'independent_held_out_test_methods_run': 104, 'source_changed': False,
        'generated_scoring': False,
    }
    if type(value) is not dict or any(type(value.get(key)) is not type(item) or value[key] != item
                                      for key, item in expected.items()):
        suite.refuse('control-receipt-refused')
    # Source matches alone cannot validate a partial or substituted receipt.
    logs = value.get('logs')
    names = {'development-coverage.json', 'development/summary.json',
             'independent-development/summary.json', 'held-out/controls.json',
             'independent-held-out/controls.json'}
    if type(logs) is not dict or logs.keys() != names:
        suite.refuse('control-receipt-refused')
    for name, digest in logs.items():
        log_raw = suite.private_bytes(Path(path).parent / name, workspace_roots)
        if type(digest) is not str or evaluation.sha(log_raw) != digest:
            suite.refuse('control-receipt-refused')
    return evaluation.sha(raw)


def score(candidates, output, receipt, *, split='development', held_out=None,
          purpose=None, workspace_roots=()):
    if split not in ('development', 'held-out') or (split == 'held-out' and
            (purpose != 'row-close-scoreboard' or held_out is None)):
        suite.refuse('scoring-purpose-refused')
    suite.private_location(output, workspace_roots)
    sources = source_hashes()
    receipt_sha256 = control_receipt(receipt, workspace_roots)
    tasks = suite.development_tasks() if split == 'development' else suite.held_out_tasks(held_out, workspace_roots)
    # Hard inherited quotas must be available before any candidate launches.
    with evaluation.coding_eval_resources.measurement(required=True) as resources:
        evaluation.new_directory(output)
        started = time.monotonic()
        rows = []
        for task in tasks:
            task_output = output / task['id']
            try:
                files = evaluation.read_candidate(Path(candidates) / task['id'], evaluation.projection(task), task['source'])
                engine = worker_oracle if split == 'held-out' else oracle
                report = engine.grade_observations(task, files, task_output,
                                                   candidate_kind='generated', required_resources=True)
                policy = report.get('adapter_policy_failed') is True
                status = 'pass' if report['passed'] else 'ungradable' if policy else 'fail'
                row = {'task': task['id'], 'passed': report['passed'], 'status': status,
                       'cause': report['cause'], 'report_sha256': evaluation.sha((task_output / 'report.json').read_bytes())}
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                # Missing/invalid snapshots and resource/setup failures remain
                # attempts. They never validate a task or reduce the denominator.
                row = {'task': task['id'], 'passed': False, 'status': 'ungradable',
                       'cause': 'input-or-infrastructure-refusal', 'error': evaluation.refusal_detail(error)}
            rows.append(row)
        evaluation.publish(output / 'attempts.json', {'tasks': rows})
    if source_hashes() != sources:
        suite.refuse('scoring-source-changed')
    summary = {'version': VERSION, 'split': split, 'kind': 'generated-candidate-observations',
               'tasks': len(tasks), 'passed_tasks': sum(row['passed'] for row in rows),
               'ungradable_attempts': sum(row['status'] == 'ungradable' for row in rows),
               'attempts': len(rows), 'excluded_attempts': 0,
               'policy': POLICY, 'control_receipt_sha256': receipt_sha256,
               'source_hashes': sources, 'inherited_lane_resources': resources,
               'wall_seconds': round(time.monotonic() - started, 6),
               'attempts_sha256': evaluation.sha((output / 'attempts.json').read_bytes()),
               'development_sha256': suite.DEVELOPMENT_SHA256, 'held_out_sha256': suite.HELD_OUT_SHA256,
               'model_attempts': 0, 'model_score': None, 'gpu_minutes': 0, 'tokens': None}
    evaluation.publish(output / 'summary.json', summary)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidates', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--controls', type=Path, required=True)
    parser.add_argument('--split', choices=('development', 'held-out'), default='development')
    parser.add_argument('--held-out', type=Path)
    parser.add_argument('--purpose', choices=('row-close-scoreboard',))
    parser.add_argument('--workspace-root', type=Path, action='append', default=[])
    args = parser.parse_args()
    try:
        result = score(args.candidates, args.output, args.controls, split=args.split, held_out=args.held_out,
                       purpose=args.purpose, workspace_roots=args.workspace_root)
        print(json.dumps(result, sort_keys=True))
        return 0
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print('admission.refused ' + evaluation.refusal_detail(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
