//! Capability-grant authority contracts.

use crate::{
    ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, AuthorizedWorkspaceHandle,
    DataSensitivity, FilePreimage, GrantId, GrantNonce, HeldWorkspaceObject, OperationBinding,
    PathPlatform, SessionId, TaskId, ToolId, WorkspaceAuthorizationId, WorkspaceId,
    WorkspaceObjectIdentity, WorkspaceObjectKind, WorkspacePath, WorkspacePathComponent,
    WorkspaceScopePath,
};

/// Authority role assigned to one grant record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantClass {
    /// Parent scope created from an explicit workspace-selection decision.
    SessionRead,
    /// Exact single-use operation derived within a current parent scope.
    Operation,
}

/// Stable, content-free reason a grant target cannot be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantTargetError {
    /// The path, authorization, adapter, platform, object, or preimage binding is inconsistent.
    InvalidBinding,
}

impl std::fmt::Display for GrantTargetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("grant.target.invalid_binding")
    }
}

impl std::error::Error for GrantTargetError {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(tag = "target_kind", rename_all = "snake_case", deny_unknown_fields)]
enum GrantTargetBinding {
    WorkspaceScope {
        path: WorkspaceScopePath,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        platform: PathPlatform,
    },
    HeldObject {
        path: WorkspacePath,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        platform: PathPlatform,
        object_kind: WorkspaceObjectKind,
        object_identity: WorkspaceObjectIdentity,
        #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
        preimage: Option<FilePreimage>,
    },
}

/// Canonical authorization-bound scope or exact descriptor-held object bound into a grant.
///
/// Fields are private and deserialization revalidates cross-field invariants. Session parents
/// carry only workspace scopes. Derived operation grants carry only exact held objects, so no
/// operation target can contain raw path strings or omit platform identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct GrantTarget(GrantTargetBinding);

impl GrantTarget {
    /// Binds one canonical root or subtree scope to an authorized workspace handle.
    pub fn workspace_scope(
        workspace: &impl AuthorizedWorkspaceHandle,
        path: WorkspaceScopePath,
    ) -> Result<Self, GrantTargetError> {
        if path.workspace_id() != workspace.workspace_id() {
            return Err(GrantTargetError::InvalidBinding);
        }
        let binding = GrantTargetBinding::WorkspaceScope {
            path,
            authorization_id: workspace.authorization_id().clone(),
            adapter_instance_id: workspace.adapter_instance_id().clone(),
            platform: workspace.platform(),
        };
        Self::try_from(binding)
    }

    /// Binds one exact canonical object, identity, and required preimage from a held descriptor.
    pub fn held_object(held: &impl HeldWorkspaceObject) -> Result<Self, GrantTargetError> {
        let binding = GrantTargetBinding::HeldObject {
            path: held.workspace_path().clone(),
            authorization_id: held.authorization_id().clone(),
            adapter_instance_id: held.adapter_instance_id().clone(),
            platform: held.object_identity().platform(),
            object_kind: held.object_kind(),
            object_identity: held.object_identity().clone(),
            preimage: held.preimage().cloned(),
        };
        Self::try_from(binding)
    }

