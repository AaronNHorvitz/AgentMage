//! Authenticated portable memory export and import without filesystem authority.

use std::fmt::{self, Write as _};

use agentmage_kernel_contracts::{DataSensitivity, EvidenceReference, WorkspaceId};
use chacha20poly1305::{
    Key, KeyInit, Tag, XChaCha20Poly1305, XNonce,
    aead::{AeadInOut, inout::InOutBuf},
};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{
    MemoryCatalog, MemoryError, MemoryId, MemoryItem, MemoryItemStatus, MemoryScope, MemoryType,
};

const FORMAT_MAGIC: &[u8; 8] = b"AMMEM001";
const FORMAT_VERSION: u16 = 1;
const SCHEMA_VERSION: u16 = 1;
const SALT_BYTES: usize = 32;
const NONCE_BYTES: usize = 24;
const DIGEST_BYTES: usize = 32;
const TAG_BYTES: usize = 16;
const HEADER_BYTES: usize = 8 + 2 + SALT_BYTES + NONCE_BYTES + 8 + DIGEST_BYTES;
const MAX_PLAINTEXT_BYTES: usize = 64 * 1024 * 1024;
const KEY_INFO: &[u8] = b"agentmage.memory.portable-export.xchacha20poly1305.v1";

/// Caller-owned memory export key, redacted from debug output and zeroized on drop.
pub struct MemoryPortableKey(Zeroizing<[u8; 32]>);

impl MemoryPortableKey {
    /// Admits a nonzero 256-bit key supplied by a trusted credential adapter.
    pub fn new(bytes: [u8; 32]) -> Result<Self, MemoryPortableError> {
        if bytes == [0; 32] {
            return Err(MemoryPortableError::InvalidInput);
        }
        Ok(Self(Zeroizing::new(bytes)))
    }

    fn bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl fmt::Debug for MemoryPortableKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MemoryPortableKey")
            .finish_non_exhaustive()
    }
}

/// Fresh public salt and nonce supplied by a trusted random-number adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryExportEntropy {
    salt: [u8; SALT_BYTES],
    nonce: [u8; NONCE_BYTES],
}

impl MemoryExportEntropy {
    /// Admits one nonzero, non-repeated salt and nonce pair.
    pub fn new(
        salt: [u8; SALT_BYTES],
        nonce: [u8; NONCE_BYTES],
    ) -> Result<Self, MemoryPortableError> {
        if salt == [0; SALT_BYTES] || nonce == [0; NONCE_BYTES] || salt[..NONCE_BYTES] == nonce {
            return Err(MemoryPortableError::InvalidInput);
        }
        Ok(Self { salt, nonce })
    }
}

/// One complete encrypted export proposal with no path or write method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncryptedMemoryExport {
    /// Versioned authenticated ciphertext bytes.
    pub bytes: Vec<u8>,
    /// Digest of the complete encrypted bytes.
    pub export_sha256: String,
    /// Digest of the exact plaintext envelope before encryption.
    pub plaintext_sha256: String,
    /// Catalog digest bound inside the authenticated envelope.
    pub catalog_sha256: String,
    /// Number of current and historical memory items in the envelope.
    pub item_count: u64,
    /// Fixed false file-write marker.
    pub files_written: bool,
}

/// Content-free result of importing and validating one encrypted export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryPortableReceipt {
    /// Authenticated format version.
    pub format_version: u16,
    /// Digest of the complete encrypted input.
    pub export_sha256: String,
    /// Digest of the authenticated plaintext envelope.
    pub plaintext_sha256: String,
    /// Imported catalog revision.
    pub catalog_revision: u64,
    /// Imported catalog digest.
    pub catalog_sha256: String,
    /// Imported current and historical item count.
    pub item_count: u64,
    /// Fixed false machine-path marker.
    pub machine_specific_paths_present: bool,
    /// Fixed false credential marker.
    pub credentials_present: bool,
    /// Fixed false file-write marker.
    pub files_written: bool,
}

