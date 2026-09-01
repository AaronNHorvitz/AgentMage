//! Verified workflow attempts driven through the existing shared coding client and coordinator.

use agentmage_kernel_contracts::{
    CanonicalWorkflowFailureClass, RuntimeEvent, RuntimeOutcome, RuntimeRunRequest,
};
use agentmage_kernel_engine::verified_workflow_supervisor::{
    VerifiedWorkflowAttemptPort, VerifiedWorkflowStepRun, WorkflowAttemptPortResult,
    WorkflowAttemptRequest, WorkflowAttemptResources, WorkflowSupervisorError,
};

use crate::coding_client::{
    CodingApprovalPort, CodingCoordinatorPort, CodingEventSink, drive_coding_client,
};

/// One fresh coordinator composition prepared by the trusted host boundary.
pub struct PreparedWorkflowCoordinatorAttempt<R, A> {
    /// Fresh supervisor attempt identity.
    pub attempt_id: String,
    /// Exact requested one-based attempt ordinal.
    pub attempt_ordinal: u32,
    /// Exact current preflight policy digest.
    pub preflight_policy_sha256: String,
    /// Whether every registered preflight was freshly observed before composition.
    pub preflight_current: bool,
    /// Exact request owned by the newly composed coordinator.
    pub request: RuntimeRunRequest,
    /// Existing common coding coordinator; no workflow-specific execution loop is introduced.
    pub coordinator: R,
    /// Protected approval port used by the common coding-client driver.
    pub approvals: A,
}

/// Trusted factory for one fresh common coordinator per workflow attempt.
pub trait WorkflowCoordinatorAttemptFactory {
    /// Existing coordinator implementation returned for an attempt.
    type Coordinator: CodingCoordinatorPort;
    /// Protected approval implementation returned for an attempt.
    type Approvals: CodingApprovalPort;

    /// Revalidates current inputs and composes one fresh coordinator and approval boundary.
    fn prepare(
        &mut self,
        request: WorkflowAttemptRequest<'_>,
    ) -> Result<
        PreparedWorkflowCoordinatorAttempt<Self::Coordinator, Self::Approvals>,
        WorkflowSupervisorError,
    >;
}

/// Deterministic policy projection from a canonical outcome into supervisor-only accounting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassifiedWorkflowOutcome {
    /// Failure class for a non-success; absent for verified success and no-op.
    pub failure_class: Option<CanonicalWorkflowFailureClass>,
    /// Whether a possibly completed effect requires reconciliation before any successor.
    pub uncertain_effect: bool,
    /// Resource accounting not already present in the canonical outcome.
    pub resources: WorkflowAttemptResources,
}

/// Trusted deterministic outcome classifier for the admitted workflow policy.
pub trait WorkflowOutcomeClassifier {
    /// Classifies one exact coordinator result without granting retry or execution authority.
    fn classify(
        &mut self,
        request: WorkflowAttemptRequest<'_>,
        runtime_request: &RuntimeRunRequest,
        outcome: &RuntimeOutcome,
    ) -> Result<ClassifiedWorkflowOutcome, WorkflowSupervisorError>;
}

/// Presentation sink that deliberately retains no runtime content.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiscardWorkflowEvents;

impl CodingEventSink for DiscardWorkflowEvents {
    fn present(
        &mut self,
        _event: &RuntimeEvent,
    ) -> Result<(), crate::coding_client::CodingClientError> {
        Ok(())
    }
}

/// Workflow-attempt adapter over `drive_coding_client` and a fresh common coordinator.
///
/// The supervisor chooses a dependency-ready step. The factory performs trusted composition, the
/// existing coding-client driver alone advances the existing coordinator, and the classifier
/// describes its verifier-owned outcome. The adapter cannot dispatch a tool or mint authority.
pub struct CoordinatorWorkflowAttemptPort<F, C, S = DiscardWorkflowEvents> {
    factory: F,
    classifier: C,
    sink: S,
}

impl<F, C> CoordinatorWorkflowAttemptPort<F, C, DiscardWorkflowEvents> {
    /// Builds an authority-free adapter with a content-discarding presentation sink.
    #[must_use]
    pub const fn new(factory: F, classifier: C) -> Self {
        Self {
            factory,
            classifier,
            sink: DiscardWorkflowEvents,
        }
    }
}

impl<F, C, S> CoordinatorWorkflowAttemptPort<F, C, S> {
    /// Builds an adapter with one caller-selected presentation-only event sink.
    #[must_use]
    pub const fn with_sink(factory: F, classifier: C, sink: S) -> Self {
        Self {
            factory,
            classifier,
            sink,
        }
    }

    /// Returns the owned factory, classifier, and presentation sink after execution.
    #[must_use]
    pub fn into_parts(self) -> (F, C, S) {
        (self.factory, self.classifier, self.sink)
    }
}

