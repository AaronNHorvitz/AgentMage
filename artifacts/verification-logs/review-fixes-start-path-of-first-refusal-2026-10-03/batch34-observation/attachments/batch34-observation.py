from pathlib import Path
import hashlib, json, os, subprocess, sys
from datetime import datetime, timezone

repo = Path.cwd()
started_at_utc = datetime.now(timezone.utc).isoformat()
assert not subprocess.check_output(['git', 'status', '--porcelain'])
out = Path(os.environ.get('BATCH34_OBSERVATION_OUT', '<state>/logs/batch34-observation-20261003'))
assert not out.exists() and not out.is_symlink()
out.mkdir(mode=0o700)


def guard():
    p = Path('<control>/STOP')
    if p.exists() or p.is_symlink():
        raise SystemExit(125)


def digest(p):
    h = hashlib.sha256()
    with p.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def run(name, args, cwd=repo, stdin_bytes=None):
    guard()
    try:
        r = subprocess.run(args, cwd=cwd, input=stdin_bytes,
                           stdin=None if stdin_bytes is not None else subprocess.DEVNULL,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60, check=False)
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout or b''
        stderr = error.stderr or b''
        (out / (name + '.stdout')).write_bytes(stdout)
        (out / (name + '.stderr')).write_bytes(stderr)
        record = {'argv': args, 'exit_code': None, 'timed_out': True, 'timeout_seconds': 60,
                  'stdout_sha256': hashlib.sha256(stdout).hexdigest(),
                  'stderr_sha256': hashlib.sha256(stderr).hexdigest(),
                  'owned_descendant_cleanup': 'not-established-by-this-observation'}
        (out / (name + '.json')).write_text(json.dumps(record, indent=2) + '\n')
        raise
    (out / (name + '.stdout')).write_bytes(r.stdout)
    (out / (name + '.stderr')).write_bytes(r.stderr)
    record = {'argv': args,
              'stdin_sha256': hashlib.sha256(stdin_bytes or b'').hexdigest() if stdin_bytes is not None else None,
              'exit_code': r.returncode, 'stdout_sha256': hashlib.sha256(r.stdout).hexdigest(),
              'stderr_sha256': hashlib.sha256(r.stderr).hexdigest()}
    (out / (name + '.json')).write_text(json.dumps(record, indent=2) + '\n')
    print(name, 'exit', r.returncode, flush=True)
    return r


def fresh_root():
    for i in range(3, 400):
        p = Path('<state>') / ('c' + str(i))
        if not p.exists() and not p.is_symlink():
            return p
    raise RuntimeError('No fresh short root')


def setup(name):
    root = fresh_root()
    r = run(name, [sys.executable, '-m', 'scripts.coding_harness', 'setup', '--root', str(root)])
    assert r.returncode == 0
    return root


def snapshot(root):
    workspace = root / 'disposable/worktree'
    git = lambda *args: subprocess.check_output(['/usr/bin/git', '-C', str(workspace), *args])
    state = {'head': git('rev-parse', 'HEAD').decode().strip(),
             'status_hex': git('status', '--porcelain=v1', '-z', '--untracked-files=all').hex(),
             'index_entries_hex': git('ls-files', '--stage', '-z').hex(),
             'unstaged_diff_sha256': hashlib.sha256(git('diff', '--binary')).hexdigest(),
             'staged_diff_sha256': hashlib.sha256(git('diff', '--cached', '--binary')).hexdigest(),
             'index_sha256': digest(workspace / '.git/index')}
    state['files'] = [{'path': p.relative_to(workspace).as_posix(), 'sha256': digest(p),
                       'mode': p.stat().st_mode & 0o777}
                      for p in sorted(workspace.rglob('*'))
                      if p.is_file() and '.git' not in p.relative_to(workspace).parts]
    return state


def tree(directory):
    return {p.relative_to(directory).as_posix(): (digest(p) if p.is_file() and not p.is_symlink() else 'link')
            for p in sorted(directory.rglob('*')) if not p.is_dir() or p.is_symlink()}


def rows(text):
    return [json.loads(line) for line in text.splitlines() if line.strip()]


OBJECTIVE = 'Repair the failing synthetic add test and rerun validation.'
binaries = [repo / 'target/debug' / name for name in ['agentmage', 'agentmage-host', 'agentmage-read-only-worker']]
before_binaries = {p.name: {'sha256': digest(p), 'bytes': p.stat().st_size} for p in binaries}

