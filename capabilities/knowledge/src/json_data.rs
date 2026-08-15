//! Strict bounded JSON parsing, validation, redaction, and comparison.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, WorkspacePath};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

use crate::word_ooxml::word_sha256;

const MAX_SOURCE_BYTES: usize = 256 * 1_024 * 1_024;
const MAX_DEPTH: usize = 512;
const MAX_NODES: usize = 5_000_000;
const MAX_STRING_BYTES: usize = 16 * 1_024 * 1_024;

/// Closed resource and number-normalization profile for JSON processing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonProfile {
    /// Stable profile identity.
    pub profile_id: String,
    /// Maximum exact input bytes.
    pub maximum_source_bytes: usize,
    /// Maximum nested array or object depth.
    pub maximum_depth: usize,
    /// Maximum total values and object keys.
    pub maximum_nodes: usize,
    /// Maximum UTF-8 bytes in one key or string value.
    pub maximum_string_bytes: usize,
    /// Explicit number policy; decimal JSON numbers use finite IEEE-754 round-trip normalization.
    pub number_policy: String,
}

impl Default for StructuredJsonProfile {
    fn default() -> Self {
        Self {
            profile_id: "structured-json-strict-v1".to_owned(),
            maximum_source_bytes: 64 * 1_024 * 1_024,
            maximum_depth: 128,
            maximum_nodes: 1_000_000,
            maximum_string_bytes: 4 * 1_024 * 1_024,
            number_policy: "finite-ieee754-round-trip-v1".to_owned(),
        }
    }
}

impl StructuredJsonProfile {
    fn valid(&self) -> bool {
        valid_identifier(&self.profile_id)
            && self.maximum_source_bytes > 0
            && self.maximum_source_bytes <= MAX_SOURCE_BYTES
            && self.maximum_depth > 0
            && self.maximum_depth <= MAX_DEPTH
            && self.maximum_nodes > 0
            && self.maximum_nodes <= MAX_NODES
            && self.maximum_string_bytes > 0
            && self.maximum_string_bytes <= MAX_STRING_BYTES
            && self.number_policy == "finite-ieee754-round-trip-v1"
    }
}

/// Closed recursive JSON schema subset used by local structured-data workflows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StructuredJsonSchema {
    /// Any admitted JSON value.
    Any,
    /// JSON null.
    Null,
    /// JSON boolean.
    Boolean,
    /// JSON number.
    Number,
    /// JSON string with Unicode scalar-count bounds.
    String {
        /// Minimum character count.
        minimum_length: usize,
        /// Maximum character count.
        maximum_length: usize,
    },
    /// JSON array with item and length constraints.
    Array {
        /// Item schema.
        items: Box<StructuredJsonSchema>,
        /// Minimum item count.
        minimum_items: usize,
        /// Maximum item count.
        maximum_items: usize,
    },
    /// JSON object with exact property, required-key, and additional-key policy.
    Object {
        /// Property schemas keyed by exact property name.
        properties: BTreeMap<String, StructuredJsonSchema>,
        /// Exact required property names.
        required: BTreeSet<String>,
        /// Whether undeclared properties remain admissible.
        additional_properties: bool,
    },
}

/// One content-minimized schema validation issue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonIssue {
    /// RFC 6901 pointer to the failing value.
    pub pointer: String,
    /// Stable content-free reason code.
    pub reason_code: String,
}

/// Bounded canonical JSON document retaining exact source identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonDocument {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Canonical caller-authorized source path.
    pub source_path: WorkspacePath,
    /// SHA-256 of exact source bytes.
    pub source_sha256: String,
    /// Exact parser profile.
    pub profile: StructuredJsonProfile,
    /// Optional caller-declared schema identity.
    pub schema_id: Option<String>,
    /// Parsed JSON value with stable object-key serialization.
    pub value: Value,
    /// Stable compact JSON bytes.
    pub canonical_json: Vec<u8>,
    /// SHA-256 of stable compact JSON bytes.
    pub canonical_sha256: String,
    /// Canonically ordered schema issues.
    pub issues: Vec<StructuredJsonIssue>,
    /// True only when there are no schema issues.
    pub schema_valid: bool,
    /// True because caller source bytes remain authoritative.
    pub original_preserved: bool,
    /// False because parsing is in memory.
    pub filesystem_effect_performed: bool,
    /// False because JSON processing uses no network.
    pub network_access_performed: bool,
    /// False because values remain inert data.
    pub execution_performed: bool,
}

