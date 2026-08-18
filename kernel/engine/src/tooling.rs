//! Exact tool registration, call validation, and pre-grant dispatch denial.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContractError, ErrorCategory, ErrorId, OperationOutcome,
    RetryDisposition, RuntimeToolAttemptState, SchemaReference, StateChange, ToolCall,
    ToolDefinition, ToolId, ToolResult, ValidationIssue, ValidationSeverity,
};
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use sha2::{Digest, Sha256};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_ARGUMENT_BYTES: usize = 1_048_576;
const MAX_TIMEOUT_MS: u64 = 300_000;

/// Declarative definition supplied by one capability pack.
///
/// This interface intentionally has no execution method. Tool execution remains
/// structurally unavailable until the dispatcher can validate and consume an exact grant.
pub trait Tool: Send + Sync {
    /// Returns the tool's immutable candidate definition.
    fn definition(&self) -> &ToolDefinition;

    /// Validates exact tool-specific argument bytes after common schema binding succeeds.
    ///
    /// The default admits no additional shape beyond common kernel validation. Capability
    /// packs with a closed request schema must override this method. Validation cannot execute
    /// the tool or grant authority.
    fn validate_arguments(&self, _arguments: &[u8]) -> Vec<ValidationIssue> {
        Vec::new()
    }
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

/// Claimed source of a proposal presented to the pre-grant dispatcher.
///
/// This classification is retained only for denial evidence. It does not authenticate the
/// caller, grant authority, or select a more privileged dispatch path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalOrigin {
    /// A user-interface shell proposed the call.
    Shell,
    /// Untrusted model output proposed the call.
    Model,
    /// A tool result or tool-owned workflow proposed another call.
    Tool,
    /// A capability-pack component proposed the call.
    CapabilityPack,
    /// The caller is outside the registered proposal-origin classes.
    UnregisteredCaller,
}

/// Exact pre-grant disposition recorded before any executor exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreGrantDispatchDisposition {
    /// The call failed closed registry or argument validation.
    InvalidCall,
    /// The call was valid but its claimed caller class is not registered.
    UnregisteredCaller,
    /// The call was valid but no exact consumable grant exists.
    GrantRequired,
}

/// Typed terminal receipt from the non-executing Sprint 4 dispatcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreGrantDispatchReceipt {
    /// Untrusted origin classification retained for the denial trace.
    pub origin: ProposalOrigin,
    /// Exact point at which the pre-grant attempt terminated.
    pub disposition: PreGrantDispatchDisposition,
    /// Terminal tool result preserving call and correlation identity.
    pub result: ToolResult,
}

/// Exact-version registry of non-executable tool definitions.
struct RegisteredTool {
    definition: ToolDefinition,
    implementation: Box<dyn Tool>,
}

/// Exact-version registry of non-executable tool definitions and validators.
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<(ToolId, String), RegisteredTool>,
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
        self.tools.insert(
            key,
            RegisteredTool {
                definition,
                implementation: tool,
            },
        );
        Ok(())
    }

    /// Returns one exact registered definition.
    #[must_use]
    pub fn get_tool(&self, tool_id: &ToolId, tool_version: &str) -> Option<&ToolDefinition> {
        self.tools
            .get(&(tool_id.clone(), tool_version.to_owned()))
            .map(|registered| &registered.definition)
    }

    /// Lists definitions in stable tool-identity and version order.
    #[must_use]
    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools
            .values()
            .map(|registered| &registered.definition)
            .collect()
    }

    /// Retains only definitions accepted by one trusted composition-time narrowing predicate.
    ///
    /// This operation can only remove non-executable registrations. It cannot add a tool, execute
    /// one, or create authority, and is intended for immutable profile construction.
    pub fn retain_tools(&mut self, mut retain: impl FnMut(&ToolDefinition) -> bool) {
        self.tools
            .retain(|_, registered| retain(&registered.definition));
    }

    /// Validates a call against its exact frozen definition and schema identity.
    pub fn validate_arguments(
        &self,
        call: &ToolCall,
    ) -> Result<&ToolDefinition, ToolRegistryError> {
        let mut issues = validate_call_shape(call);
        let registered = self
            .tools
            .get(&(call.tool_id.clone(), call.tool_version.clone()))
            .ok_or(ToolRegistryError::NotRegistered)?;
        let definition = &registered.definition;
        if call.arguments.schema != definition.input_schema {
            issues.push(issue(
                "tool.arguments.schema_mismatch",
                "arguments.schema",
                "Tool arguments do not use the registered input schema",
            ));
        }
        if issues.is_empty() {
            issues.extend(
                registered
                    .implementation
                    .validate_arguments(&call.arguments.bytes),
            );
        }
        if !issues.is_empty() {
            return Err(ToolRegistryError::InvalidCall { issues });
        }
        Ok(definition)
    }
}

