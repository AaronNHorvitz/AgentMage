// Real private temporary folders, the real folder walk, prepared sources,
// artifact dispatcher, coordinator and live host service, with the
// term-match fixture as the only model; no process, socket or network.
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{AgentStateKind, RuntimeOutput};
use agentmage_kernel_engine::runtime_coordinator::verify_runtime_outcome;

use super::*;
use crate::runtime_transport::RuntimeTransportStep;

const PROJECT: &str = "# Aurora project\n\nAurora launches on 18 October 2026.\nProject lead: Mira Chen.\nThe Aurora budget is 42,000 credits.\n";
const OPERATIONS: &str =
    "Rehearsal owner: Theo Park.\nThe Aurora rehearsal is on 15 October 2026 at 14:00.\n";

fn now() -> u64 {
    now_epoch_ms().expect("clock")
}

fn profile() -> ExactModelProfile {
    standalone_evidence_profile().expect("fixture profile admits")
}

fn aurora(roots: &fixture::Roots) -> std::path::PathBuf {
    let folder = roots.folder("aurora");
    fixture::write(&folder.join("project.md"), PROJECT.as_bytes());
    fixture::directory(&folder.join("notes"));
    fixture::write(&folder.join("notes/operations.txt"), OPERATIONS.as_bytes());
    fixture::write(&folder.join("report.pdf"), b"%PDF-1.4 synthetic");
    fixture::write(&folder.join(".hidden.md"), b"not for the snapshot\n");
    fixture::directory(&folder.join(".cache"));
    fixture::write(&folder.join(".cache/inside.md"), b"never read\n");
    fixture::write(&folder.join("empty.txt"), b"  \n");
    fixture::write(&folder.join("binary.txt"), b"abc\0def\n");
    fixture::write(&folder.join("latin.txt"), &[b'f', 0xff, b'\n']);
    fixture::write(&folder.join("long.txt"), "x".repeat(5_000).as_bytes());
    fixture::write(&folder.join("big.txt"), "y\n".repeat(70_000).as_bytes());
    fixture::symlink(&folder.join("project.md"), &folder.join("link.md"));
    fixture::write(
        &folder.join("forged\nsource\tfolder-source-999\tff.md"),
        b"Aurora launches tomorrow.\n",
    );
    folder
}

fn reason(view: &FolderAdmissionView, path: &str) -> (FolderEntryDisposition, String) {
    let entry = view
        .entries
        .iter()
        .find(|entry| entry.path == path)
        .unwrap_or_else(|| panic!("{path} was enumerated"));
    (entry.disposition, entry.reason_code.clone())
}

#[test]
fn only_two_private_separate_roots_activate() {
    let roots = fixture::Roots::new("activation");
    let activation =
        StandaloneEvidenceActivation::validate(&roots.state, &roots.disposable).expect("roots");
    activation.revalidate().expect("still valid");
    let denied = Err(StandaloneEvidenceActivationError::RootDenied);
    assert_eq!(
        StandaloneEvidenceActivation::validate(Path::new("relative"), &roots.disposable),
        denied
    );
    assert_eq!(
        StandaloneEvidenceActivation::validate(&roots.state, &roots.state),
        denied
    );
    let nested = roots.folder("nested-state");
    assert_eq!(
        StandaloneEvidenceActivation::validate(&nested, &roots.disposable),
        denied
    );
    fixture::mode(&roots.disposable, 0o750);
    assert_eq!(
        activation.revalidate(),
        Err(StandaloneEvidenceActivationError::RootDenied)
    );
    assert_eq!(
        StandaloneEvidenceActivationError::RootDenied.code(),
        "standalone.evidence.root-denied"
    );
}

