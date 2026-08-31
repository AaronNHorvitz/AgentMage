//! Read-only execution inspection and identity-bound performance qualification.

use std::collections::BTreeMap;

use agentmage_kernel_contracts::{
    AgentStateKind, EndpointClass, RuntimeEvent, RuntimeEventCursor, RuntimeEventKind,
    RuntimePayloadReference, RuntimePermissionDisposition, RuntimeSafeNextAction,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::runtime_event::{RuntimeEventError, RuntimeEventSequence};

/// Maximum events reconstructed into one inspector view.
pub const MAX_INSPECTOR_EVENTS: usize = 4_096;

/// Closed attributable fact category exposed to clients.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorFactKind {
    /// Source capture or admission.
    Capture,
    /// Source parsing or extraction.
    Parse,
    /// Context indexing or disposition.
    Context,
    /// Model route selection.
    Route,
    /// Model request, completion, or failure.
    Model,
    /// Model-authored proposal observation.
    Proposal,
    /// Policy or preflight decision.
    Policy,
    /// Human approval decision.
    Approval,
    /// Tool request or execution.
    Tool,
    /// Attempt or durable effect observation.
    Effect,
    /// Postcondition verification.
    Verification,
    /// Recovery decision.
    Recovery,
    /// Durable artifact creation.
    Artifact,
    /// Durable checkpoint commitment.
    Checkpoint,
    /// Cancellation request or observation.
    Cancellation,
    /// Run, turn, or diagnostic terminal state.
    Terminal,
    /// Content-free progress update.
    Progress,
    /// Content-free metric observation.
    Metric,
}

/// One content-free, event-attributable inspector fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InspectorFact {
    /// Canonical event sequence number.
    pub sequence: u64,
    /// Canonical event identifier.
    pub event_id: String,
    /// Closed fact category.
    pub kind: InspectorFactKind,
    /// Turn identity when the event belongs to a turn.
    pub turn_id: Option<String>,
    /// Operation identity when the event belongs to an operation.
    pub operation_id: Option<String>,
    /// Primary identity named by the event.
    pub subject_id: String,
    /// Integrity digest of the source event.
    pub event_sha256: String,
    /// Optional content-bearing artifact reference; never inline content.
    pub payload_reference: Option<RuntimePayloadReference>,
}

/// Complete read-only view reconstructed from canonical journal events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExecutionInspectorView {
    /// Run reconstructed by this view.
    pub run_id: String,
    /// Session containing the run.
    pub session_id: String,
    /// Task containing the run.
    pub task_id: String,
    /// Active turn, if one has not terminated.
    pub current_turn_id: Option<String>,
    /// Active tool operation, if one has not ended.
    pub current_operation_id: Option<String>,
    /// Most recently selected route decision.
    pub latest_route_id: Option<String>,
    /// Endpoint class selected by the latest route.
    pub latest_endpoint_class: Option<EndpointClass>,
    /// Ordered proposal identities observed in the journal.
    pub proposal_ids: Vec<String>,
    /// Ordered distinct approval identities observed in the journal.
    pub approval_ids: Vec<String>,
    /// Ordered tool-call identities observed in the journal.
    pub tool_call_ids: Vec<String>,
    /// Ordered attempt identities with observed effects.
    pub effect_attempt_ids: Vec<String>,
    /// Ordered verification identities observed in the journal.
    pub verification_ids: Vec<String>,
    /// Content-free blocker and failure codes.
    pub blocker_codes: Vec<String>,
    /// Distinct content-bearing artifact references in first-observed order.
    pub artifacts: Vec<RuntimePayloadReference>,
    /// Final agent state, when the run has terminated.
    pub terminal_state: Option<AgentStateKind>,
    /// Latest terminal diagnostic identity.
    pub terminal_diagnostic_id: Option<String>,
    /// Safe non-authoritative action named by the latest diagnostic.
    pub safe_next_action: Option<RuntimeSafeNextAction>,
    /// Resume cursor for the last reconstructed event.
    pub cursor: RuntimeEventCursor,
    /// Ordered content-free facts attributable to source events.
    pub facts: Vec<InspectorFact>,
    /// Digest of the complete view with this field zeroed while hashing.
    pub view_sha256: String,
}

