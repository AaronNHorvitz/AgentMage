import json
H='agentmage-host'
m=[]
def add(id,file,filters,expected,pairs,pkg=H):
    m.append({'id':id,'file':file,'package':pkg,'filters':filters,'expected':expected,'replace':pairs})
ipc='shells/host/src/runtime_ipc.rs'
live='shells/host/src/coding_live_runtime.rs'
F1='runtime_ipc::tests::the_client_keeps_the_first_refusal_of_a_run_until_the_next_prepare'
N3='runtime_start_tests::a_release_refused_for_a_running_run_or_another_digest_closes_no_chain'
add('F1a-latest-refusal-kept',ipc,['runtime_ipc'],F1,
    [['        if let Err(error) = &result\n            && self.last_error.is_none()\n        {','        if let Err(error) = &result {']])
add('F1b-prepare-keeps-refusal',ipc,['runtime_ipc'],F1,
    [['        self.last_error = None;\n','']])
add('F1c-refusal-never-kept',ipc,['runtime_ipc'],F1,
    [['            && self.last_error.is_none()\n','            && self.last_error.is_none()\n            && false\n']])
add('N3a-close-before-every-check',live,['runtime_start_tests'],N3,
    [['        if !session.is_terminal() || session.request.request_sha256 != request_sha256 {\n            return Err(RuntimeTransportError::RequestDenied);\n        }\n',
      '        if !session.close_run_histories() {\n            return Err(RuntimeTransportError::RuntimeFailed);\n        }\n        if !session.is_terminal() || session.request.request_sha256 != request_sha256 {\n            return Err(RuntimeTransportError::RequestDenied);\n        }\n']])
add('N3b-close-before-digest-check',live,['runtime_start_tests'],N3,
    [['        if !session.is_terminal() || session.request.request_sha256 != request_sha256 {\n            return Err(RuntimeTransportError::RequestDenied);\n        }\n',
      '        if !session.is_terminal() {\n            return Err(RuntimeTransportError::RequestDenied);\n        }\n        if !session.close_run_histories() {\n            return Err(RuntimeTransportError::RuntimeFailed);\n        }\n        if session.request.request_sha256 != request_sha256 {\n            return Err(RuntimeTransportError::RequestDenied);\n        }\n']])
open('<state>/batch33-mutations.json','w').write(json.dumps(m,indent=1)+'\n')
print(len(m))
