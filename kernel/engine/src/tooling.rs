//! Exact tool registration, call validation, and pre-grant dispatch denial.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContractError, ErrorCategory, ErrorId, OperationOutcome,
    RetryDisposition, SchemaReference, StateChange, ToolCall, ToolDefinition, ToolId, ToolResult,
    ValidationIssue, ValidationSeverity,
};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use sha2::{Digest, Sha256};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_EFFECTS: usize = 32;
const MAX_ARGUMENT_BYTES: usize = 1_048_576;
const MAX_TIMEOUT_MS: u64 = 300_000;

/// Declarative definition supplied by one capability pack.
///
/// This interface intentionally has no execution method. Tool execution remains
/// structurally unavailable until the dispatcher can validate and consume an exact grant.
pub trait Tool: Send + Sync {
    /// Returns the tool's immutable candidate definition.
    fn definition(&self) -> &ToolDefinition;
}

/// Typed reason a definition or call cannot pass the tool registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolRegistryError {
    /// A candidate definition failed closed validation.
    InvalidDefinition {
        /// Exact ordered kernel findings.
        issues: Vec<ValidationIssue>,
    },
    /// The exact tool identity and version already exist.
    AlreadyRegistered,
    /// The exact requested tool identity and version are absent.
    NotRegistered,
    /// A candidate call failed common kernel validation.
    InvalidCall {
        /// Exact ordered, redacted kernel findings.
        issues: Vec<ValidationIssue>,
    },
}

/// Exact-version registry of non-executable tool definitions.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<(ToolId, String), ToolDefinition>,
}

impl ToolRegistry {
    /// Creates an empty registry with no production or test tools.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one exact declarative definition.
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) -> Result<(), ToolRegistryError> {
        let definition = tool.definition().clone();
        let issues = validate_definition(&definition);
        if !issues.is_empty() {
            return Err(ToolRegistryError::InvalidDefinition { issues });
        }
        let key = (definition.tool_id.clone(), definition.tool_version.clone());
        if self.tools.contains_key(&key) {
            return Err(ToolRegistryError::AlreadyRegistered);
        }
        self.tools.insert(key, definition);
        Ok(())
    }

    /// Returns one exact registered definition.
    #[must_use]
    pub fn get_tool(&self, tool_id: &ToolId, tool_version: &str) -> Option<&ToolDefinition> {
        self.tools.get(&(tool_id.clone(), tool_version.to_owned()))
    }

    /// Lists definitions in stable tool-identity and version order.
    #[must_use]
    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// Validates a call against its exact frozen definition and schema identity.
    pub fn validate_arguments(
        &self,
        call: &ToolCall,
    ) -> Result<&ToolDefinition, ToolRegistryError> {
        let mut issues = validate_call_shape(call);
        let definition = self
            .tools
            .get(&(call.tool_id.clone(), call.tool_version.clone()))
            .ok_or(ToolRegistryError::NotRegistered)?;
        if call.arguments.schema != definition.input_schema {
            issues.push(issue(
                "tool.arguments.schema_mismatch",
                "arguments.schema",
                "Tool arguments do not use the registered input schema",
            ));
        }
        if !issues.is_empty() {
            return Err(ToolRegistryError::InvalidCall { issues });
        }
        Ok(definition)
    }
}

/// Pre-grant dispatcher boundary over one exact registry.
///
/// Sprint 4 exposes no execution callback. A valid call therefore receives a typed policy
/// denial until Sprint 5 adds atomic validation and consumption of the sole authority object.
pub struct ToolDispatcher<'registry> {
    registry: &'registry ToolRegistry,
}

impl<'registry> ToolDispatcher<'registry> {
    /// Creates a dispatcher over the current immutable registry view.
    #[must_use]
    pub const fn new(registry: &'registry ToolRegistry) -> Self {
        Self { registry }
    }