/// Reconstructs one bounded inspector view without granting it state authority.
pub fn build_execution_inspector(
    events: &[RuntimeEvent],
) -> Result<ExecutionInspectorView, RuntimeEventError> {
    if events.is_empty() || events.len() > MAX_INSPECTOR_EVENTS {
        return Err(RuntimeEventError::BatchLimit);
    }
    let mut sequence = RuntimeEventSequence::new();
    for event in events {
        sequence.push(event)?;
    }
    let first = &events[0];
    let last = events.last().ok_or(RuntimeEventError::IllegalTransition)?;
    let mut current_turn_id = None;
    let mut current_operation_id = None;
    let mut latest_route_id = None;
    let mut latest_endpoint_class = None;
    let mut proposal_ids = Vec::new();
    let mut approval_ids = Vec::new();
    let mut tool_call_ids = Vec::new();
    let mut effect_attempt_ids = Vec::new();
    let mut verification_ids = Vec::new();
    let mut blocker_codes = Vec::new();
    let mut artifacts = Vec::new();
    let mut artifact_references = BTreeMap::new();
    let mut terminal_state = None;
    let mut terminal_diagnostic_id = None;
    let mut safe_next_action = None;
    let mut facts = Vec::with_capacity(events.len());
    for event in events {
        if let Some(reference) = &event.payload_reference {
            let artifact_id = reference.artifact_id.as_str().to_owned();
            if let Some(existing) = artifact_references.get(&artifact_id) {
                if existing != reference {
                    return Err(RuntimeEventError::DigestMismatch);
                }
            } else {
                artifact_references.insert(artifact_id, reference.clone());
                artifacts.push(reference.clone());
            }
        }
        match &event.kind {
            RuntimeEventKind::TurnStarted => {
                current_turn_id = event
                    .turn_id
                    .as_ref()
                    .map(|value| value.as_str().to_owned());
            }
            RuntimeEventKind::TurnCompleted { .. } => {
                current_turn_id = None;
                current_operation_id = None;
            }
            RuntimeEventKind::RouteSelected {
                route_decision_id,
                endpoint_class,
                ..
            } => {
                latest_route_id = Some(route_decision_id.clone());
                latest_endpoint_class = Some(*endpoint_class);
            }
            RuntimeEventKind::ProposalObserved { proposal_id, .. } => {
                proposal_ids.push(proposal_id.clone());
            }
            RuntimeEventKind::PermissionRequested { approval_id, .. }
            | RuntimeEventKind::PermissionDecided { approval_id, .. } => {
                if !approval_ids
                    .iter()
                    .any(|value| value == approval_id.as_str())
                {
                    approval_ids.push(approval_id.as_str().to_owned());
                }
                if matches!(
                    &event.kind,
                    RuntimeEventKind::PermissionDecided {
                        disposition: RuntimePermissionDisposition::Deny,
                        ..
                    }
                ) {
                    blocker_codes.push("runtime.permission.denied".to_owned());
                }
            }
            RuntimeEventKind::ToolRequested { tool_call_id, .. } => {
                tool_call_ids.push(tool_call_id.as_str().to_owned());
                current_operation_id = event
                    .operation_id
                    .as_ref()
                    .map(|value| value.as_str().to_owned());
            }
            RuntimeEventKind::AttemptEnded { .. } => current_operation_id = None,
            RuntimeEventKind::EffectObserved { attempt_id, .. } => {
                effect_attempt_ids.push(attempt_id.clone());
            }
            RuntimeEventKind::VerificationObserved {
                verification_id, ..
            } => verification_ids.push(verification_id.clone()),
            RuntimeEventKind::ExtractionBlocked { reason_code, .. } => {
                blocker_codes.push(reason_code.clone());
            }
            RuntimeEventKind::ModelFailed { failure_code, .. }
            | RuntimeEventKind::ToolFailed { failure_code, .. } => {
                blocker_codes.push(failure_code.clone());
            }
            RuntimeEventKind::TerminalDiagnostic {
                diagnostic_id,
                safe_next_action: action,
                ..
            } => {
                terminal_diagnostic_id = Some(diagnostic_id.clone());
                safe_next_action = Some(*action);
                blocker_codes.push("runtime.terminal.diagnostic".to_owned());
            }
            RuntimeEventKind::RunTerminal { state, .. } => terminal_state = Some(*state),
            _ => {}
        }
        facts.push(inspector_fact(event));
    }
    let mut view = ExecutionInspectorView {
        run_id: first.run_id.as_str().to_owned(),
        session_id: first.session_id.as_str().to_owned(),
        task_id: first.task_id.as_str().to_owned(),
        current_turn_id,
        current_operation_id,
        latest_route_id,
        latest_endpoint_class,
        proposal_ids,
        approval_ids,
        tool_call_ids,
        effect_attempt_ids,
        verification_ids,
        blocker_codes,
        artifacts,
        terminal_state,
        terminal_diagnostic_id,
        safe_next_action,
        cursor: RuntimeEventCursor {
            run_id: last.run_id.clone(),
            event_id: last.event_id.clone(),
            sequence: last.sequence,
            event_sha256: last.event_sha256.clone(),
        },
        facts,
        view_sha256: "0".repeat(64),
    };
    view.view_sha256 = sha256_json(&view);
    Ok(view)
}

/// Reconstructs and compares a materialized inspector view with canonical events.
pub fn verify_execution_inspector(
    events: &[RuntimeEvent],
    expected: &ExecutionInspectorView,
) -> Result<(), RuntimeEventError> {
    let reconstructed = build_execution_inspector(events)?;
    if reconstructed != *expected {
        return Err(RuntimeEventError::DigestMismatch);
    }
    Ok(())
}

