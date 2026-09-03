//! Verified interactive CLI driver over the shared caller-neutral runtime transport.

use std::collections::BTreeMap;
use std::fmt;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CancellationId, RuntimeApprovalChallenge, RuntimeArtifactRef,
    RuntimeEvent, RuntimeEventCursor, RuntimeEventKind, RuntimeOutcome, RuntimeOutput,
    RuntimeRunRequest, RuntimeSessionMode,
};
use agentmage_kernel_engine::{
    model_routing::{
        AuthenticatedRoutingEnvelope, MeasuredRoutingService, NativeRoutingAuditView,
        RoutingAuthenticationVerifier,
    },
    runtime_artifact::{MAX_RUNTIME_ARTIFACT_BYTES, MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT},
    runtime_coordinator::{
        verify_runtime_approval_challenge, verify_runtime_outcome, verify_runtime_run_request,
    },
    runtime_event::RuntimeEventSequence,
};

use crate::coding_client::{
    CodingApprovalPort, CodingClientError, CodingEventSink, runtime_approval_response,
};
use crate::runtime_transport::{
    RuntimePrepareInput as NativeChatPrepareInput, RuntimeTransportError as NativeChatRuntimeError,
    RuntimeTransportPort as NativeChatRuntimePort, RuntimeTransportStep as NativeChatRuntimeStep,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure from the installed-interface measured-routing adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractiveCliRoutingError {
    /// The kernel rejected authentication or the exact routing request.
    Routing,
    /// The installed interface could not present the verified native audit view.
    Presentation,
}

/// Presentation-only sink for one verified native model-routing audit view.
pub trait InteractiveCliRoutingSink {
    /// Presents the complete content-minimized decision without gaining model authority.
    fn present(&mut self, view: &NativeRoutingAuditView) -> Result<(), InteractiveCliRoutingError>;
}

/// Drives one authenticated installed-interface request through the kernel product router.
///
/// The host adapter cannot provide candidates, select a model, alter the audit view, retry a
/// refusal, or gain model/runtime authority. The service owns the exact catalog and appends the
/// canonical audit before this presentation-only adapter receives it.
pub fn drive_interactive_cli_routing<V, S>(
    service: &mut MeasuredRoutingService<V>,
    envelope: AuthenticatedRoutingEnvelope,
    sink: &mut S,
) -> Result<NativeRoutingAuditView, InteractiveCliRoutingError>
where
    V: RoutingAuthenticationVerifier,
    S: InteractiveCliRoutingSink,
{
    let view = service
        .route(envelope)
        .map_err(|_| InteractiveCliRoutingError::Routing)?
        .clone();
    sink.present(&view)?;
    Ok(view)
}

/// Stable content-free failure from the interactive CLI runtime boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractiveCliRuntimeError {
    /// The trusted host runtime was unavailable or refused the operation.
    Runtime,
    /// The host-framed request did not match the user's exact selected inputs.
    Request,
    /// The returned event, artifact, approval, or outcome evidence was inconsistent.
    Evidence,
    /// The protected approval channel could not return one exact decision.
    Approval,
    /// The terminal presentation sink could not accept verified output.
    Presentation,
    /// The local cancellation source failed before returning an exact identity.
    Cancellation,
    /// The terminal run could not be released from the bounded host registry.
    Release,
}

impl InteractiveCliRuntimeError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Runtime => "cli.runtime.unavailable",
            Self::Request => "cli.runtime.request_denied",
            Self::Evidence => "cli.runtime.evidence_denied",
            Self::Approval => "cli.runtime.approval_unavailable",
            Self::Presentation => "cli.runtime.presentation_failed",
            Self::Cancellation => "cli.runtime.cancellation_failed",
            Self::Release => "cli.runtime.release_failed",
        }
    }
}

impl fmt::Display for InteractiveCliRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for InteractiveCliRuntimeError {}

/// Protected local cancellation source observed only at coordinator boundaries.
pub trait InteractiveCliCancellationPort {
    /// Returns one exact cancellation-tree identity, or `None` to continue normally.
    fn poll(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError>;
}

/// Cancellation source used when no terminal interrupt has been requested.
#[derive(Clone, Copy, Debug, Default)]
pub struct NeverCancelInteractiveCli;

impl InteractiveCliCancellationPort for NeverCancelInteractiveCli {
    fn poll(
        &mut self,
        _request: &RuntimeRunRequest,
    ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
        Ok(None)
    }
}

/// Verified result returned after one interactive CLI run reaches a terminal state.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveCliRuntimeResult {
    /// Exact host-framed request used by the shared coordinator.
    pub request: RuntimeRunRequest,
    /// Canonical terminal outcome verified against the event stream.
    pub outcome: RuntimeOutcome,
    /// Complete verified path-free artifact references retained by the coordinator.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Number of canonical events independently verified and presented.
    pub presented_events: u64,
}

