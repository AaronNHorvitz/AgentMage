//! Model-facing change proposals bound back into existing controlled-write contracts.

use std::fmt::Write;

use agentmage_capability_repository_map::{
    StructuredArtifactClass, StructuredEdit, StructuredEditError, StructuredFileChangePlan,
    StructuredFileChangeRequest, StructuredLanguage, StructuredReviewHook,
    build_structured_file_change, validate_structured_edit_proposal,
};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate, SchemaId,
    SchemaReference, ToolDefinition, ToolId, ToolRiskLevel, ValidationIssue, ValidationSeverity,
    WorkspaceId, WorkspaceObjectKind, WorkspacePath,
};
use agentmage_kernel_engine::{
    filesystem_control::{FileClassification, FilesystemOperationDraft, NewDestinationDraft},
    tooling::{Tool, ToolRegistry},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_PROPOSAL_BYTES: usize = 4 * 1024 * 1024;
const MAX_WRITABLE_ROOTS: usize = 64;

/// Stable native identity for one exact-preimage structured patch.
pub const STRUCTURED_PATCH_TOOL_ID: &str = "agentmage.code.patch-file";

/// Stable native identity for one controlled absent-file creation.
pub const CONTROLLED_CREATE_TOOL_ID: &str = "agentmage.code.create-file";

/// Immutable contract version shared by the initial controlled-change tools.
pub const CONTROLLED_CHANGE_TOOL_VERSION: &str = "1.0.0";

/// Closed input-schema identity for one exact-preimage patch proposal.
pub const STRUCTURED_PATCH_INPUT_SCHEMA_ID: &str = "agentmage.code.patch-file.input";

/// Closed input-schema identity for one absent-file creation proposal.
pub const CONTROLLED_CREATE_INPUT_SCHEMA_ID: &str = "agentmage.code.create-file.input";

/// Closed output-schema identity shared by controlled write receipts.
pub const CONTROLLED_CHANGE_OUTPUT_SCHEMA_ID: &str = "agentmage.code.write-result";

/// Canonical closed JSON Schema for one exact-preimage structured patch proposal.
pub const STRUCTURED_PATCH_INPUT_SCHEMA_JSON: &str = r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.code.patch-file.input","type":"object","additionalProperties":false,"required":["schema_version","change_id","path","expected_preimage_sha256","intent_sha256","change_plan_sha256","language","artifact_class","edits","additional_review_hooks","generated","allow_generated"],"properties":{"schema_version":{"const":1},"change_id":{"$ref":"#/$defs/id"},"path":{"$ref":"#/$defs/path"},"expected_preimage_sha256":{"$ref":"#/$defs/sha"},"intent_sha256":{"$ref":"#/$defs/sha"},"change_plan_sha256":{"$ref":"#/$defs/sha"},"language":{"enum":["rust","python","type_script","tsx","java_script","swift","go","shell","sql","plain_text"]},"artifact_class":{"enum":["code","configuration","test","documentation","migration","generated_output"]},"edits":{"type":"array","minItems":1,"maxItems":256,"items":{"type":"object"}},"additional_review_hooks":{"type":"array","maxItems":256,"items":{"enum":["interface","dependency","migration","security","performance","accessibility","compatibility"]}},"generated":{"type":"boolean"},"allow_generated":{"type":"boolean"}},"$defs":{"id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"sha":{"type":"string","pattern":"^[0-9a-f]{64}$"},"path":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"string","minLength":1,"maxLength":255}}}}"##;

/// Canonical closed JSON Schema for one controlled absent-file creation proposal.
pub const CONTROLLED_CREATE_INPUT_SCHEMA_JSON: &str = r##"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.code.create-file.input","type":"object","additionalProperties":false,"required":["schema_version","creation_id","path","content","mode","classification","intent_sha256","change_plan_sha256","expected_parent_sha256"],"properties":{"schema_version":{"const":1},"creation_id":{"$ref":"#/$defs/id"},"path":{"$ref":"#/$defs/path"},"content":{"type":"string","maxLength":4194304},"mode":{"enum":[384,416,420,448,480,493]},"classification":{"enum":["source_code","documentation","configuration","data","generated"]},"intent_sha256":{"$ref":"#/$defs/sha"},"change_plan_sha256":{"$ref":"#/$defs/sha"},"expected_parent_sha256":{"$ref":"#/$defs/sha"}},"$defs":{"id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"sha":{"type":"string","pattern":"^[0-9a-f]{64}$"},"path":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"string","minLength":1,"maxLength":255}}}}"##;

/// Canonical closed JSON Schema for one bounded controlled-write result.
pub const CONTROLLED_CHANGE_OUTPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.code.write-result","type":"object","additionalProperties":false,"required":["schema_version","operation_id","outcome","path_sha256","preimage_sha256","postimage_sha256","receipt_id","receipt_sha256"],"properties":{"schema_version":{"const":1},"operation_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"outcome":{"enum":["succeeded","denied","failed","cancelled","timed_out","uncertain"]},"path_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"preimage_sha256":{"type":["string","null"],"pattern":"^[0-9a-f]{64}$"},"postimage_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"receipt_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"receipt_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}"#;

