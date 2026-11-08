// SPDX-License-Identifier: Apache-2.0
//! Endpoint run framing without a native model profile or tokenizer assertion.

use crate::{
    EndpointModelSelection, PolicyId, RepositorySnapshotId, RuntimeEventCursor, RuntimeRunId,
    RuntimeRunLimits, RuntimeSessionMode, RuntimeToolReference, SessionId, Task, WorkPacket,
    WorkspaceId,
};

/// Byte ceilings for endpoint context; none of these fields counts rendered tokens.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEndpointContextBudget {
    /// Maximum ordered original text messages.
    pub max_messages: u32,
    /// Maximum aggregate original UTF-8 content bytes.
    pub max_content_bytes: u64,
    /// Maximum encoded JSON message-array bytes, including roles and escaping.
    pub max_serialized_messages_bytes: u64,
    /// Maximum full prepared request-body bytes, excluding headers and credentials.
    pub max_request_body_bytes: u64,
}

/// Statically framed run with a person-selected endpoint, not an admitted native model.
/// Sealing this record authorizes no disclosure, tool, effect or resumed execution.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEndpointRunRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable reusable-runtime run identity.
    pub run_id: RuntimeRunId,
    /// Owning local session.
    pub session_id: SessionId,
    /// Selected persistence and authority mode, subject to separate composition.
    pub mode: RuntimeSessionMode,
    /// Complete user-directed task contract.
    pub task: Task,
    /// Exact revision-controlled work packet.
    pub work_packet: WorkPacket,
    /// Authorized workspace identity.
    pub workspace_id: WorkspaceId,
    /// Digest of the exact workspace projection supplied to tools.
    pub workspace_snapshot_sha256: String,
    /// Exact repository observation identity.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Digest of that repository observation.
    pub repository_snapshot_sha256: String,
    /// Selected address and requested name, without weight or process attestation.
    pub model_selection: EndpointModelSelection,
    /// Bounded original content and prepared bytes, never native token accounting.
    pub context_budget: RuntimeEndpointContextBudget,
    /// Maximum requested output-token ceiling; the server is not locally verified.
    pub max_output_tokens: u32,
    /// Exact frozen visible tool-catalog identity.
    pub tool_catalog_id: crate::ToolCatalogId,
    /// Digest of the canonical visible tool catalog.
    pub tool_catalog_sha256: String,
    /// Ordered references to every visible tool definition.
    pub visible_tools: Vec<RuntimeToolReference>,
    /// Exact deterministic policy identity.
    pub policy_id: PolicyId,
    /// Digest of the complete policy revision.
    pub policy_sha256: String,
    /// Inclusive run ceilings.
    pub limits: RuntimeRunLimits,
    /// Optional prior event; a value alone never establishes resumability.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub event_cursor: Option<RuntimeEventCursor>,
    /// SHA-256 of this canonical request with this field set to all zeroes.
    pub request_sha256: String,
}