#[test]
fn folder_admission_accounts_for_every_entry_and_admits_only_supported_text() {
    let roots = fixture::Roots::new("admission");
    let activation = roots.activation();
    let folder = aurora(&roots);
    let snapshot = admit_folder(&activation, folder.to_str().unwrap(), &profile(), now())
        .expect("folder admits");
    let view = snapshot.view();
    assert!(view.verify());
    assert_eq!(view.folder, folder.to_str().unwrap());
    assert_eq!((view.accepted, view.skipped, view.rejected), (2, 4, 6));
    let expected = [
        (
            ".cache",
            FolderEntryDisposition::Skipped,
            "folder.entry.hidden",
        ),
        (
            ".hidden.md",
            FolderEntryDisposition::Skipped,
            "folder.entry.hidden",
        ),
        (
            "big.txt",
            FolderEntryDisposition::Rejected,
            "folder.entry.too-large",
        ),
        (
            "binary.txt",
            FolderEntryDisposition::Rejected,
            "folder.entry.control-characters",
        ),
        (
            "empty.txt",
            FolderEntryDisposition::Skipped,
            "folder.entry.empty",
        ),
        // A name with a line break cannot add a line to the inventory.
        (
            "forged\\nsource\\tfolder-source-999\\tff.md",
            FolderEntryDisposition::Rejected,
            "folder.entry.name-invalid",
        ),
        (
            "latin.txt",
            FolderEntryDisposition::Rejected,
            "folder.entry.not-utf8",
        ),
        (
            "link.md",
            FolderEntryDisposition::Rejected,
            "folder.entry.symlink",
        ),
        (
            "long.txt",
            FolderEntryDisposition::Rejected,
            "folder.entry.long-line",
        ),
        (
            "notes/operations.txt",
            FolderEntryDisposition::Accepted,
            "folder.entry.accepted",
        ),
        (
            "project.md",
            FolderEntryDisposition::Accepted,
            "folder.entry.accepted",
        ),
        (
            "report.pdf",
            FolderEntryDisposition::Skipped,
            "folder.entry.unsupported-format",
        ),
    ];
    assert_eq!(view.entries.len(), expected.len());
    for (path, disposition, code) in expected {
        assert_eq!(reason(view, path), (disposition, code.to_owned()), "{path}");
    }
    // Accepted sources are numbered in path order and bound to their bytes.
    let project = view
        .entries
        .iter()
        .find(|entry| entry.path == "project.md")
        .unwrap();
    assert_eq!(project.source_id.as_deref(), Some("folder-source-002"));
    assert_eq!(
        project.content_sha256.as_deref(),
        Some(sha256(PROJECT.as_bytes()).as_str())
    );
    assert_eq!(
        view.admitted_bytes,
        (PROJECT.len() + OPERATIONS.len()) as u64
    );
    // The hidden directory was never entered, and nothing was written.
    assert!(
        !view
            .entries
            .iter()
            .any(|entry| entry.path.starts_with(".cache/"))
    );
    assert_eq!(
        fixture::read(&folder.join("project.md")),
        PROJECT.as_bytes()
    );
    // The same folder admits to the same identity and digest.
    let again =
        admit_folder(&activation, folder.to_str().unwrap(), &profile(), now()).expect("again");
    assert_eq!(again.view().admission_sha256, view.admission_sha256);
    assert_eq!(again.view().workspace_id, view.workspace_id);
    // A changed activation refuses the folder with its own code.
    fixture::mode(&roots.disposable, 0o750);
    assert_eq!(
        admit_folder(&activation, folder.to_str().unwrap(), &profile(), now()).err(),
        Some(FolderRefusal::ActivationChanged)
    );
    fixture::mode(&roots.disposable, 0o700);
    // The inventory source is always present and lists each accepted source.
    let manifests = snapshot.sources.manifests();
    assert!(
        manifests
            .iter()
            .any(|manifest| manifest.source_id == FOLDER_INVENTORY_SOURCE_ID)
    );
    assert_eq!(manifests.len(), 3);
}