/// Stable refusal from the deterministic repeated-call guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolAttemptGuardError {
    /// The configured repeat limit is outside the closed supported range.
    InvalidLimit,
    /// The call depth exceeds the configured maximum.
    CallDepthExceeded,
    /// The exact call identity was already recorded.
    DuplicateCallIdentity,
    /// The same semantic call reached its configured repeat ceiling.
    RepeatLimitExceeded,
    /// Persisted attempt state is malformed, unordered, or exceeds current limits.
    InvalidRestoredState,
}

impl ToolAttemptGuardError {
    /// Stable content-free refusal code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimit => "tool.attempt.repeat_limit.invalid",
            Self::CallDepthExceeded => "tool.attempt.call_depth.exceeded",
            Self::DuplicateCallIdentity => "tool.attempt.call_identity.duplicate",
            Self::RepeatLimitExceeded => "tool.attempt.repeat_limit.exceeded",
            Self::InvalidRestoredState => "tool.attempt.restored_state.invalid",
        }
    }
}

/// Content-free record emitted for one semantically bounded tool attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolAttemptRecord {
    /// Monotonic in-memory attempt sequence.
    pub sequence: u64,
    /// Digest of tool, version, schema, and exact argument bytes.
    pub semantic_sha256: String,
    /// One-based occurrence of this exact semantic call.
    pub occurrence: u8,
    /// Explicit nested call depth.
    pub call_depth: u8,
}

/// Deterministic non-authoritative repeated-call and call-depth guard.
///
/// The guard does not validate grants and cannot authorize or execute a tool. A dispatcher
/// records an admitted semantic attempt before consuming the operation's sole grant.
pub struct ToolAttemptGuard {
    maximum_repeats: u8,
    maximum_call_depth: u8,
    next_sequence: u64,
    seen_call_ids: BTreeSet<String>,
    occurrences: BTreeMap<String, u8>,
}

impl ToolAttemptGuard {
    /// Creates an empty guard with closed repeat and call-depth ceilings.
    pub fn new(maximum_repeats: u8, maximum_call_depth: u8) -> Result<Self, ToolAttemptGuardError> {
        if !(1..=16).contains(&maximum_repeats) || maximum_call_depth > 32 {
            return Err(ToolAttemptGuardError::InvalidLimit);
        }
        Ok(Self {
            maximum_repeats,
            maximum_call_depth,
            next_sequence: 1,
            seen_call_ids: BTreeSet::new(),
            occurrences: BTreeMap::new(),
        })
    }