impl<F, C, S> VerifiedWorkflowAttemptPort for CoordinatorWorkflowAttemptPort<F, C, S>
where
    F: WorkflowCoordinatorAttemptFactory,
    C: WorkflowOutcomeClassifier,
    S: CodingEventSink,
{
    fn run_attempt(
        &mut self,
        request: WorkflowAttemptRequest<'_>,
    ) -> Result<WorkflowAttemptPortResult, WorkflowSupervisorError> {
        let mut prepared = self.factory.prepare(request)?;
        let client_result = drive_coding_client(
            &mut prepared.coordinator,
            &mut prepared.approvals,
            &mut self.sink,
            None,
        )
        .map_err(|_| WorkflowSupervisorError::RuntimeFailed)?;
        let classification =
            self.classifier
                .classify(request, &prepared.request, &client_result.outcome)?;
        Ok(WorkflowAttemptPortResult::Terminal(Box::new(
            VerifiedWorkflowStepRun {
                attempt_id: prepared.attempt_id,
                attempt_ordinal: prepared.attempt_ordinal,
                preflight_policy_sha256: prepared.preflight_policy_sha256,
                preflight_current: prepared.preflight_current,
                request: prepared.request,
                events: prepared.coordinator.runtime_events().to_vec(),
                outcome: client_result.outcome,
                failure_class: classification.failure_class,
                uncertain_effect: classification.uncertain_effect,
                resources: classification.resources,
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        AgentStateKind, CONTRACT_SCHEMA_VERSION, ContextSensitivity, CorrelationId, EvidenceId,
        EvidenceKind, EvidenceReference, ModelCancellationProbe, PlanStepId,
        RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEventId, RuntimeEventKind,
        RuntimeEventRetention, RuntimeEventRetentionKind,
    };
    use agentmage_kernel_engine::{
        runtime_coordinator::seal_runtime_outcome,
        runtime_event::{runtime_event_persistence, seal_runtime_event},
        runtime_loop::RuntimeCoordinatorStep,
    };

    use super::*;
    use crate::{
        coding_client::{CodingClientError, DenyHeadlessApproval},
        coding_run::tests::fixture_profile_and_request,
    };

    const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn workflow_attempts_use_the_common_client_driver_and_presentation_cannot_change_results() {
        let (_profile, request) = fixture_profile_and_request();
        let (events, outcome) = terminal_fixture(&request);
        let discarded = run_with_sink(
            request.clone(),
            events.clone(),
            outcome.clone(),
            DiscardWorkflowEvents,
        );
        let counted = run_with_sink(
            request.clone(),
            events.clone(),
            outcome.clone(),
            CountingSink::default(),
        );
        let captured = run_with_sink(request, events, outcome, CapturingSink::default());

        for candidate in [&counted, &captured] {
            assert_eq!(candidate.request, discarded.request);
            assert_eq!(candidate.events, discarded.events);
            assert_eq!(candidate.outcome, discarded.outcome);
            assert_eq!(candidate.failure_class, discarded.failure_class);
            assert_eq!(candidate.resources, discarded.resources);
        }
    }

    fn run_with_sink<S: CodingEventSink>(
        request: RuntimeRunRequest,
        events: Vec<RuntimeEvent>,
        outcome: RuntimeOutcome,
        sink: S,
    ) -> VerifiedWorkflowStepRun {
        let plan_step_id = PlanStepId::from_raw("plan-supervisor:step:0001");
        let workflow_request = WorkflowAttemptRequest {
            execution_id: "execution-adapter",
            step_id: "step-1",
            plan_step_id: &plan_step_id,
            policy_sha256: SHA,
            attempt_ordinal: 1,
            prior_attempt_id: None,
            completed_dependencies: &[],
        };
        let factory = ReplayFactory {
            request,
            events,
            outcome,
        };
        let mut adapter =
            CoordinatorWorkflowAttemptPort::with_sink(factory, SuccessClassifier, sink);
        match adapter
            .run_attempt(workflow_request)
            .expect("common client driver completes")
        {
            WorkflowAttemptPortResult::Terminal(run) => *run,
            WorkflowAttemptPortResult::Interrupted { .. } => panic!("fixture is terminal"),
        }
    }

    struct ReplayFactory {
        request: RuntimeRunRequest,
        events: Vec<RuntimeEvent>,
        outcome: RuntimeOutcome,
    }

    impl WorkflowCoordinatorAttemptFactory for ReplayFactory {
        type Coordinator = ReplayCoordinator;
        type Approvals = DenyHeadlessApproval;

        fn prepare(
            &mut self,
            request: WorkflowAttemptRequest<'_>,
        ) -> Result<
            PreparedWorkflowCoordinatorAttempt<Self::Coordinator, Self::Approvals>,
            WorkflowSupervisorError,
        > {
            Ok(PreparedWorkflowCoordinatorAttempt {
                attempt_id: "attempt-adapter-0001".to_owned(),
                attempt_ordinal: request.attempt_ordinal,
                preflight_policy_sha256: request.policy_sha256.to_owned(),
                preflight_current: true,
                request: self.request.clone(),
                coordinator: ReplayCoordinator {
                    events: self.events.clone(),
                    outcome: Some(self.outcome.clone()),
                },
                approvals: DenyHeadlessApproval,
            })
        }
    }

    struct SuccessClassifier;

    impl WorkflowOutcomeClassifier for SuccessClassifier {
        fn classify(
            &mut self,
            _request: WorkflowAttemptRequest<'_>,
            _runtime_request: &RuntimeRunRequest,
            outcome: &RuntimeOutcome,
        ) -> Result<ClassifiedWorkflowOutcome, WorkflowSupervisorError> {
            if !outcome.state.is_success() {
                return Err(WorkflowSupervisorError::EvidenceDenied);
            }
            Ok(ClassifiedWorkflowOutcome {
                failure_class: None,
                uncertain_effect: false,
                resources: WorkflowAttemptResources {
                    context_tokens: 8,
                    memory_bytes: 1_024,
                    cost_minor_units: 0,
                },
            })
        }
    }

    struct ReplayCoordinator {
        events: Vec<RuntimeEvent>,
        outcome: Option<RuntimeOutcome>,
    }

    impl CodingCoordinatorPort for ReplayCoordinator {
        fn advance(
            &mut self,
            _response: Option<&RuntimeApprovalResponse>,
            _cancellation: Option<&dyn ModelCancellationProbe>,
        ) -> Result<RuntimeCoordinatorStep, CodingClientError> {
            self.outcome
                .take()
                .map(|outcome| RuntimeCoordinatorStep::Complete { outcome })
                .ok_or(CodingClientError::Runtime)
        }

        fn runtime_events(&self) -> &[RuntimeEvent] {
            &self.events
        }

        fn runtime_artifacts(&self) -> &[RuntimeArtifactRef] {
            &[]
        }
    }

    #[derive(Default)]
    struct CountingSink(u64);

    impl CodingEventSink for CountingSink {
        fn present(&mut self, _event: &RuntimeEvent) -> Result<(), CodingClientError> {
            self.0 += 1;
            Ok(())
        }
    }

    #[derive(Default)]
    struct CapturingSink(Vec<RuntimeEventId>);

    impl CodingEventSink for CapturingSink {
        fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
            self.0.push(event.event_id.clone());
            Ok(())
        }
    }

    fn terminal_fixture(request: &RuntimeRunRequest) -> (Vec<RuntimeEvent>, RuntimeOutcome) {
        let started = event(
            request,
            0,
            None,
            ZERO_SHA256,
            RuntimeEventKind::RunStarted {
                request_sha256: request.request_sha256.clone(),
            },
        );
        let outcome = seal_runtime_outcome(
            RuntimeOutcome {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: request.run_id.clone(),
                session_id: request.session_id.clone(),
                task_id: request.task.task_id.clone(),
                request_sha256: request.request_sha256.clone(),
                state: AgentStateKind::Success,
                turn_count: 0,
                model_call_count: 0,
                tool_call_count: 0,
                prior_event_id: started.event_id.clone(),
                prior_event_sha256: started.event_sha256.clone(),
                evidence: vec![EvidenceReference {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    evidence_id: EvidenceId::from_raw("workflow-adapter-evidence"),
                    kind: EvidenceKind::Validation,
                    source_id: "workflow-adapter".to_owned(),
                    object_id: "fixture".to_owned(),
                    fragment: Some("postcondition:current".to_owned()),
                    content_sha256: SHA.to_owned(),
                    observed_revision: Some("fixture-revision".to_owned()),
                }],
                receipt_ids: Vec::new(),
                unresolved_codes: Vec::new(),
                output: None,
                answer_evidence: None,
                outcome_sha256: ZERO_SHA256.to_owned(),
            },
            request,
        )
        .expect("outcome seals");
        let terminal = event(
            request,
            1,
            Some(started.event_id.clone()),
            &started.event_sha256,
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Success,
                outcome_sha256: outcome.outcome_sha256.clone(),
            },
        );
        (vec![started, terminal], outcome)
    }

    fn event(
        request: &RuntimeRunRequest,
        sequence: u64,
        causation_event_id: Option<RuntimeEventId>,
        previous_event_sha256: &str,
        kind: RuntimeEventKind,
    ) -> RuntimeEvent {
        seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw(format!("workflow-adapter-event-{sequence}")),
            run_id: request.run_id.clone(),
            session_id: request.session_id.clone(),
            task_id: request.task.task_id.clone(),
            turn_id: None,
            operation_id: None,
            correlation_id: CorrelationId::from_raw("workflow-adapter-correlation"),
            causation_event_id,
            sequence,
            occurred_at_epoch_ms: 1_000 + sequence,
            sensitivity: ContextSensitivity::Internal,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Ephemeral,
                expires_at_epoch_ms: None,
            },
            persistence: runtime_event_persistence(&kind),
            policy_id: request.policy_id.clone(),
            payload_reference: None,
            kind,
            previous_event_sha256: previous_event_sha256.to_owned(),
            event_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("event seals")
    }
}
