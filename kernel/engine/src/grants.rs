//! Kernel-only issuance of session-read and derived operation grants.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, CapabilityGrant, DataSensitivity, GrantClass,
    GrantId, GrantNonce, GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget,
    OperationBinding, SessionId, TaskId, ToolId, to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::policy::{PolicyDenialScope, PolicyEngine, PolicyEvaluationContext};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TARGETS: usize = 128;
const MAX_EFFECTS: usize = 128;
const MAX_ROLLBACK_BYTES: usize = 1_024;
const MAX_TOOL_VERSION_BYTES: usize = 64;
const MAX_GRANT_LIFETIME_MS: u64 = 86_400_000;
const MAX_SESSION_DERIVATIONS: u32 = 4_096;

/// Stable reason a grant cannot be issued or derived.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantIssueError {
    /// One or more closed input fields are malformed or out of bounds.
    InvalidInput,
    /// The grant identity already exists in issuer state.
    DuplicateGrant,
    /// The nonce was already used by another issued grant.
    NonceReuse,
    /// The named parent grant does not exist.
    ParentNotFound,
    /// The named parent is not a session-read grant.
    InvalidParentClass,
    /// The parent is terminal or has exhausted its derivation limit.
    ParentUnavailable,
    /// The parent expired before the requested derivation.
    ParentExpired,
    /// Child policy identity differs from the exact parent policy.
    PolicyChanged,
    /// Child targets or exclusions would broaden the parent scope.
    ScopeBroadened,
}

impl GrantIssueError {
    /// Returns the redacted stable code used by receipts and tests.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "grant.issue.invalid_input",
            Self::DuplicateGrant => "grant.issue.duplicate_id",
            Self::NonceReuse => "grant.issue.nonce_reuse",
            Self::ParentNotFound => "grant.derive.parent_missing",
            Self::InvalidParentClass => "grant.derive.parent_class",
            Self::ParentUnavailable => "grant.derive.parent_unavailable",
            Self::ParentExpired => "grant.derive.parent_expired",
            Self::PolicyChanged => "grant.derive.policy_changed",
            Self::ScopeBroadened => "grant.derive.scope_broadened",
        }
    }
}

/// Exact user-confirmed inputs for a session-scoped read parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionReadGrantRequest {
    /// New grant identity selected by the kernel.
    pub grant_id: GrantId,
    /// Exact local actor identity that confirmed the workspace scope.
    pub actor_id: ActorId,
    /// Owning session identity.
    pub session_id: SessionId,
    /// Owning task identity.
    pub task_id: TaskId,
    /// Included workspace-relative roots shown to the actor.
    pub targets: Vec<GrantTarget>,
    /// Excluded subtrees shown to the actor.
    pub excluded_targets: Vec<GrantTarget>,
    /// Data sensitivity shown with the scope.
    pub sensitivity: DataSensitivity,
    /// Exact issuance instant supplied by the kernel clock boundary.
    pub issued_at_epoch_ms: u64,
    /// Exact expiration instant shown to the actor.
    pub expires_at_epoch_ms: u64,
    /// Unique anti-replay nonce selected by the kernel.
    pub nonce: GrantNonce,
    /// Maximum operation grants that may be derived from this parent.
    pub maximum_derived_operations: u32,
    /// Digest of the exact confirmed workspace preview.
    pub preview_sha256: String,
    /// Digest of the exact policy revision used for issuance.
    pub policy_sha256: String,
}

/// Exact inputs for one operation grant derived from a session-read parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedOperationGrantRequest {
    /// New child grant identity selected by the kernel.
    pub grant_id: GrantId,
    /// Exact explicit approval decision that confirmed this operation.
    pub approval_id: ApprovalId,
    /// Exact action receiving proposed authority.
    pub action_id: ActionId,
    /// Exact descriptive class of the action.
    pub action_kind: ActionKind,
    /// Closed operation class requested by the action.
    pub operation: OperationBinding,
    /// Exact registered tool identity.
    pub tool_id: ToolId,
    /// Exact registered tool-contract version.
    pub tool_version: String,
    /// Exact included targets, all of which must remain inside parent scope.
    pub targets: Vec<GrantTarget>,
    /// Digest of canonical tool arguments.
    pub argument_sha256: String,
    /// Exact target preimages required immediately before execution.
    pub preimages: Vec<GrantPreimage>,
    /// Expected side effects included in the confirmed preview.
    pub expected_side_effects: Vec<GrantSideEffect>,
    /// Bounded rollback or recovery description shown in the preview.
    pub rollback_description: String,
    /// Exact issuance instant supplied by the kernel clock boundary.
    pub issued_at_epoch_ms: u64,
    /// Exact child expiration, no later than the parent expiration.
    pub expires_at_epoch_ms: u64,
    /// Unique anti-replay nonce selected by the kernel.
    pub nonce: GrantNonce,
    /// Digest of the exact confirmed operation preview.
    pub preview_sha256: String,
    /// Digest of the exact policy revision; this must equal the parent policy.
    pub policy_sha256: String,
}

/// Stable reason an operation grant cannot be consumed for one execution attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantConsumeError {
    /// No kernel-issued grant has the requested identity.
    NotFound,
    /// Retained state, revision identity, or canonical hashing is inconsistent.
    CorruptState,
    /// Current policy or execution observations denied the grant.
    PolicyDenied(PolicyDenialScope),
}

impl GrantConsumeError {
    /// Returns the stable redacted code used by future receipts.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "grant.consume.not_found",
            Self::CorruptState => "grant.consume.corrupt_state",
            Self::PolicyDenied(_) => "grant.consume.policy_denied",
        }
    }
}

/// Non-authoritative evidence that one exact grant advanced to consumed state.
///
/// This record contains no target, tool, argument, or operation scope and cannot authorize a
/// call. The future dispatcher must invoke consumption as its final kernel transaction before
/// starting the one isolated worker attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantConsumptionRecord {
    /// Exact consumed grant identity.
    pub grant_id: GrantId,
    /// Last issued revision that policy admitted.
    pub issued_revision: u32,
    /// New terminal revision retained by the issuer.
    pub consumed_revision: u32,
    /// Canonical digest of the admitted issued revision.
    pub issued_grant_sha256: String,
    /// Canonical digest of the retained consumed revision.
    pub consumed_grant_sha256: String,
    /// Kernel-clock instant used for final policy evaluation.
    pub consumed_at_epoch_ms: u64,
    /// Kernel-computed policy identity that admitted the attempt.
    pub policy_sha256: String,
}