/// Content-free portable memory boundary failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryPortableError {
    /// Key, entropy, header, version, size, or closed schema was invalid.
    InvalidInput,
    /// Catalog content contained a credential candidate or machine-specific authority.
    NonPortableData,
    /// Authentication, payload digest, or catalog digest verification failed.
    Integrity,
    /// Imported memory identities, links, lifecycle state, or policy fields were invalid.
    InvalidCatalog,
    /// A fixed memory or arithmetic bound was exceeded.
    ResourceLimit,
}

/// Encrypts a complete catalog into one versioned, portable, no-write proposal.
pub fn export_memory_catalog(
    catalog: &MemoryCatalog,
    key: &MemoryPortableKey,
    entropy: MemoryExportEntropy,
) -> Result<EncryptedMemoryExport, MemoryPortableError> {
    let summary = catalog.inspect();
    let items = catalog
        .items()
        .iter()
        .map(item_to_wire)
        .collect::<Result<Vec<_>, _>>()?;
    let envelope = MemoryExportEnvelope {
        schema_version: SCHEMA_VERSION,
        catalog_revision: summary.revision,
        catalog_sha256: summary.catalog_sha256.clone(),
        items,
    };
    let mut plaintext = Zeroizing::new(
        serde_json::to_vec(&envelope).map_err(|_| MemoryPortableError::ResourceLimit)?,
    );
    if plaintext.is_empty() || plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(MemoryPortableError::ResourceLimit);
    }
    let plaintext_sha256 = digest_bytes(plaintext.as_slice());
    let header = build_header(entropy, plaintext.len(), &plaintext_sha256)?;
    let cipher = cipher(key, &entropy.salt)?;
    let tag = cipher
        .encrypt_inout_detached(
            &XNonce::from(entropy.nonce),
            &header,
            InOutBuf::from(plaintext.as_mut_slice()),
        )
        .map_err(|_| MemoryPortableError::Integrity)?;
    let total = HEADER_BYTES
        .checked_add(plaintext.len())
        .and_then(|value| value.checked_add(TAG_BYTES))
        .ok_or(MemoryPortableError::ResourceLimit)?;
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(plaintext.as_slice());
    bytes.extend_from_slice(tag.as_slice());
    Ok(EncryptedMemoryExport {
        export_sha256: digest_bytes(&bytes),
        plaintext_sha256,
        catalog_sha256: summary.catalog_sha256,
        item_count: summary.item_count,
        bytes,
        files_written: false,
    })
}

