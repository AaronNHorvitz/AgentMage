//! Synthetic coordinator fixtures for an explicit research admission
//! (Decision 0137). Nothing here retrieves, grants, dispatches or runs a model.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

use super::*;
use crate::public_research::{PublicSearchRequest, PublicSourceType};
use crate::research_budget::{
    ResearchBudgetProgress, ResearchDepth, ResearchLimits, ResearchNetworkMode, ResearchScope,
};
use crate::research_fetch::PublicSearchEndpoint;
use crate::research_journal::{ResearchBudgetContext, ResearchBudgetState};
use crate::research_plan::{PreparedResearchPlan, ResearchPlanDraft};
use crate::research_report::{ResearchReportClaimDraft, ResearchReportDraft, ResearchSourceSpan};
use crate::runtime_artifact::{RESEARCH_REPORT_DRAFT_MEDIA_TYPE, runtime_payload_reference};
use crate::runtime_loop::{
    RESEARCH_ADMISSION_CONSTRAINT_PREFIX, RESEARCH_BUDGET_CANCELLATION_UNCONFIRMED,
    RESEARCH_REPORT_REFUSED, RuntimeModelOperationFailure, RuntimeResearchAdmission,
    RuntimeResearchBudgetPort, valid_admitted_network_execution,
};
use agentmage_kernel_contracts::{RuntimeArtifactId, RuntimeEventCursor, RuntimeEventId};

#[path = "runtime_loop_research_completion_tests.rs"]
mod completion_tests;

/// How the synthetic budget owner answers the coordinator's opening.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BudgetAnswer {
    Exact,
    OtherPlan,
    OtherScope,
    Spent,
    SpentQueries,
    SpentBytes,
    Cancelled,
    DeadlineExhausted,
    LaterRevision,
    Refused,
}

/// One opening as the synthetic owner saw it.
struct Opening {
    context: ResearchBudgetContext,
    plan: RuntimeArtifactRef,
    plan_bytes: Vec<u8>,
    plan_event_journaled: bool,
    flushes: usize,
}

/// How the synthetic budget owner answers a cancellation (Decision 0140).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CancelAnswer {
    Exact,
    NotCancelled,
    OtherPlan,
    OtherScope,
    Refused,
}

/// One budget cancellation as the synthetic owner saw it.
struct Cancellation {
    context: ResearchBudgetContext,
    after_observation: bool,
    before_terminal: bool,
}

/// How the synthetic report owner answers a draft (Decision 0142).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReportAnswer {
    Exact,
    Refused,
    Uncertain,
    OtherReference,
}

/// One report publication as the synthetic owner saw it.
struct ReportPublication {
    context: ResearchBudgetContext,
    manifest: RuntimeArtifactManifest,
    draft: Vec<u8>,
    flushes: usize,
    journaled: usize,
}

thread_local! {
    static BUDGET_ANSWER: Cell<BudgetAnswer> = const { Cell::new(BudgetAnswer::Exact) };
    static OPENINGS: RefCell<Vec<Opening>> = const { RefCell::new(Vec::new()) };
    static CANCEL_ANSWER: Cell<CancelAnswer> = const { Cell::new(CancelAnswer::Exact) };
    static CANCELLATIONS: RefCell<Vec<Cancellation>> = const { RefCell::new(Vec::new()) };
    static REPORT_ANSWER: Cell<ReportAnswer> = const { Cell::new(ReportAnswer::Exact) };
    static REPORTS: RefCell<Vec<ReportPublication>> = const { RefCell::new(Vec::new()) };
}

impl RuntimeResearchBudgetPort for FakeToolBoundary {
    fn open_research_budget(
        &mut self,
        _request: &RuntimeRunRequest,
        context: &ResearchBudgetContext,
        plan: &RuntimeArtifactRef,
        scope: &ResearchScope,
        _now_epoch_ms: u64,
    ) -> Result<ResearchBudgetState, RuntimePortFailure> {
        // The owner reads the plan the run published and its creation event.
        let plan_bytes = self
            .artifacts
            .lock()
            .unwrap()
            .iter()
            .find(|(manifest, _)| manifest.artifact_id == plan.artifact_id)
            .map(|(_, bytes)| bytes.clone())
            .unwrap_or_default();
        let plan_event_journaled = self.journal.lock().unwrap().iter().any(|event| {
            matches!(&event.kind, RuntimeEventKind::ArtifactCreated { artifact_id, .. }
                if artifact_id == &plan.artifact_id)
        });
        let flushes = self.journal_flushes.load(Ordering::SeqCst);
        OPENINGS.with(|openings| {
            openings.borrow_mut().push(Opening {
                context: context.clone(),
                plan: plan.clone(),
                plan_bytes,
                plan_event_journaled,
                flushes,
            });
        });
        let answer = BUDGET_ANSWER.with(Cell::get);
        if answer == BudgetAnswer::Refused {
            return Err(RuntimePortFailure::Invalid);
        }
        let mut state = ResearchBudgetState {
            plan: plan.clone(),
            scope: scope.clone(),
            progress: ResearchBudgetProgress {
                started_epoch_ms: 1,
                last_epoch_ms: 1,
                queries: 0,
                visits: 0,
                reserved_bytes: 0,
                cancelled: false,
                deadline_exhausted: false,
            },
            revision: 0,
            head_sha256: SHA.to_owned(),
            remaining_revisions: 127,
        };
        match answer {
            BudgetAnswer::OtherPlan => state.plan.payload_sha256 = sha256(b"another plan"),
            BudgetAnswer::OtherScope => {
                state.scope = PreparedResearchPlan::decode(&plan_bytes_for(
                    ResearchNetworkMode::Ask,
                    "another public query",
                    true,
                ))
                .unwrap()
                .scope()
                .clone();
            }
            BudgetAnswer::Spent => state.progress.visits = 1,
            BudgetAnswer::SpentQueries => state.progress.queries = 1,
            BudgetAnswer::SpentBytes => state.progress.reserved_bytes = 1,
            BudgetAnswer::Cancelled => state.progress.cancelled = true,
            BudgetAnswer::DeadlineExhausted => state.progress.deadline_exhausted = true,
            BudgetAnswer::LaterRevision => state.revision = 1,
            BudgetAnswer::Exact | BudgetAnswer::Refused => {}
        }
        Ok(state)
    }

    fn cancel_research_budget(
        &mut self,
        _request: &RuntimeRunRequest,
        context: &ResearchBudgetContext,
    ) -> Result<ResearchBudgetState, RuntimePortFailure> {
        // The owner sees where the run's journal stood when it was asked.
        let journal = self.journal.lock().unwrap();
        let cancellation = Cancellation {
            context: context.clone(),
            after_observation: journal
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::CancellationObserved { .. })),
            before_terminal: !journal
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::RunTerminal { .. })),
        };
        drop(journal);
        CANCELLATIONS.with(|cancellations| cancellations.borrow_mut().push(cancellation));
        let answer = CANCEL_ANSWER.with(Cell::get);
        if answer == CancelAnswer::Refused {
            return Err(RuntimePortFailure::Uncertain);
        }
        // The budget the owner opened, now cancelled, with nothing spent.
        let (plan, scope) = OPENINGS.with(|openings| {
            let openings = openings.borrow();
            let opening = openings.last().unwrap();
            (
                opening.plan.clone(),
                PreparedResearchPlan::decode(&opening.plan_bytes)
                    .unwrap()
                    .scope()
                    .clone(),
            )
        });
        let mut state = ResearchBudgetState {
            plan,
            scope,
            progress: ResearchBudgetProgress {
                started_epoch_ms: 1,
                last_epoch_ms: 1,
                queries: 0,
                visits: 0,
                reserved_bytes: 0,
                cancelled: true,
                deadline_exhausted: false,
            },
            revision: 1,
            head_sha256: SHA.to_owned(),
            remaining_revisions: 126,
        };
        match answer {
            CancelAnswer::NotCancelled => state.progress.cancelled = false,
            CancelAnswer::OtherPlan => state.plan.payload_sha256 = sha256(b"another plan"),
            CancelAnswer::OtherScope => {
                state.scope = PreparedResearchPlan::decode(&plan_bytes_for(
                    ResearchNetworkMode::Ask,
                    "another public query",
                    true,
                ))
                .unwrap()
                .scope()
                .clone();
            }
            CancelAnswer::Exact | CancelAnswer::Refused => {}
        }
        Ok(state)
    }

    fn publish_research_report(
        &mut self,
        _request: &RuntimeRunRequest,
        context: &ResearchBudgetContext,
        manifest: &RuntimeArtifactManifest,
        draft: &[u8],
    ) -> Result<RuntimeArtifactRef, RuntimePortFailure> {
        // The owner sees how far the run's journal was made durable.
        let journaled = self.journal.lock().unwrap().len();
        REPORTS.with(|reports| {
            reports.borrow_mut().push(ReportPublication {
                context: context.clone(),
                manifest: manifest.clone(),
                draft: draft.to_vec(),
                flushes: self.journal_flushes.load(Ordering::SeqCst),
                journaled,
            });
        });
        match REPORT_ANSWER.with(Cell::get) {
            ReportAnswer::Refused => Err(RuntimePortFailure::Invalid),
            ReportAnswer::Uncertain => Err(RuntimePortFailure::Uncertain),
            answer => {
                let mut reference = runtime_artifact_ref(manifest).unwrap();
                if answer == ReportAnswer::OtherReference {
                    reference.manifest_sha256 = sha256(b"another manifest");
                }
                self.artifacts
                    .lock()
                    .unwrap()
                    .push((manifest.clone(), draft.to_vec()));
                Ok(reference)
            }
        }
    }
}