fn inspector_fact(event: &RuntimeEvent) -> InspectorFact {
    let (kind, subject_id) = match &event.kind {
        RuntimeEventKind::RunStarted { .. } => (InspectorFactKind::Terminal, event.run_id.as_str()),
        RuntimeEventKind::SourceAdmitted {
            source_artifact_id, ..
        } => (InspectorFactKind::Capture, source_artifact_id.as_str()),
        RuntimeEventKind::ExtractionStarted { extraction_id, .. }
        | RuntimeEventKind::ExtractionCompleted { extraction_id, .. }
        | RuntimeEventKind::ExtractionBlocked { extraction_id, .. } => {
            (InspectorFactKind::Parse, extraction_id.as_str())
        }
        RuntimeEventKind::SectionIndexed { section_id, .. }
        | RuntimeEventKind::ContextDisposition {
            context_manifest_id: section_id,
            ..
        } => (InspectorFactKind::Context, section_id.as_str()),
        RuntimeEventKind::RouteSelected {
            route_decision_id, ..
        } => (InspectorFactKind::Route, route_decision_id.as_str()),
        RuntimeEventKind::ModelRequested { model_run_id, .. }
        | RuntimeEventKind::ModelCompleted { model_run_id, .. }
        | RuntimeEventKind::ModelFailed { model_run_id, .. } => {
            (InspectorFactKind::Model, model_run_id.as_str())
        }
        RuntimeEventKind::ProposalObserved { proposal_id, .. } => {
            (InspectorFactKind::Proposal, proposal_id.as_str())
        }
        RuntimeEventKind::PermissionRequested { approval_id, .. } => {
            (InspectorFactKind::Policy, approval_id.as_str())
        }
        RuntimeEventKind::PermissionDecided { approval_id, .. } => {
            (InspectorFactKind::Approval, approval_id.as_str())
        }
        RuntimeEventKind::ToolRequested { tool_call_id, .. }
        | RuntimeEventKind::ToolStarted { tool_call_id, .. }
        | RuntimeEventKind::ToolCompleted { tool_call_id, .. }
        | RuntimeEventKind::ToolFailed { tool_call_id, .. } => {
            (InspectorFactKind::Tool, tool_call_id.as_str())
        }
        RuntimeEventKind::PreflightObserved { preflight_id, .. } => {
            (InspectorFactKind::Policy, preflight_id.as_str())
        }
        RuntimeEventKind::AttemptStarted { attempt_id, .. }
        | RuntimeEventKind::AttemptEnded { attempt_id, .. }
        | RuntimeEventKind::EffectObserved { attempt_id, .. }
        | RuntimeEventKind::RetryDecided { attempt_id, .. } => {
            (InspectorFactKind::Effect, attempt_id.as_str())
        }
        RuntimeEventKind::VerificationObserved {
            verification_id, ..
        } => (InspectorFactKind::Verification, verification_id.as_str()),
        RuntimeEventKind::RecoveryDecided { recovery_id, .. } => {
            (InspectorFactKind::Recovery, recovery_id.as_str())
        }
        RuntimeEventKind::ArtifactCreated { artifact_id, .. } => {
            (InspectorFactKind::Artifact, artifact_id.as_str())
        }
        RuntimeEventKind::FileObserved { .. } | RuntimeEventKind::FileModified { .. } => {
            (InspectorFactKind::Effect, event.event_id.as_str())
        }
        RuntimeEventKind::CheckpointCommitted { checkpoint_id, .. } => {
            (InspectorFactKind::Checkpoint, checkpoint_id.as_str())
        }
        RuntimeEventKind::CancellationRequested { cancellation_id }
        | RuntimeEventKind::CancellationObserved { cancellation_id } => {
            (InspectorFactKind::Cancellation, cancellation_id.as_str())
        }
        RuntimeEventKind::TerminalDiagnostic { diagnostic_id, .. } => {
            (InspectorFactKind::Terminal, diagnostic_id.as_str())
        }
        RuntimeEventKind::RunTerminal { .. }
        | RuntimeEventKind::TurnStarted
        | RuntimeEventKind::TurnCompleted { .. } => {
            (InspectorFactKind::Terminal, event.event_id.as_str())
        }
        RuntimeEventKind::Progress { code } => (InspectorFactKind::Progress, code.as_str()),
        RuntimeEventKind::Metric { name, .. } => (InspectorFactKind::Metric, name.as_str()),
    };
    InspectorFact {
        sequence: event.sequence,
        event_id: event.event_id.as_str().to_owned(),
        kind,
        turn_id: event
            .turn_id
            .as_ref()
            .map(|value| value.as_str().to_owned()),
        operation_id: event
            .operation_id
            .as_ref()
            .map(|value| value.as_str().to_owned()),
        subject_id: subject_id.to_owned(),
        event_sha256: event.event_sha256.clone(),
        payload_reference: event.payload_reference.clone(),
    }
}

/// Complete measured performance facts for one exact fixture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimePerformanceObservation {
    /// Exact fixture identity.
    pub fixture_id: String,
    /// Capture latency in milliseconds.
    pub capture_ms: u64,
    /// Parsing latency in milliseconds.
    pub parsing_ms: u64,
    /// Retrieval latency in milliseconds.
    pub retrieval_ms: u64,
    /// Queue latency in milliseconds.
    pub queue_ms: u64,
    /// Time to first runtime event in milliseconds.
    pub first_event_ms: u64,
    /// Time to first generated token in milliseconds.
    pub first_token_ms: u64,
    /// Generation latency in milliseconds.
    pub generation_ms: u64,
    /// Tool execution latency in milliseconds.
    pub tool_ms: u64,
    /// Verification latency in milliseconds.
    pub verification_ms: u64,
    /// End-to-end latency in milliseconds.
    pub total_ms: u64,
    /// Sustained runtime-event throughput.
    pub throughput_events_per_second: u64,
    /// Peak process memory attributable to the fixture.
    pub peak_memory_bytes: u64,
    /// Whether accelerator memory is applicable to this fixture.
    pub accelerator_applicable: bool,
    /// Peak accelerator memory when applicable.
    pub accelerator_memory_bytes: Option<u64>,
    /// Durable disk growth attributable to the fixture.
    pub disk_bytes: u64,
    /// Peak process count attributable to the fixture.
    pub process_count: u32,
    /// Durable artifact bytes attributable to the fixture.
    pub artifact_bytes: u64,
    /// Peak queued runtime-event count.
    pub peak_backlog_events: u32,
}