    /// Restores exact content-free guard state after a verified safe-boundary restart.
    pub fn restore(
        maximum_repeats: u8,
        maximum_call_depth: u8,
        attempts: &[RuntimeToolAttemptState],
    ) -> Result<Self, ToolAttemptGuardError> {
        let mut guard = Self::new(maximum_repeats, maximum_call_depth)?;
        for attempt in attempts {
            if attempt.schema_version != CONTRACT_SCHEMA_VERSION
                || attempt.sequence != guard.next_sequence
                || attempt.tool_call_id.as_str().is_empty()
                || attempt.semantic_sha256.len() != 64
                || !attempt
                    .semantic_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                || attempt.occurrence == 0
                || attempt.occurrence > guard.maximum_repeats
                || attempt.call_depth > guard.maximum_call_depth
                || !guard
                    .seen_call_ids
                    .insert(attempt.tool_call_id.as_str().to_owned())
            {
                return Err(ToolAttemptGuardError::InvalidRestoredState);
            }
            let expected_occurrence = guard
                .occurrences
                .get(&attempt.semantic_sha256)
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(ToolAttemptGuardError::InvalidRestoredState)?;
            if attempt.occurrence != expected_occurrence {
                return Err(ToolAttemptGuardError::InvalidRestoredState);
            }
            guard
                .occurrences
                .insert(attempt.semantic_sha256.clone(), attempt.occurrence);
            guard.next_sequence = guard
                .next_sequence
                .checked_add(1)
                .ok_or(ToolAttemptGuardError::InvalidRestoredState)?;
        }
        Ok(guard)
    }

    /// Records one validated attempt or refuses it without changing guard state.
    pub fn record_attempt(
        &mut self,
        call: &ToolCall,
        call_depth: u8,
    ) -> Result<ToolAttemptRecord, ToolAttemptGuardError> {
        if call_depth > self.maximum_call_depth {
            return Err(ToolAttemptGuardError::CallDepthExceeded);
        }
        if self.seen_call_ids.contains(call.tool_call_id.as_str()) {
            return Err(ToolAttemptGuardError::DuplicateCallIdentity);
        }
        let semantic_sha256 = semantic_call_sha256(call);
        let occurrence = self.occurrences.get(&semantic_sha256).copied().unwrap_or(0);
        if occurrence >= self.maximum_repeats {
            return Err(ToolAttemptGuardError::RepeatLimitExceeded);
        }
        let occurrence = occurrence + 1;
        let sequence = self.next_sequence;
        let next_sequence = sequence
            .checked_add(1)
            .ok_or(ToolAttemptGuardError::RepeatLimitExceeded)?;
        self.seen_call_ids
            .insert(call.tool_call_id.as_str().to_owned());
        self.occurrences.insert(semantic_sha256.clone(), occurrence);
        self.next_sequence = next_sequence;
        Ok(ToolAttemptRecord {
            sequence,
            semantic_sha256,
            occurrence,
            call_depth,
        })
    }
}

