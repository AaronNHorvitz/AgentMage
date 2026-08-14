use agentmage_kernel_contracts::{
    AgentFinalState, BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason,
    CancellationSignal, CorrelationId, EvidenceKind, Plan, PlanId, PlanState, PlanStep, PlanStepId,
    PlanStepState, TaskId, UserMessageDisposition, UserMessageIntent,
};

use crate::agent_progress::{AgentProgressError, PlanProgressController, build_final_response};
use crate::propagation::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StageKind {
    ModelCall,
    ToolCall,
    Planning,
    StatusRendering,
    Finalization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StageOutcome {
    Pending,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TranscriptRecord {
    stage: StageKind,
    outcome: StageOutcome,
    cleanup_count: u8,
    claims_success: bool,
}

struct InterruptibleStage {
    kind: StageKind,
    token: CancellationToken,
    outcome: StageOutcome,
    cleanup_count: u8,
}

impl InterruptibleStage {
    fn new(kind: StageKind, token: CancellationToken) -> Self {
        Self {
            kind,
            token,
            outcome: StageOutcome::Pending,
            cleanup_count: 0,
        }
    }

    fn poll(&mut self) -> TranscriptRecord {
        if self.token.is_cancelled() && self.outcome != StageOutcome::Cancelled {
            self.outcome = StageOutcome::Cancelled;
            self.cleanup_count += 1;
        }
        TranscriptRecord {
            stage: self.kind,
            outcome: self.outcome,
            cleanup_count: self.cleanup_count,
            claims_success: false,
        }
    }
}

struct CancellationTree {
    shell: CancellationToken,
    kernel: CancellationToken,
    model: CancellationToken,
    tool: CancellationToken,
}

fn tree() -> CancellationTree {
    let shell = CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("s012-rt-task"),
        CorrelationId::from_raw("s012-rt-correlation"),
    );
    let kernel = shell
        .derive_child(BoundaryKind::Kernel)
        .expect("shell derives kernel");
    let model = kernel
        .derive_child(BoundaryKind::Model)
        .expect("kernel derives model");
    let tool = kernel
        .derive_child(BoundaryKind::Tool)
        .expect("kernel derives tool");
    CancellationTree {
        shell,
        kernel,
        model,
        tool,
    }
}

fn signal(identity: &str) -> CancellationSignal {
    CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw(identity),
        correlation_id: CorrelationId::from_raw("s012-rt-correlation"),
        task_id: TaskId::from_raw("s012-rt-task"),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    }
}

fn plan() -> Plan {
    Plan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        plan_id: PlanId::from_raw("s012-rt-plan"),
        task_id: TaskId::from_raw("s012-rt-task"),
        work_packet_revision: 1,
        revision: 1,
        steps: vec![
            PlanStep {
                plan_step_id: PlanStepId::from_raw("s012-rt-plan:step:0"),
                ordinal: 0,
                description: "Run a cancellable synthetic stage".to_owned(),
                depends_on: Vec::new(),
                expected_evidence: vec![EvidenceKind::Validation],
                state: PlanStepState::Running,
            },
            PlanStep {
                plan_step_id: PlanStepId::from_raw("s012-rt-plan:step:1"),
                ordinal: 1,
                description: "Finalize only after current evidence".to_owned(),
                depends_on: vec![PlanStepId::from_raw("s012-rt-plan:step:0")],
                expected_evidence: vec![EvidenceKind::Validation],
                state: PlanStepState::Proposed,
            },
        ],
        state: PlanState::InProgress,
    }
}

fn stage_token(tree: &CancellationTree, kind: StageKind) -> CancellationToken {
    match kind {
        StageKind::ModelCall => tree.model.clone(),
        StageKind::ToolCall => tree.tool.clone(),
        StageKind::Planning | StageKind::StatusRendering | StageKind::Finalization => {
            tree.kernel.clone()
        }
    }
}

#[test]
fn s_012_rt01_interrupts_every_named_stage_and_cleans_once() {
    let mut transcript = Vec::new();
    for (index, kind) in [
        StageKind::ModelCall,
        StageKind::ToolCall,
        StageKind::Planning,
        StageKind::StatusRendering,
        StageKind::Finalization,
    ]
    .into_iter()
    .enumerate()
    {
        let tree = tree();
        let mut stage = InterruptibleStage::new(kind, stage_token(&tree, kind));
        assert_eq!(stage.poll().outcome, StageOutcome::Pending);
        tree.shell
            .cancel(signal(&format!("s012-rt-cancel-{index}")))
            .expect("shell cancellation propagates");
        for token in [&tree.shell, &tree.kernel, &tree.model, &tree.tool] {
            assert!(token.is_cancelled());
        }
        let record = stage.poll();
        assert_eq!(record.outcome, StageOutcome::Cancelled);
        assert_eq!(record.cleanup_count, 1);
        assert!(!record.claims_success);
        assert_eq!(stage.poll(), record);
        transcript.push(record);
    }
    assert_eq!(transcript.len(), 5);
    assert!(transcript.iter().all(|record| {
        record.outcome == StageOutcome::Cancelled
            && record.cleanup_count == 1
            && !record.claims_success
    }));
}