/// Exact source-independent patch proposal emitted by an untrusted model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredPatchProposal {
    /// Closed request schema version.
    pub schema_version: u16,
    /// Stable change identity.
    pub change_id: String,
    /// Canonical workspace-relative path components.
    pub path: Vec<String>,
    /// Digest of the exact preimage the model observed.
    pub expected_preimage_sha256: String,
    /// Exact normalized intent identity.
    pub intent_sha256: String,
    /// Exact review-ready plan identity.
    pub change_plan_sha256: String,
    /// Explicit source language or textual dialect.
    pub language: StructuredLanguage,
    /// Explicit artifact classification.
    pub artifact_class: StructuredArtifactClass,
    /// Stable ordered structured edits.
    pub edits: Vec<StructuredEdit>,
    /// Additional evidence-selected review hooks.
    pub additional_review_hooks: Vec<StructuredReviewHook>,
    /// Whether trusted repository evidence marks the target as generated.
    pub generated: bool,
    /// Exact generated-output exception requested for this one change.
    pub allow_generated: bool,
}

/// Closed classification for one controlled absent-file creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlledFileClassification {
    /// Program source or source-adjacent text.
    SourceCode,
    /// Human-readable project documentation.
    Documentation,
    /// Non-secret project configuration.
    Configuration,
    /// Non-protected project data.
    Data,
    /// Explicitly classified generated output.
    Generated,
}

/// Exact absent-file creation proposal emitted by an untrusted model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlledFileCreationProposal {
    /// Closed request schema version.
    pub schema_version: u16,
    /// Stable creation identity.
    pub creation_id: String,
    /// Canonical workspace-relative destination path components.
    pub path: Vec<String>,
    /// Complete bounded UTF-8 content.
    pub content: String,
    /// Exact POSIX-compatible mode from the closed allowlist.
    pub mode: u32,
    /// Explicit destination classification.
    pub classification: ControlledFileClassification,
    /// Exact normalized intent identity.
    pub intent_sha256: String,
    /// Exact review-ready plan identity.
    pub change_plan_sha256: String,
    /// Digest of the complete observed parent and sibling-name projection.
    pub expected_parent_sha256: String,
}

/// Exact writable roots for one approved workspace and owned worktree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodingWriteScope {
    workspace_id: WorkspaceId,
    writable_roots: Vec<Vec<String>>,
}

impl CodingWriteScope {
    /// Validates one stable, nonoverlapping writable-root declaration.
    pub fn new(
        workspace_id: WorkspaceId,
        writable_roots: Vec<Vec<String>>,
    ) -> Result<Self, CodingChangeError> {
        if writable_roots.is_empty()
            || writable_roots.len() > MAX_WRITABLE_ROOTS
            || writable_roots.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(CodingChangeError::InvalidScope);
        }
        for (index, root) in writable_roots.iter().enumerate() {
            if !root.is_empty()
                && WorkspacePath::new(workspace_id.clone(), root.iter().cloned()).is_err()
            {
                return Err(CodingChangeError::InvalidScope);
            }
            if writable_roots
                .iter()
                .enumerate()
                .any(|(other_index, other)| {
                    index != other_index && root.len() <= other.len() && other.starts_with(root)
                })
            {
                return Err(CodingChangeError::InvalidScope);
            }
        }
        Ok(Self {
            workspace_id,
            writable_roots,
        })
    }

    /// Returns the exact approved workspace identity.
    #[must_use]
    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns stable nonoverlapping writable roots as canonical component lists.
    #[must_use]
    pub fn writable_roots(&self) -> &[Vec<String>] {
        &self.writable_roots
    }

    fn resolve(&self, components: &[String]) -> Result<WorkspacePath, CodingChangeError> {
        let path = WorkspacePath::new(self.workspace_id.clone(), components.iter().cloned())
            .map_err(|_| CodingChangeError::PathDenied)?;
        self.writable_roots
            .iter()
            .any(|root| components.starts_with(root) && components.len() > root.len())
            .then_some(path)
            .ok_or(CodingChangeError::PathDenied)
    }
}

