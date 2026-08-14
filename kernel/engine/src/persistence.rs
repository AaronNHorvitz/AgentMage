//! Deterministic policy gate for data entering encrypted operational storage.

use std::collections::BTreeSet;
use std::fmt;

use serde::Serialize;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

const SCHEMA_VERSION: u16 = 1;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_FIELD_NAME_BYTES: usize = 64;
const MAX_FIELDS: usize = 64;
const MAX_FIELD_VALUE_BYTES: usize = 256 * 1024;
const MAX_TOTAL_VALUE_BYTES: usize = 1024 * 1024;
const DAY_MS: u64 = 86_400_000;

/// Canonical operational-record family admitted by schema version 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceRecordFamily {
    /// Session metadata.
    Sessions,
    /// Objective metadata.
    Objectives,
    /// Revision-controlled plans.
    Plans,
    /// Bounded tasks.
    Tasks,
    /// Proposed or completed actions.
    Actions,
    /// Minimized evidence.
    Evidence,
    /// User or policy decisions.
    Decisions,
    /// Capability grants.
    Grants,
    /// Terminal operation receipts.
    Receipts,
    /// Canonical checkpoints.
    Checkpoints,
    /// Content-addressed file observations.
    Files,
}

/// Closed sensitivity class applied before persistence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceSensitivity {
    /// Public material, still encrypted at rest.
    Public,
    /// Internal operational metadata.
    Internal,
    /// Private local-user data.
    Private,
    /// Data requiring a later restricted-data policy.
    Restricted,
}

/// Requested lifetime for one candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceRetentionIntent {
    /// Never durable.
    Ephemeral,
    /// Bounded by session retention.
    Session,
    /// Bounded by record-family retention.
    Retained,
}

/// Explicit minimization rule for one field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceFieldHandling {
    /// Retain validated UTF-8 inside SQLCipher.
    Persist,
    /// Retain only byte count and SHA-256 identity.
    DigestOnly,
    /// Retain neither value nor digest.
    Ephemeral,
}

/// Raw content class that is structurally ineligible for durable storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EphemeralContentClass {
    /// Original attachment bytes.
    RawAttachment,
    /// Complete unminimized tool output.
    FullToolOutput,
    /// Environment-variable name or value material.
    EnvironmentVariable,
    /// Complete user or assembled prompt content.
    Prompt,
    /// Complete unminimized model response.
    ModelResponse,
}

/// One bounded field submitted to the gate.
pub struct PersistenceField<'a> {
    name: &'a str,
    value: &'a [u8],
    handling: PersistenceFieldHandling,
    ephemeral_class: Option<EphemeralContentClass>,
}

impl<'a> PersistenceField<'a> {
    #[cfg(test)]
    const fn new(name: &'a str, value: &'a [u8], handling: PersistenceFieldHandling) -> Self {
        Self::operational_metadata(name, value, handling)
    }

    /// Creates bounded operational metadata validated during evaluation.
    #[must_use]
    pub const fn operational_metadata(
        name: &'a str,
        value: &'a [u8],
        handling: PersistenceFieldHandling,
    ) -> Self {
        Self {
            name,
            value,
            handling,
            ephemeral_class: None,
        }
    }

    /// Creates raw content that can only remain ephemeral.
    #[must_use]
    pub const fn ephemeral_content(
        name: &'a str,
        value: &'a [u8],
        class: EphemeralContentClass,
    ) -> Self {
        Self {
            name,
            value,
            handling: PersistenceFieldHandling::Ephemeral,
            ephemeral_class: Some(class),
        }
    }

    const fn is_structurally_ephemeral(&self) -> bool {
        self.ephemeral_class.is_some()
    }
}

impl fmt::Debug for PersistenceField<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersistenceField")
            .field("name_bytes", &self.name.len())
            .field("value_bytes", &self.value.len())
            .field("handling", &self.handling)
            .field("ephemeral_class", &self.ephemeral_class)
            .finish_non_exhaustive()
    }
}

/// Complete candidate considered for durable storage.
pub struct PersistenceCandidate<'a> {
    /// Closed record family.
    pub family: PersistenceRecordFamily,
    /// Stable non-secret record identity.
    pub record_id: &'a str,
    /// Trusted sensitivity selection.
    pub sensitivity: PersistenceSensitivity,
    /// Requested lifetime.
    pub retention: PersistenceRetentionIntent,
    /// Trusted wall-clock value.
    pub occurred_at_epoch_ms: u64,
    /// Explicitly handled fields.
    pub fields: &'a [PersistenceField<'a>],
}

impl fmt::Debug for PersistenceCandidate<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersistenceCandidate")
            .field("family", &self.family)
            .field("record_id_bytes", &self.record_id.len())
            .field("sensitivity", &self.sensitivity)
            .field("retention", &self.retention)
            .field("field_count", &self.fields.len())
            .finish_non_exhaustive()
    }
}

