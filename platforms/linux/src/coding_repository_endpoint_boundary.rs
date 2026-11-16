// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scoped coordinator adapter for one original native prepared edit.

use super::*;
use agentmage_kernel_contracts::{
    EvidenceId, EvidenceKind, EvidenceReference, ModelCancellationProbe, ModelOperationStop,
    OperationOutcome, ReceiptId, RuntimeArtifactKind, RuntimeRunRequest, StateChange, ToolResult,
    to_canonical_json,
};
use agentmage_kernel_engine::runtime_loop::{
    RuntimeClock, RuntimePermissionEvaluation, RuntimePortFailure, RuntimeRunScope,
    RuntimeToolBoundary, RuntimeToolExecution,
};
use std::sync::Arc;

#[derive(Default)]
enum Stage {
    #[default]
    Ready,
    Asked,
    Issued {
        evaluation: Box<RuntimePermissionEvaluation>,
        resolved_at: u64,
    },
    Spent,
}

/// One process-held scoped tool boundary over the original prepared native edit.
/// Trusted startup supplies the original shared continuous control and epoch clock.
/// No source reads, model admission, test execution, journal or recovery is added.
/// Resolving Allow issues the original grant but performs no file write;
/// execution consumes that grant through the original atomic transaction once.
pub struct LinuxCodingEndpointEditBoundary<C> {
    edit: LinuxCodingEndpointEdit,
    control: Arc<dyn ModelOperationControl + Send + Sync>,
    clock: C,
    stage: Stage,
}

impl LinuxCodingEndpointEdit {
    /// Retains this live owner as a coordinator boundary. This does not activate
    /// a service or approve anything. The control must be the original run's
    /// continuously retained control, never a freshly renewed operation timer.
    pub fn into_boundary<C: RuntimeClock>(
        self,
        control: Arc<dyn ModelOperationControl + Send + Sync>,
        clock: C,
    ) -> LinuxCodingEndpointEditBoundary<C> {
        let stage = if self.spent {
            Stage::Spent
        } else {
            Stage::Ready
        };
        LinuxCodingEndpointEditBoundary {
            edit: self,
            control,
            clock,
            stage,
        }
    }
}

impl<C: RuntimeClock> LinuxCodingEndpointEditBoundary<C> {
    /// Original native preview and last dispatch/result, including uncertainty.
    pub fn inspect(&self) -> Result<Value> {
        self.edit.inspect()
    }

    /// Abandons any pending decision/grant and returns the same live owner.
    /// Existing result/lineage remains inspectable; no pending edit is restored.
    #[must_use]
    pub fn into_repository(self) -> LinuxCodingRepository {
        self.edit.into_repository()
    }