    /// Returns the exact workspace identity.
    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { path, .. } => path.workspace_id(),
            GrantTargetBinding::HeldObject { path, .. } => path.workspace_id(),
        }
    }

    /// Returns the exact workspace-authorization event.
    #[must_use]
    pub fn authorization_id(&self) -> &WorkspaceAuthorizationId {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope {
                authorization_id, ..
            }
            | GrantTargetBinding::HeldObject {
                authorization_id, ..
            } => authorization_id,
        }
    }

    /// Returns the exact adapter instance.
    #[must_use]
    pub fn adapter_instance_id(&self) -> &AdapterInstanceId {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope {
                adapter_instance_id,
                ..
            }
            | GrantTargetBinding::HeldObject {
                adapter_instance_id,
                ..
            } => adapter_instance_id,
        }
    }

    /// Returns the exact platform family.
    #[must_use]
    pub const fn platform(&self) -> PathPlatform {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { platform, .. }
            | GrantTargetBinding::HeldObject { platform, .. } => *platform,
        }
    }

    /// Returns the validated path components without reparsing them.
    #[must_use]
    pub fn path_components(&self) -> &[WorkspacePathComponent] {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { path, .. } => path.components(),
            GrantTargetBinding::HeldObject { path, .. } => path.components(),
        }
    }

    /// Returns the root-capable canonical scope when this is a session target.
    #[must_use]
    pub const fn scope_path(&self) -> Option<&WorkspaceScopePath> {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { path, .. } => Some(path),
            GrantTargetBinding::HeldObject { .. } => None,
        }
    }

    /// Returns the non-empty canonical path when this is an operation target.
    #[must_use]
    pub const fn workspace_path(&self) -> Option<&WorkspacePath> {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { .. } => None,
            GrantTargetBinding::HeldObject { path, .. } => Some(path),
        }
    }

    /// Returns the exact object kind for an operation target.
    #[must_use]
    pub const fn object_kind(&self) -> Option<WorkspaceObjectKind> {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { .. } => None,
            GrantTargetBinding::HeldObject { object_kind, .. } => Some(*object_kind),
        }
    }

    /// Returns content-free native-object identity evidence for an operation target.
    #[must_use]
    pub const fn object_identity(&self) -> Option<&WorkspaceObjectIdentity> {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { .. } => None,
            GrantTargetBinding::HeldObject {
                object_identity, ..
            } => Some(object_identity),
        }
    }

    /// Returns the exact file preimage required by an operation target.
    #[must_use]
    pub const fn preimage(&self) -> Option<&FilePreimage> {
        match &self.0 {
            GrantTargetBinding::WorkspaceScope { .. } => None,
            GrantTargetBinding::HeldObject { preimage, .. } => preimage.as_ref(),
        }
    }

    /// Reports whether this session scope contains another scope or held object.
    #[must_use]
    pub fn contains(&self, candidate: &Self) -> bool {
        if self.authorization_id() != candidate.authorization_id()
            || self.adapter_instance_id() != candidate.adapter_instance_id()
            || self.platform() != candidate.platform()
        {
            return false;
        }
        let Some(scope) = self.scope_path() else {
            return false;
        };
        match (&candidate.0, candidate.workspace_path()) {
            (GrantTargetBinding::WorkspaceScope { path, .. }, _) => scope.contains_scope(path),
            (GrantTargetBinding::HeldObject { .. }, Some(path)) => scope.contains_path(path),
            _ => false,
        }
    }

    /// Reports whether this exact operation target was produced by the supplied held object.
    #[must_use]
    pub fn matches_held_object(&self, held: &impl HeldWorkspaceObject) -> bool {
        Self::held_object(held).as_ref() == Ok(self)
    }

    /// Returns the lowercase object-identity digest used by the preimage record.
    #[must_use]
    pub fn object_revision(&self) -> Option<String> {
        self.object_identity()
            .map(|identity| encode_hex(identity.object_identity_sha256()))
    }
}

impl TryFrom<GrantTargetBinding> for GrantTarget {
    type Error = GrantTargetError;

    fn try_from(binding: GrantTargetBinding) -> Result<Self, Self::Error> {
        let valid_identity = |value: &str| {
            !value.is_empty()
                && value.len() <= 128
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
                })
        };
        let valid = match &binding {
            GrantTargetBinding::WorkspaceScope {
                path,
                authorization_id,
                adapter_instance_id,
                ..
            } => {
                valid_identity(path.workspace_id().as_str())
                    && valid_identity(authorization_id.as_str())
                    && valid_identity(adapter_instance_id.as_str())
            }
            GrantTargetBinding::HeldObject {
                path,
                authorization_id,
                adapter_instance_id,
                platform,
                object_kind,
                object_identity,
                preimage,
            } => {
                valid_identity(path.workspace_id().as_str())
                    && valid_identity(authorization_id.as_str())
                    && valid_identity(adapter_instance_id.as_str())
                    && object_identity.platform() == *platform
                    && matches!(
                        (object_kind, preimage),
                        (WorkspaceObjectKind::RegularFile, Some(_))
                            | (WorkspaceObjectKind::Directory, None)
                    )
            }
        };
        valid
            .then_some(Self(binding))
            .ok_or(GrantTargetError::InvalidBinding)
    }
}

impl<'de> serde::Deserialize<'de> for GrantTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let binding = GrantTargetBinding::deserialize(deserializer)?;
        Self::try_from(binding).map_err(serde::de::Error::custom)
    }
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

impl GrantPreimage {
    /// Creates the canonical indexed preimage record carried by one exact file target.
    #[must_use]
    pub fn for_target(target_index: u32, target: &GrantTarget) -> Option<Self> {
        Some(Self {
            target_index,
            content_sha256: encode_hex(target.preimage()?.content_sha256()),
            observed_revision: target.object_revision(),
        })
    }