fn reset_budget(answer: BudgetAnswer) {
    BUDGET_ANSWER.with(|cell| cell.set(answer));
    OPENINGS.with(|openings| openings.borrow_mut().clear());
    CANCEL_ANSWER.with(|cell| cell.set(CancelAnswer::Exact));
    CANCELLATIONS.with(|cancellations| cancellations.borrow_mut().clear());
    REPORT_ANSWER.with(|cell| cell.set(ReportAnswer::Exact));
    REPORTS.with(|reports| reports.borrow_mut().clear());
}

fn reports() -> usize {
    REPORTS.with(|reports| reports.borrow().len())
}

fn cancellations() -> usize {
    CANCELLATIONS.with(|cancellations| cancellations.borrow().len())
}

fn openings() -> usize {
    OPENINGS.with(|openings| openings.borrow().len())
}

/// Prepares the six-member shape of a public GET completion (Decision 0097)
/// through the borrowed builder: four sources, the result and the bundle, in
/// three appends under the call's receipt. A nonzero mutation breaks one rule
/// of the admitted shape while keeping the builder's own checks satisfied.
pub(super) fn prepare_public_get(
    builder: &mut dyn RuntimeToolTerminalBuilder,
    execution: &mut RuntimeToolExecution,
    mutation: u8,
) -> Result<(), RuntimePortFailure> {
    let receipt = (
        execution.receipt_id.clone(),
        execution.receipt_sha256.clone(),
    );
    let mut sources: Vec<_> = [
        b"synthetic call".as_slice(),
        b"synthetic packet",
        b"synthetic material",
        b"synthetic frame",
    ]
    .into_iter()
    .map(artifact_preparation_tests::candidate)
    .collect();
    if mutation == 1 {
        sources[3].kind = RuntimeArtifactKind::StandardOutput;
    }
    let source_refs = builder.prepare_artifacts(&receipt.0, &receipt.1, &sources)?;
    let mut candidates = sources;
    if mutation != 2 {
        let result =
            artifact_preparation_tests::candidate(&serde_json::to_vec(&source_refs).unwrap());
        builder.prepare_artifacts(&receipt.0, &receipt.1, std::slice::from_ref(&result))?;
        candidates.push(result);
    }
    let bundle =
        artifact_preparation_tests::candidate(&serde_json::to_vec(&source_refs[0]).unwrap());
    let bundle_refs =
        builder.prepare_artifacts(&receipt.0, &receipt.1, std::slice::from_ref(&bundle))?;
    candidates.push(bundle);
    let output = serde_json::to_vec(&bundle_refs[0]).unwrap();
    let mut output = payload("runtime.tool-result", &output);
    output.media_type = "application/json".into();
    execution.result.output = Some(output);
    execution.result_output_kind = Some(if mutation == 3 {
        RuntimeArtifactKind::StandardOutput
    } else {
        RuntimeArtifactKind::Report
    });
    execution.artifact_candidates = candidates;
    Ok(())
}

fn plan_bytes_for(network: ResearchNetworkMode, query: &str, endpoint: bool) -> Vec<u8> {
    plan_bytes_with(network, query, endpoint, 30)
}

fn plan_bytes_with(
    network: ResearchNetworkMode,
    query: &str,
    endpoint: bool,
    recency_days: u16,
) -> Vec<u8> {
    let draft = ResearchPlanDraft {
        schema_version: 1 + u16::from(endpoint),
        task_id: "task-0001".into(),
        depth: ResearchDepth::Quick,
        network_mode: network,
        limits: ResearchLimits::ceiling(ResearchDepth::Quick),
        destination_domains: BTreeSet::from([
            "docs.example.com".into(),
            "search.example.com".into(),
        ]),
        queries: vec![PublicSearchRequest {
            request_id: "query-1".into(),
            query: query.into(),
            domains: vec!["docs.example.com".into()],
            recency_days,
            source_types: vec![PublicSourceType::PrimaryDocumentation],
            max_results: 5,
            max_total_bytes: 1024,
        }],
        search_endpoint: endpoint.then(|| PublicSearchEndpoint {
            domain: "search.example.com".into(),
            path: "/search".into(),
            query_field: "q".into(),
            fixed_fields: vec![],
        }),
    };
    serde_json::to_vec(PreparedResearchPlan::prepare(draft).unwrap().draft()).unwrap()
}

fn plan_bytes() -> Vec<u8> {
    plan_bytes_for(
        ResearchNetworkMode::TaskAuthorized,
        "public Rust documentation",
        true,
    )
}

fn fixture_tool(id: &str, operation: GrantOperation) -> FixtureTool {
    FixtureTool {
        correctable: true,
        definition: ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw(id),
            tool_version: "1.0.0".to_owned(),
            display_name: "Fixture tool".to_owned(),
            description: "One deterministic fixture tool".to_owned(),
            input_schema: schema("fixture.input"),
            output_schema: schema("fixture.output"),
            risk_level: ToolRiskLevel::Low,
            declared_effects: vec![OperationBinding::new(operation)],
            required_grant: RequiredGrantTemplate {
                operation: OperationBinding::new(operation),
                target_scope: "fixture.txt".to_owned(),
                single_use: true,
            },
            timeout_ms: 1_000,
        },
    }
}

/// The network tool the model calls, `fixture.read`, plus any other tools.
fn registry_with(others: &[(&str, GrantOperation)]) -> ToolRegistry {
    let mut registry = registry_for_operation(GrantOperation::NetworkAccess);
    for (id, operation) in others {
        registry
            .register_tool(Box::new(fixture_tool(id, *operation)))
            .unwrap();
    }
    registry
}

/// The registry's own network tool, `fixture.read`.
fn admitted_tool() -> ToolDefinition {
    registry_for_operation(GrantOperation::NetworkAccess).list_tools()[0].clone()
}

fn context_of(request: &RuntimeRunRequest) -> ResearchBudgetContext {
    ResearchBudgetContext {
        session_id: request.session_id.clone(),
        task_id: request.task.task_id.clone(),
        run_id: request.run_id.clone(),
        policy_sha256: request.policy_sha256.clone(),
    }
}

fn boundary(script: PermissionScript, executions: &Arc<AtomicUsize>) -> FakeToolBoundary {
    FakeToolBoundary {
        script,
        executions: Arc::clone(executions),
        emit_evidence: true,
        outcome: OperationOutcome::Succeeded,
        state_change: StateChange::Changed,
        tool_output_bytes: 0,
        tool_output_kind: Some(RuntimeArtifactKind::Report),
        artifact_candidates: Vec::new(),
        journal: Arc::new(Mutex::new(Vec::new())),
        journal_flushes: Arc::new(AtomicUsize::new(0)),
        artifacts: Arc::new(Mutex::new(Vec::new())),
        checkpoint: Arc::new(Mutex::new(None)),
    }
}

