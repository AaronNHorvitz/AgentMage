//! Linux approval, authority, and effect boundary for native coding runtime calls.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Cursor;

use agentmage_capability_read_only::{
    GitCommandPlan, GitInspectionOperation, GitInspectionOutcome, GitInspectionRequest,
    GitInspectionResult, ReadOnlyOutcome, ReadOnlyResult, parse_git_inspection,
};
use agentmage_kernel_contracts::{
    ActionKind, ActorId, ApprovalId, ApprovalRequest, AuthorityTransactionId,
    AuthorizedWorkspaceHandle, ContractPayload, DataSensitivity, EvidenceId, EvidenceKind,
    EvidenceReference, GrantId, GrantNonce, GrantOperation, OperationAttemptId, OperationOutcome,
    PlanStepId, ReceiptId, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeArtifactKind, RuntimeArtifactManifest, RuntimeArtifactRef,
    RuntimeEvent, RuntimeEventKind, RuntimeOperationId, RuntimeResumeBinding, RuntimeRunId,
    RuntimeRunRequest, RuntimeSessionMode, SessionCheckpoint, SessionCheckpointId, SessionId,
    StateChange, ToolCall, ToolDefinition, ToolResult, to_canonical_json,
};
use agentmage_kernel_engine::{
    authority_transaction::AuthorityTransactionRequest,
    command_runner::{
        BoundedCommandExecutor, CommandCapturedOutput, CommandEffectDriver, CommandReceipt,
        RegisteredCommandWrapperBinding, verify_command_receipt,
    },
    context_management::finalize_checkpoint,
    filesystem_control::{
        FileClassification, FilesystemApprovalDecision, FilesystemApprovalPreview,
        FilesystemApprovalReceipt, FilesystemGrantRequest, FilesystemOperationDraft,
        FilesystemPlan, FilesystemPlanRequest, FilesystemTransactionOutcome,
        FilesystemTransactionRequest, build_filesystem_plan, render_filesystem_preview,
        verify_filesystem_receipts,
    },
    grants::SessionReadGrantRequest,
    operational_store::{
        DurableAuthorityError, PendingRuntimeEffectCommit, PendingSpecializedEffectCommit,
    },
    policy::PolicyEvaluationContext,
    propagation::CancellationToken,
    repository_inspection::{
        BoundedRepositoryInspectionExecutor, PreparedRepositoryInspection,
        RepositoryInspectionEffectDriver, RepositoryInspectionOperation,
        RepositoryInspectionPlatformResult, RepositoryInspectionRequest,
        RepositoryInspectionTermination, prepare_repository_inspection,
    },
    runtime_artifact::{
        MAX_RUNTIME_ARTIFACT_BYTES, RUNTIME_CONTINUATION_MEDIA_TYPE, RuntimeArtifactReadRequest,
        decode_runtime_continuation_state, seal_runtime_resume_binding,
        verify_runtime_continuation_state,
    },
    runtime_coordinator::{verify_runtime_approval_response, verify_runtime_run_request},
    runtime_journal::RuntimeJournalError,
    runtime_loop::{
        RuntimeArtifactPort, RuntimeCheckpointCommit, RuntimeCheckpointPort,
        RuntimeCheckpointPublication, RuntimeCorrectnessTransactionPort, RuntimeJournalPort,
        RuntimePermissionEvaluation, RuntimePortFailure, RuntimeResumeSnapshot,
        RuntimeToolArtifactCandidate, RuntimeToolBoundary, RuntimeToolCorrectnessCommit,
        RuntimeToolExecution,
    },
    validation_result::{
        ValidationObservation, ValidationOutputClassification, ValidationReceipt, ValidationStatus,
        normalize_validation_result, verify_validation_receipt,
    },
    write_approval::{
        ShadowChangeSet, WriteApprovalDecision, WriteApprovalPreview, WriteApprovalReceipt,
        WriteChangeScope, WriteGrantRequest, WriteReviewNarrative, render_write_preview,
    },
    write_transaction::{WriteTransactionOutcome, WriteTransactionRequest, verify_write_receipts},
};
use agentmage_platform_linux::{
    LinuxAtomicWriteDriver, LinuxAtomicWriteDriverLimits, LinuxAuthorityRuntime,
    LinuxAuthorizedWorkspace, LinuxControlledFilesystemDriver, LinuxFilesystemDriverLimits,
    LinuxReadOnlyToolEffectDriver, LinuxReadOnlyToolInput, LinuxSandboxRunner,
};
use rustix::rand::{GetRandomFlags, getrandom};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    code_change::{
        BoundStructuredChange, StructuredShadowChangeSetRequest, build_structured_shadow_change_set,
    },
    coding_authority::{
        ApprovedCodingGrant, ApprovedCodingGrantRequest, CodingApprovalRequest,
        CodingRuntimePolicy, derive_approved_coding_grant, derive_approved_coding_grant_with_event,
        render_coding_approval_request,
    },
    coding_dispatch::PreparedNativeCodingCall,
    linux_coding::{
        LinuxCodingTargetBinding, LinuxCodingWorkspace, LinuxCodingWriteDraft,
        PreparedLinuxCodingOperation,
    },
};

const PREVIEW_LIFETIME_MS: u64 = 60_000;
const MAX_PENDING_OPERATIONS: usize = 8;

#[cfg(test)]
const STORY_22_1_CRASH_ENVIRONMENT: &str = "AGENTMAGE_STORY_22_1_CRASH_BOUNDARY";

#[cfg(test)]
fn story_22_1_crash_at(boundary: &str) {
    if std::env::var(STORY_22_1_CRASH_ENVIRONMENT).as_deref() == Ok(boundary) {
        std::process::exit(93);
    }
}

/// Stable content-free failure while composing the Linux coding boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCodingRuntimeError {
    /// The immutable workspace, profile, or policy bindings disagree.
    BindingDenied,
    /// A required random identity could not be obtained.
    IdentityUnavailable,
}

impl LinuxCodingRuntimeError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BindingDenied => "runtime.linux-coding.binding-denied",
            Self::IdentityUnavailable => "runtime.linux-coding.identity-unavailable",
        }
    }
}

/// CSPRNG identity boundary, injectable for deterministic runtime tests.
pub trait CodingIdentitySource {
    /// Returns one unique bounded identifier with the requested stable prefix.
    fn next(&mut self, prefix: &str) -> Result<String, LinuxCodingRuntimeError>;
}

/// Operating-system random identity source used by the packaged Linux host.
#[derive(Debug, Default)]
pub struct OsCodingIdentitySource;

impl CodingIdentitySource for OsCodingIdentitySource {
    fn next(&mut self, prefix: &str) -> Result<String, LinuxCodingRuntimeError> {
        let mut random = [0_u8; 16];
        let count = getrandom(&mut random, GetRandomFlags::empty())
            .map_err(|_| LinuxCodingRuntimeError::IdentityUnavailable)?;
        if count != random.len() {
            return Err(LinuxCodingRuntimeError::IdentityUnavailable);
        }
        Ok(format!("{prefix}-{}", hex(&random)))
    }
}

enum PendingCodingAuthority {
    Generic(Box<ApprovalRequest>),
    StructuredWrite {
        approval_id: ApprovalId,
        proposed_grant_id: GrantId,
        parent_grant_id: GrantId,
        preview: Box<WriteApprovalPreview>,
        change_set: Box<ShadowChangeSet>,
    },
    FilesystemWrite {
        approval_id: ApprovalId,
        proposed_grant_id: GrantId,
        parent_grant_id: GrantId,
        preview: Box<FilesystemApprovalPreview>,
        plan: Box<FilesystemPlan>,
    },
}

impl PendingCodingAuthority {
    fn approval_id(&self) -> &ApprovalId {
        match self {
            Self::Generic(approval) => &approval.approval_id,
            Self::StructuredWrite { approval_id, .. }
            | Self::FilesystemWrite { approval_id, .. } => approval_id,
        }
    }

    fn proposed_grant_id(&self) -> &GrantId {
        match self {
            Self::Generic(approval) => &approval.proposed_grant_id,
            Self::StructuredWrite {
                proposed_grant_id, ..
            }
            | Self::FilesystemWrite {
                proposed_grant_id, ..
            } => proposed_grant_id,
        }
    }

    fn preview_sha256(&self) -> &str {
        match self {
            Self::Generic(approval) => &approval.confirmation_sha256,
            Self::StructuredWrite { preview, .. } => &preview.preview_sha256,
            Self::FilesystemWrite { preview, .. } => &preview.preview_sha256,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_pending_coding_authority(
    parent: &agentmage_kernel_contracts::CapabilityGrant,
    registry: &agentmage_kernel_engine::tooling::ToolRegistry,
    prepared: &PreparedLinuxCodingOperation<'_>,
    call: &ToolCall,
    operation_id: &RuntimeOperationId,
    approval_id: ApprovalId,
    proposed_grant_id: GrantId,
    plan_id: String,
    now_epoch_ms: u64,
    expires_at_epoch_ms: u64,
) -> Result<PendingCodingAuthority, RuntimePortFailure> {
    match prepared.write_draft() {
        Some(LinuxCodingWriteDraft::StructuredPatch(plan)) => {
            let LinuxCodingTargetBinding::ExistingFile { target, .. } = prepared.binding() else {
                return Err(RuntimePortFailure::Invalid);
            };
            let summary = plan.summary();
            let change_set = build_structured_shadow_change_set(
                parent,
                StructuredShadowChangeSetRequest {
                    change_set_id: plan_id,
                    observed_at_epoch_ms: now_epoch_ms,
                    intent_sha256: summary.intent_sha256.clone(),
                    change_plan_sha256: summary.change_plan_sha256.clone(),
                    scope: WriteChangeScope::Minimal,
                    expanded_scope_approval_sha256: None,
                    changes: vec![BoundStructuredChange {
                        operation_id: operation_id.as_str().to_owned(),
                        target: target.clone(),
                        plan: plan.clone(),
                    }],
                    review: coding_write_review(true, prepared.operation().plan_sha256()),
                    permitted_verification: coding_write_verification(),
                },
            )
            .map_err(|_| RuntimePortFailure::Invalid)?;
            let preview =
                render_write_preview(&change_set).map_err(|_| RuntimePortFailure::Invalid)?;
            Ok(PendingCodingAuthority::StructuredWrite {
                approval_id,
                proposed_grant_id,
                parent_grant_id: parent.grant_id.clone(),
                preview: Box::new(preview),
                change_set: Box::new(change_set),
            })
        }
        Some(LinuxCodingWriteDraft::ControlledCreate(draft)) => {
            let plan = build_filesystem_plan(
                parent,
                FilesystemPlanRequest {
                    plan_id,
                    observed_at_epoch_ms: now_epoch_ms,
                    operations: vec![draft.clone()],
                    review: coding_write_review(false, prepared.operation().plan_sha256()),
                    permitted_verification: coding_write_verification(),
                },
            )
            .map_err(|_| RuntimePortFailure::Invalid)?;
            let preview =
                render_filesystem_preview(&plan).map_err(|_| RuntimePortFailure::Invalid)?;
            Ok(PendingCodingAuthority::FilesystemWrite {
                approval_id,
                proposed_grant_id,
                parent_grant_id: parent.grant_id.clone(),
                preview: Box::new(preview),
                plan: Box::new(plan),
            })
        }
        None => {
            let approval = render_coding_approval_request(
                registry,
                CodingApprovalRequest {
                    parent,
                    approval_id,
                    proposed_grant_id,
                    call,
                    operation: prepared.operation().operation(),
                    targets: operation_targets(prepared.binding())?,
                    operation_plan_sha256: prepared.operation().plan_sha256(),
                    issued_at_epoch_ms: now_epoch_ms,
                    expires_at_epoch_ms,
                },
            )
            .map_err(|_| RuntimePortFailure::Invalid)?;
            Ok(PendingCodingAuthority::Generic(Box::new(approval)))
        }
    }
}

struct PendingCodingOperation<'workspace> {
    operation_id: RuntimeOperationId,
    operation: GrantOperation,
    tool_call: ToolCall,
    expires_at_epoch_ms: u64,
    authority: PendingCodingAuthority,
    prepared: PreparedLinuxCodingOperation<'workspace>,
}

enum IssuedCodingAuthority {
    Generic {
        approval: Box<ApprovalRequest>,
        approved: Box<ApprovedCodingGrant>,
    },
    StructuredWrite {
        approval_id: ApprovalId,
        preview_sha256: String,
        decision_sha256: String,
        approval: Box<WriteApprovalReceipt>,
        change_set: Box<ShadowChangeSet>,
    },
    FilesystemWrite {
        approval_id: ApprovalId,
        preview_sha256: String,
        decision_sha256: String,
        approval: Box<FilesystemApprovalReceipt>,
        plan: Box<FilesystemPlan>,
    },
}

struct IssuedCodingOperation<'workspace> {
    tool_call: ToolCall,
    expires_at_epoch_ms: u64,
    authority: IssuedCodingAuthority,
    prepared: PreparedLinuxCodingOperation<'workspace>,
    resolved_at_epoch_ms: u64,
}

struct RuntimeEffectEventContext<'builder> {
    started_event: RuntimeEvent,
    build_terminal_event:
        &'builder mut dyn FnMut(&RuntimeToolExecution) -> Result<RuntimeEvent, RuntimePortFailure>,
}

type PermissionEventBuilder<'a> =
    dyn FnMut(&RuntimePermissionEvaluation) -> Result<RuntimeEvent, RuntimePortFailure> + 'a;

impl IssuedCodingOperation<'_> {
    fn approval_id(&self) -> &ApprovalId {
        match &self.authority {
            IssuedCodingAuthority::Generic { approval, .. } => &approval.approval_id,
            IssuedCodingAuthority::StructuredWrite { approval_id, .. }
            | IssuedCodingAuthority::FilesystemWrite { approval_id, .. } => approval_id,
        }
    }

    fn preview_sha256(&self) -> &str {
        match &self.authority {
            IssuedCodingAuthority::Generic { approval, .. } => &approval.confirmation_sha256,
            IssuedCodingAuthority::StructuredWrite { preview_sha256, .. }
            | IssuedCodingAuthority::FilesystemWrite { preview_sha256, .. } => preview_sha256,
        }
    }

    fn grant_id(&self) -> &GrantId {
        match &self.authority {
            IssuedCodingAuthority::Generic { approved, .. } => &approved.grant.grant_id,
            IssuedCodingAuthority::StructuredWrite { approval, .. } => &approval.grant.grant_id,
            IssuedCodingAuthority::FilesystemWrite { approval, .. } => &approval.grant.grant_id,
        }
    }

    fn decision_sha256(&self) -> &str {
        match &self.authority {
            IssuedCodingAuthority::Generic { approved, .. } => &approved.decision_sha256,
            IssuedCodingAuthority::StructuredWrite {
                decision_sha256, ..
            }
            | IssuedCodingAuthority::FilesystemWrite {
                decision_sha256, ..
            } => decision_sha256,
        }
    }

    fn authority_sha256(&self) -> &str {
        match &self.authority {
            IssuedCodingAuthority::Generic { approved, .. } => &approved.authority_sha256,
            IssuedCodingAuthority::StructuredWrite { approval, .. } => &approval.binding_sha256,
            IssuedCodingAuthority::FilesystemWrite { approval, .. } => &approval.binding_sha256,
        }
    }

    fn generic_authority(
        &self,
    ) -> Result<(&ApprovalRequest, &ApprovedCodingGrant), RuntimePortFailure> {
        match &self.authority {
            IssuedCodingAuthority::Generic { approval, approved } => Ok((approval, approved)),
            _ => Err(RuntimePortFailure::Invalid),
        }
    }
}

/// Explicit verified dependencies required to construct one Linux coding boundary.
pub struct LinuxCodingRuntimeBoundaryInput<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    /// Descriptor-bound owned worktree and immutable coding profile.
    pub workspace: &'workspace LinuxCodingWorkspace<'session, 'platform>,
    /// Durable authority store under its continuously held private root.
    pub authority: LinuxAuthorityRuntime,
    /// Verified offline Linux worker sandbox.
    pub sandbox: LinuxSandboxRunner,
    /// Verified bounded command executor for the profile's exact command inventory.
    pub command_executor: E,
    /// Verified offline Git inspection executor for the held owned worktree.
    pub git_executor: G,
    /// Run-stable deny-by-default policy bound into the runtime request.
    pub policy: CodingRuntimePolicy,
    /// Exact authenticated local actor.
    pub actor_id: ActorId,
    /// Exact owning local session.
    pub session_id: SessionId,
    /// Data handling label inherited by session authority.
    pub sensitivity: DataSensitivity,
    /// CSPRNG or deterministic test identity source.
    pub identities: I,
}

/// Production Linux implementation of the reusable runtime's native coding boundary.
pub struct LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    workspace: &'workspace LinuxCodingWorkspace<'session, 'platform>,
    authority: LinuxAuthorityRuntime,
    sandbox: Option<LinuxSandboxRunner>,
    command_executor: Option<E>,
    git_executor: Option<G>,
    policy: CodingRuntimePolicy,
    actor_id: ActorId,
    session_id: SessionId,
    sensitivity: DataSensitivity,
    identities: I,
    pending: BTreeMap<String, PendingCodingOperation<'workspace>>,
    issued: BTreeMap<String, IssuedCodingOperation<'workspace>>,
}