run('cli-help', [str(binaries[0]), '--help'])
help_text = (out / 'cli-help.stdout').read_text()
for option in ('--recipe ABSOLUTE_FILE', '--doc-pack-search', '--memory-list', '--extension-list', '--ended-run RUN_ID'):
    assert option in help_text, option


def harness(name, root, *arguments):
    return run(name, [sys.executable, '-m', 'scripts.coding_harness', *arguments, '--root', str(root)])


def start(name, root, *extra, approve=True):
    arguments = [sys.executable, '-m', 'scripts.coding_harness', 'start', '--root', str(root),
                 '--scenario', 'failed-test-repair', '--model', 'scripted', '--objective', OBJECTIVE,
                 '--log-dir', str(out / name)]
    if approve:
        arguments.append('--approve-this-run')
    return run(name, arguments + list(extra))


def stderr_rows(name):
    parsed = []
    for line in (out / name / 'stderr.log').read_text().splitlines():
        try:
            parsed.append(json.loads(line))
        except json.JSONDecodeError:
            parsed.append(line)
    return parsed


NATIVE_REFUSAL = 'Error: linux.repository.git_artifact.invalid'
ENVELOPE_FAILURE = 'linux.development.launch-envelope.failed'
WIRE_REFUSALS = (b'host.runtime.evidence_denied', b'host.runtime.request_denied')
crossed = []


def catalog_call(name, root, *arguments, expect=0):
    """One CLI and catalog host process pair; every frame between them is
    decoded exactly (Decision 0135), so a wire refusal would show here."""
    result = harness(name, root, *arguments)
    assert result.returncode == expect, (name, result.returncode, result.stderr[-400:])
    for code in WIRE_REFUSALS:
        assert code not in result.stdout and code not in result.stderr, (name, code)
    crossed.append({'name': name, 'operation': arguments[0], 'exit_code': result.returncode})
    return result


# 1. The coding host is still refused at native Git trust before any run, and
# its workspace is preserved.
clean = setup('clean-setup')
clean_before = snapshot(clean)
clean_start = start('clean-start', clean)
assert clean_start.returncode == 5
clean_lines = (out / 'clean-start/stderr.log').read_text().splitlines()
assert clean_lines[0] == NATIVE_REFUSAL and clean_lines[-1] == ENVELOPE_FAILURE
assert snapshot(clean) == clean_before

# 2. Every catalog host operation the CLI has, through actual processes:
# documentation packs, memory, extensions with revocation lists and the
# ended-run read. Each answer below was decoded exactly by the CLI, and each
# request exactly by the host.
catalog = setup('catalog-setup')
catalog_before = snapshot(catalog)
pack = fresh_root()
assert run('doc-pack-sample', [sys.executable, '-m', 'scripts.coding_harness', 'doc-pack-sample',
                                '--directory', str(pack), '--version', '1.0.0',
                                '--retrieved-on', '2026-10-01']).returncode == 0
pack_tree = tree(pack)
assert rows(catalog_call('doc-pack-list-empty', catalog, 'doc-pack', '--list').stdout.decode()) == [
    {'packs': 0, 'type': 'doc_pack_list'}]
imported = catalog_call('doc-pack-import', catalog, 'doc-pack', '--import', str(pack),
                        '--allow-license', 'LicenseRef-agentmage-sample')
receipt = rows(imported.stdout.decode())[-1]['receipt']
assert receipt['network_used'] is False and receipt['file_count'] == 3
refused_license = catalog_call('doc-pack-import-other-license', catalog, 'doc-pack', '--import', str(pack),
                               '--allow-license', 'MIT', expect=4)
assert rows(refused_license.stderr.decode()) == [{'code': 'doc-pack.license-not-allowed', 'type': 'doc_pack_refused'}]
searched = catalog_call('doc-pack-search', catalog, 'doc-pack', '--search', 'remote cache')
hits = [row['hit'] for row in rows(searched.stdout.decode()) if row['type'] == 'doc_pack_hit']
assert hits and all(hit['path'] == 'guide/cache.md' for hit in hits)
inspected = catalog_call('doc-pack-inspect', catalog, 'doc-pack', '--inspect', 'agentmage-sample-guide')
assert rows(inspected.stdout.decode())[-1]['status']['current'] == {'major': 1, 'minor': 0, 'patch': 0}