    fn check(
        &self,
        scope: RuntimeRunScope<'_>,
        definition: &ToolDefinition,
        call: &ToolCall,
    ) -> std::result::Result<(), RuntimePortFailure> {
        if scope.endpoint_request() != Some(&self.edit.request)
            || definition != &EndpointEditTool::new().0
            || call != &self.edit.call
        {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(())
    }

    fn running(&self) -> std::result::Result<(), RuntimePortFailure> {
        self.control
            .remaining_ms()
            .map(|_| ())
            .map_err(|stop| match stop {
                ModelOperationStop::Cancelled => RuntimePortFailure::Cancelled,
                ModelOperationStop::TimedOut => RuntimePortFailure::TimedOut,
                ModelOperationStop::Unavailable => RuntimePortFailure::Unavailable,
                ModelOperationStop::Invalid => RuntimePortFailure::Invalid,
            })
    }

    fn current(&self, now: u64) -> std::result::Result<(), RuntimePortFailure> {
        let pending = self
            .edit
            .repository
            .edit
            .as_ref()
            .ok_or(RuntimePortFailure::Invalid)?;
        if now < pending.created_at || now >= self.edit.challenge.expires_at_epoch_ms {
            return Err(RuntimePortFailure::Invalid);
        }
        if self
            .edit
            .inspect()
            .map_err(|_| RuntimePortFailure::Unavailable)?["pending"]
            != self.edit.preview
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let created = self
            .edit
            .repository
            .verify_edit_scope(&pending.scope)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        if created.creation_receipt_sha256 != pending.creation_receipt_sha256 {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(())
    }
}

impl<C: RuntimeClock> RuntimeToolBoundary for LinuxCodingEndpointEditBoundary<C> {
    fn evaluate_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        operation: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.check(scope, definition, call)?;
        if !matches!(self.stage, Stage::Ready) || operation != &self.edit.challenge.operation_id {
            return Err(RuntimePortFailure::Invalid);
        }
        // Matching entry is retired before any control/native dependency call.
        self.stage = Stage::Spent;
        self.running()?;
        self.current(now)?;
        self.stage = Stage::Asked;
        let c = &self.edit.challenge;
        Ok(RuntimePermissionEvaluation::Ask {
            approval_id: c.approval_id.clone(),
            grant_id: c.proposed_grant_id.clone(),
            preview_sha256: c.preview_sha256.clone(),
            expires_at_epoch_ms: c.expires_at_epoch_ms,
        })
    }

    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.check(scope, definition, call)?;
        if !matches!(self.stage, Stage::Asked)
            || challenge != &self.edit.challenge
            || response.disposition == RuntimeApprovalDisposition::Narrow
        {
            return Err(RuntimePortFailure::Invalid);
        }
        verify_runtime_approval_response(challenge, response, now)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        // An invalid response is inert. A matching decision is spent before
        // control, native dependencies or grant issuance, including unwind.
        self.stage = Stage::Spent;
        self.edit.spent = true;
        self.running()?;
        self.current(now)?;
        let decision_sha256 =
            hash(&to_canonical_json(response).map_err(|_| RuntimePortFailure::Invalid)?);
        if response.disposition == RuntimeApprovalDisposition::Deny {
            self.edit
                .repository
                .decide_edit(&challenge.preview_sha256, false, now)
                .map_err(|_| RuntimePortFailure::Unavailable)?;
            return Ok(RuntimePermissionEvaluation::Deny {
                approval_id: challenge.approval_id.clone(),
                grant_id: challenge.proposed_grant_id.clone(),
                preview_sha256: challenge.preview_sha256.clone(),
                expires_at_epoch_ms: challenge.expires_at_epoch_ms,
                decision_sha256,
                reason_code: "code.endpoint-edit.user-denied".into(),
            });
        }
        self.running()?;
        let pending = self
            .edit
            .repository
            .edit
            .as_mut()
            .ok_or(RuntimePortFailure::Invalid)?;
        let changes = pending.preview["write_preview"]["change_set_sha256"]
            .as_str()
            .ok_or(RuntimePortFailure::Invalid)?;
        let preview = pending.preview["write_preview"]["preview_sha256"]
            .as_str()
            .ok_or(RuntimePortFailure::Invalid)?;
        let approval = pending
            .write
            .approve(changes, preview, now)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        if approval.grant.grant_id != challenge.proposed_grant_id {
            return Err(RuntimePortFailure::Invalid);
        }
        let evaluation = RuntimePermissionEvaluation::Allow {
            approval_id: challenge.approval_id.clone(),
            grant_id: approval.grant.grant_id.clone(),
            preview_sha256: challenge.preview_sha256.clone(),
            expires_at_epoch_ms: challenge.expires_at_epoch_ms,
            decision_sha256,
            authority_sha256: approval.binding_sha256.clone(),
        };
        self.stage = Stage::Issued {
            evaluation: Box::new(evaluation.clone()),
            resolved_at: now,
        };
        Ok(evaluation)
    }

    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
        self.check(scope, definition, call)?;
        let Stage::Issued {
            evaluation: issued,
            resolved_at,
        } = &self.stage
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        if issued.as_ref() != evaluation {
            return Err(RuntimePortFailure::Invalid);
        }
        let resolved_at = *resolved_at;
        self.stage = Stage::Spent;
        self.running()?;
        if let Some(probe) = cancellation {
            match probe
                .observe()
                .map_err(|_| RuntimePortFailure::Unavailable)?
            {
                None => (),
                Some(signal)
                    if signal.task_id == self.edit.request.task.task_id
                        && signal.correlation_id == call.correlation_id =>
                {
                    return Err(RuntimePortFailure::Cancelled);
                }
                Some(_) => return Err(RuntimePortFailure::Invalid),
            }
        }
        let now = self.clock.now_epoch_ms()?;
        if now < resolved_at {
            return Err(RuntimePortFailure::Invalid);
        }
        self.current(now)?;
        self.running()?;
        // Original transaction remains non-interruptible after admission. Any
        // stop after this point cannot establish no effect or allow replay.
        let started = Instant::now();
        let value = self
            .edit
            .repository
            .decide_edit(&self.edit.challenge.preview_sha256, true, now)
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        project(
            &self.edit.request,
            definition,
            call,
            value,
            started.elapsed(),
        )
    }

    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> std::result::Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
}