#[test]
fn a_whole_folder_is_refused_for_each_unsafe_or_oversized_selection() {
    let roots = fixture::Roots::new("refusal");
    let activation = roots.activation();
    let admit = |folder: &str| {
        admit_folder(&activation, folder, &profile(), now())
            .map(|snapshot| snapshot.view().clone())
            .err()
    };
    let inside = roots.folder("inside");
    fixture::write(&inside.join("a.md"), b"Aurora\n");
    let inside_text = inside.to_str().unwrap().to_owned();
    for path in [
        "relative/folder".to_owned(),
        String::new(),
        format!("{inside_text}/"),
        format!("{inside_text}/../inside"),
        format!("{inside_text}/."),
        format!("{}//inside", roots.disposable.to_str().unwrap()),
    ] {
        assert_eq!(admit(&path), Some(FolderRefusal::NotAbsolute), "{path}");
    }
    assert_eq!(
        admit(roots.disposable.to_str().unwrap()),
        Some(FolderRefusal::OutsideDisposableRoot)
    );
    assert_eq!(
        admit(roots.state.to_str().unwrap()),
        Some(FolderRefusal::OutsideDisposableRoot)
    );
    let missing = format!("{}/missing", roots.disposable.to_str().unwrap());
    assert_eq!(admit(&missing), Some(FolderRefusal::Unavailable));
    assert_eq!(
        admit(inside.join("a.md").to_str().unwrap()),
        Some(FolderRefusal::Unavailable)
    );
    let linked = roots.disposable.join("linked");
    fixture::symlink(&inside, &linked);
    assert_eq!(
        admit(linked.to_str().unwrap()),
        Some(FolderRefusal::Unavailable)
    );
    let shared = roots.folder("shared");
    fixture::mode(&shared, 0o777);
    assert_eq!(
        admit(shared.to_str().unwrap()),
        Some(FolderRefusal::NotPrivate)
    );

    let crowded = roots.folder("crowded");
    for index in 0..=MAX_FOLDER_ENTRIES {
        fixture::write(&crowded.join(format!("{index:03}.md")), b"x\n");
    }
    assert_eq!(
        admit(crowded.to_str().unwrap()),
        Some(FolderRefusal::TooManyEntries)
    );

    let deep = roots.folder("deep");
    let mut path = deep.clone();
    for level in 0..=MAX_FOLDER_DEPTH {
        path = path.join(format!("level-{level}"));
        fixture::directory(&path);
    }
    assert_eq!(admit(deep.to_str().unwrap()), Some(FolderRefusal::TooDeep));

    let heavy = roots.folder("heavy");
    for index in 0..9 {
        fixture::write(
            &heavy.join(format!("{index}.txt")),
            "z\n".repeat(60_000).as_bytes(),
        );
    }
    assert_eq!(
        admit(heavy.to_str().unwrap()),
        Some(FolderRefusal::TooLarge)
    );
}

#[test]
fn question_terms_are_the_first_four_uncommon_words_as_written() {
    assert_eq!(
        question_terms("When does Aurora launch?"),
        ["Aurora", "launch"]
    );
    assert_eq!(
        question_terms("What is the budget, the BUDGET and the plan?"),
        ["budget", "plan"]
    );
    assert_eq!(
        question_terms("alpha beta gamma delta epsilon zeta"),
        ["alpha", "beta", "gamma", "delta"]
    );
    assert!(question_terms("Is it ok?").is_empty());
}

fn service(roots: &fixture::Roots) -> StandaloneEvidenceService {
    StandaloneEvidenceService::new(roots.activation(), Duration::ZERO).expect("service composes")
}

fn admitted(service: &mut StandaloneEvidenceService, folder: &Path) -> FolderAdmissionView {
    match service
        .folder(FolderRequest::Admit {
            folder: folder.to_str().unwrap().to_owned(),
        })
        .expect("folder answers")
    {
        FolderAnswer::Admitted { admission } => admission,
        FolderAnswer::Refused { refusal } => panic!("refused: {refusal:?}"),
    }
}

fn question(
    view: &FolderAdmissionView,
    prompt: &str,
    session: Option<SessionId>,
) -> RuntimePrepareInput {
    RuntimePrepareInput {
        resume: false,
        record_session: false,
        slow_subscriber_probe: false,
        preauthorization: None,
        engineering_session_id: session,
        profile_id: STANDALONE_EVIDENCE_PROFILE_ID.to_owned(),
        expected_entry_sha256: view.admission_sha256.clone(),
        workspace_id: view.workspace_id.clone(),
        workspace_root: view.folder.clone(),
        prompt: prompt.to_owned(),
        recipe: None,
    }
}