/// Stable reason a post-attempt grant lifecycle transition cannot be retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantLifecycleError {
    /// No kernel-issued grant has the requested identity.
    NotFound,
    /// The grant is not the exact consumed attempt expected by this transition.
    InvalidState,
    /// The caller's expected consumed-revision digest does not match retained state.
    RevisionMismatch,
    /// Retained state or canonical hashing is inconsistent.
    CorruptState,
}

impl GrantLifecycleError {
    /// Returns the stable redacted code used by future receipts.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotFound => "grant.lifecycle.not_found",
            Self::InvalidState => "grant.lifecycle.invalid_state",
            Self::RevisionMismatch => "grant.lifecycle.revision_mismatch",
            Self::CorruptState => "grant.lifecycle.corrupt_state",
        }
    }
}

/// Non-authoritative evidence of one retained terminal lifecycle transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantLifecycleRecord {
    /// Exact transitioned grant identity.
    pub grant_id: GrantId,
    /// Previous retained revision.
    pub previous_revision: u32,
    /// New retained terminal revision.
    pub terminal_revision: u32,
    /// Canonical digest of the previous revision.
    pub previous_grant_sha256: String,
    /// Canonical digest of the new terminal revision.
    pub terminal_grant_sha256: String,
    /// New terminal status.
    pub status: GrantStatus,
    /// Kernel-clock instant associated with the transition.
    pub occurred_at_epoch_ms: u64,
}

/// Kernel grant state machine cached from canonical encrypted state.
///
/// Authority exists only when a candidate exactly matches this issuer's retained record.
/// Publicly constructed `CapabilityGrant` values are not inserted implicitly.
#[derive(Clone, Debug)]
pub struct GrantIssuer {
    current: BTreeMap<GrantId, CapabilityGrant>,
    histories: BTreeMap<GrantId, Vec<CapabilityGrant>>,
    revision_hashes: BTreeMap<(GrantId, u32), String>,
    used_nonces: BTreeSet<GrantNonce>,
}

pub(crate) struct GrantDurableParts<'a> {
    pub(crate) histories: &'a BTreeMap<GrantId, Vec<CapabilityGrant>>,
    pub(crate) revision_hashes: &'a BTreeMap<(GrantId, u32), String>,
    pub(crate) used_nonces: &'a BTreeSet<GrantNonce>,
}