CITE = 'agentmage-sample-guide@1.0.0:'
remembered = catalog_call('memory-remember', catalog, 'memory', '--remember',
                          'Clear the build cache when the toolchain changes.', '--workspace', 'calculator',
                          '--cite', CITE + 'guide/cache.md')
item = rows(remembered.stdout.decode())[0]['item']
assert item['status'] == 'approved' and item['workspace_id'] == 'calculator'
prohibited = catalog_call('memory-remember-credential', catalog, 'memory', '--remember',
                          'The deploy password=hunter2 is shared.', '--workspace', 'calculator',
                          '--cite', CITE + 'guide/cache.md', expect=4)
assert [row['code'] for row in rows(prohibited.stderr.decode())] == ['memory.candidate.prohibited']
listed = catalog_call('memory-list', catalog, 'memory', '--list')
assert rows(listed.stdout.decode())[-1] == {'catalog_revision': 1, 'items': 1, 'type': 'memory_list'}
revoked = catalog_call('memory-revoke', catalog, 'memory', '--revoke', item['memory_id'])
assert rows(revoked.stdout.decode())[0]['type'] == 'memory_revoked'
deleted = catalog_call('memory-delete', catalog, 'memory', '--delete', item['memory_id'])
assert rows(deleted.stdout.decode())[0]['type'] == 'memory_deleted'

sample = fresh_root()
copied = run('extension-sample', [sys.executable, '-m', 'scripts.coding_harness', 'extension-sample',
                                  '--directory', str(sample)])
assert copied.returncode == 0
sample_tree = tree(sample)
for statement in ('signer', 'issuer'):
    catalog_call(f'extension-trust-{statement}', catalog, 'extension', '--trust',
                 str(sample / f'{statement}-trust.json'), '--workspace', 'calculator')
catalog_call('extension-install', catalog, 'extension', '--install', str(sample / 'packages/sample-formatter'),
             '--allow-license', 'LicenseRef-agentmage-sample', '--workspace', 'calculator')
first = catalog_call('extension-revocations-1', catalog, 'extension', '--revocations',
                     str(sample / 'revocations/1.json'), '--workspace', 'calculator')
first_answer = rows(first.stdout.decode())[0]['answer']
assert first_answer['answer'] == 'revocations-applied'
assert first_answer['revocations']['issuer_id'] == 'agentmage-sample-issuer'
renamed = catalog_call('extension-revocations-renamed', catalog, 'extension', '--revocations',
                       str(sample / 'revocations/renamed.json'), '--workspace', 'calculator', expect=3)
assert [row['code'] for row in rows(renamed.stderr.decode())] == ['extension.revocations-untrusted']
extensions = rows(catalog_call('extension-list', catalog, 'extension', '--list', '--workspace',
                               'calculator').stdout.decode())
assert extensions[0]['scope']['revocations'] == first_answer['revocations']
assert extensions[-1] == {'catalog_revision': 4, 'scopes': 1, 'type': 'extension_list'}

# Route grants (Decision 0144): a grant is shown and kept only after a typed
# yes; any other answer sends nothing; a second live grant of a route and a
# second revocation are refused.
grants_dir = fresh_root()
grants_dir.mkdir(mode=0o700)
GRANT_WORKSPACE = 'coding-development-0123456789abcdef01234567'


def grant_file(name, grant_id):
    path = grants_dir / name
    path.write_text(json.dumps({
        'schema_version': 1, 'grant_id': grant_id, 'route_id': 'example-remote-route',
        'candidate_sha256': 'd' * 64, 'provider_id': 'example-provider',
        'data_classes': ['conversation'], 'max_requests': 5, 'max_input_tokens': 100000,
        'fallback_allowed': False, 'valid_for_hours': 24}))
    path.chmod(0o600)
    return path


first_grant = grant_file('grant-1.json', 'grant-example')
second_grant = grant_file('grant-2.json', 'grant-second')
grant_runs = []