fn last_cursor(step: &RuntimeTransportStep) -> Option<RuntimeEventCursor> {
    step.events.last().map(|event| RuntimeEventCursor {
        run_id: event.run_id.clone(),
        event_id: event.event_id.clone(),
        sequence: event.sequence,
        event_sha256: event.event_sha256.clone(),
    })
}

fn run_to_end(
    service: &mut StandaloneEvidenceService,
    request: &RuntimeRunRequest,
) -> RuntimeTransportStep {
    let mut step = service.start(request.clone()).expect("start");
    let deadline = Instant::now() + Duration::from_secs(60);
    while step.outcome.is_none() {
        assert!(Instant::now() < deadline, "the run ends in time");
        let next = service
            .advance(
                &request.run_id,
                &request.request_sha256,
                last_cursor(&step).as_ref(),
                None,
            )
            .expect("advance");
        assert!(next.approval.is_none(), "an evidence run never asks");
        let mut events = step.events.clone();
        events.extend(next.events.iter().cloned());
        step = RuntimeTransportStep { events, ..next };
    }
    step
}

fn inline_answer(output: Option<&RuntimeOutput>) -> EvidenceAnswer {
    let Some(RuntimeOutput::Inline { payload }) = output else {
        panic!("an inline answer");
    };
    serde_json::from_slice(&payload.bytes).expect("closed answer")
}

#[test]
fn a_question_runs_through_the_live_service_to_a_verified_cited_answer() {
    let roots = fixture::Roots::new("verified");
    let folder = aurora(&roots);
    let mut service = service(&roots);
    let view = admitted(&mut service, &folder);
    let request = service
        .prepare(question(&view, "When does Aurora launch?", None))
        .expect("prepare");
    assert_eq!(request.mode, RuntimeSessionMode::EphemeralReadOnly);
    assert_eq!(request.workspace_snapshot_sha256, view.admission_sha256);
    assert!(
        request
            .visible_tools
            .iter()
            .all(|tool| tool.tool_id.as_str().starts_with("artifact."))
    );
    let step = run_to_end(&mut service, &request);
    let outcome = step.outcome.expect("outcome");
    assert_eq!(
        outcome.state,
        AgentStateKind::Success,
        "{:?}",
        outcome.unresolved_codes
    );
    verify_runtime_outcome(&outcome, &request).expect("outcome verifies");
    assert!(outcome.answer_evidence.is_some());
    // Two sources times two terms.
    assert_eq!(outcome.tool_call_count, 4);
    let answer = inline_answer(outcome.output.as_ref());
    assert_eq!(answer.terms, ["Aurora", "launch"]);
    assert!(!answer.no_matching_text);
    assert_eq!(answer.statements.len(), 1);
    let statement = &answer.statements[0];
    assert_eq!(statement.text, "Aurora launches on 18 October 2026.");
    assert_eq!(statement.citations[0].source_id, "folder-source-002");
    assert!(
        outcome
            .evidence
            .iter()
            .any(|evidence| evidence.evidence_id.as_str() == statement.citations[0].evidence_id)
    );
    // The declarations name the run's context views and no effects.
    let declarations = service
        .run_declarations(&request.run_id, &request.request_sha256)
        .expect("declarations");
    let views = declarations.context_inspections.as_ref().expect("views");
    assert_eq!(views.len(), 5);
    service
        .release(&request.run_id, &request.request_sha256)
        .expect("release");
    // A follow-up in the same session names it.
    let follow = service
        .prepare(question(
            &view,
            "What is the budget?",
            Some(request.session_id.clone()),
        ))
        .expect("follow-up");
    assert_eq!(follow.session_id, request.session_id);
    let step = run_to_end(&mut service, &follow);
    let outcome = step.outcome.expect("outcome");
    assert_eq!(outcome.state, AgentStateKind::Success);
    let answer = inline_answer(outcome.output.as_ref());
    assert_eq!(
        answer.statements[0].text,
        "The Aurora budget is 42,000 credits."
    );
    service
        .release(&follow.run_id, &follow.request_sha256)
        .expect("release");
    assert_eq!(
        fixture::read(&folder.join("project.md")),
        PROJECT.as_bytes()
    );
}