/// Exact qualification ceilings for the same performance fixture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePerformanceBudget {
    /// Maximum capture latency in milliseconds.
    pub max_capture_ms: u64,
    /// Maximum parsing latency in milliseconds.
    pub max_parsing_ms: u64,
    /// Maximum retrieval latency in milliseconds.
    pub max_retrieval_ms: u64,
    /// Maximum queue latency in milliseconds.
    pub max_queue_ms: u64,
    /// Maximum time to first event in milliseconds.
    pub max_first_event_ms: u64,
    /// Maximum time to first token in milliseconds.
    pub max_first_token_ms: u64,
    /// Maximum generation latency in milliseconds.
    pub max_generation_ms: u64,
    /// Maximum tool latency in milliseconds.
    pub max_tool_ms: u64,
    /// Maximum verification latency in milliseconds.
    pub max_verification_ms: u64,
    /// Maximum end-to-end latency in milliseconds.
    pub max_total_ms: u64,
    /// Minimum sustained event throughput.
    pub min_throughput_events_per_second: u64,
    /// Maximum peak process memory.
    pub max_peak_memory_bytes: u64,
    /// Maximum accelerator memory when applicable.
    pub max_accelerator_memory_bytes: Option<u64>,
    /// Maximum attributable disk growth.
    pub max_disk_bytes: u64,
    /// Maximum attributable process count.
    pub max_process_count: u32,
    /// Maximum attributable durable artifact bytes.
    pub max_artifact_bytes: u64,
    /// Maximum queued runtime-event count.
    pub max_backlog_events: u32,
}

/// Recomputed qualification result; threshold failure carries no runtime authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuntimePerformanceQualification {
    /// Exact fixture identity.
    pub fixture_id: String,
    /// Whether every declared budget passed.
    pub passed: bool,
    /// Closed metric field names that exceeded their budgets.
    pub violations: Vec<String>,
    /// Digest of the complete observation.
    pub observation_sha256: String,
    /// Digest of the qualification with this field zeroed while hashing.
    pub qualification_sha256: String,
}

/// Qualifies every declared metric against its exact fixture-bound ceiling.
pub fn qualify_runtime_performance(
    observation: &RuntimePerformanceObservation,
    budget: &RuntimePerformanceBudget,
) -> Result<RuntimePerformanceQualification, RuntimeEventError> {
    if observation.fixture_id.is_empty()
        || observation.total_ms < observation.first_event_ms
        || observation.total_ms < observation.first_token_ms
        || observation.accelerator_applicable != observation.accelerator_memory_bytes.is_some()
        || observation.accelerator_applicable != budget.max_accelerator_memory_bytes.is_some()
    {
        return Err(RuntimeEventError::InvalidValue);
    }
    let mut violations = Vec::new();
    macro_rules! maximum {
        ($field:ident, $limit:ident) => {
            if observation.$field > budget.$limit {
                violations.push(stringify!($field).to_owned());
            }
        };
    }
    maximum!(capture_ms, max_capture_ms);
    maximum!(parsing_ms, max_parsing_ms);
    maximum!(retrieval_ms, max_retrieval_ms);
    maximum!(queue_ms, max_queue_ms);
    maximum!(first_event_ms, max_first_event_ms);
    maximum!(first_token_ms, max_first_token_ms);
    maximum!(generation_ms, max_generation_ms);
    maximum!(tool_ms, max_tool_ms);
    maximum!(verification_ms, max_verification_ms);
    maximum!(total_ms, max_total_ms);
    if observation.throughput_events_per_second < budget.min_throughput_events_per_second {
        violations.push("throughput_events_per_second".to_owned());
    }
    maximum!(peak_memory_bytes, max_peak_memory_bytes);
    if observation.accelerator_memory_bytes > budget.max_accelerator_memory_bytes {
        violations.push("accelerator_memory_bytes".to_owned());
    }
    maximum!(disk_bytes, max_disk_bytes);
    maximum!(process_count, max_process_count);
    maximum!(artifact_bytes, max_artifact_bytes);
    maximum!(peak_backlog_events, max_backlog_events);
    let mut result = RuntimePerformanceQualification {
        fixture_id: observation.fixture_id.clone(),
        passed: violations.is_empty(),
        violations,
        observation_sha256: sha256_json(observation),
        qualification_sha256: "0".repeat(64),
    };
    result.qualification_sha256 = sha256_json(&result);
    Ok(result)
}

