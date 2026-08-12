//! Non-authoritative approval-display contracts.

use crate::{
    ActionKind, ActorId, ApprovalId, DataSensitivity, GrantId, GrantPreimage, GrantSideEffect,
    GrantTarget, OperationBinding, SessionId, TaskId, ToolCall,
};

/// Versioned exact snapshot rendered for one user approval decision.
///
/// This object describes a proposed operation. It contains no nonce, use limit, lifecycle state,
/// signature, or conversion into `CapabilityGrant`, and therefore cannot authorize execution.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable identity assigned to this exact approval decision.
    pub approval_id: ApprovalId,
    /// Proposed future grant identity shown for correlation only.
    pub proposed_grant_id: GrantId,
    /// Exact parent session grant identity used to bound the proposal.
    pub parent_grant_id: GrantId,
    /// Canonical digest of the parent revision observed for this preview.
    pub parent_grant_sha256: String,
    /// Local actor expected to make the decision.
    pub actor_id: ActorId,
    /// Owning session identity.
    pub session_id: SessionId,
    /// Owning task identity.
    pub task_id: TaskId,
    /// Descriptive action class.
    pub action_kind: ActionKind,
    /// Versioned canonical proposed operation and authority class.
    pub operation: OperationBinding,
    /// Exact proposed tool call, including canonical argument bytes and digest.
    pub tool_call: ToolCall,
    /// Exact included workspace-relative targets shown to the actor.
    pub targets: Vec<GrantTarget>,
    /// Exact inherited exclusions shown to the actor.
    pub excluded_targets: Vec<GrantTarget>,
    /// Data sensitivity shown with the proposed scope.
    pub sensitivity: DataSensitivity,
    /// Exact currently observed target preimages.
    pub preimages: Vec<GrantPreimage>,
    /// Exact expected side effects covered by the decision.
    pub expected_side_effects: Vec<GrantSideEffect>,
    /// Bounded rollback or recovery description shown to the actor.
    pub rollback_description: String,
    /// Proposed issuance instant in UTC Unix epoch milliseconds.
    pub issued_at_epoch_ms: u64,
    /// Proposed expiration instant in UTC Unix epoch milliseconds.
    pub expires_at_epoch_ms: u64,
    /// Kernel-computed policy revision digest used for the proposal.
    pub policy_sha256: String,
    /// Digest of the canonical request with this field set to 64 lowercase zeroes.
    pub confirmation_sha256: String,
}

#[cfg(test)]
mod tests {
    use super::ApprovalRequest;
    use crate::{
        ActionId, ActionKind, ActorId, ApprovalId, ContractPayload, CorrelationId, DataSensitivity,
        GrantId, GrantOperation, GrantPreimage, GrantSideEffect, GrantTarget, OperationBinding,
        SchemaId, SchemaReference, SessionId, TaskId, ToolCall, ToolCallId, ToolId,
        VersionedContract, from_json, to_canonical_json,
    };

    fn target() -> GrantTarget {
        serde_json::from_value(serde_json::json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-0001", "components": ["src"]},
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

    fn fixture() -> ApprovalRequest {
        ApprovalRequest {
            schema_version: crate::CONTRACT_SCHEMA_VERSION,
            approval_id: ApprovalId::from_raw("approval-0001"),
            proposed_grant_id: GrantId::from_raw("grant-proposed-0001"),
            parent_grant_id: GrantId::from_raw("grant-parent-0001"),
            parent_grant_sha256: "1".repeat(64),
            actor_id: ActorId::from_raw("actor-local-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_kind: ActionKind::DeterministicTool,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            tool_call: ToolCall {
                schema_version: crate::CONTRACT_SCHEMA_VERSION,
                tool_call_id: ToolCallId::from_raw("call-0001"),
                correlation_id: CorrelationId::from_raw("correlation-0001"),
                action_id: ActionId::from_raw("action-0001"),
                tool_id: ToolId::from_raw("fixture.read"),
                tool_version: "1.0.0".to_owned(),
                arguments: ContractPayload {
                    schema: SchemaReference {
                        schema_id: SchemaId::from_raw("fixture.input"),
                        schema_version: 1,
                        schema_sha256: "2".repeat(64),
                    },
                    media_type: "application/json".to_owned(),
                    bytes: b"{}".to_vec(),
                    sha256: "3".repeat(64),
                },
            },
            targets: vec![target()],
            excluded_targets: Vec::new(),
            sensitivity: DataSensitivity::Ephemeral,
            preimages: vec![GrantPreimage::for_target(0, &target()).expect("file preimage")],
            expected_side_effects: vec![GrantSideEffect {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                target_indexes: vec![0],
                details_sha256: "5".repeat(64),
            }],
            rollback_description: "No state change is permitted".to_owned(),
            issued_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 2_000,
            policy_sha256: "6".repeat(64),
            confirmation_sha256: "0".repeat(64),
        }
    }

    fn assert_versioned<T: VersionedContract>() {}

    #[test]
    fn approval_request_round_trips_without_grant_lifecycle_authority() {
        assert_versioned::<ApprovalRequest>();
        let request = fixture();
        let bytes = to_canonical_json(&request).expect("request must serialize");
        assert_eq!(from_json::<ApprovalRequest>(&bytes), Ok(request));

        let value: serde_json::Value = serde_json::from_slice(&bytes).expect("request JSON");
        let fields = value.as_object().expect("request object");
        for forbidden in ["nonce", "use_limit", "use_count", "status", "grant_class"] {
            assert!(!fields.contains_key(forbidden));
        }
    }
}