/// Runs one interactive CLI operation through the same host-owned runtime port as native Chat.
///
/// The driver owns no model, tool, policy, grant, storage, filesystem, or process authority. It
/// submits the host-framed request unchanged, independently verifies every returned boundary,
/// presents only verified events, and returns exact approval or cancellation choices.
pub fn drive_interactive_cli_runtime<P, A, S, C>(
    runtime: &mut P,
    input: NativeChatPrepareInput,
    approvals: &mut A,
    sink: &mut S,
    cancellation: &mut C,
) -> Result<InteractiveCliRuntimeResult, InteractiveCliRuntimeError>
where
    P: NativeChatRuntimePort + ?Sized,
    A: CodingApprovalPort,
    S: CodingEventSink,
    C: InteractiveCliCancellationPort,
{
    let request = runtime.prepare(input.clone()).map_err(map_runtime_error)?;
    if verify_prepared_request(&request, &input).is_err() {
        let _ = runtime.release(&request.run_id, &request.request_sha256);
        return Err(InteractiveCliRuntimeError::Request);
    }

    let mut verifier = InteractiveCliRuntimeVerifier::new(request.clone());
    let mut step = match runtime.start(request.clone()) {
        Ok(step) => step,
        Err(error) => {
            let _ = runtime.release(&request.run_id, &request.request_sha256);
            return Err(map_runtime_error(error));
        }
    };

    loop {
        if let Err(error) = verifier.accept(&step) {
            best_effort_release(runtime, &request);
            return Err(error);
        }
        for event in &step.events {
            if let Err(error) = sink.present(event) {
                best_effort_release(runtime, &request);
                return Err(map_client_error(error));
            }
        }

        if let Some(outcome) = step.outcome {
            runtime
                .release(&request.run_id, &request.request_sha256)
                .map_err(|_| InteractiveCliRuntimeError::Release)?;
            return Ok(InteractiveCliRuntimeResult {
                request,
                outcome,
                artifacts: step.artifacts,
                presented_events: verifier.event_count(),
            });
        }

        let Some(challenge) = step.approval.as_ref() else {
            best_effort_release(runtime, &request);
            return Err(InteractiveCliRuntimeError::Evidence);
        };
        let cancellation_id = match cancellation.poll(&request) {
            Ok(cancellation_id) => cancellation_id,
            Err(error) => {
                best_effort_release(runtime, &request);
                return Err(error);
            }
        };
        step = if let Some(cancellation_id) = cancellation_id {
            match runtime.cancel(
                &request.run_id,
                &request.request_sha256,
                cancellation_id,
                verifier.cursor().as_ref(),
            ) {
                Ok(step) => step,
                Err(error) => {
                    best_effort_release(runtime, &request);
                    return Err(map_runtime_error(error));
                }
            }
        } else {
            let disposition = match approvals.decide(challenge) {
                Ok(disposition) => disposition,
                Err(error) => {
                    best_effort_release(runtime, &request);
                    return Err(map_client_error(error));
                }
            };
            let response = runtime_approval_response(challenge, disposition);
            match runtime.advance(
                &request.run_id,
                &request.request_sha256,
                verifier.cursor().as_ref(),
                Some(&response),
            ) {
                Ok(step) => step,
                Err(error) => {
                    best_effort_release(runtime, &request);
                    return Err(map_runtime_error(error));
                }
            }
        };
    }
}

fn best_effort_release<P>(runtime: &mut P, request: &RuntimeRunRequest)
where
    P: NativeChatRuntimePort + ?Sized,
{
    let _ = runtime.release(&request.run_id, &request.request_sha256);
}

fn verify_prepared_request(
    request: &RuntimeRunRequest,
    input: &NativeChatPrepareInput,
) -> Result<(), InteractiveCliRuntimeError> {
    verify_runtime_run_request(request).map_err(|_| InteractiveCliRuntimeError::Request)?;
    if !matches!(
        request.mode,
        RuntimeSessionMode::EphemeralReadOnly | RuntimeSessionMode::ControlledWrite
    ) || request.event_cursor.is_some()
        || request.model_profile.profile_id.as_str() != input.profile_id
        || request.workspace_id.as_str() != input.workspace_id
        || request.task.objective != input.prompt
    {
        return Err(InteractiveCliRuntimeError::Request);
    }
    Ok(())
}

#[derive(Clone)]
struct InteractiveCliRuntimeVerifier {
    request: RuntimeRunRequest,
    sequence: RuntimeEventSequence,
    events: BTreeMap<String, String>,
    artifacts: BTreeMap<String, RuntimeArtifactRef>,
    last_event: Option<RuntimeEvent>,
}

impl InteractiveCliRuntimeVerifier {
    fn new(request: RuntimeRunRequest) -> Self {
        Self {
            request,
            sequence: RuntimeEventSequence::new(),
            events: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            last_event: None,
        }
    }