fn sha256_json(value: &impl Serialize) -> String {
    Sha256::digest(serde_json::to_vec(value).expect("closed inspector value serializes"))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use agentmage_kernel_contracts::{
        AgentStateKind, ApprovalId, CONTRACT_SCHEMA_VERSION, ContextSensitivity, CorrelationId,
        EndpointClass, GrantId, GrantOperation, ModelRunId, PolicyId, ReceiptId, RuntimeArtifactId,
        RuntimeEvent, RuntimeEventId, RuntimeEventKind, RuntimeEventRetention,
        RuntimeEventRetentionKind, RuntimeOperationId, RuntimePayloadReference,
        RuntimePermissionDisposition, RuntimeRunId, RuntimeSafeNextAction, RuntimeTurnId,
        SessionId, TaskId, ToolCallId,
    };

    use super::{
        InspectorFactKind, MAX_INSPECTOR_EVENTS, RuntimePerformanceBudget,
        RuntimePerformanceObservation, build_execution_inspector, qualify_runtime_performance,
        verify_execution_inspector,
    };
    use crate::runtime_event::{RuntimeEventError, runtime_event_persistence, seal_runtime_event};

    const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    struct FixtureStream {
        next_sequence: u64,
        previous_sha256: String,
        causation_event_id: Option<RuntimeEventId>,
    }

    impl FixtureStream {
        fn new() -> Self {
            Self {
                next_sequence: 0,
                previous_sha256: ZERO_SHA256.to_owned(),
                causation_event_id: None,
            }
        }

        fn event(
            &mut self,
            kind: RuntimeEventKind,
            turn_id: Option<&str>,
            operation_id: Option<&str>,
        ) -> RuntimeEvent {
            let event_id = RuntimeEventId::from_raw(format!("event-21-4-{}", self.next_sequence));
            let payload_reference = match &kind {
                RuntimeEventKind::SourceAdmitted {
                    source_artifact_id, ..
                } => Some(payload(
                    source_artifact_id.clone(),
                    "application/octet-stream",
                )),
                RuntimeEventKind::ExtractionCompleted { .. }
                | RuntimeEventKind::TerminalDiagnostic { .. } => Some(payload(
                    RuntimeArtifactId::from_raw(format!("payload-21-4-{}", self.next_sequence)),
                    "application/json",
                )),
                _ => None,
            };
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: event_id.clone(),
                run_id: RuntimeRunId::from_raw("run-21-4"),
                session_id: SessionId::from_raw("session-21-4"),
                task_id: TaskId::from_raw("task-21-4"),
                turn_id: turn_id.map(RuntimeTurnId::from_raw),
                operation_id: operation_id.map(RuntimeOperationId::from_raw),
                correlation_id: CorrelationId::from_raw("correlation-21-4"),
                causation_event_id: self.causation_event_id.clone(),
                sequence: self.next_sequence,
                occurred_at_epoch_ms: 10_000 + self.next_sequence,
                sensitivity: ContextSensitivity::Internal,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Ephemeral,
                    expires_at_epoch_ms: None,
                },
                persistence: runtime_event_persistence(&kind),
                policy_id: PolicyId::from_raw("policy-21-4"),
                payload_reference,
                kind,
                previous_event_sha256: self.previous_sha256.clone(),
                event_sha256: ZERO_SHA256.to_owned(),
            })
            .expect("Story 21.4 fixture event must seal");
            self.next_sequence += 1;
            self.previous_sha256.clone_from(&event.event_sha256);
            self.causation_event_id = Some(event_id);
            event
        }
    }

    fn hash(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn payload(artifact_id: RuntimeArtifactId, media_type: &str) -> RuntimePayloadReference {
        RuntimePayloadReference {
            artifact_id,
            sha256: hash('a'),
            byte_size: 32,
            media_type: media_type.to_owned(),
        }
    }

    fn lineage() -> Vec<RuntimeEvent> {
        let mut stream = FixtureStream::new();
        let source = RuntimeArtifactId::from_raw("source-21-4");
        let tool_call = ToolCallId::from_raw("tool-call-21-4");
        vec![
            stream.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                None,
                None,
            ),
            stream.event(
                RuntimeEventKind::SourceAdmitted {
                    source_artifact_id: source.clone(),
                    manifest_sha256: hash('2'),
                },
                None,
                None,
            ),
            stream.event(
                RuntimeEventKind::ExtractionStarted {
                    source_artifact_id: source.clone(),
                    extraction_id: "extraction-21-4".to_owned(),
                    input_sha256: hash('3'),
                },
                None,
                None,
            ),
            stream.event(
                RuntimeEventKind::ExtractionCompleted {
                    source_artifact_id: source.clone(),
                    extraction_id: "extraction-21-4".to_owned(),
                    result_sha256: hash('4'),
                },
                None,
                None,
            ),
            stream.event(
                RuntimeEventKind::SectionIndexed {
                    source_artifact_id: source.clone(),
                    extraction_id: "extraction-21-4".to_owned(),
                    section_id: "section-21-4".to_owned(),
                    locator_sha256: hash('5'),
                },
                None,
                None,
            ),
            stream.event(
                RuntimeEventKind::ContextDisposition {
                    context_manifest_id: "context-21-4".to_owned(),
                    source_artifact_id: source,
                    disposition_sha256: hash('6'),
                },
                None,
                None,
            ),
            stream.event(RuntimeEventKind::TurnStarted, Some("turn-21-4"), None),
            stream.event(
                RuntimeEventKind::RouteSelected {
                    route_decision_id: "route-21-4".to_owned(),
                    endpoint_class: EndpointClass::StrictLocal,
                    decision_sha256: hash('7'),
                },
                Some("turn-21-4"),
                None,
            ),
            stream.event(
                RuntimeEventKind::ModelRequested {
                    model_run_id: ModelRunId::from_raw("model-21-4"),
                    request_sha256: hash('8'),
                },
                Some("turn-21-4"),
                None,
            ),
            stream.event(
                RuntimeEventKind::ModelCompleted {
                    model_run_id: ModelRunId::from_raw("model-21-4"),
                    result_sha256: hash('9'),
                },
                Some("turn-21-4"),
                None,
            ),
            stream.event(
                RuntimeEventKind::ProposalObserved {
                    proposal_id: "proposal-21-4".to_owned(),
                    proposal_sha256: hash('b'),
                },
                Some("turn-21-4"),
                None,
            ),
            stream.event(
                RuntimeEventKind::ToolRequested {
                    tool_call_id: tool_call.clone(),
                    arguments_sha256: hash('c'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::PermissionRequested {
                    approval_id: ApprovalId::from_raw("approval-21-4"),
                    operation: GrantOperation::WorkspaceWrite,
                    preview_sha256: hash('d'),
                    expires_at_epoch_ms: 20_000,
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::PermissionDecided {
                    approval_id: ApprovalId::from_raw("approval-21-4"),
                    disposition: RuntimePermissionDisposition::Allow,
                    grant_id: Some(GrantId::from_raw("grant-21-4")),
                    decision_sha256: hash('e'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::PreflightObserved {
                    attempt_id: "attempt-21-4".to_owned(),
                    preflight_id: "preflight-21-4".to_owned(),
                    observation_sha256: hash('f'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::AttemptStarted {
                    attempt_id: "attempt-21-4".to_owned(),
                    tool_call_id: tool_call.clone(),
                    prepared_sha256: hash('1'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::ToolStarted {
                    tool_call_id: tool_call.clone(),
                    authority_sha256: hash('2'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::ToolCompleted {
                    tool_call_id: tool_call.clone(),
                    receipt_id: ReceiptId::from_raw("receipt-21-4"),
                    result_sha256: hash('3'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::AttemptEnded {
                    attempt_id: "attempt-21-4".to_owned(),
                    tool_call_id: tool_call,
                    observation_id: "observation-21-4".to_owned(),
                    observation_sha256: hash('4'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::EffectObserved {
                    attempt_id: "attempt-21-4".to_owned(),
                    changed: true,
                    effect_sha256: hash('5'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::VerificationObserved {
                    attempt_id: "attempt-21-4".to_owned(),
                    verification_id: "verification-21-4".to_owned(),
                    result_sha256: hash('6'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::RetryDecided {
                    attempt_id: "attempt-21-4".to_owned(),
                    eligible: false,
                    decision_sha256: hash('7'),
                },
                Some("turn-21-4"),
                Some("operation-21-4"),
            ),
            stream.event(
                RuntimeEventKind::TurnCompleted {
                    outcome_sha256: hash('8'),
                },
                Some("turn-21-4"),
                None,
            ),
            stream.event(
                RuntimeEventKind::RecoveryDecided {
                    recovery_id: "recovery-21-4".to_owned(),
                    decision_sha256: hash('9'),
                },
                None,
                None,
            ),
            stream.event(
                RuntimeEventKind::TerminalDiagnostic {
                    diagnostic_id: "diagnostic-21-4".to_owned(),
                    diagnostic_sha256: hash('b'),
                    safe_next_action: RuntimeSafeNextAction::InspectEvidence,
                },
                None,
                None,
            ),
            stream.event(
                RuntimeEventKind::RunTerminal {
                    state: AgentStateKind::Failed,
                    outcome_sha256: hash('c'),
                },
                None,
                None,
            ),
        ]
    }

    #[test]
    fn story_21_4_inspector_reconstructs_complete_content_free_lineage() {
        let events = lineage();
        let view = build_execution_inspector(&events).expect("lineage must reconstruct");
        assert_eq!(view.run_id, "run-21-4");
        assert_eq!(view.latest_route_id.as_deref(), Some("route-21-4"));
        assert_eq!(view.latest_endpoint_class, Some(EndpointClass::StrictLocal));
        assert_eq!(view.proposal_ids, ["proposal-21-4"]);
        assert_eq!(view.approval_ids, ["approval-21-4"]);
        assert_eq!(view.tool_call_ids, ["tool-call-21-4"]);
        assert_eq!(view.effect_attempt_ids, ["attempt-21-4"]);
        assert_eq!(view.verification_ids, ["verification-21-4"]);
        assert_eq!(view.terminal_state, Some(AgentStateKind::Failed));
        assert_eq!(
            view.safe_next_action,
            Some(RuntimeSafeNextAction::InspectEvidence)
        );
        assert_eq!(view.facts.len(), events.len());
        for category in [
            InspectorFactKind::Capture,
            InspectorFactKind::Parse,
            InspectorFactKind::Context,
            InspectorFactKind::Route,
            InspectorFactKind::Model,
            InspectorFactKind::Proposal,
            InspectorFactKind::Policy,
            InspectorFactKind::Approval,
            InspectorFactKind::Tool,
            InspectorFactKind::Effect,
            InspectorFactKind::Verification,
            InspectorFactKind::Recovery,
            InspectorFactKind::Terminal,
        ] {
            assert!(view.facts.iter().any(|fact| fact.kind == category));
        }
        let json = serde_json::to_string(&view).expect("view serializes");
        assert!(!json.contains("raw prompt"));
        assert!(!json.contains("hidden reasoning"));
        assert!(!json.contains("/private/workspace"));
        verify_execution_inspector(&events, &view).expect("materialized view verifies");
    }

    #[test]
    fn story_21_4_inspector_supports_prefix_resume_and_refuses_tampering() {
        let events = lineage();
        let prefix = &events[..16];
        let first = build_execution_inspector(prefix).expect("valid prefix reconstructs");
        let reloaded = build_execution_inspector(prefix).expect("reload reconstructs identically");
        assert_eq!(first, reloaded);
        assert_eq!(first.current_turn_id.as_deref(), Some("turn-21-4"));
        assert_eq!(
            first.current_operation_id.as_deref(),
            Some("operation-21-4")
        );
        let mut tampered_view = first.clone();
        tampered_view
            .blocker_codes
            .push("invented.blocker".to_owned());
        assert_eq!(
            verify_execution_inspector(prefix, &tampered_view),
            Err(RuntimeEventError::DigestMismatch)
        );
        let mut reordered = events.clone();
        reordered.swap(8, 9);
        assert!(matches!(
            build_execution_inspector(&reordered),
            Err(RuntimeEventError::OrderingMismatch)
                | Err(RuntimeEventError::CausationMismatch)
                | Err(RuntimeEventError::IllegalTransition)
        ));
        assert_eq!(
            build_execution_inspector(&vec![events[0].clone(); MAX_INSPECTOR_EVENTS + 1]),
            Err(RuntimeEventError::BatchLimit)
        );

        let mut artifact_stream = FixtureStream::new();
        let shared_artifact = RuntimeArtifactId::from_raw("shared-artifact-21-4");
        let artifact_start = artifact_stream.event(
            RuntimeEventKind::RunStarted {
                request_sha256: hash('1'),
            },
            None,
            None,
        );
        let source = artifact_stream.event(
            RuntimeEventKind::SourceAdmitted {
                source_artifact_id: shared_artifact.clone(),
                manifest_sha256: hash('2'),
            },
            None,
            None,
        );
        let mut diagnostic = artifact_stream.event(
            RuntimeEventKind::TerminalDiagnostic {
                diagnostic_id: "diagnostic-conflict".to_owned(),
                diagnostic_sha256: hash('3'),
                safe_next_action: RuntimeSafeNextAction::InspectEvidence,
            },
            None,
            None,
        );
        diagnostic.payload_reference = Some(RuntimePayloadReference {
            artifact_id: shared_artifact,
            sha256: hash('f'),
            byte_size: 64,
            media_type: "application/json".to_owned(),
        });
        diagnostic = seal_runtime_event(diagnostic).expect("conflicting reference remains sealed");
        assert_eq!(
            build_execution_inspector(&[artifact_start, source, diagnostic]),
            Err(RuntimeEventError::DigestMismatch)
        );
    }

    #[test]
    fn story_21_4_lineage_gap_events_enforce_exact_ordering() {
        let mut route_stream = FixtureStream::new();
        let duplicate_route = vec![
            route_stream.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                None,
                None,
            ),
            route_stream.event(RuntimeEventKind::TurnStarted, Some("turn-route"), None),
            route_stream.event(
                RuntimeEventKind::RouteSelected {
                    route_decision_id: "route-first".to_owned(),
                    endpoint_class: EndpointClass::StrictLocal,
                    decision_sha256: hash('2'),
                },
                Some("turn-route"),
                None,
            ),
            route_stream.event(
                RuntimeEventKind::RouteSelected {
                    route_decision_id: "route-second".to_owned(),
                    endpoint_class: EndpointClass::RemotePrivate,
                    decision_sha256: hash('3'),
                },
                Some("turn-route"),
                None,
            ),
        ];
        assert_eq!(
            build_execution_inspector(&duplicate_route),
            Err(RuntimeEventError::IllegalTransition)
        );

        let mut proposal_stream = FixtureStream::new();
        let early_proposal = vec![
            proposal_stream.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                None,
                None,
            ),
            proposal_stream.event(RuntimeEventKind::TurnStarted, Some("turn-proposal"), None),
            proposal_stream.event(
                RuntimeEventKind::ProposalObserved {
                    proposal_id: "proposal-early".to_owned(),
                    proposal_sha256: hash('2'),
                },
                Some("turn-proposal"),
                None,
            ),
        ];
        assert_eq!(
            build_execution_inspector(&early_proposal),
            Err(RuntimeEventError::IllegalTransition)
        );

        let mut effect_stream = FixtureStream::new();
        let early_effect = vec![
            effect_stream.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: hash('1'),
                },
                None,
                None,
            ),
            effect_stream.event(RuntimeEventKind::TurnStarted, Some("turn-effect"), None),
            effect_stream.event(
                RuntimeEventKind::EffectObserved {
                    attempt_id: "attempt-early".to_owned(),
                    changed: false,
                    effect_sha256: hash('2'),
                },
                Some("turn-effect"),
                Some("operation-effect"),
            ),
        ];
        assert_eq!(
            build_execution_inspector(&early_effect),
            Err(RuntimeEventError::IllegalTransition)
        );
    }

    fn observation() -> RuntimePerformanceObservation {
        RuntimePerformanceObservation {
            fixture_id: "story-21-4-microfixture".to_owned(),
            capture_ms: 1,
            parsing_ms: 1,
            retrieval_ms: 1,
            queue_ms: 1,
            first_event_ms: 1,
            first_token_ms: 1,
            generation_ms: 1,
            tool_ms: 1,
            verification_ms: 1,
            total_ms: 10,
            throughput_events_per_second: 100,
            peak_memory_bytes: 1_024,
            accelerator_applicable: false,
            accelerator_memory_bytes: None,
            disk_bytes: 2_048,
            process_count: 1,
            artifact_bytes: 512,
            peak_backlog_events: 4,
        }
    }

    fn budget() -> RuntimePerformanceBudget {
        RuntimePerformanceBudget {
            max_capture_ms: 10,
            max_parsing_ms: 10,
            max_retrieval_ms: 10,
            max_queue_ms: 10,
            max_first_event_ms: 10,
            max_first_token_ms: 10,
            max_generation_ms: 10,
            max_tool_ms: 10,
            max_verification_ms: 10,
            max_total_ms: 10,
            min_throughput_events_per_second: 100,
            max_peak_memory_bytes: 1_024,
            max_accelerator_memory_bytes: None,
            max_disk_bytes: 2_048,
            max_process_count: 1,
            max_artifact_bytes: 512,
            max_backlog_events: 4,
        }
    }

    #[test]
    fn story_21_4_performance_qualification_is_complete_and_fail_closed() {
        let passing = qualify_runtime_performance(&observation(), &budget())
            .expect("valid fixture qualifies");
        assert!(passing.passed);
        assert!(passing.violations.is_empty());

        let mut cases = Vec::new();
        macro_rules! over {
            ($field:ident, $value:expr) => {{
                let mut candidate = observation();
                candidate.$field = $value;
                cases.push((stringify!($field), candidate));
            }};
        }
        over!(capture_ms, 11);
        over!(parsing_ms, 11);
        over!(retrieval_ms, 11);
        over!(queue_ms, 11);
        {
            let mut candidate = observation();
            candidate.first_event_ms = 11;
            candidate.total_ms = 11;
            cases.push(("first_event_ms", candidate));
        }
        {
            let mut candidate = observation();
            candidate.first_token_ms = 11;
            candidate.total_ms = 11;
            cases.push(("first_token_ms", candidate));
        }
        over!(generation_ms, 11);
        over!(tool_ms, 11);
        over!(verification_ms, 11);
        over!(total_ms, 11);
        over!(throughput_events_per_second, 99);
        over!(peak_memory_bytes, 1_025);
        over!(disk_bytes, 2_049);
        over!(process_count, 2);
        over!(artifact_bytes, 513);
        over!(peak_backlog_events, 5);
        for (expected, candidate) in cases {
            let result = qualify_runtime_performance(&candidate, &budget())
                .expect("well-formed threshold failure qualifies");
            assert!(!result.passed, "{expected} must fail");
            assert!(result.violations.iter().any(|value| value == expected));
        }

        let mut accelerated = observation();
        accelerated.accelerator_applicable = true;
        accelerated.accelerator_memory_bytes = Some(257);
        let mut accelerated_budget = budget();
        accelerated_budget.max_accelerator_memory_bytes = Some(256);
        let result = qualify_runtime_performance(&accelerated, &accelerated_budget)
            .expect("applicable accelerator fixture qualifies");
        assert_eq!(result.violations, ["accelerator_memory_bytes"]);

        let mut invalid = observation();
        invalid.accelerator_applicable = true;
        assert_eq!(
            qualify_runtime_performance(&invalid, &budget()),
            Err(RuntimeEventError::InvalidValue)
        );
    }

    #[test]
    fn story_21_4_measures_the_bounded_local_inspector_fixture() {
        let events = lineage();
        let started = Instant::now();
        let mut last_view = None;
        let mut sample_micros = Vec::with_capacity(100);
        for _ in 0..100 {
            let sample_started = Instant::now();
            last_view = Some(build_execution_inspector(&events).expect("fixture reconstructs"));
            sample_micros
                .push(u64::try_from(sample_started.elapsed().as_micros()).unwrap_or(u64::MAX));
        }
        let elapsed = started.elapsed();
        let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let elapsed_nanos = elapsed.as_nanos().max(1);
        let event_count = u128::try_from(events.len() * 100).expect("fixture count fits");
        let throughput = u64::try_from(event_count.saturating_mul(1_000_000_000) / elapsed_nanos)
            .unwrap_or(u64::MAX);
        let serialized_bytes = serde_json::to_vec(&last_view.expect("view exists"))
            .expect("view serializes")
            .len();
        let measured = RuntimePerformanceObservation {
            fixture_id: "story-21-4-local-inspector-100x".to_owned(),
            capture_ms: 0,
            parsing_ms: 0,
            retrieval_ms: elapsed_ms,
            queue_ms: 0,
            first_event_ms: elapsed_ms,
            first_token_ms: elapsed_ms,
            generation_ms: 0,
            tool_ms: 0,
            verification_ms: elapsed_ms,
            total_ms: elapsed_ms,
            throughput_events_per_second: throughput,
            peak_memory_bytes: u64::try_from(std::mem::size_of_val(events.as_slice()))
                .unwrap_or(u64::MAX),
            accelerator_applicable: false,
            accelerator_memory_bytes: None,
            disk_bytes: 0,
            process_count: 1,
            artifact_bytes: u64::try_from(serialized_bytes).unwrap_or(u64::MAX),
            peak_backlog_events: u32::try_from(events.len()).unwrap_or(u32::MAX),
        };
        let measured_budget = RuntimePerformanceBudget {
            max_capture_ms: 1,
            max_parsing_ms: 1,
            max_retrieval_ms: 30_000,
            max_queue_ms: 1,
            max_first_event_ms: 30_000,
            max_first_token_ms: 30_000,
            max_generation_ms: 1,
            max_tool_ms: 1,
            max_verification_ms: 30_000,
            max_total_ms: 30_000,
            min_throughput_events_per_second: 1,
            max_peak_memory_bytes: 1_048_576,
            max_accelerator_memory_bytes: None,
            max_disk_bytes: 0,
            max_process_count: 1,
            max_artifact_bytes: 1_048_576,
            max_backlog_events: 64,
        };
        let result = qualify_runtime_performance(&measured, &measured_budget)
            .expect("measured fixture is internally valid");
        assert!(result.passed, "measured fixture: {result:?}");
        sample_micros.sort_unstable();
        let percentile =
            |percent: usize| sample_micros[(sample_micros.len() * percent / 100).min(99)];
        eprintln!(
            "AGENTMAGE_STORY_21_4_PERFORMANCE=total_ms:{},events_per_second:{},p50_us:{},p95_us:{},p99_us:{},artifact_bytes:{}",
            elapsed_ms,
            throughput,
            percentile(50),
            percentile(95),
            percentile(99),
            serialized_bytes
        );
    }
}