    /// Reports whether this record exactly matches its indexed target binding.
    #[must_use]
    pub fn matches_target(&self, target_index: u32, target: &GrantTarget) -> bool {
        Self::for_target(target_index, target).as_ref() == Some(self)
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// One exact side effect included in the user-confirmed preview.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantSideEffect {
    /// Canonical operation binding; policy must agree with the grant operation.
    pub operation: OperationBinding,
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
    /// Whether this record is a session-read parent or derived operation grant.
    pub grant_class: GrantClass,
    /// Exact local actor identity whose decision created the authority.
    pub actor_id: ActorId,
    /// Exact approval decision for an operation grant; absent for a session-read parent.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub approval_id: Option<ApprovalId>,
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
    /// Versioned canonical operation and authority class.
    pub operation: OperationBinding,
    /// Exact tool identity for an operation grant; absent only for a session read parent.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub tool_id: Option<ToolId>,
    /// Exact tool-contract version paired with `tool_id`.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub tool_version: Option<String>,
    /// Ordered authorization-bound scopes or exact held-object targets.
    pub targets: Vec<GrantTarget>,
    /// Ordered authorization-bound scope subtrees excluded from the session and all children.
    pub excluded_targets: Vec<GrantTarget>,
    /// Data handling class shown when the workspace scope was approved.
    pub sensitivity: DataSensitivity,
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
    /// Maximum kernel-authorized admissions or child derivations.
    pub use_limit: u32,
    /// Number of admissions or derivations already atomically consumed in this revision.
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
        CapabilityGrant, GrantClass, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget,
    };
    use crate::{
        ActionId, ActionKind, ActorId, ApprovalId, DataSensitivity, GrantId, GrantNonce,
        GrantOperation, OperationBinding, SessionId, TaskId, ToolId,
    };

    fn target(path: &[&str]) -> GrantTarget {
        serde_json::from_value(serde_json::json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-0001", "components": path},
            "authorization_id": "authorization-0001",
            "adapter_instance_id": "adapter-0001",
            "platform": "deterministic_fake",
            "object_kind": "regular_file",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": vec![2_u8; 32]
            },
            "preimage": {"byte_len": 7, "content_sha256": vec![3_u8; 32]}
        }))
        .expect("exact target")
    }

    fn scope(path: &[&str]) -> GrantTarget {
        serde_json::from_value(serde_json::json!({
            "target_kind": "workspace_scope",
            "path": {"workspace_id": "workspace-0001", "components": path},
            "authorization_id": "authorization-0001",
            "adapter_instance_id": "adapter-0001",
            "platform": "deterministic_fake"
        }))
        .expect("workspace scope")
    }

    fn grant() -> CapabilityGrant {
        CapabilityGrant {
            schema_version: crate::CONTRACT_SCHEMA_VERSION,
            grant_id: GrantId::from_raw("grant-0001"),
            revision: 1,
            grant_class: GrantClass::Operation,
            actor_id: ActorId::from_raw("actor-local-0001"),
            approval_id: Some(ApprovalId::from_raw("approval-0001")),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: Some(ActionId::from_raw("action-0001")),
            action_kind: Some(ActionKind::DeterministicTool),
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            tool_id: Some(ToolId::from_raw("fixture.read")),
            tool_version: Some("1.0.0".to_owned()),
            targets: vec![target(&["fixtures", "input.txt"])],
            excluded_targets: vec![scope(&["fixtures", "private"])],
            sensitivity: DataSensitivity::Ephemeral,
            argument_sha256: "1".repeat(64),
            preimages: vec![
                GrantPreimage::for_target(0, &target(&["fixtures", "input.txt"]))
                    .expect("file preimage"),
            ],
            expected_side_effects: vec![GrantSideEffect {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
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
        assert_eq!(
            grant.targets[0]
                .path_components()
                .iter()
                .map(|component| component.as_str())
                .collect::<Vec<_>>(),
            ["fixtures", "input.txt"]
        );
        assert_eq!(
            grant.operation,
            OperationBinding::new(GrantOperation::WorkspaceRead)
        );
        assert_eq!(grant.status, GrantStatus::Issued);
    }

    #[test]
    fn every_top_level_key_is_required_and_unknown_authority_shapes_fail_closed() {
        let grant = grant();
        let value = serde_json::to_value(&grant).expect("grant fixture must encode");
        let object = value.as_object().expect("grant must be an object");
        assert_eq!(object.len(), 30);

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
        wildcard["operation"]["operation"] = serde_json::json!("all");
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