impl<'workspace, 'session, 'platform, I, E, G>
    LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    /// Composes already verified workspace, authority, sandbox, and policy objects.
    pub fn new(
        input: LinuxCodingRuntimeBoundaryInput<'workspace, 'session, 'platform, I, E, G>,
    ) -> Result<Self, LinuxCodingRuntimeError> {
        let LinuxCodingRuntimeBoundaryInput {
            workspace,
            authority,
            sandbox,
            command_executor,
            git_executor,
            policy,
            actor_id,
            session_id,
            sensitivity,
            identities,
        } = input;
        let root = workspace.workspace();
        if policy.parent_targets().len() != 1
            || policy.parent_targets().iter().any(|target| {
                target.workspace_id() != root.workspace_id()
                    || target.authorization_id() != root.authorization_id()
                    || target.adapter_instance_id() != root.adapter_instance_id()
                    || target.platform() != root.platform()
                    || !target.path_components().is_empty()
                    || target.is_operation_target()
            })
            || policy.excluded_targets().iter().any(|target| {
                target.workspace_id() != root.workspace_id()
                    || target.authorization_id() != root.authorization_id()
                    || target.adapter_instance_id() != root.adapter_instance_id()
                    || target.platform() != root.platform()
                    || target.is_operation_target()
            })
        {
            return Err(LinuxCodingRuntimeError::BindingDenied);
        }
        root.revalidate()
            .map_err(|_| LinuxCodingRuntimeError::BindingDenied)?;
        authority
            .revalidate_root()
            .map_err(|_| LinuxCodingRuntimeError::BindingDenied)?;
        Ok(Self {
            workspace,
            authority,
            sandbox: Some(sandbox),
            command_executor: Some(command_executor),
            git_executor: Some(git_executor),
            policy,
            actor_id,
            session_id,
            sensitivity,
            identities,
            pending: BTreeMap::new(),
            issued: BTreeMap::new(),
        })
    }

    fn evaluate_call(
        &mut self,
        request: &RuntimeRunRequest,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
        mut build_event: Option<&mut PermissionEventBuilder<'_>>,
    ) -> Result<(RuntimePermissionEvaluation, Option<RuntimeEvent>), RuntimePortFailure> {
        if self.pending.len() >= MAX_PENDING_OPERATIONS
            || !self.request_matches(request)
            || !self.definition_matches(definition, call)
            || !self.policy.admits_action(&call.action_id)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.workspace
            .workspace()
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let workspace: &'workspace LinuxCodingWorkspace<'session, 'platform> = self.workspace;
        let prepared = workspace
            .prepare(call)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let expires_at_epoch_ms = now_epoch_ms
            .checked_add(PREVIEW_LIFETIME_MS)
            .ok_or(RuntimePortFailure::Invalid)?;
        let parent_preview_sha256 = parent_preview_sha256(
            request,
            operation_id,
            self.workspace.profile().profile_sha256(),
            prepared.operation().plan_sha256(),
            self.policy.parent_targets(),
            self.policy.excluded_targets(),
        )?;
        let parent_grant_id = GrantId::from_raw(self.next_id("grant-parent")?);
        let parent_nonce = GrantNonce::from_raw(self.next_id("nonce-parent")?);
        let approval_id = ApprovalId::from_raw(self.next_id("approval")?);
        let proposed_grant_id = GrantId::from_raw(self.next_id("grant-operation")?);
        let plan_id = self.next_id("change-plan")?;
        let grant_request = SessionReadGrantRequest {
            grant_id: parent_grant_id,
            actor_id: self.actor_id.clone(),
            session_id: self.session_id.clone(),
            task_id: request.task.task_id.clone(),
            targets: self.policy.parent_targets().to_vec(),
            excluded_targets: self.policy.excluded_targets().to_vec(),
            sensitivity: self.sensitivity,
            issued_at_epoch_ms: now_epoch_ms,
            expires_at_epoch_ms,
            nonce: parent_nonce,
            maximum_derived_operations: 1,
            preview_sha256: parent_preview_sha256,
            policy_sha256: self.policy.engine().policy_sha256().to_owned(),
        };
        let registry = self.workspace.profile().registry();
        let build_authority = |parent: &agentmage_kernel_contracts::CapabilityGrant| {
            let authority = build_pending_coding_authority(
                parent,
                registry,
                &prepared,
                call,
                operation_id,
                approval_id.clone(),
                proposed_grant_id.clone(),
                plan_id.clone(),
                now_epoch_ms,
                expires_at_epoch_ms,
            )?;
            let evaluation = RuntimePermissionEvaluation::Ask {
                approval_id: authority.approval_id().clone(),
                grant_id: authority.proposed_grant_id().clone(),
                preview_sha256: authority.preview_sha256().to_owned(),
                expires_at_epoch_ms,
            };
            Ok::<_, RuntimePortFailure>((authority, evaluation))
        };
        let (authority, evaluation, committed_event) = if let Some(builder) = build_event.as_mut() {
            let (_, (authority, evaluation), event) = self
                .authority
                .authority_mut()
                .issue_session_read_with_runtime_event(grant_request, |parent| {
                    let (authority, evaluation) =
                        build_authority(parent).map_err(|_| RuntimeJournalError::InvalidEvent)?;
                    let event =
                        builder(&evaluation).map_err(|_| RuntimeJournalError::InvalidEvent)?;
                    Ok(((authority, evaluation), event))
                })
                .map_err(map_journal_failure)?;
            (authority, evaluation, Some(event))
        } else {
            let parent = self
                .authority
                .authority_mut()
                .issue_session_read(grant_request)
                .map_err(|_| RuntimePortFailure::Invalid)?;
            let (authority, evaluation) = build_authority(&parent)?;
            (authority, evaluation, None)
        };
        let key = authority.approval_id().as_str().to_owned();
        if self
            .pending
            .insert(
                key,
                PendingCodingOperation {
                    operation_id: operation_id.clone(),
                    operation: prepared.operation().operation().operation(),
                    tool_call: call.clone(),
                    expires_at_epoch_ms,
                    authority,
                    prepared,
                },
            )
            .is_some()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        Ok((evaluation, committed_event))
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_call(
        &mut self,
        request: &RuntimeRunRequest,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
        mut build_event: Option<&mut PermissionEventBuilder<'_>>,
    ) -> Result<(RuntimePermissionEvaluation, Option<RuntimeEvent>), RuntimePortFailure> {
        if !self.request_matches(request) || !self.definition_matches(definition, call) {
            return Err(RuntimePortFailure::Invalid);
        }
        let key = challenge.approval_id.as_str();
        let pending = self.pending.get(key).ok_or(RuntimePortFailure::Invalid)?;
        if !challenge_matches(request, challenge, pending, call)
            || verify_runtime_approval_response(challenge, response, now_epoch_ms).is_err()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let decision_sha256 = canonical_sha256(response)?;
        if response.disposition == RuntimeApprovalDisposition::Deny {
            let pending = self
                .pending
                .remove(key)
                .ok_or(RuntimePortFailure::Invalid)?;
            let evaluation = RuntimePermissionEvaluation::Deny {
                approval_id: pending.authority.approval_id().clone(),
                grant_id: pending.authority.proposed_grant_id().clone(),
                preview_sha256: pending.authority.preview_sha256().to_owned(),
                expires_at_epoch_ms: pending.expires_at_epoch_ms,
                decision_sha256,
                reason_code: "runtime.coding.user-denied".to_owned(),
            };
            let event = if let Some(builder) = build_event.as_mut() {
                let event = builder(&evaluation)?;
                self.authority
                    .authority_mut()
                    .record_runtime_event_with_authority_snapshot(event.clone())
                    .map_err(map_journal_failure)?;
                Some(event)
            } else {
                None
            };
            return Ok((evaluation, event));
        }

        pending
            .prepared
            .revalidate()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let pending = self
            .pending
            .remove(key)
            .ok_or(RuntimePortFailure::Invalid)?;
        let operation_nonce = GrantNonce::from_raw(self.next_id("nonce-operation")?);
        let (issued_authority, committed_event) = match pending.authority {
            PendingCodingAuthority::Generic(approval) => {
                let grant_request = ApprovedCodingGrantRequest {
                    authority: self.authority.authority_mut(),
                    registry: self.workspace.profile().registry(),
                    policy: self.policy.engine(),
                    approval: &approval,
                    challenge,
                    response,
                    nonce: operation_nonce,
                    now_epoch_ms,
                };
                let (approved, event) = if let Some(builder) = build_event.as_mut() {
                    let (approved, event) =
                        derive_approved_coding_grant_with_event(grant_request, *builder)
                            .map_err(|_| RuntimePortFailure::Invalid)?;
                    (approved, Some(event))
                } else {
                    (
                        derive_approved_coding_grant(grant_request)
                            .map_err(|_| RuntimePortFailure::Invalid)?,
                        None,
                    )
                };
                (
                    IssuedCodingAuthority::Generic {
                        approval,
                        approved: Box::new(approved),
                    },
                    event,
                )
            }
            PendingCodingAuthority::StructuredWrite {
                approval_id,
                proposed_grant_id,
                parent_grant_id,
                preview,
                change_set,
            } => {
                let decision = WriteApprovalDecision {
                    approval_id: approval_id.clone(),
                    approved_change_set_sha256: change_set.change_set_sha256().to_owned(),
                    approved_preview_sha256: preview.preview_sha256.clone(),
                    approved_at_epoch_ms: now_epoch_ms,
                    expires_at_epoch_ms: pending.expires_at_epoch_ms,
                    permitted_verification: change_set.permitted_verification().to_vec(),
                    user_confirmed: true,
                };
                let request = WriteGrantRequest {
                    parent_grant_id,
                    grant_id: proposed_grant_id,
                    action_id: call.action_id.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    tool_id: call.tool_id.clone(),
                    tool_version: call.tool_version.clone(),
                    nonce: operation_nonce,
                    policy_sha256: self.policy.engine().policy_sha256().to_owned(),
                };
                let (approval, event) = if let Some(builder) = build_event.as_mut() {
                    let (approval, _, event) = self
                        .authority
                        .authority_mut()
                        .issue_write_approval_with_runtime_event(
                            &change_set,
                            &preview,
                            &decision,
                            request,
                            |approval| {
                                let evaluation = RuntimePermissionEvaluation::Allow {
                                    approval_id: approval_id.clone(),
                                    preview_sha256: preview.preview_sha256.clone(),
                                    expires_at_epoch_ms: pending.expires_at_epoch_ms,
                                    grant_id: approval.grant.grant_id.clone(),
                                    decision_sha256: decision_sha256.clone(),
                                    authority_sha256: approval.binding_sha256.clone(),
                                };
                                let event = builder(&evaluation)
                                    .map_err(|_| RuntimeJournalError::InvalidEvent)?;
                                Ok(((), event))
                            },
                        )
                        .map_err(map_journal_failure)?;
                    (approval, Some(event))
                } else {
                    (
                        self.authority
                            .authority_mut()
                            .issue_write_approval(&change_set, &preview, &decision, request)
                            .map_err(|_| RuntimePortFailure::Unavailable)?,
                        None,
                    )
                };
                (
                    IssuedCodingAuthority::StructuredWrite {
                        approval_id,
                        preview_sha256: preview.preview_sha256,
                        decision_sha256: decision_sha256.clone(),
                        approval: Box::new(approval),
                        change_set,
                    },
                    event,
                )
            }
            PendingCodingAuthority::FilesystemWrite {
                approval_id,
                proposed_grant_id,
                parent_grant_id,
                preview,
                plan,
            } => {
                let decision = FilesystemApprovalDecision {
                    approval_id: approval_id.clone(),
                    approved_plan_sha256: plan.plan_sha256().to_owned(),
                    approved_preview_sha256: preview.preview_sha256.clone(),
                    approved_at_epoch_ms: now_epoch_ms,
                    expires_at_epoch_ms: pending.expires_at_epoch_ms,
                    permitted_verification: plan.permitted_verification().to_vec(),
                    user_confirmed: true,
                    high_risk_delete_confirmed: false,
                };
                let request = FilesystemGrantRequest {
                    parent_grant_id,
                    grant_id: proposed_grant_id,
                    action_id: call.action_id.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    tool_id: call.tool_id.clone(),
                    tool_version: call.tool_version.clone(),
                    nonce: operation_nonce,
                    policy_sha256: self.policy.engine().policy_sha256().to_owned(),
                };
                let (approval, event) = if let Some(builder) = build_event.as_mut() {
                    let (approval, _, event) = self
                        .authority
                        .authority_mut()
                        .issue_filesystem_approval_with_runtime_event(
                            &plan,
                            &preview,
                            &decision,
                            request,
                            |approval| {
                                let evaluation = RuntimePermissionEvaluation::Allow {
                                    approval_id: approval_id.clone(),
                                    preview_sha256: preview.preview_sha256.clone(),
                                    expires_at_epoch_ms: pending.expires_at_epoch_ms,
                                    grant_id: approval.grant.grant_id.clone(),
                                    decision_sha256: decision_sha256.clone(),
                                    authority_sha256: approval.binding_sha256.clone(),
                                };
                                let event = builder(&evaluation)
                                    .map_err(|_| RuntimeJournalError::InvalidEvent)?;
                                Ok(((), event))
                            },
                        )
                        .map_err(map_journal_failure)?;
                    (approval, Some(event))
                } else {
                    (
                        self.authority
                            .authority_mut()
                            .issue_filesystem_approval(&plan, &preview, &decision, request)
                            .map_err(|_| RuntimePortFailure::Unavailable)?,
                        None,
                    )
                };
                (
                    IssuedCodingAuthority::FilesystemWrite {
                        approval_id,
                        preview_sha256: preview.preview_sha256,
                        decision_sha256: decision_sha256.clone(),
                        approval: Box::new(approval),
                        plan,
                    },
                    event,
                )
            }
        };
        let issued = IssuedCodingOperation {
            tool_call: pending.tool_call,
            expires_at_epoch_ms: pending.expires_at_epoch_ms,
            authority: issued_authority,
            prepared: pending.prepared,
            resolved_at_epoch_ms: now_epoch_ms,
        };
        let evaluation = RuntimePermissionEvaluation::Allow {
            approval_id: issued.approval_id().clone(),
            preview_sha256: issued.preview_sha256().to_owned(),
            expires_at_epoch_ms: issued.expires_at_epoch_ms,
            grant_id: issued.grant_id().clone(),
            decision_sha256: issued.decision_sha256().to_owned(),
            authority_sha256: issued.authority_sha256().to_owned(),
        };
        let grant_key = issued.grant_id().as_str().to_owned();
        if self.issued.insert(grant_key, issued).is_some() {
            return Err(RuntimePortFailure::Invalid);
        }
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        Ok((evaluation, committed_event))
    }

    fn execute_call(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
        event_context: Option<RuntimeEffectEventContext<'_>>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        let RuntimePermissionEvaluation::Allow {
            approval_id,
            preview_sha256,
            expires_at_epoch_ms,
            grant_id,
            decision_sha256,
            authority_sha256,
        } = evaluation
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        if !self.request_matches(request) || !self.definition_matches(definition, call) {
            return Err(RuntimePortFailure::Invalid);
        }
        let key = grant_id.as_str();
        let issued = self.issued.get(key).ok_or(RuntimePortFailure::Invalid)?;
        if issued.approval_id() != approval_id
            || issued.preview_sha256() != preview_sha256
            || issued.expires_at_epoch_ms != *expires_at_epoch_ms
            || issued.tool_call != *call
            || issued.grant_id() != grant_id
            || issued.decision_sha256() != decision_sha256
            || issued.authority_sha256() != authority_sha256
        {
            return Err(RuntimePortFailure::Invalid);
        }
        if cancellation
            .map(|probe| probe.observe())
            .transpose()
            .map_err(|_| RuntimePortFailure::Unavailable)?
            .flatten()
            .is_some()
        {
            return Err(RuntimePortFailure::Cancelled);
        }
        if matches!(&issued.authority, IssuedCodingAuthority::Generic { .. }) {
            issued
                .prepared
                .revalidate()
                .map_err(|_| RuntimePortFailure::Invalid)?;
        } else {
            self.workspace
                .workspace()
                .revalidate()
                .map_err(|_| RuntimePortFailure::Invalid)?;
        }
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        let issued = self.issued.remove(key).ok_or(RuntimePortFailure::Invalid)?;
        match issued.prepared.operation().prepared() {
            PreparedNativeCodingCall::ReadOnly { .. } => {
                self.execute_prepared_read(request, definition, call, issued, event_context)
            }
            PreparedNativeCodingCall::GitInspection { .. } => {
                self.execute_prepared_git(request, definition, call, issued, event_context)
            }
            PreparedNativeCodingCall::Command { .. } => {
                self.execute_prepared_command(request, definition, call, issued, event_context)
            }
            PreparedNativeCodingCall::Validation { .. } => {
                self.execute_prepared_validation(request, definition, call, issued, event_context)
            }
            PreparedNativeCodingCall::StructuredPatch { .. } => self
                .execute_prepared_structured_write(
                    request,
                    definition,
                    call,
                    issued,
                    event_context,
                ),
            PreparedNativeCodingCall::ControlledCreate { .. } => self
                .execute_prepared_controlled_create(
                    request,
                    definition,
                    call,
                    issued,
                    event_context,
                ),
        }
    }

    fn execute_prepared_read(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        issued: IssuedCodingOperation<'workspace>,
        event_context: Option<RuntimeEffectEventContext<'_>>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        let policy = issued.generic_authority()?.1.policy.clone();
        let transaction = self.authority_transaction(call, &issued)?;
        let (operation, binding, write_draft, _) = issued.prepared.into_parts();
        let kind = match operation.prepared() {
            PreparedNativeCodingCall::ReadOnly { kind, .. } => *kind,
            _ => return Err(RuntimePortFailure::Unavailable),
        };
        let LinuxCodingTargetBinding::ReadProjection { held, .. } = binding else {
            return Err(RuntimePortFailure::Invalid);
        };
        if operation.expected_state_change() != StateChange::NotChanged || write_draft.is_some() {
            return Err(RuntimePortFailure::Invalid);
        }
        let worker_input = LinuxReadOnlyToolInput::seal(
            call.tool_id.as_str(),
            call.tool_version.as_str(),
            &call.arguments.bytes,
            held,
        )
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let runner = self.sandbox.take().ok_or(RuntimePortFailure::Unavailable)?;
        let mut driver = LinuxReadOnlyToolEffectDriver::new(runner, worker_input);
        let receipt_result = self.execute_effect_authority(
            self.workspace.profile().registry(),
            &policy,
            transaction,
            &mut driver,
            event_context.as_ref(),
        );
        let worker_result = driver.take_result();
        self.sandbox = Some(driver.into_runner());
        let (receipt, pending) = receipt_result?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let worker_result = worker_result.ok_or(RuntimePortFailure::Uncertain)?;
        if !worker_result.success() {
            let execution = RuntimeToolExecution {
                receipt_id: receipt.receipt_id,
                receipt_sha256: receipt.receipt_sha256,
                result: ToolResult {
                    schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                    tool_call_id: call.tool_call_id.clone(),
                    correlation_id: call.correlation_id.clone(),
                    outcome: OperationOutcome::Failed,
                    output: None,
                    validation_issues: Vec::new(),
                    evidence: Vec::new(),
                    error: None,
                    elapsed_ms: 0,
                    state_change: StateChange::NotChanged,
                },
                result_output_kind: None,
                artifact_candidates: Vec::new(),
            };
            return self.finish_effect_execution(execution, event_context, pending);
        }
        let result = serde_json::from_slice::<ReadOnlyResult>(worker_result.stdout())
            .ok()
            .filter(|result| result.verify(kind))
            .ok_or(RuntimePortFailure::Uncertain)?;
        let output_bytes = serde_json::to_vec(&result).map_err(|_| RuntimePortFailure::Invalid)?;
        if output_bytes.len() as u64 > request.limits.max_output_bytes {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let output_sha256 = sha256(&output_bytes);
        let outcome = read_outcome(result.outcome);
        let evidence = if matches!(
            result.outcome,
            ReadOnlyOutcome::Succeeded
                | ReadOnlyOutcome::NoResult
                | ReadOnlyOutcome::Partial
                | ReadOnlyOutcome::Truncated
        ) {
            vec![EvidenceReference {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw(self.next_id("evidence")?),
                kind: EvidenceKind::ToolOutput,
                source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
                object_id: call.tool_call_id.as_str().to_owned(),
                fragment: None,
                content_sha256: result.result_sha256.clone(),
                observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
            }]
        } else {
            Vec::new()
        };
        let execution = RuntimeToolExecution {
            receipt_id: receipt.receipt_id,
            receipt_sha256: receipt.receipt_sha256,
            result: ToolResult {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome,
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".to_owned(),
                    bytes: output_bytes,
                    sha256: output_sha256,
                }),
                validation_issues: Vec::new(),
                evidence,
                error: None,
                elapsed_ms: 0,
                state_change: StateChange::NotChanged,
            },
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates: Vec::new(),
        };
        self.finish_effect_execution(execution, event_context, pending)
    }

    fn execute_prepared_command(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        issued: IssuedCodingOperation<'workspace>,
        event_context: Option<RuntimeEffectEventContext<'_>>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        let policy = issued.generic_authority()?.1.policy.clone();
        let transaction = self.authority_transaction(call, &issued)?;
        let (operation, binding, write_draft, workspace) = issued.prepared.into_parts();
        let prepared = match operation.prepared() {
            PreparedNativeCodingCall::Command { prepared } => prepared.as_ref().clone(),
            _ => return Err(RuntimePortFailure::Invalid),
        };
        if !matches!(binding, LinuxCodingTargetBinding::OwnedWorktreeRoot { .. })
            || operation.expected_state_change() != StateChange::NotChanged
            || write_draft.is_some()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let cancellation = CancellationToken::root(
            agentmage_kernel_contracts::BoundaryKind::Tool,
            request.task.task_id.clone(),
            call.correlation_id.clone(),
        );
        let executor = self
            .command_executor
            .take()
            .ok_or(RuntimePortFailure::Unavailable)?;
        let mut driver =
            CommandEffectDriver::new(executor, workspace, prepared.clone(), cancellation);
        let receipt_result = self.execute_effect_authority(
            self.workspace.profile().registry(),
            &policy,
            transaction,
            &mut driver,
            event_context.as_ref(),
        );
        let command_receipt = driver.take_receipt();
        let output = driver.take_output();
        self.command_executor = Some(driver.into_executor());
        let (receipt, pending) = receipt_result?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let command_receipt = command_receipt.ok_or(RuntimePortFailure::Uncertain)?;
        let output = output.ok_or(RuntimePortFailure::Uncertain)?;
        if !verify_command_receipt(&prepared, &command_receipt) {
            return Err(RuntimePortFailure::Uncertain);
        }
        let execution =
            self.command_execution(request, definition, call, receipt, command_receipt, output)?;
        self.finish_effect_execution(execution, event_context, pending)
    }

    fn execute_prepared_git(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        issued: IssuedCodingOperation<'workspace>,
        event_context: Option<RuntimeEffectEventContext<'_>>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        let policy = issued.generic_authority()?.1.policy.clone();
        let transaction = self.authority_transaction(call, &issued)?;
        let operation_plan_sha256 = issued.prepared.operation().plan_sha256().to_owned();
        let (operation, binding, write_draft, workspace) = issued.prepared.into_parts();
        let (git_request, capability_plan) = match operation.prepared() {
            PreparedNativeCodingCall::GitInspection { request, plan } => {
                (request.clone(), plan.clone())
            }
            _ => return Err(RuntimePortFailure::Invalid),
        };
        if !matches!(binding, LinuxCodingTargetBinding::OwnedWorktreeRoot { .. })
            || operation.expected_state_change() != StateChange::NotChanged
            || write_draft.is_some()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let prepared = prepare_kernel_git_inspection(
            &git_request,
            &capability_plan,
            &call.arguments.sha256,
            &operation_plan_sha256,
        )?;
        let cancellation = CancellationToken::root(
            agentmage_kernel_contracts::BoundaryKind::Tool,
            request.task.task_id.clone(),
            call.correlation_id.clone(),
        );
        let executor = self
            .git_executor
            .take()
            .ok_or(RuntimePortFailure::Unavailable)?;
        let mut driver =
            RepositoryInspectionEffectDriver::new(executor, workspace, prepared, cancellation);
        let receipt_result = self.execute_effect_authority(
            self.workspace.profile().registry(),
            &policy,
            transaction,
            &mut driver,
            event_context.as_ref(),
        );
        let platform = driver.take_result();
        self.git_executor = Some(driver.into_executor());
        let (receipt, pending) = receipt_result?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let platform = platform.ok_or(RuntimePortFailure::Uncertain)?;
        let outcome = repository_inspection_outcome(&platform);
        if !matches!(
            platform.termination,
            RepositoryInspectionTermination::Exited
        ) {
            let execution = RuntimeToolExecution {
                receipt_id: receipt.receipt_id,
                receipt_sha256: receipt.receipt_sha256,
                result: ToolResult {
                    schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                    tool_call_id: call.tool_call_id.clone(),
                    correlation_id: call.correlation_id.clone(),
                    outcome,
                    output: None,
                    validation_issues: Vec::new(),
                    evidence: Vec::new(),
                    error: None,
                    elapsed_ms: platform.elapsed_ms,
                    state_change: if platform.descendants_terminated {
                        StateChange::NotChanged
                    } else {
                        StateChange::Uncertain
                    },
                },
                result_output_kind: None,
                artifact_candidates: Vec::new(),
            };
            return self.finish_effect_execution(execution, event_context, pending);
        }
        let git_result = parse_git_inspection(
            &git_request,
            self.workspace.profile().repository_snapshot_sha256(),
            &self.workspace.profile().worktree().record_sha256,
            &operation_plan_sha256,
            outcome == OperationOutcome::Succeeded,
            &platform.stdout,
        )
        .map_err(|_| RuntimePortFailure::Uncertain)?;
        if !git_result.verify() {
            return Err(RuntimePortFailure::Uncertain);
        }
        let execution = self.git_execution(
            request,
            definition,
            call,
            receipt,
            git_result,
            platform.elapsed_ms,
        )?;
        self.finish_effect_execution(execution, event_context, pending)
    }

    fn git_execution(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        receipt: agentmage_kernel_contracts::Receipt,
        git_result: GitInspectionResult,
        elapsed_ms: u64,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let output_bytes =
            serde_json::to_vec(&git_result).map_err(|_| RuntimePortFailure::Invalid)?;
        if output_bytes.len() as u64 > request.limits.max_output_bytes {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let evidence = vec![EvidenceReference {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(self.next_id("evidence")?),
            kind: EvidenceKind::Observation,
            source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
            object_id: call.tool_call_id.as_str().to_owned(),
            fragment: None,
            content_sha256: git_result.result_sha256.clone(),
            observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
        }];
        Ok(RuntimeToolExecution {
            receipt_id: receipt.receipt_id,
            receipt_sha256: receipt.receipt_sha256,
            result: ToolResult {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome: git_outcome(git_result.outcome),
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".to_owned(),
                    sha256: sha256(&output_bytes),
                    bytes: output_bytes,
                }),
                validation_issues: Vec::new(),
                evidence,
                error: None,
                elapsed_ms,
                state_change: StateChange::NotChanged,
            },
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates: Vec::new(),
        })
    }

    fn execute_prepared_validation(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        issued: IssuedCodingOperation<'workspace>,
        event_context: Option<RuntimeEffectEventContext<'_>>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        let (generic_approval, approved) = issued.generic_authority()?;
        let approval_sha256 = generic_approval.confirmation_sha256.clone();
        let policy = approved.policy.clone();
        let transaction = self.authority_transaction(call, &issued)?;
        let operation_plan_sha256 = issued.prepared.operation().plan_sha256().to_owned();
        let (operation, binding, write_draft, workspace) = issued.prepared.into_parts();
        let (validation_request, template, prepared) = match operation.prepared() {
            PreparedNativeCodingCall::Validation {
                request,
                template,
                prepared,
            } => (
                request.clone(),
                template.as_ref().clone(),
                prepared.as_ref().clone(),
            ),
            _ => return Err(RuntimePortFailure::Invalid),
        };
        if !matches!(binding, LinuxCodingTargetBinding::OwnedWorktreeRoot { .. })
            || operation.expected_state_change() != StateChange::NotChanged
            || write_draft.is_some()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let cancellation = CancellationToken::root(
            agentmage_kernel_contracts::BoundaryKind::Tool,
            request.task.task_id.clone(),
            call.correlation_id.clone(),
        );
        let executor = self
            .command_executor
            .take()
            .ok_or(RuntimePortFailure::Unavailable)?;
        let wrapper = RegisteredCommandWrapperBinding::new(call, operation_plan_sha256)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let mut driver = CommandEffectDriver::new_registered_wrapper(
            executor,
            workspace,
            prepared.clone(),
            cancellation,
            wrapper,
        );
        let receipt_result = self.execute_effect_authority(
            self.workspace.profile().registry(),
            &policy,
            transaction,
            &mut driver,
            event_context.as_ref(),
        );
        let command_receipt = driver.take_receipt();
        let output = driver.take_output();
        self.command_executor = Some(driver.into_executor());
        let (receipt, pending) = receipt_result?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let command_receipt = command_receipt.ok_or(RuntimePortFailure::Uncertain)?;
        let output = output.ok_or(RuntimePortFailure::Uncertain)?;
        if !verify_command_receipt(&prepared, &command_receipt)
            || !captured_output_matches(&output, &command_receipt)
        {
            return Err(RuntimePortFailure::Uncertain);
        }
        let artifact_candidates = captured_output_artifact_candidates(
            &output,
            RuntimeArtifactKind::TestLog,
            RuntimeArtifactKind::TestLog,
        );
        let validation_receipt = normalize_validation_result(
            &template,
            &prepared,
            &command_receipt,
            ValidationObservation {
                validation_attempt_id: validation_request.validation_attempt_id,
                approval_sha256,
                stdout: output.stdout().to_vec(),
                stderr: output.stderr().to_vec(),
                stdout_classification: ValidationOutputClassification::Restricted,
                stderr_classification: ValidationOutputClassification::Restricted,
                secret_match_count: 0,
                artifacts: Vec::new(),
                affected_files: Vec::new(),
                changed_paths: Vec::new(),
                baseline_failure_sha256s: Vec::new(),
                baseline_known_clean: false,
                unverified_kinds: Vec::new(),
            },
        )
        .map_err(|_| RuntimePortFailure::Uncertain)?;
        if !verify_validation_receipt(&template, &prepared, &command_receipt, &validation_receipt) {
            return Err(RuntimePortFailure::Uncertain);
        }
        let execution = self.validation_execution(
            request,
            definition,
            call,
            receipt,
            validation_receipt,
            artifact_candidates,
        )?;
        self.finish_effect_execution(execution, event_context, pending)
    }

    fn validation_execution(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        receipt: agentmage_kernel_contracts::Receipt,
        validation_receipt: ValidationReceipt,
        artifact_candidates: Vec<RuntimeToolArtifactCandidate>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let output_bytes =
            serde_json::to_vec(&validation_receipt).map_err(|_| RuntimePortFailure::Invalid)?;
        if output_bytes.len() as u64 > request.limits.max_output_bytes {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let evidence = vec![EvidenceReference {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(self.next_id("evidence")?),
            kind: EvidenceKind::Validation,
            source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
            object_id: validation_receipt.validation_attempt_id.clone(),
            fragment: None,
            content_sha256: validation_receipt.receipt_sha256.clone(),
            observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
        }];
        let outcome = validation_outcome(validation_receipt.status);
        Ok(RuntimeToolExecution {
            receipt_id: receipt.receipt_id,
            receipt_sha256: receipt.receipt_sha256,
            result: ToolResult {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome,
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".to_owned(),
                    sha256: sha256(&output_bytes),
                    bytes: output_bytes,
                }),
                validation_issues: Vec::new(),
                evidence,
                error: None,
                elapsed_ms: validation_receipt.duration_ms,
                state_change: StateChange::NotChanged,
            },
            result_output_kind: Some(RuntimeArtifactKind::TestLog),
            artifact_candidates,
        })
    }

    fn command_execution(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        receipt: agentmage_kernel_contracts::Receipt,
        command_receipt: CommandReceipt,
        output: CommandCapturedOutput,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        if !captured_output_matches(&output, &command_receipt) {
            return Err(RuntimePortFailure::Uncertain);
        }
        let output_bytes =
            serde_json::to_vec(&command_receipt).map_err(|_| RuntimePortFailure::Invalid)?;
        if output_bytes.len() as u64 > request.limits.max_output_bytes {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let artifact_candidates = captured_output_artifact_candidates(
            &output,
            RuntimeArtifactKind::StandardOutput,
            RuntimeArtifactKind::StandardError,
        );
        let evidence = vec![EvidenceReference {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(self.next_id("evidence")?),
            kind: EvidenceKind::ToolOutput,
            source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
            object_id: call.tool_call_id.as_str().to_owned(),
            fragment: None,
            content_sha256: command_receipt.receipt_sha256.clone(),
            observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
        }];
        Ok(RuntimeToolExecution {
            receipt_id: receipt.receipt_id,
            receipt_sha256: receipt.receipt_sha256,
            result: ToolResult {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome: command_receipt.outcome,
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".to_owned(),
                    sha256: sha256(&output_bytes),
                    bytes: output_bytes,
                }),
                validation_issues: Vec::new(),
                evidence,
                error: None,
                elapsed_ms: command_receipt.elapsed_ms,
                state_change: StateChange::NotChanged,
            },
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates,
        })
    }

    fn execute_prepared_structured_write(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        issued: IssuedCodingOperation<'workspace>,
        event_context: Option<RuntimeEffectEventContext<'_>>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        let IssuedCodingOperation {
            authority:
                IssuedCodingAuthority::StructuredWrite {
                    approval,
                    change_set,
                    ..
                },
            prepared,
            resolved_at_epoch_ms,
            ..
        } = issued
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        let (operation, binding, write_draft, workspace) = prepared.into_parts();
        let Some(LinuxCodingWriteDraft::StructuredPatch(plan)) = write_draft else {
            return Err(RuntimePortFailure::Invalid);
        };
        let LinuxCodingTargetBinding::ExistingFile { target, .. } = binding else {
            return Err(RuntimePortFailure::Invalid);
        };
        let Some(change) = change_set.operations().first() else {
            return Err(RuntimePortFailure::Invalid);
        };
        if change_set.operations().len() != 1
            || !matches!(
                operation.prepared(),
                PreparedNativeCodingCall::StructuredPatch { .. }
            )
            || operation.expected_state_change() != StateChange::Changed
            || change.target() != &target
            || change.preimage_bytes() != plan.preimage()
            || change.proposed_bytes() != plan.postimage()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let transaction_id = self.next_id("write-transaction")?;
        let mut driver =
            LinuxAtomicWriteDriver::new(workspace, LinuxAtomicWriteDriverLimits::default());
        let policy = self.policy.engine().clone();
        let write_request = WriteTransactionRequest {
            transaction_id,
            now_epoch_ms: resolved_at_epoch_ms,
        };
        let (result, pending) = if let Some(context) = event_context.as_ref() {
            let (result, pending) = self
                .authority
                .authority_mut()
                .begin_controlled_write_with_runtime_event(
                    &policy,
                    &change_set,
                    &approval,
                    write_request,
                    &mut driver,
                    context.started_event.clone(),
                )
                .map_err(map_journal_failure)?;
            (result, Some(pending))
        } else {
            (
                self.authority
                    .authority_mut()
                    .execute_controlled_write(
                        &policy,
                        &change_set,
                        &approval,
                        write_request,
                        &mut driver,
                    )
                    .map_err(|_| RuntimePortFailure::Uncertain)?,
                None,
            )
        };
        verify_write_receipts(&result.receipts).map_err(|_| RuntimePortFailure::Uncertain)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let receipt = result
            .receipts
            .last()
            .ok_or(RuntimePortFailure::Uncertain)?;
        let (outcome, state_change) = write_transaction_outcome(result.outcome);
        let execution = self.controlled_change_execution(
            request,
            definition,
            call,
            ControlledChangeOutput {
                schema_version: 1,
                operation_id: receipt.operation_id.clone(),
                outcome,
                path_sha256: sha256(change.path().as_bytes()),
                preimage_sha256: Some(receipt.preimage_sha256.clone()),
                postimage_sha256: receipt.postimage_sha256.clone(),
                receipt_id: specialized_receipt_id("write", &receipt.receipt_sha256),
                receipt_sha256: receipt.receipt_sha256.clone(),
            },
            state_change,
        )?;
        self.finish_specialized_effect_event(execution, event_context, pending)
    }

    fn execute_prepared_controlled_create(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        issued: IssuedCodingOperation<'workspace>,
        event_context: Option<RuntimeEffectEventContext<'_>>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        let IssuedCodingOperation {
            authority: IssuedCodingAuthority::FilesystemWrite { approval, plan, .. },
            prepared,
            resolved_at_epoch_ms,
            ..
        } = issued
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        let (operation, binding, write_draft, workspace) = prepared.into_parts();
        let Some(LinuxCodingWriteDraft::ControlledCreate(create_draft)) = write_draft else {
            return Err(RuntimePortFailure::Invalid);
        };
        let generated_bytes = match &create_draft {
            FilesystemOperationDraft::Create {
                content,
                classification: FileClassification::Generated,
                ..
            } => Some(content.clone()),
            FilesystemOperationDraft::Create { .. } => None,
            _ => return Err(RuntimePortFailure::Invalid),
        };
        if !matches!(binding, LinuxCodingTargetBinding::DestinationParent { .. })
            || !matches!(
                operation.prepared(),
                PreparedNativeCodingCall::ControlledCreate { .. }
            )
            || operation.expected_state_change() != StateChange::Changed
            || plan.operations().len() != 1
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let transaction_id = self.next_id("filesystem-transaction")?;
        let mut driver =
            LinuxControlledFilesystemDriver::new(workspace, LinuxFilesystemDriverLimits::default());
        let policy = self.policy.engine().clone();
        let filesystem_request = FilesystemTransactionRequest {
            transaction_id,
            now_epoch_ms: resolved_at_epoch_ms,
            cancelled_before_consume: false,
        };
        let (result, pending) = if let Some(context) = event_context.as_ref() {
            let (result, pending) = self
                .authority
                .authority_mut()
                .begin_controlled_filesystem_with_runtime_event(
                    &policy,
                    &plan,
                    &approval,
                    filesystem_request,
                    &mut driver,
                    context.started_event.clone(),
                )
                .map_err(map_journal_failure)?;
            (result, Some(pending))
        } else {
            (
                self.authority
                    .authority_mut()
                    .execute_controlled_filesystem(
                        &policy,
                        &plan,
                        &approval,
                        filesystem_request,
                        &mut driver,
                    )
                    .map_err(|_| RuntimePortFailure::Uncertain)?,
                None,
            )
        };
        verify_filesystem_receipts(&result.receipts).map_err(|_| RuntimePortFailure::Uncertain)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let receipt = result
            .receipts
            .last()
            .ok_or(RuntimePortFailure::Uncertain)?;
        let destination = receipt
            .destination_path
            .as_deref()
            .ok_or(RuntimePortFailure::Uncertain)?;
        let (outcome, state_change) = filesystem_transaction_outcome(result.outcome);
        let mut execution = self.controlled_change_execution(
            request,
            definition,
            call,
            ControlledChangeOutput {
                schema_version: 1,
                operation_id: receipt.operation_id.clone(),
                outcome,
                path_sha256: sha256(destination.as_bytes()),
                preimage_sha256: None,
                postimage_sha256: receipt.postimage_sha256.clone(),
                receipt_id: specialized_receipt_id("filesystem", &receipt.receipt_sha256),
                receipt_sha256: receipt.receipt_sha256.clone(),
            },
            state_change,
        )?;
        if state_change == StateChange::Changed
            && let Some(bytes) = generated_bytes
        {
            execution
                .artifact_candidates
                .push(RuntimeToolArtifactCandidate {
                    kind: RuntimeArtifactKind::GeneratedFile,
                    media_type: "text/plain".to_owned(),
                    bytes,
                });
        }
        self.finish_specialized_effect_event(execution, event_context, pending)
    }

    fn controlled_change_execution(
        &mut self,
        request: &RuntimeRunRequest,
        definition: &ToolDefinition,
        call: &ToolCall,
        output: ControlledChangeOutput,
        state_change: StateChange,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let output_bytes = serde_json::to_vec(&output).map_err(|_| RuntimePortFailure::Invalid)?;
        if output_bytes.len() as u64 > request.limits.max_output_bytes {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        let receipt_id = ReceiptId::from_raw(output.receipt_id.clone());
        let evidence = vec![EvidenceReference {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(self.next_id("evidence")?),
            kind: EvidenceKind::Receipt,
            source_id: format!("native:{}@{}", call.tool_id.as_str(), call.tool_version),
            object_id: output.operation_id.clone(),
            fragment: None,
            content_sha256: output.receipt_sha256.clone(),
            observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
        }];
        Ok(RuntimeToolExecution {
            receipt_id,
            receipt_sha256: output.receipt_sha256.clone(),
            result: ToolResult {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome: output.outcome,
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".to_owned(),
                    sha256: sha256(&output_bytes),
                    bytes: output_bytes,
                }),
                validation_issues: Vec::new(),
                evidence,
                error: None,
                elapsed_ms: 0,
                state_change,
            },
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates: Vec::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_effect_authority<D: agentmage_kernel_engine::authority_transaction::EffectDriver>(
        &mut self,
        registry: &agentmage_kernel_engine::tooling::ToolRegistry,
        policy: &agentmage_kernel_engine::policy::PolicyEngine,
        transaction: AuthorityTransactionRequest,
        driver: &mut D,
        event_context: Option<&RuntimeEffectEventContext<'_>>,
    ) -> Result<
        (
            agentmage_kernel_contracts::Receipt,
            Option<PendingRuntimeEffectCommit>,
        ),
        RuntimePortFailure,
    > {
        if let Some(context) = event_context {
            self.authority
                .authority_mut()
                .begin_effect_with_runtime_event(
                    registry,
                    policy,
                    transaction,
                    driver,
                    context.started_event.clone(),
                )
                .map_err(map_journal_failure)
                .map(|(receipt, pending)| (receipt, Some(pending)))
        } else {
            self.authority
                .authority_mut()
                .execute_effect(registry, policy, transaction, driver)
                .map_err(|_| RuntimePortFailure::Uncertain)
                .map(|receipt| (receipt, None))
        }
    }

    fn finish_effect_execution(
        &mut self,
        execution: RuntimeToolExecution,
        event_context: Option<RuntimeEffectEventContext<'_>>,
        pending: Option<PendingRuntimeEffectCommit>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        match (event_context, pending) {
            (None, None) => Ok((execution, Vec::new())),
            (Some(context), Some(pending)) => {
                let terminal = (context.build_terminal_event)(&execution)?;
                #[cfg(test)]
                story_22_1_crash_at("before-tool-terminal-commit");
                let events = self
                    .authority
                    .authority_mut()
                    .finish_effect_with_runtime_event(pending, terminal)
                    .map_err(map_journal_failure)?;
                #[cfg(test)]
                story_22_1_crash_at("after-tool-terminal-commit");
                self.authority
                    .revalidate_root()
                    .map_err(|_| RuntimePortFailure::Uncertain)?;
                Ok((execution, events.into()))
            }
            _ => Err(RuntimePortFailure::Invalid),
        }
    }

    fn finish_specialized_effect_event(
        &mut self,
        execution: RuntimeToolExecution,
        event_context: Option<RuntimeEffectEventContext<'_>>,
        pending: Option<PendingSpecializedEffectCommit>,
    ) -> Result<(RuntimeToolExecution, Vec<RuntimeEvent>), RuntimePortFailure> {
        match (event_context, pending) {
            (None, None) => Ok((execution, Vec::new())),
            (Some(context), Some(pending)) => {
                let terminal = (context.build_terminal_event)(&execution)?;
                let events = self
                    .authority
                    .authority_mut()
                    .finish_specialized_effect_with_runtime_event(pending, terminal)
                    .map_err(map_journal_failure)?;
                self.authority
                    .revalidate_root()
                    .map_err(|_| RuntimePortFailure::Uncertain)?;
                Ok((execution, events.into()))
            }
            _ => Err(RuntimePortFailure::Invalid),
        }
    }

    fn authority_transaction(
        &mut self,
        call: &ToolCall,
        issued: &IssuedCodingOperation<'workspace>,
    ) -> Result<AuthorityTransactionRequest, RuntimePortFailure> {
        let (approval, approved) = issued.generic_authority()?;
        let grant = &approved.grant;
        let action_id = grant.action_id.clone().ok_or(RuntimePortFailure::Invalid)?;
        let action_kind = grant.action_kind.ok_or(RuntimePortFailure::Invalid)?;
        let tool_id = grant.tool_id.clone().ok_or(RuntimePortFailure::Invalid)?;
        let tool_version = grant
            .tool_version
            .clone()
            .ok_or(RuntimePortFailure::Invalid)?;
        let context = PolicyEvaluationContext {
            actor_id: grant.actor_id.clone(),
            session_id: grant.session_id.clone(),
            task_id: grant.task_id.clone(),
            action_id,
            action_kind,
            tool_id,
            tool_version,
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: issued.resolved_at_epoch_ms,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw(self.next_id("transaction")?),
            OperationAttemptId::from_raw(self.next_id("attempt")?),
            approval.approval_id.clone(),
            grant.grant_id.clone(),
            call.clone(),
            context,
            issued.resolved_at_epoch_ms,
            format!("epoch-ms:{}", issued.resolved_at_epoch_ms),
        )
        .map_err(|_| RuntimePortFailure::Invalid)
    }

    fn request_matches(&self, request: &RuntimeRunRequest) -> bool {
        let profile = self.workspace.profile();
        verify_runtime_run_request(request).is_ok()
            && request.mode == RuntimeSessionMode::ControlledWrite
            && request.session_id == self.session_id
            && request.task.task_id.as_str() == profile.worktree().task_id
            && request.workspace_id == *profile.write_scope().workspace_id()
            && request.workspace_snapshot_sha256 == profile.worktree().record_sha256
            && request.repository_snapshot_id == *profile.repository_snapshot_id()
            && request.repository_snapshot_sha256 == profile.repository_snapshot_sha256()
            && request.model_profile == *profile.model_profile()
            && request.context_budget == profile.model_profile().context
            && request.tool_catalog_id == *profile.tool_catalog_id()
            && request.tool_catalog_sha256 == profile.tool_catalog_sha256()
            && request.visible_tools == profile.visible_tools()
            && request.policy_id == *self.policy.policy_id()
            && request.policy_sha256 == self.policy.engine().policy_sha256()
            && request.limits == *profile.limits()
            && self.policy.binds_run(
                &self.actor_id,
                &request.task.task_id,
                &request.run_id,
                request.limits.max_tool_calls,
            )
    }

    fn definition_matches(&self, definition: &ToolDefinition, call: &ToolCall) -> bool {
        self.workspace
            .profile()
            .registry()
            .get_tool(&call.tool_id, &call.tool_version)
            .is_some_and(|registered| registered == definition)
            && self
                .workspace
                .profile()
                .registry()
                .validate_arguments(call)
                .is_ok()
    }

    fn build_checkpoint_publication(
        &mut self,
        input: RuntimeCheckpointCommit<'_>,
    ) -> Result<RuntimeCheckpointPublication, RuntimePortFailure> {
        if !self.request_matches(input.request)
            || verify_runtime_continuation_state(input.continuation).is_err()
            || input.continuation_artifact.media_type != RUNTIME_CONTINUATION_MEDIA_TYPE
            || input.continuation.run_id != input.request.run_id
            || input.continuation.session_id != input.request.session_id
            || input.continuation.task_id != input.request.task.task_id
            || input.event_cursor.run_id != input.request.run_id
            || !input
                .artifacts
                .iter()
                .any(|reference| reference == input.continuation_artifact)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let checkpoint_id = SessionCheckpointId::from_raw(self.next_id("checkpoint")?);
        let plan_step_id = PlanStepId::from_raw(self.next_id("plan-step")?);
        let profile = self.workspace.profile();
        let plan_id = input
            .request
            .work_packet
            .plan_id
            .clone()
            .filter(|plan_id| plan_id.as_str() == profile.change_plan().plan_id())
            .ok_or(RuntimePortFailure::Invalid)?;
        let mut evidence_ids = input
            .continuation
            .evidence
            .iter()
            .map(|evidence| evidence.evidence_id.clone())
            .collect::<Vec<_>>();
        evidence_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        let citation_set_sha256 = serde_json::to_vec(&evidence_ids)
            .map(|bytes| sha256(&bytes))
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let next_action_sha256 = sha256(
            format!(
                "runtime-resume:{}:{}",
                input.request.run_id.as_str(),
                input.continuation.continuation_sha256
            )
            .as_bytes(),
        );
        let checkpoint = finalize_checkpoint(SessionCheckpoint {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            checkpoint_id,
            session_id: input.request.session_id.clone(),
            task_id: input.request.task.task_id.clone(),
            objective_sha256: sha256(input.request.task.objective.as_bytes()),
            plan_id,
            plan_revision: input.request.work_packet.revision,
            plan_step_id,
            next_action_sha256,
            workspace_id: input.request.workspace_id.clone(),
            workspace_state_sha256: profile.worktree().record_sha256.clone(),
            repository_snapshot_id: input.request.repository_snapshot_id.clone(),
            repository_branch: profile.worktree().branch_ref.clone(),
            repository_map_sha256: input.request.repository_snapshot_sha256.clone(),
            files: Vec::new(),
            instruction_sha256: profile.instruction_ledger().ledger_sha256.clone(),
            permission_profile_id: input.request.policy_id.as_str().to_owned(),
            permission_profile_sha256: input.request.policy_sha256.clone(),
            policy_id: input.request.policy_id.clone(),
            policy_sha256: input.request.policy_sha256.clone(),
            model_profile_id: input.request.model_profile.profile_id.clone(),
            model_manifest_sha256: input.request.model_profile.manifest_sha256.clone(),
            model_runtime_sha256: input.request.model_profile.runtime.runtime_sha256.clone(),
            evidence_ids,
            citation_set_sha256,
            blockers: Vec::new(),
            context_packet_sha256: input.continuation.continuation_sha256.clone(),
            action_id: None,
            action_state: None,
            consumed_grant_id: None,
            receipt_id: None,
            receipt_sha256: None,
            ephemeral: false,
            checkpoint_sha256: "0".repeat(64),
        })
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let binding = seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            session_id: input.request.session_id.clone(),
            task_id: input.request.task.task_id.clone(),
            run_id: input.request.run_id.clone(),
            event_cursor: input.event_cursor.clone(),
            artifacts: input.artifacts.to_vec(),
            binding_sha256: "0".repeat(64),
        })
        .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(RuntimeCheckpointPublication {
            checkpoint,
            binding,
        })
    }

    fn next_id(&mut self, prefix: &str) -> Result<String, RuntimePortFailure> {
        self.identities
            .next(prefix)
            .map_err(|_| RuntimePortFailure::Unavailable)
    }
}

