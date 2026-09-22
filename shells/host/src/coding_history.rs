//! Sealed coding change records and exact history/rollback tool contracts.

use std::fmt::Write;

use agentmage_capability_repository_map::{StructuredArtifactClass, StructuredLanguage};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate,
    RuntimeArtifactRef, SchemaId, SchemaReference, ToolDefinition, ToolId, ToolRiskLevel,
    ValidationIssue, ValidationSeverity,
};
use agentmage_kernel_engine::tooling::{Tool, ToolRegistry};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Immutable artifact media type for one exact reversible structured write.
pub const CODING_CHANGE_RECORD_MEDIA_TYPE: &str =
    "application/vnd.agentmage.coding-change-record+json";
/// Native bounded change-history inspection tool.
pub const CHANGE_HISTORY_TOOL_ID: &str = "agentmage.code.change-history";
/// Native fresh-authority rollback tool.
pub const ROLLBACK_TOOL_ID: &str = "agentmage.code.rollback";
/// Version shared by the initial history and rollback contracts.
pub const CODING_HISTORY_TOOL_VERSION: &str = "1.0.0";
/// Change-history request schema identity.
pub const CHANGE_HISTORY_INPUT_SCHEMA_ID: &str = "agentmage.code.change-history.input";
/// Change-history result schema identity.
pub const CHANGE_HISTORY_OUTPUT_SCHEMA_ID: &str = "agentmage.code.change-history.output";
/// Exact rollback request schema identity.
pub const ROLLBACK_INPUT_SCHEMA_ID: &str = "agentmage.code.rollback.input";

/// Closed change-history request schema.
pub const CHANGE_HISTORY_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.code.change-history.input","type":"object","additionalProperties":false,"required":["schema_version","max_records"],"properties":{"schema_version":{"const":1},"max_records":{"type":"integer","minimum":1,"maximum":64}}}"#;
/// Closed change-history result schema.
pub const CHANGE_HISTORY_OUTPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.code.change-history.output","type":"object","additionalProperties":false,"required":["schema_version","records","result_sha256"],"properties":{"schema_version":{"const":1},"records":{"type":"array","maxItems":64,"items":{"type":"object"}},"result_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}"#;
/// Closed rollback request schema.
pub const ROLLBACK_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.code.rollback.input","type":"object","additionalProperties":false,"required":["schema_version","rollback_id","source","intent_sha256","change_plan_sha256"],"properties":{"schema_version":{"const":1},"rollback_id":{"type":"string","pattern":"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$"},"source":{"type":"object"},"intent_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"change_plan_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"}}}"#;

/// Exact retained source for one successful structured write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingChangeRecord {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable record identity derived from the write receipt.
    pub record_id: String,
    /// Owning durable session.
    pub session_id: String,
    /// Owning coding task.
    pub task_id: String,
    /// Run that produced the write.
    pub producer_run_id: String,
    /// Controlled-write operation identity.
    pub operation_id: String,
    /// Canonical workspace-relative target components.
    pub path: Vec<String>,
    /// Explicit source language.
    pub language: StructuredLanguage,
    /// Explicit artifact classification.
    pub artifact_class: StructuredArtifactClass,
    /// Exact UTF-8 preimage needed for a possible inverse write.
    pub preimage: String,
    /// Digest of the exact preimage.
    pub preimage_sha256: String,
    /// Digest of the exact committed postimage.
    pub postimage_sha256: String,
    /// Whether trusted repository evidence classified the file as generated.
    pub generated: bool,
    /// Stable specialized receipt identity.
    pub receipt_id: String,
    /// Digest of the canonical controlled-write receipt.
    pub receipt_sha256: String,
    /// Trusted resolution time for the write.
    pub created_at_epoch_ms: u64,
    /// Digest sealing every preceding record field.
    pub record_sha256: String,
}

/// Path-free verified artifact plus its sealed decoded record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedCodingChange {
    /// Canonical immutable artifact reference.
    pub reference: RuntimeArtifactRef,
    /// Verified record decoded from that exact artifact.
    pub record: CodingChangeRecord,
}