/// Stable content-free refusal from coding-change proposal composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingChangeError {
    /// Writable roots are empty, malformed, overlapping, duplicated, or unordered.
    InvalidScope,
    /// A request is malformed, oversized, stale-shaped, or internally inconsistent.
    InvalidRequest,
    /// The requested path is noncanonical or outside every writable root.
    PathDenied,
    /// A held parent directory or its sibling projection differs from the proposal observation.
    ParentObservationMismatch,
    /// The existing structured-edit planner rejected the proposal and exact preimage.
    StructuredEdit(StructuredEditError),
    /// One native definition could not enter the common registry.
    RegistrationDenied,
}

impl CodingChangeError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidScope => "coding-change.scope.invalid",
            Self::InvalidRequest => "coding-change.request.invalid",
            Self::PathDenied => "coding-change.path.denied",
            Self::ParentObservationMismatch => "coding-change.parent-observation.mismatch",
            Self::StructuredEdit(error) => error.code(),
            Self::RegistrationDenied => "coding-change.registration.denied",
        }
    }
}

impl From<StructuredEditError> for CodingChangeError {
    fn from(value: StructuredEditError) -> Self {
        Self::StructuredEdit(value)
    }
}

struct RegisteredPatchTool {
    definition: ToolDefinition,
    scope: CodingWriteScope,
}

impl Tool for RegisteredPatchTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        proposal_issue(validate_patch_bytes(&self.scope, arguments))
    }
}

struct RegisteredCreateTool {
    definition: ToolDefinition,
    scope: CodingWriteScope,
}

impl Tool for RegisteredCreateTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn validate_arguments(&self, arguments: &[u8]) -> Vec<ValidationIssue> {
        proposal_issue(validate_create_bytes(&self.scope, arguments))
    }
}

/// Binds one inert patch proposal to trusted exact preimage bytes and runs the existing planner.
pub fn bind_structured_patch_proposal(
    scope: &CodingWriteScope,
    proposal: StructuredPatchProposal,
    preimage: Vec<u8>,
) -> Result<StructuredFileChangePlan, CodingChangeError> {
    validate_patch(scope, &proposal)?;
    let path = scope.resolve(&proposal.path)?;
    build_structured_file_change(StructuredFileChangeRequest {
        change_id: proposal.change_id,
        path,
        intent_sha256: proposal.intent_sha256,
        change_plan_sha256: proposal.change_plan_sha256,
        language: proposal.language,
        artifact_class: proposal.artifact_class,
        preimage,
        expected_preimage_sha256: proposal.expected_preimage_sha256,
        edits: proposal.edits,
        additional_review_hooks: proposal.additional_review_hooks,
        generated: proposal.generated,
        allow_generated: proposal.allow_generated,
    })
    .map_err(Into::into)
}

/// Computes the canonical identity of one held parent and sorted sibling-name projection.
pub fn controlled_create_parent_observation_sha256(
    parent: &agentmage_kernel_contracts::GrantTarget,
    observed_sibling_names: &[String],
) -> Result<String, CodingChangeError> {
    if parent.object_kind() != Some(WorkspaceObjectKind::Directory)
        || parent.preimage().is_some()
        || !parent.is_operation_target()
        || observed_sibling_names
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || observed_sibling_names
            .iter()
            .any(|name| WorkspacePath::new(parent.workspace_id().clone(), [name.as_str()]).is_err())
    {
        return Err(CodingChangeError::ParentObservationMismatch);
    }
    serde_json::to_vec(&ControlledCreateParentObservation {
        schema_version: 1,
        parent,
        observed_sibling_names,
    })
    .map(|bytes| sha256_hex(&bytes))
    .map_err(|_| CodingChangeError::ParentObservationMismatch)
}