// Issuer construction must remain an explicit authority-boundary decision.
#[allow(clippy::new_without_default)]
impl GrantIssuer {
    /// Creates empty issuer state with no ambient or imported authority.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: BTreeMap::new(),
            histories: BTreeMap::new(),
            revision_hashes: BTreeMap::new(),
            used_nonces: BTreeSet::new(),
        }
    }

    /// Returns one current kernel-issued grant by exact identity.
    #[must_use]
    pub fn current(&self, grant_id: &GrantId) -> Option<&CapabilityGrant> {
        self.current.get(grant_id)
    }

    /// Returns the canonical digest of one retained grant revision.
    #[must_use]
    pub fn revision_hash(&self, grant_id: &GrantId, revision: u32) -> Option<&str> {
        self.revision_hashes
            .get(&(grant_id.clone(), revision))
            .map(String::as_str)
    }

    /// Returns the immutable revision history for one exact grant identity.
    #[must_use]
    pub fn history(&self, grant_id: &GrantId) -> Option<&[CapabilityGrant]> {
        self.histories.get(grant_id).map(Vec::as_slice)
    }

    /// Revalidates and atomically consumes one exact operation grant for execution.
    ///
    /// All fallible integrity, policy, arithmetic, and hashing work completes before issuer state
    /// changes. The exclusive borrow makes the current-record check and terminal transition one
    /// deterministic candidate update. The durable authority runtime publishes that update in the
    /// same encrypted checkpoint as the transaction's `grant_consumed` revision before launch.
    pub fn consume_for_execution(
        &mut self,
        grant_id: &GrantId,
        policy: &PolicyEngine,
        context: &PolicyEvaluationContext,
    ) -> Result<GrantConsumptionRecord, GrantConsumeError> {
        let issued = self
            .current
            .get(grant_id)
            .cloned()
            .ok_or(GrantConsumeError::NotFound)?;
        let issued_sha256 = grant_sha256(&issued).map_err(|_| GrantConsumeError::CorruptState)?;
        if self.revision_hash(grant_id, issued.revision) != Some(issued_sha256.as_str()) {
            return Err(GrantConsumeError::CorruptState);
        }

        let decision = policy.evaluate(self, &issued, context);
        if let Some(scope) = decision.denial_scope {
            if issued.schema_version == agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
                && issued.grant_class == GrantClass::Operation
                && issued.status == GrantStatus::Issued
                && issued.use_limit == 1
                && issued.use_count == 0
            {
                let status = if context.now_epoch_ms >= issued.expires_at_epoch_ms {
                    GrantStatus::Expired
                } else {
                    GrantStatus::Invalidated
                };
                self.transition_status(&issued, &issued_sha256, status, context.now_epoch_ms)
                    .map_err(|_| GrantConsumeError::CorruptState)?;
            }
            return Err(GrantConsumeError::PolicyDenied(scope));
        }
        if !decision.allowed {
            return Err(GrantConsumeError::CorruptState);
        }

        let mut consumed = issued;
        let issued_revision = consumed.revision;
        consumed.revision = consumed
            .revision
            .checked_add(1)
            .ok_or(GrantConsumeError::CorruptState)?;
        consumed.use_count = consumed
            .use_count
            .checked_add(1)
            .ok_or(GrantConsumeError::CorruptState)?;
        if consumed.use_count != consumed.use_limit {
            return Err(GrantConsumeError::CorruptState);
        }
        consumed.status = GrantStatus::Consumed;
        let consumed_sha256 =
            grant_sha256(&consumed).map_err(|_| GrantConsumeError::CorruptState)?;
        let record = GrantConsumptionRecord {
            grant_id: consumed.grant_id.clone(),
            issued_revision,
            consumed_revision: consumed.revision,
            issued_grant_sha256: issued_sha256,
            consumed_grant_sha256: consumed_sha256.clone(),
            consumed_at_epoch_ms: context.now_epoch_ms,
            policy_sha256: decision.policy_sha256,
        };

        self.revision_hashes.insert(
            (consumed.grant_id.clone(), consumed.revision),
            consumed_sha256,
        );
        self.histories
            .get_mut(&consumed.grant_id)
            .ok_or(GrantConsumeError::CorruptState)?
            .push(consumed.clone());
        self.current.insert(consumed.grant_id.clone(), consumed);
        Ok(record)
    }

    /// Advances one exact consumed attempt to terminal `uncertain` state.
    ///
    /// The expected consumed digest binds this transition to the one worker attempt that already
    /// consumed the grant. An uncertain result never restores, reissues, or retries authority.
    pub fn mark_execution_uncertain(
        &mut self,
        grant_id: &GrantId,
        expected_consumed_sha256: &str,
        occurred_at_epoch_ms: u64,
    ) -> Result<GrantLifecycleRecord, GrantLifecycleError> {
        if validate_digest(expected_consumed_sha256).is_err() {
            return Err(GrantLifecycleError::RevisionMismatch);
        }
        let consumed = self
            .current
            .get(grant_id)
            .cloned()
            .ok_or(GrantLifecycleError::NotFound)?;
        if consumed.status != GrantStatus::Consumed
            || consumed.grant_class != GrantClass::Operation
            || consumed.use_limit != 1
            || consumed.use_count != 1
            || occurred_at_epoch_ms < consumed.issued_at_epoch_ms
        {
            return Err(GrantLifecycleError::InvalidState);
        }
        let consumed_sha256 =
            grant_sha256(&consumed).map_err(|_| GrantLifecycleError::CorruptState)?;
        if consumed_sha256 != expected_consumed_sha256
            || self.revision_hash(grant_id, consumed.revision) != Some(consumed_sha256.as_str())
        {
            return Err(GrantLifecycleError::RevisionMismatch);
        }
        self.transition_status(
            &consumed,
            &consumed_sha256,
            GrantStatus::Uncertain,
            occurred_at_epoch_ms,
        )
    }

    /// Issues one session-read parent after closed shape and scope validation.
    pub fn issue_session_read(
        &mut self,
        request: SessionReadGrantRequest,
    ) -> Result<CapabilityGrant, GrantIssueError> {
        self.validate_new_identity(&request.grant_id, &request.nonce)?;
        validate_identifier(request.actor_id.as_str())?;
        validate_identifier(request.session_id.as_str())?;
        validate_identifier(request.task_id.as_str())?;
        validate_identifier(request.grant_id.as_str())?;
        validate_identifier(request.nonce.as_str())?;
        validate_digest(&request.preview_sha256)?;
        validate_digest(&request.policy_sha256)?;
        validate_lifetime(request.issued_at_epoch_ms, request.expires_at_epoch_ms)?;
        if request.maximum_derived_operations == 0
            || request.maximum_derived_operations > MAX_SESSION_DERIVATIONS
        {
            return Err(GrantIssueError::InvalidInput);
        }
        validate_scope(&request.targets, &request.excluded_targets)?;
        if request
            .targets
            .iter()
            .chain(&request.excluded_targets)
            .any(|target| target.scope_path().is_none())
        {
            return Err(GrantIssueError::InvalidInput);
        }
        let scope_sha256 = session_scope_sha256(
            &request.targets,
            &request.excluded_targets,
            request.sensitivity,
        )?;
        let target_indexes = (0..request.targets.len())
            .map(|index| u32::try_from(index).map_err(|_| GrantIssueError::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        let grant = CapabilityGrant {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            grant_id: request.grant_id,
            revision: 1,
            grant_class: GrantClass::SessionRead,
            actor_id: request.actor_id,
            approval_id: None,
            session_id: request.session_id,
            task_id: request.task_id,
            action_id: None,
            action_kind: None,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            tool_id: None,
            tool_version: None,
            targets: request.targets,
            excluded_targets: request.excluded_targets,
            sensitivity: request.sensitivity,
            argument_sha256: scope_sha256.clone(),
            preimages: Vec::new(),
            expected_side_effects: vec![GrantSideEffect {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                target_indexes,
                details_sha256: scope_sha256,
            }],
            rollback_description: "No state change is permitted".to_owned(),
            issued_at_epoch_ms: request.issued_at_epoch_ms,
            expires_at_epoch_ms: request.expires_at_epoch_ms,
            nonce: request.nonce,
            use_limit: request.maximum_derived_operations,
            use_count: 0,
            parent_grant_id: None,
            parent_grant_sha256: None,
            preview_sha256: request.preview_sha256,
            policy_sha256: request.policy_sha256,
            status: GrantStatus::Issued,
        };
        self.insert_new(grant.clone())?;
        Ok(grant)
    }

    /// Derives one exact single-use operation grant within a current parent scope.
    pub fn derive_operation(
        &mut self,
        parent_grant_id: &GrantId,
        request: DerivedOperationGrantRequest,
    ) -> Result<CapabilityGrant, GrantIssueError> {
        self.validate_new_identity(&request.grant_id, &request.nonce)?;
        let parent = self
            .current
            .get(parent_grant_id)
            .cloned()
            .ok_or(GrantIssueError::ParentNotFound)?;
        if parent.grant_class != GrantClass::SessionRead {
            return Err(GrantIssueError::InvalidParentClass);
        }
        if parent.status != GrantStatus::Issued || parent.use_count >= parent.use_limit {
            return Err(GrantIssueError::ParentUnavailable);
        }
        if request.issued_at_epoch_ms >= parent.expires_at_epoch_ms {
            return Err(GrantIssueError::ParentExpired);
        }
        if request.policy_sha256 != parent.policy_sha256 {
            return Err(GrantIssueError::PolicyChanged);
        }
        validate_derived_request(&request, &parent)?;
        let parent_revision = parent.revision;
        let parent_sha256 = grant_sha256(&parent)?;
        let child = CapabilityGrant {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            grant_id: request.grant_id,
            revision: 1,
            grant_class: GrantClass::Operation,
            actor_id: parent.actor_id.clone(),
            approval_id: Some(request.approval_id),
            session_id: parent.session_id.clone(),
            task_id: parent.task_id.clone(),
            action_id: Some(request.action_id),
            action_kind: Some(request.action_kind),
            operation: request.operation,
            tool_id: Some(request.tool_id),
            tool_version: Some(request.tool_version),
            targets: request.targets,
            excluded_targets: parent.excluded_targets.clone(),
            sensitivity: parent.sensitivity,
            argument_sha256: request.argument_sha256,
            preimages: request.preimages,
            expected_side_effects: request.expected_side_effects,
            rollback_description: request.rollback_description,
            issued_at_epoch_ms: request.issued_at_epoch_ms,
            expires_at_epoch_ms: request.expires_at_epoch_ms,
            nonce: request.nonce,
            use_limit: 1,
            use_count: 0,
            parent_grant_id: Some(parent.grant_id.clone()),
            parent_grant_sha256: Some(parent_sha256),
            preview_sha256: request.preview_sha256,
            policy_sha256: request.policy_sha256,
            status: GrantStatus::Issued,
        };

        let mut updated_parent = parent;
        updated_parent.revision = updated_parent
            .revision
            .checked_add(1)
            .ok_or(GrantIssueError::ParentUnavailable)?;
        updated_parent.use_count = updated_parent
            .use_count
            .checked_add(1)
            .ok_or(GrantIssueError::ParentUnavailable)?;
        if updated_parent.use_count == updated_parent.use_limit {
            updated_parent.status = GrantStatus::Consumed;
        }
        let updated_parent_hash = grant_sha256(&updated_parent)?;
        let child_hash = grant_sha256(&child)?;

        self.used_nonces.insert(child.nonce.clone());
        self.revision_hashes.insert(
            (updated_parent.grant_id.clone(), updated_parent.revision),
            updated_parent_hash,
        );
        self.revision_hashes
            .insert((child.grant_id.clone(), child.revision), child_hash);
        self.current
            .insert(updated_parent.grant_id.clone(), updated_parent.clone());
        self.current.insert(child.grant_id.clone(), child.clone());
        self.histories
            .get_mut(&updated_parent.grant_id)
            .ok_or(GrantIssueError::ParentUnavailable)?
            .push(updated_parent);
        self.histories
            .insert(child.grant_id.clone(), vec![child.clone()]);
        debug_assert!(
            self.revision_hashes
                .contains_key(&(parent_grant_id.clone(), parent_revision))
        );
        Ok(child)
    }

    fn validate_new_identity(
        &self,
        grant_id: &GrantId,
        nonce: &GrantNonce,
    ) -> Result<(), GrantIssueError> {
        if self.current.contains_key(grant_id) {
            return Err(GrantIssueError::DuplicateGrant);
        }
        if self.used_nonces.contains(nonce) {
            return Err(GrantIssueError::NonceReuse);
        }
        Ok(())
    }

    fn insert_new(&mut self, grant: CapabilityGrant) -> Result<(), GrantIssueError> {
        let digest = grant_sha256(&grant)?;
        self.used_nonces.insert(grant.nonce.clone());
        self.revision_hashes
            .insert((grant.grant_id.clone(), grant.revision), digest);
        self.histories
            .insert(grant.grant_id.clone(), vec![grant.clone()]);
        self.current.insert(grant.grant_id.clone(), grant);
        Ok(())
    }

    fn transition_status(
        &mut self,
        previous: &CapabilityGrant,
        previous_sha256: &str,
        status: GrantStatus,
        occurred_at_epoch_ms: u64,
    ) -> Result<GrantLifecycleRecord, GrantLifecycleError> {
        if self.revision_hash(&previous.grant_id, previous.revision) != Some(previous_sha256) {
            return Err(GrantLifecycleError::CorruptState);
        }
        let mut terminal = previous.clone();
        terminal.revision = terminal
            .revision
            .checked_add(1)
            .ok_or(GrantLifecycleError::CorruptState)?;
        terminal.status = status;
        let terminal_sha256 =
            grant_sha256(&terminal).map_err(|_| GrantLifecycleError::CorruptState)?;
        let record = GrantLifecycleRecord {
            grant_id: terminal.grant_id.clone(),
            previous_revision: previous.revision,
            terminal_revision: terminal.revision,
            previous_grant_sha256: previous_sha256.to_owned(),
            terminal_grant_sha256: terminal_sha256.clone(),
            status,
            occurred_at_epoch_ms,
        };
        self.revision_hashes.insert(
            (terminal.grant_id.clone(), terminal.revision),
            terminal_sha256,
        );
        self.histories
            .get_mut(&terminal.grant_id)
            .ok_or(GrantLifecycleError::CorruptState)?
            .push(terminal.clone());
        self.current.insert(terminal.grant_id.clone(), terminal);
        Ok(record)
    }

    pub(crate) fn durable_parts(&self) -> GrantDurableParts<'_> {
        GrantDurableParts {
            histories: &self.histories,
            revision_hashes: &self.revision_hashes,
            used_nonces: &self.used_nonces,
        }
    }

    pub(crate) fn from_durable_parts(
        histories: BTreeMap<GrantId, Vec<CapabilityGrant>>,
        revision_hashes: BTreeMap<(GrantId, u32), String>,
        used_nonces: BTreeSet<GrantNonce>,
    ) -> Result<Self, GrantLifecycleError> {
        let mut current = BTreeMap::new();
        let mut observed_nonces = BTreeSet::new();
        for (grant_id, history) in &histories {
            if history.is_empty() {
                return Err(GrantLifecycleError::CorruptState);
            }
            for (index, record) in history.iter().enumerate() {
                let expected_revision = u32::try_from(index)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or(GrantLifecycleError::CorruptState)?;
                let digest = grant_sha256(record).map_err(|_| GrantLifecycleError::CorruptState)?;
                if &record.grant_id != grant_id
                    || record.revision != expected_revision
                    || revision_hashes.get(&(grant_id.clone(), record.revision)) != Some(&digest)
                {
                    return Err(GrantLifecycleError::CorruptState);
                }
                observed_nonces.insert(record.nonce.clone());
            }
            let latest = history
                .last()
                .cloned()
                .ok_or(GrantLifecycleError::CorruptState)?;
            current.insert(grant_id.clone(), latest);
        }
        if observed_nonces != used_nonces
            || revision_hashes.len() != histories.values().map(Vec::len).sum::<usize>()
        {
            return Err(GrantLifecycleError::CorruptState);
        }
        Ok(Self {
            current,
            histories,
            revision_hashes,
            used_nonces,
        })
    }
}