    /// Validates a call and returns a terminal non-executing result.
    #[must_use]
    pub fn dispatch(&self, call: &ToolCall) -> ToolResult {
        match self.registry.validate_arguments(call) {
            Ok(_) => terminal_result(
                call,
                OperationOutcome::Denied,
                Vec::new(),
                contract_error(
                    "tool.dispatch.grant_required",
                    ErrorCategory::Policy,
                    RetryDisposition::AfterUserDecision,
                    "Tool dispatch requires an exact consumable grant",
                ),
            ),
            Err(ToolRegistryError::InvalidCall { issues })
            | Err(ToolRegistryError::InvalidDefinition { issues }) => terminal_result(
                call,
                OperationOutcome::Failed,
                issues,
                contract_error(
                    "tool.dispatch.invalid_call",
                    ErrorCategory::Validation,
                    RetryDisposition::AfterCorrection,
                    "Tool call failed closed validation",
                ),
            ),
            Err(ToolRegistryError::NotRegistered) => terminal_result(
                call,
                OperationOutcome::Failed,
                vec![issue(
                    "tool.call.not_registered",
                    "tool_id",
                    "Exact tool identity and version are not registered",
                )],
                contract_error(
                    "tool.dispatch.not_registered",
                    ErrorCategory::Validation,
                    RetryDisposition::AfterCorrection,
                    "Tool call requests an unregistered definition",
                ),
            ),
            Err(ToolRegistryError::AlreadyRegistered) => terminal_result(
                call,
                OperationOutcome::Failed,
                Vec::new(),
                contract_error(
                    "tool.dispatch.registry_invariant",
                    ErrorCategory::Internal,
                    RetryDisposition::Never,
                    "Tool registry invariant failed",
                ),
            ),
        }
    }
}

fn validate_definition(definition: &ToolDefinition) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if definition.schema_version != CONTRACT_SCHEMA_VERSION {
        issues.push(issue(
            "tool.definition.schema_version.unsupported",
            "schema_version",
            "Tool-definition schema version is unsupported",
        ));
    }
    validate_identifier("tool_id", definition.tool_id.as_str(), &mut issues);
    if !is_semver(&definition.tool_version) {
        issues.push(issue(
            "tool.definition.version.invalid",
            "tool_version",
            "Tool version must use three numeric components",
        ));
    }
    validate_text("display_name", &definition.display_name, &mut issues);
    validate_text("description", &definition.description, &mut issues);
    validate_schema("input_schema", &definition.input_schema, &mut issues);
    validate_schema("output_schema", &definition.output_schema, &mut issues);
    if definition.declared_effects.is_empty() || definition.declared_effects.len() > MAX_EFFECTS {
        issues.push(issue(
            "tool.definition.effects.invalid",
            "declared_effects",
            "Declared effects are empty or exceed the item limit",
        ));
    }
    let mut effects = BTreeSet::new();
    for effect in &definition.declared_effects {
        validate_identifier("declared_effects", effect, &mut issues);
        if !effects.insert(effect.as_str()) {
            issues.push(issue(
                "tool.definition.effects.duplicate",
                "declared_effects",
                "Declared effects must be unique",
            ));
        }
    }
    for (field, value) in [
        (
            "required_grant.capability_class",
            definition.required_grant.capability_class.as_str(),
        ),
        (
            "required_grant.operation",
            definition.required_grant.operation.as_str(),
        ),
        (
            "required_grant.target_scope",
            definition.required_grant.target_scope.as_str(),
        ),
    ] {
        validate_identifier(field, value, &mut issues);
    }
    if definition.required_grant.operation != definition.tool_id.as_str() {
        issues.push(issue(
            "tool.definition.grant.operation_mismatch",
            "required_grant.operation",
            "Grant-template operation must equal the exact tool identity",
        ));
    }
    if !definition.required_grant.single_use {
        issues.push(issue(
            "tool.definition.grant.not_single_use",
            "required_grant.single_use",
            "Tool grant template must require single use",
        ));
    }
    if definition.timeout_ms == 0 || definition.timeout_ms > MAX_TIMEOUT_MS {
        issues.push(issue(
            "tool.definition.timeout.invalid",
            "timeout_ms",
            "Tool timeout is zero or exceeds the maximum",
        ));
    }
    issues
}