impl<'workspace, 'session, 'platform, I, E, G> RuntimeToolBoundary
    for LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    fn evaluate(
        &mut self,
        request: &RuntimeRunRequest,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.evaluate_call(request, operation_id, definition, call, now_epoch_ms, None)
            .map(|(evaluation, _)| evaluation)
    }

    fn resolve(
        &mut self,
        request: &RuntimeRunRequest,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.resolve_call(
            request,
            challenge,
            response,
            definition,
            call,
            now_epoch_ms,
            None,
        )
        .map(|(evaluation, _)| evaluation)
    }

    fn execute(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        self.execute_call(request, evaluation, definition, call, cancellation, None)
            .map(|(execution, _)| execution)
    }
}

impl<'workspace, 'session, 'platform, I, E, G> RuntimeJournalPort
    for LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    fn append_runtime_event(&mut self, event: &RuntimeEvent) -> Result<(), RuntimePortFailure> {
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        self.authority
            .authority_mut()
            .record_runtime_event(event.clone())
            .map_err(map_journal_failure)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)
    }

    fn flush_runtime_events(&mut self) -> Result<(), RuntimePortFailure> {
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        self.authority
            .authority_mut()
            .flush_runtime_events()
            .map_err(map_journal_failure)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)
    }

    fn load_runtime_events(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Result<Vec<RuntimeEvent>, RuntimePortFailure> {
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let events = self
            .authority
            .authority()
            .runtime_events(run_id)
            .map_err(map_journal_failure)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        Ok(events)
    }
}