/// Known secret signature found by the deterministic scanner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretFindingClass {
    /// Credential-like field name.
    CredentialField,
    /// Private-key envelope.
    PrivateKey,
    /// Bearer authorization value.
    BearerCredential,
    /// Known provider-token prefix.
    ProviderToken,
    /// Cloud access-key identifier.
    CloudAccessKey,
    /// URI with embedded user information.
    EmbeddedUriCredential,
}

/// Scans one named byte value with the deterministic persistence secret detector.
///
/// The result contains only stable finding classes and never retains or returns the
/// candidate value. Callers must still apply their own boundary-specific policy.
#[must_use]
pub fn detect_secret_classes(name: &str, value: &[u8]) -> Vec<SecretFindingClass> {
    detect(name, value).into_iter().collect()
}

/// Storage encryption selected by the gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceEncryption {
    /// No record may be written.
    None,
    /// The admitted SQLCipher operational store.
    SqlCipherOperationalStore,
}

/// Terminal gate outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceOutcome {
    /// Candidate remains memory-only.
    Ephemeral,
    /// Minimized record is admitted.
    Admitted,
    /// Persist-marked field contained a secret signature.
    DeniedSecret,
    /// Restricted-data policy is unavailable.
    DeniedRestricted,
}

/// Assigned lifecycle for an admitted record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersistenceRetentionAssignment {
    disposition: PersistenceRetentionIntent,
    expires_at_epoch_ms: u64,
    legal_hold: bool,
    policy_sha256: String,
}

impl PersistenceRetentionAssignment {
    /// Returns the assigned disposition.
    #[must_use]
    pub const fn disposition(&self) -> PersistenceRetentionIntent {
        self.disposition
    }
    /// Returns the exclusive expiration boundary.
    #[must_use]
    pub const fn expires_at_epoch_ms(&self) -> u64 {
        self.expires_at_epoch_ms
    }
    /// Returns the policy identity.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }
}

/// Content-free receipt for one complete decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PersistenceReceipt {
    schema_version: u16,
    candidate_shape_sha256: String,
    record_family: PersistenceRecordFamily,
    record_id_sha256: String,
    sensitivity: PersistenceSensitivity,
    requested_retention: PersistenceRetentionIntent,
    outcome: PersistenceOutcome,
    encryption: PersistenceEncryption,
    retention: Option<PersistenceRetentionAssignment>,
    secret_findings: Vec<SecretFindingClass>,
    persisted_fields: u16,
    digested_fields: u16,
    omitted_fields: u16,
    minimized_record_sha256: Option<String>,
    policy_sha256: String,
    occurred_at_epoch_ms: u64,
    receipt_sha256: String,
}

impl PersistenceReceipt {
    /// Returns the terminal outcome.
    #[must_use]
    pub const fn outcome(&self) -> PersistenceOutcome {
        self.outcome
    }
    /// Returns the encryption disposition.
    #[must_use]
    pub const fn encryption(&self) -> PersistenceEncryption {
        self.encryption
    }
    /// Returns ordered unique secret classes without values or value digests.
    #[must_use]
    pub fn secret_findings(&self) -> &[SecretFindingClass] {
        &self.secret_findings
    }
    /// Returns the receipt identity.
    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }
    /// Serializes a receipt containing no candidate field values.
    pub fn to_json(&self) -> Result<Vec<u8>, PersistenceError> {
        serde_json::to_vec(self).map_err(|_| PersistenceError::SerializationFailed)
    }
}

/// Minimized bytes and lifecycle metadata eligible for SQLCipher.
pub struct PreparedPersistence {
    record_json: Zeroizing<Vec<u8>>,
    record_sha256: String,
    retention: PersistenceRetentionAssignment,
}

impl PreparedPersistence {
    /// Exposes minimized bytes only during one callback.
    pub fn with_record_json<T>(&self, operation: impl FnOnce(&[u8]) -> T) -> T {
        operation(&self.record_json)
    }
    /// Returns the minimized-record identity.
    #[must_use]
    pub fn record_sha256(&self) -> &str {
        &self.record_sha256
    }
    /// Returns assigned lifecycle metadata.
    #[must_use]
    pub const fn retention(&self) -> &PersistenceRetentionAssignment {
        &self.retention
    }
}

impl fmt::Debug for PreparedPersistence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedPersistence")
            .field("record_bytes", &self.record_json.len())
            .field("record_sha256", &self.record_sha256)
            .field("retention", &self.retention)
            .finish_non_exhaustive()
    }
}

/// Receipt plus optional bytes admitted for storage.
#[derive(Debug)]
pub struct PersistenceDecision {
    receipt: PersistenceReceipt,
    prepared: Option<PreparedPersistence>,
}

impl PersistenceDecision {
    /// Returns the content-free receipt.
    #[must_use]
    pub const fn receipt(&self) -> &PersistenceReceipt {
        &self.receipt
    }
    /// Returns bytes only for an admitted decision.
    #[must_use]
    pub const fn prepared(&self) -> Option<&PreparedPersistence> {
        self.prepared.as_ref()
    }
}