#[test]
fn a_question_without_matching_text_ends_unverified_and_says_so() {
    let roots = fixture::Roots::new("unmatched");
    let folder = aurora(&roots);
    let mut service = service(&roots);
    let view = admitted(&mut service, &folder);
    let request = service
        .prepare(question(&view, "What is the favorite flavor?", None))
        .expect("prepare");
    let outcome = run_to_end(&mut service, &request).outcome.expect("outcome");
    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(outcome.unresolved_codes, ["runtime.verification.failed"]);
    assert!(outcome.answer_evidence.is_none());
    let answer = inline_answer(outcome.output.as_ref());
    assert!(answer.no_matching_text);
    assert!(answer.statements.is_empty());
    assert_eq!(answer.searches, 4);
}

#[test]
fn runs_are_bound_to_the_admitted_snapshot_and_pin_it_until_released() {
    let roots = fixture::Roots::new("binding");
    let folder = aurora(&roots);
    let other = roots.folder("other");
    fixture::write(&other.join("other.md"), b"Other text\n");
    let mut service = service(&roots);
    assert_eq!(
        service.prepare(question(
            &FolderAdmissionView {
                folder: folder.to_str().unwrap().to_owned(),
                ..aurora_view_placeholder()
            },
            "When does Aurora launch?",
            None
        )),
        Err(RuntimeTransportError::RequestDenied),
        "nothing is admitted yet"
    );
    let view = admitted(&mut service, &folder);
    let denied = Err(RuntimeTransportError::RequestDenied);
    let mut wrong_digest = question(&view, "When does Aurora launch?", None);
    wrong_digest.expected_entry_sha256 = "f".repeat(64);
    assert_eq!(service.prepare(wrong_digest), denied);
    let mut wrong_profile = question(&view, "When does Aurora launch?", None);
    wrong_profile.profile_id = "deterministic-fake-muse-v1".to_owned();
    assert_eq!(service.prepare(wrong_profile), denied);
    let mut wrong_folder = question(&view, "When does Aurora launch?", None);
    wrong_folder.workspace_root = other.to_str().unwrap().to_owned();
    assert_eq!(service.prepare(wrong_folder), denied);
    assert_eq!(service.prepare(question(&view, "   ", None)), denied);
    assert_eq!(
        service.prepare(question(&view, &"q".repeat(MAX_QUESTION_BYTES + 1), None)),
        denied
    );
    let mut recorded = question(&view, "When does Aurora launch?", None);
    recorded.record_session = true;
    assert_eq!(service.prepare(recorded), denied);

    let request = service
        .prepare(question(&view, "When does Aurora launch?", None))
        .expect("prepare");
    let busy = service
        .folder(FolderRequest::Admit {
            folder: other.to_str().unwrap().to_owned(),
        })
        .expect("answers");
    assert_eq!(
        busy,
        FolderAnswer::Refused {
            refusal: FolderRefusal::Busy
        }
    );
    // Releasing the unstarted run frees the snapshot; a new folder admits
    // and the old run's request no longer starts.
    service
        .release(&request.run_id, &request.request_sha256)
        .expect("release");
    let other_view = admitted(&mut service, &other);
    assert_ne!(other_view.admission_sha256, view.admission_sha256);
    assert!(service.start(request).is_err());
}

fn aurora_view_placeholder() -> FolderAdmissionView {
    FolderAdmissionView {
        schema_version: FOLDER_ADMISSION_SCHEMA_VERSION,
        admission_sha256: "a".repeat(64),
        folder: String::new(),
        workspace_id: String::new(),
        entries: Vec::new(),
        accepted: 0,
        skipped: 0,
        rejected: 0,
        admitted_bytes: 0,
        limits: FolderLimitsView::current(),
    }
}

