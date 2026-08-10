//! Capability-grant authority contracts.

use crate::{
    ActionId, ActionKind, ActorId, GrantId, GrantNonce, SessionId, TaskId, ToolId, WorkspaceId,
};

/// Closed operation class authorized by one exact grant.
///
/// There is deliberately no wildcard, custom, or approve-all variant. Policy may deny
/// any represented operation, and v0.1 admits only separately approved read behavior.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum GrantOperation {
    /// Observe content beneath one approved workspace.
    WorkspaceRead,
    /// Change exact workspace content in a later controlled-write release.
    WorkspaceWrite,
    /// Delete exact workspace content in a later release.
    WorkspaceDelete,
    /// Execute one exact command in a later release.
    CommandExecute,
    /// Contact one exact network destination in a separately approved flow.
    NetworkAccess,
    /// Create one exact source-control commit in a later release.
    GitCommit,
    /// Push one exact source-control state in a later release.
    GitPush,
    /// Publish one exact artifact in a later release.
    Publish,
    /// Send one exact message in a later release.
    Send,
    /// Upload one exact artifact in a later release.
    Upload,
    /// Deploy one exact artifact in a later release.
    Deploy,
    /// Read one exact database scope in a later capability.
    DatabaseRead,
    /// Change one exact database scope in a later capability.
    DatabaseWrite,
    /// Access one exact credential through a separately approved provider.
    CredentialAccess,
    /// Invoke one exact local model profile without giving the model operation authority.
    ModelInference,
}

/// Candidate workspace-relative target bound into a grant.
///
/// Components make ambient absolute-path authority structurally absent. The Sprint 6 path
/// boundary performs canonical component, platform, symlink, and file-identity validation
/// before any grant containing this candidate can become current.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantTarget {
    /// Approved workspace identity that owns the target.
    pub workspace_id: WorkspaceId,
    /// Ordered candidate relative-path components.
    pub path_components: Vec<String>,
}

/// Exact observed target state required immediately before an operation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantPreimage {
    /// Zero-based index into the grant's target sequence.
    pub target_index: u32,
    /// Lowercase SHA-256 digest of the exact observed target state.
    pub content_sha256: String,
    /// Optional stable object revision or file identity observed with the digest.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub observed_revision: Option<String>,
}

/// One exact side effect included in the user-confirmed preview.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantSideEffect {
    /// Closed effect class; policy must agree with the grant operation.
    pub operation: GrantOperation,
    /// Ordered target indexes affected by this effect.
    pub target_indexes: Vec<u32>,
    /// Lowercase SHA-256 digest of canonical effect details shown in the preview.
    pub details_sha256: String,
}

/// Lifecycle state of one grant record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantStatus {
    /// The grant was issued and may be considered by the policy engine.
    Issued,
    /// The grant was consumed by its one admitted terminal attempt.
    Consumed,
    /// The grant was explicitly revoked before use.
    Revoked,
    /// The grant passed its exact expiration time before use.
    Expired,
    /// A bound identity, policy, scope, preview, or preimage became stale.
    Invalidated,
    /// An attempted effect cannot be reconciled safely and may never be retried.
    Uncertain,
}