/// Exact redaction selection for canonical JSON regeneration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonRedactionPolicy {
    /// Stable policy identity.
    pub policy_id: String,
    /// Strictly sorted unique RFC 6901 pointers.
    pub pointers: Vec<String>,
    /// Strictly sorted unique exact key names redacted at every depth.
    pub key_names: Vec<String>,
}

/// Full-regeneration JSON redaction result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonRedactionEntry {
    /// Exact redacted RFC 6901 pointer.
    pub pointer: String,
    /// SHA-256 of the removed canonical JSON value.
    pub value_sha256: String,
}

/// Full-regeneration JSON redaction result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonRedaction {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Exact source digest.
    pub source_sha256: String,
    /// Exact source canonical digest.
    pub source_canonical_sha256: String,
    /// Stable redaction policy identity.
    pub policy_id: String,
    /// Canonically ordered pointer and removed-value digest records.
    pub redactions: Vec<StructuredJsonRedactionEntry>,
    /// Regenerated canonical JSON bytes.
    pub canonical_json: Vec<u8>,
    /// Exact regenerated byte digest.
    pub canonical_sha256: String,
    /// True because the source document remains unchanged.
    pub original_preserved: bool,
    /// False because this is an in-memory proposal.
    pub filesystem_effect_performed: bool,
    /// False because no network is used.
    pub network_access_performed: bool,
    /// False because JSON is never executed.
    pub execution_performed: bool,
}

/// Closed reason for one deterministic JSON difference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredJsonDifferenceReason {
    /// Pointer exists only in the right document.
    MissingLeft,
    /// Pointer exists only in the left document.
    MissingRight,
    /// Both values exist but their JSON types differ.
    TypeMismatch,
    /// Both values have the same JSON type but different canonical values.
    ValueMismatch,
}

/// One content-minimized JSON difference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonDifference {
    /// RFC 6901 pointer to the difference.
    pub pointer: String,
    /// Closed difference reason.
    pub reason: StructuredJsonDifferenceReason,
    /// Canonical left-value digest when present.
    pub left_value_sha256: Option<String>,
    /// Canonical right-value digest when present.
    pub right_value_sha256: Option<String>,
}

/// Deterministic content-minimized comparison between two JSON documents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredJsonComparison {
    /// Kernel contract schema version.
    pub schema_version: u16,
    /// Exact left source digest.
    pub left_source_sha256: String,
    /// Exact right source digest.
    pub right_source_sha256: String,
    /// Canonically ordered differences.
    pub differences: Vec<StructuredJsonDifference>,
    /// True only when no differences exist.
    pub exact_match: bool,
    /// False because comparison is in memory.
    pub filesystem_effect_performed: bool,
    /// False because no network is used.
    pub network_access_performed: bool,
    /// False because values remain inert data.
    pub execution_performed: bool,
}

/// Stable structured JSON failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructuredJsonError {
    /// Profile, schema identity, redaction policy, or comparison input is invalid.
    InvalidInput,
    /// JSON is malformed, has trailing bytes, or contains duplicate keys.
    MalformedJson,
    /// Source, nesting, node, string, schema, or output limit was exceeded.
    ResourceLimit,
    /// Prototype-like object keys are prohibited by the strict profile.
    ProhibitedKey,
    /// A requested redaction pointer does not exist.
    MissingRedactionTarget,
}

impl StructuredJsonError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "json.input.invalid",
            Self::MalformedJson => "json.malformed",
            Self::ResourceLimit => "json.resource.limit",
            Self::ProhibitedKey => "json.key.prohibited",
            Self::MissingRedactionTarget => "json.redaction.target-missing",
        }
    }
}

impl fmt::Display for StructuredJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for StructuredJsonError {}

struct UniqueJson(Value);

impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(UniqueJson)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<UniqueJson>()? {
            values.push(value.0);
        }
        Ok(UniqueJson(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            values.insert(key, object.next_value::<UniqueJson>()?.0);
        }
        Ok(UniqueJson(Value::Object(values)))
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn pointer_unescape(value: &str) -> Option<String> {
    let mut output = String::new();
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '~' {
            output.push(character);
            continue;
        }
        match characters.next()? {
            '0' => output.push('~'),
            '1' => output.push('/'),
            _ => return None,
        }
    }
    Some(output)
}