    fn accept(&mut self, step: &NativeChatRuntimeStep) -> Result<(), InteractiveCliRuntimeError> {
        if step.run_id != self.request.run_id
            || step.request_sha256 != self.request.request_sha256
            || (step.approval.is_some() == step.outcome.is_some())
            || step.events.is_empty()
            || u64::try_from(step.events.len()).map_or(true, |event_count| {
                self.sequence
                    .event_count()
                    .checked_add(event_count)
                    .is_none_or(|total| total > u64::from(self.request.limits.max_events))
            })
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }

        let mut candidate = self.clone();
        for event in &step.events {
            candidate.accept_event(event)?;
        }
        candidate.verify_artifacts(&step.artifacts)?;
        match (&step.approval, &step.outcome) {
            (Some(challenge), None) => candidate.verify_approval(challenge)?,
            (None, Some(outcome)) => candidate.verify_outcome(outcome, &step.artifacts)?,
            _ => return Err(InteractiveCliRuntimeError::Evidence),
        }
        *self = candidate;
        Ok(())
    }

    fn accept_event(&mut self, event: &RuntimeEvent) -> Result<(), InteractiveCliRuntimeError> {
        if event.run_id != self.request.run_id
            || event.session_id != self.request.session_id
            || event.task_id != self.request.task.task_id
            || event.policy_id != self.request.policy_id
            || (event.sequence == 0
                && !matches!(
                    &event.kind,
                    RuntimeEventKind::RunStarted { request_sha256 }
                        if request_sha256 == &self.request.request_sha256
                ))
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        self.sequence
            .push(event)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)?;
        if self
            .events
            .insert(
                event.event_id.as_str().to_owned(),
                event.event_sha256.clone(),
            )
            .is_some()
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        if let RuntimeEventKind::ArtifactCreated {
            artifact_id,
            manifest_sha256,
        } = &event.kind
        {
            let payload = event
                .payload_reference
                .as_ref()
                .ok_or(InteractiveCliRuntimeError::Evidence)?;
            if payload.artifact_id != *artifact_id
                || !valid_sha256(manifest_sha256)
                || self.artifacts.len() >= MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
            let reference = RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: artifact_id.clone(),
                manifest_sha256: manifest_sha256.clone(),
                payload_sha256: payload.sha256.clone(),
                byte_size: payload.byte_size,
                media_type: payload.media_type.clone(),
            };
            if self
                .artifacts
                .insert(artifact_id.as_str().to_owned(), reference)
                .is_some()
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
        }
        self.last_event = Some(event.clone());
        Ok(())
    }

    fn verify_artifacts(
        &self,
        artifacts: &[RuntimeArtifactRef],
    ) -> Result<(), InteractiveCliRuntimeError> {
        if artifacts.len() != self.artifacts.len()
            || artifacts.len() > MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        let mut observed = BTreeMap::new();
        for reference in artifacts {
            if reference.schema_version != CONTRACT_SCHEMA_VERSION
                || !valid_identifier(reference.artifact_id.as_str())
                || !valid_sha256(&reference.manifest_sha256)
                || !valid_sha256(&reference.payload_sha256)
                || reference.byte_size == 0
                || reference.byte_size > MAX_RUNTIME_ARTIFACT_BYTES
                || !valid_media_type(&reference.media_type)
                || observed
                    .insert(reference.artifact_id.as_str(), reference)
                    .is_some()
                || self.artifacts.get(reference.artifact_id.as_str()) != Some(reference)
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
        }
        Ok(())
    }

    fn verify_approval(
        &self,
        challenge: &RuntimeApprovalChallenge,
    ) -> Result<(), InteractiveCliRuntimeError> {
        verify_runtime_approval_challenge(challenge)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)?;
        let Some(RuntimeEvent {
            turn_id: Some(turn_id),
            operation_id: Some(operation_id),
            kind:
                RuntimeEventKind::PermissionRequested {
                    approval_id,
                    operation,
                    preview_sha256,
                    expires_at_epoch_ms,
                },
            ..
        }) = self.last_event.as_ref()
        else {
            return Err(InteractiveCliRuntimeError::Evidence);
        };
        if self.sequence.is_terminal()
            || challenge.run_id != self.request.run_id
            || challenge.task_id != self.request.task.task_id
            || challenge.turn_id != *turn_id
            || challenge.operation_id != *operation_id
            || challenge.approval_id != *approval_id
            || challenge.operation != *operation
            || challenge.preview_sha256 != *preview_sha256
            || challenge.expires_at_epoch_ms != *expires_at_epoch_ms
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        Ok(())
    }

    fn verify_outcome(
        &self,
        outcome: &RuntimeOutcome,
        artifacts: &[RuntimeArtifactRef],
    ) -> Result<(), InteractiveCliRuntimeError> {
        verify_runtime_outcome(outcome, &self.request)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)?;
        let Some(RuntimeEvent {
            causation_event_id: Some(causation_event_id),
            previous_event_sha256,
            kind:
                RuntimeEventKind::RunTerminal {
                    state,
                    outcome_sha256,
                },
            ..
        }) = self.last_event.as_ref()
        else {
            return Err(InteractiveCliRuntimeError::Evidence);
        };
        if !self.sequence.is_terminal()
            || outcome.state != *state
            || outcome.outcome_sha256 != *outcome_sha256
            || outcome.prior_event_id != *causation_event_id
            || outcome.prior_event_sha256 != *previous_event_sha256
            || self.events.get(causation_event_id.as_str()) != Some(previous_event_sha256)
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        if let Some(RuntimeOutput::Artifact { reference }) = &outcome.output {
            let Some(retained) = artifacts
                .iter()
                .find(|item| item.artifact_id == reference.artifact_id)
            else {
                return Err(InteractiveCliRuntimeError::Evidence);
            };
            if retained.payload_sha256 != reference.sha256
                || retained.byte_size != reference.byte_size
                || retained.media_type != reference.media_type
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
        }
        Ok(())
    }

    fn cursor(&self) -> Option<RuntimeEventCursor> {
        self.last_event.as_ref().map(|event| RuntimeEventCursor {
            run_id: event.run_id.clone(),
            event_id: event.event_id.clone(),
            sequence: event.sequence,
            event_sha256: event.event_sha256.clone(),
        })
    }

    const fn event_count(&self) -> u64 {
        self.sequence.event_count()
    }
}