/// Every input of one admitted run, before composition.
struct Parts {
    request: RuntimeRunRequest,
    registry: ToolRegistry,
    admission: RuntimeResearchAdmission,
    boundary: FakeToolBoundary,
    profile: ExactModelProfile,
    executions: Arc<AtomicUsize>,
}

fn parts(script: PermissionScript, registry: ToolRegistry) -> Parts {
    let profile = profile("runtime-loop-research");
    let mut request = request(profile.clone(), &registry);
    request.mode = RuntimeSessionMode::DurableReadOnly;
    let admission =
        RuntimeResearchAdmission::new(&plan_bytes(), context_of(&request), &admitted_tool())
            .unwrap();
    request.task.constraints.push(admission.constraint());
    let executions = Arc::new(AtomicUsize::new(0));
    Parts {
        request,
        registry,
        admission,
        boundary: boundary(script, &executions),
        profile,
        executions,
    }
}

/// An admitted coordinator over the fixture's ports with any model.
type AdmittedCoordinator<M> =
    ReusableRuntimeCoordinator<M, FakeContext, FakeToolBoundary, FakeVerifier, FakeClock>;

fn compose(parts: Parts) -> Result<FixtureCoordinator, RuntimeLoopError> {
    compose_with(parts, |profile| {
        FakeModel::new(profile, [ModelScript::Tool, ModelScript::Completion])
    })
}

fn compose_with<M: RuntimeModelPort>(
    parts: Parts,
    model: impl FnOnce(ExactModelProfile) -> M,
) -> Result<AdmittedCoordinator<M>, RuntimeLoopError> {
    let mut request = parts.request;
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).unwrap();
    ReusableRuntimeCoordinator::new_with_research_admission(
        request,
        model(parts.profile),
        FakeContext,
        parts.registry,
        parts.boundary,
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-research-0001"),
            source: VerifierSource::DeterministicPostcondition,
        },
        FakeClock { now: 9_000 },
        parts.admission,
    )
}

fn admitted_run(script: PermissionScript) -> (FixtureCoordinator, Arc<AtomicUsize>) {
    let parts = parts(script, registry_with(&[]));
    let executions = Arc::clone(&parts.executions);
    (compose(parts).unwrap(), executions)
}

#[test]
fn an_admitted_run_publishes_its_plan_opens_its_budget_and_accepts_the_public_get_completion() {
    reset_budget(BudgetAnswer::Exact);
    let (mut runtime, executions) = admitted_run(PermissionScript::PreparePublicGet(0));
    let admission_context = runtime
        .research
        .as_ref()
        .unwrap()
        .admission
        .context()
        .clone();
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("admitted completion expected");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(executions.load(Ordering::SeqCst), 1);

    // The plan is the run's second run-level artifact, after the request: a
    // JSON report with no turn, operation or receipt, as the owner requires.
    let artifacts = runtime.tool_boundary.artifacts.lock().unwrap();
    let (plan_manifest, published) = &artifacts[1];
    assert_eq!(published, &plan_bytes());
    assert_eq!(plan_manifest.kind, RuntimeArtifactKind::Report);
    assert_eq!(plan_manifest.media_type, "application/json");
    assert!(plan_manifest.producer_turn_id.is_none());
    assert!(plan_manifest.producer_operation_id.is_none());
    assert!(plan_manifest.receipt_id.is_none());
    // The owner opened the budget once, for exactly that artifact, after its
    // creation event was journaled, under the admission's own context.
    OPENINGS.with(|openings| {
        let openings = openings.borrow();
        assert_eq!(openings.len(), 1);
        assert_eq!(
            openings[0].plan,
            runtime_artifact_ref(plan_manifest).unwrap()
        );
        assert_eq!(openings[0].plan_bytes, plan_bytes());
        assert!(openings[0].plan_event_journaled);
        // The journal was flushed before the request artifact and again
        // after the plan's creation event, before the owner read it.
        assert_eq!(openings[0].flushes, 2);
        assert_eq!(openings[0].context, admission_context);
    });
    // The plan precedes the first turn; the completion's six artifacts are
    // published under the call's receipt after its terminal.
    let events = runtime.events();
    let plan_event = events
        .iter()
        .position(|event| {
            matches!(&event.kind, RuntimeEventKind::ArtifactCreated { artifact_id, .. }
            if artifact_id == &plan_manifest.artifact_id)
        })
        .unwrap();
    let first_turn = events
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::TurnStarted))
        .unwrap();
    assert!(plan_event < first_turn);
    assert_eq!(
        artifacts
            .iter()
            .filter(|(manifest, _)| manifest.receipt_id.is_some())
            .count(),
        6
    );
    drop(artifacts);
    assert_valid_terminal_stream(&runtime);
}

#[test]
fn an_admission_binds_its_plan_context_and_tool_and_refuses_what_it_cannot_bind() {
    let request = request(profile("runtime-loop-research"), &registry_with(&[]));
    let context = context_of(&request);
    let admission =
        RuntimeResearchAdmission::new(&plan_bytes(), context.clone(), &admitted_tool()).unwrap();
    assert_eq!(
        admission.constraint(),
        format!(
            "{RESEARCH_ADMISSION_CONSTRAINT_PREFIX}{}",
            admission.digest()
        )
    );
    assert!(!format!("{admission:?}").contains("public Rust documentation"));

    // Each bound input changes the digest, a plan that differs only outside
    // its restrictions included.
    let recency = plan_bytes_with(
        ResearchNetworkMode::TaskAuthorized,
        "public Rust documentation",
        true,
        7,
    );
    assert_eq!(
        PreparedResearchPlan::decode(&recency)
            .unwrap()
            .scope()
            .policy_sha256(),
        PreparedResearchPlan::decode(&plan_bytes())
            .unwrap()
            .scope()
            .policy_sha256()
    );
    let mut other_run = context.clone();
    other_run.run_id = RuntimeRunId::from_raw("runtime-run-other");
    let mut other_session = context.clone();
    other_session.session_id = SessionId::from_raw("session-other");
    let mut other_policy = context.clone();
    other_policy.policy_sha256 = sha256(b"another policy");
    let mut other_version = admitted_tool();
    other_version.tool_version = "1.0.1".to_owned();
    for other in [
        RuntimeResearchAdmission::new(
            &plan_bytes_for(ResearchNetworkMode::TaskAuthorized, "another query", true),
            context.clone(),
            &admitted_tool(),
        ),
        RuntimeResearchAdmission::new(
            &plan_bytes_for(ResearchNetworkMode::Ask, "public Rust documentation", true),
            context.clone(),
            &admitted_tool(),
        ),
        RuntimeResearchAdmission::new(&recency, context.clone(), &admitted_tool()),
        RuntimeResearchAdmission::new(&plan_bytes(), other_run, &admitted_tool()),
        RuntimeResearchAdmission::new(&plan_bytes(), other_session, &admitted_tool()),
        RuntimeResearchAdmission::new(&plan_bytes(), other_policy, &admitted_tool()),
        RuntimeResearchAdmission::new(&plan_bytes(), context.clone(), &other_version),
    ] {
        assert_ne!(other.unwrap().digest(), admission.digest());
    }

    let mut pretty =
        serde_json::to_vec_pretty(PreparedResearchPlan::decode(&plan_bytes()).unwrap().draft())
            .unwrap();
    pretty.push(b'\n');
    let mut other_task = context.clone();
    other_task.task_id = TaskId::from_raw("task-other");
    let mut bad_policy = context.clone();
    bad_policy.policy_sha256 = "A".repeat(64);
    let mut bad_session = context.clone();
    bad_session.session_id = SessionId::from_raw("bad session");
    let mut bad_run = context.clone();
    bad_run.run_id = RuntimeRunId::from_raw("bad run");
    let mut read_tool = admitted_tool();
    read_tool.required_grant.operation = OperationBinding::new(GrantOperation::WorkspaceRead);
    read_tool.declared_effects = vec![OperationBinding::new(GrantOperation::WorkspaceRead)];
    let mut two_effects = admitted_tool();
    two_effects
        .declared_effects
        .push(OperationBinding::new(GrantOperation::WorkspaceRead));
    let mut reusable = admitted_tool();
    reusable.required_grant.single_use = false;
    let mut read_grant = admitted_tool();
    read_grant.required_grant.operation = OperationBinding::new(GrantOperation::WorkspaceRead);
    for (bytes, context, tool) in [
        (b"{}".to_vec(), context.clone(), admitted_tool()),
        (pretty, context.clone(), admitted_tool()),
        (plan_bytes(), other_task, admitted_tool()),
        (plan_bytes(), bad_policy, admitted_tool()),
        (plan_bytes(), bad_session, admitted_tool()),
        (plan_bytes(), bad_run, admitted_tool()),
        (
            plan_bytes_for(ResearchNetworkMode::TaskAuthorized, "a query", false),
            context.clone(),
            admitted_tool(),
        ),
        (
            plan_bytes_for(ResearchNetworkMode::Offline, "a query", true),
            context.clone(),
            admitted_tool(),
        ),
        (plan_bytes(), context.clone(), read_tool),
        (plan_bytes(), context.clone(), two_effects),
        (plan_bytes(), context.clone(), reusable),
        (plan_bytes(), context.clone(), read_grant),
    ] {
        assert_eq!(
            RuntimeResearchAdmission::new(&bytes, context, &tool).err(),
            Some(RuntimeLoopError::UnsupportedMode)
        );
    }
}