#[test]
fn s_012_rt01_replacement_cancels_plan_and_every_descendant() {
    let tree = tree();
    let mut progress = PlanProgressController::new(plan()).expect("plan is valid");
    assert_eq!(
        progress.status().status,
        agentmage_kernel_contracts::AgentStatusKind::Running
    );
    assert_eq!(
        progress.handle_user_message(
            UserMessageIntent::ReplaceTask,
            Some((&tree.shell, signal("s012-rt-replacement"))),
        ),
        Ok(UserMessageDisposition::InterruptedForReplacement)
    );
    assert_eq!(progress.current().state, PlanState::Cancelled);
    assert_eq!(progress.current().revision, 2);
    assert!(
        progress
            .current()
            .steps
            .iter()
            .all(|step| step.state == PlanStepState::Cancelled)
    );
    for token in [&tree.shell, &tree.kernel, &tree.model, &tree.tool] {
        assert!(token.is_cancelled());
    }
}

#[test]
fn s_012_rt01_status_query_does_not_interrupt_but_status_render_observes_cancellation() {
    let tree = tree();
    let mut progress = PlanProgressController::new(plan()).expect("plan is valid");
    let revision = progress.current().revision;
    assert_eq!(
        progress.handle_user_message(UserMessageIntent::StatusQuery, None),
        Ok(UserMessageDisposition::StatusReturned)
    );
    assert_eq!(progress.current().revision, revision);
    assert!(!tree.shell.is_cancelled());
    assert!(!tree.kernel.is_cancelled());

    let mut renderer = InterruptibleStage::new(StageKind::StatusRendering, tree.kernel.clone());
    tree.shell
        .cancel(signal("s012-rt-status-render"))
        .expect("render cancellation propagates");
    let record = renderer.poll();
    assert_eq!(record.outcome, StageOutcome::Cancelled);
    assert_eq!(record.cleanup_count, 1);
    assert!(!record.claims_success);
}

#[test]
fn s_012_rt01_finalization_never_converts_cancellation_into_success() {
    let tree = tree();
    let mut finalizer = InterruptibleStage::new(StageKind::Finalization, tree.kernel.clone());
    tree.shell
        .cancel(signal("s012-rt-finalization"))
        .expect("finalization cancellation propagates");
    assert_eq!(finalizer.poll().outcome, StageOutcome::Cancelled);
    assert_eq!(
        build_final_response(
            TaskId::from_raw("s012-rt-task"),
            AgentFinalState::Completed,
            "Unsupported completion".to_owned(),
            Vec::new(),
            Vec::new(),
        ),
        Err(AgentProgressError::InvalidFinalResponse)
    );
    let response = build_final_response(
        TaskId::from_raw("s012-rt-task"),
        AgentFinalState::Cancelled,
        "The interrupted synthetic stage did not complete".to_owned(),
        Vec::new(),
        vec!["Finalization was cancelled before completion evidence".to_owned()],
    )
    .expect("cancelled response is truthful");
    assert_eq!(response.state, AgentFinalState::Cancelled);
    assert!(response.evidence.is_empty());
    assert_eq!(response.unresolved.len(), 1);
}

#[test]
fn s_012_rt01_first_signal_late_descendants_and_cleanup_remain_sticky() {
    let tree = tree();
    let first = signal("s012-rt-first");
    assert_eq!(tree.shell.cancel(first.clone()), Ok(first.clone()));
    let mut second = signal("s012-rt-second");
    second.reason = CancellationReason::Shutdown;
    assert_eq!(tree.shell.cancel(second), Ok(first.clone()));
    let late = tree
        .kernel
        .derive_child(BoundaryKind::Tool)
        .expect("late descendant derives");
    assert_eq!(late.signal(), Some(first));

    let mut stage = InterruptibleStage::new(StageKind::ToolCall, late);
    let first_cleanup = stage.poll();
    let repeated_cleanup = stage.poll();
    assert_eq!(first_cleanup, repeated_cleanup);
    assert_eq!(first_cleanup.cleanup_count, 1);
    assert_eq!(first_cleanup.outcome, StageOutcome::Cancelled);
    assert!(!first_cleanup.claims_success);
}