/// Stable failure before a policy decision can be formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistenceError {
    /// Invalid identifier, duplicate name, or non-UTF-8 persisted value.
    InvalidCandidate,
    /// Closed field or byte bound exceeded.
    ResourceLimit,
    /// Retention arithmetic overflowed.
    ClockOverflow,
    /// Internal deterministic serialization failed.
    SerializationFailed,
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidCandidate => "persistence.candidate.invalid",
            Self::ResourceLimit => "persistence.resource.exceeded",
            Self::ClockOverflow => "persistence.clock.overflow",
            Self::SerializationFailed => "persistence.serialization.failed",
        })
    }
}
impl std::error::Error for PersistenceError {}

/// Fixed bounded policy matching public retention-schema day limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistencePolicy {
    sessions_days: u16,
    receipts_days: u16,
    policy_sha256: String,
}

impl PersistencePolicy {
    /// Creates a deterministic policy; restricted persistence remains disabled.
    pub fn new(sessions_days: u16, receipts_days: u16) -> Result<Self, PersistenceError> {
        if sessions_days > 3650 || !(1..=3650).contains(&receipts_days) {
            return Err(PersistenceError::InvalidCandidate);
        }
        let bytes = serde_json::to_vec(&(SCHEMA_VERSION, sessions_days, receipts_days, false))
            .map_err(|_| PersistenceError::SerializationFailed)?;
        Ok(Self {
            sessions_days,
            receipts_days,
            policy_sha256: sha256_hex(&bytes),
        })
    }
    /// Returns the policy identity.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Classifies, minimizes, assigns retention, and emits one receipt.
    pub fn evaluate(
        &self,
        candidate: &PersistenceCandidate<'_>,
    ) -> Result<PersistenceDecision, PersistenceError> {
        validate_candidate(candidate)?;
        let scan = scan(candidate);
        let shape = shape_sha256(candidate, &scan)?;
        let only_structurally_ephemeral = candidate
            .fields
            .iter()
            .all(PersistenceField::is_structurally_ephemeral);
        let denied = if candidate.sensitivity == PersistenceSensitivity::Restricted {
            Some(PersistenceOutcome::DeniedRestricted)
        } else if scan.persisted_secret {
            Some(PersistenceOutcome::DeniedSecret)
        } else if candidate.retention == PersistenceRetentionIntent::Ephemeral
            || only_structurally_ephemeral
        {
            Some(PersistenceOutcome::Ephemeral)
        } else {
            None
        };
        if let Some(outcome) = denied {
            return self.finish(
                candidate,
                scan,
                shape,
                outcome,
                None,
                Counts {
                    omitted: candidate.fields.len() as u16,
                    ..Counts::default()
                },
            );
        }
        let (record_json, counts) = minimize(candidate, &scan.secret_fields)?;
        let retention = self.retention(candidate)?;
        let record_sha256 = sha256_hex(&record_json);
        let prepared = PreparedPersistence {
            record_json: Zeroizing::new(record_json),
            record_sha256,
            retention,
        };
        self.finish(
            candidate,
            scan,
            shape,
            PersistenceOutcome::Admitted,
            Some(prepared),
            counts,
        )
    }

    fn retention(
        &self,
        candidate: &PersistenceCandidate<'_>,
    ) -> Result<PersistenceRetentionAssignment, PersistenceError> {
        let days = if candidate.family == PersistenceRecordFamily::Receipts {
            self.receipts_days
        } else {
            self.sessions_days
        };
        let expires_at_epoch_ms = candidate
            .occurred_at_epoch_ms
            .checked_add(
                u64::from(days)
                    .checked_mul(DAY_MS)
                    .ok_or(PersistenceError::ClockOverflow)?,
            )
            .ok_or(PersistenceError::ClockOverflow)?;
        Ok(PersistenceRetentionAssignment {
            disposition: candidate.retention,
            expires_at_epoch_ms,
            legal_hold: false,
            policy_sha256: self.policy_sha256.clone(),
        })
    }

    fn finish(
        &self,
        candidate: &PersistenceCandidate<'_>,
        scan: Scan,
        shape: String,
        outcome: PersistenceOutcome,
        prepared: Option<PreparedPersistence>,
        counts: Counts,
    ) -> Result<PersistenceDecision, PersistenceError> {
        let encryption = if prepared.is_some() {
            PersistenceEncryption::SqlCipherOperationalStore
        } else {
            PersistenceEncryption::None
        };
        let retention = prepared.as_ref().map(|value| value.retention.clone());
        let minimized_record_sha256 = prepared.as_ref().map(|value| value.record_sha256.clone());
        let safe = (
            SCHEMA_VERSION,
            &shape,
            candidate.family,
            sha256_hex(candidate.record_id.as_bytes()),
            candidate.sensitivity,
            candidate.retention,
            outcome,
            encryption,
            &retention,
            &scan.findings,
            counts.persisted,
            counts.digested,
            counts.omitted,
            &minimized_record_sha256,
            &self.policy_sha256,
            candidate.occurred_at_epoch_ms,
        );
        let receipt_sha256 = sha256_hex(
            &serde_json::to_vec(&safe).map_err(|_| PersistenceError::SerializationFailed)?,
        );
        Ok(PersistenceDecision {
            receipt: PersistenceReceipt {
                schema_version: SCHEMA_VERSION,
                candidate_shape_sha256: shape,
                record_family: candidate.family,
                record_id_sha256: sha256_hex(candidate.record_id.as_bytes()),
                sensitivity: candidate.sensitivity,
                requested_retention: candidate.retention,
                outcome,
                encryption,
                retention,
                secret_findings: scan.findings,
                persisted_fields: counts.persisted,
                digested_fields: counts.digested,
                omitted_fields: counts.omitted,
                minimized_record_sha256,
                policy_sha256: self.policy_sha256.clone(),
                occurred_at_epoch_ms: candidate.occurred_at_epoch_ms,
                receipt_sha256,
            },
            prepared,
        })
    }
}