fn validate_derived_request(
    request: &DerivedOperationGrantRequest,
    parent: &CapabilityGrant,
) -> Result<(), GrantIssueError> {
    for identifier in [
        request.grant_id.as_str(),
        request.approval_id.as_str(),
        request.action_id.as_str(),
        request.tool_id.as_str(),
        request.nonce.as_str(),
    ] {
        validate_identifier(identifier)?;
    }
    if request.tool_version.is_empty()
        || request.tool_version.len() > MAX_TOOL_VERSION_BYTES
        || !request
            .tool_version
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'.' | b'-' | b'+'))
    {
        return Err(GrantIssueError::InvalidInput);
    }
    for digest in [
        &request.argument_sha256,
        &request.preview_sha256,
        &request.policy_sha256,
    ] {
        validate_digest(digest)?;
    }
    if request.expires_at_epoch_ms <= request.issued_at_epoch_ms
        || request.expires_at_epoch_ms > parent.expires_at_epoch_ms
    {
        return Err(GrantIssueError::InvalidInput);
    }
    validate_scope(&request.targets, &[])?;
    if request
        .targets
        .iter()
        .any(|target| target.workspace_path().is_none())
    {
        return Err(GrantIssueError::InvalidInput);
    }
    if request.targets.iter().any(|target| {
        !parent
            .targets
            .iter()
            .any(|allowed| target_within(target, allowed))
    }) || request.targets.iter().any(|target| {
        parent
            .excluded_targets
            .iter()
            .any(|excluded| target_within(target, excluded))
    }) {
        return Err(GrantIssueError::ScopeBroadened);
    }
    if request.preimages.len() > MAX_TARGETS
        || request.expected_side_effects.is_empty()
        || request.expected_side_effects.len() > MAX_EFFECTS
        || request.rollback_description.is_empty()
        || request.rollback_description.len() > MAX_ROLLBACK_BYTES
    {
        return Err(GrantIssueError::InvalidInput);
    }
    let mut preimage_indexes = BTreeSet::new();
    for preimage in &request.preimages {
        let Some(target_index) = usize::try_from(preimage.target_index)
            .ok()
            .filter(|index| *index < request.targets.len())
        else {
            return Err(GrantIssueError::InvalidInput);
        };
        if !preimage_indexes.insert(preimage.target_index)
            || !preimage.matches_target(preimage.target_index, &request.targets[target_index])
        {
            return Err(GrantIssueError::InvalidInput);
        }
        validate_digest(&preimage.content_sha256)?;
    }
    if request.targets.iter().enumerate().any(|(index, target)| {
        target.preimage().is_some()
            != preimage_indexes.contains(&u32::try_from(index).unwrap_or(u32::MAX))
    }) {
        return Err(GrantIssueError::InvalidInput);
    }
    for effect in &request.expected_side_effects {
        if effect.operation != request.operation
            || effect.target_indexes.is_empty()
            || effect.target_indexes.iter().any(|index| {
                usize::try_from(*index)
                    .ok()
                    .filter(|value| *value < request.targets.len())
                    .is_none()
            })
        {
            return Err(GrantIssueError::InvalidInput);
        }
        validate_digest(&effect.details_sha256)?;
    }
    Ok(())
}