#[test]
fn composition_refuses_a_run_that_does_not_name_or_fit_its_admission() {
    reset_budget(BudgetAnswer::Exact);
    let cursor = |request: &mut RuntimeRunRequest| {
        request.event_cursor = Some(RuntimeEventCursor {
            run_id: request.run_id.clone(),
            event_id: RuntimeEventId::from_raw("event-0001"),
            sequence: 0,
            event_sha256: SHA.to_owned(),
        });
    };
    type Change = Box<dyn Fn(&mut Parts)>;
    let cases: Vec<(&str, Change)> = vec![
        (
            "no line",
            Box::new(|parts| {
                parts.request.task.constraints.pop();
            }),
        ),
        (
            "two lines",
            Box::new(|parts| {
                let line = parts.admission.constraint();
                parts.request.task.constraints.push(line);
            }),
        ),
        (
            "another digest",
            Box::new(|parts| {
                *parts.request.task.constraints.last_mut().unwrap() =
                    format!("{RESEARCH_ADMISSION_CONSTRAINT_PREFIX}{}", "b".repeat(64));
            }),
        ),
        (
            "ephemeral",
            Box::new(|parts| {
                parts.request.mode = RuntimeSessionMode::EphemeralReadOnly;
            }),
        ),
        ("resume", Box::new(move |parts| cursor(&mut parts.request))),
        // The line names this admission exactly, but the request belongs to
        // another task or session than the admission's context (review F3
        // of `7611c8bf`).
        (
            "another task",
            Box::new(|parts| {
                parts.request.task.task_id = TaskId::from_raw("task-other");
                parts.request.work_packet.task_id = TaskId::from_raw("task-other");
            }),
        ),
        (
            "another session",
            Box::new(|parts| {
                parts.request.session_id = SessionId::from_raw("session-other");
                parts.request.task.session_id = SessionId::from_raw("session-other");
            }),
        ),
        (
            "another run",
            Box::new(|parts| {
                let mut context = parts.admission.context().clone();
                context.run_id = RuntimeRunId::from_raw("runtime-run-other");
                parts.admission =
                    RuntimeResearchAdmission::new(&plan_bytes(), context, &admitted_tool())
                        .unwrap();
                *parts.request.task.constraints.last_mut().unwrap() = parts.admission.constraint();
            }),
        ),
        (
            "another policy",
            Box::new(|parts| {
                let mut context = parts.admission.context().clone();
                context.policy_sha256 = sha256(b"another policy");
                parts.admission =
                    RuntimeResearchAdmission::new(&plan_bytes(), context, &admitted_tool())
                        .unwrap();
                *parts.request.task.constraints.last_mut().unwrap() = parts.admission.constraint();
            }),
        ),
    ];
    for (name, change) in cases {
        let mut parts = parts(PermissionScript::PreparePublicGet(0), registry_with(&[]));
        change(&mut parts);
        let executions = Arc::clone(&parts.executions);
        assert_eq!(
            compose(parts).err(),
            Some(RuntimeLoopError::UnsupportedMode),
            "{name}"
        );
        assert_eq!(executions.load(Ordering::SeqCst), 0);
    }
    // A controlled-write run is never admitted, whichever check refuses it.
    let mut controlled = parts(PermissionScript::PreparePublicGet(0), registry_with(&[]));
    controlled.request.mode = RuntimeSessionMode::ControlledWrite;
    assert!(compose(controlled).is_err());

    // The catalog holds the admitted tool once and nothing else that is not
    // ordinary for the mode; another read tool is fine.
    let read_only = parts(
        PermissionScript::PreparePublicGet(0),
        registry_for_operation(GrantOperation::WorkspaceRead),
    );
    assert_eq!(
        compose(read_only).err(),
        Some(RuntimeLoopError::ToolCatalogBinding)
    );
    for (others, admitted) in [
        (
            &[("fixture.fetch", GrantOperation::NetworkAccess)][..],
            false,
        ),
        (
            &[("fixture.write", GrantOperation::WorkspaceWrite)][..],
            false,
        ),
        (
            &[("fixture.other", GrantOperation::WorkspaceRead)][..],
            true,
        ),
    ] {
        let parts = parts(PermissionScript::PreparePublicGet(0), registry_with(others));
        assert_eq!(parts.request.visible_tools.len(), 2);
        let composed = compose(parts);
        if admitted {
            assert!(composed.is_ok());
        } else {
            assert_eq!(composed.err(), Some(RuntimeLoopError::ToolCatalogBinding));
        }
    }
    assert_eq!(openings(), 0);
}

#[test]
fn ordinary_constructors_refuse_a_request_that_names_an_admission() {
    for registry in [
        registry_for_operation(GrantOperation::WorkspaceRead),
        registry_with(&[]),
    ] {
        let parts = parts(PermissionScript::PreparePublicGet(0), registry);
        let mut request = parts.request;
        request.request_sha256 = "0".repeat(64);
        let request = seal_runtime_run_request(request).unwrap();
        let composed = ReusableRuntimeCoordinator::new_with_durable_state(
            request,
            FakeModel::new(parts.profile, [ModelScript::Tool, ModelScript::Completion]),
            FakeContext,
            parts.registry,
            parts.boundary,
            FakeVerifier {
                verifier_id: VerifierId::from_raw("verifier-research-0001"),
                source: VerifierSource::DeterministicPostcondition,
            },
            FakeClock { now: 9_000 },
        );
        assert_eq!(composed.err(), Some(RuntimeLoopError::UnsupportedMode));
    }
}

#[test]
fn the_run_continues_only_when_the_owner_opens_exactly_the_published_plan() {
    for answer in [
        BudgetAnswer::OtherPlan,
        BudgetAnswer::OtherScope,
        BudgetAnswer::Spent,
        BudgetAnswer::SpentQueries,
        BudgetAnswer::SpentBytes,
        BudgetAnswer::Cancelled,
        BudgetAnswer::DeadlineExhausted,
        BudgetAnswer::LaterRevision,
        BudgetAnswer::Refused,
    ] {
        reset_budget(answer);
        let (mut runtime, executions) = admitted_run(PermissionScript::PreparePublicGet(0));
        let refused = runtime.run_until_boundary(None, None).err();
        let expected = if answer == BudgetAnswer::Refused {
            RuntimeLoopError::Dependency(RuntimePortFailure::Invalid)
        } else {
            RuntimeLoopError::InvalidBoundaryResult
        };
        assert_eq!(refused, Some(expected), "{answer:?}");
        assert_eq!(openings(), 1, "{answer:?}");
        assert_eq!(executions.load(Ordering::SeqCst), 0, "{answer:?}");
        assert!(
            !runtime
                .events()
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::TurnStarted)),
            "{answer:?}"
        );
    }
}