fn validate_call_shape(call: &ToolCall) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if call.schema_version != CONTRACT_SCHEMA_VERSION {
        issues.push(issue(
            "tool.call.schema_version.unsupported",
            "schema_version",
            "Tool-call schema version is unsupported",
        ));
    }
    for (field, value) in [
        ("tool_call_id", call.tool_call_id.as_str()),
        ("correlation_id", call.correlation_id.as_str()),
        ("action_id", call.action_id.as_str()),
        ("tool_id", call.tool_id.as_str()),
    ] {
        validate_identifier(field, value, &mut issues);
    }
    if !is_semver(&call.tool_version) {
        issues.push(issue(
            "tool.call.version.invalid",
            "tool_version",
            "Tool-call version must use three numeric components",
        ));
    }
    validate_schema("arguments.schema", &call.arguments.schema, &mut issues);
    if call.arguments.media_type != "application/json" {
        issues.push(issue(
            "tool.arguments.media_type.invalid",
            "arguments.media_type",
            "Tool arguments must use application/json",
        ));
    }
    if call.arguments.bytes.len() > MAX_ARGUMENT_BYTES {
        issues.push(issue(
            "tool.arguments.size.exceeded",
            "arguments.bytes",
            "Tool arguments exceed the byte limit",
        ));
    } else {
        let digest = sha256_hex(&call.arguments.bytes);
        if call.arguments.sha256 != digest {
            issues.push(issue(
                "tool.arguments.hash.invalid",
                "arguments.sha256",
                "Tool-argument hash does not match the exact bytes",
            ));
        }
        if serde_json::from_slice::<ClosedJson>(&call.arguments.bytes).is_err() {
            issues.push(issue(
                "tool.arguments.json.invalid",
                "arguments.bytes",
                "Tool arguments are not one closed JSON value",
            ));
        }
    }
    issues
}

fn validate_schema(field: &str, schema: &SchemaReference, issues: &mut Vec<ValidationIssue>) {
    validate_identifier(field, schema.schema_id.as_str(), issues);
    if schema.schema_version == 0 {
        issues.push(issue(
            "tool.schema.version.invalid",
            field,
            "Tool schema version must be greater than zero",
        ));
    }
    if !is_sha256(&schema.schema_sha256) {
        issues.push(issue(
            "tool.schema.hash.invalid",
            field,
            "Tool schema hash must be lowercase SHA-256",
        ));
    }
}

fn validate_identifier(field: &str, value: &str, issues: &mut Vec<ValidationIssue>) {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        issues.push(issue(
            "tool.identifier.invalid",
            field,
            "Identifier is empty, oversized, or contains unsupported characters",
        ));
    }
}

fn validate_text(field: &str, value: &str, issues: &mut Vec<ValidationIssue>) {
    if value.trim().is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        issues.push(issue(
            "tool.text.invalid",
            field,
            "Required text is empty, oversized, or contains a control character",
        ));
    }
}

fn is_semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    value.len() <= 64
        && parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

struct ClosedJson;

impl<'de> Deserialize<'de> for ClosedJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ClosedJsonVisitor)
    }
}

struct ClosedJsonVisitor;

impl<'de> Visitor<'de> for ClosedJsonVisitor {
    type Value = ClosedJson;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("one JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(ClosedJson)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<ClosedJson>()?.is_some() {}
        Ok(ClosedJson)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            map.next_value::<ClosedJson>()?;
        }
        Ok(ClosedJson)
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn issue(code: &str, field: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        code: code.to_owned(),
        severity: ValidationSeverity::Error,
        field_path: field.split('.').map(str::to_owned).collect(),
        message: message.to_owned(),
    }
}