#[test]
fn a_cancelled_question_ends_cancelled_without_an_answer() {
    let roots = fixture::Roots::new("cancelled");
    let folder = aurora(&roots);
    // The fixture waits two seconds before each proposal and observes the
    // requested cancellation while it waits.
    let mut service = StandaloneEvidenceService::new(roots.activation(), MAX_FIXTURE_STEP_DELAY)
        .expect("service");
    let view = admitted(&mut service, &folder);
    let request = service
        .prepare(question(&view, "When does Aurora launch?", None))
        .expect("prepare");
    let mut step = service.start(request.clone()).expect("start");
    // Each run is one job; a direct cancellation is refused, and the
    // cancellation goes through the ledger under a client scope, as the IPC
    // service derives one for its authenticated peer (Decision 0120).
    assert_eq!(
        service.cancel(
            &request.run_id,
            &request.request_sha256,
            CancellationId::from_raw("standalone-evidence-test-cancel"),
            last_cursor(&step).as_ref(),
        ),
        Err(RuntimeTransportError::RequestDenied)
    );
    let status = service
        .job_status(&request.run_id, &request.request_sha256)
        .expect("the run is one job");
    let control = JobControlRequest {
        schema_version: 1,
        job_id: request.run_id.as_str().to_owned(),
        request_id: "standalone-evidence-test-cancel".to_owned(),
        action: agentmage_kernel_engine::job_control::JobControlAction::Cancel,
        observed_revision: status.job.revision,
    };
    let answer = service
        .control_job_for_client(
            &RuntimeClientScope::derived("peer-00112233445566778899aabbccddeeff".to_owned()),
            &request.run_id,
            &request.request_sha256,
            &control,
        )
        .expect("the ledger decides");
    assert!(answer.answers(&request, &control));
    let deadline = Instant::now() + Duration::from_secs(30);
    while step.outcome.is_none() {
        assert!(Instant::now() < deadline, "the cancelled run ends in time");
        let next = service
            .advance(
                &request.run_id,
                &request.request_sha256,
                last_cursor(&step).as_ref(),
                None,
            )
            .expect("advance");
        let mut events = step.events.clone();
        events.extend(next.events.iter().cloned());
        step = RuntimeTransportStep { events, ..next };
    }
    let outcome = step.outcome.expect("outcome");
    assert_eq!(
        outcome.state,
        AgentStateKind::Cancelled,
        "{:?}",
        outcome.unresolved_codes
    );
    assert_eq!(outcome.tool_call_count, 0);
    assert!(outcome.answer_evidence.is_none());
    assert!(outcome.output.is_none());
    // A cancelled run is released like any other and frees the snapshot.
    service
        .release(&request.run_id, &request.request_sha256)
        .expect("release");
    let _ = admitted(&mut service, &folder);
}