def grant_call(name, *arguments, stdin=None, expect=0, crosses=True):
    result = run(name, [sys.executable, '-m', 'scripts.coding_harness', 'route-grant', *arguments,
                        '--root', str(catalog)], stdin_bytes=stdin)
    assert result.returncode == expect, (name, result.returncode, result.stderr[-400:])
    for code in WIRE_REFUSALS:
        assert code not in result.stdout and code not in result.stderr, (name, code)
    if crosses:
        crossed.append({'name': name, 'operation': 'route-grant', 'exit_code': result.returncode})
    grant_runs.append({'name': name, 'exit_code': result.returncode})
    return result


declined = grant_call('route-grant-declined', '--grant', str(first_grant), '--workspace', GRANT_WORKSPACE,
                      stdin=b'no\n', expect=6, crosses=False)
preview_row, _ = declined.stderr.decode().split('\n', 1)
preview_row = json.loads(preview_row)
assert preview_row['type'] == 'route_grant_preview' and preview_row['grant']['grant_id'] == 'grant-example'
assert preview_row['remote_route_offered'] is False
assert declined.stderr.decode().endswith('Type yes to keep this grant: {"type":"route_grant","result":"declined"}\n')
granted = grant_call('route-grant-grant', '--grant', str(first_grant), '--workspace', GRANT_WORKSPACE,
                     stdin=b'yes\n')
granted_row = rows(granted.stdout.decode())[0]
assert granted_row['result'] == 'granted' and granted_row['grant']['state'] == 'live'
assert granted_row['receipt']['catalog_revision'] == 1
again = grant_call('route-grant-same-route', '--grant', str(second_grant), '--workspace', GRANT_WORKSPACE,
                   stdin=b'yes\n', expect=4)
assert rows(again.stderr.decode().split('Type yes to keep this grant: ')[-1])[-1]['refusal'] == \
    'route-grant.route-already-granted'
listed_grants = rows(grant_call('route-grant-list', '--list', '--workspace', GRANT_WORKSPACE).stdout.decode())
assert [grant['state'] for grant in listed_grants[0]['grants']] == ['live']
revoked_grant = rows(grant_call('route-grant-revoke', '--revoke', 'grant-example', '--workspace',
                                GRANT_WORKSPACE).stdout.decode())[0]
assert revoked_grant['result'] == 'revoked' and revoked_grant['grant']['state'] == 'revoked'
twice = grant_call('route-grant-revoke-again', '--revoke', 'grant-example', '--workspace', GRANT_WORKSPACE,
                   expect=4)
assert rows(twice.stderr.decode())[-1]['refusal'] == 'route-grant.already-revoked'
final_grants = rows(grant_call('route-grant-list-all', '--list').stdout.decode())[0]
assert [grant['state'] for grant in final_grants['grants']] == ['revoked']
assert final_grants['catalog_revision'] == 2

ended = catalog_call('ended-run-read', catalog, 'ended-run', '--run', 'coding-development-run-0123456789abcdef')
assert rows(ended.stdout.decode()) == [{'available': True, 'effects': None, 'job_control': None, 'routes': None,
                                        'run_id': 'coding-development-run-0123456789abcdef',
                                        'type': 'ended_run_action_histories'}]
assert tree(pack) == pack_tree and tree(sample) == sample_tree
assert snapshot(catalog) == catalog_before

# 3. A recipe is still read, checked and shown before the coding host is
# launched; the host is then refused at native Git trust.
recipes = fresh_root()
assert run('recipe-sample', [sys.executable, '-m', 'scripts.coding_harness', 'recipe-sample',
                             '--directory', str(recipes)]).returncode == 0
accepted = setup('recipe-accepted-setup')
accepted_before = snapshot(accepted)
accepted_start = start('recipe-accepted', accepted, '--recipe', str(recipes / 'repair-in-src.json'),
                       '--recipe-param', 'target=src/calc.py', '--recipe-param', 'attempts=2', approve=False)
assert accepted_start.returncode == 5
accepted_rows = stderr_rows('recipe-accepted')
assert accepted_rows[0]['type'] == 'recipe_requested'
assert accepted_rows[0]['recipe_id'] == 'agentmage-sample-repair'
assert accepted_rows[1] == NATIVE_REFUSAL and accepted_rows[-1] == ENVELOPE_FAILURE
assert snapshot(accepted) == accepted_before

