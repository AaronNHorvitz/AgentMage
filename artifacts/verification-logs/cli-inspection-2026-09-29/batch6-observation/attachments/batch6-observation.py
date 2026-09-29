from pathlib import Path
import hashlib,json,subprocess,sys
from datetime import datetime,timezone
repo=Path.cwd()
started_at_utc=datetime.now(timezone.utc).isoformat()
assert not subprocess.check_output(['git','status','--porcelain'])
out=Path('<state>/logs/batch6-observation-20260929')
assert not out.exists() and not out.is_symlink()
out.mkdir(mode=0o700)

def guard():
    p=Path('<control>/STOP')
    if p.exists() or p.is_symlink(): raise SystemExit(125)

def digest(p):
    h=hashlib.sha256()
    with p.open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''): h.update(block)
    return h.hexdigest()

def run(name,args,cwd=repo):
    guard()
    try:
        r=subprocess.run(args,cwd=cwd,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=60,check=False)
    except subprocess.TimeoutExpired as error:
        stdout=error.stdout or b''; stderr=error.stderr or b''
        (out/(name+'.stdout')).write_bytes(stdout)
        (out/(name+'.stderr')).write_bytes(stderr)
        record={'argv':args,'exit_code':None,'timed_out':True,'timeout_seconds':60,
            'stdout_sha256':hashlib.sha256(stdout).hexdigest(),'stderr_sha256':hashlib.sha256(stderr).hexdigest(),
            'owned_descendant_cleanup':'not-established-by-this-observation'}
        (out/(name+'.json')).write_text(json.dumps(record,indent=2)+'\n')
        raise
    (out/(name+'.stdout')).write_bytes(r.stdout)
    (out/(name+'.stderr')).write_bytes(r.stderr)
    record={'argv':args,'exit_code':r.returncode,'stdout_sha256':hashlib.sha256(r.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(r.stderr).hexdigest()}
    (out/(name+'.json')).write_text(json.dumps(record,indent=2)+'\n')
    print(name,'exit',r.returncode,flush=True)
    return r

def fresh_root():
    for i in range(3,100):
        p=Path('<state>')/('c'+str(i))
        if not p.exists() and not p.is_symlink(): return p
    raise RuntimeError('No fresh short root')

def setup(name):
    root=fresh_root()
    r=run(name,[sys.executable,'-m','scripts.coding_harness','setup','--root',str(root)])
    assert r.returncode==0
    return root

def snapshot(root):
    workspace=root/'disposable/worktree'
    git=lambda *args:subprocess.check_output(['/usr/bin/git','-C',str(workspace),*args])
    state={'head':git('rev-parse','HEAD').decode().strip(),'status_hex':git('status','--porcelain=v1','-z','--untracked-files=all').hex(),'index_entries_hex':git('ls-files','--stage','-z').hex(),'unstaged_diff_sha256':hashlib.sha256(git('diff','--binary')).hexdigest(),'staged_diff_sha256':hashlib.sha256(git('diff','--cached','--binary')).hexdigest(),'index_sha256':digest(workspace/'.git/index')}
    state['files']=[{'path':p.relative_to(workspace).as_posix(),'sha256':digest(p),'mode':p.stat().st_mode & 0o777} for p in sorted(workspace.rglob('*')) if p.is_file() and '.git' not in p.relative_to(workspace).parts]
    return state

binaries=[repo/'target/debug'/name for name in ['agentmage','agentmage-host','agentmage-read-only-worker']]
before_binaries={p.name:{'sha256':digest(p),'bytes':p.stat().st_size} for p in binaries}
run('cli-help',[str(binaries[0]),'--help'])
clean=setup('clean-setup')
clean_before=snapshot(clean)
clean_diagnose=run('clean-diagnose',[sys.executable,'-m','scripts.coding_harness','diagnose','--root',str(clean)])
clean_start=run('clean-start',[sys.executable,'-m','scripts.coding_harness','start','--root',str(clean),'--scenario','failed-test-repair','--model','scripted','--objective','Repair the failing synthetic add test and rerun validation.','--approve-this-run','--log-dir',str(out/'coding-clean')])
clean_after=snapshot(clean)
clean_status=run('clean-status',[sys.executable,'-m','scripts.coding_harness','status','--root',str(clean)])
for response in (clean_diagnose,clean_status):
    diagnosis=json.loads(response.stdout)
    assert diagnosis['schema_version']==2
    assert diagnosis['scope']=='development-diagnostics-only'
    assert diagnosis['fixture_profile_id']=='scripted-executable-fixture-32k-v1'
    assert 'profile_id' not in diagnosis and 'qualification' not in diagnosis
    assert diagnosis['model_request']=={'selection':None,'observation':'unavailable','serving':'not-observed','qualification':'not-assessed'}
    assert diagnosis['lifecycle']=='ready' and diagnosis['confinement'] is False
clean_stop=run('clean-stop-idle',[sys.executable,'-m','scripts.coding_harness','stop','--root',str(clean)])
assert clean_stop.returncode==1
assert snapshot(clean)==clean_before
assert clean_before==clean_after
assert clean_start.returncode==5
assert (out/'coding-clean/stdout.jsonl').read_bytes()==b''
assert (out/'coding-clean/stderr.log').read_text().splitlines()==[
    'Error: linux.repository.git_artifact.invalid',
    'linux.development.launch-envelope.failed',
]

dirty=setup('dirty-setup')
workspace=dirty/'disposable/worktree'
notes=workspace/'operator-note.txt';notes.write_text('Synthetic pre-existing staged work.\n');notes.chmod(0o600)
subprocess.run(['/usr/bin/git','-C',str(workspace),'add','--','operator-note.txt'],check=True)
notes.write_text('Synthetic pre-existing staged work.\nAdditional unstaged work.\n')
scratch=workspace/'operator-scratch.txt';scratch.write_text('Synthetic pre-existing untracked work.\n');scratch.chmod(0o600)
dirty_before=snapshot(dirty)
dirty_start=run('dirty-start',[sys.executable,'-m','scripts.coding_harness','start','--root',str(dirty),'--scenario','failed-test-repair','--model','scripted','--objective','Repair the failing synthetic add test and rerun validation.','--approve-this-run','--log-dir',str(out/'coding-dirty')])
dirty_after=snapshot(dirty)
assert dirty_before==dirty_after
run('dirty-status',[sys.executable,'-m','scripts.coding_harness','status','--root',str(dirty)])

assert dirty_start.returncode==1
assert not (out/'coding-dirty').exists()
assert not list((clean/'state').rglob('*.*'))
clean_entries=sorted(p.relative_to(clean/'state').as_posix() for p in (clean/'state').rglob('*'))
assert clean_entries==['repository-management']
assert list((clean/'state/repository-management').iterdir())==[]
assert list((dirty/'state').iterdir())==[]
after_binaries={p.name:{'sha256':digest(p),'bytes':p.stat().st_size} for p in binaries}
assert before_binaries==after_binaries
report={
 'started_at_utc':started_at_utc,'finished_at_utc':datetime.now(timezone.utc).isoformat(),
 'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
 'source_tree':subprocess.check_output(['git','rev-parse','HEAD^{tree}'],text=True).strip(),
 'source_worktree_clean':not subprocess.check_output(['git','status','--porcelain']),
 'binaries':before_binaries,'binaries_unchanged':True,
 'clean':{'root':str(clean),'exit_code':clean_start.returncode,'before':clean_before,'after':clean_after,'preserved':True,'state_entries':clean_entries,'state_has_no_files':not any(p.is_file() for p in (clean/'state').rglob('*')),'status':json.loads(clean_status.stdout)},
 'dirty':{'root':str(dirty),'exit_code':dirty_start.returncode,'before':dirty_before,'after':dirty_after,'preserved':True,'state_entries':[],'log_directory_created':False},
 'diagnosis_schema_version':2,'idle_stop_exit_code':clean_stop.returncode,'active_cancellation_observed':False,
 'model':'scripted','real_model_run':False,'complete_coding_workflow_observed':False,
 'interactive_confirmations_reached':False,'native_tools_executed':False,
 'qualification':'actual-binary-native-prerequisite-refusal-only'
}
(out/'observation.json').write_text(json.dumps(report,indent=2)+'\n')
print('BATCH6_LINUX_OBSERVATION_COMPLETE=1',flush=True)
