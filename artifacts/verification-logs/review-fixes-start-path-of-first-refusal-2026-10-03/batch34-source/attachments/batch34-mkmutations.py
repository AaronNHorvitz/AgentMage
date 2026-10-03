import json
H='agentmage-host'
m=[]
def add(id,file,filters,expected,pairs,pkg=H):
    m.append({'id':id,'file':file,'package':pkg,'filters':filters,'expected':expected,'replace':pairs})
ipc='shells/host/src/runtime_ipc.rs'
F1='runtime_ipc::tests::the_client_keeps_the_first_refusal_of_a_run_until_the_next_prepare'
START_HEAD='    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {\n        match self.exchange(RuntimeIpcRequest::Start { request })? {\n'
ADV_HEAD='    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {\n        let mut after_event_cursor = after_event_cursor.cloned();\n'
# RB of the f33245fb review: start clears the kept refusal before its exchange.
add('F1d-start-clears-refusal',ipc,['runtime_ipc'],F1,
    [[START_HEAD, START_HEAD.replace('{\n        match','{\n        self.last_error = None;\n        match',1)]])
add('F1e-advance-clears-refusal',ipc,['runtime_ipc'],F1,
    [[ADV_HEAD, ADV_HEAD.replace('{\n        let mut','{\n        self.last_error = None;\n        let mut',1)]])
add('F1f-start-bypasses-exchange',ipc,['runtime_ipc'],F1,
    [['        match self.exchange(RuntimeIpcRequest::Start { request })? {\n','        match self.exchange_inner(RuntimeIpcRequest::Start { request })? {\n']])
add('F1g-advance-bypasses-exchange',ipc,['runtime_ipc'],F1,
    [['        match self.exchange(RuntimeIpcRequest::Advance {\n','        match self.exchange_inner(RuntimeIpcRequest::Advance {\n']])
open('<state>/batch34-mutations.json','w').write(json.dumps(m,indent=1)+'\n')
print(len(m))