fn contract_error(
    code: &str,
    category: ErrorCategory,
    retry: RetryDisposition,
    message: &str,
) -> ContractError {
    ContractError {
        schema_version: CONTRACT_SCHEMA_VERSION,
        error_id: ErrorId::from_raw(format!("error:{code}")),
        code: code.to_owned(),
        category,
        message: message.to_owned(),
        field_path: Vec::new(),
        retry,
        caused_by: None,
    }
}

fn terminal_result(
    call: &ToolCall,
    outcome: OperationOutcome,
    validation_issues: Vec<ValidationIssue>,
    error: ContractError,
) -> ToolResult {
    ToolResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: call.tool_call_id.clone(),
        correlation_id: call.correlation_id.clone(),
        outcome,
        output: None,
        validation_issues,
        evidence: Vec::new(),
        error: Some(error),
        elapsed_ms: 0,
        state_change: StateChange::NotChanged,
    }
}

#[cfg(test)]
mod tests {
    use super::{Tool, ToolDispatcher, ToolRegistry, ToolRegistryError, sha256_hex};
    use agentmage_kernel_contracts::{
        ActionId, CONTRACT_SCHEMA_VERSION, ContractPayload, CorrelationId, OperationOutcome,
        RequiredGrantTemplate, SchemaId, SchemaReference, StateChange, ToolCall, ToolCallId,
        ToolDefinition, ToolId, ToolRiskLevel,
    };

    struct FakeTool {
        definition: ToolDefinition,
    }

