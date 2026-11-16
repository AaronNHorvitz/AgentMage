// SPDX-License-Identifier: MIT OR Apache-2.0
//! Shared event framing for native runs and selected-endpoint model turns.

use agentmage_kernel_contracts::{
    ContextSensitivity, DataSensitivity, PolicyId, RuntimeEndpointRunRequest, RuntimeRunId,
    SessionId, TaskId,
};

use super::*;

/// Borrow only event identity. This cannot convert endpoint selection into a
/// native request, grant, tokenizer observation or task-success assertion.
pub(super) struct RuntimeEventScope<'a> {
    pub run_id: &'a RuntimeRunId,
    pub session_id: &'a SessionId,
    pub task_id: &'a TaskId,
    pub policy_id: &'a PolicyId,
    pub sensitivity: ContextSensitivity,
}

impl<'a> RuntimeEventScope<'a> {
    pub fn native(request: &'a RuntimeRunRequest) -> Self {
        Self {
            run_id: &request.run_id,
            session_id: &request.session_id,
            task_id: &request.task.task_id,
            policy_id: &request.policy_id,
            sensitivity: event_sensitivity(request.work_packet.sensitivity),
        }
    }

    pub fn endpoint(request: &'a RuntimeEndpointRunRequest) -> Self {
        Self {
            run_id: &request.run_id,
            session_id: &request.session_id,
            task_id: &request.task.task_id,
            policy_id: &request.policy_id,
            sensitivity: event_sensitivity(request.work_packet.sensitivity),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        &self,
        correlation_id: &CorrelationId,
        sequence: u64,
        previous: Option<&RuntimeEvent>,
        occurred_at_epoch_ms: u64,
        kind: RuntimeEventKind,
        turn_id: Option<&RuntimeTurnId>,
        operation_id: Option<&RuntimeOperationId>,
        payload_reference: Option<RuntimePayloadReference>,
        retention: RuntimeEventRetentionKind,
    ) -> Result<RuntimeEvent, RuntimeEventError> {
        seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw(derived_id("event", self.run_id.as_str(), sequence)),
            run_id: self.run_id.clone(),
            session_id: self.session_id.clone(),
            task_id: self.task_id.clone(),
            turn_id: turn_id.cloned(),
            operation_id: operation_id.cloned(),
            correlation_id: correlation_id.clone(),
            causation_event_id: previous.map(|event| event.event_id.clone()),
            sequence,
            occurred_at_epoch_ms,
            sensitivity: self.sensitivity,
            retention: RuntimeEventRetention {
                kind: retention,
                expires_at_epoch_ms: None,
            },
            persistence: runtime_event_persistence(&kind),
            policy_id: self.policy_id.clone(),
            payload_reference,
            kind,
            previous_event_sha256: previous.map_or_else(
                || ZERO_SHA256.to_owned(),
                |event| event.event_sha256.clone(),
            ),
            event_sha256: ZERO_SHA256.to_owned(),
        })
    }
}

pub(super) fn event_sensitivity(sensitivity: DataSensitivity) -> ContextSensitivity {
    match sensitivity {
        DataSensitivity::Ephemeral => ContextSensitivity::Private,
        DataSensitivity::Operational | DataSensitivity::Durable => ContextSensitivity::Internal,
        DataSensitivity::Restricted => ContextSensitivity::Restricted,
    }
}