fn validate_scope(
    targets: &[GrantTarget],
    excluded_targets: &[GrantTarget],
) -> Result<(), GrantIssueError> {
    if targets.is_empty() || targets.len() > MAX_TARGETS || excluded_targets.len() > MAX_TARGETS {
        return Err(GrantIssueError::InvalidInput);
    }
    for target in targets.iter().chain(excluded_targets) {
        validate_identifier(target.workspace_id().as_str())?;
        validate_identifier(target.authorization_id().as_str())?;
        validate_identifier(target.adapter_instance_id().as_str())?;
    }
    let workspace = targets[0].workspace_id();
    let authorization = targets[0].authorization_id();
    let adapter = targets[0].adapter_instance_id();
    let platform = targets[0].platform();
    if targets.iter().chain(excluded_targets).any(|target| {
        target.workspace_id() != workspace
            || target.authorization_id() != authorization
            || target.adapter_instance_id() != adapter
            || target.platform() != platform
    }) {
        return Err(GrantIssueError::InvalidInput);
    }
    if excluded_targets.iter().any(|excluded| {
        !targets
            .iter()
            .any(|included| target_within(excluded, included))
    }) {
        return Err(GrantIssueError::InvalidInput);
    }
    Ok(())
}

fn target_within(candidate: &GrantTarget, scope: &GrantTarget) -> bool {
    scope.contains(candidate)
}

fn validate_identifier(value: &str) -> Result<(), GrantIssueError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || value.contains('*')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(GrantIssueError::InvalidInput);
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), GrantIssueError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(GrantIssueError::InvalidInput);
    }
    Ok(())
}

fn validate_lifetime(issued: u64, expires: u64) -> Result<(), GrantIssueError> {
    if expires <= issued || expires - issued > MAX_GRANT_LIFETIME_MS {
        return Err(GrantIssueError::InvalidInput);
    }
    Ok(())
}

fn session_scope_sha256(
    targets: &[GrantTarget],
    excluded_targets: &[GrantTarget],
    sensitivity: DataSensitivity,
) -> Result<String, GrantIssueError> {
    let bytes = serde_json::to_vec(&(targets, excluded_targets, sensitivity))
        .map_err(|_| GrantIssueError::InvalidInput)?;
    Ok(hex_sha256(&bytes))
}

fn grant_sha256(grant: &CapabilityGrant) -> Result<String, GrantIssueError> {
    let bytes = to_canonical_json(grant).map_err(|_| GrantIssueError::InvalidInput)?;
    Ok(hex_sha256(&bytes))
}