#[test]
fn an_admitted_success_must_be_the_prepared_public_get_completion() {
    // Two prepared artifacts, a non-report artifact, five artifacts and a
    // non-report output are each refused; so are six unprepared artifacts.
    for script in [
        PermissionScript::PrepareArtifacts(0),
        PermissionScript::PreparePublicGet(1),
        PermissionScript::PreparePublicGet(2),
        PermissionScript::PreparePublicGet(3),
        PermissionScript::Allow,
    ] {
        reset_budget(BudgetAnswer::Exact);
        let (mut runtime, executions) = admitted_run(script);
        if matches!(script, PermissionScript::Allow) {
            runtime.tool_boundary.artifact_candidates = (0..6)
                .map(|index| artifact_preparation_tests::candidate(&[b'0' + index]))
                .collect();
        }
        // The borrowed builder refuses to seal it, or the coordinator refuses
        // what the port returned.
        let refused = runtime.run_until_boundary(None, None).err();
        assert!(
            matches!(
                refused,
                Some(
                    RuntimeLoopError::Dependency(RuntimePortFailure::Invalid)
                        | RuntimeLoopError::InvalidBoundaryResult
                )
            ),
            "{refused:?}"
        );
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert!(runtime.tool_results.is_empty());
        assert!(
            runtime
                .tool_boundary
                .artifacts
                .lock()
                .unwrap()
                .iter()
                .all(|(manifest, _)| manifest.receipt_id.is_none())
        );
        assert_callback_failure_stays_latched(&mut runtime, &executions);
    }
}

#[test]
fn an_admitted_failure_that_changed_nothing_is_accepted() {
    for outcome in [
        OperationOutcome::Failed,
        OperationOutcome::Denied,
        OperationOutcome::TimedOut,
        OperationOutcome::Uncertain,
    ] {
        reset_budget(BudgetAnswer::Exact);
        let (mut runtime, executions) = admitted_run(PermissionScript::Allow);
        runtime.tool_boundary.outcome = outcome;
        assert_ne!(
            runtime.run_until_boundary(None, None).err(),
            Some(RuntimeLoopError::InvalidBoundaryResult),
            "{outcome:?}"
        );
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert!(
            runtime
                .events()
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::ToolFailed { .. })),
            "{outcome:?}"
        );
    }
}

#[test]
fn an_admitted_attempt_that_did_not_succeed_retains_no_output_or_artifacts() {
    // Review F2 of `7611c8bf`: the coordinator refuses an uncertain or failed
    // attempt that carries output or artifact candidates, and publishes
    // nothing under its receipt.
    for (outcome, output, candidates) in [
        (OperationOutcome::Uncertain, true, 0),
        (OperationOutcome::Uncertain, false, 3),
        (OperationOutcome::Failed, true, 0),
        (OperationOutcome::Cancelled, false, 1),
    ] {
        reset_budget(BudgetAnswer::Exact);
        let (mut runtime, executions) = admitted_run(PermissionScript::Allow);
        runtime.tool_boundary.outcome = outcome;
        runtime.tool_boundary.emit_evidence = false;
        runtime.tool_boundary.tool_output_bytes = if output { 16 } else { 0 };
        runtime.tool_boundary.artifact_candidates = (0..candidates)
            .map(|index| artifact_preparation_tests::candidate(&[b'0' + index]))
            .collect();
        // The borrowed builder refuses to seal it, or the coordinator refuses
        // what the port returned.
        let refused = runtime.run_until_boundary(None, None).err();
        assert!(
            matches!(
                refused,
                Some(
                    RuntimeLoopError::Dependency(RuntimePortFailure::Invalid)
                        | RuntimeLoopError::InvalidBoundaryResult
                )
            ),
            "{outcome:?} {output} {candidates} {refused:?}"
        );
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert!(runtime.tool_results.is_empty());
        let artifacts = runtime.tool_boundary.artifacts.lock().unwrap();
        assert!(
            artifacts
                .iter()
                .all(|(manifest, _)| manifest.receipt_id.is_none()),
            "{outcome:?} {output} {candidates}"
        );
        // Only the request and the plan were published.
        assert_eq!(artifacts.len(), 2);
    }
}

#[test]
fn the_admitted_tool_returns_only_these_network_results() {
    let execution =
        |outcome, state_change, candidates: usize, kind: Option<RuntimeArtifactKind>| {
            RuntimeToolExecution {
                receipt_id: ReceiptId::from_raw("receipt-1"),
                receipt_sha256: SHA.to_owned(),
                result: ToolResult {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    tool_call_id: agentmage_kernel_contracts::ToolCallId::from_raw("call-1"),
                    correlation_id: CorrelationId::from_raw("correlation-1"),
                    outcome,
                    output: kind.map(|_| payload("runtime.tool-result", b"{}")),
                    validation_issues: Vec::new(),
                    evidence: Vec::new(),
                    error: None,
                    elapsed_ms: 1,
                    state_change,
                },
                result_output_kind: kind,
                artifact_candidates: (0..candidates)
                    .map(|_| artifact_preparation_tests::candidate(b"x"))
                    .collect(),
            }
        };
    let report = Some(RuntimeArtifactKind::Report);
    for (outcome, state_change, candidates, kind, accepted) in [
        (
            OperationOutcome::Succeeded,
            StateChange::Changed,
            6,
            report,
            true,
        ),
        (
            OperationOutcome::Succeeded,
            StateChange::Changed,
            5,
            report,
            false,
        ),
        (
            OperationOutcome::Succeeded,
            StateChange::Changed,
            7,
            report,
            false,
        ),
        (
            OperationOutcome::Succeeded,
            StateChange::Changed,
            6,
            None,
            false,
        ),
        (
            OperationOutcome::Succeeded,
            StateChange::Changed,
            6,
            Some(RuntimeArtifactKind::StandardOutput),
            false,
        ),
        (
            OperationOutcome::Succeeded,
            StateChange::NotChanged,
            6,
            report,
            false,
        ),
        (
            OperationOutcome::Succeeded,
            StateChange::Uncertain,
            6,
            report,
            false,
        ),
        (
            OperationOutcome::Uncertain,
            StateChange::Uncertain,
            0,
            None,
            true,
        ),
        (
            OperationOutcome::Uncertain,
            StateChange::NotChanged,
            0,
            None,
            false,
        ),
        (
            OperationOutcome::Uncertain,
            StateChange::Changed,
            0,
            None,
            false,
        ),
        (
            OperationOutcome::Failed,
            StateChange::NotChanged,
            0,
            None,
            true,
        ),
        (
            OperationOutcome::Denied,
            StateChange::NotChanged,
            0,
            None,
            true,
        ),
        (
            OperationOutcome::Cancelled,
            StateChange::NotChanged,
            0,
            None,
            true,
        ),
        (
            OperationOutcome::TimedOut,
            StateChange::NotChanged,
            0,
            None,
            true,
        ),
        (
            OperationOutcome::Failed,
            StateChange::Changed,
            0,
            None,
            false,
        ),
        (
            OperationOutcome::Cancelled,
            StateChange::Uncertain,
            0,
            None,
            false,
        ),
    ] {
        assert_eq!(
            valid_admitted_network_execution(&execution(outcome, state_change, candidates, kind)),
            accepted,
            "{outcome:?} {state_change:?} {candidates} {kind:?}"
        );
    }
    let mut non_report = execution(OperationOutcome::Succeeded, StateChange::Changed, 6, report);
    non_report.artifact_candidates[2].kind = RuntimeArtifactKind::StandardError;
    assert!(!valid_admitted_network_execution(&non_report));
    // Only a success carries output, evidence or artifacts: an uncertain or
    // failed attempt has no trusted source bytes to retain (review F2 of
    // `7611c8bf`).
    for (outcome, state_change) in [
        (OperationOutcome::Uncertain, StateChange::Uncertain),
        (OperationOutcome::Failed, StateChange::NotChanged),
        (OperationOutcome::Denied, StateChange::NotChanged),
        (OperationOutcome::Cancelled, StateChange::NotChanged),
        (OperationOutcome::TimedOut, StateChange::NotChanged),
    ] {
        assert!(valid_admitted_network_execution(&execution(
            outcome,
            state_change,
            0,
            None
        )));
        for (candidates, kind) in [(0, report), (1, None), (6, None), (6, report)] {
            assert!(
                !valid_admitted_network_execution(&execution(
                    outcome,
                    state_change,
                    candidates,
                    kind
                )),
                "{outcome:?} {candidates} {kind:?}"
            );
        }
        let mut kind_without_output = execution(outcome, state_change, 0, report);
        kind_without_output.result.output = None;
        assert!(!valid_admitted_network_execution(&kind_without_output));
        let mut output_without_kind = execution(outcome, state_change, 0, None);
        output_without_kind.result.output = Some(payload("runtime.tool-result", b"{}"));
        assert!(!valid_admitted_network_execution(&output_without_kind));
        let mut with_evidence = execution(outcome, state_change, 0, None);
        with_evidence.result.evidence = vec![evidence("evidence-1", EvidenceKind::ToolOutput)];
        assert!(
            !valid_admitted_network_execution(&with_evidence),
            "{outcome:?} evidence"
        );
    }
}

