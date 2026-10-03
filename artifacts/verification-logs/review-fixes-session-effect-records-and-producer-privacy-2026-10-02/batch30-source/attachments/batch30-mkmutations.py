import json
H='agentmage-host'
S='coding_session_recoverability::tests::persisted::'
T='runtime_start_tests::'
G='coding_route_grants::tests::'
m=[]
def add(id,file,filters,expected,pairs,pkg=H):
    m.append({'id':id,'file':file,'package':pkg,'filters':filters,'expected':expected,'replace':pairs})
s='shells/host/src/coding_session_recoverability.rs'
l='shells/host/src/coding_live_runtime.rs'
add('F1a-close-without-pending-mark',s,['coding_session_recoverability'],S+'a_miss_the_store_could_not_mark_keeps_the_chain_open_and_the_session_undeclared',
    [['        if !self.store_pending_mark() {\n            return Err(RunEffectRecordStoreError::Storage);\n        }\n        self.writes.close(&self.run_id)','        self.writes.close(&self.run_id)']])
add('F1b-append-does-not-retry-mark',s,['coding_session_recoverability'],S+'a_pending_mark_is_retried_at_the_next_record_and_carried_across_a_continuation',
    [['        self.store_pending_mark();\n        if self.writes.append(','        if self.writes.append(']])
add('F1c-continuation-carries-nothing',s,['coding_session_recoverability'],S+'a_pending_mark_is_retried_at_the_next_record_and_carried_across_a_continuation',
    [['        if earlier.0.missed.load(Ordering::SeqCst) {','        if false && earlier.0.missed.load(Ordering::SeqCst) {']])
add('F1d-miss-not-shared',s,['coding_session_recoverability'],S+'a_miss_the_store_could_not_mark_keeps_the_chain_open_and_the_session_undeclared',
    [['        !self.knowledge.missed.load(Ordering::SeqCst)\n    }','        true\n    }']])
add('F1e-service-does-not-carry',l,['runtime_start_tests'],T+'a_miss_the_store_could_not_mark_carries_across_an_in_host_continuation',
    [['            chain.continue_from(earlier);','            let _ = (chain, earlier);']])
add('N4-marked-before-check',s,['coding_session_recoverability'],S+'a_resumed_chain_of_another_session_is_refused_before_it_is_marked',
    [['                match records.attach(run_id, RUN_EFFECT_RECORD_OWNER, false) {','                match records.attach(run_id, RUN_EFFECT_RECORD_OWNER, after_restart) {']])
add('F3a-service-never-closes-effect-record',l,['runtime_start_tests'],T+'the_live_service_owns_each_run_job_and_decides_cancellation_through_the_ledger',
    [['        match chain.close_released() {','        match chain\n            .kept_every_entry()\n            .then_some(())\n            .ok_or(RunEffectRecordStoreError::NotFound)\n        {']])
add('F3b-dropped-ended-run-not-closed',l,['runtime_start_tests'],T+'the_live_service_closes_the_effect_record_of_a_held_ended_run_and_of_a_new_run_it_cannot_start',
    [['        if self.is_terminal() {\n            self.close_run_histories();','        if false && self.is_terminal() {\n            self.close_run_histories();']])
add('F5-unstarted-run-left-open',l,['runtime_start_tests'],T+'the_live_service_closes_the_effect_record_of_a_held_ended_run_and_of_a_new_run_it_cannot_start',
    [['            if start == StoredChainStart::New {\n                session.close_run_histories();\n            }\n','']])
add('N7-duplicate-grant-identity-decodes','shells/host/src/coding_route_grants.rs',['coding_route_grants'],G+'a_changed_catalog_is_refused_and_an_answer_must_name_what_was_sent',
    [['                    .all(|kept| kept.valid() && identities.insert(kept.grant.grant_id.as_str()))','                    .all(|kept| kept.valid() && (identities.insert(kept.grant.grant_id.as_str()) || true))']])
open('<state>/batch30-mutations.json','w').write(json.dumps(m,indent=1)+'\n')
print(len(m))