fn hex_sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        sync::{Arc, Barrier, Mutex},
        thread,
    };

    use super::{
        DerivedOperationGrantRequest, GrantConsumeError, GrantIssueError, GrantIssuer,
        GrantLifecycleError, SessionReadGrantRequest,
    };
    use crate::policy::{
        PolicyDenialScope, PolicyDocument, PolicyEngine, PolicyEvaluationContext, ScopeRules,
        ToolPolicyBinding,
    };
    use crate::test_target::{preimage, scope, target, target_for};
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, ApprovalId, CapabilityGrant, DataSensitivity, GrantClass,
        GrantId, GrantNonce, GrantOperation, GrantSideEffect, GrantStatus, OperationBinding,
        SessionId, TaskId, ToolId,
    };

    fn session_request() -> SessionReadGrantRequest {
        SessionReadGrantRequest {
            grant_id: GrantId::from_raw("grant-session-0001"),
            actor_id: ActorId::from_raw("actor-local-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            targets: vec![scope(&[])],
            excluded_targets: vec![scope(&["private"])],
            sensitivity: DataSensitivity::Ephemeral,
            issued_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 61_000,
            nonce: GrantNonce::from_raw("nonce-session-0001"),
            maximum_derived_operations: 2,
            preview_sha256: "1".repeat(64),
            policy_sha256: "2".repeat(64),
        }
    }

    fn operation_request(id: &str, nonce: &str, path: &[&str]) -> DerivedOperationGrantRequest {
        let target = target(path);
        DerivedOperationGrantRequest {
            grant_id: GrantId::from_raw(id),
            approval_id: ApprovalId::from_raw(format!("approval-{id}")),
            action_id: ActionId::from_raw(format!("action-{id}")),
            action_kind: ActionKind::DeterministicTool,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
            targets: vec![target.clone()],
            argument_sha256: "3".repeat(64),
            preimages: vec![preimage(0, &target)],
            expected_side_effects: vec![GrantSideEffect {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                target_indexes: vec![0],
                details_sha256: "5".repeat(64),
            }],
            rollback_description: "No state change is permitted".to_owned(),
            issued_at_epoch_ms: 2_000,
            expires_at_epoch_ms: 30_000,
            nonce: GrantNonce::from_raw(nonce),
            preview_sha256: "6".repeat(64),
            policy_sha256: "2".repeat(64),
        }
    }

    fn rules<T: Ord>(value: T) -> ScopeRules<T> {
        ScopeRules {
            allowed: BTreeSet::from([value]),
            denied: BTreeSet::new(),
        }
    }

    fn policy_document(revision: u32) -> PolicyDocument {
        PolicyDocument {
            schema_version: 1,
            revision,
            actors: rules(ActorId::from_raw("actor-local-0001")),
            tasks: rules(TaskId::from_raw("task-0001")),
            actions: rules(ActionId::from_raw("action-grant-operation-consume")),
            tools: rules(ToolPolicyBinding {
                tool_id: ToolId::from_raw("fixture.read"),
                tool_version: "1.0.0".to_owned(),
            }),
            operations: rules(OperationBinding::new(GrantOperation::WorkspaceRead)),
            targets: rules(target(&["src"])),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        }
    }

    fn issued_for_consumption() -> (GrantIssuer, PolicyEngine, CapabilityGrant) {
        let policy = PolicyEngine::new(policy_document(1)).expect("policy must build");
        let mut issuer = GrantIssuer::new();
        let mut parent_request = session_request();
        parent_request.maximum_derived_operations = 1;
        parent_request.policy_sha256 = policy.policy_sha256().to_owned();
        let parent = issuer
            .issue_session_read(parent_request)
            .expect("parent must issue");
        let mut child_request = operation_request(
            "grant-operation-consume",
            "nonce-operation-consume",
            &["src"],
        );
        child_request.policy_sha256 = policy.policy_sha256().to_owned();
        let child = issuer
            .derive_operation(&parent.grant_id, child_request)
            .expect("child must derive");
        (issuer, policy, child)
    }

    fn consumption_context(grant: &CapabilityGrant) -> PolicyEvaluationContext {
        PolicyEvaluationContext {
            actor_id: grant.actor_id.clone(),
            session_id: grant.session_id.clone(),
            task_id: grant.task_id.clone(),
            action_id: grant.action_id.clone().expect("operation action"),
            action_kind: grant.action_kind.expect("operation action kind"),
            tool_id: grant.tool_id.clone().expect("operation tool"),
            tool_version: grant.tool_version.clone().expect("operation tool version"),
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: 3_000,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        }
    }

    #[test]
    fn session_parent_and_children_are_exact_bounded_and_single_use() {
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(session_request())
            .expect("session scope must issue");
        assert_eq!(parent.grant_class, GrantClass::SessionRead);
        assert_eq!(
            parent.operation,
            OperationBinding::new(GrantOperation::WorkspaceRead)
        );
        assert_eq!(parent.use_limit, 2);
        assert_eq!(parent.use_count, 0);
        assert!(parent.action_id.is_none());
        assert!(parent.tool_id.is_none());
        let parent_hash = issuer
            .revision_hash(&parent.grant_id, 1)
            .expect("parent revision hash")
            .to_owned();

        let child = issuer
            .derive_operation(
                &parent.grant_id,
                operation_request("grant-operation-0001", "nonce-operation-0001", &["src"]),
            )
            .expect("child must derive");
        assert_eq!(child.grant_class, GrantClass::Operation);
        assert_eq!(child.use_limit, 1);
        assert_eq!(child.use_count, 0);
        assert_eq!(child.status, GrantStatus::Issued);
        assert_eq!(child.parent_grant_id, Some(parent.grant_id.clone()));
        assert_eq!(
            child.parent_grant_sha256.as_deref(),
            Some(parent_hash.as_str())
        );
        assert_eq!(child.excluded_targets, parent.excluded_targets);
        assert_eq!(child.sensitivity, parent.sensitivity);
        assert_eq!(
            issuer
                .current(&parent.grant_id)
                .expect("current parent")
                .use_count,
            1
        );

        issuer
            .derive_operation(
                &parent.grant_id,
                operation_request("grant-operation-0002", "nonce-operation-0002", &["tests"]),
            )
            .expect("second bounded child must derive");
        let exhausted = issuer.current(&parent.grant_id).expect("current parent");
        assert_eq!(exhausted.use_count, 2);
        assert_eq!(exhausted.status, GrantStatus::Consumed);
        let error = issuer
            .derive_operation(
                &parent.grant_id,
                operation_request("grant-operation-0003", "nonce-operation-0003", &["docs"]),
            )
            .expect_err("exhausted parent must fail closed");
        assert_eq!(error, GrantIssueError::ParentUnavailable);
    }

    #[test]
    fn duplicate_identity_nonce_invalid_lifetime_and_scope_fail_before_state_change() {
        let mut invalid_issuer = GrantIssuer::new();
        let mut invalid_lifetime = session_request();
        invalid_lifetime.expires_at_epoch_ms = invalid_lifetime.issued_at_epoch_ms;
        let error = invalid_issuer
            .issue_session_read(invalid_lifetime)
            .expect_err("empty lifetime must fail");
        assert_eq!(error, GrantIssueError::InvalidInput);

        let malformed = serde_json::json!({
            "target_kind": "workspace_scope",
            "path": {"workspace_id": "workspace-0001", "components": ["**"]},
            "authorization_id": "authorization-0001",
            "adapter_instance_id": "adapter-0001",
            "platform": "deterministic_fake"
        });
        assert!(
            serde_json::from_value::<agentmage_kernel_contracts::GrantTarget>(malformed).is_err()
        );
        assert!(invalid_issuer.current.is_empty());

        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(session_request())
            .expect("session scope must issue");
        let original_parent = issuer
            .current(&parent.grant_id)
            .expect("parent must exist")
            .clone();

        let duplicate = issuer
            .issue_session_read(session_request())
            .expect_err("duplicate parent must fail");
        assert_eq!(duplicate, GrantIssueError::DuplicateGrant);

        let mut reused =
            operation_request("grant-operation-reused", "nonce-session-0001", &["src"]);
        let error = issuer
            .derive_operation(&parent.grant_id, reused.clone())
            .expect_err("reused nonce must fail");
        assert_eq!(error, GrantIssueError::NonceReuse);

        reused.nonce = GrantNonce::from_raw("nonce-operation-outside");
        reused.targets = vec![target(&["private", "secret.txt"])];
        let error = issuer
            .derive_operation(&parent.grant_id, reused.clone())
            .expect_err("excluded child must fail");
        assert_eq!(error, GrantIssueError::ScopeBroadened);

        reused.targets = vec![target_for(
            &["src"],
            "workspace-other",
            "authorization-other",
            "adapter-0001",
        )];
        let error = issuer
            .derive_operation(&parent.grant_id, reused.clone())
            .expect_err("other workspace must fail");
        assert_eq!(error, GrantIssueError::ScopeBroadened);

        reused.targets = vec![target(&["src"])];
        reused.policy_sha256 = "7".repeat(64);
        let error = issuer
            .derive_operation(&parent.grant_id, reused)
            .expect_err("changed policy must fail");
        assert_eq!(error, GrantIssueError::PolicyChanged);
        assert_eq!(issuer.current(&parent.grant_id), Some(&original_parent));
    }

    #[test]
    fn child_derivation_rejects_siblings_exclusions_and_scope_aggregation() {
        let mut issuer = GrantIssuer::new();
        let mut parent_request = session_request();
        parent_request.targets = vec![scope(&["src"])];
        parent_request.excluded_targets = vec![scope(&["src", "private"])];
        let parent = issuer
            .issue_session_read(parent_request)
            .expect("narrow parent must issue");
        let original_parent = issuer.current(&parent.grant_id).expect("parent").clone();

        for (id, nonce, targets) in [
            (
                "grant-child-sibling",
                "nonce-child-sibling",
                vec![target(&["tests"])],
            ),
            (
                "grant-child-excluded",
                "nonce-child-excluded",
                vec![target(&["src", "private", "secret.txt"])],
            ),
            (
                "grant-child-aggregate",
                "nonce-child-aggregate",
                vec![target(&["src", "allowed.txt"]), target(&["docs"])],
            ),
        ] {
            let mut request = operation_request(id, nonce, &["src"]);
            request.targets = targets;
            assert_eq!(
                issuer
                    .derive_operation(&parent.grant_id, request)
                    .expect_err("broadened child must fail"),
                GrantIssueError::ScopeBroadened
            );
            assert_eq!(issuer.current(&parent.grant_id), Some(&original_parent));
        }

        issuer
            .derive_operation(
                &parent.grant_id,
                operation_request(
                    "grant-child-narrow",
                    "nonce-child-narrow",
                    &["src", "allowed.txt"],
                ),
            )
            .expect("narrow child must derive");
    }

    #[test]
    fn final_validation_consumes_once_and_retains_both_revision_hashes() {
        let (mut issuer, policy, grant) = issued_for_consumption();
        let issued_hash = issuer
            .revision_hash(&grant.grant_id, grant.revision)
            .expect("issued revision hash")
            .to_owned();
        let context = consumption_context(&grant);

        let record = issuer
            .consume_for_execution(&grant.grant_id, &policy, &context)
            .expect("exact grant must consume");
        assert_eq!(record.grant_id, grant.grant_id);
        assert_eq!(record.issued_revision, 1);
        assert_eq!(record.consumed_revision, 2);
        assert_eq!(record.issued_grant_sha256, issued_hash);
        assert_eq!(record.consumed_at_epoch_ms, context.now_epoch_ms);
        assert_eq!(record.policy_sha256, policy.policy_sha256());
        assert_eq!(
            issuer.revision_hash(&grant.grant_id, 1),
            Some(record.issued_grant_sha256.as_str())
        );
        assert_eq!(
            issuer.revision_hash(&grant.grant_id, 2),
            Some(record.consumed_grant_sha256.as_str())
        );
        let consumed = issuer.current(&grant.grant_id).expect("consumed grant");
        assert_eq!(consumed.revision, 2);
        assert_eq!(consumed.use_count, 1);
        assert_eq!(consumed.use_limit, 1);
        assert_eq!(consumed.status, GrantStatus::Consumed);

        assert_eq!(
            issuer
                .consume_for_execution(&grant.grant_id, &policy, &context)
                .expect_err("replay must fail"),
            GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant)
        );
    }

    #[test]
    fn stale_context_policy_and_expiry_terminalize_without_consumption() {
        let (mut issuer, policy, grant) = issued_for_consumption();
        let mut changed_arguments = consumption_context(&grant);
        changed_arguments.argument_sha256 = "a".repeat(64);
        assert_eq!(
            issuer
                .consume_for_execution(&grant.grant_id, &policy, &changed_arguments)
                .expect_err("changed arguments must fail"),
            GrantConsumeError::PolicyDenied(PolicyDenialScope::Argument)
        );
        let invalidated = issuer.current(&grant.grant_id).expect("invalidated grant");
        assert_eq!(invalidated.status, GrantStatus::Invalidated);
        assert_eq!(invalidated.revision, 2);
        assert_eq!(invalidated.use_count, 0);
        assert_eq!(
            issuer
                .consume_for_execution(&grant.grant_id, &policy, &consumption_context(&grant))
                .expect_err("invalidated grant must not recover"),
            GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant)
        );

        let (mut issuer, policy, grant) = issued_for_consumption();
        let mut stale_preimage = consumption_context(&grant);
        stale_preimage.preimages[0].content_sha256 = "d".repeat(64);
        assert_eq!(
            issuer
                .consume_for_execution(&grant.grant_id, &policy, &stale_preimage)
                .expect_err("stale preimage must fail"),
            GrantConsumeError::PolicyDenied(PolicyDenialScope::Preimage)
        );
        let invalidated = issuer.current(&grant.grant_id).expect("invalidated grant");
        assert_eq!(invalidated.status, GrantStatus::Invalidated);
        assert_eq!(invalidated.revision, 2);
        assert_eq!(invalidated.use_count, 0);

        let (mut issuer, _original_policy, grant) = issued_for_consumption();
        let changed_policy = PolicyEngine::new(policy_document(2)).expect("policy must build");
        assert_eq!(
            issuer
                .consume_for_execution(
                    &grant.grant_id,
                    &changed_policy,
                    &consumption_context(&grant),
                )
                .expect_err("changed policy must fail"),
            GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant)
        );
        let invalidated = issuer.current(&grant.grant_id).expect("invalidated grant");
        assert_eq!(invalidated.status, GrantStatus::Invalidated);
        assert_eq!(invalidated.revision, 2);
        assert_eq!(invalidated.use_count, 0);

        let (mut issuer, policy, grant) = issued_for_consumption();
        let mut expired = consumption_context(&grant);
        expired.now_epoch_ms = grant.expires_at_epoch_ms;
        assert_eq!(
            issuer
                .consume_for_execution(&grant.grant_id, &policy, &expired)
                .expect_err("expired grant must fail"),
            GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant)
        );
        let expired_grant = issuer.current(&grant.grant_id).expect("expired grant");
        assert_eq!(expired_grant.status, GrantStatus::Expired);
        assert_eq!(expired_grant.revision, 2);
        assert_eq!(expired_grant.use_count, 0);

        let (mut issuer, policy, grant) = issued_for_consumption();
        let original = issuer
            .current(&grant.grant_id)
            .expect("issued grant")
            .clone();
        let key = (grant.grant_id.clone(), grant.revision);
        issuer.revision_hashes.insert(key, "f".repeat(64));
        assert_eq!(
            issuer
                .consume_for_execution(&grant.grant_id, &policy, &consumption_context(&grant))
                .expect_err("corrupt retained hash must fail"),
            GrantConsumeError::CorruptState
        );
        assert_eq!(issuer.current(&grant.grant_id), Some(&original));
    }

    #[test]
    fn uncertain_attempt_is_bound_to_consumed_revision_and_never_reusable() {
        let (mut issuer, policy, grant) = issued_for_consumption();
        let context = consumption_context(&grant);
        let consumed = issuer
            .consume_for_execution(&grant.grant_id, &policy, &context)
            .expect("grant must consume");

        assert_eq!(
            issuer.mark_execution_uncertain(&grant.grant_id, &"f".repeat(64), 4_000),
            Err(GrantLifecycleError::RevisionMismatch)
        );
        assert_eq!(
            issuer
                .current(&grant.grant_id)
                .expect("grant remains consumed")
                .status,
            GrantStatus::Consumed
        );

        let uncertain = issuer
            .mark_execution_uncertain(&grant.grant_id, &consumed.consumed_grant_sha256, 4_000)
            .expect("exact consumed attempt must become uncertain");
        assert_eq!(uncertain.previous_revision, 2);
        assert_eq!(uncertain.terminal_revision, 3);
        assert_eq!(uncertain.status, GrantStatus::Uncertain);
        assert_eq!(uncertain.occurred_at_epoch_ms, 4_000);
        let retained = issuer.current(&grant.grant_id).expect("uncertain grant");
        assert_eq!(retained.status, GrantStatus::Uncertain);
        assert_eq!(retained.revision, 3);
        assert_eq!(retained.use_count, 1);

        assert_eq!(
            issuer
                .consume_for_execution(&grant.grant_id, &policy, &context)
                .expect_err("uncertain grant must never retry"),
            GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant)
        );
        assert_eq!(
            issuer.mark_execution_uncertain(
                &grant.grant_id,
                &consumed.consumed_grant_sha256,
                5_000,
            ),
            Err(GrantLifecycleError::InvalidState)
        );
        assert_eq!(
            issuer
                .current(&grant.grant_id)
                .expect("uncertain grant remains terminal")
                .revision,
            3
        );
    }

    #[test]
    fn two_racing_consumers_produce_exactly_one_terminal_transition() {
        let (issuer, policy, grant) = issued_for_consumption();
        let issuer = Arc::new(Mutex::new(issuer));
        let policy = Arc::new(policy);
        let barrier = Arc::new(Barrier::new(3));
        let handles = (0..2)
            .map(|_| {
                let issuer = Arc::clone(&issuer);
                let policy = Arc::clone(&policy);
                let barrier = Arc::clone(&barrier);
                let grant_id = grant.grant_id.clone();
                let context = consumption_context(&grant);
                thread::spawn(move || {
                    barrier.wait();
                    issuer
                        .lock()
                        .expect("issuer lock must not be poisoned")
                        .consume_for_execution(&grant_id, &policy, &context)
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let outcomes = handles
            .into_iter()
            .map(|handle| handle.join().expect("consumer must finish"))
            .collect::<Vec<_>>();
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| {
                    matches!(
                        outcome,
                        Err(GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant))
                    )
                })
                .count(),
            1
        );
        let issuer = issuer.lock().expect("issuer lock must not be poisoned");
        let consumed = issuer
            .current(&grant.grant_id)
            .expect("grant remains retained");
        assert_eq!(consumed.revision, 2);
        assert_eq!(consumed.use_count, 1);
        assert_eq!(consumed.status, GrantStatus::Consumed);
    }
}