struct Scan {
    findings: Vec<SecretFindingClass>,
    secret_fields: BTreeSet<usize>,
    persisted_secret: bool,
}
#[derive(Clone, Copy, Default)]
struct Counts {
    persisted: u16,
    digested: u16,
    omitted: u16,
}

#[derive(Serialize)]
#[serde(tag = "representation", rename_all = "snake_case")]
enum MinimizedField {
    Text {
        name: String,
        value: String,
    },
    Digest {
        name: String,
        byte_count: u64,
        sha256: String,
    },
}

fn validate_candidate(candidate: &PersistenceCandidate<'_>) -> Result<(), PersistenceError> {
    if !valid_id(candidate.record_id, MAX_IDENTIFIER_BYTES) || candidate.fields.is_empty() {
        return Err(PersistenceError::InvalidCandidate);
    }
    if candidate.fields.len() > MAX_FIELDS {
        return Err(PersistenceError::ResourceLimit);
    }
    let mut names = BTreeSet::new();
    let mut total = 0usize;
    for field in candidate.fields {
        if !valid_id(field.name, MAX_FIELD_NAME_BYTES) || !names.insert(field.name) {
            return Err(PersistenceError::InvalidCandidate);
        }
        if field.value.len() > MAX_FIELD_VALUE_BYTES {
            return Err(PersistenceError::ResourceLimit);
        }
        total = total
            .checked_add(field.value.len())
            .ok_or(PersistenceError::ResourceLimit)?;
        if total > MAX_TOTAL_VALUE_BYTES {
            return Err(PersistenceError::ResourceLimit);
        }
        if field.handling == PersistenceFieldHandling::Persist
            && std::str::from_utf8(field.value).is_err()
        {
            return Err(PersistenceError::InvalidCandidate);
        }
        if field.is_structurally_ephemeral()
            && field.handling != PersistenceFieldHandling::Ephemeral
        {
            return Err(PersistenceError::InvalidCandidate);
        }
    }
    Ok(())
}

fn scan(candidate: &PersistenceCandidate<'_>) -> Scan {
    let mut findings = BTreeSet::new();
    let mut secret_fields = BTreeSet::new();
    let mut persisted_secret = false;
    for (index, field) in candidate.fields.iter().enumerate() {
        let found = detect(field.name, field.value);
        if !found.is_empty() {
            findings.extend(found);
            secret_fields.insert(index);
            persisted_secret |= field.handling == PersistenceFieldHandling::Persist;
        }
    }
    Scan {
        findings: findings.into_iter().collect(),
        secret_fields,
        persisted_secret,
    }
}

fn minimize(
    candidate: &PersistenceCandidate<'_>,
    secret_fields: &BTreeSet<usize>,
) -> Result<(Vec<u8>, Counts), PersistenceError> {
    let mut output = Vec::new();
    let mut counts = Counts::default();
    for (index, field) in candidate.fields.iter().enumerate() {
        if secret_fields.contains(&index)
            || field.is_structurally_ephemeral()
            || field.handling == PersistenceFieldHandling::Ephemeral
        {
            counts.omitted += 1;
            continue;
        }
        match field.handling {
            PersistenceFieldHandling::Persist => {
                output.push(MinimizedField::Text {
                    name: field.name.to_owned(),
                    value: std::str::from_utf8(field.value)
                        .map_err(|_| PersistenceError::InvalidCandidate)?
                        .to_owned(),
                });
                counts.persisted += 1;
            }
            PersistenceFieldHandling::DigestOnly => {
                output.push(MinimizedField::Digest {
                    name: field.name.to_owned(),
                    byte_count: field.value.len() as u64,
                    sha256: sha256_hex(field.value),
                });
                counts.digested += 1;
            }
            PersistenceFieldHandling::Ephemeral => unreachable!(),
        }
    }
    serde_json::to_vec(&(
        SCHEMA_VERSION,
        candidate.family,
        candidate.record_id,
        output,
    ))
    .map(|bytes| (bytes, counts))
    .map_err(|_| PersistenceError::SerializationFailed)
}