/// A user's cancellation of the admitted run's own task.
fn cancellation_of<M: RuntimeModelPort>(runtime: &AdmittedCoordinator<M>) -> CancellationSignal {
    CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("research-cancellation-0001"),
        correlation_id: runtime.correlation_id.clone(),
        task_id: runtime.request.task.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    }
}

/// A cancellation the probe reports only after the tool was executed.
struct CancelAfterExecution {
    executions: Arc<AtomicUsize>,
    signal: CancellationSignal,
}

impl agentmage_kernel_contracts::ModelCancellationProbe for CancelAfterExecution {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok((self.executions.load(Ordering::SeqCst) > 0).then(|| self.signal.clone()))
    }
}

#[test]
fn a_cancelled_admitted_run_cancels_its_budget_before_its_outcome() {
    // Decision 0140: between phases, and when the cancelled tool's outcome
    // observes the cancellation.
    for during_tool in [false, true] {
        reset_budget(BudgetAnswer::Exact);
        let (mut runtime, executions) = admitted_run(PermissionScript::Allow);
        runtime.tool_boundary.outcome = OperationOutcome::Cancelled;
        runtime.tool_boundary.emit_evidence = false;
        let context = runtime
            .research
            .as_ref()
            .unwrap()
            .admission
            .context()
            .clone();
        // Between phases the probe reports the cancellation at once; during
        // the tool it follows the fixture's own execution count.
        let probe = CancelAfterExecution {
            executions: if during_tool {
                Arc::clone(&executions)
            } else {
                Arc::new(AtomicUsize::new(1))
            },
            signal: cancellation_of(&runtime),
        };
        let RuntimeCoordinatorStep::Complete { outcome } =
            runtime.run_until_boundary(None, Some(&probe)).unwrap()
        else {
            panic!("a cancelled run ends");
        };
        assert_eq!(
            executions.load(Ordering::SeqCst),
            usize::from(during_tool),
            "{during_tool}"
        );
        assert_eq!(outcome.state, AgentStateKind::Cancelled, "{during_tool}");
        assert!(
            !outcome
                .unresolved_codes
                .contains(&RESEARCH_BUDGET_CANCELLATION_UNCONFIRMED.to_owned()),
            "{during_tool}"
        );
        CANCELLATIONS.with(|cancellations| {
            let cancellations = cancellations.borrow();
            assert_eq!(cancellations.len(), 1, "{during_tool}");
            assert_eq!(cancellations[0].context, context);
            assert!(cancellations[0].after_observation);
            assert!(cancellations[0].before_terminal);
        });
        assert_valid_terminal_stream(&runtime);
    }
}

#[test]
fn a_run_that_observed_no_cancellation_leaves_its_budget_alone() {
    for outcome in [OperationOutcome::Succeeded, OperationOutcome::Failed] {
        reset_budget(BudgetAnswer::Exact);
        let script = if outcome == OperationOutcome::Succeeded {
            PermissionScript::PreparePublicGet(0)
        } else {
            PermissionScript::Allow
        };
        let (mut runtime, _) = admitted_run(script);
        runtime.tool_boundary.outcome = outcome;
        runtime.tool_boundary.emit_evidence = outcome == OperationOutcome::Succeeded;
        runtime.run_until_boundary(None, None).unwrap();
        assert!(runtime.outcome().is_some(), "{outcome:?}");
        assert_eq!(cancellations(), 0, "{outcome:?}");
    }
}

#[test]
fn an_unconfirmed_budget_cancellation_is_named_in_the_outcome() {
    for answer in [
        CancelAnswer::NotCancelled,
        CancelAnswer::OtherPlan,
        CancelAnswer::OtherScope,
        CancelAnswer::Refused,
    ] {
        reset_budget(BudgetAnswer::Exact);
        CANCEL_ANSWER.with(|cell| cell.set(answer));
        let (mut runtime, executions) = admitted_run(PermissionScript::PreparePublicGet(0));
        let signal = cancellation_of(&runtime);
        let RuntimeCoordinatorStep::Complete { outcome } =
            runtime.run_until_boundary(None, Some(&signal)).unwrap()
        else {
            panic!("the run still ends");
        };
        assert_eq!(outcome.state, AgentStateKind::Cancelled, "{answer:?}");
        assert_eq!(
            outcome.unresolved_codes,
            [
                "runtime.cancelled",
                RESEARCH_BUDGET_CANCELLATION_UNCONFIRMED
            ],
            "{answer:?}"
        );
        assert_eq!(cancellations(), 1, "{answer:?}");
        assert_eq!(executions.load(Ordering::SeqCst), 0);
        assert_valid_terminal_stream(&runtime);
    }
}

#[test]
fn a_resumed_admitted_run_is_refused_even_with_its_checkpoint() {
    // Resuming an admitted run is later work (Decision 0137): even with the
    // run's own committed checkpoint, a resumed request is refused.
    reset_budget(BudgetAnswer::Exact);
    let parts = parts(PermissionScript::PreparePublicGet(0), registry_with(&[]));
    let request = {
        let mut request = parts.request.clone();
        request.request_sha256 = "0".repeat(64);
        seal_runtime_run_request(request).unwrap()
    };
    let admission = parts.admission.clone();
    let executions = Arc::clone(&parts.executions);
    let mut first = compose(parts).unwrap();
    first.start().unwrap();
    first.run_turn(None).unwrap();
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let last = first.events().last().unwrap().clone();
    assert!(matches!(
        last.kind,
        RuntimeEventKind::CheckpointCommitted { .. }
    ));
    let mut resumed = request;
    resumed.event_cursor = Some(runtime_event_cursor(&last));
    resumed.request_sha256 = "0".repeat(64);
    let resumed = seal_runtime_run_request(resumed).unwrap();
    let composed = ReusableRuntimeCoordinator::new_with_research_admission(
        resumed,
        FakeModel::new(first.model.profile.clone(), [ModelScript::Completion]),
        FakeContext,
        first.registry,
        first.tool_boundary,
        first.verifier,
        FakeClock { now: 20_000 },
        admission,
    );
    assert_eq!(composed.err(), Some(RuntimeLoopError::UnsupportedMode));
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_eq!(openings(), 1);
}

/// A model whose dispatch observes the run's cancellation and then reports a
/// dependency failure, which outranks it (review F1 of `a8fd53e3`).
struct LatchingModel {
    profile: ExactModelProfile,
    requested: Arc<AtomicUsize>,
    failure: RuntimePortFailure,
}

impl RuntimeModelPort for LatchingModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        &self.profile
    }

    fn run_model(
        &mut self,
        _request: &ModelRunRequest,
        _context: &ModelContextPacket,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        panic!("the coordinator dispatches under run control")
    }

    fn run_model_controlled(
        &mut self,
        _request: &ModelRunRequest,
        _context: &ModelContextPacket,
        _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
        control: &dyn agentmage_kernel_contracts::ModelOperationControl,
    ) -> Result<ModelRunResult, RuntimeModelOperationFailure> {
        control
            .remaining_ms()
            .map_err(RuntimeModelOperationFailure::Stopped)?;
        self.requested.store(1, Ordering::SeqCst);
        assert!(
            control.remaining_ms().is_err(),
            "the phase observes the cancellation"
        );
        Err(RuntimeModelOperationFailure::Port(self.failure))
    }
}