const fn map_runtime_error(_error: NativeChatRuntimeError) -> InteractiveCliRuntimeError {
    InteractiveCliRuntimeError::Runtime
}

const fn map_client_error(error: CodingClientError) -> InteractiveCliRuntimeError {
    match error {
        CodingClientError::Approval => InteractiveCliRuntimeError::Approval,
        CodingClientError::Presentation => InteractiveCliRuntimeError::Presentation,
        CodingClientError::Runtime | CodingClientError::EventStream => {
            InteractiveCliRuntimeError::Evidence
        }
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value != ZERO_SHA256
}

fn valid_media_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'-' | b'.'))
}

#[cfg(all(test, feature = "source-artifacts", feature = "workflow-supervisor"))]
mod tests {
    use agentmage_kernel_contracts::{
        AgentStateKind, ApprovalId, ContextSensitivity, CorrelationId, GrantId, GrantOperation,
        ModelRunId, RuntimeApprovalDisposition, RuntimeEventId, RuntimeEventRetention,
        RuntimeEventRetentionKind, RuntimeOperationId, RuntimePermissionDisposition, RuntimeTurnId,
        ToolCallId,
    };
    use agentmage_kernel_engine::{
        model_routing::{
            MeasuredRoutingRequest, RoutingActionRisk, RoutingBudget, RoutingPlatform,
            RoutingResourceState, RoutingTaskClass,
        },
        runtime_coordinator::{seal_runtime_approval_challenge, seal_runtime_outcome},
        runtime_event::{runtime_event_persistence, seal_runtime_event},
    };

    use super::*;

    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    use agentmage_kernel_contracts::RuntimeArtifactId;

    struct RoutingVerifier;

    impl RoutingAuthenticationVerifier for RoutingVerifier {
        fn verify(&self, actor_id: &str, session_id: &str, authentication_sha256: &str) -> bool {
            actor_id == "actor-installed-1"
                && session_id == "session-installed-1"
                && authentication_sha256 == "e".repeat(64)
        }
    }

    #[derive(Default)]
    struct RoutingSink(Vec<NativeRoutingAuditView>);

    impl InteractiveCliRoutingSink for RoutingSink {
        fn present(
            &mut self,
            view: &NativeRoutingAuditView,
        ) -> Result<(), InteractiveCliRoutingError> {
            self.0.push(view.clone());
            Ok(())
        }
    }

    fn routing_envelope(authentication_sha256: String) -> AuthenticatedRoutingEnvelope {
        AuthenticatedRoutingEnvelope {
            actor_id: "actor-installed-1".to_owned(),
            session_id: "session-installed-1".to_owned(),
            authentication_sha256,
            request: MeasuredRoutingRequest {
                task_id: "task-installed-routing-1".to_owned(),
                task_class: RoutingTaskClass::Coding,
                action_risk: RoutingActionRisk::Moderate,
                budget: RoutingBudget::Standard,
                platform: RoutingPlatform::FedoraX86_64,
                required_context_tokens: 8_192,
                required_tool_proposals: 2,
                resources: RoutingResourceState {
                    available_memory_bytes: 32 * 1024 * 1024 * 1024,
                    available_context_tokens: 16_384,
                    current: true,
                },
                manual_profile_id: None,
                policy_sha256: "a".repeat(64),
                benchmark_generation_sha256: "b".repeat(64),
            },
        }
    }