/// Versioned, exact authority candidate for one bounded operation.
///
/// This contract is the only type allowed to carry operation authority, but parsing or
/// constructing one does not make it valid. The kernel policy engine must validate every
/// field and atomically consume current authority immediately before a concrete executor.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGrant {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable grant identity.
    pub grant_id: GrantId,
    /// Monotonically increasing immutable grant-state revision.
    pub revision: u32,
    /// Exact local actor identity whose decision created the authority.
    pub actor_id: ActorId,
    /// Owning local session.
    pub session_id: SessionId,
    /// Exact user-directed task.
    pub task_id: TaskId,
    /// Exact action identity for an operation grant; absent only for a session read parent.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub action_id: Option<ActionId>,
    /// Exact descriptive action class paired with `action_id`.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub action_kind: Option<ActionKind>,
    /// Closed operation class.
    pub operation: GrantOperation,
    /// Exact tool identity for an operation grant; absent only for a session read parent.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub tool_id: Option<ToolId>,
    /// Exact tool-contract version paired with `tool_id`.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub tool_version: Option<String>,
    /// Ordered candidate workspace-relative targets.
    pub targets: Vec<GrantTarget>,
    /// Lowercase SHA-256 digest of canonical operation arguments.
    pub argument_sha256: String,
    /// Exact target states required immediately before execution.
    pub preimages: Vec<GrantPreimage>,
    /// Exact expected effects included in the confirmed preview.
    pub expected_side_effects: Vec<GrantSideEffect>,
    /// Bounded recovery or rollback description shown before approval.
    pub rollback_description: String,
    /// UTC Unix epoch millisecond at which the kernel issued the grant.
    pub issued_at_epoch_ms: u64,
    /// Exact UTC Unix epoch millisecond after which the grant is invalid.
    pub expires_at_epoch_ms: u64,
    /// Exact anti-replay nonce.
    pub nonce: GrantNonce,
    /// Maximum successful admission count; operation grants must later require one.
    pub use_limit: u32,
    /// Number of uses already atomically consumed in this state revision.
    pub use_count: u32,
    /// Optional parent grant identity for derived authority.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub parent_grant_id: Option<GrantId>,
    /// Optional digest of the exact parent grant revision used for derivation.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub parent_grant_sha256: Option<String>,
    /// Digest of the exact user-confirmed preview.
    pub preview_sha256: String,
    /// Digest of the exact policy revision used when authority was issued.
    pub policy_sha256: String,
    /// Current grant lifecycle state.
    pub status: GrantStatus,
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityGrant, GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget,
    };
    use crate::{
        ActionId, ActionKind, ActorId, GrantId, GrantNonce, SessionId, TaskId, ToolId, WorkspaceId,
    };

    fn grant() -> CapabilityGrant {
        CapabilityGrant {
            schema_version: 1,
            grant_id: GrantId::from_raw("grant-0001"),
            revision: 1,
            actor_id: ActorId::from_raw("actor-local-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: Some(ActionId::from_raw("action-0001")),
            action_kind: Some(ActionKind::DeterministicTool),
            operation: GrantOperation::WorkspaceRead,
            tool_id: Some(ToolId::from_raw("fixture.read")),
            tool_version: Some("1.0.0".to_owned()),
            targets: vec![GrantTarget {
                workspace_id: WorkspaceId::from_raw("workspace-0001"),
                path_components: vec!["fixtures".to_owned(), "input.txt".to_owned()],
            }],
            argument_sha256: "1".repeat(64),
            preimages: vec![GrantPreimage {
                target_index: 0,
                content_sha256: "2".repeat(64),
                observed_revision: Some("fixture-v1".to_owned()),
            }],
            expected_side_effects: vec![GrantSideEffect {
                operation: GrantOperation::WorkspaceRead,
                target_indexes: vec![0],
                details_sha256: "3".repeat(64),
            }],
            rollback_description: "No state change is permitted".to_owned(),
            issued_at_epoch_ms: 1_786_320_000_000,
            expires_at_epoch_ms: 1_786_320_060_000,
            nonce: GrantNonce::from_raw("nonce-0001"),
            use_limit: 1,
            use_count: 0,
            parent_grant_id: Some(GrantId::from_raw("grant-parent-0001")),
            parent_grant_sha256: Some("4".repeat(64)),
            preview_sha256: "5".repeat(64),
            policy_sha256: "6".repeat(64),
            status: GrantStatus::Issued,
        }
    }

    #[test]
    fn schema_binds_exact_authority_fields_without_a_wildcard_operation() {
        let grant = grant();
        assert_eq!(grant.use_limit, 1);
        assert_eq!(grant.use_count, 0);
        assert_eq!(grant.targets[0].path_components, ["fixtures", "input.txt"]);
        assert_eq!(grant.operation, GrantOperation::WorkspaceRead);
        assert_eq!(grant.status, GrantStatus::Issued);
    }

    #[test]
    fn every_top_level_key_is_required_and_unknown_authority_shapes_fail_closed() {
        let grant = grant();
        let value = serde_json::to_value(&grant).expect("grant fixture must encode");
        let object = value.as_object().expect("grant must be an object");
        assert_eq!(object.len(), 26);

        for key in object.keys() {
            let mut candidate = value.clone();
            candidate
                .as_object_mut()
                .expect("grant must be an object")
                .remove(key);
            let bytes = serde_json::to_vec(&candidate).expect("candidate must encode");
            let error = crate::from_json::<CapabilityGrant>(&bytes)
                .expect_err("missing grant key must fail closed");
            assert_eq!(error.code, "contract.field.missing", "missing key: {key}");
        }

        let mut wildcard = value.clone();
        wildcard
            .as_object_mut()
            .expect("grant must be an object")
            .insert("operation".to_owned(), serde_json::json!("all"));
        let error = crate::from_json::<CapabilityGrant>(
            &serde_json::to_vec(&wildcard).expect("wildcard candidate must encode"),
        )
        .expect_err("wildcard operation must fail closed");
        assert_eq!(error.code, "contract.value.unsupported");

        let mut nested = value;
        nested["targets"][0]["absolute_path"] = serde_json::json!("/private/fixture");
        let error = crate::from_json::<CapabilityGrant>(
            &serde_json::to_vec(&nested).expect("nested candidate must encode"),
        )
        .expect_err("unknown target authority must fail closed");
        assert_eq!(error.code, "contract.field.unknown");
    }
}
