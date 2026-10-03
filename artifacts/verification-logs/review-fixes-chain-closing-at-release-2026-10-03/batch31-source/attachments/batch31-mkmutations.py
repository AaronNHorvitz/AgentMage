import json
H='agentmage-host'
T='runtime_start_tests::'
m=[]
def add(id,file,filters,expected,pairs,pkg=H):
    m.append({'id':id,'file':file,'package':pkg,'filters':filters,'expected':expected,'replace':pairs})
l='shells/host/src/coding_live_runtime.rs'
F1=T+'a_run_resumed_after_a_restart_keeps_its_chains_open_when_it_cannot_start'
N6=T+'a_new_run_whose_session_cannot_spawn_closes_its_effect_record_empty'
F2=T+'a_release_whose_chains_cannot_be_closed_is_refused_and_keeps_the_run_held'
add('F1a-resumed-spawn-failure-closes',l,['runtime_start_tests'],F1,
    [['                    if start == StoredChainStart::New {\n                        let _ =\n','                    if true {\n                        let _ =\n']])
add('F1b-resumed-job-failure-closes',l,['runtime_start_tests'],F1,
    [['            if start == StoredChainStart::New {\n                let _ = session.close_run_histories();','            if true {\n                let _ = session.close_run_histories();']])
add('N6-new-spawn-failure-left-open',l,['runtime_start_tests'],N6,
    [['                    if start == StoredChainStart::New {\n                        let _ =\n','                    if false {\n                        let _ =\n']])
add('F2a-release-ignores-close',l,['runtime_start_tests'],F2,
    [['        if !session.close_run_histories() {\n            return Err(RuntimeTransportError::RuntimeFailed);','        if !session.close_run_histories() && false {\n            return Err(RuntimeTransportError::RuntimeFailed);']])
add('F2b-refused-release-forgets-run',l,['runtime_start_tests'],F2,
    [['        if !session.close_run_histories() {\n            return Err(RuntimeTransportError::RuntimeFailed);','        if !session.close_run_histories() {\n            self.active.remove(key);\n            return Err(RuntimeTransportError::RuntimeFailed);']])
add('F2c-history-close-failure-ignored',l,['runtime_start_tests'],F2,
    [['                    eprintln!("coding.live.history-close-failed");\n                    closed = false;','                    eprintln!("coding.live.history-close-failed");']])
add('F2d-effect-close-failure-ignored',l,['runtime_start_tests'],F2,
    [['                eprintln!("coding.live.effect-record-close-failed");\n                closed = false;','                eprintln!("coding.live.effect-record-close-failed");']])
open('<state>/batch31-mutations.json','w').write(json.dumps(m,indent=1)+'\n')
print(len(m))