fn shape_sha256(
    candidate: &PersistenceCandidate<'_>,
    scan: &Scan,
) -> Result<String, PersistenceError> {
    let fields: Vec<_> = candidate
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            (
                field.name,
                field.value.len(),
                field.handling,
                field.ephemeral_class,
                scan.secret_fields.contains(&index),
            )
        })
        .collect();
    serde_json::to_vec(&(
        SCHEMA_VERSION,
        candidate.family,
        sha256_hex(candidate.record_id.as_bytes()),
        candidate.sensitivity,
        candidate.retention,
        fields,
        &scan.findings,
    ))
    .map(|bytes| sha256_hex(&bytes))
    .map_err(|_| PersistenceError::SerializationFailed)
}

fn detect(name: &str, value: &[u8]) -> BTreeSet<SecretFindingClass> {
    let mut found = BTreeSet::new();
    let mut name = lower(name.as_bytes());
    for byte in name.iter_mut() {
        if matches!(byte, b'-' | b'.') {
            *byte = b'_';
        }
    }
    if [
        b"password".as_slice(),
        b"passwd",
        b"token",
        b"api_key",
        b"apikey",
        b"secret",
        b"authorization",
        b"cookie",
        b"private_key",
        b"client_secret",
        b"access_key",
        b"credential",
    ]
    .iter()
    .any(|needle| contains(&name, needle))
    {
        found.insert(SecretFindingClass::CredentialField);
    }
    let normalized = lower(value);
    if contains(&normalized, b"-----begin ") && contains(&normalized, b"private key-----") {
        found.insert(SecretFindingClass::PrivateKey);
    }
    if contains(&normalized, b"bearer ") || contains(&normalized, b"authorization: bearer") {
        found.insert(SecretFindingClass::BearerCredential);
    }
    if [
        b"ghp_".as_slice(),
        b"github_pat_",
        b"xoxb-",
        b"xoxp-",
        b"sk-",
    ]
    .iter()
    .any(|prefix| token_prefix(&normalized, prefix, 20))
    {
        found.insert(SecretFindingClass::ProviderToken);
    }
    if value.windows(20).any(|part| {
        part.starts_with(b"AKIA")
            && part[4..]
                .iter()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    }) {
        found.insert(SecretFindingClass::CloudAccessKey);
    }
    if [b"http".as_slice(), b"https", b"postgres", b"mysql"]
        .iter()
        .any(|scheme| userinfo_uri(&normalized, scheme))
    {
        found.insert(SecretFindingClass::EmbeddedUriCredential);
    }
    found
}