fn project(
    request: &RuntimeEndpointRunRequest,
    definition: &ToolDefinition,
    call: &ToolCall,
    value: Value,
    elapsed: Duration,
) -> std::result::Result<RuntimeToolExecution, RuntimePortFailure> {
    let (outcome, state_change) = match value["outcome"].as_str() {
        Some("committed") => (OperationOutcome::Succeeded, StateChange::Changed),
        Some("failed_no_change" | "restored") => {
            (OperationOutcome::Failed, StateChange::NotChanged)
        }
        Some("uncertain") => (OperationOutcome::Uncertain, StateChange::Uncertain),
        _ => return Err(RuntimePortFailure::Uncertain),
    };
    // Project the actual final operation-chain receipt; the full native chain
    // and consumed-grant digest remain in the output. No new authority receipt.
    let last = value["write_result"]["receipts"]
        .as_array()
        .and_then(|r| r.last())
        .ok_or(RuntimePortFailure::Uncertain)?;
    let transaction = last["transaction_id"]
        .as_str()
        .ok_or(RuntimePortFailure::Uncertain)?;
    let sequence = last["sequence"]
        .as_u64()
        .ok_or(RuntimePortFailure::Uncertain)?;
    let receipt_id = ReceiptId::from_raw(format!("{transaction}:{sequence}"));
    let receipt_sha256 = last["receipt_sha256"]
        .as_str()
        .ok_or(RuntimePortFailure::Uncertain)?
        .to_owned();
    let bytes = serde_json::to_vec(&value).map_err(|_| RuntimePortFailure::Uncertain)?;
    if bytes.len() as u64 > request.limits.max_output_bytes {
        return Err(RuntimePortFailure::Uncertain);
    }
    let digest = hash(&bytes);
    Ok(RuntimeToolExecution {
        receipt_id,
        receipt_sha256,
        result: ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: call.tool_call_id.clone(),
            correlation_id: call.correlation_id.clone(),
            outcome,
            output: Some(ContractPayload {
                schema: definition.output_schema.clone(),
                media_type: "application/json".into(),
                sha256: digest.clone(),
                bytes,
            }),
            validation_issues: vec![],
            evidence: vec![EvidenceReference {
                schema_version: CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw(format!("native-edit:{}", &digest[..32])),
                kind: EvidenceKind::ToolOutput,
                source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
                object_id: call.tool_call_id.as_str().into(),
                fragment: None,
                content_sha256: digest,
                observed_revision: Some(request.repository_snapshot_id.as_str().into()),
            }],
            error: None,
            elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
            state_change,
        },
        result_output_kind: Some(RuntimeArtifactKind::Report),
        artifact_candidates: vec![],
    })
}