/// Bounded request for retained changes in the current durable session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeHistoryRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Maximum number of newest records to return.
    pub max_records: u16,
}

/// Sealed bounded change history result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeHistoryOutput {
    /// Contract schema version.
    pub schema_version: u16,
    /// Oldest-to-newest selected retained changes.
    pub records: Vec<RetainedCodingChange>,
    /// Digest sealing the complete result.
    pub result_sha256: String,
}

/// Fresh inverse-write proposal bound to one retained exact change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Fresh rollback operation identity.
    pub rollback_id: String,
    /// Exact retained source artifact and decoded record.
    pub source: RetainedCodingChange,
    /// Fresh normalized intent identity.
    pub intent_sha256: String,
    /// Fresh review-ready plan identity.
    pub change_plan_sha256: String,
}

/// Validates and seals one coding change record.
pub fn seal_change_record(
    mut record: CodingChangeRecord,
) -> Result<CodingChangeRecord, CodingHistoryError> {
    record.record_sha256 = "0".repeat(64);
    validate_record(&record)?;
    record.record_sha256 = canonical_sha256(&record)?;
    Ok(record)
}

/// Verifies one sealed coding change record.
pub fn verify_change_record(record: &CodingChangeRecord) -> Result<(), CodingHistoryError> {
    validate_record(record)?;
    let mut unsigned = record.clone();
    unsigned.record_sha256 = "0".repeat(64);
    (canonical_sha256(&unsigned)? == record.record_sha256)
        .then_some(())
        .ok_or(CodingHistoryError::InvalidRecord)
}

/// Validates and seals one bounded history result.
pub fn seal_history_output(
    mut output: ChangeHistoryOutput,
) -> Result<ChangeHistoryOutput, CodingHistoryError> {
    if output.schema_version != 1 || output.records.len() > 64 {
        return Err(CodingHistoryError::InvalidRequest);
    }
    for retained in &output.records {
        verify_retained_change(retained)?;
    }
    output.result_sha256 = "0".repeat(64);
    output.result_sha256 = canonical_sha256(&output)?;
    Ok(output)
}

/// Verifies a rollback request and its exact retained source reference.
pub fn verify_rollback_request(request: &RollbackRequest) -> Result<(), CodingHistoryError> {
    verify_retained_change(&request.source)?;
    if request.schema_version != 1
        || !valid_identifier(&request.rollback_id)
        || !valid_sha256(&request.intent_sha256)
        || !valid_sha256(&request.change_plan_sha256)
    {
        return Err(CodingHistoryError::InvalidRequest);
    }
    Ok(())
}

fn verify_retained_change(change: &RetainedCodingChange) -> Result<(), CodingHistoryError> {
    verify_change_record(&change.record)?;
    let bytes =
        serde_json::to_vec(&change.record).map_err(|_| CodingHistoryError::Serialization)?;
    if change.reference.media_type != CODING_CHANGE_RECORD_MEDIA_TYPE
        || change.reference.payload_sha256 != sha256(&bytes)
        || change.reference.byte_size != bytes.len() as u64
    {
        return Err(CodingHistoryError::InvalidRecord);
    }
    Ok(())
}

fn validate_record(record: &CodingChangeRecord) -> Result<(), CodingHistoryError> {
    if record.schema_version != 1
        || !valid_identifier(&record.record_id)
        || !valid_identifier(&record.session_id)
        || !valid_identifier(&record.task_id)
        || !valid_identifier(&record.producer_run_id)
        || !valid_identifier(&record.operation_id)
        || record.path.is_empty()
        || record.path.len() > 64
        || record
            .path
            .iter()
            .any(|part| part.is_empty() || part.len() > 255 || matches!(part.as_str(), "." | ".."))
        || record.preimage.is_empty()
        || record.preimage.len() > 4 * 1024 * 1024
        || record.preimage_sha256 != sha256(record.preimage.as_bytes())
        || !valid_sha256(&record.postimage_sha256)
        || !valid_identifier(&record.receipt_id)
        || !valid_sha256(&record.receipt_sha256)
        || record.created_at_epoch_ms == 0
        || !valid_sha256(&record.record_sha256)
    {
        return Err(CodingHistoryError::InvalidRecord);
    }
    Ok(())
}