/// Binds one controlled-create proposal to a trusted exact parent observation.
pub fn prepare_controlled_file_creation(
    scope: &CodingWriteScope,
    proposal: ControlledFileCreationProposal,
    parent: agentmage_kernel_contracts::GrantTarget,
    observed_sibling_names: Vec<String>,
) -> Result<FilesystemOperationDraft, CodingChangeError> {
    validate_create(scope, &proposal)?;
    let path = scope.resolve(&proposal.path)?;
    let parent_matches = parent.workspace_id() == path.workspace_id()
        && parent.path_components() == &path.components()[..path.components().len() - 1];
    if !parent_matches
        || controlled_create_parent_observation_sha256(&parent, &observed_sibling_names)?
            != proposal.expected_parent_sha256
    {
        return Err(CodingChangeError::ParentObservationMismatch);
    }
    Ok(FilesystemOperationDraft::Create {
        operation_id: proposal.creation_id,
        destination: NewDestinationDraft {
            parent,
            path,
            observed_sibling_names,
        },
        content: proposal.content.into_bytes(),
        mode: proposal.mode,
        classification: map_file_classification(proposal.classification),
    })
}

#[derive(Serialize)]
struct ControlledCreateParentObservation<'observation> {
    schema_version: u16,
    parent: &'observation agentmage_kernel_contracts::GrantTarget,
    observed_sibling_names: &'observation [String],
}

const fn map_file_classification(value: ControlledFileClassification) -> FileClassification {
    match value {
        ControlledFileClassification::SourceCode => FileClassification::SourceCode,
        ControlledFileClassification::Documentation => FileClassification::Documentation,
        ControlledFileClassification::Configuration => FileClassification::Configuration,
        ControlledFileClassification::Data => FileClassification::Data,
        ControlledFileClassification::Generated => FileClassification::Generated,
    }
}

/// Returns the exact structured-patch tool definition.
#[must_use]
pub fn structured_patch_tool_definition() -> ToolDefinition {
    controlled_change_definition(
        STRUCTURED_PATCH_TOOL_ID,
        "Apply structured patch",
        "Binds ordered structured edits to one exact held preimage in an owned worktree",
        STRUCTURED_PATCH_INPUT_SCHEMA_ID,
        STRUCTURED_PATCH_INPUT_SCHEMA_JSON,
    )
}

/// Returns the exact controlled-create tool definition.
#[must_use]
pub fn controlled_create_tool_definition() -> ToolDefinition {
    controlled_change_definition(
        CONTROLLED_CREATE_TOOL_ID,
        "Create controlled file",
        "Creates one exact absent UTF-8 file beneath an approved owned-worktree root",
        CONTROLLED_CREATE_INPUT_SCHEMA_ID,
        CONTROLLED_CREATE_INPUT_SCHEMA_JSON,
    )
}

/// Registers patch and controlled-create proposals through the common tool registry.
pub fn register_controlled_change_runtime_tools(
    registry: &mut ToolRegistry,
    scope: CodingWriteScope,
) -> Result<(), CodingChangeError> {
    if registry
        .get_tool(
            &ToolId::from_raw(STRUCTURED_PATCH_TOOL_ID),
            CONTROLLED_CHANGE_TOOL_VERSION,
        )
        .is_some()
        || registry
            .get_tool(
                &ToolId::from_raw(CONTROLLED_CREATE_TOOL_ID),
                CONTROLLED_CHANGE_TOOL_VERSION,
            )
            .is_some()
    {
        return Err(CodingChangeError::RegistrationDenied);
    }
    registry
        .register_tool(Box::new(RegisteredPatchTool {
            definition: structured_patch_tool_definition(),
            scope: scope.clone(),
        }))
        .map_err(|_| CodingChangeError::RegistrationDenied)?;
    registry
        .register_tool(Box::new(RegisteredCreateTool {
            definition: controlled_create_tool_definition(),
            scope,
        }))
        .map_err(|_| CodingChangeError::RegistrationDenied)
}