impl<'workspace, 'session, 'platform, I, E, G> RuntimeCorrectnessTransactionPort
    for LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    fn evaluate_with_correctness_event(
        &mut self,
        request: &RuntimeRunRequest,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
        build_event: &mut dyn FnMut(
            &RuntimePermissionEvaluation,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimePermissionEvaluation, RuntimeEvent), RuntimePortFailure> {
        let (evaluation, event) = self.evaluate_call(
            request,
            operation_id,
            definition,
            call,
            now_epoch_ms,
            Some(build_event),
        )?;
        Ok((evaluation, event.ok_or(RuntimePortFailure::Invalid)?))
    }

    fn resolve_with_correctness_event(
        &mut self,
        request: &RuntimeRunRequest,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
        build_event: &mut dyn FnMut(
            &RuntimePermissionEvaluation,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimePermissionEvaluation, RuntimeEvent), RuntimePortFailure> {
        let (evaluation, event) = self.resolve_call(
            request,
            challenge,
            response,
            definition,
            call,
            now_epoch_ms,
            Some(build_event),
        )?;
        Ok((evaluation, event.ok_or(RuntimePortFailure::Invalid)?))
    }

    fn execute_with_correctness_events(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
        started_event: RuntimeEvent,
        build_terminal_event: &mut dyn FnMut(
            &RuntimeToolExecution,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<RuntimeToolCorrectnessCommit, RuntimePortFailure> {
        let (execution, events) = self.execute_call(
            request,
            evaluation,
            definition,
            call,
            cancellation,
            Some(RuntimeEffectEventContext {
                started_event,
                build_terminal_event,
            }),
        )?;
        Ok(RuntimeToolCorrectnessCommit { execution, events })
    }

    fn commit_checkpoint_with_correctness_event(
        &mut self,
        input: RuntimeCheckpointCommit<'_>,
        build_event: &mut dyn FnMut(
            &RuntimeCheckpointPublication,
        ) -> Result<RuntimeEvent, RuntimePortFailure>,
    ) -> Result<(RuntimeCheckpointPublication, RuntimeEvent), RuntimePortFailure> {
        let publication = self.build_checkpoint_publication(input)?;
        let event = build_event(&publication)?;
        #[cfg(test)]
        story_22_1_crash_at("before-checkpoint-commit");
        self.authority
            .authority_mut()
            .checkpoint_runtime_session_with_event(
                &publication.checkpoint,
                &publication.binding,
                event.clone(),
            )
            .map_err(map_journal_failure)?;
        #[cfg(test)]
        story_22_1_crash_at("after-checkpoint-commit");
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        Ok((publication, event))
    }
}

impl<'workspace, 'session, 'platform, I, E, G> RuntimeArtifactPort
    for LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    fn publish_runtime_artifact(
        &mut self,
        manifest: RuntimeArtifactManifest,
        payload: &[u8],
    ) -> Result<RuntimeArtifactRef, RuntimePortFailure> {
        let publication = self
            .authority
            .publish_runtime_artifact(manifest.clone(), &mut Cursor::new(payload))
            .map_err(map_journal_failure)?;
        if publication.manifest != manifest {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(publication.reference)
    }
}

impl<'workspace, 'session, 'platform, I, E, G> RuntimeCheckpointPort
    for LinuxCodingRuntimeBoundary<'workspace, 'session, 'platform, I, E, G>
where
    I: CodingIdentitySource,
    E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
{
    fn commit_runtime_checkpoint(
        &mut self,
        input: RuntimeCheckpointCommit<'_>,
    ) -> Result<RuntimeCheckpointPublication, RuntimePortFailure> {
        let publication = self.build_checkpoint_publication(input)?;
        self.authority
            .checkpoint_runtime_session(&publication.checkpoint, &publication.binding)
            .map_err(map_journal_failure)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        Ok(publication)
    }

    fn load_runtime_checkpoint(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Option<RuntimeResumeSnapshot>, RuntimePortFailure> {
        if request.event_cursor.is_none() || !self.request_matches(request) {
            return Err(RuntimePortFailure::Invalid);
        }
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        let checkpoint = self
            .authority
            .authority()
            .current_session_checkpoint()
            .map_err(map_journal_failure)?;
        let binding = self
            .authority
            .authority()
            .current_runtime_resume_binding()
            .map_err(map_journal_failure)?;
        let (Some(checkpoint), Some(binding)) = (checkpoint, binding) else {
            return Ok(None);
        };
        let events = self
            .authority
            .authority()
            .runtime_events(&request.run_id)
            .map_err(map_journal_failure)?;
        let event_index = usize::try_from(binding.event_cursor.sequence)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let event = events.get(event_index).ok_or(RuntimePortFailure::Invalid)?;
        let RuntimeEventKind::ArtifactCreated {
            artifact_id,
            manifest_sha256,
        } = &event.kind
        else {
            return Err(RuntimePortFailure::Invalid);
        };
        if event.run_id != binding.event_cursor.run_id
            || event.event_id != binding.event_cursor.event_id
            || event.event_sha256 != binding.event_cursor.event_sha256
            || event.sequence != binding.event_cursor.sequence
            || event.turn_id.is_some()
            || event.operation_id.is_some()
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let continuation_artifact = binding
            .artifacts
            .iter()
            .find(|reference| {
                reference.artifact_id == *artifact_id
                    && reference.manifest_sha256 == *manifest_sha256
                    && reference.media_type == RUNTIME_CONTINUATION_MEDIA_TYPE
            })
            .cloned()
            .ok_or(RuntimePortFailure::Invalid)?;
        let now_epoch_ms = events
            .last()
            .map(|retained| retained.occurred_at_epoch_ms)
            .filter(|value| *value > 0)
            .ok_or(RuntimePortFailure::Invalid)?;
        let bytes = self
            .authority
            .read_runtime_artifact(&RuntimeArtifactReadRequest {
                session_id: request.session_id.clone(),
                task_id: request.task.task_id.clone(),
                policy_sha256: request.policy_sha256.clone(),
                reference: continuation_artifact.clone(),
                now_epoch_ms,
                maximum_bytes: MAX_RUNTIME_ARTIFACT_BYTES,
            })
            .map_err(map_journal_failure)?;
        let continuation =
            decode_runtime_continuation_state(&bytes).map_err(|_| RuntimePortFailure::Invalid)?;
        self.authority
            .revalidate_root()
            .map_err(|_| RuntimePortFailure::Uncertain)?;
        Ok(Some(RuntimeResumeSnapshot {
            checkpoint,
            binding,
            continuation,
            continuation_artifact,
        }))
    }
}

fn map_journal_failure(error: DurableAuthorityError) -> RuntimePortFailure {
    match error {
        DurableAuthorityError::RuntimeJournal(RuntimeJournalError::QueueSaturated) => {
            RuntimePortFailure::ResourceExhausted
        }
        DurableAuthorityError::RuntimeJournal(error) if !error.poisons_writer() => {
            RuntimePortFailure::Invalid
        }
        DurableAuthorityError::RuntimeJournal(_)
        | DurableAuthorityError::Store(_)
        | DurableAuthorityError::Poisoned => RuntimePortFailure::Uncertain,
        DurableAuthorityError::RuntimeArtifact(error) if error.poisons_runtime() => {
            RuntimePortFailure::Uncertain
        }
        DurableAuthorityError::Grant(_)
        | DurableAuthorityError::Transaction(_)
        | DurableAuthorityError::Checkpoint(_)
        | DurableAuthorityError::WriteApproval(_)
        | DurableAuthorityError::FilesystemApproval(_)
        | DurableAuthorityError::WriteTransaction(_)
        | DurableAuthorityError::FilesystemTransaction(_)
        | DurableAuthorityError::RuntimeArtifact(_) => RuntimePortFailure::Invalid,
    }
}

fn operation_targets(
    binding: &LinuxCodingTargetBinding,
) -> Result<Vec<agentmage_kernel_contracts::GrantTarget>, RuntimePortFailure> {
    (0..binding.target_count())
        .map(|index| {
            binding
                .target(index)
                .cloned()
                .ok_or(RuntimePortFailure::Invalid)
        })
        .collect()
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ControlledChangeOutput {
    schema_version: u16,
    operation_id: String,
    outcome: OperationOutcome,
    path_sha256: String,
    preimage_sha256: Option<String>,
    postimage_sha256: String,
    receipt_id: String,
    receipt_sha256: String,
}

fn coding_write_review(
    structured_patch: bool,
    operation_plan_sha256: &str,
) -> WriteReviewNarrative {
    let change = if structured_patch {
        "exact structured replacement"
    } else {
        "exact absent-file creation"
    };
    WriteReviewNarrative {
        rationale: format!("Apply one {change} bound to operation plan {operation_plan_sha256}"),
        behavior_change: format!("The approved owned worktree receives only the reviewed {change}"),
        verification_plan: vec![
            "Freshly observe the exact approved postimage after atomic application".to_owned(),
        ],
        risks: vec!["Concurrent target changes must cause refusal or exact restoration".to_owned()],
        rollback: if structured_patch {
            "Restore the exact reviewed preimage if application or postimage verification fails"
                .to_owned()
        } else {
            "Remove only the newly created exact file through a separately approved operation"
                .to_owned()
        },
        unverified_assumptions: vec![
            "Separately permissioned project validation has not run yet".to_owned(),
        ],
    }
}

fn coding_write_verification() -> Vec<String> {
    vec!["postwrite.exact-observation".to_owned()]
}

const fn write_transaction_outcome(
    outcome: WriteTransactionOutcome,
) -> (OperationOutcome, StateChange) {
    match outcome {
        WriteTransactionOutcome::Committed => (OperationOutcome::Succeeded, StateChange::Changed),
        WriteTransactionOutcome::FailedNoChange | WriteTransactionOutcome::Restored => {
            (OperationOutcome::Failed, StateChange::NotChanged)
        }
        WriteTransactionOutcome::Uncertain => (OperationOutcome::Uncertain, StateChange::Uncertain),
    }
}

const fn filesystem_transaction_outcome(
    outcome: FilesystemTransactionOutcome,
) -> (OperationOutcome, StateChange) {
    match outcome {
        FilesystemTransactionOutcome::Committed => {
            (OperationOutcome::Succeeded, StateChange::Changed)
        }
        FilesystemTransactionOutcome::FailedNoChange | FilesystemTransactionOutcome::Restored => {
            (OperationOutcome::Failed, StateChange::NotChanged)
        }
        FilesystemTransactionOutcome::Uncertain => {
            (OperationOutcome::Uncertain, StateChange::Uncertain)
        }
    }
}

fn specialized_receipt_id(kind: &str, receipt_sha256: &str) -> String {
    format!("receipt-{kind}-{}", &receipt_sha256[..32])
}

fn challenge_matches(
    request: &RuntimeRunRequest,
    challenge: &RuntimeApprovalChallenge,
    pending: &PendingCodingOperation<'_>,
    call: &ToolCall,
) -> bool {
    challenge.run_id == request.run_id
        && challenge.task_id == request.task.task_id
        && challenge.operation_id == pending.operation_id
        && challenge.tool_call_id == call.tool_call_id
        && challenge.approval_id == *pending.authority.approval_id()
        && challenge.proposed_grant_id == *pending.authority.proposed_grant_id()
        && challenge.operation == pending.operation
        && challenge.preview_sha256 == pending.authority.preview_sha256()
        && challenge.expires_at_epoch_ms == pending.expires_at_epoch_ms
        && pending.tool_call == *call
}

#[derive(Serialize)]
struct ParentPreviewMaterial<'material> {
    run_id: &'material str,
    operation_id: &'material str,
    profile_sha256: &'material str,
    operation_plan_sha256: &'material str,
    parent_targets: &'material [agentmage_kernel_contracts::GrantTarget],
    excluded_targets: &'material [agentmage_kernel_contracts::GrantTarget],
}

fn parent_preview_sha256(
    request: &RuntimeRunRequest,
    operation_id: &RuntimeOperationId,
    profile_sha256: &str,
    operation_plan_sha256: &str,
    parent_targets: &[agentmage_kernel_contracts::GrantTarget],
    excluded_targets: &[agentmage_kernel_contracts::GrantTarget],
) -> Result<String, RuntimePortFailure> {
    serde_json::to_vec(&ParentPreviewMaterial {
        run_id: request.run_id.as_str(),
        operation_id: operation_id.as_str(),
        profile_sha256,
        operation_plan_sha256,
        parent_targets,
        excluded_targets,
    })
    .map(|bytes| sha256(&bytes))
    .map_err(|_| RuntimePortFailure::Invalid)
}

fn canonical_sha256<T>(value: &T) -> Result<String, RuntimePortFailure>
where
    T: agentmage_kernel_contracts::VersionedContract,
{
    to_canonical_json(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| RuntimePortFailure::Invalid)
}

const fn read_outcome(outcome: ReadOnlyOutcome) -> OperationOutcome {
    match outcome {
        ReadOnlyOutcome::Succeeded
        | ReadOnlyOutcome::NoResult
        | ReadOnlyOutcome::Partial
        | ReadOnlyOutcome::Truncated => OperationOutcome::Succeeded,
        ReadOnlyOutcome::Denied => OperationOutcome::Denied,
        ReadOnlyOutcome::Cancelled => OperationOutcome::Cancelled,
        ReadOnlyOutcome::Malformed | ReadOnlyOutcome::Failed => OperationOutcome::Failed,
    }
}

fn captured_output_matches(output: &CommandCapturedOutput, receipt: &CommandReceipt) -> bool {
    output.stdout().len() as u64 == receipt.stdout_retained_bytes
        && output.stderr().len() as u64 == receipt.stderr_retained_bytes
        && if receipt.stdout_truncated {
            receipt.stdout_total_bytes > receipt.stdout_retained_bytes
        } else {
            receipt.stdout_total_bytes == receipt.stdout_retained_bytes
                && sha256(output.stdout()) == receipt.stdout_sha256
        }
        && if receipt.stderr_truncated {
            receipt.stderr_total_bytes > receipt.stderr_retained_bytes
        } else {
            receipt.stderr_total_bytes == receipt.stderr_retained_bytes
                && sha256(output.stderr()) == receipt.stderr_sha256
        }
}

fn captured_output_artifact_candidates(
    output: &CommandCapturedOutput,
    stdout_kind: RuntimeArtifactKind,
    stderr_kind: RuntimeArtifactKind,
) -> Vec<RuntimeToolArtifactCandidate> {
    let mut candidates = Vec::with_capacity(2);
    if !output.stdout().is_empty() {
        candidates.push(RuntimeToolArtifactCandidate {
            kind: stdout_kind,
            media_type: "application/octet-stream".to_owned(),
            bytes: output.stdout().to_vec(),
        });
    }
    if !output.stderr().is_empty() {
        candidates.push(RuntimeToolArtifactCandidate {
            kind: stderr_kind,
            media_type: "application/octet-stream".to_owned(),
            bytes: output.stderr().to_vec(),
        });
    }
    candidates
}

fn prepare_kernel_git_inspection(
    request: &GitInspectionRequest,
    capability_plan: &GitCommandPlan,
    call_argument_sha256: &str,
    operation_plan_sha256: &str,
) -> Result<PreparedRepositoryInspection, RuntimePortFailure> {
    let prepared = prepare_repository_inspection(
        RepositoryInspectionRequest {
            schema_version: request.schema_version,
            operation: kernel_git_operation(request.operation),
            revision: request.revision.clone(),
            object_id: request.object_id.clone(),
            pathspecs: request.pathspecs.clone(),
            max_records: request.max_records,
            max_output_bytes: request.max_output_bytes,
        },
        call_argument_sha256,
        operation_plan_sha256,
    )
    .map_err(|_| RuntimePortFailure::Invalid)?;
    if capability_plan.argv.first().map(String::as_str) != Some("git")
        || capability_plan.argv[1..] != *prepared.arguments()
        || capability_plan.environment != *prepared.environment()
        || capability_plan.stdin != prepared.stdin()
    {
        return Err(RuntimePortFailure::Invalid);
    }
    Ok(prepared)
}

const fn kernel_git_operation(operation: GitInspectionOperation) -> RepositoryInspectionOperation {
    match operation {
        GitInspectionOperation::Status => RepositoryInspectionOperation::Status,
        GitInspectionOperation::CurrentBranch => RepositoryInspectionOperation::CurrentBranch,
        GitInspectionOperation::Upstream => RepositoryInspectionOperation::Upstream,
        GitInspectionOperation::BranchList => RepositoryInspectionOperation::BranchList,
        GitInspectionOperation::Log => RepositoryInspectionOperation::Log,
        GitInspectionOperation::Diff => RepositoryInspectionOperation::Diff,
        GitInspectionOperation::StagedDiff => RepositoryInspectionOperation::StagedDiff,
        GitInspectionOperation::Show => RepositoryInspectionOperation::Show,
        GitInspectionOperation::WorktreeList => RepositoryInspectionOperation::WorktreeList,
        GitInspectionOperation::Object => RepositoryInspectionOperation::Object,
        GitInspectionOperation::Ref => RepositoryInspectionOperation::Ref,
        GitInspectionOperation::DirtyTree => RepositoryInspectionOperation::DirtyTree,
        GitInspectionOperation::UntrackedFiles => RepositoryInspectionOperation::UntrackedFiles,
    }
}

const fn repository_inspection_outcome(
    platform: &RepositoryInspectionPlatformResult,
) -> OperationOutcome {
    if !platform.descendants_terminated {
        return OperationOutcome::Uncertain;
    }
    match platform.termination {
        RepositoryInspectionTermination::Exited if matches!(platform.exit_code, Some(0)) => {
            OperationOutcome::Succeeded
        }
        RepositoryInspectionTermination::Exited
        | RepositoryInspectionTermination::OutputLimit
        | RepositoryInspectionTermination::LaunchFailed => OperationOutcome::Failed,
        RepositoryInspectionTermination::Cancelled => OperationOutcome::Cancelled,
        RepositoryInspectionTermination::TimedOut => OperationOutcome::TimedOut,
    }
}

const fn git_outcome(outcome: GitInspectionOutcome) -> OperationOutcome {
    match outcome {
        GitInspectionOutcome::Succeeded
        | GitInspectionOutcome::NoResult
        | GitInspectionOutcome::Truncated => OperationOutcome::Succeeded,
        GitInspectionOutcome::Failed => OperationOutcome::Failed,
    }
}

const fn validation_outcome(status: ValidationStatus) -> OperationOutcome {
    match status {
        ValidationStatus::Passed => OperationOutcome::Succeeded,
        ValidationStatus::Cancelled => OperationOutcome::Cancelled,
        ValidationStatus::TimedOut => OperationOutcome::TimedOut,
        ValidationStatus::AssertionFailed
        | ValidationStatus::CompileFailed
        | ValidationStatus::InfrastructureFailed
        | ValidationStatus::Crashed
        | ValidationStatus::Flaky
        | ValidationStatus::SkippedOnly
        | ValidationStatus::Malformed
        | ValidationStatus::Truncated
        | ValidationStatus::ZeroTests
        | ValidationStatus::Unverified
        | ValidationStatus::SensitiveOutput => OperationOutcome::Failed,
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
    use std::io::Write as _;
    use std::ops::Deref;
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use agentmage_capability_read_only::{
        GIT_INSPECTION_TOOL_ID, GIT_INSPECTION_TOOL_VERSION, GitInspectionOperation,
        GitInspectionRequest, ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest, ReadOnlyToolKind,
        plan_git_inspection,
    };
    use agentmage_capability_repository_map::{
        GitTrackedState, RepositoryFileInput, RepositoryMapInput, StructuredArtifactClass,
        StructuredEdit, StructuredLanguage, build_repository_map,
    };
    #[cfg(feature = "workflow-caller")]
    use agentmage_kernel_contracts::GrantOperation;
    use agentmage_kernel_contracts::{
        ActionId, AgentStateKind, AuthorityClass, BoundaryKind, BudgetLimit, BudgetResource,
        CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
        ClosedModelProposal, CorrelationId, DataSensitivity, ExactModelProfile, GrantStatus,
        GrantTarget, ModelCancellationProbe, ModelContextPacket, ModelMessageRole,
        ModelProposalKind, ModelResourceReport, ModelRunRequest, ModelRunResult,
        ModelRunTerminalState, ModelStreamId, ModelToolCallCandidate, PathResolutionIntent, PlanId,
        ProposalId, RollbackPlan, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
        RuntimeApprovalResponse, RuntimeEventCursor, RuntimeOperationId, RuntimeRunId,
        RuntimeRunRequest, RuntimeSessionMode, RuntimeTurnId, SessionId, StopCondition,
        StopConditionKind, Task, TaskId, TaskStatus, ToolCall, ToolCallId, ToolId, WorkPacket,
        WorkPacketId, WorkPacketState, WorkspaceAuthorizationId, WorkspacePath,
    };
    #[cfg(feature = "workflow-caller")]
    use agentmage_kernel_engine::workflow_authority::{
        WorkflowAuthorityLayer, WorkflowAuthorityLayerKind, intersect_workflow_authority,
    };
    use agentmage_kernel_engine::{
        command_runner::{
            CommandLaunchPermit, CommandPlatformResult, CommandRequest, CommandTermination,
        },
        model_codec::proposal_digest,
        operational_store::{OperationalStoreKeyError, OperationalStoreKeyProvider},
        repository_inspection::{
            BoundedRepositoryInspectionExecutor, RepositoryInspectionLaunchPermit,
            RepositoryInspectionPlatformResult, RepositoryInspectionTermination,
        },
        runtime_coordinator::{
            seal_runtime_approval_challenge, seal_runtime_run_request, verify_runtime_outcome,
        },
        runtime_event::RuntimeEventSequence,
        runtime_loop::{
            RuntimeClock, RuntimeCoordinatorStep, RuntimeLoopError, RuntimeModelPort,
            RuntimePermissionEvaluation, RuntimeToolBoundary, runtime_action_id,
        },
    };
    use agentmage_platform_linux::{
        LinuxBoundedRepositoryInspectionExecutor, LinuxGitArtifact, LinuxSandboxLimits,
        LinuxSandboxManifest, LinuxSandboxRunner, linux_repository_path_sha256,
        open_test_linux_authority, resolve_test_linux_workspace_object,
    };

    use super::*;
    #[cfg(feature = "interactive-cli")]
    use crate::cli::{
        render_runtime_event_human, render_runtime_event_json, render_runtime_outcome_human,
        render_runtime_outcome_json,
    };
    use crate::{
        coding_authority::{CodingRuntimePolicyRequest, build_coding_runtime_policy},
        coding_changes::{
            CONTROLLED_CHANGE_TOOL_VERSION, CONTROLLED_CREATE_TOOL_ID,
            ControlledFileClassification, ControlledFileCreationProposal, STRUCTURED_PATCH_TOOL_ID,
            StructuredPatchProposal, controlled_create_parent_observation_sha256,
        },
        coding_client::{
            CodingApprovalPort, CodingClientError, CodingEventSink, DenyHeadlessApproval,
            drive_coding_client,
        },
        coding_context::{CodingContextPort, CodingTokenCounter},
        coding_harness::{
            compose_durable_coding_coordinator, compose_ephemeral_coding_coordinator,
        },
        coding_session::{CodingSessionProfile, tests::input_with_worktree_path_sha256},
        coding_tools::TargetedValidationRequest,
        coding_verifier::{
            CodingCompletionCandidate, CodingTerminalClaim, coding_completion_payload,
        },
        linux_coding::LinuxCodingWorkspace,
    };
    #[cfg(feature = "workflow-caller")]
    use crate::{
        workflow_assignment::{
            ChildReviewDisposition, ChildRuntimeAssignment, ChildRuntimeCaller,
            seal_child_runtime_assignment,
        },
        workflow_caller::{
            InMemoryWorkflowCaller, WorkflowCallerError, WorkflowCallerIdentity,
            WorkflowCallerState, WorkflowRuntimeSubmission, seal_workflow_runtime_submission,
        },
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    struct TempRoot(PathBuf);

    impl Deref for TempRoot {
        type Target = PathBuf;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    const CHECKPOINT_CLOCK_UNARMED: usize = usize::MAX;
    const ARTIFACT_RESUME_METRIC_PREFIX: &str = "AGENTMAGE_ARTIFACT_RESUME=";
    const STORY_22_1_CRASH_ROOT_ENVIRONMENT: &str = "AGENTMAGE_STORY_22_1_CRASH_ROOT";
    const STORY_22_1_CRASH_CHILD_EXIT: i32 = 93;
    const STORY_22_1_LONG_RESUME_METRIC_PREFIX: &str = "AGENTMAGE_STORY_22_1_LONG_RESUME=";
    const STORY_22_1_RESUME_METRIC_PREFIX: &str = "AGENTMAGE_STORY_22_1_RESUME=";
    const STORY_22_1_REPETITIONS_PER_BOUNDARY: usize = 25;
    const STORY_22_1_CRASH_BOUNDARIES: [&str; 4] = [
        "before-tool-terminal-commit",
        "after-tool-terminal-commit",
        "before-checkpoint-commit",
        "after-checkpoint-commit",
    ];

    struct TestIdentities {
        next: u64,
        checkpoint_stop: Option<(Arc<AtomicUsize>, usize)>,
    }

    impl TestIdentities {
        fn new(next: u64) -> Self {
            Self {
                next,
                checkpoint_stop: None,
            }
        }

        fn stop_after_checkpoint(next: u64, checkpoint_clock: Arc<AtomicUsize>) -> Self {
            Self {
                next,
                checkpoint_stop: Some((checkpoint_clock, 1)),
            }
        }

        fn stop_after_checkpoint_count(
            next: u64,
            checkpoint_clock: Arc<AtomicUsize>,
            checkpoint_count: usize,
        ) -> Self {
            assert!(checkpoint_count > 0);
            Self {
                next,
                checkpoint_stop: Some((checkpoint_clock, checkpoint_count)),
            }
        }
    }

    impl CodingIdentitySource for TestIdentities {
        fn next(&mut self, prefix: &str) -> Result<String, LinuxCodingRuntimeError> {
            self.next += 1;
            if prefix == "checkpoint"
                && let Some((checkpoint_clock, remaining)) = &mut self.checkpoint_stop
            {
                *remaining -= 1;
                if *remaining == 0 {
                    checkpoint_clock.store(1, Ordering::SeqCst);
                }
            }
            Ok(format!("{prefix}-{:032x}", self.next))
        }
    }

    enum ScriptedCodingStep {
        Tool(ModelToolCallCandidate),
        Complete(ContractPayload),
        Blocked(ContractPayload),
    }

    struct ScriptedCodingModel {
        profile: ExactModelProfile,
        steps: VecDeque<ScriptedCodingStep>,
        calls: u32,
    }

    impl RuntimeModelPort for ScriptedCodingModel {
        fn exact_profile(&self) -> &ExactModelProfile {
            &self.profile
        }

        fn run_model(
            &mut self,
            request: &ModelRunRequest,
            context: &ModelContextPacket,
            _cancellation: Option<&dyn ModelCancellationProbe>,
        ) -> Result<ModelRunResult, RuntimePortFailure> {
            assert!(
                context
                    .messages
                    .iter()
                    .any(|message| message.role == ModelMessageRole::System)
            );
            let step = self
                .steps
                .pop_front()
                .ok_or(RuntimePortFailure::ResourceExhausted)?;
            self.calls += 1;
            let (kind, payload, tool_call) = match step {
                ScriptedCodingStep::Tool(call) => (ModelProposalKind::ToolCall, None, Some(call)),
                ScriptedCodingStep::Complete(payload) => {
                    (ModelProposalKind::CompletionCandidate, Some(payload), None)
                }
                ScriptedCodingStep::Blocked(payload) => {
                    (ModelProposalKind::Blocked, Some(payload), None)
                }
            };
            let mut proposal = ClosedModelProposal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                proposal_id: ProposalId::from_raw(format!("coding-e2e-proposal-{}", self.calls)),
                model_run_id: request.model_run_id.clone(),
                context_packet_id: request.context_packet_id.clone(),
                profile_id: request.profile_id.clone(),
                codec_id: self.profile.codec.codec_id.clone(),
                correlation_id: request.correlation_id.clone(),
                kind,
                payload,
                tool_call,
                proposal_sha256: "0".repeat(64),
            };
            proposal.proposal_sha256 =
                proposal_digest(&proposal).map_err(|_| RuntimePortFailure::Invalid)?;
            Ok(ModelRunResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                model_run_id: request.model_run_id.clone(),
                stream_id: ModelStreamId::from_raw(format!("coding-e2e-stream-{}", self.calls)),
                correlation_id: request.correlation_id.clone(),
                terminal_state: ModelRunTerminalState::Proposed,
                fragment_count: 1,
                response_sha256: sha256(proposal.proposal_sha256.as_bytes()),
                proposal: Some(proposal),
                failure: None,
                resources: ModelResourceReport {
                    adapter_id: request.adapter_id.clone(),
                    profile_id: request.profile_id.clone(),
                    model_run_id: Some(request.model_run_id.clone()),
                    resident_memory_bytes: 1,
                    accelerator_memory_bytes: 0,
                    input_tokens: context.input_tokens,
                    output_tokens: 1,
                    elapsed_ms: 1,
                },
            })
        }
    }

    struct FixtureTokenCounter;

    impl CodingTokenCounter for FixtureTokenCounter {
        fn counter_id(&self) -> &str {
            "fixture-counter-v1"
        }

        fn count_tokens(&mut self, bytes: &[u8]) -> Result<u32, RuntimePortFailure> {
            u32::try_from(bytes.len().div_ceil(4).max(1))
                .map_err(|_| RuntimePortFailure::ResourceExhausted)
        }
    }

    struct FixtureClock(u64);

    impl RuntimeClock for FixtureClock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            self.0 += 1;
            Ok(self.0)
        }
    }

    struct StopAfterCheckpointClock {
        now_epoch_ms: u64,
        checkpoint_clock: Arc<AtomicUsize>,
    }

    impl RuntimeClock for StopAfterCheckpointClock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            let remaining = self.checkpoint_clock.load(Ordering::SeqCst);
            if remaining == 0 {
                return Err(RuntimePortFailure::Unavailable);
            }
            if remaining != CHECKPOINT_CLOCK_UNARMED {
                self.checkpoint_clock.fetch_sub(1, Ordering::SeqCst);
            }
            self.now_epoch_ms += 1;
            Ok(self.now_epoch_ms)
        }
    }

    struct AllowApproval;

    impl CodingApprovalPort for AllowApproval {
        fn decide(
            &mut self,
            _challenge: &RuntimeApprovalChallenge,
        ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
            Ok(RuntimeApprovalDisposition::Allow)
        }
    }

    #[derive(Default)]
    struct CollectingEventSink(Vec<RuntimeEvent>);

    impl CodingEventSink for CollectingEventSink {
        fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
            self.0.push(event.clone());
            Ok(())
        }
    }

    struct FakeCommandExecutor {
        launches: usize,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        exit_code: i32,
        shared_launches: Option<Arc<AtomicUsize>>,
    }

    impl Default for FakeCommandExecutor {
        fn default() -> Self {
            Self {
                launches: 0,
                stdout: b"command-ok\n".to_vec(),
                stderr: Vec::new(),
                exit_code: 0,
                shared_launches: None,
            }
        }
    }

    impl BoundedCommandExecutor for FakeCommandExecutor {
        type WorkingDirectory = LinuxAuthorizedWorkspace;

        fn execute(
            &mut self,
            _permit: CommandLaunchPermit<'_>,
            working_directory: &Self::WorkingDirectory,
            cancellation: &CancellationToken,
        ) -> CommandPlatformResult {
            self.launches += 1;
            if let Some(shared_launches) = &self.shared_launches {
                shared_launches.fetch_add(1, Ordering::SeqCst);
            }
            assert!(working_directory.revalidate().is_ok());
            assert!(!cancellation.is_cancelled());
            let stdout = self.stdout.clone();
            let stderr = self.stderr.clone();
            CommandPlatformResult {
                termination: CommandTermination::Exited,
                exit_code: Some(self.exit_code),
                signal: None,
                stdout_sha256: sha256(&stdout),
                stdout_total_bytes: stdout.len() as u64,
                stdout,
                stderr_sha256: sha256(&stderr),
                stderr_total_bytes: stderr.len() as u64,
                stderr,
                elapsed_ms: 2,
                descendants_terminated: true,
                platform_code: "fixture.command.exited".to_owned(),
            }
        }
    }

    struct FakeGitExecutor {
        launches: usize,
        stdout: Vec<u8>,
        shared_launches: Option<Arc<AtomicUsize>>,
        durable_launch_record: Option<PathBuf>,
    }

    impl Default for FakeGitExecutor {
        fn default() -> Self {
            Self {
                launches: 0,
                stdout: b"# branch.head main\0? src/new.rs\0".to_vec(),
                shared_launches: None,
                durable_launch_record: None,
            }
        }
    }

    impl FakeGitExecutor {
        fn clean() -> Self {
            Self {
                launches: 0,
                stdout: b"# branch.head main\0".to_vec(),
                shared_launches: None,
                durable_launch_record: None,
            }
        }

        fn clean_with_counter(shared_launches: Arc<AtomicUsize>) -> Self {
            Self {
                launches: 0,
                stdout: b"# branch.head main\0".to_vec(),
                shared_launches: Some(shared_launches),
                durable_launch_record: None,
            }
        }

        fn clean_with_durable_counter(path: PathBuf) -> Self {
            Self {
                launches: 0,
                stdout: b"# branch.head main\0".to_vec(),
                shared_launches: None,
                durable_launch_record: Some(path),
            }
        }
    }

    impl BoundedRepositoryInspectionExecutor for FakeGitExecutor {
        type WorkingDirectory = LinuxAuthorizedWorkspace;

        fn execute(
            &mut self,
            permit: RepositoryInspectionLaunchPermit<'_>,
            working_directory: &Self::WorkingDirectory,
            cancellation: &CancellationToken,
        ) -> RepositoryInspectionPlatformResult {
            self.launches += 1;
            if let Some(shared_launches) = &self.shared_launches {
                shared_launches.fetch_add(1, Ordering::SeqCst);
            }
            if let Some(path) = &self.durable_launch_record {
                let prior = fs::read_to_string(path)
                    .ok()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(0);
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(path)
                    .expect("durable launch record opens");
                write!(file, "{}", prior + 1).expect("durable launch count writes");
                file.sync_all().expect("durable launch count synchronizes");
            }
            assert!(working_directory.revalidate().is_ok());
            assert!(!cancellation.is_cancelled());
            assert_eq!(permit.prepared().arguments()[9], "status");
            let stdout = self.stdout.clone();
            RepositoryInspectionPlatformResult {
                termination: RepositoryInspectionTermination::Exited,
                exit_code: Some(0),
                stdout_sha256: sha256(&stdout),
                stdout_bytes: stdout.len() as u64,
                stdout,
                stderr_sha256: sha256(&[]),
                stderr_bytes: 0,
                elapsed_ms: 3,
                descendants_terminated: true,
                platform_code: "fixture.git.exited".to_owned(),
            }
        }
    }

    struct Fixture<G = FakeGitExecutor>
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        root: TempRoot,
        request: RuntimeRunRequest,
        definition: ToolDefinition,
        call: ToolCall,
        operation_id: RuntimeOperationId,
        boundary: LinuxCodingRuntimeBoundary<
            'static,
            'static,
            'static,
            TestIdentities,
            FakeCommandExecutor,
            G,
        >,
    }

    fn fixture() -> Fixture {
        fixture_with_git(FakeGitExecutor::default())
    }

    fn fixture_with_git<G>(git_executor: G) -> Fixture<G>
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        fixture_with_git_and_checkpoint_clock(git_executor, None)
    }

    fn fixture_with_git_and_checkpoint_target<G>(
        git_executor: G,
        checkpoint_clock: Arc<AtomicUsize>,
        checkpoint_count: usize,
    ) -> Fixture<G>
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let mut fixture = fixture_with_git_and_checkpoint_clock(git_executor, None);
        fixture.boundary.identities =
            TestIdentities::stop_after_checkpoint_count(0, checkpoint_clock, checkpoint_count);
        fixture
    }

    fn fixture_with_git_and_checkpoint_clock<G>(
        git_executor: G,
        checkpoint_clock: Option<Arc<AtomicUsize>>,
    ) -> Fixture<G>
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let root = temp_root("boundary");
        fixture_with_git_and_checkpoint_clock_at(root, true, 1, git_executor, checkpoint_clock)
    }

    fn fixture_with_git_and_checkpoint_clock_at<G>(
        root: PathBuf,
        initialize: bool,
        now_epoch_ms: u64,
        git_executor: G,
        checkpoint_clock: Option<Arc<AtomicUsize>>,
    ) -> Fixture<G>
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let worktree_root = root.join("worktree");
        let state_root = root.join("state");
        if initialize {
            fs::create_dir_all(worktree_root.join("src")).expect("worktree tree");
            fs::create_dir(&state_root).expect("state root");
            fs::set_permissions(&state_root, fs::Permissions::from_mode(0o700))
                .expect("private state root");
            fs::write(
                worktree_root.join("src/lib.rs"),
                b"pub fn runtime_fixture() {}\n",
            )
            .expect("fixture source");
        }
        let content = fs::read(worktree_root.join("src/lib.rs")).expect("fixture source reads");
        let worktree_sha256 =
            linux_repository_path_sha256(&worktree_root).expect("worktree identity");
        let mut profile_input = input_with_worktree_path_sha256(worktree_sha256.clone());
        let repository_map = build_repository_map(RepositoryMapInput {
            workspace_id: profile_input.write_scope.workspace_id().clone(),
            repository_sha256: "a".repeat(64),
            worktree_sha256,
            branch: Some(profile_input.worktree.branch_ref.clone()),
            commit_id: profile_input.immutable_base_commit.clone(),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![RepositoryFileInput {
                path: vec!["src".to_owned(), "lib.rs".to_owned()],
                size_bytes: content.len() as u64,
                content_sha256: sha256(&content),
                content: Some(content),
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
            }],
        })
        .expect("repository map");
        profile_input.repository_snapshot_sha256 = repository_map.map_sha256.clone();
        profile_input.change_plan =
            crate::coding_plan::fixture_coding_plan_binding(&repository_map);
        let profile = Box::leak(Box::new(
            CodingSessionProfile::build(profile_input).expect("coding profile"),
        ));
        let workspace = Box::leak(Box::new(
            LinuxCodingWorkspace::bind_test(
                profile,
                repository_map,
                &worktree_root,
                WorkspaceAuthorizationId::from_raw("authorization-coding-runtime"),
                agentmage_kernel_contracts::AdapterInstanceId::from_raw("adapter-coding-runtime"),
            )
            .expect("Linux coding workspace"),
        ));
        let actor_id = ActorId::from_raw("actor-coding-runtime");
        let session_id = SessionId::from_raw("session-coding-runtime");
        let task_id = TaskId::from_raw(profile.worktree().task_id.clone());
        let run_id = RuntimeRunId::from_raw("run-coding-runtime");
        let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &actor_id,
            task_id: &task_id,
            run_id: &run_id,
            workspace: workspace.workspace(),
            profile,
            excluded_scopes: Vec::new(),
        })
        .expect("coding runtime policy");
        let request = runtime_request(profile, &policy, run_id, session_id.clone(), task_id);
        let definition = profile
            .registry()
            .get_tool(&ToolId::from_raw(ReadOnlyToolKind::ReadText.id()), "1.0.0")
            .expect("read tool")
            .clone();
        let arguments = serde_json::to_vec(&ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".to_owned(), "lib.rs".to_owned()]],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(128),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        })
        .expect("read request");
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-coding-runtime"),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        let mut key = TestKey([51; 32]);
        let authority =
            open_test_linux_authority(&state_root, &mut key, now_epoch_ms).expect("test authority");
        let sandbox = sandbox();
        let boundary = LinuxCodingRuntimeBoundary::new(LinuxCodingRuntimeBoundaryInput {
            workspace,
            authority,
            sandbox,
            command_executor: FakeCommandExecutor::default(),
            git_executor,
            policy,
            actor_id,
            session_id,
            sensitivity: DataSensitivity::Operational,
            identities: checkpoint_clock.map_or_else(
                || TestIdentities::new(0),
                |clock| TestIdentities::stop_after_checkpoint(0, clock),
            ),
        })
        .expect("coding runtime boundary");
        Fixture {
            root: TempRoot(root),
            request,
            definition,
            call,
            operation_id: RuntimeOperationId::from_raw("operation-coding-runtime"),
            boundary,
        }
    }

    fn runtime_request(
        profile: &CodingSessionProfile,
        policy: &CodingRuntimePolicy,
        run_id: RuntimeRunId,
        session_id: SessionId,
        task_id: TaskId,
    ) -> RuntimeRunRequest {
        seal_runtime_run_request(RuntimeRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id,
            session_id: session_id.clone(),
            mode: RuntimeSessionMode::ControlledWrite,
            task: Task {
                schema_version: CONTRACT_SCHEMA_VERSION,
                task_id: task_id.clone(),
                session_id,
                objective: "Inspect the exact current source".to_owned(),
                acceptance_criteria: vec!["Return grounded read evidence".to_owned()],
                constraints: vec!["No network".to_owned()],
                status: TaskStatus::Ready,
            },
            work_packet: work_packet(profile, task_id),
            workspace_id: profile.write_scope().workspace_id().clone(),
            workspace_snapshot_sha256: profile.worktree().record_sha256.clone(),
            repository_snapshot_id: profile.repository_snapshot_id().clone(),
            repository_snapshot_sha256: profile.repository_snapshot_sha256().to_owned(),
            model_profile: profile.model_profile().clone(),
            context_budget: profile.model_profile().context.clone(),
            tool_catalog_id: profile.tool_catalog_id().clone(),
            tool_catalog_sha256: profile.tool_catalog_sha256().to_owned(),
            visible_tools: profile.visible_tools().to_vec(),
            policy_id: policy.policy_id().clone(),
            policy_sha256: policy.engine().policy_sha256().to_owned(),
            limits: profile.limits().clone(),
            event_cursor: None,
            request_sha256: "0".repeat(64),
        })
        .expect("runtime request")
    }

    fn work_packet(profile: &CodingSessionProfile, task_id: TaskId) -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("packet-coding-runtime"),
            task_id,
            revision: 1,
            objective: "Inspect the exact current source".to_owned(),
            reason: "Prove the production coding authority boundary".to_owned(),
            owner: "fixture-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["src/lib.rs".to_owned()],
            expected_output: "Grounded read evidence".to_owned(),
            acceptance_checks: vec!["Return grounded read evidence".to_owned()],
            required_evidence: vec![EvidenceKind::ToolOutput],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 16,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCallDepth,
                    limit: 2,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 32,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 32,
                },
                BudgetLimit {
                    resource: BudgetResource::InputBytes,
                    limit: 4 * 1024 * 1024,
                },
                BudgetLimit {
                    resource: BudgetResource::OutputBytes,
                    limit: 4 * 1024 * 1024,
                },
                BudgetLimit {
                    resource: BudgetResource::ElapsedMilliseconds,
                    limit: 600_000,
                },
                BudgetLimit {
                    resource: BudgetResource::MemoryBytes,
                    limit: 256 * 1024 * 1024,
                },
                BudgetLimit {
                    resource: BudgetResource::DiskBytes,
                    limit: 64 * 1024 * 1024,
                },
                BudgetLimit {
                    resource: BudgetResource::ProcessCount,
                    limit: 32,
                },
            ],
            stop_conditions: [
                StopConditionKind::AcceptanceSatisfied,
                StopConditionKind::UserDecisionRequired,
                StopConditionKind::PolicyDenied,
                StopConditionKind::Error,
                StopConditionKind::Cancelled,
                StopConditionKind::BudgetExhausted,
                StopConditionKind::UncertainResult,
            ]
            .into_iter()
            .map(|kind| StopCondition {
                kind,
                description: format!("Stop for {kind:?}"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No state change is permitted".to_owned(),
            },
            sensitivity: DataSensitivity::Operational,
            last_verification_date: "2026-08-17".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw(profile.change_plan().plan_id().to_owned())),
            state: WorkPacketState::Active,
        }
    }

    fn challenge<G>(
        fixture: &Fixture<G>,
        evaluation: &RuntimePermissionEvaluation,
    ) -> RuntimeApprovalChallenge
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let RuntimePermissionEvaluation::Ask {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
        } = evaluation
        else {
            panic!("expected ask")
        };
        seal_runtime_approval_challenge(RuntimeApprovalChallenge {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: fixture.request.run_id.clone(),
            task_id: fixture.request.task.task_id.clone(),
            turn_id: RuntimeTurnId::from_raw("turn-coding-runtime"),
            operation_id: fixture.operation_id.clone(),
            tool_call_id: fixture.call.tool_call_id.clone(),
            approval_id: approval_id.clone(),
            proposed_grant_id: grant_id.clone(),
            operation: fixture.definition.required_grant.operation.operation(),
            preview_sha256: preview_sha256.clone(),
            expires_at_epoch_ms: *expires_at_epoch_ms,
            challenge_sha256: "0".repeat(64),
        })
        .expect("approval challenge")
    }

    fn response(
        challenge: &RuntimeApprovalChallenge,
        disposition: RuntimeApprovalDisposition,
    ) -> RuntimeApprovalResponse {
        RuntimeApprovalResponse {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: challenge.run_id.clone(),
            approval_id: challenge.approval_id.clone(),
            disposition,
            challenge_sha256: challenge.challenge_sha256.clone(),
            grant_id: (disposition == RuntimeApprovalDisposition::Allow)
                .then(|| challenge.proposed_grant_id.clone()),
        }
    }

    #[cfg(feature = "workflow-caller")]
    fn workflow_submission(request: &RuntimeRunRequest) -> WorkflowRuntimeSubmission {
        let requested_tool_ids = request
            .visible_tools
            .iter()
            .map(|tool| tool.tool_id.clone())
            .collect::<Vec<_>>();
        assert!(
            requested_tool_ids.windows(2).all(|pair| pair[0] < pair[1]),
            "runtime visible tools must remain canonical"
        );
        let mut targets = vec![
            request.workspace_snapshot_sha256.clone(),
            request.repository_snapshot_sha256.clone(),
        ];
        targets.sort();
        targets.dedup();
        let layers = WorkflowAuthorityLayerKind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| WorkflowAuthorityLayer {
                kind,
                operations: GrantOperation::ALL.to_vec(),
                tool_ids: requested_tool_ids.clone(),
                target_scope_sha256s: targets.clone(),
                source_sha256: format!("{}", index + 1).repeat(64),
            })
            .collect();
        seal_workflow_runtime_submission(WorkflowRuntimeSubmission {
            schema_version: CONTRACT_SCHEMA_VERSION,
            caller: WorkflowCallerIdentity {
                caller_id: "fixture-workflow-adapter".to_owned(),
                workflow_id: "fixture-workflow".to_owned(),
                node_id: "fixture-node".to_owned(),
                parent_invocation_id: "fixture-parent".to_owned(),
            },
            runtime_request: request.clone(),
            work_packet: request.work_packet.clone(),
            requested_tool_ids,
            authority: intersect_workflow_authority(layers).expect("workflow authority"),
            submission_sha256: "0".repeat(64),
        })
        .expect("workflow submission")
    }

    fn configure_git_status<G>(fixture: &mut Fixture<G>)
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let definition = fixture
            .profile_for_test()
            .registry()
            .get_tool(
                &ToolId::from_raw(GIT_INSPECTION_TOOL_ID),
                GIT_INSPECTION_TOOL_VERSION,
            )
            .expect("Git inspection tool")
            .clone();
        let arguments = serde_json::to_vec(&GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Status,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 32,
            max_output_bytes: 4_096,
        })
        .expect("Git inspection request");
        fixture.call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-git-runtime"),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&fixture.request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        fixture.definition = definition;
    }

    fn configure_runtime_call<G, T>(
        fixture: &mut Fixture<G>,
        tool_id: &str,
        call_id: &str,
        value: &T,
    ) where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
        T: serde::Serialize,
    {
        let definition = fixture
            .profile_for_test()
            .registry()
            .get_tool(&ToolId::from_raw(tool_id), CONTROLLED_CHANGE_TOOL_VERSION)
            .expect("controlled-change tool")
            .clone();
        let arguments = serde_json::to_vec(value).expect("controlled-change request");
        fixture.call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(call_id),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&fixture.request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        fixture.definition = definition;
    }

    fn configure_structured_patch<G>(fixture: &mut Fixture<G>)
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let source = fs::read(fixture.root.join("worktree/src/lib.rs")).expect("source preimage");
        let intent_sha256 = fixture
            .profile_for_test()
            .change_plan()
            .intent_sha256()
            .to_owned();
        let change_plan_sha256 = fixture
            .profile_for_test()
            .change_plan()
            .plan_sha256()
            .to_owned();
        configure_runtime_call(
            fixture,
            STRUCTURED_PATCH_TOOL_ID,
            "call-patch-runtime",
            &StructuredPatchProposal {
                schema_version: 1,
                change_id: "change-patch-runtime".to_owned(),
                path: vec!["src".to_owned(), "lib.rs".to_owned()],
                expected_preimage_sha256: sha256(&source),
                intent_sha256,
                change_plan_sha256,
                language: StructuredLanguage::Rust,
                artifact_class: StructuredArtifactClass::Code,
                edits: vec![StructuredEdit::RenameIdentifier {
                    edit_id: "edit-patch-runtime".to_owned(),
                    old: "runtime_fixture".to_owned(),
                    replacement: "runtime_updated".to_owned(),
                }],
                additional_review_hooks: Vec::new(),
                generated: false,
                allow_generated: false,
            },
        );
    }

    fn configure_controlled_create<G>(fixture: &mut Fixture<G>)
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        configure_controlled_create_with(
            fixture,
            "new.rs",
            "pub fn newly_created() {}\n".to_owned(),
            ControlledFileClassification::SourceCode,
        );
    }

    fn configure_controlled_create_with<G>(
        fixture: &mut Fixture<G>,
        file_name: &str,
        content: String,
        classification: ControlledFileClassification,
    ) where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let intent_sha256 = fixture
            .profile_for_test()
            .change_plan()
            .intent_sha256()
            .to_owned();
        let change_plan_sha256 = fixture
            .profile_for_test()
            .change_plan()
            .plan_sha256()
            .to_owned();
        let workspace = fixture.boundary.workspace.workspace();
        let parent_path = WorkspacePath::new(workspace.workspace_id().clone(), ["src"])
            .expect("source parent path");
        let held_parent = resolve_test_linux_workspace_object(
            workspace,
            workspace.adapter_instance_id().clone(),
            &parent_path,
            PathResolutionIntent::ReadDirectory,
        )
        .expect("source parent");
        let parent = GrantTarget::held_object(&held_parent).expect("source parent target");
        let siblings = held_parent
            .observe_directory_names(4_096, 1024 * 1024)
            .expect("source sibling projection");
        let expected_parent_sha256 =
            controlled_create_parent_observation_sha256(&parent, &siblings)
                .expect("source parent observation");
        configure_runtime_call(
            fixture,
            CONTROLLED_CREATE_TOOL_ID,
            "call-create-runtime",
            &ControlledFileCreationProposal {
                schema_version: 1,
                creation_id: "creation-runtime".to_owned(),
                path: vec!["src".to_owned(), file_name.to_owned()],
                content,
                mode: 0o644,
                classification,
                intent_sha256,
                change_plan_sha256,
                expected_parent_sha256,
            },
        );
    }

    fn configure_targeted_validation<G>(fixture: &mut Fixture<G>)
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let template = fixture
            .profile_for_test()
            .validations()
            .templates
            .first()
            .expect("validation template")
            .clone();
        let definition = fixture
            .profile_for_test()
            .registry()
            .get_tool(
                &ToolId::from_raw(crate::coding_tools::TARGETED_VALIDATION_TOOL_ID),
                crate::coding_tools::TARGETED_VALIDATION_TOOL_VERSION,
            )
            .expect("validation tool")
            .clone();
        let arguments = serde_json::to_vec(&TargetedValidationRequest {
            schema_version: 1,
            validation_attempt_id: "validation-attempt-e2e".to_owned(),
            validation_id: template.validation_id,
            template_sha256: template.template_sha256,
        })
        .expect("validation request");
        fixture.call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-validation-e2e"),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&fixture.request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        fixture.definition = definition;
        fixture
            .boundary
            .command_executor
            .as_mut()
            .expect("test command executor")
            .stdout = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "status": "passed",
            "passed": 1,
            "failed": 0,
            "skipped": 0,
            "duration_ms": 1,
            "failed_names": [],
            "artifact_ids": [],
            "retry_count": 0,
            "initial_failure_sha256": null
        }))
        .expect("validation output");
    }

    fn configure_bounded_command<G>(fixture: &mut Fixture<G>)
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let command = fixture
            .profile_for_test()
            .commands()
            .commands()
            .into_iter()
            .next()
            .expect("registered command");
        let definition = fixture
            .profile_for_test()
            .registry()
            .get_tool(
                &ToolId::from_raw(crate::coding_tools::BOUNDED_COMMAND_TOOL_ID),
                crate::coding_tools::BOUNDED_COMMAND_TOOL_VERSION,
            )
            .expect("command tool")
            .clone();
        let arguments = serde_json::to_vec(&CommandRequest::new("command-attempt-e2e", command))
            .expect("command request");
        fixture.call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-command-e2e"),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&fixture.request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        fixture.definition = definition;
    }

    fn scripted_call(call: &ToolCall) -> ModelToolCallCandidate {
        ModelToolCallCandidate {
            tool_call_id: call.tool_call_id.clone(),
            tool_id: call.tool_id.clone(),
            tool_version: call.tool_version.clone(),
            arguments: call.arguments.clone(),
        }
    }

    fn approve<G>(fixture: &mut Fixture<G>, now_epoch_ms: u64) -> RuntimePermissionEvaluation
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                now_epoch_ms,
            )
            .expect("approval preview");
        let challenge = challenge(fixture, &evaluation);
        fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                now_epoch_ms + 1,
            )
            .expect("exact approval")
    }

    #[test]
    fn story_48_2_fake_model_completes_real_git_patch_test_git_and_verify_path() {
        let mut fixture = fixture();
        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-before-e2e");
        let git_before = scripted_call(&fixture.call);

        configure_structured_patch(&mut fixture);
        let patch = scripted_call(&fixture.call);

        configure_targeted_validation(&mut fixture);
        let validation = scripted_call(&fixture.call);

        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-after-e2e");
        let git_after = scripted_call(&fixture.call);

        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Validation];
        fixture.request = seal_runtime_run_request(fixture.request.clone())
            .expect("updated evidence requirement");

        let profile = fixture.profile_for_test();
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::Changed,
            summary: "Updated the bounded fixture and verified the registered test.".to_owned(),
            checks_not_run: Vec::new(),
            residual_risks: Vec::new(),
        })
        .expect("completion payload");
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(git_before),
                ScriptedCodingStep::Tool(patch),
                ScriptedCodingStep::Tool(validation),
                ScriptedCodingStep::Tool(git_after),
                ScriptedCodingStep::Complete(completion),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("coding context");
        let Fixture {
            root,
            request,
            boundary,
            ..
        } = fixture;
        let admitted_request = request.clone();
        let mut coordinator = compose_ephemeral_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(20_000),
        )
        .expect("ephemeral coding coordinator");

        let mut next_response = None;
        let outcome = loop {
            match coordinator
                .run_until_boundary(next_response.as_ref(), None)
                .expect("coding coordinator boundary")
            {
                RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                    next_response = Some(response(&challenge, RuntimeApprovalDisposition::Allow));
                }
                RuntimeCoordinatorStep::Complete { outcome } => break outcome,
            }
        };

        assert_eq!(outcome.state, AgentStateKind::Success, "{outcome:#?}");
        assert_eq!(outcome.model_call_count, 5);
        assert_eq!(outcome.tool_call_count, 4);
        assert_eq!(outcome.receipt_ids.len(), 4);
        assert!(outcome.unresolved_codes.is_empty());
        assert_eq!(
            fs::read(root.join("worktree/src/lib.rs")).expect("updated source"),
            b"pub fn runtime_updated() {}\n"
        );
        verify_runtime_outcome(&outcome, &admitted_request).expect("verified runtime outcome");
        let mut sequence = RuntimeEventSequence::new();
        for event in coordinator.events() {
            sequence.push(event).expect("ordered runtime event");
        }
        assert!(sequence.is_terminal());
    }

    #[test]
    fn s_048_mvp_e2e_controlled_create_test_git_and_verify_path() {
        let mut fixture = fixture();
        configure_controlled_create(&mut fixture);
        let create = scripted_call(&fixture.call);

        configure_targeted_validation(&mut fixture);
        let validation = scripted_call(&fixture.call);

        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-after-create-e2e");
        let git_after = scripted_call(&fixture.call);

        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Validation];
        fixture.request =
            seal_runtime_run_request(fixture.request.clone()).expect("validation requirement");
        let profile = fixture.profile_for_test();
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::Changed,
            summary: "Created one planned file and verified the registered check.".to_owned(),
            checks_not_run: Vec::new(),
            residual_risks: Vec::new(),
        })
        .expect("completion payload");
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(create),
                ScriptedCodingStep::Tool(validation),
                ScriptedCodingStep::Tool(git_after),
                ScriptedCodingStep::Complete(completion),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("coding context");
        let Fixture {
            root,
            request,
            boundary,
            ..
        } = fixture;
        let mut coordinator = compose_ephemeral_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(30_000),
        )
        .expect("ephemeral coding coordinator");

        let mut next_response = None;
        let outcome = loop {
            match coordinator
                .run_until_boundary(next_response.as_ref(), None)
                .expect("coding coordinator boundary")
            {
                RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                    next_response = Some(response(&challenge, RuntimeApprovalDisposition::Allow));
                }
                RuntimeCoordinatorStep::Complete { outcome } => break outcome,
            }
        };

        assert_eq!(outcome.state, AgentStateKind::Success, "{outcome:#?}");
        assert_eq!(outcome.tool_call_count, 3);
        assert_eq!(
            fs::read(root.join("worktree/src/new.rs")).expect("created source"),
            b"pub fn newly_created() {}\n"
        );
    }

    #[test]
    fn s_048_mvp_e2e_repository_exploration_finishes_as_verified_no_op() {
        let mut fixture = fixture_with_git(FakeGitExecutor::clean());
        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-clean-e2e");
        let clean_git = scripted_call(&fixture.call);
        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Observation];
        fixture.request =
            seal_runtime_run_request(fixture.request.clone()).expect("observation requirement");

        let profile = fixture.profile_for_test();
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::NoOp,
            summary: "Inspected the requested source; no change was required.".to_owned(),
            checks_not_run: vec!["No mutation-dependent validation was needed.".to_owned()],
            residual_risks: Vec::new(),
        })
        .expect("completion payload");
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(clean_git),
                ScriptedCodingStep::Complete(completion),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("coding context");
        let Fixture {
            request, boundary, ..
        } = fixture;
        #[cfg(feature = "interactive-cli")]
        let render_request = request.clone();
        let mut coordinator = compose_ephemeral_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(40_000),
        )
        .expect("ephemeral coding coordinator");

        let mut approvals = AllowApproval;
        let mut sink = CollectingEventSink::default();
        let client_result = drive_coding_client(&mut coordinator, &mut approvals, &mut sink, None)
            .expect("interactive coding client");
        let outcome = client_result.outcome;

        assert_eq!(outcome.state, AgentStateKind::NoOp, "{outcome:#?}");
        assert_eq!(client_result.presented_events as usize, sink.0.len());
        assert_eq!(sink.0, coordinator.events());
        #[cfg(feature = "interactive-cli")]
        {
            for event in &sink.0 {
                render_runtime_event_human(event).expect("human runtime event");
                render_runtime_event_json(event).expect("JSON runtime event");
            }
            render_runtime_outcome_human(&render_request, &outcome).expect("human runtime outcome");
            render_runtime_outcome_json(&render_request, &outcome).expect("JSON runtime outcome");
        }
        assert_eq!(outcome.tool_call_count, 1);
        assert!(outcome.unresolved_codes.is_empty());
    }

    #[cfg(feature = "workflow-caller")]
    #[test]
    fn story_50_2_and_sprint_95_child_caller_use_the_real_no_op_runtime_path() {
        let mut fixture = fixture_with_git(FakeGitExecutor::clean());
        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-workflow-e2e");
        let clean_git = scripted_call(&fixture.call);
        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Observation];
        fixture.request =
            seal_runtime_run_request(fixture.request.clone()).expect("observation requirement");

        let profile = fixture.profile_for_test();
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::NoOp,
            summary: "Inspected the requested source; no change was required.".to_owned(),
            checks_not_run: vec!["No mutation-dependent validation was needed.".to_owned()],
            residual_risks: Vec::new(),
        })
        .expect("completion payload");
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(clean_git),
                ScriptedCodingStep::Complete(completion),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("coding context");
        let Fixture {
            request, boundary, ..
        } = fixture;
        let mut submission = workflow_submission(&request);
        submission.caller.node_id = "fixture-child".to_owned();
        submission.submission_sha256 = "0".repeat(64);
        let submission = seal_workflow_runtime_submission(submission).expect("child submission");
        let assignment = seal_child_runtime_assignment(ChildRuntimeAssignment {
            assignment_id: "fixture-child-assignment".to_owned(),
            parent_invocation_id: submission.caller.parent_invocation_id.clone(),
            child_id: submission.caller.node_id.clone(),
            dependency_ids: Vec::new(),
            nesting_depth: 1,
            submission,
            parent_review_required: true,
            assignment_sha256: "0".repeat(64),
        })
        .expect("child assignment");
        let coordinator = compose_ephemeral_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(41_000),
        )
        .expect("ephemeral coding coordinator");
        let mut caller =
            ChildRuntimeCaller::submit(coordinator, assignment).expect("child runtime caller");

        let waiting = caller.advance(None).expect("workflow approval boundary");
        assert_eq!(waiting.state, WorkflowCallerState::WaitingForUser);
        let challenge = waiting.approval.expect("protected challenge");
        let step = caller
            .resume_after_user_decision(
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                None,
            )
            .expect("workflow terminal boundary");
        assert_eq!(step.state, WorkflowCallerState::Terminal);
        let proposal = caller
            .terminal_proposal(step.clone())
            .expect("untrusted child proposal");
        assert_eq!(proposal.review, ChildReviewDisposition::Pending);
        assert!(!proposal.authority_from_result);
        assert_eq!(
            step.outcome.expect("terminal outcome").state,
            AgentStateKind::NoOp
        );
        assert!(!step.events.is_empty());
        assert!(!step.evidence.is_empty());
        assert_eq!(
            caller.history(),
            [
                WorkflowCallerState::Submitted,
                WorkflowCallerState::Acknowledged,
                WorkflowCallerState::Streaming,
                WorkflowCallerState::WaitingForUser,
                WorkflowCallerState::Resumed,
                WorkflowCallerState::Streaming,
                WorkflowCallerState::Terminal,
            ]
        );
    }

    #[cfg(feature = "workflow-caller")]
    #[test]
    fn story_50_2_workflow_caller_waits_for_user_and_denial_starts_no_effect() {
        let mut fixture = fixture();
        configure_structured_patch(&mut fixture);
        let patch = scripted_call(&fixture.call);
        let original = fs::read(fixture.root.join("worktree/src/lib.rs")).expect("source");
        let profile = fixture.profile_for_test();
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [ScriptedCodingStep::Tool(patch)].into_iter().collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("coding context");
        let Fixture {
            root,
            request,
            boundary,
            ..
        } = fixture;
        let submission = workflow_submission(&request);
        let coordinator = compose_ephemeral_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(42_000),
        )
        .expect("ephemeral coding coordinator");
        let mut caller =
            InMemoryWorkflowCaller::submit(coordinator, submission).expect("workflow caller");

        let waiting = caller.advance(None).expect("approval boundary");
        assert_eq!(waiting.state, WorkflowCallerState::WaitingForUser);
        let challenge = waiting.approval.expect("protected challenge");
        let mut stale_response = response(&challenge, RuntimeApprovalDisposition::Allow);
        stale_response.approval_id = ApprovalId::from_raw("stale-workflow-approval");
        assert_eq!(
            caller.resume_after_user_decision(&stale_response, None),
            Err(WorkflowCallerError::TransitionDenied)
        );
        let denied = caller
            .resume_after_user_decision(
                &response(&challenge, RuntimeApprovalDisposition::Deny),
                None,
            )
            .expect("denial terminal boundary");
        assert_eq!(denied.state, WorkflowCallerState::Terminal);
        assert_eq!(
            denied.outcome.expect("terminal outcome").state,
            AgentStateKind::Declined
        );
        assert_eq!(
            fs::read(root.join("worktree/src/lib.rs")).expect("preserved source"),
            original
        );
        assert_eq!(
            caller.history(),
            [
                WorkflowCallerState::Submitted,
                WorkflowCallerState::Acknowledged,
                WorkflowCallerState::Streaming,
                WorkflowCallerState::WaitingForUser,
                WorkflowCallerState::Resumed,
                WorkflowCallerState::Streaming,
                WorkflowCallerState::Terminal,
            ]
        );
    }

    #[test]
    fn durable_coding_run_persists_continuation_artifact_and_checkpoint() {
        let mut fixture = fixture_with_git(FakeGitExecutor::clean());
        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-durable-e2e");
        let clean_git = scripted_call(&fixture.call);
        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Observation];
        fixture.request =
            seal_runtime_run_request(fixture.request.clone()).expect("observation requirement");

        let profile = fixture.profile_for_test();
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::NoOp,
            summary: "Inspected the requested source; no change was required.".to_owned(),
            checks_not_run: vec!["No mutation-dependent validation was needed.".to_owned()],
            residual_risks: Vec::new(),
        })
        .expect("completion payload");
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(clean_git),
                ScriptedCodingStep::Complete(completion),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("coding context");
        let Fixture {
            request, boundary, ..
        } = fixture;
        let mut coordinator = compose_durable_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(45_000),
        )
        .expect("durable coding coordinator");

        let mut next_response = None;
        let outcome = loop {
            match coordinator
                .run_until_boundary(next_response.as_ref(), None)
                .expect("coding coordinator boundary")
            {
                RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                    next_response = Some(response(&challenge, RuntimeApprovalDisposition::Allow));
                }
                RuntimeCoordinatorStep::Complete { outcome } => break outcome,
            }
        };

        assert_eq!(outcome.state, AgentStateKind::NoOp, "{outcome:#?}");
        assert!(
            coordinator
                .artifact_references()
                .iter()
                .any(|reference| { reference.media_type == RUNTIME_CONTINUATION_MEDIA_TYPE })
        );
        assert!(
            coordinator.events().iter().any(|event| {
                matches!(event.kind, RuntimeEventKind::CheckpointCommitted { .. })
            })
        );
        let event_position = |predicate: fn(&RuntimeEventKind) -> bool| {
            coordinator
                .events()
                .iter()
                .position(|event| predicate(&event.kind))
                .expect("required durable correctness event")
        };
        let permission_requested =
            event_position(|kind| matches!(kind, RuntimeEventKind::PermissionRequested { .. }));
        let permission_decided =
            event_position(|kind| matches!(kind, RuntimeEventKind::PermissionDecided { .. }));
        let tool_started =
            event_position(|kind| matches!(kind, RuntimeEventKind::ToolStarted { .. }));
        let tool_completed =
            event_position(|kind| matches!(kind, RuntimeEventKind::ToolCompleted { .. }));
        let checkpoint_committed =
            event_position(|kind| matches!(kind, RuntimeEventKind::CheckpointCommitted { .. }));
        assert!(permission_requested < permission_decided);
        assert!(permission_decided < tool_started);
        assert!(tool_started < tool_completed);
        assert!(tool_completed < checkpoint_committed);
        assert!(
            coordinator.events()[tool_started].occurred_at_epoch_ms
                < coordinator.events()[tool_completed].occurred_at_epoch_ms
        );
        let mut sequence = RuntimeEventSequence::new();
        for event in coordinator.events() {
            sequence.push(event).expect("ordered durable runtime event");
        }
        assert!(sequence.is_terminal());
    }

    #[test]
    #[ignore = "subprocess stop target; invoked only by the Story 22.1 native resume matrix"]
    fn story_22_1_native_tool_terminal_resume_child() {
        let root = PathBuf::from(
            std::env::var_os(STORY_22_1_CRASH_ROOT_ENVIRONMENT).expect("Story 22.1 child root"),
        );
        let launch_record = root.join("worker-launches");
        let mut fixture = fixture_with_git_and_checkpoint_clock_at(
            root,
            true,
            1,
            FakeGitExecutor::clean_with_durable_counter(launch_record),
            None,
        );
        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-native-resume-22-1");
        let clean_git = scripted_call(&fixture.call);
        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Observation];
        fixture.request = seal_runtime_run_request(fixture.request.clone())
            .expect("native resume evidence requirement");
        let profile = fixture.profile_for_test();
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [ScriptedCodingStep::Tool(clean_git)].into_iter().collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("native resume context");
        let Fixture {
            request, boundary, ..
        } = fixture;
        let mut coordinator = compose_durable_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(75_000),
        )
        .expect("native resume child coordinator");
        let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
            .run_until_boundary(None, None)
            .expect("native inspection reaches approval")
        else {
            panic!("native inspection must require approval");
        };
        let _ = coordinator.run_until_boundary(
            Some(&response(&challenge, RuntimeApprovalDisposition::Allow)),
            None,
        );
        panic!("configured no-unwind crash boundary was not reached");
    }

    #[test]
    #[ignore = "explicit Story 22.1 native one-hundred-resume campaign"]
    fn story_22_1_native_tool_terminal_resume_matrix_never_replays_or_invents_state() {
        let executable = std::env::current_exe().expect("current test executable");
        let mut total_cases = 0;
        for boundary in STORY_22_1_CRASH_BOUNDARIES {
            for repetition in 0..STORY_22_1_REPETITIONS_PER_BOUNDARY {
                let root = temp_root(&format!("story-22-1-{boundary}-{repetition}"));
                let launch_record = root.join("worker-launches");
                let output = Command::new(&executable)
                    .arg(
                        "linux_coding_runtime::tests::story_22_1_native_tool_terminal_resume_child",
                    )
                    .arg("--exact")
                    .arg("--ignored")
                    .arg("--nocapture")
                    .arg("--test-threads=1")
                    .env(STORY_22_1_CRASH_ROOT_ENVIRONMENT, &root)
                    .env(STORY_22_1_CRASH_ENVIRONMENT, boundary)
                    .output()
                    .expect("native resume crash child launches");
                assert_eq!(
                    output.status.code(),
                    Some(STORY_22_1_CRASH_CHILD_EXIT),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    fs::read_to_string(&launch_record).expect("worker launch count reads"),
                    "1"
                );

                let fixture = fixture_with_git_and_checkpoint_clock_at(
                    root,
                    false,
                    90_000,
                    FakeGitExecutor::clean_with_durable_counter(launch_record.clone()),
                    None,
                );
                let authority = fixture.boundary.authority.authority();
                assert_eq!(authority.receipts().len(), 1);
                assert_eq!(authority.receipts()[0].outcome, OperationOutcome::Succeeded);
                let events = authority
                    .runtime_events(&fixture.request.run_id)
                    .expect("native resume events read");
                let terminal_count = events
                    .iter()
                    .filter(|event| matches!(event.kind, RuntimeEventKind::ToolCompleted { .. }))
                    .count();
                let checkpoint_count = events
                    .iter()
                    .filter(|event| {
                        matches!(event.kind, RuntimeEventKind::CheckpointCommitted { .. })
                    })
                    .count();
                assert_eq!(
                    terminal_count,
                    usize::from(boundary != "before-tool-terminal-commit")
                );
                assert_eq!(
                    checkpoint_count,
                    usize::from(boundary == "after-checkpoint-commit")
                );
                assert_eq!(
                    authority
                        .current_session_checkpoint()
                        .expect("checkpoint state reads")
                        .is_some(),
                    boundary == "after-checkpoint-commit"
                );
                assert_eq!(
                    fs::read_to_string(&launch_record).expect("launch count remains readable"),
                    "1"
                );
                total_cases += 1;
            }
        }
        assert_eq!(total_cases, 100);
        println!(
            "{STORY_22_1_RESUME_METRIC_PREFIX}{}",
            serde_json::json!({
                "boundaries": STORY_22_1_CRASH_BOUNDARIES,
                "case_count": total_cases,
                "checkpoint_after_commit_cases": STORY_22_1_REPETITIONS_PER_BOUNDARY,
                "external_network_used": false,
                "false_checkpoint_count": 0,
                "false_terminal_count": 0,
                "manual_fuzzing_executed": false,
                "repetitions_per_boundary": STORY_22_1_REPETITIONS_PER_BOUNDARY,
                "total_worker_launches_per_case": 1
            })
        );
    }

    #[test]
    fn story_22_1_long_native_session_blocks_drift_and_preserves_exact_history() {
        const TOOL_TURNS: usize = 7;
        let checkpoint_clock = Arc::new(AtomicUsize::new(CHECKPOINT_CLOCK_UNARMED));
        let shared_launches = Arc::new(AtomicUsize::new(0));
        let mut fixture = fixture_with_git_and_checkpoint_target(
            FakeGitExecutor::clean_with_counter(Arc::clone(&shared_launches)),
            Arc::clone(&checkpoint_clock),
            TOOL_TURNS,
        );
        configure_git_status(&mut fixture);
        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Observation];
        fixture.request =
            seal_runtime_run_request(fixture.request.clone()).expect("long-session requirement");
        let profile = fixture.profile_for_test();
        let workspace = fixture.boundary.workspace;
        let state_root = fixture.root.join("state");
        let base_candidate = scripted_call(&fixture.call);
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::NoOp,
            summary: "Resumed the complete long-session history without replay.".to_owned(),
            checks_not_run: vec!["No mutation-dependent validation was needed.".to_owned()],
            residual_risks: Vec::new(),
        })
        .expect("long-session completion payload");
        let mut steps = (0..TOOL_TURNS)
            .map(|index| {
                let mut candidate = base_candidate.clone();
                candidate.tool_call_id =
                    ToolCallId::from_raw(format!("call-git-long-session-{index:02}"));
                let arguments = serde_json::to_vec(&GitInspectionRequest {
                    schema_version: 1,
                    operation: GitInspectionOperation::Status,
                    revision: None,
                    object_id: None,
                    pathspecs: Vec::new(),
                    max_records: 32 + index as u32,
                    max_output_bytes: 4_096,
                })
                .expect("long-session Git request");
                candidate.arguments.sha256 = sha256(&arguments);
                candidate.arguments.bytes = arguments;
                ScriptedCodingStep::Tool(candidate)
            })
            .collect::<VecDeque<_>>();
        steps.push_back(ScriptedCodingStep::Complete(completion.clone()));
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps,
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("long-session context");
        let Fixture {
            root: _root,
            request,
            boundary,
            ..
        } = fixture;
        let base_request = request.clone();
        let mut coordinator = compose_durable_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            StopAfterCheckpointClock {
                now_epoch_ms: 95_000,
                checkpoint_clock: Arc::clone(&checkpoint_clock),
            },
        )
        .expect("long-session coordinator");
        let mut next_response = None;
        loop {
            match coordinator.run_until_boundary(next_response.as_ref(), None) {
                Ok(RuntimeCoordinatorStep::AwaitingApproval { challenge }) => {
                    next_response = Some(response(&challenge, RuntimeApprovalDisposition::Allow));
                }
                Err(RuntimeLoopError::Dependency(RuntimePortFailure::Unavailable)) => break,
                Ok(RuntimeCoordinatorStep::Complete { outcome }) => {
                    panic!(
                        "long session must stop at the selected checkpoint: state={:?}, clock={}, events={}",
                        outcome.state,
                        checkpoint_clock.load(Ordering::SeqCst),
                        coordinator.events().len()
                    )
                }
                Err(error) => panic!("unexpected long-session boundary: {error:?}"),
            }
        }
        assert_eq!(checkpoint_clock.load(Ordering::SeqCst), 0);
        assert_eq!(shared_launches.load(Ordering::SeqCst), TOOL_TURNS);
        assert_eq!(
            coordinator
                .events()
                .iter()
                .filter(|event| matches!(event.kind, RuntimeEventKind::ToolCompleted { .. }))
                .count(),
            TOOL_TURNS
        );
        assert_eq!(
            coordinator
                .events()
                .iter()
                .filter(|event| matches!(event.kind, RuntimeEventKind::CheckpointCommitted { .. }))
                .count(),
            TOOL_TURNS
        );
        let checkpoint_event = coordinator.events().last().expect("final checkpoint event");
        let requested_cursor = RuntimeEventCursor {
            run_id: checkpoint_event.run_id.clone(),
            event_id: checkpoint_event.event_id.clone(),
            sequence: checkpoint_event.sequence,
            event_sha256: checkpoint_event.event_sha256.clone(),
        };
        let pre_restart_artifacts = coordinator.artifact_references().to_vec();
        let mut expected_binding_artifacts = pre_restart_artifacts.clone();
        expected_binding_artifacts
            .sort_by(|left, right| left.artifact_id.as_str().cmp(right.artifact_id.as_str()));
        drop(coordinator);

        let actor_id = ActorId::from_raw("actor-coding-runtime");
        let task_id = base_request.task.task_id.clone();
        let run_id = base_request.run_id.clone();
        let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &actor_id,
            task_id: &task_id,
            run_id: &run_id,
            workspace: workspace.workspace(),
            profile,
            excluded_scopes: Vec::new(),
        })
        .expect("long-session policy rebuilds exactly");
        let mut key = TestKey([51; 32]);
        let authority =
            open_test_linux_authority(&state_root, &mut key, 96_000).expect("authority reopens");
        let mut resumed_boundary =
            LinuxCodingRuntimeBoundary::new(LinuxCodingRuntimeBoundaryInput {
                workspace,
                authority,
                sandbox: sandbox(),
                command_executor: FakeCommandExecutor::default(),
                git_executor: FakeGitExecutor::clean_with_counter(Arc::clone(&shared_launches)),
                policy,
                actor_id,
                session_id: base_request.session_id.clone(),
                sensitivity: DataSensitivity::Operational,
                identities: TestIdentities::new(30_000),
            })
            .expect("long-session resumed boundary");
        let mut resumed_request = base_request.clone();
        resumed_request.event_cursor = Some(requested_cursor.clone());
        resumed_request =
            seal_runtime_run_request(resumed_request).expect("long-session request reseals");

        let mut drift_cases = Vec::new();
        let mut repository = resumed_request.clone();
        repository.repository_snapshot_sha256 = "f".repeat(64);
        drift_cases.push(("repository", repository));
        let mut policy = resumed_request.clone();
        policy.policy_sha256 = "f".repeat(64);
        drift_cases.push(("policy", policy));
        let mut model_profile = resumed_request.clone();
        model_profile.model_profile.profile_id =
            agentmage_kernel_contracts::ModelProfileId::from_raw("profile-drifted");
        drift_cases.push(("model-profile", model_profile));
        let mut model_manifest = resumed_request.clone();
        model_manifest.model_profile.manifest_sha256 = "f".repeat(64);
        drift_cases.push(("model-manifest", model_manifest));
        let mut model_runtime = resumed_request.clone();
        model_runtime.model_profile.runtime.runtime_sha256 = "f".repeat(64);
        drift_cases.push(("model-runtime", model_runtime));
        let mut workspace_state = resumed_request.clone();
        workspace_state.workspace_snapshot_sha256 = "f".repeat(64);
        drift_cases.push(("workspace", workspace_state));
        let mut configuration = resumed_request.clone();
        configuration.tool_catalog_sha256 = "f".repeat(64);
        drift_cases.push(("configuration", configuration));
        for (name, mut drifted) in drift_cases {
            drifted.request_sha256 = "0".repeat(64);
            match seal_runtime_run_request(drifted) {
                Ok(drifted) => assert!(
                    matches!(
                        RuntimeCheckpointPort::load_runtime_checkpoint(
                            &mut resumed_boundary,
                            &drifted,
                        ),
                        Err(RuntimePortFailure::Invalid)
                    ),
                    "{name} drift must block before another action"
                ),
                Err(_) => assert_eq!(
                    name, "configuration",
                    "only inconsistent configuration must fail while sealing"
                ),
            }
        }
        let snapshot =
            RuntimeCheckpointPort::load_runtime_checkpoint(&mut resumed_boundary, &resumed_request)
                .expect("exact long-session snapshot loads")
                .expect("exact long-session snapshot exists");
        assert_eq!(snapshot.binding.artifacts, expected_binding_artifacts);
        assert_eq!(
            snapshot.binding.event_cursor.run_id,
            requested_cursor.run_id
        );
        assert_eq!(
            snapshot.binding.event_cursor.sequence + 1,
            requested_cursor.sequence
        );

        let resumed_model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [ScriptedCodingStep::Complete(completion)]
                .into_iter()
                .collect(),
            calls: 0,
        };
        let resumed_context =
            CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
                .expect("long-session resumed context");
        let mut resumed = compose_durable_coding_coordinator(
            profile,
            resumed_request,
            resumed_model,
            resumed_context,
            resumed_boundary,
            FixtureClock(97_000),
        )
        .expect("long-session coordinator restores");
        let RuntimeCoordinatorStep::Complete { outcome } = resumed
            .run_until_boundary(None, None)
            .expect("long-session resume completes")
        else {
            panic!("long-session completion cannot request approval");
        };
        assert_eq!(outcome.state, AgentStateKind::NoOp);
        assert_eq!(outcome.tool_call_count, TOOL_TURNS as u32);
        assert_eq!(outcome.receipt_ids.len(), TOOL_TURNS);
        assert_eq!(shared_launches.load(Ordering::SeqCst), TOOL_TURNS);
        assert_eq!(resumed.artifact_references(), expected_binding_artifacts);
        assert_eq!(
            base_request.task.objective,
            "Inspect the exact current source"
        );
        println!(
            "{STORY_22_1_LONG_RESUME_METRIC_PREFIX}{}",
            serde_json::json!({
                "blocked_drift_classes": [
                    "configuration",
                    "model-manifest",
                    "model-profile",
                    "model-runtime",
                    "policy",
                    "repository",
                    "workspace"
                ],
                "checkpoint_count": TOOL_TURNS,
                "exact_artifact_set_restored": true,
                "external_network_used": false,
                "intent_preserved": true,
                "manual_fuzzing_executed": false,
                "post_resume_total_worker_launches": TOOL_TURNS,
                "receipt_count": TOOL_TURNS,
                "tool_turn_count": TOOL_TURNS
            })
        );
    }

    #[test]
    fn story_22_2_linux_generated_file_is_published_and_checkpoint_bound() {
        let checkpoint_clock = Arc::new(AtomicUsize::new(CHECKPOINT_CLOCK_UNARMED));
        let mut fixture = fixture_with_git_and_checkpoint_clock(
            FakeGitExecutor::default(),
            Some(Arc::clone(&checkpoint_clock)),
        );
        let generated_content = "generated-line\n".repeat(5_200);
        assert!(generated_content.len() > 64 * 1024);
        configure_controlled_create_with(
            &mut fixture,
            "generated.txt",
            generated_content.clone(),
            ControlledFileClassification::Generated,
        );
        let create = scripted_call(&fixture.call);
        let profile = fixture.profile_for_test();
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [ScriptedCodingStep::Tool(create)].into_iter().collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("generated-file context");
        let state_root = fixture.root.join("state");
        let Fixture {
            root,
            request,
            boundary,
            ..
        } = fixture;
        let mut coordinator = compose_durable_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            StopAfterCheckpointClock {
                now_epoch_ms: 45_000,
                checkpoint_clock: Arc::clone(&checkpoint_clock),
            },
        )
        .expect("generated-file coordinator");

        let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
            .run_until_boundary(None, None)
            .expect("generated-file creation reaches approval")
        else {
            panic!("generated-file creation must require an explicit decision");
        };
        assert!(matches!(
            coordinator.run_until_boundary(
                Some(&response(&challenge, RuntimeApprovalDisposition::Allow)),
                None,
            ),
            Err(RuntimeLoopError::Dependency(
                RuntimePortFailure::Unavailable
            ))
        ));
        let generated_sha256 = sha256(generated_content.as_bytes());
        let generated_reference = coordinator
            .artifact_references()
            .iter()
            .find(|reference| reference.payload_sha256 == generated_sha256)
            .cloned()
            .expect("large generated file publishes as an artifact");
        assert_eq!(
            generated_reference.byte_size,
            generated_content.len() as u64
        );
        assert_eq!(generated_reference.media_type, "text/plain");
        assert!(matches!(
            coordinator.events().last().map(|event| &event.kind),
            Some(RuntimeEventKind::CheckpointCommitted { .. })
        ));
        drop(coordinator);

        let mut key = TestKey([51; 32]);
        let authority =
            open_test_linux_authority(&state_root, &mut key, 46_000).expect("authority reopens");
        let operator_view = authority
            .runtime_artifact_operator_view(&generated_reference)
            .expect("generated-file operator view");
        assert_eq!(operator_view.kind, RuntimeArtifactKind::GeneratedFile);
        assert_eq!(operator_view.checkpoint_reference_count, 1);
        assert_eq!(
            operator_view.lifecycle,
            agentmage_kernel_contracts::RuntimeArtifactLifecycleState::Active
        );
        let binding = authority
            .authority()
            .current_runtime_resume_binding()
            .expect("resume binding reads")
            .expect("resume binding exists");
        assert!(binding.artifacts.contains(&generated_reference));
        assert_eq!(
            fs::read(root.join("worktree/src/generated.txt")).expect("generated file reads"),
            generated_content.as_bytes()
        );
    }

    #[test]
    fn story_22_2_linux_restart_restores_large_command_and_test_artifacts_without_replay() {
        let checkpoint_clock = Arc::new(AtomicUsize::new(CHECKPOINT_CLOCK_UNARMED));
        let shared_launches = Arc::new(AtomicUsize::new(0));
        let mut fixture = fixture_with_git_and_checkpoint_target(
            FakeGitExecutor::clean(),
            Arc::clone(&checkpoint_clock),
            3,
        );
        configure_bounded_command(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-large-command-resume-22-2");
        let command = scripted_call(&fixture.call);
        configure_targeted_validation(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-large-validation-resume-22-2");
        let validation = scripted_call(&fixture.call);
        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-after-large-output-22-2");
        let git_after = scripted_call(&fixture.call);

        let mut large_validation_output = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "status": "passed",
            "passed": 1,
            "failed": 0,
            "skipped": 0,
            "duration_ms": 1,
            "failed_names": [],
            "artifact_ids": [],
            "retry_count": 0,
            "initial_failure_sha256": null
        }))
        .expect("large validation output");
        large_validation_output.resize(70 * 1024, b' ');
        let executor = fixture
            .boundary
            .command_executor
            .as_mut()
            .expect("large-output command executor");
        executor.stdout = large_validation_output.clone();
        executor.shared_launches = Some(Arc::clone(&shared_launches));
        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Validation];
        fixture.request = seal_runtime_run_request(fixture.request.clone())
            .expect("large-artifact validation requirement");

        let profile = fixture.profile_for_test();
        let workspace = fixture.boundary.workspace;
        let state_root = fixture.root.join("state");
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::NoOp,
            summary: "Resumed after exact command and validation artifact recovery.".to_owned(),
            checks_not_run: Vec::new(),
            residual_risks: Vec::new(),
        })
        .expect("large-artifact completion payload");
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(command),
                ScriptedCodingStep::Tool(validation),
                ScriptedCodingStep::Tool(git_after),
                ScriptedCodingStep::Complete(completion.clone()),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("large-artifact context");
        let Fixture {
            root: _root,
            request,
            boundary,
            ..
        } = fixture;
        let base_request = request.clone();
        let mut coordinator = compose_durable_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            StopAfterCheckpointClock {
                now_epoch_ms: 58_000,
                checkpoint_clock: Arc::clone(&checkpoint_clock),
            },
        )
        .expect("large-artifact coordinator");

        let RuntimeCoordinatorStep::AwaitingApproval {
            challenge: command_challenge,
        } = coordinator
            .run_until_boundary(None, None)
            .expect("large command reaches approval")
        else {
            panic!("large command must require approval");
        };
        let RuntimeCoordinatorStep::AwaitingApproval {
            challenge: validation_challenge,
        } = coordinator
            .run_until_boundary(
                Some(&response(
                    &command_challenge,
                    RuntimeApprovalDisposition::Allow,
                )),
                None,
            )
            .expect("large validation reaches approval")
        else {
            panic!("large validation must require approval");
        };
        let RuntimeCoordinatorStep::AwaitingApproval {
            challenge: git_challenge,
        } = coordinator
            .run_until_boundary(
                Some(&response(
                    &validation_challenge,
                    RuntimeApprovalDisposition::Allow,
                )),
                None,
            )
            .expect("post-validation Git inspection reaches approval")
        else {
            panic!("post-validation Git inspection must require approval");
        };
        assert!(matches!(
            coordinator.run_until_boundary(
                Some(&response(&git_challenge, RuntimeApprovalDisposition::Allow,)),
                None,
            ),
            Err(RuntimeLoopError::Dependency(
                RuntimePortFailure::Unavailable
            ))
        ));
        assert_eq!(shared_launches.load(Ordering::SeqCst), 2);
        let checkpoint_event = coordinator
            .events()
            .last()
            .expect("large-artifact checkpoint event");
        assert!(matches!(
            checkpoint_event.kind,
            RuntimeEventKind::CheckpointCommitted { .. }
        ));
        let requested_cursor = RuntimeEventCursor {
            run_id: checkpoint_event.run_id.clone(),
            event_id: checkpoint_event.event_id.clone(),
            sequence: checkpoint_event.sequence,
            event_sha256: checkpoint_event.event_sha256.clone(),
        };
        let pre_restart_artifacts = coordinator.artifact_references().to_vec();
        drop(coordinator);

        let actor_id = ActorId::from_raw("actor-coding-runtime");
        let task_id = base_request.task.task_id.clone();
        let run_id = base_request.run_id.clone();
        let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &actor_id,
            task_id: &task_id,
            run_id: &run_id,
            workspace: workspace.workspace(),
            profile,
            excluded_scopes: Vec::new(),
        })
        .expect("large-artifact resume policy");
        let mut key = TestKey([51; 32]);
        let authority =
            open_test_linux_authority(&state_root, &mut key, 59_000).expect("authority reopens");
        let artifact_with_kind = |kind| {
            pre_restart_artifacts
                .iter()
                .find(|reference| {
                    authority
                        .runtime_artifact_operator_view(reference)
                        .is_ok_and(|view| view.kind == kind)
                })
                .cloned()
                .expect("expected large artifact kind")
        };
        let command_reference = artifact_with_kind(RuntimeArtifactKind::StandardOutput);
        let test_reference = artifact_with_kind(RuntimeArtifactKind::TestLog);
        assert_eq!(
            command_reference.byte_size,
            large_validation_output.len() as u64
        );
        assert_eq!(
            test_reference.byte_size,
            large_validation_output.len() as u64
        );
        for reference in [&command_reference, &test_reference] {
            assert_eq!(
                authority
                    .read_runtime_artifact(&RuntimeArtifactReadRequest {
                        session_id: base_request.session_id.clone(),
                        task_id: base_request.task.task_id.clone(),
                        policy_sha256: base_request.policy_sha256.clone(),
                        reference: reference.clone(),
                        now_epoch_ms: 59_001,
                        maximum_bytes: MAX_RUNTIME_ARTIFACT_BYTES,
                    })
                    .expect("large artifact reads after restart"),
                large_validation_output
            );
        }

        let mut resumed_boundary =
            LinuxCodingRuntimeBoundary::new(LinuxCodingRuntimeBoundaryInput {
                workspace,
                authority,
                sandbox: sandbox(),
                command_executor: FakeCommandExecutor {
                    shared_launches: Some(Arc::clone(&shared_launches)),
                    ..FakeCommandExecutor::default()
                },
                git_executor: FakeGitExecutor::clean(),
                policy,
                actor_id,
                session_id: base_request.session_id.clone(),
                sensitivity: DataSensitivity::Operational,
                identities: TestIdentities::new(30_000),
            })
            .expect("large-artifact resumed boundary");
        let mut resumed_request = base_request.clone();
        resumed_request.event_cursor = Some(requested_cursor);
        resumed_request =
            seal_runtime_run_request(resumed_request).expect("large-artifact resume request");
        let snapshot =
            RuntimeCheckpointPort::load_runtime_checkpoint(&mut resumed_boundary, &resumed_request)
                .expect("large-artifact checkpoint reads")
                .expect("large-artifact checkpoint exists");
        assert_eq!(snapshot.binding.artifacts, pre_restart_artifacts);

        let resumed_model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [ScriptedCodingStep::Complete(completion)]
                .into_iter()
                .collect(),
            calls: 0,
        };
        let resumed_context =
            CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
                .expect("large-artifact resumed context");
        let mut resumed = compose_durable_coding_coordinator(
            profile,
            resumed_request,
            resumed_model,
            resumed_context,
            resumed_boundary,
            FixtureClock(60_000),
        )
        .expect("large-artifact coordinator restores");
        let RuntimeCoordinatorStep::Complete { outcome } = resumed
            .run_until_boundary(None, None)
            .expect("large-artifact resume completes")
        else {
            panic!("large-artifact resume cannot request another approval");
        };
        assert_eq!(outcome.state, AgentStateKind::NoOp, "{outcome:#?}");
        assert_eq!(outcome.tool_call_count, 3);
        assert_eq!(outcome.receipt_ids.len(), 3);
        assert_eq!(shared_launches.load(Ordering::SeqCst), 2);
        assert_eq!(resumed.artifact_references(), pre_restart_artifacts);
        println!(
            "{ARTIFACT_RESUME_METRIC_PREFIX}{}",
            serde_json::json!({
                "artifact_kinds": ["standard_output", "test_log"],
                "artifact_payload_bytes_each": large_validation_output.len(),
                "exact_artifact_set_restored": true,
                "external_network_used": false,
                "manual_fuzzing_executed": false,
                "post_resume_command_executions": 2,
                "pre_restart_command_executions": 2,
                "scenario": "large-command-test-resume",
                "terminal_state": "no_op",
                "tool_call_count": 3
            })
        );
    }

    #[test]
    fn story_22_2_linux_restart_recovers_large_terminal_model_artifact() {
        let fixture = fixture();
        let profile = fixture.profile_for_test();
        let state_root = fixture.root.join("state");
        let large_model_bytes = vec![b'm'; 70 * 1024];
        let blocked_payload = ContractPayload {
            schema: fixture.call.arguments.schema.clone(),
            media_type: "text/plain".to_owned(),
            sha256: sha256(&large_model_bytes),
            bytes: large_model_bytes.clone(),
        };
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [ScriptedCodingStep::Blocked(blocked_payload)]
                .into_iter()
                .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("large-model context");
        let Fixture {
            request, boundary, ..
        } = fixture;
        let session_id = request.session_id.clone();
        let task_id = request.task.task_id.clone();
        let policy_sha256 = request.policy_sha256.clone();
        let mut coordinator = compose_durable_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(61_000),
        )
        .expect("large-model coordinator");
        let RuntimeCoordinatorStep::Complete { outcome } = coordinator
            .run_until_boundary(None, None)
            .expect("large-model terminal boundary")
        else {
            panic!("blocked model output cannot request tool approval");
        };
        assert_eq!(outcome.state, AgentStateKind::Blocked);
        let model_reference = coordinator
            .artifact_references()
            .iter()
            .find(|reference| reference.payload_sha256 == sha256(&large_model_bytes))
            .cloned()
            .expect("large terminal model output is artifact-backed");
        drop(coordinator);

        let mut key = TestKey([51; 32]);
        let authority =
            open_test_linux_authority(&state_root, &mut key, 62_000).expect("authority reopens");
        assert_eq!(
            authority
                .runtime_artifact_operator_view(&model_reference)
                .expect("large-model operator view")
                .kind,
            RuntimeArtifactKind::ModelOutput
        );
        assert_eq!(
            authority
                .read_runtime_artifact(&RuntimeArtifactReadRequest {
                    session_id,
                    task_id,
                    policy_sha256,
                    reference: model_reference,
                    now_epoch_ms: 62_001,
                    maximum_bytes: MAX_RUNTIME_ARTIFACT_BYTES,
                })
                .expect("large terminal model artifact reads after restart"),
            large_model_bytes
        );
        println!(
            "{ARTIFACT_RESUME_METRIC_PREFIX}{}",
            serde_json::json!({
                "artifact_kind": "model_output",
                "artifact_payload_bytes": 70 * 1024,
                "external_network_used": false,
                "manual_fuzzing_executed": false,
                "payload_recovered_exactly": true,
                "scenario": "large-model-terminal-restart",
                "terminal_state": "blocked"
            })
        );
    }

    #[test]
    fn story_22_2_linux_restart_restores_checkpoint_without_replaying_the_tool() {
        let checkpoint_clock = Arc::new(AtomicUsize::new(CHECKPOINT_CLOCK_UNARMED));
        let shared_launches = Arc::new(AtomicUsize::new(0));
        let mut fixture = fixture_with_git_and_checkpoint_clock(
            FakeGitExecutor::clean_with_counter(Arc::clone(&shared_launches)),
            Some(Arc::clone(&checkpoint_clock)),
        );
        configure_git_status(&mut fixture);
        fixture.call.tool_call_id = ToolCallId::from_raw("call-git-resume-22-2");
        let clean_git = scripted_call(&fixture.call);
        fixture.request.work_packet.required_evidence = vec![EvidenceKind::Observation];
        fixture.request =
            seal_runtime_run_request(fixture.request.clone()).expect("resume evidence requirement");

        let profile = fixture.profile_for_test();
        let workspace = fixture.boundary.workspace;
        let state_root = fixture.root.join("state");
        let completion = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(fixture.request.task.objective.as_bytes()),
            terminal_claim: CodingTerminalClaim::NoOp,
            summary: "Resumed the verified inspection without repeating it.".to_owned(),
            checks_not_run: vec!["No mutation-dependent validation was needed.".to_owned()],
            residual_risks: Vec::new(),
        })
        .expect("resume completion payload");
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(clean_git),
                ScriptedCodingStep::Complete(completion.clone()),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("resume context");
        let Fixture {
            root: _root,
            request,
            boundary,
            ..
        } = fixture;
        let base_request = request.clone();
        let mut coordinator = compose_durable_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            StopAfterCheckpointClock {
                now_epoch_ms: 55_000,
                checkpoint_clock: Arc::clone(&checkpoint_clock),
            },
        )
        .expect("pre-restart coordinator");

        let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
            .run_until_boundary(None, None)
            .expect("Git inspection reaches approval")
        else {
            panic!("Git inspection must require an explicit decision");
        };
        assert!(matches!(
            coordinator.run_until_boundary(
                Some(&response(&challenge, RuntimeApprovalDisposition::Allow)),
                None,
            ),
            Err(RuntimeLoopError::Dependency(
                RuntimePortFailure::Unavailable
            ))
        ));
        assert_eq!(checkpoint_clock.load(Ordering::SeqCst), 0);
        assert_eq!(shared_launches.load(Ordering::SeqCst), 1);
        let checkpoint_event = coordinator
            .events()
            .last()
            .expect("checkpoint is the final pre-restart event");
        assert!(matches!(
            checkpoint_event.kind,
            RuntimeEventKind::CheckpointCommitted { .. }
        ));
        let requested_cursor = RuntimeEventCursor {
            run_id: checkpoint_event.run_id.clone(),
            event_id: checkpoint_event.event_id.clone(),
            sequence: checkpoint_event.sequence,
            event_sha256: checkpoint_event.event_sha256.clone(),
        };
        let pre_restart_artifacts = coordinator.artifact_references().to_vec();
        assert!(
            pre_restart_artifacts
                .iter()
                .any(|reference| reference.media_type == RUNTIME_CONTINUATION_MEDIA_TYPE)
        );
        let pre_restart_receipts = coordinator
            .events()
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::ToolCompleted { .. }))
            .count();
        assert_eq!(pre_restart_receipts, 1);
        drop(coordinator);

        let actor_id = ActorId::from_raw("actor-coding-runtime");
        let task_id = base_request.task.task_id.clone();
        let run_id = base_request.run_id.clone();
        let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
            actor_id: &actor_id,
            task_id: &task_id,
            run_id: &run_id,
            workspace: workspace.workspace(),
            profile,
            excluded_scopes: Vec::new(),
        })
        .expect("resume policy rebuilds exactly");
        let mut key = TestKey([51; 32]);
        let authority =
            open_test_linux_authority(&state_root, &mut key, 56_000).expect("authority reopens");
        let mut resumed_boundary =
            LinuxCodingRuntimeBoundary::new(LinuxCodingRuntimeBoundaryInput {
                workspace,
                authority,
                sandbox: sandbox(),
                command_executor: FakeCommandExecutor::default(),
                git_executor: FakeGitExecutor::clean_with_counter(Arc::clone(&shared_launches)),
                policy,
                actor_id,
                session_id: base_request.session_id.clone(),
                sensitivity: DataSensitivity::Operational,
                identities: TestIdentities::new(10_000),
            })
            .expect("resumed Linux boundary");
        let mut resumed_request = base_request.clone();
        resumed_request.event_cursor = Some(requested_cursor);
        resumed_request =
            seal_runtime_run_request(resumed_request).expect("resume request reseals");

        let mut repository_drift = resumed_request.clone();
        repository_drift.repository_snapshot_sha256 = "f".repeat(64);
        repository_drift =
            seal_runtime_run_request(repository_drift).expect("repository drift reseals");
        assert!(matches!(
            RuntimeCheckpointPort::load_runtime_checkpoint(
                &mut resumed_boundary,
                &repository_drift,
            ),
            Err(RuntimePortFailure::Invalid)
        ));
        let mut policy_drift = resumed_request.clone();
        policy_drift.policy_sha256 = "f".repeat(64);
        policy_drift = seal_runtime_run_request(policy_drift).expect("policy drift reseals");
        assert!(matches!(
            RuntimeCheckpointPort::load_runtime_checkpoint(&mut resumed_boundary, &policy_drift),
            Err(RuntimePortFailure::Invalid)
        ));
        let mut model_profile_drift = resumed_request.clone();
        model_profile_drift.model_profile.profile_id =
            agentmage_kernel_contracts::ModelProfileId::from_raw("model-profile-drift-resume-22-2");
        model_profile_drift =
            seal_runtime_run_request(model_profile_drift).expect("model profile drift reseals");
        assert!(matches!(
            RuntimeCheckpointPort::load_runtime_checkpoint(
                &mut resumed_boundary,
                &model_profile_drift,
            ),
            Err(RuntimePortFailure::Invalid)
        ));
        let mut model_drift = resumed_request.clone();
        model_drift.model_profile.runtime.runtime_sha256 = "f".repeat(64);
        model_drift = seal_runtime_run_request(model_drift).expect("model drift reseals");
        assert!(matches!(
            RuntimeCheckpointPort::load_runtime_checkpoint(&mut resumed_boundary, &model_drift),
            Err(RuntimePortFailure::Invalid)
        ));
        let snapshot =
            RuntimeCheckpointPort::load_runtime_checkpoint(&mut resumed_boundary, &resumed_request)
                .expect("exact resume snapshot loads")
                .expect("exact resume snapshot exists");
        assert_eq!(snapshot.binding.artifacts, pre_restart_artifacts);
        assert_eq!(snapshot.binding.event_cursor.run_id, resumed_request.run_id);

        let resumed_model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [ScriptedCodingStep::Complete(completion)]
                .into_iter()
                .collect(),
            calls: 0,
        };
        let resumed_context =
            CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
                .expect("resumed context");
        let mut resumed = compose_durable_coding_coordinator(
            profile,
            resumed_request,
            resumed_model,
            resumed_context,
            resumed_boundary,
            FixtureClock(57_000),
        )
        .expect("coordinator restores from exact checkpoint");
        let RuntimeCoordinatorStep::Complete { outcome } = resumed
            .run_until_boundary(None, None)
            .expect("resumed coordinator completes")
        else {
            panic!("resumed completion must not request another tool approval");
        };
        assert_eq!(outcome.state, AgentStateKind::NoOp);
        assert_eq!(shared_launches.load(Ordering::SeqCst), 1);
        assert_eq!(outcome.receipt_ids.len(), 1);
        assert_eq!(resumed.artifact_references(), pre_restart_artifacts);
        assert_eq!(
            resumed
                .events()
                .iter()
                .filter(|event| matches!(event.kind, RuntimeEventKind::ToolStarted { .. }))
                .count(),
            1
        );
        assert_eq!(
            resumed
                .events()
                .iter()
                .filter(|event| matches!(event.kind, RuntimeEventKind::ToolCompleted { .. }))
                .count(),
            1
        );
        assert!(matches!(
            resumed.events().last().map(|event| &event.kind),
            Some(RuntimeEventKind::RunTerminal {
                state: AgentStateKind::NoOp,
                ..
            })
        ));
        println!(
            "{ARTIFACT_RESUME_METRIC_PREFIX}{}",
            serde_json::json!({
                "artifact_set": "exact",
                "model_profile_drift": "blocked",
                "model_runtime_drift": "blocked",
                "policy_drift": "blocked",
                "post_resume_receipts": 1,
                "post_resume_total_tool_executions": 1,
                "pre_restart_receipts": 1,
                "pre_restart_tool_executions": 1,
                "repository_drift": "blocked",
                "scenario": "exact-checkpoint-resume",
                "terminal_state": "no_op"
            })
        );
    }

    #[test]
    fn story_22_2_linux_restart_rejects_lost_continuation_without_replaying_the_tool() {
        for corrupt_payload in [false, true] {
            let checkpoint_clock = Arc::new(AtomicUsize::new(CHECKPOINT_CLOCK_UNARMED));
            let shared_launches = Arc::new(AtomicUsize::new(0));
            let mut fixture = fixture_with_git_and_checkpoint_clock(
                FakeGitExecutor::clean_with_counter(Arc::clone(&shared_launches)),
                Some(Arc::clone(&checkpoint_clock)),
            );
            configure_git_status(&mut fixture);
            fixture.call.tool_call_id = ToolCallId::from_raw(format!(
                "call-git-lost-continuation-{}",
                if corrupt_payload {
                    "corrupt"
                } else {
                    "missing"
                }
            ));
            let clean_git = scripted_call(&fixture.call);
            fixture.request.work_packet.required_evidence = vec![EvidenceKind::Observation];
            fixture.request = seal_runtime_run_request(fixture.request.clone())
                .expect("lost-continuation evidence requirement");

            let profile = fixture.profile_for_test();
            let workspace = fixture.boundary.workspace;
            let state_root = fixture.root.join("state");
            let model = ScriptedCodingModel {
                profile: profile.model_profile().clone(),
                steps: [ScriptedCodingStep::Tool(clean_git)].into_iter().collect(),
                calls: 0,
            };
            let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
                .expect("lost-continuation context");
            let Fixture {
                root: _root,
                request,
                boundary,
                ..
            } = fixture;
            let base_request = request.clone();
            let mut coordinator = compose_durable_coding_coordinator(
                profile,
                request,
                model,
                context,
                boundary,
                StopAfterCheckpointClock {
                    now_epoch_ms: 65_000,
                    checkpoint_clock: Arc::clone(&checkpoint_clock),
                },
            )
            .expect("lost-continuation coordinator");

            let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
                .run_until_boundary(None, None)
                .expect("Git inspection reaches approval")
            else {
                panic!("Git inspection must require an explicit decision");
            };
            assert!(matches!(
                coordinator.run_until_boundary(
                    Some(&response(&challenge, RuntimeApprovalDisposition::Allow)),
                    None,
                ),
                Err(RuntimeLoopError::Dependency(
                    RuntimePortFailure::Unavailable
                ))
            ));
            assert_eq!(shared_launches.load(Ordering::SeqCst), 1);
            let checkpoint_event = coordinator
                .events()
                .last()
                .expect("checkpoint is the final pre-restart event");
            let requested_cursor = RuntimeEventCursor {
                run_id: checkpoint_event.run_id.clone(),
                event_id: checkpoint_event.event_id.clone(),
                sequence: checkpoint_event.sequence,
                event_sha256: checkpoint_event.event_sha256.clone(),
            };
            let continuation_artifact = coordinator
                .artifact_references()
                .iter()
                .find(|reference| reference.media_type == RUNTIME_CONTINUATION_MEDIA_TYPE)
                .cloned()
                .expect("checkpoint continuation artifact");
            drop(coordinator);

            let continuation_path = state_root
                .join(".agentmage-runtime-payloads-v1")
                .join("objects")
                .join(&continuation_artifact.payload_sha256);
            if corrupt_payload {
                fs::write(&continuation_path, b"corrupt encrypted continuation")
                    .expect("continuation payload corrupts");
            } else {
                fs::remove_file(&continuation_path).expect("continuation payload disappears");
            }

            let actor_id = ActorId::from_raw("actor-coding-runtime");
            let task_id = base_request.task.task_id.clone();
            let run_id = base_request.run_id.clone();
            let policy = build_coding_runtime_policy(CodingRuntimePolicyRequest {
                actor_id: &actor_id,
                task_id: &task_id,
                run_id: &run_id,
                workspace: workspace.workspace(),
                profile,
                excluded_scopes: Vec::new(),
            })
            .expect("lost-continuation policy rebuilds exactly");
            let mut key = TestKey([51; 32]);
            let authority = open_test_linux_authority(&state_root, &mut key, 66_000)
                .expect("authority reconciles continuation loss");
            let operator_view = authority
                .runtime_artifact_operator_view(&continuation_artifact)
                .expect("quarantined continuation remains operator-visible");
            assert_eq!(
                operator_view.lifecycle,
                agentmage_kernel_contracts::RuntimeArtifactLifecycleState::Quarantined
            );
            assert_eq!(
                operator_view.cleanup,
                agentmage_kernel_contracts::RuntimeArtifactCleanupState::Blocked
            );
            assert_eq!(
                operator_view.integrity,
                if corrupt_payload {
                    agentmage_kernel_contracts::RuntimeArtifactIntegrityState::Corrupt
                } else {
                    agentmage_kernel_contracts::RuntimeArtifactIntegrityState::Missing
                }
            );
            assert_eq!(
                operator_view.reason_code,
                if corrupt_payload {
                    "artifact.payload_corrupt"
                } else {
                    "artifact.payload_missing"
                }
            );
            let resumed_boundary =
                LinuxCodingRuntimeBoundary::new(LinuxCodingRuntimeBoundaryInput {
                    workspace,
                    authority,
                    sandbox: sandbox(),
                    command_executor: FakeCommandExecutor::default(),
                    git_executor: FakeGitExecutor::clean_with_counter(Arc::clone(&shared_launches)),
                    policy,
                    actor_id,
                    session_id: base_request.session_id.clone(),
                    sensitivity: DataSensitivity::Operational,
                    identities: TestIdentities::new(20_000),
                })
                .expect("lost-continuation Linux boundary");
            let mut resumed_request = base_request.clone();
            resumed_request.event_cursor = Some(requested_cursor);
            resumed_request =
                seal_runtime_run_request(resumed_request).expect("lost-continuation request");
            let resumed_model = ScriptedCodingModel {
                profile: profile.model_profile().clone(),
                steps: VecDeque::new(),
                calls: 0,
            };
            let resumed_context =
                CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
                    .expect("lost-continuation resumed context");
            assert!(matches!(
                compose_durable_coding_coordinator(
                    profile,
                    resumed_request,
                    resumed_model,
                    resumed_context,
                    resumed_boundary,
                    FixtureClock(67_000),
                ),
                Err(RuntimeLoopError::Dependency(RuntimePortFailure::Invalid))
            ));
            assert_eq!(shared_launches.load(Ordering::SeqCst), 1);
            assert!(!continuation_path.exists());
        }
        println!(
            "{ARTIFACT_RESUME_METRIC_PREFIX}{}",
            serde_json::json!({
                "active_continuation_objects_after_reconcile": 0,
                "cases": ["missing", "corrupt"],
                "cases_blocked": 2,
                "operator_cleanup": "blocked",
                "operator_integrity_by_case": {
                    "corrupt": "corrupt",
                    "missing": "missing"
                },
                "operator_lifecycle": "quarantined",
                "post_failure_total_tool_executions_per_case": 1,
                "pre_restart_tool_executions_per_case": 1,
                "scenario": "continuation-integrity-loss"
            })
        );
    }

    #[test]
    fn s_048_mvp_e2e_denial_and_user_cancellation_start_no_effect() {
        for cancel in [false, true] {
            let mut fixture = fixture();
            configure_structured_patch(&mut fixture);
            let patch = scripted_call(&fixture.call);
            let original =
                fs::read(fixture.root.join("worktree/src/lib.rs")).expect("original source");
            let profile = fixture.profile_for_test();
            let model = ScriptedCodingModel {
                profile: profile.model_profile().clone(),
                steps: [ScriptedCodingStep::Tool(patch)].into_iter().collect(),
                calls: 0,
            };
            let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
                .expect("coding context");
            let Fixture {
                root,
                request,
                boundary,
                ..
            } = fixture;
            let mut coordinator = compose_ephemeral_coding_coordinator(
                profile,
                request,
                model,
                context,
                boundary,
                FixtureClock(50_000),
            )
            .expect("ephemeral coding coordinator");
            let outcome = if cancel {
                let RuntimeCoordinatorStep::AwaitingApproval { challenge } = coordinator
                    .run_until_boundary(None, None)
                    .expect("approval boundary")
                else {
                    panic!("patch must require approval");
                };
                let approval_response = response(&challenge, RuntimeApprovalDisposition::Allow);
                let cancellation = CancellationSignal {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    cancellation_id: CancellationId::from_raw("cancellation-coding-e2e"),
                    correlation_id: coordinator
                        .events()
                        .first()
                        .expect("run-start event")
                        .correlation_id
                        .clone(),
                    task_id: challenge.task_id.clone(),
                    reason: CancellationReason::UserRequested,
                    requested_by: BoundaryKind::Shell,
                };
                let RuntimeCoordinatorStep::Complete { outcome } = coordinator
                    .run_until_boundary(Some(&approval_response), Some(&cancellation))
                    .expect("terminal boundary")
                else {
                    panic!("cancellation must be terminal");
                };
                outcome
            } else {
                let mut approvals = DenyHeadlessApproval;
                let mut sink = CollectingEventSink::default();
                drive_coding_client(&mut coordinator, &mut approvals, &mut sink, None)
                    .expect("headless denial")
                    .outcome
            };
            assert_eq!(
                outcome.state,
                if cancel {
                    AgentStateKind::Cancelled
                } else {
                    AgentStateKind::Declined
                }
            );
            assert_eq!(
                fs::read(root.join("worktree/src/lib.rs")).expect("preserved source"),
                original
            );
            assert!(outcome.receipt_ids.is_empty());
        }
    }

    #[test]
    fn s_048_mvp_e2e_failed_validation_is_never_reported_as_success() {
        let mut fixture = fixture();
        configure_structured_patch(&mut fixture);
        let patch = scripted_call(&fixture.call);
        configure_targeted_validation(&mut fixture);
        let validation = scripted_call(&fixture.call);
        let executor = fixture
            .boundary
            .command_executor
            .as_mut()
            .expect("test command executor");
        executor.exit_code = 1;
        executor.stdout = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "status": "assertion_failed",
            "passed": 0,
            "failed": 1,
            "skipped": 0,
            "duration_ms": 1,
            "failed_names": ["fixture::regression"],
            "artifact_ids": [],
            "retry_count": 0,
            "initial_failure_sha256": null
        }))
        .expect("failed validation output");

        let profile = fixture.profile_for_test();
        let model = ScriptedCodingModel {
            profile: profile.model_profile().clone(),
            steps: [
                ScriptedCodingStep::Tool(patch),
                ScriptedCodingStep::Tool(validation),
            ]
            .into_iter()
            .collect(),
            calls: 0,
        };
        let context = CodingContextPort::for_profile(profile, Vec::new(), FixtureTokenCounter)
            .expect("coding context");
        let Fixture {
            root,
            request,
            boundary,
            ..
        } = fixture;
        let mut coordinator = compose_ephemeral_coding_coordinator(
            profile,
            request,
            model,
            context,
            boundary,
            FixtureClock(60_000),
        )
        .expect("ephemeral coding coordinator");

        let mut next_response = None;
        let outcome = loop {
            match coordinator
                .run_until_boundary(next_response.as_ref(), None)
                .expect("coding coordinator boundary")
            {
                RuntimeCoordinatorStep::AwaitingApproval { challenge } => {
                    next_response = Some(response(&challenge, RuntimeApprovalDisposition::Allow));
                }
                RuntimeCoordinatorStep::Complete { outcome } => break outcome,
            }
        };

        assert_eq!(outcome.state, AgentStateKind::Failed, "{outcome:#?}");
        assert!(
            outcome
                .unresolved_codes
                .iter()
                .any(|code| code == "runtime.tool.failed")
        );
        assert_eq!(
            fs::read(root.join("worktree/src/lib.rs")).expect("changed source"),
            b"pub fn runtime_updated() {}\n"
        );
    }

    #[test]
    fn story_48_2_linux_runtime_asks_then_derives_one_exact_operation_grant() {
        let mut fixture = fixture();
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                1_000,
            )
            .expect("approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let allowed = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                1_001,
            )
            .expect("exact allow");

        assert!(matches!(allowed, RuntimePermissionEvaluation::Allow { .. }));
        assert!(fixture.boundary.pending.is_empty());
        assert_eq!(fixture.boundary.issued.len(), 1);
        assert!(fixture.boundary.authority.authority().receipts().is_empty());
    }

    #[test]
    fn story_48_2_linux_runtime_user_denial_is_inert_and_non_replayable() {
        let mut fixture = fixture();
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                2_000,
            )
            .expect("approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let denied = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Deny),
                &fixture.definition,
                &fixture.call,
                2_001,
            )
            .expect("exact deny");

        assert!(matches!(denied, RuntimePermissionEvaluation::Deny { .. }));
        assert!(fixture.boundary.pending.is_empty());
        assert!(fixture.boundary.issued.is_empty());
        assert!(fixture.boundary.authority.authority().receipts().is_empty());
        assert_eq!(
            fixture.boundary.resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Deny),
                &fixture.definition,
                &fixture.call,
                2_002,
            ),
            Err(RuntimePortFailure::Invalid)
        );
    }

    #[test]
    fn story_48_2_linux_runtime_rejects_substitution_staleness_and_unbudgeted_actions() {
        let mut fixture = fixture();
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                3_000,
            )
            .expect("approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let mut substituted = fixture.call.clone();
        substituted.tool_call_id = ToolCallId::from_raw("call-substituted");
        assert_eq!(
            fixture.boundary.resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &substituted,
                3_001,
            ),
            Err(RuntimePortFailure::Invalid)
        );

        fs::write(
            fixture.root.join("worktree/src/lib.rs"),
            b"pub fn changed_after_preview() {}\n",
        )
        .expect("change fixture after preview");
        assert_eq!(
            fixture.boundary.resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                3_002,
            ),
            Err(RuntimePortFailure::Invalid)
        );

        let mut out_of_budget = fixture.call.clone();
        out_of_budget.tool_call_id = ToolCallId::from_raw("call-out-of-budget");
        out_of_budget.action_id = ActionId::from_raw("action-out-of-budget");
        assert_eq!(
            fixture.boundary.evaluate(
                &fixture.request,
                &RuntimeOperationId::from_raw("operation-out-of-budget"),
                &fixture.definition,
                &out_of_budget,
                3_003,
            ),
            Err(RuntimePortFailure::Invalid)
        );
    }

    #[test]
    fn story_48_2_linux_runtime_executes_one_exact_bounded_command_after_approval() {
        let mut fixture = fixture();
        let command = fixture
            .profile_for_test()
            .commands()
            .commands()
            .into_iter()
            .next()
            .expect("registered command");
        let definition = fixture
            .profile_for_test()
            .registry()
            .get_tool(
                &ToolId::from_raw(crate::coding_tools::BOUNDED_COMMAND_TOOL_ID),
                crate::coding_tools::BOUNDED_COMMAND_TOOL_VERSION,
            )
            .expect("command tool")
            .clone();
        let arguments =
            serde_json::to_vec(&CommandRequest::new("command-attempt-runtime", command))
                .expect("command request");
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-command-runtime"),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&fixture.request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        fixture.call = call;
        fixture.definition = definition;
        assert!(fixture.boundary.request_matches(&fixture.request));
        assert!(
            fixture
                .boundary
                .definition_matches(&fixture.definition, &fixture.call)
        );
        assert!(fixture.boundary.workspace.prepare(&fixture.call).is_ok());
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                4_000,
            )
            .expect("command approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let allowed = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                4_001,
            )
            .expect("command allow");
        let execution = fixture
            .boundary
            .execute(
                &fixture.request,
                &allowed,
                &fixture.definition,
                &fixture.call,
                None,
            )
            .expect("bounded command execution");

        assert_eq!(execution.result.outcome, OperationOutcome::Succeeded);
        assert_eq!(
            execution.result_output_kind,
            Some(RuntimeArtifactKind::Report)
        );
        assert_eq!(execution.artifact_candidates.len(), 1);
        assert_eq!(
            execution.artifact_candidates[0].kind,
            RuntimeArtifactKind::StandardOutput
        );
        assert_eq!(execution.artifact_candidates[0].bytes, b"command-ok\n");
        let command_receipt: CommandReceipt =
            serde_json::from_slice(&execution.result.output.expect("command output").bytes)
                .expect("command receipt payload");
        assert_eq!(command_receipt.stdout_sha256, sha256(b"command-ok\n"));
        assert_eq!(execution.result.evidence.len(), 1);
        assert_eq!(
            fixture
                .boundary
                .command_executor
                .as_ref()
                .expect("returned executor")
                .launches,
            1
        );
        assert_eq!(fixture.boundary.authority.authority().receipts().len(), 1);
    }

    #[test]
    fn story_48_2_linux_runtime_executes_one_kernel_checked_git_inspection() {
        let mut fixture = fixture();
        configure_git_status(&mut fixture);
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                4_100,
            )
            .expect("Git approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let allowed = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                4_101,
            )
            .expect("Git allow");
        let execution = fixture
            .boundary
            .execute(
                &fixture.request,
                &allowed,
                &fixture.definition,
                &fixture.call,
                None,
            )
            .expect("Git inspection execution");

        assert_eq!(execution.result.outcome, OperationOutcome::Succeeded);
        let result: GitInspectionResult =
            serde_json::from_slice(&execution.result.output.expect("Git output").bytes)
                .expect("Git result payload");
        assert!(result.verify());
        assert_eq!(result.operation, GitInspectionOperation::Status);
        assert_eq!(result.records.len(), 2);
        assert_eq!(execution.result.evidence[0].kind, EvidenceKind::Observation);
        assert_eq!(
            fixture
                .boundary
                .git_executor
                .as_ref()
                .expect("returned Git executor")
                .launches,
            1
        );
        assert_eq!(fixture.boundary.authority.authority().receipts().len(), 1);
    }

    #[test]
    fn story_48_2_linux_git_adapter_runs_the_approved_plan_without_a_shell() {
        let git = LinuxGitArtifact::verify("/usr/bin/git").expect("verified system Git");
        let mut fixture = fixture_with_git(LinuxBoundedRepositoryInspectionExecutor::new(git));
        let initialized = Command::new("/usr/bin/git")
            .env_clear()
            .env("HOME", "/nonexistent")
            .env("LANG", "C")
            .args(["init", "--quiet", "--initial-branch=main"])
            .current_dir(fixture.root.join("worktree"))
            .status()
            .expect("initialize fixture repository");
        assert!(initialized.success());
        configure_git_status(&mut fixture);
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                4_200,
            )
            .expect("Git approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let allowed = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                4_201,
            )
            .expect("Git allow");
        let execution = fixture
            .boundary
            .execute(
                &fixture.request,
                &allowed,
                &fixture.definition,
                &fixture.call,
                None,
            )
            .expect("live Git inspection");

        assert_eq!(execution.result.outcome, OperationOutcome::Succeeded);
        let result: GitInspectionResult =
            serde_json::from_slice(&execution.result.output.expect("Git output").bytes)
                .expect("Git result payload");
        assert!(result.verify());
        assert!(
            result
                .records
                .iter()
                .any(|record| record.record_kind == "untracked")
        );
        assert_eq!(fixture.boundary.authority.authority().receipts().len(), 1);
    }

    #[test]
    fn story_48_2_kernel_and_capability_git_planners_match_every_operation() {
        for operation in GitInspectionOperation::ALL {
            let request = GitInspectionRequest {
                schema_version: 1,
                operation,
                revision: matches!(
                    operation,
                    GitInspectionOperation::Show | GitInspectionOperation::Ref
                )
                .then(|| "HEAD".to_owned()),
                object_id: (operation == GitInspectionOperation::Object).then(|| "a".repeat(40)),
                pathspecs: matches!(
                    operation,
                    GitInspectionOperation::Diff
                        | GitInspectionOperation::StagedDiff
                        | GitInspectionOperation::Show
                )
                .then(|| vec![vec!["src".to_owned(), "lib.rs".to_owned()]])
                .unwrap_or_default(),
                max_records: 32,
                max_output_bytes: 4_096,
            };
            let plan = plan_git_inspection(&request).expect("capability Git plan");
            let prepared =
                prepare_kernel_git_inspection(&request, &plan, &"a".repeat(64), &"b".repeat(64))
                    .expect("matching kernel Git plan");
            assert_eq!(plan.argv[1..], prepared.arguments()[..]);
        }

        let request = GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::Status,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 32,
            max_output_bytes: 4_096,
        };
        let mut changed = plan_git_inspection(&request).expect("capability Git plan");
        changed
            .environment
            .insert("HOME".to_owned(), "/tmp".to_owned());
        assert_eq!(
            prepare_kernel_git_inspection(&request, &changed, &"a".repeat(64), &"b".repeat(64),)
                .expect_err("planner drift must fail"),
            RuntimePortFailure::Invalid
        );
    }

    #[test]
    fn story_48_2_linux_runtime_normalizes_targeted_validation_after_one_granted_command() {
        let mut fixture = fixture();
        let template = fixture
            .profile_for_test()
            .validations()
            .templates
            .first()
            .expect("validation template")
            .clone();
        let definition = fixture
            .profile_for_test()
            .registry()
            .get_tool(
                &ToolId::from_raw(crate::coding_tools::TARGETED_VALIDATION_TOOL_ID),
                crate::coding_tools::TARGETED_VALIDATION_TOOL_VERSION,
            )
            .expect("validation tool")
            .clone();
        let arguments = serde_json::to_vec(&TargetedValidationRequest {
            schema_version: 1,
            validation_attempt_id: "validation-attempt-runtime".to_owned(),
            validation_id: template.validation_id.clone(),
            template_sha256: template.template_sha256.clone(),
        })
        .expect("validation request");
        fixture.call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-validation-runtime"),
            correlation_id: CorrelationId::from_raw("correlation-coding-runtime"),
            action_id: runtime_action_id(&fixture.request.run_id, 1),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256(&arguments),
                bytes: arguments,
            },
        };
        fixture.definition = definition;
        fixture
            .boundary
            .command_executor
            .as_mut()
            .expect("test executor")
            .stdout = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "status": "passed",
            "passed": 1,
            "failed": 0,
            "skipped": 0,
            "duration_ms": 1,
            "failed_names": [],
            "artifact_ids": [],
            "retry_count": 0,
            "initial_failure_sha256": null
        }))
        .expect("validation output");
        let evaluation = fixture
            .boundary
            .evaluate(
                &fixture.request,
                &fixture.operation_id,
                &fixture.definition,
                &fixture.call,
                5_000,
            )
            .expect("validation approval preview");
        let challenge = challenge(&fixture, &evaluation);
        let allowed = fixture
            .boundary
            .resolve(
                &fixture.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Allow),
                &fixture.definition,
                &fixture.call,
                5_001,
            )
            .expect("validation allow");
        let execution = fixture
            .boundary
            .execute(
                &fixture.request,
                &allowed,
                &fixture.definition,
                &fixture.call,
                None,
            )
            .expect("targeted validation execution");

        assert_eq!(execution.result.outcome, OperationOutcome::Succeeded);
        assert_eq!(
            execution.result_output_kind,
            Some(RuntimeArtifactKind::TestLog)
        );
        assert_eq!(execution.artifact_candidates.len(), 1);
        assert_eq!(
            execution.artifact_candidates[0].kind,
            RuntimeArtifactKind::TestLog
        );
        let validation_receipt: ValidationReceipt =
            serde_json::from_slice(&execution.result.output.expect("validation output").bytes)
                .expect("validation receipt payload");
        assert_eq!(validation_receipt.status, ValidationStatus::Passed);
        assert_eq!(validation_receipt.passed, 1);
        assert_eq!(execution.result.evidence[0].kind, EvidenceKind::Validation);
        assert_eq!(fixture.boundary.authority.authority().receipts().len(), 1);
    }

    #[test]
    fn story_48_2_linux_runtime_applies_one_exact_structured_patch() {
        let mut fixture = fixture();
        let untouched_path = fixture.root.join("worktree/src/untouched.rs");
        fs::write(&untouched_path, b"pub fn untouched() {}\n").expect("untouched source");
        configure_structured_patch(&mut fixture);
        let allowed = approve(&mut fixture, 6_000);
        let grant_id = match &allowed {
            RuntimePermissionEvaluation::Allow { grant_id, .. } => grant_id.clone(),
            _ => panic!("expected exact write grant"),
        };

        let execution = fixture
            .boundary
            .execute(
                &fixture.request,
                &allowed,
                &fixture.definition,
                &fixture.call,
                None,
            )
            .expect("structured write execution");

        assert_eq!(execution.result.outcome, OperationOutcome::Succeeded);
        assert_eq!(execution.result.state_change, StateChange::Changed);
        assert_eq!(execution.result.evidence[0].kind, EvidenceKind::Receipt);
        assert_eq!(
            fs::read(fixture.root.join("worktree/src/lib.rs")).expect("updated source"),
            b"pub fn runtime_updated() {}\n"
        );
        assert_eq!(
            fs::read(&untouched_path).expect("untouched source remains"),
            b"pub fn untouched() {}\n"
        );
        let output: serde_json::Value = serde_json::from_slice(
            &execution
                .result
                .output
                .expect("controlled write output")
                .bytes,
        )
        .expect("controlled write JSON");
        assert_eq!(output["outcome"], "succeeded");
        assert_eq!(
            output["preimage_sha256"],
            sha256(b"pub fn runtime_fixture() {}\n")
        );
        assert_eq!(
            fixture
                .boundary
                .authority
                .authority()
                .current_grant(&grant_id)
                .expect("durable write grant")
                .status,
            GrantStatus::Consumed
        );
    }

    #[test]
    fn story_48_2_linux_runtime_creates_one_exact_absent_file() {
        let mut fixture = fixture();
        let original = fs::read(fixture.root.join("worktree/src/lib.rs")).expect("original source");
        configure_controlled_create(&mut fixture);
        let allowed = approve(&mut fixture, 7_000);
        let grant_id = match &allowed {
            RuntimePermissionEvaluation::Allow { grant_id, .. } => grant_id.clone(),
            _ => panic!("expected exact filesystem grant"),
        };

        let execution = fixture
            .boundary
            .execute(
                &fixture.request,
                &allowed,
                &fixture.definition,
                &fixture.call,
                None,
            )
            .expect("controlled create execution");

        assert_eq!(execution.result.outcome, OperationOutcome::Succeeded);
        assert_eq!(execution.result.state_change, StateChange::Changed);
        assert_eq!(
            fs::read(fixture.root.join("worktree/src/new.rs")).expect("created source"),
            b"pub fn newly_created() {}\n"
        );
        assert_eq!(
            fs::read(fixture.root.join("worktree/src/lib.rs")).expect("original source remains"),
            original
        );
        let output: serde_json::Value = serde_json::from_slice(
            &execution
                .result
                .output
                .expect("controlled create output")
                .bytes,
        )
        .expect("controlled create JSON");
        assert!(output["preimage_sha256"].is_null());
        assert_eq!(
            fixture
                .boundary
                .authority
                .authority()
                .current_grant(&grant_id)
                .expect("durable filesystem grant")
                .status,
            GrantStatus::Consumed
        );
    }

    #[test]
    fn story_48_2_linux_runtime_persists_stale_patch_invalidation_without_overwrite() {
        let mut fixture = fixture();
        configure_structured_patch(&mut fixture);
        let allowed = approve(&mut fixture, 8_000);
        let grant_id = match &allowed {
            RuntimePermissionEvaluation::Allow { grant_id, .. } => grant_id.clone(),
            _ => panic!("expected exact write grant"),
        };
        let concurrent = b"pub fn concurrent_user_change() {}\n";
        fs::write(fixture.root.join("worktree/src/lib.rs"), concurrent)
            .expect("concurrent source change");

        assert_eq!(
            fixture.boundary.execute(
                &fixture.request,
                &allowed,
                &fixture.definition,
                &fixture.call,
                None,
            ),
            Err(RuntimePortFailure::Uncertain)
        );
        assert_eq!(
            fs::read(fixture.root.join("worktree/src/lib.rs")).expect("concurrent source remains"),
            concurrent
        );
        assert_eq!(
            fixture
                .boundary
                .authority
                .authority()
                .current_grant(&grant_id)
                .expect("invalidated write grant")
                .status,
            GrantStatus::Invalidated
        );
    }

    #[test]
    fn story_48_2_linux_runtime_denial_and_create_collision_are_inert() {
        let mut denied = fixture();
        configure_controlled_create(&mut denied);
        let evaluation = denied
            .boundary
            .evaluate(
                &denied.request,
                &denied.operation_id,
                &denied.definition,
                &denied.call,
                9_000,
            )
            .expect("create preview");
        let challenge = challenge(&denied, &evaluation);
        denied
            .boundary
            .resolve(
                &denied.request,
                &challenge,
                &response(&challenge, RuntimeApprovalDisposition::Deny),
                &denied.definition,
                &denied.call,
                9_001,
            )
            .expect("create denial");
        assert!(!denied.root.join("worktree/src/new.rs").exists());

        let mut collision = fixture();
        configure_controlled_create(&mut collision);
        let allowed = approve(&mut collision, 10_000);
        let grant_id = match &allowed {
            RuntimePermissionEvaluation::Allow { grant_id, .. } => grant_id.clone(),
            _ => panic!("expected exact filesystem grant"),
        };
        let user_content = b"pub fn user_created_first() {}\n";
        fs::write(collision.root.join("worktree/src/new.rs"), user_content)
            .expect("user collision");
        assert_eq!(
            collision.boundary.execute(
                &collision.request,
                &allowed,
                &collision.definition,
                &collision.call,
                None,
            ),
            Err(RuntimePortFailure::Uncertain)
        );
        assert_eq!(
            fs::read(collision.root.join("worktree/src/new.rs")).expect("collision remains"),
            user_content
        );
        assert_eq!(
            collision
                .boundary
                .authority
                .authority()
                .current_grant(&grant_id)
                .expect("invalidated filesystem grant")
                .status,
            GrantStatus::Invalidated
        );
    }

    impl<G> Fixture<G>
    where
        G: BoundedRepositoryInspectionExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>,
    {
        fn profile_for_test(&self) -> &'static CodingSessionProfile {
            self.boundary.workspace.profile()
        }
    }

    fn sandbox() -> LinuxSandboxRunner {
        let manifest = LinuxSandboxManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/bwrap",
            "/usr/bin/cat",
            &[],
        )
        .expect("test sandbox manifest");
        LinuxSandboxRunner::new(manifest, LinuxSandboxLimits::default())
            .expect("test sandbox runner")
    }

    fn temp_root(label: &str) -> PathBuf {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agentmage-linux-coding-runtime-{}-{label}-{id}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("remove stale fixture");
        }
        fs::create_dir(&path).expect("create fixture root");
        path
    }
}