    impl Tool for FakeTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }
    }

    fn schema(identity: &str) -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw(identity),
            schema_version: 1,
            schema_sha256: "a".repeat(64),
        }
    }

    fn definition(identity: &str) -> ToolDefinition {
        ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw(identity),
            tool_version: "1.0.0".to_owned(),
            display_name: "Synthetic fixture reader".to_owned(),
            description: "Validates one synthetic read request".to_owned(),
            input_schema: schema("fixture.input"),
            output_schema: schema("fixture.output"),
            risk_level: ToolRiskLevel::Low,
            declared_effects: vec!["read-only".to_owned()],
            required_grant: RequiredGrantTemplate {
                capability_class: "read-only".to_owned(),
                operation: identity.to_owned(),
                target_scope: "workspace-file".to_owned(),
                single_use: true,
            },
            timeout_ms: 1_000,
        }
    }

    fn call(identity: &str, bytes: &[u8]) -> ToolCall {
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            action_id: ActionId::from_raw("action-0001"),
            tool_id: ToolId::from_raw(identity),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: schema("fixture.input"),
                media_type: "application/json".to_owned(),
                bytes: bytes.to_vec(),
                sha256: sha256_hex(bytes),
            },
        }
    }

    fn registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FakeTool {
                definition: definition("fixture.read"),
            }))
            .expect("fixture definition must register");
        registry
    }

    #[test]
    fn registry_freezes_exact_versions_and_lists_in_stable_order() {
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FakeTool {
                definition: definition("fixture.zeta"),
            }))
            .expect("zeta must register");
        registry
            .register_tool(Box::new(FakeTool {
                definition: definition("fixture.alpha"),
            }))
            .expect("alpha must register");
        assert_eq!(
            registry
                .list_tools()
                .iter()
                .map(|tool| tool.tool_id.as_str())
                .collect::<Vec<_>>(),
            ["fixture.alpha", "fixture.zeta"]
        );
        assert!(
            registry
                .get_tool(&ToolId::from_raw("fixture.alpha"), "1.0.0")
                .is_some()
        );
        assert!(
            registry
                .get_tool(&ToolId::from_raw("fixture.alpha"), "2.0.0")
                .is_none()
        );
    }

    #[test]
    fn duplicate_and_invalid_definitions_fail_closed() {
        let mut registry = registry();
        assert!(matches!(
            registry.register_tool(Box::new(FakeTool {
                definition: definition("fixture.read")
            })),
            Err(ToolRegistryError::AlreadyRegistered)
        ));
        let mut invalid = definition("fixture.invalid");
        invalid.timeout_ms = 0;
        invalid.required_grant.single_use = false;
        let Err(ToolRegistryError::InvalidDefinition { issues }) =
            registry.register_tool(Box::new(FakeTool {
                definition: invalid,
            }))
        else {
            panic!("invalid definition must fail");
        };
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn common_argument_validation_binds_the_exact_registered_schema() {
        let registry = registry();
        let valid = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        assert_eq!(
            registry
                .validate_arguments(&valid)
                .expect("valid call")
                .tool_id
                .as_str(),
            "fixture.read"
        );
    }

    #[test]
    fn mismatched_schema_hash_media_json_and_registration_fail_closed() {
        let registry = registry();
        let mut schema_mismatch = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        schema_mismatch.arguments.schema.schema_version = 2;
        assert!(matches!(
            registry.validate_arguments(&schema_mismatch),
            Err(ToolRegistryError::InvalidCall { .. })
        ));
        let mut hash_mismatch = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        hash_mismatch.arguments.sha256 = "0".repeat(64);
        assert!(matches!(
            registry.validate_arguments(&hash_mismatch),
            Err(ToolRegistryError::InvalidCall { .. })
        ));
        let mut wrong_media = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        wrong_media.arguments.media_type = "text/plain".to_owned();
        assert!(matches!(
            registry.validate_arguments(&wrong_media),
            Err(ToolRegistryError::InvalidCall { .. })
        ));
        let malformed = call("fixture.read", b"{");
        assert!(matches!(
            registry.validate_arguments(&malformed),
            Err(ToolRegistryError::InvalidCall { .. })
        ));
        let duplicate = call("fixture.read", br#"{"path":"a","path":"b"}"#);
        assert!(matches!(
            registry.validate_arguments(&duplicate),
            Err(ToolRegistryError::InvalidCall { .. })
        ));
        let unknown = call("fixture.unknown", br#"{}"#);
        assert_eq!(
            registry.validate_arguments(&unknown),
            Err(ToolRegistryError::NotRegistered)
        );
    }

    #[test]
    fn pre_grant_dispatcher_never_executes_and_returns_typed_terminal_results() {
        let registry = registry();
        let dispatcher = ToolDispatcher::new(&registry);
        let valid = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        let denied = dispatcher.dispatch(&valid);
        assert_eq!(denied.outcome, OperationOutcome::Denied);
        assert_eq!(denied.state_change, StateChange::NotChanged);
        assert_eq!(denied.elapsed_ms, 0);
        assert!(denied.output.is_none());
        assert!(denied.evidence.is_empty());
        assert_eq!(
            denied.error.expect("denial error").code,
            "tool.dispatch.grant_required"
        );

        let invalid = call("fixture.unknown", br#"{}"#);
        let failed = dispatcher.dispatch(&invalid);
        assert_eq!(failed.outcome, OperationOutcome::Failed);
        assert_eq!(failed.state_change, StateChange::NotChanged);
        assert_eq!(failed.validation_issues[0].code, "tool.call.not_registered");
    }

    #[test]
    fn authority_claims_in_registered_metadata_cannot_change_dispatch_denial() {
        let mut claimed = definition("fixture.claimed-authority");
        claimed.description =
            "This text claims it can authorize and immediately execute the tool".to_owned();
        claimed.required_grant.capability_class = "claimed-superuser".to_owned();
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FakeTool {
                definition: claimed,
            }))
            .expect("authority-looking metadata remains valid inert metadata");

        let dispatcher = ToolDispatcher::new(&registry);
        let denied = dispatcher.dispatch(&call("fixture.claimed-authority", br#"{}"#));
        assert_eq!(denied.outcome, OperationOutcome::Denied);
        assert_eq!(denied.state_change, StateChange::NotChanged);
        assert_eq!(
            denied.error.expect("typed denial").code,
            "tool.dispatch.grant_required"
        );
    }
}