# 4. Pre-existing work is refused before launch and every byte kept.
dirty = setup('dirty-setup')
workspace = dirty / 'disposable/worktree'
note = workspace / 'operator-note.txt'
note.write_text('Synthetic pre-existing staged work.\n')
note.chmod(0o600)
subprocess.run(['/usr/bin/git', '-C', str(workspace), 'add', '--', 'operator-note.txt'], check=True)
note.write_text('Synthetic pre-existing staged work.\nAdditional unstaged work.\n')
scratch = workspace / 'operator-scratch.txt'
scratch.write_text('Synthetic pre-existing untracked work.\n')
scratch.chmod(0o600)
dirty_before = snapshot(dirty)
dirty_start = start('coding-dirty', dirty)
assert dirty_start.returncode == 1 and snapshot(dirty) == dirty_before
assert not (out / 'coding-dirty').exists()

# 5. The actual public research worker, started outside its sandbox, writes only
# its closed failure report and exits 5 (Decision 0141). It refuses before any
# network use: an inadmissible environment or argument fails the first check.
worker = repo / 'target/debug/agentmage-public-research-worker'
worker_before = {'sha256': digest(worker), 'bytes': worker.stat().st_size}
from_engine = {
    'research.worker.environment-denied', 'research.worker.input-denied',
    'research.worker.destination-denied', 'research.worker.transport-failed',
    'research.worker.response-denied', 'research.worker.limit', 'research.worker.deadline',
    'research.worker.output-failed'}
worker_runs = []
for name, argv in [
    ('worker-ambient-environment', ['/usr/bin/env', '-i', 'FOO=bar', str(worker)]),
    ('worker-extra-argument', ['/usr/bin/env', '-i', 'LANG=C', 'PWD=/input', str(worker), 'extra']),
    ('worker-admitted-environment', ['/usr/bin/env', '-i', 'LANG=C', 'PWD=/input', str(worker)]),
]:
    r = run(name, argv)
    report = r.stderr.decode('utf-8', errors='replace')
    closed = r.returncode == 5 and not r.stdout and report.endswith('\n') and report.count('\n') == 1 \
        and report[:-1] in from_engine
    worker_runs.append({'name': name, 'exit_code': r.returncode, 'stdout_bytes': len(r.stdout),
                        'report': report[:-1] if closed else None, 'closed_report': closed})
    assert closed, name
assert worker_runs[0]['report'] == 'research.worker.environment-denied'
assert worker_runs[1]['report'] == 'research.worker.environment-denied'
assert worker_before == {'sha256': digest(worker), 'bytes': worker.stat().st_size}

after_binaries = {p.name: {'sha256': digest(p), 'bytes': p.stat().st_size} for p in binaries}
assert before_binaries == after_binaries
report = {
    'started_at_utc': started_at_utc, 'finished_at_utc': datetime.now(timezone.utc).isoformat(),
    'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
    'source_tree': subprocess.check_output(['git', 'rev-parse', 'HEAD^{tree}'], text=True).strip(),
    'source_worktree_clean': not subprocess.check_output(['git', 'status', '--porcelain']),
    'binaries': before_binaries, 'binaries_unchanged': True,
    'clean': {'exit_code': clean_start.returncode, 'preserved': True,
              'refusal_code': 'linux.repository.git_artifact.invalid', 'failure_code': ENVELOPE_FAILURE},
    'catalog_host_operations': crossed,
    'catalog_host_wire_refusals_seen': False,
    'catalog_workspace_preserved': True,
    'extension_renamed_issuer_refused': True,
    'route_grants': {'runs': grant_runs, 'declined_sent_nothing': True, 'remote_route_offered': False},
    'ended_run_chains_present': False,
    'recipe_shown_before_launch': {'exit_code': accepted_start.returncode,
                                   'then_refused_at': 'linux.repository.git_artifact.invalid', 'preserved': True},
    'dirty': {'exit_code': dirty_start.returncode, 'preserved': True, 'log_directory_created': False},
    'malformed_frame_injected': False,
    'research_worker': {'binary': worker_before, 'runs': worker_runs,
                        'sandboxed': False, 'network_attempted': False},
    'network_used': False, 'model': 'scripted', 'real_model_run': False,
    'complete_coding_workflow_observed': False, 'native_tools_executed': False,
    'qualification': 'actual-binary-catalog-host-operations-under-exact-wire-decoding-plus-native-prerequisite-refusal',
}
(out / 'observation.json').write_text(json.dumps(report, indent=2) + '\n')
print('BATCH34_LINUX_OBSERVATION_COMPLETE=1', flush=True)