#[test]
fn a_cancellation_latched_before_a_dependency_failure_cancels_the_budget() {
    // Review F1 of `a8fd53e3`, the third path of Decision 0140: the model
    // phase observes the cancellation, a dependency failure outranks it, and
    // the observation is recorded only when the terminal is sealed.
    for (failure, state) in [
        (RuntimePortFailure::Uncertain, AgentStateKind::Failed),
        (RuntimePortFailure::Unavailable, AgentStateKind::Failed),
        (
            RuntimePortFailure::ResourceExhausted,
            AgentStateKind::Exhausted,
        ),
    ] {
        reset_budget(BudgetAnswer::Exact);
        let requested = Arc::new(AtomicUsize::new(0));
        let mut runtime = compose_with(
            parts(PermissionScript::Allow, registry_with(&[])),
            |profile| LatchingModel {
                profile,
                requested: Arc::clone(&requested),
                failure,
            },
        )
        .unwrap();
        let context = runtime
            .research
            .as_ref()
            .unwrap()
            .admission
            .context()
            .clone();
        let probe = CancelAfterExecution {
            executions: Arc::clone(&requested),
            signal: cancellation_of(&runtime),
        };
        let RuntimeCoordinatorStep::Complete { outcome } =
            runtime.run_until_boundary(None, Some(&probe)).unwrap()
        else {
            panic!("the run ends");
        };
        assert_eq!(requested.load(Ordering::SeqCst), 1, "{failure:?}");
        assert_eq!(outcome.state, state, "{failure:?}");
        assert_eq!(outcome.unresolved_codes, [failure.code()], "{failure:?}");
        // The observation follows the closed turn and names no turn.
        let events = runtime.events();
        let closed = events
            .iter()
            .position(|event| matches!(event.kind, RuntimeEventKind::TurnCompleted { .. }))
            .unwrap();
        let observed = events
            .iter()
            .position(|event| matches!(event.kind, RuntimeEventKind::CancellationObserved { .. }))
            .unwrap();
        assert!(closed < observed, "{failure:?}");
        assert_eq!(events[observed].turn_id, None);
        CANCELLATIONS.with(|cancellations| {
            let cancellations = cancellations.borrow();
            assert_eq!(cancellations.len(), 1, "{failure:?}");
            assert_eq!(cancellations[0].context, context);
            assert!(cancellations[0].after_observation, "{failure:?}");
            assert!(cancellations[0].before_terminal, "{failure:?}");
        });
        assert_valid_terminal_stream(&runtime);
    }
}

/// The fixture model with its completion's payload replaced by `draft` under
/// the report draft media type and proposed as `kind` (Decision 0142). At its
/// dispatch it records how often the run's journal had been flushed.
struct ReportModel {
    inner: FakeModel,
    draft: Vec<u8>,
    kind: ModelProposalKind,
    journal_flushes: Arc<AtomicUsize>,
    flushes_at_dispatch: Arc<AtomicUsize>,
}

impl RuntimeModelPort for ReportModel {
    fn exact_profile(&self) -> &ExactModelProfile {
        self.inner.exact_profile()
    }

    fn bind_context_tokens(
        &self,
        packet: &mut ModelContextPacket,
    ) -> Result<(), RuntimePortFailure> {
        self.inner.bind_context_tokens(packet)
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        context: &ModelContextPacket,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        self.flushes_at_dispatch.store(
            self.journal_flushes.load(Ordering::SeqCst),
            Ordering::SeqCst,
        );
        let mut result = self.inner.run_model(request, context, cancellation)?;
        if let Some(proposal) = result.proposal.as_mut()
            && proposal.kind == ModelProposalKind::CompletionCandidate
        {
            let mut draft = payload("research.report-draft", &self.draft);
            draft.media_type = RESEARCH_REPORT_DRAFT_MEDIA_TYPE.to_owned();
            proposal.kind = self.kind;
            proposal.payload = Some(draft);
            proposal.proposal_sha256 = "0".repeat(64);
            proposal.proposal_sha256 =
                proposal_digest(proposal).map_err(|_| RuntimePortFailure::Invalid)?;
            result.response_sha256 = sha256(proposal.proposal_sha256.as_bytes());
        }
        Ok(result)
    }
}

/// A well-formed draft citing one source; only the owner can say whether it
/// holds. `unresolved` questions of `unresolved_bytes` each enlarge it.
fn report_draft(unresolved: usize, unresolved_bytes: usize) -> Vec<u8> {
    serde_json::to_vec(&ResearchReportDraft {
        schema_version: 1,
        report_id: "report-0001".into(),
        sources: vec![RuntimeArtifactRef {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("artifact-source-0001"),
            manifest_sha256: sha256(b"source manifest"),
            payload_sha256: sha256(b"source bundle"),
            byte_size: 512,
            media_type: "application/json".into(),
        }],
        spans: vec![ResearchSourceSpan {
            source_index: 0,
            body_sha256: sha256(b"source body"),
            start_byte: 0,
            end_byte: 6,
            excerpt: "Public".into(),
        }],
        claims: vec![ResearchReportClaimDraft::Observed {
            claim_id: "claim-1".into(),
            span_index: 0,
        }],
        conflicts: vec![],
        unresolved: (0..unresolved)
            .map(|index| format!("{index} {}", "q".repeat(unresolved_bytes)))
            .collect(),
    })
    .unwrap()
}

/// An admitted run whose model completes at once with `draft` as `kind`.
fn report_run(
    draft: Vec<u8>,
    kind: ModelProposalKind,
    max_output_bytes: u64,
) -> AdmittedCoordinator<ReportModel> {
    let mut parts = parts(PermissionScript::Allow, registry_with(&[]));
    parts.request.limits.max_output_bytes = max_output_bytes;
    let journal_flushes = Arc::clone(&parts.boundary.journal_flushes);
    compose_with(parts, |profile| ReportModel {
        inner: FakeModel::new(profile, [ModelScript::Completion]),
        draft,
        kind,
        journal_flushes,
        flushes_at_dispatch: Arc::new(AtomicUsize::new(0)),
    })
    .unwrap()
}

fn draft_artifacts<M: RuntimeModelPort>(runtime: &AdmittedCoordinator<M>) -> usize {
    runtime
        .artifact_references
        .iter()
        .filter(|reference| reference.media_type == RESEARCH_REPORT_DRAFT_MEDIA_TYPE)
        .count()
}

#[test]
fn an_admitted_run_retains_its_report_through_the_owner_before_its_outcome() {
    reset_budget(BudgetAnswer::Exact);
    let draft = report_draft(0, 0);
    let mut runtime = report_run(
        draft.clone(),
        ModelProposalKind::CompletionCandidate,
        MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64,
    );
    let context = runtime
        .research
        .as_ref()
        .unwrap()
        .admission
        .context()
        .clone();
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("the run completes");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert!(outcome.unresolved_codes.is_empty());
    assert!(outcome.answer_evidence.is_some());
    let (manifest, flushes, journaled) = REPORTS.with(|reports| {
        let reports = reports.borrow();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].context, context);
        assert_eq!(reports[0].draft, draft);
        (
            reports[0].manifest.clone(),
            reports[0].flushes,
            reports[0].journaled,
        )
    });
    // A run-level report of the completing turn, without an operation,
    // receipt or preview, created when the owner checks it.
    let events = runtime.events();
    let turn = events
        .iter()
        .rev()
        .find_map(|event| event.turn_id.clone())
        .unwrap();
    assert_eq!(manifest.media_type, RESEARCH_REPORT_DRAFT_MEDIA_TYPE);
    assert_eq!(manifest.kind, RuntimeArtifactKind::Report);
    assert_eq!(manifest.producer_turn_id, Some(turn.clone()));
    assert_eq!(manifest.producer_operation_id, None);
    assert_eq!(manifest.receipt_id, None);
    assert_eq!(manifest.preview, None);
    assert_eq!(manifest.payload_sha256, sha256(&draft));
    // The journal was flushed after the model's dispatch and before the owner
    // was asked, and the creation event follows the owner's answer.
    assert_eq!(
        flushes,
        runtime.model.flushes_at_dispatch.load(Ordering::SeqCst) + 1
    );
    let created = events
        .iter()
        .position(|event| {
            matches!(&event.kind, RuntimeEventKind::ArtifactCreated { artifact_id, manifest_sha256 }
                if artifact_id == &manifest.artifact_id && manifest_sha256 == &manifest.manifest_sha256)
        })
        .unwrap();
    assert_eq!(created, journaled);
    assert_eq!(events[created].turn_id, Some(turn));
    assert_eq!(events[created].operation_id, None);
    assert_eq!(
        events[created].occurred_at_epoch_ms,
        manifest.created_at_epoch_ms
    );
    assert_eq!(
        events[created].payload_reference,
        Some(runtime_payload_reference(&manifest).unwrap())
    );
    assert_eq!(
        outcome.output,
        Some(RuntimeOutput::Artifact {
            reference: runtime_payload_reference(&manifest).unwrap(),
        })
    );
    assert_eq!(draft_artifacts(&runtime), 1);
    assert_eq!(cancellations(), 0);
    assert_valid_terminal_stream(&runtime);
}

