import json
H='agentmage-host'
m=[]
def add(id,file,filters,expected,pairs,pkg=H):
    m.append({'id':id,'file':file,'package':pkg,'filters':filters,'expected':expected,'replace':pairs})
ev='shells/host/src/standalone_evidence.rs'
sh='shells/host/src/standalone_shell.rs'
T='standalone_evidence::tests::'
S='standalone_shell::tests::'
add('M1-quote-unchecked',ev,['standalone_'],T+'the_verifier_admits_only_observed_citations_with_exact_quotes',
    [['.is_some_and(|quote| quote.is_empty() || !fragment.contains(quote))','.is_some_and(|quote| quote.is_empty())']])
add('M2-folder-follows-links',ev,['standalone_'],T+'a_whole_folder_is_refused_for_each_unsafe_or_oversized_selection',
    [['        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,','        ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS,']])
add('M3-folder-privacy-unchecked',ev,['standalone_'],T+'a_whole_folder_is_refused_for_each_unsafe_or_oversized_selection',
    [['metadata.st_uid != rustix::process::getuid().as_raw() || metadata.st_mode & 0o022 != 0','metadata.st_uid != rustix::process::getuid().as_raw()']])
add('M4-boundary-allows-any-tool',ev,['standalone_'],T+'the_verifier_admits_only_observed_citations_with_exact_quotes',
    [['        if Self::artifact_kind(definition, call).is_none()\n            || request.mode != RuntimeSessionMode::EphemeralReadOnly','        if request.mode != RuntimeSessionMode::EphemeralReadOnly']])
add('M5-cards-for-unverified-runs',sh,['standalone_'],S+'only_a_verified_run_with_observed_citations_becomes_evidence_cards',
    [['    let (AnswerState::Verified, Some(answer)) = (view.state, answer) else {','    let (_, Some(answer)) = (view.state, answer) else {']])
add('M6-gate-repeats-sequence',sh,['standalone_'],S+'the_request_gate_admits_only_exact_requests_in_increasing_order',
    [['        if sequence <= self.last_sequence {','        if sequence < self.last_sequence {']])
add('M7-readmission-while-pinned',ev,['standalone_'],T+'runs_are_bound_to_the_admitted_snapshot_and_pin_it_until_released',
    [['        if !shared.in_flight.is_empty() {\n            return refused(FolderRefusal::Busy);\n        }\n        drop(shared);','        drop(shared);'],
     ['                if !shared.in_flight.is_empty() {\n                    return refused(FolderRefusal::Busy);\n                }\n','']])
add('M8-statementless-answer-admitted',ev,['standalone_'],T+'the_verifier_admits_only_observed_citations_with_exact_quotes',
    [['    if answer.statements.is_empty() || answer.no_matching_text {','    if false {']])
add('M9-admission-digest-unbound',ev,['standalone_'],T+'runs_are_bound_to_the_admitted_snapshot_and_pin_it_until_released',
    [['            || input.expected_entry_sha256 != view.admission_sha256\n','']])
open('<state>/batch35-mutations.json','w').write(json.dumps(m,indent=1)+'\n')
print(len(m))