/// Authenticates and imports one encrypted export into a new in-memory catalog.
pub fn import_memory_catalog(
    encrypted: &[u8],
    key: &MemoryPortableKey,
) -> Result<(MemoryCatalog, MemoryPortableReceipt), MemoryPortableError> {
    if encrypted.len() < HEADER_BYTES + TAG_BYTES
        || encrypted.len() > HEADER_BYTES + MAX_PLAINTEXT_BYTES + TAG_BYTES
    {
        return Err(MemoryPortableError::InvalidInput);
    }
    let header: [u8; HEADER_BYTES] = encrypted[..HEADER_BYTES]
        .try_into()
        .map_err(|_| MemoryPortableError::InvalidInput)?;
    let parsed = parse_header(&header)?;
    let expected_len = HEADER_BYTES
        .checked_add(parsed.plaintext_len)
        .and_then(|value| value.checked_add(TAG_BYTES))
        .ok_or(MemoryPortableError::ResourceLimit)?;
    if encrypted.len() != expected_len {
        return Err(MemoryPortableError::InvalidInput);
    }
    let mut plaintext =
        Zeroizing::new(encrypted[HEADER_BYTES..HEADER_BYTES + parsed.plaintext_len].to_vec());
    let tag_bytes: [u8; TAG_BYTES] = encrypted[HEADER_BYTES + parsed.plaintext_len..]
        .try_into()
        .map_err(|_| MemoryPortableError::InvalidInput)?;
    cipher(key, &parsed.salt)?
        .decrypt_inout_detached(
            &XNonce::from(parsed.nonce),
            &header,
            InOutBuf::from(plaintext.as_mut_slice()),
            &Tag::from(tag_bytes),
        )
        .map_err(|_| MemoryPortableError::Integrity)?;
    let plaintext_sha256 = digest_bytes(plaintext.as_slice());
    if plaintext_sha256 != parsed.plaintext_sha256 {
        return Err(MemoryPortableError::Integrity);
    }
    let envelope: MemoryExportEnvelope = serde_json::from_slice(plaintext.as_slice())
        .map_err(|_| MemoryPortableError::InvalidInput)?;
    if envelope.schema_version != SCHEMA_VERSION || !valid_sha256(&envelope.catalog_sha256) {
        return Err(MemoryPortableError::InvalidInput);
    }
    let items = envelope
        .items
        .into_iter()
        .map(item_from_wire)
        .collect::<Result<Vec<_>, _>>()?;
    let catalog = MemoryCatalog::from_portable_parts(
        envelope.catalog_revision,
        items,
        &envelope.catalog_sha256,
    )
    .map_err(map_catalog_error)?;
    let summary = catalog.inspect();
    let receipt = MemoryPortableReceipt {
        format_version: FORMAT_VERSION,
        export_sha256: digest_bytes(encrypted),
        plaintext_sha256,
        catalog_revision: summary.revision,
        catalog_sha256: summary.catalog_sha256,
        item_count: summary.item_count,
        machine_specific_paths_present: false,
        credentials_present: false,
        files_written: false,
    };
    Ok((catalog, receipt))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryExportEnvelope {
    schema_version: u16,
    catalog_revision: u64,
    catalog_sha256: String,
    items: Vec<MemoryItemWire>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryItemWire {
    memory_id: String,
    memory_type: String,
    workspace_id: String,
    project_id: Option<String>,
    conversation_id: Option<String>,
    content: Option<String>,
    fact_key: Option<String>,
    tags: Vec<String>,
    links: Vec<String>,
    evidence: Vec<EvidenceReference>,
    sensitivity: String,
    confidence_bps: u32,
    status: String,
    created_at: String,
    decided_at: String,
    last_verified_at: String,
    expires_at: Option<String>,
    candidate_sha256: String,
    decision_sha256: String,
    superseded_by: Option<String>,
}

struct ParsedHeader {
    salt: [u8; SALT_BYTES],
    nonce: [u8; NONCE_BYTES],
    plaintext_len: usize,
    plaintext_sha256: String,
}

fn item_to_wire(item: &MemoryItem) -> Result<MemoryItemWire, MemoryPortableError> {
    validate_portable_item(item)?;
    Ok(MemoryItemWire {
        memory_id: item.memory_id.as_str().to_owned(),
        memory_type: memory_type_id(item.memory_type).to_owned(),
        workspace_id: item.scope.workspace_id.as_str().to_owned(),
        project_id: item.scope.project_id.clone(),
        conversation_id: item.scope.conversation_id.clone(),
        content: item.content.clone(),
        fact_key: item.fact_key.clone(),
        tags: item.tags.clone(),
        links: item
            .links
            .iter()
            .map(|value| value.as_str().to_owned())
            .collect(),
        evidence: item.evidence.clone(),
        sensitivity: sensitivity_id(item.sensitivity).to_owned(),
        confidence_bps: item.confidence_bps,
        status: status_id(item.status).to_owned(),
        created_at: item.created_at.clone(),
        decided_at: item.decided_at.clone(),
        last_verified_at: item.last_verified_at.clone(),
        expires_at: item.expires_at.clone(),
        candidate_sha256: item.candidate_sha256.clone(),
        decision_sha256: item.decision_sha256.clone(),
        superseded_by: item
            .superseded_by
            .as_ref()
            .map(|value| value.as_str().to_owned()),
    })
}

fn item_from_wire(wire: MemoryItemWire) -> Result<MemoryItem, MemoryPortableError> {
    let item = MemoryItem {
        memory_id: MemoryId::parse(wire.memory_id)
            .map_err(|_| MemoryPortableError::InvalidCatalog)?,
        memory_type: parse_memory_type(&wire.memory_type)?,
        scope: MemoryScope {
            workspace_id: WorkspaceId::from_raw(wire.workspace_id),
            project_id: wire.project_id,
            conversation_id: wire.conversation_id,
        },
        content: wire.content,
        fact_key: wire.fact_key,
        tags: wire.tags,
        links: wire
            .links
            .into_iter()
            .map(MemoryId::parse)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| MemoryPortableError::InvalidCatalog)?,
        evidence: wire.evidence,
        sensitivity: parse_sensitivity(&wire.sensitivity)?,
        confidence_bps: wire.confidence_bps,
        status: parse_status(&wire.status)?,
        created_at: wire.created_at,
        decided_at: wire.decided_at,
        last_verified_at: wire.last_verified_at,
        expires_at: wire.expires_at,
        candidate_sha256: wire.candidate_sha256,
        decision_sha256: wire.decision_sha256,
        superseded_by: wire
            .superseded_by
            .map(MemoryId::parse)
            .transpose()
            .map_err(|_| MemoryPortableError::InvalidCatalog)?,
    };
    validate_portable_item(&item)?;
    Ok(item)
}

fn validate_portable_item(item: &MemoryItem) -> Result<(), MemoryPortableError> {
    let identifiers = std::iter::once(item.memory_id.as_str())
        .chain(std::iter::once(item.scope.workspace_id.as_str()))
        .chain(item.scope.project_id.iter().map(String::as_str))
        .chain(item.scope.conversation_id.iter().map(String::as_str))
        .chain(item.fact_key.iter().map(String::as_str))
        .chain(item.tags.iter().map(String::as_str))
        .chain(item.links.iter().map(MemoryId::as_str))
        .chain(item.superseded_by.iter().map(MemoryId::as_str));
    if identifiers
        .into_iter()
        .any(|value| !portable_identifier(value) || crate::domain::secret_candidate(value))
        || item.sensitivity == DataSensitivity::Restricted
        || item.content.as_ref().is_some_and(|value| {
            crate::domain::secret_candidate(value) || contains_machine_path(value)
        })
        || !valid_timestamp(&item.created_at)
        || !valid_timestamp(&item.decided_at)
        || !valid_timestamp(&item.last_verified_at)
        || item
            .expires_at
            .as_ref()
            .is_some_and(|value| !valid_timestamp(value))
        || !valid_sha256(&item.candidate_sha256)
        || !valid_sha256(&item.decision_sha256)
        || item.evidence.iter().any(|evidence| {
            evidence.schema_version != 1
                || !portable_identifier(evidence.evidence_id.as_str())
                || !portable_identifier(&evidence.source_id)
                || !portable_identifier(&evidence.object_id)
                || evidence
                    .fragment
                    .as_ref()
                    .is_some_and(|value| !portable_identifier(value))
                || evidence
                    .observed_revision
                    .as_ref()
                    .is_some_and(|value| !portable_identifier(value))
                || !valid_sha256(&evidence.content_sha256)
                || crate::domain::secret_candidate(evidence.evidence_id.as_str())
                || crate::domain::secret_candidate(&evidence.source_id)
                || crate::domain::secret_candidate(&evidence.object_id)
                || evidence
                    .fragment
                    .as_ref()
                    .is_some_and(|value| crate::domain::secret_candidate(value))
                || evidence
                    .observed_revision
                    .as_ref()
                    .is_some_and(|value| crate::domain::secret_candidate(value))
        })
    {
        return Err(MemoryPortableError::NonPortableData);
    }
    Ok(())
}

fn build_header(
    entropy: MemoryExportEntropy,
    plaintext_len: usize,
    plaintext_sha256: &str,
) -> Result<[u8; HEADER_BYTES], MemoryPortableError> {
    let len = u64::try_from(plaintext_len).map_err(|_| MemoryPortableError::ResourceLimit)?;
    let digest = decode_sha256(plaintext_sha256)?;
    let mut header = [0_u8; HEADER_BYTES];
    header[..8].copy_from_slice(FORMAT_MAGIC);
    header[8..10].copy_from_slice(&FORMAT_VERSION.to_be_bytes());
    header[10..42].copy_from_slice(&entropy.salt);
    header[42..66].copy_from_slice(&entropy.nonce);
    header[66..74].copy_from_slice(&len.to_be_bytes());
    header[74..].copy_from_slice(&digest);
    Ok(header)
}

fn parse_header(header: &[u8; HEADER_BYTES]) -> Result<ParsedHeader, MemoryPortableError> {
    if &header[..8] != FORMAT_MAGIC
        || u16::from_be_bytes(
            header[8..10]
                .try_into()
                .map_err(|_| MemoryPortableError::InvalidInput)?,
        ) != FORMAT_VERSION
    {
        return Err(MemoryPortableError::InvalidInput);
    }
    let salt: [u8; SALT_BYTES] = header[10..42]
        .try_into()
        .map_err(|_| MemoryPortableError::InvalidInput)?;
    let nonce: [u8; NONCE_BYTES] = header[42..66]
        .try_into()
        .map_err(|_| MemoryPortableError::InvalidInput)?;
    MemoryExportEntropy::new(salt, nonce)?;
    let plaintext_len = usize::try_from(u64::from_be_bytes(
        header[66..74]
            .try_into()
            .map_err(|_| MemoryPortableError::InvalidInput)?,
    ))
    .map_err(|_| MemoryPortableError::ResourceLimit)?;
    if plaintext_len == 0 || plaintext_len > MAX_PLAINTEXT_BYTES {
        return Err(MemoryPortableError::ResourceLimit);
    }
    Ok(ParsedHeader {
        salt,
        nonce,
        plaintext_len,
        plaintext_sha256: encode_sha256(&header[74..]),
    })
}

fn cipher(
    key: &MemoryPortableKey,
    salt: &[u8; SALT_BYTES],
) -> Result<XChaCha20Poly1305, MemoryPortableError> {
    let derivation = Hkdf::<Sha256>::new(Some(salt), key.bytes());
    let mut derived = Zeroizing::new([0_u8; 32]);
    derivation
        .expand(KEY_INFO, derived.as_mut_slice())
        .map_err(|_| MemoryPortableError::InvalidInput)?;
    Ok(XChaCha20Poly1305::new(&Key::from(*derived)))
}

fn portable_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

fn contains_machine_path(value: &str) -> bool {
    let normalized = value.replace('\\', "/").to_ascii_lowercase();
    normalized.contains("/home/")
        || normalized.contains("/var/home/")
        || normalized.contains("/users/")
        || normalized.contains("file://")
        || normalized
            .as_bytes()
            .windows(3)
            .any(|window| window[0].is_ascii_alphabetic() && window[1] == b':' && window[2] == b'/')
}

fn valid_timestamp(value: &str) -> bool {
    value.len() >= 20
        && value.len() <= 64
        && value.ends_with('Z')
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value.as_bytes().get(10) == Some(&b'T')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn decode_sha256(value: &str) -> Result<[u8; DIGEST_BYTES], MemoryPortableError> {
    if !valid_sha256(value) {
        return Err(MemoryPortableError::InvalidInput);
    }
    let mut bytes = [0_u8; DIGEST_BYTES];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(bytes)
}

fn hex_nibble(value: u8) -> Result<u8, MemoryPortableError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(MemoryPortableError::InvalidInput),
    }
}