fn lower(value: &[u8]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(value.iter().map(u8::to_ascii_lowercase).collect())
}
fn contains(value: &[u8], needle: &[u8]) -> bool {
    value.windows(needle.len()).any(|part| part == needle)
}
fn token_prefix(value: &[u8], prefix: &[u8], minimum: usize) -> bool {
    value
        .windows(prefix.len())
        .enumerate()
        .any(|(index, part)| {
            part == prefix
                && prefix.len()
                    + value[index + prefix.len()..]
                        .iter()
                        .take_while(|byte| {
                            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                        })
                        .count()
                    >= minimum
        })
}
fn userinfo_uri(value: &[u8], scheme: &[u8]) -> bool {
    value
        .windows(scheme.len())
        .position(|part| part == scheme)
        .is_some_and(|start| {
            let delimiter = value.get(start + scheme.len()..start + scheme.len() + 3);
            if delimiter != Some(b"://") {
                return false;
            }
            let rest = &value[start + scheme.len() + 3..];
            let end = rest
                .iter()
                .position(|byte| matches!(byte, b'/' | b'?' | b'#'))
                .unwrap_or(rest.len());
            rest[..end].contains(&b'@')
        })
}
fn valid_id(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}
fn sha256_hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        use fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> PersistencePolicy {
        PersistencePolicy::new(30, 365).expect("policy")
    }
    fn candidate<'a>(
        fields: &'a [PersistenceField<'a>],
        sensitivity: PersistenceSensitivity,
        retention: PersistenceRetentionIntent,
    ) -> PersistenceCandidate<'a> {
        PersistenceCandidate {
            family: PersistenceRecordFamily::Sessions,
            record_id: "session-0001",
            sensitivity,
            retention,
            occurred_at_epoch_ms: 1_000,
            fields,
        }
    }

    #[test]
    fn admission_minimizes_encrypts_assigns_retention_and_receipts() {
        let fields = [
            PersistenceField::new("title", b"bounded", PersistenceFieldHandling::Persist),
            PersistenceField::new(
                "content",
                b"synthetic",
                PersistenceFieldHandling::DigestOnly,
            ),
            PersistenceField::new("scratch", b"discarded", PersistenceFieldHandling::Ephemeral),
        ];
        let decision = policy()
            .evaluate(&candidate(
                &fields,
                PersistenceSensitivity::Private,
                PersistenceRetentionIntent::Session,
            ))
            .expect("decision");
        assert_eq!(decision.receipt().outcome(), PersistenceOutcome::Admitted);
        assert_eq!(
            decision.receipt().encryption(),
            PersistenceEncryption::SqlCipherOperationalStore
        );
        let prepared = decision.prepared().expect("prepared");
        assert_eq!(
            prepared.retention().expires_at_epoch_ms(),
            1_000 + 30 * DAY_MS
        );
        prepared.with_record_json(|bytes| {
            let text = String::from_utf8_lossy(bytes);
            assert!(text.contains("bounded"));
            assert!(text.contains(&sha256_hex(b"synthetic")));
            assert!(!text.contains("discarded"));
        });
    }

    #[test]
    fn persisted_secret_denies_without_value_or_value_digest_in_receipt() {
        let mut secret = String::from("gh");
        secret.push_str("p_");
        secret.push_str(&"A".repeat(32));
        let fields = [PersistenceField::new(
            "summary",
            secret.as_bytes(),
            PersistenceFieldHandling::Persist,
        )];
        let decision = policy()
            .evaluate(&candidate(
                &fields,
                PersistenceSensitivity::Private,
                PersistenceRetentionIntent::Session,
            ))
            .expect("receipt");
        assert_eq!(
            decision.receipt().outcome(),
            PersistenceOutcome::DeniedSecret
        );
        assert!(decision.prepared().is_none());
        let receipt = String::from_utf8(decision.receipt().to_json().expect("json")).expect("utf8");
        assert!(!receipt.contains(&secret));
        assert!(!receipt.contains(&sha256_hex(secret.as_bytes())));
    }

    #[test]
    fn secrets_in_omitted_fields_are_neither_persisted_nor_digested() {
        let secret = format!("Bearer {}", "q".repeat(32));
        for handling in [
            PersistenceFieldHandling::Ephemeral,
            PersistenceFieldHandling::DigestOnly,
        ] {
            let fields = [
                PersistenceField::new("title", b"safe", PersistenceFieldHandling::Persist),
                PersistenceField::new("transient", secret.as_bytes(), handling),
            ];
            let decision = policy()
                .evaluate(&candidate(
                    &fields,
                    PersistenceSensitivity::Internal,
                    PersistenceRetentionIntent::Retained,
                ))
                .expect("decision");
            decision
                .prepared()
                .expect("prepared")
                .with_record_json(|bytes| {
                    let text = String::from_utf8_lossy(bytes);
                    assert!(!text.contains(&secret));
                    assert!(!text.contains(&sha256_hex(secret.as_bytes())));
                });
        }
    }

    #[test]
    fn restricted_and_ephemeral_candidates_select_no_storage() {
        let fields = [PersistenceField::new(
            "title",
            b"safe",
            PersistenceFieldHandling::Persist,
        )];
        for (sensitivity, retention, outcome) in [
            (
                PersistenceSensitivity::Restricted,
                PersistenceRetentionIntent::Retained,
                PersistenceOutcome::DeniedRestricted,
            ),
            (
                PersistenceSensitivity::Public,
                PersistenceRetentionIntent::Ephemeral,
                PersistenceOutcome::Ephemeral,
            ),
        ] {
            let decision = policy()
                .evaluate(&candidate(&fields, sensitivity, retention))
                .expect("decision");
            assert_eq!(decision.receipt().outcome(), outcome);
            assert_eq!(decision.receipt().encryption(), PersistenceEncryption::None);
            assert!(decision.prepared().is_none());
        }
    }

    #[test]
    fn every_raw_content_class_is_ephemeral_without_value_or_digest_receipt() {
        for class in [
            EphemeralContentClass::RawAttachment,
            EphemeralContentClass::FullToolOutput,
            EphemeralContentClass::EnvironmentVariable,
            EphemeralContentClass::Prompt,
            EphemeralContentClass::ModelResponse,
        ] {
            let first = [PersistenceField::ephemeral_content(
                "raw_content",
                b"alpha",
                class,
            )];
            let second = [PersistenceField::ephemeral_content(
                "raw_content",
                b"bravo",
                class,
            )];
            let first_decision = policy()
                .evaluate(&candidate(
                    &first,
                    PersistenceSensitivity::Private,
                    PersistenceRetentionIntent::Retained,
                ))
                .expect("first decision");
            let second_decision = policy()
                .evaluate(&candidate(
                    &second,
                    PersistenceSensitivity::Private,
                    PersistenceRetentionIntent::Retained,
                ))
                .expect("second decision");

            assert_eq!(
                first_decision.receipt().outcome(),
                PersistenceOutcome::Ephemeral
            );
            assert_eq!(
                first_decision.receipt().encryption(),
                PersistenceEncryption::None
            );
            assert!(first_decision.prepared().is_none());
            assert_eq!(
                first_decision.receipt().receipt_sha256(),
                second_decision.receipt().receipt_sha256()
            );
            let receipt =
                String::from_utf8(first_decision.receipt().to_json().expect("json")).expect("utf8");
            for raw in [b"alpha".as_slice(), b"bravo"] {
                assert!(!receipt.contains(std::str::from_utf8(raw).expect("ascii")));
                assert!(!receipt.contains(&sha256_hex(raw)));
            }
        }
    }

    #[test]
    fn raw_content_is_omitted_when_minimized_metadata_is_admitted() {
        for class in [
            EphemeralContentClass::RawAttachment,
            EphemeralContentClass::FullToolOutput,
            EphemeralContentClass::EnvironmentVariable,
            EphemeralContentClass::Prompt,
            EphemeralContentClass::ModelResponse,
        ] {
            let raw = format!("private-raw-{class:?}");
            let fields = [
                PersistenceField::operational_metadata(
                    "summary",
                    b"bounded",
                    PersistenceFieldHandling::Persist,
                ),
                PersistenceField::ephemeral_content("raw_content", raw.as_bytes(), class),
            ];
            let decision = policy()
                .evaluate(&candidate(
                    &fields,
                    PersistenceSensitivity::Private,
                    PersistenceRetentionIntent::Session,
                ))
                .expect("decision");
            assert_eq!(decision.receipt().outcome(), PersistenceOutcome::Admitted);
            decision
                .prepared()
                .expect("prepared metadata")
                .with_record_json(|record| {
                    let record = String::from_utf8_lossy(record);
                    assert!(record.contains("bounded"));
                    assert!(!record.contains(&raw));
                    assert!(!record.contains(&sha256_hex(raw.as_bytes())));
                });
            let receipt =
                String::from_utf8(decision.receipt().to_json().expect("json")).expect("utf8");
            assert!(!receipt.contains(&raw));
            assert!(!receipt.contains(&sha256_hex(raw.as_bytes())));
        }
    }

    #[test]
    fn forged_raw_content_handling_fails_before_policy_decision() {
        let fields = [PersistenceField {
            name: "raw_content",
            value: b"must-remain-ephemeral",
            handling: PersistenceFieldHandling::Persist,
            ephemeral_class: Some(EphemeralContentClass::Prompt),
        }];
        assert_eq!(
            policy()
                .evaluate(&candidate(
                    &fields,
                    PersistenceSensitivity::Private,
                    PersistenceRetentionIntent::Retained,
                ))
                .expect_err("forged handling must fail"),
            PersistenceError::InvalidCandidate
        );
    }

    #[test]
    fn scanner_covers_all_declared_classes() {
        let provider = format!("sk-{}", "z".repeat(32));
        let cloud = format!("AKIA{}", "A1".repeat(8));
        let cases: Vec<(&str, Vec<u8>, SecretFindingClass)> = vec![
            (
                "password",
                b"ordinary".to_vec(),
                SecretFindingClass::CredentialField,
            ),
            (
                "body",
                format!("-----BEGIN {}-----", "PRIVATE KEY").into_bytes(),
                SecretFindingClass::PrivateKey,
            ),
            (
                "body",
                b"Bearer abc".to_vec(),
                SecretFindingClass::BearerCredential,
            ),
            (
                "body",
                provider.into_bytes(),
                SecretFindingClass::ProviderToken,
            ),
            (
                "body",
                cloud.into_bytes(),
                SecretFindingClass::CloudAccessKey,
            ),
            (
                "body",
                [b"https".as_slice(), b"://u:p@example.invalid/x"].concat(),
                SecretFindingClass::EmbeddedUriCredential,
            ),
        ];
        for (name, value, expected) in cases {
            assert!(detect(name, &value).contains(&expected));
        }
    }

    #[test]
    fn malformed_and_oversized_candidates_fail_content_free() {
        let duplicate = [
            PersistenceField::new("title", b"a", PersistenceFieldHandling::Persist),
            PersistenceField::new("title", b"b", PersistenceFieldHandling::Persist),
        ];
        let binary = [PersistenceField::new(
            "title",
            &[0xff],
            PersistenceFieldHandling::Persist,
        )];
        let large_value = vec![b'x'; MAX_FIELD_VALUE_BYTES + 1];
        let large = [PersistenceField::new(
            "title",
            &large_value,
            PersistenceFieldHandling::DigestOnly,
        )];
        for fields in [&duplicate[..], &binary[..], &large[..]] {
            let error = policy()
                .evaluate(&candidate(
                    fields,
                    PersistenceSensitivity::Internal,
                    PersistenceRetentionIntent::Session,
                ))
                .expect_err("failure");
            assert!(!error.to_string().contains("title"));
        }
    }

    #[test]
    fn identities_are_deterministic_and_policy_bound() {
        assert_eq!(policy().policy_sha256(), policy().policy_sha256());
        assert_ne!(
            policy().policy_sha256(),
            PersistencePolicy::new(29, 365)
                .expect("policy")
                .policy_sha256()
        );
        let fields = [PersistenceField::new(
            "title",
            b"safe",
            PersistenceFieldHandling::Persist,
        )];
        let one = policy()
            .evaluate(&candidate(
                &fields,
                PersistenceSensitivity::Internal,
                PersistenceRetentionIntent::Session,
            ))
            .expect("one");
        let two = policy()
            .evaluate(&candidate(
                &fields,
                PersistenceSensitivity::Internal,
                PersistenceRetentionIntent::Session,
            ))
            .expect("two");
        assert_eq!(
            one.receipt().receipt_sha256(),
            two.receipt().receipt_sha256()
        );
    }

    #[test]
    fn receipt_family_retention_and_overflow_are_closed() {
        let fields = [PersistenceField::new(
            "status",
            b"ok",
            PersistenceFieldHandling::Persist,
        )];
        let base = PersistenceCandidate {
            family: PersistenceRecordFamily::Receipts,
            record_id: "receipt-1",
            sensitivity: PersistenceSensitivity::Internal,
            retention: PersistenceRetentionIntent::Retained,
            occurred_at_epoch_ms: 1_000,
            fields: &fields,
        };
        assert_eq!(
            policy()
                .evaluate(&base)
                .expect("decision")
                .prepared()
                .expect("prepared")
                .retention()
                .expires_at_epoch_ms(),
            1_000 + 365 * DAY_MS
        );
        let overflow = PersistenceCandidate {
            occurred_at_epoch_ms: u64::MAX,
            ..base
        };
        assert_eq!(
            policy().evaluate(&overflow).expect_err("overflow"),
            PersistenceError::ClockOverflow
        );
    }

    #[test]
    fn synthetic_canary_campaign_covers_every_family_and_field_boundary() {
        const CANARY: &[u8] = b"AM_SYNTHETIC_SECRET_S011_EVERY_INPUT_BOUNDARY";
        let families = [
            PersistenceRecordFamily::Sessions,
            PersistenceRecordFamily::Objectives,
            PersistenceRecordFamily::Plans,
            PersistenceRecordFamily::Tasks,
            PersistenceRecordFamily::Actions,
            PersistenceRecordFamily::Evidence,
            PersistenceRecordFamily::Decisions,
            PersistenceRecordFamily::Grants,
            PersistenceRecordFamily::Receipts,
            PersistenceRecordFamily::Checkpoints,
            PersistenceRecordFamily::Files,
        ];
        let fields = [
            PersistenceField::operational_metadata(
                "persisted_value",
                CANARY,
                PersistenceFieldHandling::Persist,
            ),
            PersistenceField::operational_metadata(
                "digested_value",
                CANARY,
                PersistenceFieldHandling::DigestOnly,
            ),
            PersistenceField::operational_metadata(
                "ephemeral_value",
                CANARY,
                PersistenceFieldHandling::Ephemeral,
            ),
            PersistenceField::ephemeral_content(
                "raw_attachment",
                CANARY,
                EphemeralContentClass::RawAttachment,
            ),
            PersistenceField::ephemeral_content(
                "full_tool_output",
                CANARY,
                EphemeralContentClass::FullToolOutput,
            ),
            PersistenceField::ephemeral_content(
                "environment_variable",
                CANARY,
                EphemeralContentClass::EnvironmentVariable,
            ),
            PersistenceField::ephemeral_content("prompt", CANARY, EphemeralContentClass::Prompt),
            PersistenceField::ephemeral_content(
                "model_response",
                CANARY,
                EphemeralContentClass::ModelResponse,
            ),
        ];
        let canary = std::str::from_utf8(CANARY).expect("ASCII canary");
        let canary_sha256 = sha256_hex(CANARY);

        for family in families {
            let candidate = PersistenceCandidate {
                family,
                record_id: canary,
                sensitivity: PersistenceSensitivity::Private,
                retention: PersistenceRetentionIntent::Retained,
                occurred_at_epoch_ms: 1_000,
                fields: &fields,
            };
            let decision = policy().evaluate(&candidate).expect("campaign decision");
            assert_eq!(decision.receipt().outcome(), PersistenceOutcome::Admitted);
            decision
                .prepared()
                .expect("one policy-authorized encrypted record")
                .with_record_json(|record| {
                    let record = std::str::from_utf8(record).expect("canonical JSON");
                    assert_eq!(record.matches(canary).count(), 2);
                    assert_eq!(record.matches(&canary_sha256).count(), 1);
                });
            let receipt = String::from_utf8(decision.receipt().to_json().expect("receipt JSON"))
                .expect("UTF-8 receipt");
            assert!(!receipt.contains(canary));
            assert!(!format!("{candidate:?}{decision:?}").contains(canary));

            let denied_field = [PersistenceField::operational_metadata(
                "api_key",
                CANARY,
                PersistenceFieldHandling::Persist,
            )];
            let denied_candidate = PersistenceCandidate {
                fields: &denied_field,
                ..candidate
            };
            let denied = policy()
                .evaluate(&denied_candidate)
                .expect("content-free denial");
            assert_eq!(denied.receipt().outcome(), PersistenceOutcome::DeniedSecret);
            assert!(denied.prepared().is_none());
            assert!(
                !String::from_utf8(denied.receipt().to_json().expect("denial JSON"))
                    .expect("UTF-8 denial")
                    .contains(canary)
            );
        }
    }

    #[test]
    fn debug_surfaces_exclude_values_and_record_identity() {
        let fields = [PersistenceField::new(
            "title",
            b"private-canary",
            PersistenceFieldHandling::Persist,
        )];
        let candidate = candidate(
            &fields,
            PersistenceSensitivity::Private,
            PersistenceRetentionIntent::Session,
        );
        let outputs = [
            format!("{candidate:?}"),
            format!("{:?}", policy().evaluate(&candidate).expect("decision")),
        ];
        for output in outputs {
            assert!(!output.contains("private-canary"));
            assert!(!output.contains("session-0001"));
        }
    }
}
