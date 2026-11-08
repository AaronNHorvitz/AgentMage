// SPDX-License-Identifier: Apache-2.0
//! Existing runtime event publication for explicitly admitted endpoint turns.

use agentmage_kernel_contracts::{
    CorrelationId, ModelRunId, RuntimeEndpointRunRequest, RuntimeEvent, RuntimeEventCursor,
    RuntimeEventKind, RuntimeEventRetentionKind, RuntimeTurnId, to_canonical_json,
};

use crate::runtime_event::{
    RuntimeEventBatch, RuntimeEventBatchLimits, RuntimeEventError, RuntimeEventPublisher,
    RuntimeEventSubscription, replay_runtime_events,
};
use crate::runtime_hardening::RuntimeResourceLedger;
use crate::runtime_loop::{RuntimePortFailure, event_frame::RuntimeEventScope};

pub(super) struct RuntimeEndpointTurnEvents {
    publisher: RuntimeEventPublisher,
    events: Vec<RuntimeEvent>,
}

pub(super) struct EndpointEventFrame<'a> {
    pub request: &'a RuntimeEndpointRunRequest,
    pub correlation: &'a CorrelationId,
}

impl RuntimeEndpointTurnEvents {
    pub fn new(
        frame: &EndpointEventFrame<'_>,
        resources: &mut RuntimeResourceLedger,
        now: u64,
    ) -> Result<Self, RuntimePortFailure> {
        let mut value = Self {
            publisher: RuntimeEventPublisher::new(),
            events: Vec::new(),
        };
        value.append(
            frame,
            resources,
            now,
            RuntimeEventKind::RunStarted {
                request_sha256: frame.request.request_sha256.clone(),
            },
            None,
        )?;
        Ok(value)
    }

    /// Check all four prospective envelopes before any dispatch or publication.
    /// Use the longest legal timestamp and both possible completion shapes;
    /// actual counts/bytes alone are committed on append. No plaintext enters.
    pub fn preflight(
        &self,
        frame: &EndpointEventFrame<'_>,
        resources: &RuntimeResourceLedger,
        turn: &RuntimeTurnId,
        model: &ModelRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimePortFailure> {
        let mut planned = resources.clone();
        let mut previous = self.events.last().cloned();
        let digest = "0".repeat(64);
        let kinds = [
            vec![RuntimeEventKind::TurnStarted],
            vec![RuntimeEventKind::ModelRequested {
                model_run_id: model.clone(),
                request_sha256: request_sha256.to_owned(),
            }],
            vec![
                RuntimeEventKind::ModelCompleted {
                    model_run_id: model.clone(),
                    result_sha256: digest.clone(),
                },
                RuntimeEventKind::ModelFailed {
                    model_run_id: model.clone(),
                    failure_code: "runtime.model.incomplete.unknown".to_owned(),
                },
            ],
            vec![RuntimeEventKind::TurnCompleted {
                outcome_sha256: digest,
            }],
        ];
        for (offset, alternatives) in kinds.into_iter().enumerate() {
            let mut largest = None;
            for kind in alternatives {
                let event = frame.seal(
                    (self.events.len() + offset) as u64,
                    previous.as_ref(),
                    u64::MAX,
                    kind,
                    Some(turn),
                )?;
                let size = event_size(&event)?;
                if largest.as_ref().is_none_or(|(bytes, _)| size > *bytes) {
                    largest = Some((size, event));
                }
            }
            let (size, event) = largest.ok_or(RuntimePortFailure::Invalid)?;
            planned
                .admit_event(size)
                .map_err(|_| RuntimePortFailure::ResourceExhausted)?;
            previous = Some(event);
        }
        Ok(())
    }

    pub fn append(
        &mut self,
        frame: &EndpointEventFrame<'_>,
        resources: &mut RuntimeResourceLedger,
        now: u64,
        kind: RuntimeEventKind,
        turn: Option<&RuntimeTurnId>,
    ) -> Result<u64, RuntimePortFailure> {
        let event = frame.seal(
            self.events.len() as u64,
            self.events.last(),
            now,
            kind,
            turn,
        )?;
        let size = event_size(&event)?;
        let mut admitted = resources.clone();
        admitted
            .admit_event(size)
            .map_err(|_| RuntimePortFailure::ResourceExhausted)?;
        self.publisher
            .publish(event.clone())
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.events.push(event);
        *resources = admitted;
        Ok(size)
    }

    pub fn events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    pub fn subscribe(
        &self,
        capacity: usize,
    ) -> Result<RuntimeEventSubscription, RuntimeEventError> {
        self.publisher.subscribe(capacity)
    }

    pub fn replay(
        &self,
        after: Option<&RuntimeEventCursor>,
        limits: RuntimeEventBatchLimits,
    ) -> Result<RuntimeEventBatch, RuntimeEventError> {
        replay_runtime_events(&self.events, after, limits)
    }
}

impl EndpointEventFrame<'_> {
    fn seal(
        &self,
        sequence: u64,
        previous: Option<&RuntimeEvent>,
        now: u64,
        kind: RuntimeEventKind,
        turn: Option<&RuntimeTurnId>,
    ) -> Result<RuntimeEvent, RuntimePortFailure> {
        RuntimeEventScope::endpoint(self.request)
            .seal(
                self.correlation,
                sequence,
                previous,
                now,
                kind,
                turn,
                None,
                None,
                RuntimeEventRetentionKind::Ephemeral,
            )
            .map_err(|_| RuntimePortFailure::Invalid)
    }
}

fn event_size(event: &RuntimeEvent) -> Result<u64, RuntimePortFailure> {
    to_canonical_json(event)
        .map(|bytes| bytes.len() as u64)
        .map_err(|_| RuntimePortFailure::Invalid)
}