fn validate_patch_bytes(scope: &CodingWriteScope, bytes: &[u8]) -> Result<(), CodingChangeError> {
    if bytes.is_empty() || bytes.len() > MAX_PROPOSAL_BYTES {
        return Err(CodingChangeError::InvalidRequest);
    }
    let proposal = serde_json::from_slice(bytes).map_err(|_| CodingChangeError::InvalidRequest)?;
    validate_patch(scope, &proposal)
}

fn validate_patch(
    scope: &CodingWriteScope,
    proposal: &StructuredPatchProposal,
) -> Result<(), CodingChangeError> {
    scope.resolve(&proposal.path)?;
    if proposal.schema_version != 1
        || !valid_identifier(&proposal.change_id)
        || !is_sha256(&proposal.expected_preimage_sha256)
        || !is_sha256(&proposal.intent_sha256)
        || !is_sha256(&proposal.change_plan_sha256)
        || (proposal.artifact_class == StructuredArtifactClass::GeneratedOutput)
            != proposal.generated
        || (proposal.generated && !proposal.allow_generated)
        || (!proposal.generated && proposal.allow_generated)
    {
        return Err(CodingChangeError::InvalidRequest);
    }
    validate_structured_edit_proposal(
        proposal.language,
        &proposal.edits,
        &proposal.additional_review_hooks,
    )
    .map_err(Into::into)
}

fn validate_create_bytes(scope: &CodingWriteScope, bytes: &[u8]) -> Result<(), CodingChangeError> {
    if bytes.is_empty() || bytes.len() > MAX_PROPOSAL_BYTES {
        return Err(CodingChangeError::InvalidRequest);
    }
    let proposal: ControlledFileCreationProposal =
        serde_json::from_slice(bytes).map_err(|_| CodingChangeError::InvalidRequest)?;
    validate_create(scope, &proposal)
}

fn validate_create(
    scope: &CodingWriteScope,
    proposal: &ControlledFileCreationProposal,
) -> Result<(), CodingChangeError> {
    scope.resolve(&proposal.path)?;
    if proposal.schema_version != 1
        || !valid_identifier(&proposal.creation_id)
        || proposal.content.len() > MAX_PROPOSAL_BYTES
        || proposal.content.contains('\0')
        || !matches!(proposal.mode, 0o600 | 0o640 | 0o644 | 0o700 | 0o740 | 0o755)
        || !is_sha256(&proposal.intent_sha256)
        || !is_sha256(&proposal.change_plan_sha256)
        || !is_sha256(&proposal.expected_parent_sha256)
    {
        return Err(CodingChangeError::InvalidRequest);
    }
    Ok(())
}

fn proposal_issue(result: Result<(), CodingChangeError>) -> Vec<ValidationIssue> {
    result.map_or_else(
        |error| {
            vec![ValidationIssue {
                code: error.code().to_owned(),
                severity: ValidationSeverity::Error,
                field_path: vec!["arguments".to_owned()],
                message: "Controlled coding-change arguments failed closed validation".to_owned(),
            }]
        },
        |()| Vec::new(),
    )
}

fn controlled_change_definition(
    tool_id: &str,
    display_name: &str,
    description: &str,
    input_schema_id: &str,
    input_schema_json: &str,
) -> ToolDefinition {
    let operation = OperationBinding::new(GrantOperation::WorkspaceWrite);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(tool_id),
        tool_version: CONTROLLED_CHANGE_TOOL_VERSION.to_owned(),
        display_name: display_name.to_owned(),
        description: description.to_owned(),
        input_schema: schema(input_schema_id, input_schema_json.as_bytes()),
        output_schema: schema(
            CONTROLLED_CHANGE_OUTPUT_SCHEMA_ID,
            CONTROLLED_CHANGE_OUTPUT_SCHEMA_JSON.as_bytes(),
        ),
        risk_level: ToolRiskLevel::High,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: "one-exact-owned-worktree-path".to_owned(),
            single_use: true,
        },
        timeout_ms: 60_000,
    }
}