    #[test]
    fn installed_interface_presents_only_the_kernel_owned_zero_profile_audit() {
        let mut service = MeasuredRoutingService::new(RoutingVerifier, Vec::new(), "c".repeat(64))
            .expect("routing service");
        let mut sink = RoutingSink::default();
        assert_eq!(
            drive_interactive_cli_routing(
                &mut service,
                routing_envelope("d".repeat(64)),
                &mut sink,
            ),
            Err(InteractiveCliRoutingError::Routing)
        );
        assert!(sink.0.is_empty());
        assert!(service.audit_log().is_empty());

        let view = drive_interactive_cli_routing(
            &mut service,
            routing_envelope("e".repeat(64)),
            &mut sink,
        )
        .expect("visible blocked routing view");
        assert_eq!(view.result_code, "model.routing.no-eligible-profile");
        assert!(view.selected_profile.is_none());
        assert!(!view.frontier_transfer);
        assert!(!view.model_confidence_used);
        assert_eq!(sink.0, [view]);
        assert_eq!(service.audit_log().len(), 1);
    }

    struct ScriptedPort {
        input: NativeChatPrepareInput,
        request: RuntimeRunRequest,
        start: Option<NativeChatRuntimeStep>,
        advance: Option<NativeChatRuntimeStep>,
        cancel: Option<NativeChatRuntimeStep>,
        released: usize,
    }