fn canonical(value: &Value) -> Result<Vec<u8>, StructuredJsonError> {
    serde_json::to_vec(value).map_err(|_| StructuredJsonError::ResourceLimit)
}

fn inspect_limits(
    value: &Value,
    profile: &StructuredJsonProfile,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), StructuredJsonError> {
    if depth > profile.maximum_depth {
        return Err(StructuredJsonError::ResourceLimit);
    }
    *nodes = nodes
        .checked_add(1)
        .ok_or(StructuredJsonError::ResourceLimit)?;
    if *nodes > profile.maximum_nodes {
        return Err(StructuredJsonError::ResourceLimit);
    }
    match value {
        Value::String(item) if item.len() > profile.maximum_string_bytes => {
            Err(StructuredJsonError::ResourceLimit)
        }
        Value::Array(items) => {
            for item in items {
                inspect_limits(item, profile, depth + 1, nodes)?;
            }
            Ok(())
        }
        Value::Object(items) => {
            for (key, item) in items {
                if key.len() > profile.maximum_string_bytes {
                    return Err(StructuredJsonError::ResourceLimit);
                }
                if matches!(key.as_str(), "__proto__" | "prototype" | "constructor") {
                    return Err(StructuredJsonError::ProhibitedKey);
                }
                *nodes = nodes
                    .checked_add(1)
                    .ok_or(StructuredJsonError::ResourceLimit)?;
                if *nodes > profile.maximum_nodes {
                    return Err(StructuredJsonError::ResourceLimit);
                }
                inspect_limits(item, profile, depth + 1, nodes)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn schema_issues(
    value: &Value,
    schema: &StructuredJsonSchema,
    pointer: &str,
    issues: &mut Vec<StructuredJsonIssue>,
) {
    let mismatch = |issues: &mut Vec<StructuredJsonIssue>| {
        issues.push(StructuredJsonIssue {
            pointer: pointer.to_owned(),
            reason_code: "json.schema.type-mismatch".to_owned(),
        });
    };
    match schema {
        StructuredJsonSchema::Any => {}
        StructuredJsonSchema::Null => {
            if !value.is_null() {
                mismatch(issues);
            }
        }
        StructuredJsonSchema::Boolean => {
            if !value.is_boolean() {
                mismatch(issues);
            }
        }
        StructuredJsonSchema::Number => {
            if !value.is_number() {
                mismatch(issues);
            }
        }
        StructuredJsonSchema::String {
            minimum_length,
            maximum_length,
        } => match value.as_str() {
            Some(item) if item.chars().count() < *minimum_length => {
                issues.push(StructuredJsonIssue {
                    pointer: pointer.to_owned(),
                    reason_code: "json.schema.string-too-short".to_owned(),
                });
            }
            Some(item) if item.chars().count() > *maximum_length => {
                issues.push(StructuredJsonIssue {
                    pointer: pointer.to_owned(),
                    reason_code: "json.schema.string-too-long".to_owned(),
                });
            }
            Some(_) => {}
            None => mismatch(issues),
        },
        StructuredJsonSchema::Array {
            items,
            minimum_items,
            maximum_items,
        } => match value.as_array() {
            Some(values) => {
                if values.len() < *minimum_items || values.len() > *maximum_items {
                    issues.push(StructuredJsonIssue {
                        pointer: pointer.to_owned(),
                        reason_code: "json.schema.array-length".to_owned(),
                    });
                }
                for (index, item) in values.iter().enumerate() {
                    schema_issues(item, items, &format!("{pointer}/{index}"), issues);
                }
            }
            None => mismatch(issues),
        },
        StructuredJsonSchema::Object {
            properties,
            required,
            additional_properties,
        } => match value.as_object() {
            Some(values) => {
                for key in required {
                    if !values.contains_key(key) {
                        issues.push(StructuredJsonIssue {
                            pointer: format!("{pointer}/{}", pointer_escape(key)),
                            reason_code: "json.schema.required-missing".to_owned(),
                        });
                    }
                }
                for (key, item) in values {
                    let child = format!("{pointer}/{}", pointer_escape(key));
                    if let Some(rule) = properties.get(key) {
                        schema_issues(item, rule, &child, issues);
                    } else if !additional_properties {
                        issues.push(StructuredJsonIssue {
                            pointer: child,
                            reason_code: "json.schema.additional-property".to_owned(),
                        });
                    }
                }
            }
            None => mismatch(issues),
        },
    }
}

/// Parses exact JSON bytes, rejects duplicate/prototype-like keys, applies bounds, and validates an optional closed schema.
pub fn parse_structured_json(
    source_path: WorkspacePath,
    source: &[u8],
    profile: &StructuredJsonProfile,
    schema: Option<(&str, &StructuredJsonSchema)>,
) -> Result<StructuredJsonDocument, StructuredJsonError> {
    if !profile.valid() || source.is_empty() || source.len() > profile.maximum_source_bytes {
        return Err(StructuredJsonError::InvalidInput);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(source);
    let value = UniqueJson::deserialize(&mut deserializer)
        .map_err(|_| StructuredJsonError::MalformedJson)?
        .0;
    deserializer
        .end()
        .map_err(|_| StructuredJsonError::MalformedJson)?;
    let mut nodes = 0;
    inspect_limits(&value, profile, 0, &mut nodes)?;
    let schema_id = schema.map(|item| item.0.to_owned());
    if schema_id
        .as_deref()
        .is_some_and(|item| !valid_identifier(item))
    {
        return Err(StructuredJsonError::InvalidInput);
    }
    let mut issues = Vec::new();
    if let Some((_, rule)) = schema {
        schema_issues(&value, rule, "", &mut issues);
    }
    issues.sort_by(|left, right| {
        (&left.pointer, &left.reason_code).cmp(&(&right.pointer, &right.reason_code))
    });
    let canonical_json = canonical(&value)?;
    if canonical_json.len() > profile.maximum_source_bytes {
        return Err(StructuredJsonError::ResourceLimit);
    }
    Ok(StructuredJsonDocument {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_path,
        source_sha256: word_sha256(source),
        profile: profile.clone(),
        schema_id,
        value,
        canonical_sha256: word_sha256(&canonical_json),
        canonical_json,
        schema_valid: issues.is_empty(),
        issues,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

fn strict_sorted(values: &[String]) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
}

fn pointer_segments(pointer: &str) -> Option<Vec<String>> {
    if pointer.is_empty() {
        return Some(Vec::new());
    }
    pointer
        .strip_prefix('/')?
        .split('/')
        .map(pointer_unescape)
        .collect()
}

fn value_mut_at<'a>(value: &'a mut Value, segments: &[String]) -> Option<&'a mut Value> {
    if segments.is_empty() {
        return Some(value);
    }
    match value {
        Value::Object(items) => value_mut_at(items.get_mut(&segments[0])?, &segments[1..]),
        Value::Array(items) => {
            let index = segments[0].parse::<usize>().ok()?;
            value_mut_at(items.get_mut(index)?, &segments[1..])
        }
        _ => None,
    }
}

fn redact_keys(
    value: &mut Value,
    keys: &BTreeSet<&str>,
    pointer: &str,
    redacted: &mut BTreeMap<String, String>,
) -> Result<(), StructuredJsonError> {
    match value {
        Value::Object(items) => {
            for (key, item) in items {
                let child = format!("{pointer}/{}", pointer_escape(key));
                if keys.contains(key.as_str()) {
                    redacted.insert(child, word_sha256(&canonical(item)?));
                    *item = Value::String("[REDACTED]".to_owned());
                } else {
                    redact_keys(item, keys, &child, redacted)?;
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                redact_keys(item, keys, &format!("{pointer}/{index}"), redacted)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Regenerates canonical JSON after exact pointer and key-name redaction without mutating the source document.
pub fn redact_structured_json(
    document: &StructuredJsonDocument,
    policy: &StructuredJsonRedactionPolicy,
) -> Result<StructuredJsonRedaction, StructuredJsonError> {
    if !valid_identifier(&policy.policy_id)
        || (!policy.pointers.is_empty() && !strict_sorted(&policy.pointers))
        || (!policy.key_names.is_empty() && !strict_sorted(&policy.key_names))
        || policy
            .pointers
            .iter()
            .any(|pointer| pointer_segments(pointer).is_none())
        || policy.key_names.iter().any(String::is_empty)
    {
        return Err(StructuredJsonError::InvalidInput);
    }
    let mut value = document.value.clone();
    let mut redacted = BTreeMap::new();
    let keys = policy.key_names.iter().map(String::as_str).collect();
    redact_keys(&mut value, &keys, "", &mut redacted)?;
    for pointer in &policy.pointers {
        if redacted.contains_key(pointer) {
            continue;
        }
        let target = value_mut_at(
            &mut value,
            &pointer_segments(pointer).ok_or(StructuredJsonError::InvalidInput)?,
        )
        .ok_or(StructuredJsonError::MissingRedactionTarget)?;
        redacted.insert(pointer.clone(), word_sha256(&canonical(target)?));
        *target = Value::String("[REDACTED]".to_owned());
    }
    let canonical_json = canonical(&value)?;
    if canonical_json.len() > document.profile.maximum_source_bytes {
        return Err(StructuredJsonError::ResourceLimit);
    }
    Ok(StructuredJsonRedaction {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_sha256: document.source_sha256.clone(),
        source_canonical_sha256: document.canonical_sha256.clone(),
        policy_id: policy.policy_id.clone(),
        redactions: redacted
            .into_iter()
            .map(|(pointer, value_sha256)| StructuredJsonRedactionEntry {
                pointer,
                value_sha256,
            })
            .collect(),
        canonical_sha256: word_sha256(&canonical_json),
        canonical_json,
        original_preserved: true,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    })
}

fn value_digest(value: &Value) -> String {
    word_sha256(&canonical(value).unwrap_or_default())
}

fn compare_values(
    left: Option<&Value>,
    right: Option<&Value>,
    pointer: &str,
    differences: &mut Vec<StructuredJsonDifference>,
) {
    match (left, right) {
        (Some(Value::Object(left)), Some(Value::Object(right))) => {
            for key in left.keys().chain(right.keys()).collect::<BTreeSet<_>>() {
                compare_values(
                    left.get(key),
                    right.get(key),
                    &format!("{pointer}/{}", pointer_escape(key)),
                    differences,
                );
            }
        }
        (Some(Value::Array(left)), Some(Value::Array(right))) => {
            for index in 0..left.len().max(right.len()) {
                compare_values(
                    left.get(index),
                    right.get(index),
                    &format!("{pointer}/{index}"),
                    differences,
                );
            }
        }
        (Some(left), Some(right)) if left == right => {}
        (Some(left), Some(right)) => differences.push(StructuredJsonDifference {
            pointer: pointer.to_owned(),
            reason: if value_kind(left) == value_kind(right) {
                StructuredJsonDifferenceReason::ValueMismatch
            } else {
                StructuredJsonDifferenceReason::TypeMismatch
            },
            left_value_sha256: Some(value_digest(left)),
            right_value_sha256: Some(value_digest(right)),
        }),
        (Some(left), None) => differences.push(StructuredJsonDifference {
            pointer: pointer.to_owned(),
            reason: StructuredJsonDifferenceReason::MissingRight,
            left_value_sha256: Some(value_digest(left)),
            right_value_sha256: None,
        }),
        (None, Some(right)) => differences.push(StructuredJsonDifference {
            pointer: pointer.to_owned(),
            reason: StructuredJsonDifferenceReason::MissingLeft,
            left_value_sha256: None,
            right_value_sha256: Some(value_digest(right)),
        }),
        (None, None) => {}
    }
}

/// Compares two parsed JSON documents with canonical pointers, hashes, and closed reason codes.
#[must_use]
pub fn compare_structured_json(
    left: &StructuredJsonDocument,
    right: &StructuredJsonDocument,
) -> StructuredJsonComparison {
    let mut differences = Vec::new();
    compare_values(Some(&left.value), Some(&right.value), "", &mut differences);
    StructuredJsonComparison {
        schema_version: CONTRACT_SCHEMA_VERSION,
        left_source_sha256: left.source_sha256.clone(),
        right_source_sha256: right.source_sha256.clone(),
        exact_match: differences.is_empty(),
        differences,
        filesystem_effect_performed: false,
        network_access_performed: false,
        execution_performed: false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::*;

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-json"),
            ["data", "input.json"],
        )
        .expect("path")
    }

    fn schema() -> StructuredJsonSchema {
        StructuredJsonSchema::Object {
            properties: BTreeMap::from([
                (
                    "id".to_owned(),
                    StructuredJsonSchema::String {
                        minimum_length: 1,
                        maximum_length: 32,
                    },
                ),
                ("active".to_owned(), StructuredJsonSchema::Boolean),
            ]),
            required: BTreeSet::from(["id".to_owned(), "active".to_owned()]),
            additional_properties: false,
        }
    }

    #[test]
    fn parses_stable_keys_numbers_nulls_and_closed_schema_without_effects() {
        let source = br#"{ "id": "A-1", "active": true }"#;
        let document = parse_structured_json(
            path(),
            source,
            &StructuredJsonProfile::default(),
            Some(("person-v1", &schema())),
        )
        .expect("parse");
        assert_eq!(document.canonical_json, br#"{"active":true,"id":"A-1"}"#);
        assert!(document.schema_valid);
        assert!(document.issues.is_empty());
        assert_eq!(document.source_sha256, word_sha256(source));
        assert!(!document.filesystem_effect_performed);
        assert!(!document.network_access_performed);
        assert!(!document.execution_performed);
    }

    #[test]
    fn rejects_duplicates_prototype_keys_trailing_input_and_resource_limits() {
        for source in [
            br#"{"a":1,"a":2}"#.as_slice(),
            br#"{"a":1} trailing"#.as_slice(),
        ] {
            assert_eq!(
                parse_structured_json(path(), source, &StructuredJsonProfile::default(), None),
                Err(StructuredJsonError::MalformedJson)
            );
        }
        assert_eq!(
            parse_structured_json(
                path(),
                br#"{"__proto__":{}}"#,
                &StructuredJsonProfile::default(),
                None,
            ),
            Err(StructuredJsonError::ProhibitedKey)
        );
        let tiny = StructuredJsonProfile {
            maximum_source_bytes: 4,
            ..StructuredJsonProfile::default()
        };
        assert!(parse_structured_json(path(), br#"{"a":1}"#, &tiny, None).is_err());
    }

    #[test]
    fn records_schema_failures_without_guessing_or_mutating_source() {
        let document = parse_structured_json(
            path(),
            br#"{"id":"","extra":1}"#,
            &StructuredJsonProfile::default(),
            Some(("person-v1", &schema())),
        )
        .expect("parse");
        assert!(!document.schema_valid);
        assert_eq!(document.issues.len(), 3);
        assert!(document.original_preserved);
    }

    #[test]
    fn redacts_exact_pointers_and_keys_by_regeneration_without_raw_value_ledger() {
        let document = parse_structured_json(
            path(),
            br#"{"account":{"email":"a@example.invalid","token":"secret"},"token":"other"}"#,
            &StructuredJsonProfile::default(),
            None,
        )
        .expect("parse");
        let result = redact_structured_json(
            &document,
            &StructuredJsonRedactionPolicy {
                policy_id: "redact-v1".to_owned(),
                pointers: vec!["/account/email".to_owned()],
                key_names: vec!["token".to_owned()],
            },
        )
        .expect("redact");
        let text = String::from_utf8(result.canonical_json.clone()).expect("utf8");
        assert!(!text.contains("secret"));
        assert!(!text.contains("other"));
        assert!(!text.contains("a@example.invalid"));
        assert_eq!(result.redactions.len(), 3);
        assert!(result.original_preserved);
    }

    #[test]
    fn compares_nested_objects_and_arrays_with_stable_hash_only_differences() {
        let left = parse_structured_json(
            path(),
            br#"{"a":[1,2],"b":true}"#,
            &StructuredJsonProfile::default(),
            None,
        )
        .expect("left");
        let right = parse_structured_json(
            path(),
            br#"{"a":[1,"2",3],"c":null}"#,
            &StructuredJsonProfile::default(),
            None,
        )
        .expect("right");
        let comparison = compare_structured_json(&left, &right);
        assert!(!comparison.exact_match);
        assert_eq!(
            comparison
                .differences
                .iter()
                .map(|item| item.pointer.as_str())
                .collect::<Vec<_>>(),
            ["/a/1", "/a/2", "/b", "/c"]
        );
        assert!(
            comparison.differences.iter().all(|item| {
                item.left_value_sha256.is_some() || item.right_value_sha256.is_some()
            })
        );
        assert!(!comparison.execution_performed);
    }
}