#[test]
fn a_refused_report_ends_the_run_without_output_or_retention() {
    reset_budget(BudgetAnswer::Exact);
    REPORT_ANSWER.with(|cell| cell.set(ReportAnswer::Refused));
    let mut runtime = report_run(
        report_draft(0, 0),
        ModelProposalKind::CompletionCandidate,
        MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64,
    );
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("the run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(outcome.unresolved_codes, [RESEARCH_REPORT_REFUSED]);
    assert_eq!(outcome.output, None);
    assert_eq!(outcome.answer_evidence, None);
    assert_eq!(reports(), 1);
    assert_eq!(draft_artifacts(&runtime), 0);
    assert_valid_terminal_stream(&runtime);
}

#[test]
fn a_failed_or_mismatched_report_publication_stops_the_run() {
    for (answer, error) in [
        (
            ReportAnswer::Uncertain,
            RuntimeLoopError::Dependency(RuntimePortFailure::Uncertain),
        ),
        (
            ReportAnswer::OtherReference,
            RuntimeLoopError::InvalidBoundaryResult,
        ),
    ] {
        reset_budget(BudgetAnswer::Exact);
        REPORT_ANSWER.with(|cell| cell.set(answer));
        let mut runtime = report_run(
            report_draft(0, 0),
            ModelProposalKind::CompletionCandidate,
            MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64,
        );
        assert_eq!(
            runtime.run_until_boundary(None, None).err(),
            Some(error),
            "{answer:?}"
        );
        assert!(runtime.outcome().is_none(), "{answer:?}");
        assert_eq!(reports(), 1, "{answer:?}");
        assert_eq!(draft_artifacts(&runtime), 0, "{answer:?}");
    }
}

#[test]
fn a_report_draft_is_accepted_only_as_an_exact_admitted_completion() {
    let draft = report_draft(0, 0);
    let pretty =
        serde_json::to_vec_pretty(&serde_json::from_slice::<ResearchReportDraft>(&draft).unwrap())
            .unwrap();
    let mut trailing = draft.clone();
    trailing.push(b' ');
    for (bytes, kind) in [
        (pretty, ModelProposalKind::CompletionCandidate),
        (trailing, ModelProposalKind::CompletionCandidate),
        (b"{}".to_vec(), ModelProposalKind::CompletionCandidate),
        (draft.clone(), ModelProposalKind::Blocked),
        (draft.clone(), ModelProposalKind::UserQuestion),
    ] {
        reset_budget(BudgetAnswer::Exact);
        let mut runtime = report_run(bytes, kind, MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64);
        let RuntimeCoordinatorStep::Complete { outcome } =
            runtime.run_until_boundary(None, None).unwrap()
        else {
            panic!("the run ends");
        };
        assert_eq!(outcome.state, AgentStateKind::Failed, "{kind:?}");
        assert_eq!(outcome.unresolved_codes, ["runtime.proposal.invalid"]);
        assert_eq!(outcome.output, None);
        assert_eq!(reports(), 0, "{kind:?}");
        assert_eq!(draft_artifacts(&runtime), 0);
        assert_valid_terminal_stream(&runtime);
    }
    // A run without a research admission never retains a draft either.
    reset_budget(BudgetAnswer::Exact);
    let (template, _) = coordinator([ModelScript::Completion], PermissionScript::Allow, true);
    let mut ordinary = ReusableRuntimeCoordinator::new(
        template.request.clone(),
        ReportModel {
            inner: FakeModel::new(template.model.profile.clone(), [ModelScript::Completion]),
            draft,
            kind: ModelProposalKind::CompletionCandidate,
            journal_flushes: Arc::new(AtomicUsize::new(0)),
            flushes_at_dispatch: Arc::new(AtomicUsize::new(0)),
        },
        FakeContext,
        template.registry,
        template.tool_boundary,
        template.verifier,
        FakeClock { now: 9_000 },
    )
    .unwrap();
    let RuntimeCoordinatorStep::Complete { outcome } =
        ordinary.run_until_boundary(None, None).unwrap()
    else {
        panic!("the ordinary run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(outcome.unresolved_codes, ["runtime.proposal.invalid"]);
    assert_eq!(outcome.output, None);
    assert_eq!(reports(), 0);
}

#[test]
fn an_unverified_report_is_never_retained() {
    reset_budget(BudgetAnswer::Exact);
    let mut runtime = report_run(
        report_draft(0, 0),
        ModelProposalKind::CompletionCandidate,
        MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64,
    );
    runtime.verifier.source = VerifierSource::ModelProse;
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("the run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(outcome.unresolved_codes, ["runtime.verification.failed"]);
    assert_eq!(outcome.output, None);
    assert_eq!(reports(), 0);
    assert_eq!(draft_artifacts(&runtime), 0);
    assert_valid_terminal_stream(&runtime);
}

#[test]
fn a_report_the_output_budget_cannot_hold_is_not_retained() {
    // The draft fits the run's payload bound, but the task's output budget
    // is one byte short of it.
    reset_budget(BudgetAnswer::Exact);
    let draft = report_draft(0, 0);
    let mut parts = parts(PermissionScript::Allow, registry_with(&[]));
    parts.request.limits.max_output_bytes = MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64;
    for budget in &mut parts.request.work_packet.budgets {
        if budget.resource == BudgetResource::OutputBytes {
            budget.limit = draft.len() as u64 - 1;
        }
    }
    let journal_flushes = Arc::clone(&parts.boundary.journal_flushes);
    let mut runtime = compose_with(parts, |profile| ReportModel {
        inner: FakeModel::new(profile, [ModelScript::Completion]),
        draft,
        kind: ModelProposalKind::CompletionCandidate,
        journal_flushes,
        flushes_at_dispatch: Arc::new(AtomicUsize::new(0)),
    })
    .unwrap();
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("the run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Exhausted);
    assert_eq!(outcome.unresolved_codes, ["runtime.budget.exhausted"]);
    assert_eq!(outcome.output, None);
    assert_eq!(reports(), 0);
    assert_eq!(draft_artifacts(&runtime), 0);
    assert_valid_terminal_stream(&runtime);
}

/// A cancellation the probe reports once a report was published.
struct CancelAfterReport(CancellationSignal);

impl agentmage_kernel_contracts::ModelCancellationProbe for CancelAfterReport {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok((reports() > 0).then(|| self.0.clone()))
    }
}

#[test]
fn a_cancellation_after_the_report_keeps_it_and_cancels_the_budget() {
    reset_budget(BudgetAnswer::Exact);
    let mut runtime = report_run(
        report_draft(0, 0),
        ModelProposalKind::CompletionCandidate,
        MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64,
    );
    let probe = CancelAfterReport(cancellation_of(&runtime));
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, Some(&probe)).unwrap()
    else {
        panic!("the run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Cancelled);
    assert_eq!(outcome.unresolved_codes, ["runtime.cancelled"]);
    assert_eq!(outcome.output, None);
    // The draft stays retained; the cancellation cannot withdraw it.
    assert_eq!(reports(), 1);
    assert_eq!(draft_artifacts(&runtime), 1);
    CANCELLATIONS.with(|cancellations| {
        let cancellations = cancellations.borrow();
        assert_eq!(cancellations.len(), 1);
        assert!(cancellations[0].after_observation);
        assert!(cancellations[0].before_terminal);
    });
    assert_valid_terminal_stream(&runtime);
}
