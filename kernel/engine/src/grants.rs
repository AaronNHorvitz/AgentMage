//! Kernel-only issuance of session-read and derived operation grants.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, CapabilityGrant, DataSensitivity, GrantClass, GrantId,
    GrantNonce, GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget,
    SessionId, TaskId, ToolId, to_canonical_json,
};
use sha2::{Digest, Sha256};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_COMPONENTS: usize = 64;
const MAX_PATH_COMPONENT_BYTES: usize = 255;
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
    /// Exact action receiving proposed authority.
    pub action_id: ActionId,
    /// Exact descriptive class of the action.
    pub action_kind: ActionKind,
    /// Closed operation class requested by the action.
    pub operation: GrantOperation,
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

/// In-memory kernel issuer used before durable grant storage is introduced.
///
/// Authority exists only when a candidate exactly matches this issuer's retained record.
/// Publicly constructed `CapabilityGrant` values are not inserted implicitly.
#[derive(Debug)]
pub struct GrantIssuer {
    current: BTreeMap<GrantId, CapabilityGrant>,
    revision_hashes: BTreeMap<(GrantId, u32), String>,
    used_nonces: BTreeSet<GrantNonce>,
}

// Issuer construction must remain an explicit authority-boundary decision.
#[allow(clippy::new_without_default)]
impl GrantIssuer {
    /// Creates empty issuer state with no ambient or imported authority.
    #[must_use]
    pub fn new() -> Self {
        Self {
            current: BTreeMap::new(),
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
            session_id: request.session_id,
            task_id: request.task_id,
            action_id: None,
            action_kind: None,
            operation: GrantOperation::WorkspaceRead,
            tool_id: None,
            tool_version: None,
            targets: request.targets,
            excluded_targets: request.excluded_targets,
            sensitivity: request.sensitivity,
            argument_sha256: scope_sha256.clone(),
            preimages: Vec::new(),
            expected_side_effects: vec![GrantSideEffect {
                operation: GrantOperation::WorkspaceRead,
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
            .insert(updated_parent.grant_id.clone(), updated_parent);
        self.current.insert(child.grant_id.clone(), child.clone());
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
        self.current.insert(grant.grant_id.clone(), grant);
        Ok(())
    }
}

fn validate_derived_request(
    request: &DerivedOperationGrantRequest,
    parent: &CapabilityGrant,
) -> Result<(), GrantIssueError> {
    for identifier in [
        request.grant_id.as_str(),
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
        if usize::try_from(preimage.target_index)
            .ok()
            .filter(|index| *index < request.targets.len())
            .is_none()
            || !preimage_indexes.insert(preimage.target_index)
        {
            return Err(GrantIssueError::InvalidInput);
        }
        validate_digest(&preimage.content_sha256)?;
        if preimage
            .observed_revision
            .as_ref()
            .is_some_and(|revision| revision.is_empty() || revision.len() > MAX_IDENTIFIER_BYTES)
        {
            return Err(GrantIssueError::InvalidInput);
        }
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
        validate_identifier(target.workspace_id.as_str())?;
        if target.path_components.len() > MAX_PATH_COMPONENTS {
            return Err(GrantIssueError::InvalidInput);
        }
        for component in &target.path_components {
            if component.is_empty()
                || component.len() > MAX_PATH_COMPONENT_BYTES
                || matches!(component.as_str(), "." | ".." | "*" | "**")
                || component
                    .bytes()
                    .any(|value| matches!(value, 0 | b'/' | b'\\'))
            {
                return Err(GrantIssueError::InvalidInput);
            }
        }
    }
    let workspace = targets[0].workspace_id.as_str();
    if targets
        .iter()
        .chain(excluded_targets)
        .any(|target| target.workspace_id.as_str() != workspace)
    {
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
    candidate.workspace_id == scope.workspace_id
        && candidate
            .path_components
            .starts_with(&scope.path_components)
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
    use super::{
        DerivedOperationGrantRequest, GrantIssueError, GrantIssuer, SessionReadGrantRequest,
    };
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, DataSensitivity, GrantClass, GrantId, GrantNonce,
        GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, SessionId,
        TaskId, ToolId, WorkspaceId,
    };

    fn target(path: &[&str]) -> GrantTarget {
        GrantTarget {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            path_components: path.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn session_request() -> SessionReadGrantRequest {
        SessionReadGrantRequest {
            grant_id: GrantId::from_raw("grant-session-0001"),
            actor_id: ActorId::from_raw("actor-local-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            targets: vec![target(&[])],
            excluded_targets: vec![target(&["private"])],
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
        DerivedOperationGrantRequest {
            grant_id: GrantId::from_raw(id),
            action_id: ActionId::from_raw(format!("action-{id}")),
            action_kind: ActionKind::DeterministicTool,
            operation: GrantOperation::WorkspaceRead,
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
            targets: vec![target(path)],
            argument_sha256: "3".repeat(64),
            preimages: vec![GrantPreimage {
                target_index: 0,
                content_sha256: "4".repeat(64),
                observed_revision: Some("fixture-v1".to_owned()),
            }],
            expected_side_effects: vec![GrantSideEffect {
                operation: GrantOperation::WorkspaceRead,
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

    #[test]
    fn session_parent_and_children_are_exact_bounded_and_single_use() {
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(session_request())
            .expect("session scope must issue");
        assert_eq!(parent.grant_class, GrantClass::SessionRead);
        assert_eq!(parent.operation, GrantOperation::WorkspaceRead);
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

        let mut invalid_scope = session_request();
        invalid_scope.grant_id = GrantId::from_raw("grant-session-wildcard");
        invalid_scope.nonce = GrantNonce::from_raw("nonce-session-wildcard");
        invalid_scope.targets[0].path_components = vec!["**".to_owned()];
        let error = invalid_issuer
            .issue_session_read(invalid_scope)
            .expect_err("wildcard scope must fail");
        assert_eq!(error, GrantIssueError::InvalidInput);
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

        reused.targets = vec![GrantTarget {
            workspace_id: WorkspaceId::from_raw("workspace-other"),
            path_components: vec!["src".to_owned()],
        }];
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
}