fn semantic_call_sha256(call: &ToolCall) -> String {
    let material = serde_json::to_vec(&(
        call.tool_id.as_str(),
        &call.tool_version,
        call.arguments.schema.schema_id.as_str(),
        call.arguments.schema.schema_version,
        &call.arguments.schema.schema_sha256,
        &call.arguments.sha256,
    ))
    .expect("closed semantic call material serializes");
    sha256_hex(&material)
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

    /// Validates a call and returns a terminal non-executing receipt.
    #[must_use]
    pub fn dispatch(&self, origin: ProposalOrigin, call: &ToolCall) -> PreGrantDispatchReceipt {
        match self.registry.validate_arguments(call) {
            Ok(_) if origin == ProposalOrigin::UnregisteredCaller => PreGrantDispatchReceipt {
                origin,
                disposition: PreGrantDispatchDisposition::UnregisteredCaller,
                result: terminal_result(
                    call,
                    OperationOutcome::Denied,
                    Vec::new(),
                    contract_error(
                        "tool.dispatch.caller_not_registered",
                        ErrorCategory::Policy,
                        RetryDisposition::Never,
                        "Tool dispatch caller is not registered",
                    ),
                ),
            },
            Ok(_) => PreGrantDispatchReceipt {
                origin,
                disposition: PreGrantDispatchDisposition::GrantRequired,
                result: terminal_result(
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
            },
            Err(ToolRegistryError::InvalidCall { issues })
            | Err(ToolRegistryError::InvalidDefinition { issues }) => PreGrantDispatchReceipt {
                origin,
                disposition: PreGrantDispatchDisposition::InvalidCall,
                result: terminal_result(
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
            },
            Err(ToolRegistryError::NotRegistered) => PreGrantDispatchReceipt {
                origin,
                disposition: PreGrantDispatchDisposition::InvalidCall,
                result: terminal_result(
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
            },
            Err(ToolRegistryError::AlreadyRegistered) => PreGrantDispatchReceipt {
                origin,
                disposition: PreGrantDispatchDisposition::InvalidCall,
                result: terminal_result(
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
            },
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
    if definition.declared_effects.len() != 1 {
        issues.push(issue(
            "tool.definition.effects.invalid",
            "declared_effects",
            "A tool must declare exactly one canonical operation",
        ));
    }
    let mut effects = BTreeSet::new();
    for effect in &definition.declared_effects {
        if !effects.insert(*effect) {
            issues.push(issue(
                "tool.definition.effects.duplicate",
                "declared_effects",
                "Declared effects must be unique",
            ));
        }
    }
    validate_identifier(
        "required_grant.target_scope",
        &definition.required_grant.target_scope,
        &mut issues,
    );
    if definition.declared_effects.as_slice() != [definition.required_grant.operation] {
        issues.push(issue(
            "tool.definition.grant.operation_mismatch",
            "required_grant.operation",
            "Grant-template operation must equal the one declared canonical operation",
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
    use super::{
        PreGrantDispatchDisposition, ProposalOrigin, Tool, ToolAttemptGuard, ToolAttemptGuardError,
        ToolDispatcher, ToolRegistry, ToolRegistryError, sha256_hex,
    };
    use agentmage_kernel_contracts::{
        ActionId, CONTRACT_SCHEMA_VERSION, ContractPayload, CorrelationId, GrantOperation,
        OperationBinding, OperationOutcome, RequiredGrantTemplate, RuntimeToolAttemptState,
        SchemaId, SchemaReference, StateChange, ToolCall, ToolCallId, ToolDefinition, ToolId,
        ToolRiskLevel,
    };
    use serde_json::{Value, json};

    struct FakeTool {
        definition: ToolDefinition,
    }

    impl Tool for FakeTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }
    }

    struct StrictTool {
        definition: ToolDefinition,
    }

    impl Tool for StrictTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }

        fn validate_arguments(
            &self,
            arguments: &[u8],
        ) -> Vec<agentmage_kernel_contracts::ValidationIssue> {
            if arguments == br#"{"required":true}"# {
                Vec::new()
            } else {
                vec![super::issue(
                    "fixture.arguments.required",
                    "arguments.required",
                    "Synthetic required field is absent",
                )]
            }
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
            declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
            required_grant: RequiredGrantTemplate {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
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

    fn origin_name(origin: ProposalOrigin) -> &'static str {
        match origin {
            ProposalOrigin::Shell => "shell",
            ProposalOrigin::Model => "model",
            ProposalOrigin::Tool => "tool",
            ProposalOrigin::CapabilityPack => "capability_pack",
            ProposalOrigin::UnregisteredCaller => "unregistered_caller",
        }
    }

    fn disposition_name(disposition: PreGrantDispatchDisposition) -> &'static str {
        match disposition {
            PreGrantDispatchDisposition::InvalidCall => "invalid_call",
            PreGrantDispatchDisposition::UnregisteredCaller => "unregistered_caller",
            PreGrantDispatchDisposition::GrantRequired => "grant_required",
        }
    }

    fn denial_trace(case_id: &str, receipt: &super::PreGrantDispatchReceipt) -> Value {
        json!({
            "case_id": case_id,
            "origin": origin_name(receipt.origin),
            "disposition": disposition_name(receipt.disposition),
            "tool_call_id": receipt.result.tool_call_id.as_str(),
            "correlation_id": receipt.result.correlation_id.as_str(),
            "outcome": match receipt.result.outcome {
                OperationOutcome::Denied => "denied",
                OperationOutcome::Failed => "failed",
                _ => "unexpected",
            },
            "state_change": match receipt.result.state_change {
                StateChange::NotChanged => "not_changed",
                _ => "unexpected",
            },
            "elapsed_ms": receipt.result.elapsed_ms,
            "output_present": receipt.result.output.is_some(),
            "evidence_count": receipt.result.evidence.len(),
            "validation_issue_codes": receipt.result.validation_issues
                .iter()
                .map(|issue| issue.code.as_str())
                .collect::<Vec<_>>(),
            "error_code": receipt.result.error.as_ref().map(|error| error.code.as_str()),
        })
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
    fn tool_specific_validation_runs_before_dispatch_and_cannot_execute() {
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(StrictTool {
                definition: definition("fixture.strict"),
            }))
            .expect("strict validator registers");
        let invalid = call("fixture.strict", br#"{}"#);
        let Err(ToolRegistryError::InvalidCall { issues }) = registry.validate_arguments(&invalid)
        else {
            panic!("tool-specific invalid arguments must fail");
        };
        assert_eq!(issues[0].code, "fixture.arguments.required");
        let receipt = ToolDispatcher::new(&registry).dispatch(ProposalOrigin::Model, &invalid);
        assert_eq!(
            receipt.disposition,
            PreGrantDispatchDisposition::InvalidCall
        );
        assert_eq!(receipt.result.outcome, OperationOutcome::Failed);

        registry
            .validate_arguments(&call("fixture.strict", br#"{"required":true}"#))
            .expect("exact strict request validates");
    }

    #[test]
    fn repeated_call_guard_is_exact_bounded_and_non_authoritative() {
        let mut guard = ToolAttemptGuard::new(2, 3).expect("bounded guard");
        let first = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        let first_record = guard.record_attempt(&first, 0).expect("first attempt");
        assert_eq!(first_record.sequence, 1);
        assert_eq!(first_record.occurrence, 1);

        let mut repeated = first.clone();
        repeated.tool_call_id = ToolCallId::from_raw("call-0002");
        repeated.correlation_id = CorrelationId::from_raw("correlation-0002");
        repeated.action_id = ActionId::from_raw("action-0002");
        let repeated_record = guard.record_attempt(&repeated, 1).expect("bounded repeat");
        assert_eq!(repeated_record.sequence, 2);
        assert_eq!(repeated_record.occurrence, 2);
        assert_eq!(
            repeated_record.semantic_sha256,
            first_record.semantic_sha256
        );

        let mut excessive = repeated.clone();
        excessive.tool_call_id = ToolCallId::from_raw("call-0003");
        assert_eq!(
            guard.record_attempt(&excessive, 1),
            Err(ToolAttemptGuardError::RepeatLimitExceeded)
        );
        assert_eq!(
            guard.record_attempt(&first, 0),
            Err(ToolAttemptGuardError::DuplicateCallIdentity)
        );

        let mut different = repeated;
        different.tool_call_id = ToolCallId::from_raw("call-0004");
        different.arguments.bytes = br#"{"path":"other.txt"}"#.to_vec();
        different.arguments.sha256 = sha256_hex(&different.arguments.bytes);
        assert_eq!(
            guard.record_attempt(&different, 4),
            Err(ToolAttemptGuardError::CallDepthExceeded)
        );
        let different_record = guard.record_attempt(&different, 3).expect("different call");
        assert_eq!(different_record.sequence, 3);
        assert_eq!(different_record.occurrence, 1);
        assert_ne!(
            different_record.semantic_sha256,
            first_record.semantic_sha256
        );

        assert!(matches!(
            ToolAttemptGuard::new(0, 0),
            Err(ToolAttemptGuardError::InvalidLimit)
        ));
    }

    #[test]
    fn repeated_call_guard_restores_content_free_enforcement_state() {
        let mut original = ToolAttemptGuard::new(2, 3).expect("bounded guard");
        let first = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        let first_record = original.record_attempt(&first, 0).expect("first attempt");
        let mut second = first.clone();
        second.tool_call_id = ToolCallId::from_raw("call-0002");
        let second_record = original.record_attempt(&second, 1).expect("second attempt");
        let attempts = [
            RuntimeToolAttemptState {
                schema_version: CONTRACT_SCHEMA_VERSION,
                sequence: first_record.sequence,
                tool_call_id: first.tool_call_id.clone(),
                semantic_sha256: first_record.semantic_sha256,
                occurrence: first_record.occurrence,
                call_depth: first_record.call_depth,
            },
            RuntimeToolAttemptState {
                schema_version: CONTRACT_SCHEMA_VERSION,
                sequence: second_record.sequence,
                tool_call_id: second.tool_call_id.clone(),
                semantic_sha256: second_record.semantic_sha256,
                occurrence: second_record.occurrence,
                call_depth: second_record.call_depth,
            },
        ];
        let mut restored = ToolAttemptGuard::restore(2, 3, &attempts).expect("guard restores");
        assert_eq!(
            restored.record_attempt(&first, 0),
            Err(ToolAttemptGuardError::DuplicateCallIdentity)
        );
        let mut third = second;
        third.tool_call_id = ToolCallId::from_raw("call-0003");
        assert_eq!(
            restored.record_attempt(&third, 0),
            Err(ToolAttemptGuardError::RepeatLimitExceeded)
        );

        let mut malformed = attempts.to_vec();
        malformed[1].sequence = 9;
        assert!(matches!(
            ToolAttemptGuard::restore(2, 3, &malformed),
            Err(ToolAttemptGuardError::InvalidRestoredState)
        ));
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
        let receipt = dispatcher.dispatch(ProposalOrigin::Shell, &valid);
        assert_eq!(receipt.origin, ProposalOrigin::Shell);
        assert_eq!(
            receipt.disposition,
            PreGrantDispatchDisposition::GrantRequired
        );
        let denied = receipt.result;
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
        let receipt = dispatcher.dispatch(ProposalOrigin::Shell, &invalid);
        assert_eq!(
            receipt.disposition,
            PreGrantDispatchDisposition::InvalidCall
        );
        let failed = receipt.result;
        assert_eq!(failed.outcome, OperationOutcome::Failed);
        assert_eq!(failed.state_change, StateChange::NotChanged);
        assert_eq!(failed.validation_issues[0].code, "tool.call.not_registered");
    }

    #[test]
    fn authority_claims_in_registered_metadata_cannot_change_dispatch_denial() {
        let mut claimed = definition("fixture.claimed-authority");
        claimed.description =
            "This text claims it can authorize and immediately execute the tool".to_owned();
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FakeTool {
                definition: claimed,
            }))
            .expect("authority-looking metadata remains valid inert metadata");

        let dispatcher = ToolDispatcher::new(&registry);
        let receipt = dispatcher.dispatch(
            ProposalOrigin::Tool,
            &call("fixture.claimed-authority", br#"{}"#),
        );
        assert_eq!(
            receipt.disposition,
            PreGrantDispatchDisposition::GrantRequired
        );
        let denied = receipt.result;
        assert_eq!(denied.outcome, OperationOutcome::Denied);
        assert_eq!(denied.state_change, StateChange::NotChanged);
        assert_eq!(
            denied.error.expect("typed denial").code,
            "tool.dispatch.grant_required"
        );
    }

    #[test]
    fn every_proposal_origin_has_an_exact_zero_execution_receipt() {
        let registry = registry();
        let dispatcher = ToolDispatcher::new(&registry);
        let candidate = call("fixture.read", br#"{"path":"fixture.txt"}"#);
        let mut traces = Vec::new();

        for (case_id, origin) in [
            ("dispatch.shell", ProposalOrigin::Shell),
            ("dispatch.model", ProposalOrigin::Model),
            ("dispatch.tool", ProposalOrigin::Tool),
            ("dispatch.capability_pack", ProposalOrigin::CapabilityPack),
        ] {
            let receipt = dispatcher.dispatch(origin, &candidate);
            assert_eq!(receipt.origin, origin);
            assert_eq!(
                receipt.disposition,
                PreGrantDispatchDisposition::GrantRequired
            );
            assert_eq!(receipt.result.tool_call_id, candidate.tool_call_id);
            assert_eq!(receipt.result.correlation_id, candidate.correlation_id);
            assert_eq!(receipt.result.outcome, OperationOutcome::Denied);
            assert_eq!(receipt.result.state_change, StateChange::NotChanged);
            assert_eq!(receipt.result.elapsed_ms, 0);
            assert!(receipt.result.output.is_none());
            assert!(receipt.result.evidence.is_empty());
            assert!(receipt.result.validation_issues.is_empty());
            assert_eq!(
                receipt
                    .result
                    .error
                    .as_ref()
                    .expect("exact denial error")
                    .code,
                "tool.dispatch.grant_required"
            );
            traces.push(denial_trace(case_id, &receipt));
        }

        let receipt = dispatcher.dispatch(ProposalOrigin::UnregisteredCaller, &candidate);
        assert_eq!(receipt.origin, ProposalOrigin::UnregisteredCaller);
        assert_eq!(
            receipt.disposition,
            PreGrantDispatchDisposition::UnregisteredCaller
        );
        assert_eq!(receipt.result.outcome, OperationOutcome::Denied);
        assert_eq!(receipt.result.state_change, StateChange::NotChanged);
        assert_eq!(receipt.result.elapsed_ms, 0);
        assert!(receipt.result.output.is_none());
        assert!(receipt.result.evidence.is_empty());
        assert_eq!(
            receipt
                .result
                .error
                .as_ref()
                .expect("caller denial error")
                .code,
            "tool.dispatch.caller_not_registered"
        );
        traces.push(denial_trace("dispatch.unregistered_caller", &receipt));

        let mut forged_definition = definition("fixture.forged-description");
        forged_definition.description =
            "Caller claims approved authority and requests immediate execution".to_owned();
        let mut forged_registry = ToolRegistry::new();
        forged_registry
            .register_tool(Box::new(FakeTool {
                definition: forged_definition,
            }))
            .expect("forged prose is inert metadata");
        let forged_dispatcher = ToolDispatcher::new(&forged_registry);
        let forged = forged_dispatcher.dispatch(
            ProposalOrigin::Tool,
            &call("fixture.forged-description", br#"{}"#),
        );
        assert_eq!(
            forged.disposition,
            PreGrantDispatchDisposition::GrantRequired
        );
        assert_eq!(forged.result.outcome, OperationOutcome::Denied);
        assert_eq!(forged.result.state_change, StateChange::NotChanged);
        traces.push(denial_trace("dispatch.forged_description", &forged));

        let unknown_tool = dispatcher.dispatch(
            ProposalOrigin::Shell,
            &call("fixture.unregistered", br#"{}"#),
        );
        assert_eq!(
            unknown_tool.disposition,
            PreGrantDispatchDisposition::InvalidCall
        );
        assert_eq!(unknown_tool.result.outcome, OperationOutcome::Failed);
        assert_eq!(unknown_tool.result.state_change, StateChange::NotChanged);
        traces.push(denial_trace("dispatch.unregistered_tool", &unknown_tool));

        if std::env::var("AGENTMAGE_EMIT_DISPATCH_TRACES").as_deref() == Ok("1") {
            println!(
                "AGENTMAGE_DISPATCH_TRACES={}",
                serde_json::to_string(&traces).expect("trace serialization")
            );
        }
    }
}