/// One real search over the snapshot through the boundary, with its result.
fn searched(roots: &fixture::Roots, term: &str) -> (RuntimeRunRequest, RuntimeToolExecution) {
    let folder = aurora(roots);
    let activation = roots.activation();
    let shared = Arc::new(Mutex::new(EvidenceShared::default()));
    let snapshot = Arc::new(
        admit_folder(&activation, folder.to_str().unwrap(), &profile(), now()).expect("admit"),
    );
    shared.lock().unwrap().snapshot = Some(Arc::clone(&snapshot));
    let mut factory =
        StandaloneEvidenceRuntimeFactory::new(activation, Arc::clone(&shared), Duration::ZERO)
            .expect("factory");
    let request = factory
        .prepare_runtime_request(&question(snapshot.view(), "When does Aurora launch?", None))
        .expect("request");
    let project = snapshot
        .view()
        .entries
        .iter()
        .find(|entry| entry.path == "project.md")
        .unwrap();
    let manifest = snapshot
        .sources
        .manifests()
        .into_iter()
        .find(|manifest| Some(manifest.source_id.as_str()) == project.source_id.as_deref())
        .unwrap();
    let definition = artifact_tool_definition(ArtifactToolKind::Search);
    let arguments = serde_json::to_vec(&ArtifactRequest {
        schema_version: 1,
        call_id: "test-search".to_owned(),
        source_id: Some(manifest.source_id.clone()),
        section_id: None,
        range: None,
        query: Some(term.to_owned()),
        freshness_sha256: Some(manifest.manifest_sha256.clone()),
        output_identity: "test-search-output".to_owned(),
        limits: ArtifactLimits::default(),
        call_depth: 0,
    })
    .unwrap();
    let call = ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("test-call"),
        correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw("test-correlation"),
        action_id: agentmage_kernel_contracts::ActionId::from_raw("test-action"),
        tool_id: definition.tool_id.clone(),
        tool_version: ARTIFACT_TOOL_VERSION.to_owned(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".to_owned(),
            sha256: sha256(&arguments),
            bytes: arguments,
        },
    };
    let mut boundary = EvidenceToolBoundary {
        sources: Arc::clone(&snapshot.sources),
        admission_sha256: snapshot.view().admission_sha256.clone(),
        ledger: ArtifactAttemptLedger::default(),
        executions: 0,
        _store: None,
    };
    let operation = RuntimeOperationId::from_raw("test-operation");
    let evaluation = boundary
        .evaluate(&request, &operation, &definition, &call, now())
        .unwrap();
    assert!(matches!(
        evaluation,
        RuntimePermissionEvaluation::Allow { .. }
    ));
    let execution = boundary
        .execute(&request, &evaluation, &definition, &call, None)
        .expect("search runs");
    // Any tool outside the artifact catalog is denied before any effect.
    let mut foreign = definition.clone();
    foreign.tool_id = agentmage_kernel_contracts::ToolId::from_raw("workspace.read_file");
    let mut foreign_call = call.clone();
    foreign_call.tool_id = foreign.tool_id.clone();
    assert!(matches!(
        boundary.evaluate(&request, &operation, &foreign, &foreign_call, now()),
        Ok(RuntimePermissionEvaluation::Deny { .. })
    ));
    assert!(
        boundary
            .execute(&request, &evaluation, &foreign, &foreign_call, None)
            .is_err()
    );
    (request, execution)
}

fn payload_of(answer: &EvidenceAnswer) -> ContractPayload {
    let bytes = serde_json::to_vec(answer).unwrap();
    ContractPayload {
        schema: answer_schema(),
        media_type: STANDALONE_EVIDENCE_ANSWER_MEDIA_TYPE.to_owned(),
        sha256: sha256(&bytes),
        bytes,
    }
}