fn schema(id: &str, bytes: &[u8]) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: sha256_hex(bytes),
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ActionId, ContractPayload, CorrelationId, GrantTarget, ToolCall, ToolCallId,
    };
    use agentmage_kernel_engine::tooling::ToolRegistry;
    use serde_json::json;

    use super::*;

    fn scope() -> CodingWriteScope {
        CodingWriteScope::new(
            WorkspaceId::from_raw("workspace-coding"),
            vec![vec!["src".to_owned()], vec!["tests".to_owned()]],
        )
        .expect("write scope")
    }

    fn patch() -> StructuredPatchProposal {
        StructuredPatchProposal {
            schema_version: 1,
            change_id: "change-0001".to_owned(),
            path: vec!["src".to_owned(), "guide.md".to_owned()],
            expected_preimage_sha256: sha256_hex(b"old text\n"),
            intent_sha256: "a".repeat(64),
            change_plan_sha256: "b".repeat(64),
            language: StructuredLanguage::PlainText,
            artifact_class: StructuredArtifactClass::Documentation,
            edits: vec![StructuredEdit::ReplaceExactText {
                edit_id: "edit-0001".to_owned(),
                expected: "old text".to_owned(),
                replacement: "new text".to_owned(),
            }],
            additional_review_hooks: Vec::new(),
            generated: false,
            allow_generated: false,
        }
    }

    fn create() -> ControlledFileCreationProposal {
        ControlledFileCreationProposal {
            schema_version: 1,
            creation_id: "creation-0001".to_owned(),
            path: vec!["tests".to_owned(), "new_test.rs".to_owned()],
            content: "#[test]\nfn new_test() {}\n".to_owned(),
            mode: 0o644,
            classification: ControlledFileClassification::SourceCode,
            intent_sha256: "a".repeat(64),
            change_plan_sha256: "b".repeat(64),
            expected_parent_sha256: "c".repeat(64),
        }
    }

    fn parent(path: &[&str]) -> GrantTarget {
        let identity: [u8; 32] = Sha256::digest(path.join("/").as_bytes()).into();
        serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-coding", "components": path},
            "authorization_id": "authorization-coding",
            "adapter_instance_id": "adapter-coding",
            "platform": "deterministic_fake",
            "object_kind": "directory",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": identity
            },
            "preimage": null
        }))
        .expect("held parent")
    }

    fn root_parent() -> GrantTarget {
        serde_json::from_value(json!({
            "target_kind": "held_workspace_root",
            "path": {"workspace_id": "workspace-coding", "components": []},
            "authorization_id": "authorization-coding",
            "adapter_instance_id": "adapter-coding",
            "platform": "deterministic_fake",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": vec![2_u8; 32]
            }
        }))
        .expect("held workspace root")
    }

    fn call(definition: &ToolDefinition, bytes: Vec<u8>) -> ToolCall {
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("change-call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-change-0001"),
            action_id: ActionId::from_raw("action-change-0001"),
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            arguments: ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".to_owned(),
                sha256: sha256_hex(&bytes),
                bytes,
            },
        }
    }

    #[test]
    fn story_48_2_patch_proposal_binds_to_the_existing_exact_preimage_planner() {
        let scope = scope();
        let proposal = patch();
        let plan = bind_structured_patch_proposal(&scope, proposal.clone(), b"old text\n".to_vec())
            .expect("structured plan");
        assert_eq!(plan.postimage(), b"new text\n");

        let mut tools = ToolRegistry::new();
        register_controlled_change_runtime_tools(&mut tools, scope.clone()).expect("change tools");
        let definition = structured_patch_tool_definition();
        let bytes = serde_json::to_vec(&proposal).expect("patch proposal");
        assert!(tools.validate_arguments(&call(&definition, bytes)).is_ok());

        let mut stale = proposal;
        stale.expected_preimage_sha256 = "d".repeat(64);
        assert!(bind_structured_patch_proposal(&scope, stale, b"old text\n".to_vec()).is_err());
    }

    #[test]
    fn story_48_2_change_tools_reject_traversal_out_of_scope_and_create_overrides() {
        let scope = scope();
        let mut tools = ToolRegistry::new();
        register_controlled_change_runtime_tools(&mut tools, scope).expect("change tools");

        let patch_definition = structured_patch_tool_definition();
        let mut traversal = patch();
        traversal.path = vec!["src".to_owned(), "..".to_owned(), "Cargo.toml".to_owned()];
        assert!(
            tools
                .validate_arguments(&call(
                    &patch_definition,
                    serde_json::to_vec(&traversal).expect("traversal proposal"),
                ))
                .is_err()
        );

        let mut outside = patch();
        outside.path = vec!["Cargo.toml".to_owned()];
        assert!(
            tools
                .validate_arguments(&call(
                    &patch_definition,
                    serde_json::to_vec(&outside).expect("outside proposal"),
                ))
                .is_err()
        );

        let create_definition = controlled_create_tool_definition();
        let valid_create = create();
        assert!(
            tools
                .validate_arguments(&call(
                    &create_definition,
                    serde_json::to_vec(&valid_create).expect("create proposal"),
                ))
                .is_ok()
        );
        let mut invalid_mode = valid_create;
        invalid_mode.mode = 0o777;
        assert!(
            tools
                .validate_arguments(&call(
                    &create_definition,
                    serde_json::to_vec(&invalid_mode).expect("invalid mode proposal"),
                ))
                .is_err()
        );
    }

    #[test]
    fn story_48_2_create_proposal_binds_to_exact_parent_and_sibling_projection() {
        let scope = scope();
        let parent = parent(&["tests"]);
        let siblings = vec!["existing.rs".to_owned(), "fixture.rs".to_owned()];
        let mut proposal = create();
        proposal.expected_parent_sha256 =
            controlled_create_parent_observation_sha256(&parent, &siblings)
                .expect("parent observation");

        let draft = prepare_controlled_file_creation(
            &scope,
            proposal.clone(),
            parent.clone(),
            siblings.clone(),
        )
        .expect("creation draft");
        let FilesystemOperationDraft::Create {
            operation_id,
            destination,
            content,
            mode,
            classification,
        } = draft
        else {
            panic!("wrong filesystem draft");
        };
        assert_eq!(operation_id, proposal.creation_id);
        assert_eq!(destination.parent, parent);
        assert_eq!(destination.observed_sibling_names, siblings);
        assert_eq!(content, proposal.content.as_bytes());
        assert_eq!(mode, proposal.mode);
        assert_eq!(classification, FileClassification::SourceCode);

        let stale_siblings = vec!["added.rs".to_owned(), "existing.rs".to_owned()];
        assert!(matches!(
            prepare_controlled_file_creation(&scope, proposal, parent, stale_siblings),
            Err(CodingChangeError::ParentObservationMismatch)
        ));
    }

    #[test]
    fn story_48_2_create_proposal_supports_the_exact_owned_worktree_root() {
        let scope =
            CodingWriteScope::new(WorkspaceId::from_raw("workspace-coding"), vec![Vec::new()])
                .expect("root write scope");
        let parent = root_parent();
        let siblings = vec!["Cargo.toml".to_owned(), "src".to_owned()];
        let mut proposal = create();
        proposal.path = vec!["README.md".to_owned()];
        proposal.expected_parent_sha256 =
            controlled_create_parent_observation_sha256(&parent, &siblings)
                .expect("root observation");

        let draft = prepare_controlled_file_creation(&scope, proposal, parent.clone(), siblings)
            .expect("root creation draft");
        let FilesystemOperationDraft::Create { destination, .. } = draft else {
            panic!("wrong filesystem draft");
        };
        assert_eq!(destination.parent, parent);
        assert_eq!(destination.path.components()[0].as_str(), "README.md");
    }

    #[test]
    fn story_48_2_writable_roots_are_ordered_nonoverlapping_and_exact() {
        for schema in [
            STRUCTURED_PATCH_INPUT_SCHEMA_JSON,
            CONTROLLED_CREATE_INPUT_SCHEMA_JSON,
            CONTROLLED_CHANGE_OUTPUT_SCHEMA_JSON,
        ] {
            let parsed: serde_json::Value = serde_json::from_str(schema).expect("schema JSON");
            assert_eq!(parsed["additionalProperties"], false);
        }
        assert!(
            CodingWriteScope::new(
                WorkspaceId::from_raw("workspace-coding"),
                vec![
                    vec!["src".to_owned()],
                    vec!["src".to_owned(), "bin".to_owned()]
                ],
            )
            .is_err()
        );
        assert!(
            CodingWriteScope::new(
                WorkspaceId::from_raw("workspace-coding"),
                vec![vec!["tests".to_owned()], vec!["src".to_owned()]],
            )
            .is_err()
        );
    }
}