    impl NativeChatRuntimePort for ScriptedPort {
        fn prepare(
            &mut self,
            input: NativeChatPrepareInput,
        ) -> Result<RuntimeRunRequest, NativeChatRuntimeError> {
            if input != self.input {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            Ok(self.request.clone())
        }

        fn start(
            &mut self,
            request: RuntimeRunRequest,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            if request != self.request {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            self.start
                .take()
                .ok_or(NativeChatRuntimeError::RunUnavailable)
        }

        fn advance(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            after_event_cursor: Option<&RuntimeEventCursor>,
            response: Option<&agentmage_kernel_contracts::RuntimeApprovalResponse>,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            verify_call(&self.request, run_id, request_sha256, after_event_cursor)?;
            let challenge = self.start.as_ref().and_then(|step| step.approval.as_ref());
            if challenge.is_some() || response.is_none() {
                return Err(NativeChatRuntimeError::ApprovalDenied);
            }
            self.advance
                .take()
                .ok_or(NativeChatRuntimeError::RunUnavailable)
        }

        fn cancel(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            _cancellation_id: CancellationId,
            after_event_cursor: Option<&RuntimeEventCursor>,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            verify_call(&self.request, run_id, request_sha256, after_event_cursor)?;
            self.cancel
                .take()
                .ok_or(NativeChatRuntimeError::RunUnavailable)
        }

        fn release(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
        ) -> Result<(), NativeChatRuntimeError> {
            if run_id != &self.request.run_id || request_sha256 != self.request.request_sha256 {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            self.released += 1;
            Ok(())
        }
    }

    fn verify_call(
        request: &RuntimeRunRequest,
        run_id: &agentmage_kernel_contracts::RuntimeRunId,
        request_sha256: &str,
        cursor: Option<&RuntimeEventCursor>,
    ) -> Result<(), NativeChatRuntimeError> {
        if run_id != &request.run_id || request_sha256 != request.request_sha256 || cursor.is_none()
        {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        Ok(())
    }

    #[derive(Default)]
    struct RecordingSink(Vec<RuntimeEvent>);

    impl CodingEventSink for RecordingSink {
        fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
            self.0.push(event.clone());
            Ok(())
        }
    }

    struct PanicApproval;

    impl CodingApprovalPort for PanicApproval {
        fn decide(
            &mut self,
            _challenge: &RuntimeApprovalChallenge,
        ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
            panic!("terminal fixture must not request approval")
        }
    }

    struct DenyApproval;

    impl CodingApprovalPort for DenyApproval {
        fn decide(
            &mut self,
            _challenge: &RuntimeApprovalChallenge,
        ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
            Ok(RuntimeApprovalDisposition::Deny)
        }
    }

    struct CancelOnce(Option<CancellationId>);

    impl InteractiveCliCancellationPort for CancelOnce {
        fn poll(
            &mut self,
            _request: &RuntimeRunRequest,
        ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
            Ok(self.0.take())
        }
    }

    struct FailingSink;

    impl CodingEventSink for FailingSink {
        fn present(&mut self, _event: &RuntimeEvent) -> Result<(), CodingClientError> {
            Err(CodingClientError::Presentation)
        }
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn completed_shared_runtime_uses_exact_request_stream_outcome_and_release() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(step(&request, events.clone(), None, Some(outcome.clone()))),
            advance: None,
            cancel: None,
            released: 0,
        };
        let mut sink = RecordingSink::default();
        let result = drive_interactive_cli_runtime(
            &mut port,
            input,
            &mut PanicApproval,
            &mut sink,
            &mut NeverCancelInteractiveCli,
        )
        .expect("terminal runtime completes");

        assert_eq!(result.request, request);
        assert_eq!(result.outcome, outcome);
        assert_eq!(result.presented_events, events.len() as u64);
        assert_eq!(sink.0, events);
        assert_eq!(port.released, 1);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn protected_denial_and_cancellation_return_through_exact_runtime_cursor() {
        let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
        let fixture = approval_fixture(&request);
        let input = input(&request);
        let mut denied_port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting.clone()),
            advance: Some(fixture.denied.clone()),
            cancel: None,
            released: 0,
        };
        let mut denied_sink = RecordingSink::default();
        let denied = drive_interactive_cli_runtime(
            &mut denied_port,
            input.clone(),
            &mut DenyApproval,
            &mut denied_sink,
            &mut NeverCancelInteractiveCli,
        )
        .expect("exact denial completes");
        assert_eq!(denied.outcome.state, AgentStateKind::Declined);
        assert_eq!(denied_port.released, 1);

        let mut cancelled_port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting),
            advance: None,
            cancel: Some(fixture.cancelled),
            released: 0,
        };
        let mut cancelled_sink = RecordingSink::default();
        let cancelled = drive_interactive_cli_runtime(
            &mut cancelled_port,
            input,
            &mut PanicApproval,
            &mut cancelled_sink,
            &mut CancelOnce(Some(CancellationId::from_raw("cli-cancellation-0001"))),
        )
        .expect("exact cancellation completes");
        assert_eq!(cancelled.outcome.state, AgentStateKind::Cancelled);
        assert_eq!(cancelled_port.released, 1);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn request_event_artifact_and_outcome_substitution_fail_before_success() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);

        let mut request_port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: None,
            advance: None,
            cancel: None,
            released: 0,
        };
        request_port.input.profile_id = "substituted-profile".to_owned();
        let substituted_input = request_port.input.clone();
        assert_eq!(
            drive_interactive_cli_runtime(
                &mut request_port,
                substituted_input,
                &mut PanicApproval,
                &mut RecordingSink::default(),
                &mut NeverCancelInteractiveCli,
            ),
            Err(InteractiveCliRuntimeError::Request)
        );
        assert_eq!(request_port.released, 1);

        let mut corrupt_events = events.clone();
        corrupt_events[0].event_sha256 = "f".repeat(64);
        assert_evidence_denied(
            &request,
            &input,
            corrupt_events,
            Vec::new(),
            outcome.clone(),
        );
        assert_evidence_denied(&request, &input, Vec::new(), Vec::new(), outcome.clone());

        let artifact = RuntimeArtifactRef {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("unannounced-artifact-0001"),
            manifest_sha256: "e".repeat(64),
            payload_sha256: "f".repeat(64),
            byte_size: 1,
            media_type: "text/plain".to_owned(),
        };
        assert_evidence_denied(
            &request,
            &input,
            events.clone(),
            vec![artifact],
            outcome.clone(),
        );

        let mut substituted_outcome = outcome;
        substituted_outcome.outcome_sha256 = "f".repeat(64);
        assert_evidence_denied(&request, &input, events, Vec::new(), substituted_outcome);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn story_50_2_confusion_and_interface_bypasses_never_present_or_complete() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);

        let mut replayed = events.clone();
        replayed.insert(1, events[0].clone());
        assert_evidence_denied(&request, &input, replayed, Vec::new(), outcome.clone());

        let mut foreign_session = events.clone();
        foreign_session[0].session_id =
            agentmage_kernel_contracts::SessionId::from_raw("session-foreign");
        foreign_session[0] =
            seal_runtime_event(foreign_session[0].clone()).expect("foreign event reseals");
        assert_evidence_denied(
            &request,
            &input,
            foreign_session,
            Vec::new(),
            outcome.clone(),
        );

        let mut malformed_terminal = events.clone();
        let terminal = malformed_terminal
            .last_mut()
            .expect("terminal event exists");
        let RuntimeEventKind::RunTerminal { state, .. } = &mut terminal.kind else {
            panic!("fixture must end in a terminal event");
        };
        *state = AgentStateKind::Failed;
        *terminal = seal_runtime_event(terminal.clone()).expect("terminal mutation reseals");
        assert_evidence_denied(
            &request,
            &input,
            malformed_terminal,
            Vec::new(),
            outcome.clone(),
        );

        let mut forged_run = step(&request, events.clone(), None, Some(outcome.clone()));
        forged_run.run_id = agentmage_kernel_contracts::RuntimeRunId::from_raw("run-foreign");
        assert_step_evidence_denied(&request, &input, forged_run);

        let mut forged_request = step(&request, events, None, Some(outcome));
        forged_request.request_sha256 = "f".repeat(64);
        assert_step_evidence_denied(&request, &input, forged_request);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn terminal_presentation_failure_releases_without_claiming_completion() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(step(&request, events, None, Some(outcome))),
            advance: None,
            cancel: None,
            released: 0,
        };

        assert_eq!(
            drive_interactive_cli_runtime(
                &mut port,
                input,
                &mut PanicApproval,
                &mut FailingSink,
                &mut NeverCancelInteractiveCli,
            ),
            Err(InteractiveCliRuntimeError::Presentation)
        );
        assert_eq!(port.released, 1);
    }

    fn assert_evidence_denied(
        request: &RuntimeRunRequest,
        input: &NativeChatPrepareInput,
        events: Vec<RuntimeEvent>,
        artifacts: Vec<RuntimeArtifactRef>,
        outcome: RuntimeOutcome,
    ) {
        assert_step_evidence_denied(
            request,
            input,
            NativeChatRuntimeStep {
                run_id: request.run_id.clone(),
                request_sha256: request.request_sha256.clone(),
                events,
                artifacts,
                approval: None,
                outcome: Some(outcome),
            },
        );
    }

    fn assert_step_evidence_denied(
        request: &RuntimeRunRequest,
        input: &NativeChatPrepareInput,
        step: NativeChatRuntimeStep,
    ) {
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(step),
            advance: None,
            cancel: None,
            released: 0,
        };
        let mut sink = RecordingSink::default();
        assert_eq!(
            drive_interactive_cli_runtime(
                &mut port,
                input.clone(),
                &mut PanicApproval,
                &mut sink,
                &mut NeverCancelInteractiveCli,
            ),
            Err(InteractiveCliRuntimeError::Evidence)
        );
        assert!(sink.0.is_empty());
    }

    fn input(request: &RuntimeRunRequest) -> NativeChatPrepareInput {
        NativeChatPrepareInput {
            engineering_session_id: None,
            profile_id: request.model_profile.profile_id.as_str().to_owned(),
            expected_entry_sha256: "a".repeat(64),
            workspace_id: request.workspace_id.as_str().to_owned(),
            workspace_root: "/tmp/agentmage-cli-runtime-fixture".to_owned(),
            prompt: request.task.objective.clone(),
        }
    }

    fn step(
        request: &RuntimeRunRequest,
        events: Vec<RuntimeEvent>,
        approval: Option<RuntimeApprovalChallenge>,
        outcome: Option<RuntimeOutcome>,
    ) -> NativeChatRuntimeStep {
        NativeChatRuntimeStep {
            run_id: request.run_id.clone(),
            request_sha256: request.request_sha256.clone(),
            events,
            artifacts: Vec::new(),
            approval,
            outcome,
        }
    }

    struct ApprovalFixture {
        awaiting: NativeChatRuntimeStep,
        denied: NativeChatRuntimeStep,
        cancelled: NativeChatRuntimeStep,
    }

    fn approval_fixture(request: &RuntimeRunRequest) -> ApprovalFixture {
        let turn_id = RuntimeTurnId::from_raw("cli-turn-0001");
        let operation_id = RuntimeOperationId::from_raw("cli-operation-0001");
        let tool_call_id = ToolCallId::from_raw("cli-tool-call-0001");
        let challenge = seal_runtime_approval_challenge(RuntimeApprovalChallenge {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: request.run_id.clone(),
            task_id: request.task.task_id.clone(),
            turn_id: turn_id.clone(),
            operation_id: operation_id.clone(),
            tool_call_id: tool_call_id.clone(),
            approval_id: ApprovalId::from_raw("cli-approval-0001"),
            proposed_grant_id: GrantId::from_raw("cli-grant-0001"),
            operation: GrantOperation::WorkspaceRead,
            preview_sha256: "a".repeat(64),
            expires_at_epoch_ms: 50_000,
            challenge_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("challenge");
        let mut stream = FixtureStream::new(request);
        let prefix = vec![
            stream.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: request.request_sha256.clone(),
                },
                None,
                None,
            ),
            stream.event(RuntimeEventKind::TurnStarted, Some(&turn_id), None),
            stream.event(
                RuntimeEventKind::ModelRequested {
                    model_run_id: ModelRunId::from_raw("cli-model-run-0001"),
                    request_sha256: "b".repeat(64),
                },
                Some(&turn_id),
                None,
            ),
            stream.event(
                RuntimeEventKind::ModelCompleted {
                    model_run_id: ModelRunId::from_raw("cli-model-run-0001"),
                    result_sha256: "c".repeat(64),
                },
                Some(&turn_id),
                None,
            ),
            stream.event(
                RuntimeEventKind::ToolRequested {
                    tool_call_id,
                    arguments_sha256: "d".repeat(64),
                },
                Some(&turn_id),
                Some(&operation_id),
            ),
            stream.event(
                RuntimeEventKind::PermissionRequested {
                    approval_id: challenge.approval_id.clone(),
                    operation: challenge.operation,
                    preview_sha256: challenge.preview_sha256.clone(),
                    expires_at_epoch_ms: challenge.expires_at_epoch_ms,
                },
                Some(&turn_id),
                Some(&operation_id),
            ),
        ];

        let awaiting = step(request, prefix, Some(challenge.clone()), None);

        let mut denied_stream = stream.clone();
        let mut denied_events = Vec::new();
        denied_events.push(denied_stream.event(
            RuntimeEventKind::PermissionDecided {
                approval_id: challenge.approval_id,
                disposition: RuntimePermissionDisposition::Deny,
                grant_id: None,
                decision_sha256: challenge.challenge_sha256,
            },
            Some(&turn_id),
            Some(&operation_id),
        ));
        denied_events.push(denied_stream.event(
            RuntimeEventKind::TurnCompleted {
                outcome_sha256: "e".repeat(64),
            },
            Some(&turn_id),
            None,
        ));
        let denied_outcome = denied_stream.outcome(AgentStateKind::Declined);
        denied_events.push(denied_stream.event(
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Declined,
                outcome_sha256: denied_outcome.outcome_sha256.clone(),
            },
            None,
            None,
        ));

        let mut cancelled_stream = stream;
        let cancellation_id = CancellationId::from_raw("cli-cancellation-0001");
        let mut cancelled_events = Vec::new();
        cancelled_events.push(cancelled_stream.event(
            RuntimeEventKind::CancellationRequested {
                cancellation_id: cancellation_id.clone(),
            },
            None,
            None,
        ));
        cancelled_events.push(cancelled_stream.event(
            RuntimeEventKind::CancellationObserved { cancellation_id },
            None,
            None,
        ));
        let cancelled_outcome = cancelled_stream.outcome(AgentStateKind::Cancelled);
        cancelled_events.push(cancelled_stream.event(
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Cancelled,
                outcome_sha256: cancelled_outcome.outcome_sha256.clone(),
            },
            None,
            None,
        ));

        ApprovalFixture {
            awaiting,
            denied: step(request, denied_events, None, Some(denied_outcome)),
            cancelled: step(request, cancelled_events, None, Some(cancelled_outcome)),
        }
    }

    #[derive(Clone)]
    struct FixtureStream {
        request: RuntimeRunRequest,
        correlation_id: CorrelationId,
        sequence: u64,
        previous_sha256: String,
        causation_event_id: Option<RuntimeEventId>,
        occurred_at_epoch_ms: u64,
    }

    impl FixtureStream {
        fn new(request: &RuntimeRunRequest) -> Self {
            Self {
                request: request.clone(),
                correlation_id: CorrelationId::from_raw("cli-correlation-0001"),
                sequence: 0,
                previous_sha256: ZERO_SHA256.to_owned(),
                causation_event_id: None,
                occurred_at_epoch_ms: 1_000,
            }
        }

        fn event(
            &mut self,
            kind: RuntimeEventKind,
            turn_id: Option<&RuntimeTurnId>,
            operation_id: Option<&RuntimeOperationId>,
        ) -> RuntimeEvent {
            self.occurred_at_epoch_ms += 1;
            let event_id = RuntimeEventId::from_raw(format!("cli-event-{:04}", self.sequence));
            let persistence = runtime_event_persistence(&kind);
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: event_id.clone(),
                run_id: self.request.run_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                turn_id: turn_id.cloned(),
                operation_id: operation_id.cloned(),
                correlation_id: self.correlation_id.clone(),
                causation_event_id: self.causation_event_id.clone(),
                sequence: self.sequence,
                occurred_at_epoch_ms: self.occurred_at_epoch_ms,
                sensitivity: ContextSensitivity::Internal,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Ephemeral,
                    expires_at_epoch_ms: None,
                },
                persistence,
                policy_id: self.request.policy_id.clone(),
                payload_reference: None,
                kind,
                previous_event_sha256: self.previous_sha256.clone(),
                event_sha256: ZERO_SHA256.to_owned(),
            })
            .expect("event");
            self.sequence += 1;
            self.previous_sha256.clone_from(&event.event_sha256);
            self.causation_event_id = Some(event_id);
            event
        }

        fn outcome(&self, state: AgentStateKind) -> RuntimeOutcome {
            seal_runtime_outcome(
                RuntimeOutcome {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    run_id: self.request.run_id.clone(),
                    session_id: self.request.session_id.clone(),
                    task_id: self.request.task.task_id.clone(),
                    request_sha256: self.request.request_sha256.clone(),
                    state,
                    turn_count: 1,
                    model_call_count: 1,
                    tool_call_count: 0,
                    prior_event_id: self.causation_event_id.clone().expect("prior event exists"),
                    prior_event_sha256: self.previous_sha256.clone(),
                    evidence: Vec::new(),
                    receipt_ids: Vec::new(),
                    unresolved_codes: Vec::new(),
                    output: None,
                    answer_evidence: None,
                    outcome_sha256: ZERO_SHA256.to_owned(),
                },
                &self.request,
            )
            .expect("outcome")
        }
    }
}