struct HistoryTool(ToolDefinition);
impl Tool for HistoryTool {
    fn definition(&self) -> &ToolDefinition {
        &self.0
    }
    fn validate_arguments(&self, bytes: &[u8]) -> Vec<ValidationIssue> {
        let valid = serde_json::from_slice::<ChangeHistoryRequest>(bytes)
            .is_ok_and(|value| value.schema_version == 1 && (1..=64).contains(&value.max_records));
        issue(valid)
    }
}

struct RollbackTool(ToolDefinition);
impl Tool for RollbackTool {
    fn definition(&self) -> &ToolDefinition {
        &self.0
    }
    fn validate_arguments(&self, bytes: &[u8]) -> Vec<ValidationIssue> {
        issue(
            serde_json::from_slice::<RollbackRequest>(bytes)
                .is_ok_and(|value| verify_rollback_request(&value).is_ok()),
        )
    }
}

/// Registers the native history and rollback contracts.
pub fn register_coding_history_tools(
    registry: &mut ToolRegistry,
) -> Result<(), CodingHistoryError> {
    registry
        .register_tool(Box::new(HistoryTool(history_definition())))
        .map_err(|_| CodingHistoryError::Registration)?;
    registry
        .register_tool(Box::new(RollbackTool(rollback_definition())))
        .map_err(|_| CodingHistoryError::Registration)
}

fn history_definition() -> ToolDefinition {
    definition(
        CHANGE_HISTORY_TOOL_ID,
        "Inspect change history",
        "Reads bounded sealed change records from the current durable session",
        GrantOperation::WorkspaceRead,
        ToolRiskLevel::Low,
        CHANGE_HISTORY_INPUT_SCHEMA_ID,
        CHANGE_HISTORY_INPUT_SCHEMA_JSON,
        CHANGE_HISTORY_OUTPUT_SCHEMA_ID,
        CHANGE_HISTORY_OUTPUT_SCHEMA_JSON,
    )
}

fn rollback_definition() -> ToolDefinition {
    let mut definition = definition(
        ROLLBACK_TOOL_ID,
        "Rollback exact change",
        "Proposes one fresh exact-preimage inverse write from a sealed retained change record",
        GrantOperation::WorkspaceWrite,
        ToolRiskLevel::High,
        ROLLBACK_INPUT_SCHEMA_ID,
        ROLLBACK_INPUT_SCHEMA_JSON,
        crate::coding_changes::CONTROLLED_CHANGE_OUTPUT_SCHEMA_ID,
        crate::coding_changes::CONTROLLED_CHANGE_OUTPUT_SCHEMA_JSON,
    );
    definition.required_grant.target_scope = "one-exact-owned-worktree-path".to_owned();
    definition
}

#[allow(clippy::too_many_arguments)]
fn definition(
    tool_id: &str,
    display: &str,
    description: &str,
    operation: GrantOperation,
    risk: ToolRiskLevel,
    input_id: &str,
    input: &str,
    output_id: &str,
    output: &str,
) -> ToolDefinition {
    let operation = OperationBinding::new(operation);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(tool_id),
        tool_version: CODING_HISTORY_TOOL_VERSION.to_owned(),
        display_name: display.to_owned(),
        description: description.to_owned(),
        input_schema: schema(input_id, input),
        output_schema: schema(output_id, output),
        risk_level: risk,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: "one-exact-owned-worktree-history".to_owned(),
            single_use: true,
        },
        timeout_ms: 60_000,
    }
}

fn schema(id: &str, value: &str) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: sha256(value.as_bytes()),
    }
}