fn encode_sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(value.len() * 2);
    for byte in value {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn digest_bytes(value: &[u8]) -> String {
    encode_sha256(&Sha256::digest(value))
}

fn map_catalog_error(value: MemoryError) -> MemoryPortableError {
    match value {
        MemoryError::ResourceLimit => MemoryPortableError::ResourceLimit,
        MemoryError::DecisionDrift => MemoryPortableError::Integrity,
        _ => MemoryPortableError::InvalidCatalog,
    }
}

const fn memory_type_id(value: MemoryType) -> &'static str {
    match value {
        MemoryType::Working => "working",
        MemoryType::Episodic => "episodic",
        MemoryType::Semantic => "semantic",
        MemoryType::Procedural => "procedural",
        MemoryType::Preference => "preference",
    }
}

fn parse_memory_type(value: &str) -> Result<MemoryType, MemoryPortableError> {
    match value {
        "working" => Ok(MemoryType::Working),
        "episodic" => Ok(MemoryType::Episodic),
        "semantic" => Ok(MemoryType::Semantic),
        "procedural" => Ok(MemoryType::Procedural),
        "preference" => Ok(MemoryType::Preference),
        _ => Err(MemoryPortableError::InvalidCatalog),
    }
}

const fn status_id(value: MemoryItemStatus) -> &'static str {
    match value {
        MemoryItemStatus::Candidate => "candidate",
        MemoryItemStatus::Approved => "approved",
        MemoryItemStatus::Rejected => "rejected",
        MemoryItemStatus::Superseded => "superseded",
        MemoryItemStatus::Expired => "expired",
        MemoryItemStatus::Hold => "hold",
        MemoryItemStatus::Deleted => "deleted",
    }
}