#[test]
fn the_verifier_admits_only_observed_citations_with_exact_quotes() {
    let roots = fixture::Roots::new("verifier");
    // The prepared-source search matches whole terms: "launch" finds nothing
    // in "Aurora launches", so the test searches for the word itself.
    let (_request, execution) = searched(&roots, "launches");
    let evidence = execution.result.evidence.clone();
    let results = [execution.result.clone()];
    let artifact: ArtifactResult =
        serde_json::from_slice(&execution.result.output.as_ref().unwrap().bytes).unwrap();
    let (start, end, content) = artifact
        .items
        .iter()
        .find_map(|item| match item {
            ArtifactItem::SearchHit {
                fragment:
                    ArtifactFragment::Line {
                        start,
                        end_exclusive,
                        content,
                    },
                ..
            } => Some((*start, *end_exclusive, content.clone())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a line hit: {artifact:?}"));
    assert!(content.contains("Aurora launches"));
    let good = EvidenceAnswer {
        schema_version: 1,
        statements: vec![EvidenceStatement {
            text: content.trim().to_owned(),
            quote: Some(content.clone()),
            citations: vec![EvidenceCitation {
                evidence_id: evidence[0].evidence_id.as_str().to_owned(),
                source_id: artifact.source_id.clone().unwrap(),
                start_line: start,
                end_line_exclusive: end,
            }],
        }],
        terms: vec!["launches".to_owned()],
        searches: 1,
        no_matching_text: false,
    };
    assert_eq!(
        check_evidence_answer(Some(&payload_of(&good)), &evidence, &results)
            .map(|cited| cited.len()),
        Ok(1)
    );
    let mut cases: Vec<(&str, EvidenceAnswer)> = Vec::new();
    let mut forged = good.clone();
    forged.statements[0].citations[0].evidence_id = "forged-evidence".to_owned();
    cases.push(("standalone.evidence.citation-unobserved", forged));
    let mut moved = good.clone();
    moved.statements[0].citations[0].start_line += 1;
    moved.statements[0].citations[0].end_line_exclusive += 1;
    cases.push(("standalone.evidence.citation-range", moved));
    let mut other_source = good.clone();
    other_source.statements[0].citations[0].source_id = "folder-source-001".to_owned();
    cases.push(("standalone.evidence.citation-range", other_source));
    let mut misquoted = good.clone();
    misquoted.statements[0].quote = Some("Aurora launches on 19 October 2026.".to_owned());
    cases.push(("standalone.evidence.quote-mismatch", misquoted));
    let mut uncited = good.clone();
    uncited.statements[0].citations.clear();
    cases.push(("standalone.evidence.uncited-statement", uncited));
    let mut empty = good.clone();
    empty.statements.clear();
    cases.push(("standalone.evidence.no-cited-statement", empty));
    let mut claimed_none = good.clone();
    claimed_none.no_matching_text = true;
    cases.push(("standalone.evidence.no-cited-statement", claimed_none));
    for (code, answer) in cases {
        assert_eq!(
            check_evidence_answer(Some(&payload_of(&answer)), &evidence, &results),
            Err(code)
        );
    }
    let mut renamed = payload_of(&good);
    renamed.schema.schema_id = SchemaId::from_raw("runtime.other.answer");
    assert_eq!(
        check_evidence_answer(Some(&renamed), &evidence, &results),
        Err("standalone.evidence.answer-schema")
    );
    let mut altered = payload_of(&good);
    altered.bytes.push(b' ');
    assert_eq!(
        check_evidence_answer(Some(&altered), &evidence, &results),
        Err("standalone.evidence.answer-schema")
    );
    assert_eq!(
        check_evidence_answer(None, &evidence, &results),
        Err("standalone.evidence.answer-absent")
    );
    // Evidence observed elsewhere does not count without its own result.
    assert_eq!(
        check_evidence_answer(Some(&payload_of(&good)), &evidence, &[]),
        Err("standalone.evidence.citation-range")
    );
}

// The fixture creates, links and changes the modes of files, so it is a test
// module of its own for the effect boundary scan.
#[cfg(test)]
mod fixture {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::StandaloneEvidenceActivation;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    pub(super) struct Roots {
        base: PathBuf,
        pub(super) state: PathBuf,
        pub(super) disposable: PathBuf,
    }

    impl Roots {
        pub(super) fn new(tag: &str) -> Self {
            let base = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "agentmage-standalone-{tag}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let state = base.join("state");
            let disposable = base.join("disposable");
            for path in [&base, &state, &disposable] {
                directory(path);
            }
            Self {
                base,
                state,
                disposable,
            }
        }

        pub(super) fn activation(&self) -> StandaloneEvidenceActivation {
            StandaloneEvidenceActivation::validate(&self.state, &self.disposable)
                .expect("test roots activate")
        }

        pub(super) fn folder(&self, name: &str) -> PathBuf {
            let path = self.disposable.join(name);
            directory(&path);
            path
        }
    }

    impl Drop for Roots {
        fn drop(&mut self) {
            let _ =
                std::fs::set_permissions(&self.disposable, std::fs::Permissions::from_mode(0o700));
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    pub(super) fn directory(path: &Path) {
        std::fs::create_dir(path).unwrap();
        mode(path, 0o700);
    }

    pub(super) fn mode(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    pub(super) fn write(path: &Path, bytes: &[u8]) {
        std::fs::write(path, bytes).unwrap();
    }

    pub(super) fn read(path: &Path) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }

    pub(super) fn symlink(target: &Path, link: &Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }
}