fn issue(valid: bool) -> Vec<ValidationIssue> {
    if valid {
        Vec::new()
    } else {
        vec![ValidationIssue {
            code: "coding-history.request.invalid".to_owned(),
            severity: ValidationSeverity::Error,
            field_path: vec!["arguments".to_owned()],
            message: "Coding history request failed closed validation".to_owned(),
        }]
    }
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, CodingHistoryError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| CodingHistoryError::Serialization)
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("String write");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

/// Stable content-free coding-history contract failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingHistoryError {
    /// A request violated a closed bound or identity rule.
    InvalidRequest,
    /// A retained record or artifact binding did not verify.
    InvalidRecord,
    /// Canonical serialization failed.
    Serialization,
    /// The immutable registry rejected a definition.
    Registration,
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::RuntimeArtifactId;

    use super::*;

    fn sealed_record() -> CodingChangeRecord {
        let preimage = "def add(left, right):\n    return left + right\n".to_owned();
        seal_change_record(CodingChangeRecord {
            schema_version: 1,
            record_id: "change-record:fixture".to_owned(),
            session_id: "session-fixture".to_owned(),
            task_id: "task-fixture".to_owned(),
            producer_run_id: "run-fixture".to_owned(),
            operation_id: "operation-fixture".to_owned(),
            path: vec!["src".to_owned(), "calc.py".to_owned()],
            language: StructuredLanguage::Python,
            artifact_class: StructuredArtifactClass::Code,
            preimage_sha256: sha256(preimage.as_bytes()),
            preimage,
            postimage_sha256: sha256(b"def temporary_add(left, right):\n    return left + right\n"),
            generated: false,
            receipt_id: "receipt-fixture".to_owned(),
            receipt_sha256: sha256(b"receipt-fixture"),
            created_at_epoch_ms: 1,
            record_sha256: "0".repeat(64),
        })
        .expect("sealed fixture")
    }

    fn retained(record: CodingChangeRecord) -> RetainedCodingChange {
        let bytes = serde_json::to_vec(&record).expect("record bytes");
        RetainedCodingChange {
            reference: RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw("artifact:change-fixture"),
                manifest_sha256: sha256(b"manifest-fixture"),
                payload_sha256: sha256(&bytes),
                byte_size: bytes.len() as u64,
                media_type: CODING_CHANGE_RECORD_MEDIA_TYPE.to_owned(),
            },
            record,
        }
    }

    #[test]
    fn exact_record_and_reference_admit_one_fresh_rollback_request() {
        let source = retained(sealed_record());
        let request = RollbackRequest {
            schema_version: 1,
            rollback_id: "rollback-fixture".to_owned(),
            source,
            intent_sha256: sha256(b"intent-fixture"),
            change_plan_sha256: sha256(b"plan-fixture"),
        };
        verify_rollback_request(&request).expect("exact retained source");
    }

    #[test]
    fn record_or_artifact_substitution_fails_closed() {
        let mut changed_record = retained(sealed_record());
        changed_record.record.preimage.push_str("# substituted\n");
        assert_eq!(
            verify_rollback_request(&RollbackRequest {
                schema_version: 1,
                rollback_id: "rollback-fixture".to_owned(),
                source: changed_record,
                intent_sha256: sha256(b"intent-fixture"),
                change_plan_sha256: sha256(b"plan-fixture"),
            }),
            Err(CodingHistoryError::InvalidRecord)
        );

        let mut changed_reference = retained(sealed_record());
        changed_reference.reference.payload_sha256 = sha256(b"substituted");
        assert_eq!(
            verify_rollback_request(&RollbackRequest {
                schema_version: 1,
                rollback_id: "rollback-fixture".to_owned(),
                source: changed_reference,
                intent_sha256: sha256(b"intent-fixture"),
                change_plan_sha256: sha256(b"plan-fixture"),
            }),
            Err(CodingHistoryError::InvalidRecord)
        );
    }
}