fn parse_status(value: &str) -> Result<MemoryItemStatus, MemoryPortableError> {
    match value {
        "candidate" => Ok(MemoryItemStatus::Candidate),
        "approved" => Ok(MemoryItemStatus::Approved),
        "rejected" => Ok(MemoryItemStatus::Rejected),
        "superseded" => Ok(MemoryItemStatus::Superseded),
        "expired" => Ok(MemoryItemStatus::Expired),
        "hold" => Ok(MemoryItemStatus::Hold),
        "deleted" => Ok(MemoryItemStatus::Deleted),
        _ => Err(MemoryPortableError::InvalidCatalog),
    }
}

const fn sensitivity_id(value: DataSensitivity) -> &'static str {
    match value {
        DataSensitivity::Ephemeral => "ephemeral",
        DataSensitivity::Operational => "operational",
        DataSensitivity::Durable => "durable",
        DataSensitivity::Restricted => "restricted",
    }
}

fn parse_sensitivity(value: &str) -> Result<DataSensitivity, MemoryPortableError> {
    match value {
        "ephemeral" => Ok(DataSensitivity::Ephemeral),
        "operational" => Ok(DataSensitivity::Operational),
        "durable" => Ok(DataSensitivity::Durable),
        "restricted" => Ok(DataSensitivity::Restricted),
        _ => Err(MemoryPortableError::InvalidCatalog),
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{EvidenceId, EvidenceKind};

    use super::*;

    fn key(byte: u8) -> MemoryPortableKey {
        MemoryPortableKey::new([byte; 32]).expect("key")
    }

    fn entropy(salt: u8, nonce: u8) -> MemoryExportEntropy {
        MemoryExportEntropy::new([salt; SALT_BYTES], [nonce; NONCE_BYTES]).expect("entropy")
    }

    fn item(identity: &str) -> MemoryItem {
        MemoryItem {
            memory_id: MemoryId::parse(identity).expect("identity"),
            memory_type: MemoryType::Semantic,
            scope: MemoryScope {
                workspace_id: WorkspaceId::from_raw("workspace-portable"),
                project_id: Some("project-one".to_owned()),
                conversation_id: Some("conversation-one".to_owned()),
            },
            content: Some("The release owner is Alice.".to_owned()),
            fact_key: Some("release.owner".to_owned()),
            tags: vec!["release".to_owned()],
            links: Vec::new(),
            evidence: vec![EvidenceReference {
                schema_version: 1,
                evidence_id: EvidenceId::from_raw("evidence-portable-one"),
                kind: EvidenceKind::Document,
                source_id: "source-one".to_owned(),
                object_id: "object-one".to_owned(),
                fragment: Some("line-1".to_owned()),
                content_sha256: "a".repeat(64),
                observed_revision: Some("revision-one".to_owned()),
            }],
            sensitivity: DataSensitivity::Durable,
            confidence_bps: 9_000,
            status: MemoryItemStatus::Approved,
            created_at: "2026-08-18T10:00:00Z".to_owned(),
            decided_at: "2026-08-18T10:01:00Z".to_owned(),
            last_verified_at: "2026-08-18T10:01:00Z".to_owned(),
            expires_at: None,
            candidate_sha256: "b".repeat(64),
            decision_sha256: "c".repeat(64),
            superseded_by: None,
        }
    }

    fn catalog() -> MemoryCatalog {
        let mut catalog = MemoryCatalog::new();
        catalog.insert(item("memory-portable-one")).expect("insert");
        catalog
    }

    #[test]
    fn encrypted_export_import_preserves_complete_catalog_without_writes() {
        let source = catalog();
        let exported = export_memory_catalog(&source, &key(1), entropy(2, 3)).expect("export");
        assert!(!exported.files_written);
        assert!(
            !exported
                .bytes
                .windows("The release owner is Alice.".len())
                .any(|window| window == b"The release owner is Alice.")
        );
        let (imported, receipt) = import_memory_catalog(&exported.bytes, &key(1)).expect("import");
        assert_eq!(imported.inspect(), source.inspect());
        assert_eq!(imported.items(), source.items());
        assert_eq!(receipt.export_sha256, exported.export_sha256);
        assert_eq!(receipt.plaintext_sha256, exported.plaintext_sha256);
        assert!(!receipt.machine_specific_paths_present);
        assert!(!receipt.credentials_present);
        assert!(!receipt.files_written);
    }

    #[test]
    fn fresh_entropy_changes_ciphertext_while_preserving_imported_identity() {
        let source = catalog();
        let first = export_memory_catalog(&source, &key(1), entropy(2, 3)).expect("first");
        let second = export_memory_catalog(&source, &key(1), entropy(4, 5)).expect("second");
        assert_ne!(first.bytes, second.bytes);
        let (first_catalog, _) =
            import_memory_catalog(&first.bytes, &key(1)).expect("first import");
        let (second_catalog, _) =
            import_memory_catalog(&second.bytes, &key(1)).expect("second import");
        assert_eq!(first_catalog.items(), second_catalog.items());
    }

    #[test]
    fn catalog_digest_binds_portable_metadata_and_evidence() {
        let first = catalog();
        let mut second = MemoryCatalog::new();
        let mut changed = item("memory-portable-one");
        changed.evidence[0].object_id = "object-two".to_owned();
        second.insert(changed).expect("changed insert");
        assert_ne!(
            first.inspect().catalog_sha256,
            second.inspect().catalog_sha256
        );
    }

    #[test]
    fn wrong_key_tampering_truncation_and_version_drift_fail_closed() {
        let exported = export_memory_catalog(&catalog(), &key(1), entropy(2, 3)).expect("export");
        assert!(matches!(
            import_memory_catalog(&exported.bytes, &key(9)),
            Err(MemoryPortableError::Integrity)
        ));
        let mut tampered = exported.bytes.clone();
        tampered[HEADER_BYTES] ^= 1;
        assert!(matches!(
            import_memory_catalog(&tampered, &key(1)),
            Err(MemoryPortableError::Integrity)
        ));
        assert!(matches!(
            import_memory_catalog(&exported.bytes[..exported.bytes.len() - 1], &key(1)),
            Err(MemoryPortableError::InvalidInput)
        ));
        let mut wrong_version = exported.bytes;
        wrong_version[9] = 2;
        assert!(matches!(
            import_memory_catalog(&wrong_version, &key(1)),
            Err(MemoryPortableError::InvalidInput)
        ));
    }

    #[test]
    fn machine_paths_restricted_data_zero_keys_and_bad_entropy_are_rejected() {
        let mut path_catalog = MemoryCatalog::new();
        let mut path_item = item("memory-path");
        path_item.evidence[0].object_id = "/home/user/private.md".to_owned();
        path_catalog
            .insert(path_item)
            .expect("catalog accepts opaque source identity");
        assert!(matches!(
            export_memory_catalog(&path_catalog, &key(1), entropy(2, 3)),
            Err(MemoryPortableError::NonPortableData)
        ));

        let mut restricted_catalog = MemoryCatalog::new();
        let mut restricted = item("memory-restricted");
        restricted.sensitivity = DataSensitivity::Restricted;
        restricted_catalog
            .insert(restricted)
            .expect("catalog insertion");
        assert!(matches!(
            export_memory_catalog(&restricted_catalog, &key(1), entropy(2, 3)),
            Err(MemoryPortableError::NonPortableData)
        ));
        assert!(matches!(
            MemoryPortableKey::new([0; 32]),
            Err(MemoryPortableError::InvalidInput)
        ));
        assert!(matches!(
            MemoryExportEntropy::new([0; SALT_BYTES], [1; NONCE_BYTES]),
            Err(MemoryPortableError::InvalidInput)
        ));
    }

    #[test]
    fn export_key_debug_never_exposes_key_bytes() {
        let rendered = format!("{:?}", key(7));
        assert_eq!(rendered, "MemoryPortableKey { .. }");
        assert!(!rendered.contains('7'));
    }
}
