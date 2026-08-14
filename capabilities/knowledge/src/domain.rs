//! Closed canonical record values and validation.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{DataSensitivity, EvidenceReference};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{KNOWLEDGE_SCHEMA_VERSION, knowledge_schema};

const MAX_ID_BYTES: usize = 128;
const MAX_TITLE_BYTES: usize = 512;
const MAX_FIELD_NAME_BYTES: usize = 64;
const MAX_FIELD_VALUE_BYTES: usize = 32 * 1024;
const MAX_FIELDS: usize = 128;
const MAX_LINKS: usize = 512;
const MAX_EVIDENCE: usize = 256;
const MAX_TAGS: usize = 128;
const MAX_TAG_BYTES: usize = 128;

/// Stable local identity stored inside canonical Markdown and independent of its path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct KnowledgeRecordId(String);

impl KnowledgeRecordId {
    /// Constructs and validates one stable knowledge identity.
    pub fn parse(value: impl Into<String>) -> Result<Self, KnowledgeError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_ID_BYTES
            || !value.starts_with("knowledge-")
            || value
                .bytes()
                .any(|byte| !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'))
        {
            return Err(KnowledgeError::InvalidIdentity);
        }
        Ok(Self(value))
    }

    /// Returns the stable wire value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for KnowledgeRecordId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Closed canonical human knowledge record kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRecordKind {
    /// A person known to the user.
    Person,
    /// An organization.
    Organization,
    /// A project or bounded body of work.
    Project,
    /// A meeting record.
    Meeting,
    /// A human-owned task record.
    Task,
    /// A confirmed, proposed, or superseded decision.
    Decision,
    /// A commitment between identified parties.
    Commitment,
    /// A document register entry.
    Document,
    /// A correspondence record.
    Correspondence,
    /// A deadline or suspense date.
    Deadline,
    /// An approval request or decision.
    Approval,
    /// A tracked risk.
    Risk,
    /// An open, answered, or superseded question.
    Question,
    /// A handoff record.
    Handoff,
}

/// User-visible privacy level for a canonical knowledge record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgePrivacy {
    /// Ordinary local knowledge.
    Ordinary,
    /// Private user knowledge.
    Private,
    /// Confidential knowledge requiring narrower retrieval.
    Confidential,
    /// Highly restricted content that cannot enter canonical Markdown.
    HighlyRestricted,
}

/// Closed retention policy class for canonical human knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRetentionKind {
    /// Retain until the user supersedes or deletes the record.
    UntilSupersededOrDeleted,
    /// Retain until the exact declared UTC expiration.
    UntilExpiration,
    /// Retain under an explicit user hold.
    UserHold,
}

/// Visible retention rule carried by the canonical Markdown record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRetention {
    /// Retention class.
    pub kind: KnowledgeRetentionKind,
    /// Exact UTC expiration for expiration-based retention.
    #[serde(deserialize_with = "deserialize_required_option")]
    pub expires_at: Option<String>,
}

/// Closed relationship vocabulary between canonical records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeLinkKind {
    /// General relation with no stronger supported meaning.
    Related,
    /// Record is owned by the target person or organization.
    OwnedBy,
    /// Record concerns the target record.
    About,
    /// Record supports the target record.
    Supports,
    /// Record depends on the target record.
    DependsOn,
    /// Record supersedes the target record without deleting it.
    Supersedes,
    /// Record follows from the target record.
    FollowUpTo,
}

/// One typed relationship to another stable knowledge identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeLink {
    /// Relationship kind.
    pub kind: KnowledgeLinkKind,
    /// Stable target identity, never a path.
    pub target_id: KnowledgeRecordId,
}

/// One schema-governed field in stable lexical order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeField {
    /// Closed field name from the record-kind schema.
    pub name: String,
    /// Bounded field value retained in canonical Markdown.
    pub value: String,
}

/// Complete canonical human knowledge record represented by user-owned Markdown.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRecord {
    /// Knowledge schema version.
    pub schema_version: u16,
    /// Stable identity independent of filename and folder.
    pub record_id: KnowledgeRecordId,
    /// Closed record kind.
    pub kind: KnowledgeRecordKind,
    /// User-visible title.
    pub title: String,
    /// Privacy classification applied before persistence or indexing.
    pub privacy: KnowledgePrivacy,
    /// Kernel data sensitivity used by shared policy.
    pub sensitivity: DataSensitivity,
    /// Visible retention rule.
    pub retention: KnowledgeRetention,
    /// Exact creation timestamp supplied by the trusted caller.
    pub created_at: String,
    /// Exact last-update timestamp supplied by the trusted caller.
    pub updated_at: String,
    /// Exact last verification timestamp, or no completed verification.
    #[serde(deserialize_with = "deserialize_required_option")]
    pub last_verified_at: Option<String>,
    /// Kind-specific closed fields.
    pub fields: Vec<KnowledgeField>,
    /// Typed stable-identity relationships.
    pub links: Vec<KnowledgeLink>,
    /// Stable searchable tags without leading hash markers.
    pub tags: Vec<String>,
    /// Content-addressed supporting evidence references.
    pub evidence: Vec<EvidenceReference>,
}

/// Content-free canonical knowledge validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeError {
    /// Stable identity is malformed.
    InvalidIdentity,
    /// Record schema version or kind is unsupported.
    UnsupportedSchema,
    /// A required field is absent.
    MissingRequiredField,
    /// A field is unknown, malformed, duplicated, or out of bounds.
    InvalidField,
    /// Common metadata is malformed or out of bounds.
    InvalidMetadata,
    /// Privacy and sensitivity would permit unsafe Markdown persistence.
    PersistenceDenied,
    /// Link closure is malformed or contains duplicates.
    InvalidLink,
    /// Evidence closure is malformed, duplicated, or out of bounds.
    InvalidEvidence,
    /// Plain-folder layout or naming template is malformed.
    InvalidTemplate,
    /// A supplied knowledge path or entry kind is prohibited.
    InvalidPath,
    /// Canonical Markdown is malformed, non-canonical, or unsupported.
    InvalidMarkdown,
    /// Supplied bytes do not match their expected digest.
    ContentDrift,
    /// Stable identity or canonical path is duplicated.
    DuplicateRecord,
}

impl KnowledgeError {
    /// Returns the stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentity => "knowledge.identity.invalid",
            Self::UnsupportedSchema => "knowledge.schema.unsupported",
            Self::MissingRequiredField => "knowledge.field.required_missing",
            Self::InvalidField => "knowledge.field.invalid",
            Self::InvalidMetadata => "knowledge.metadata.invalid",
            Self::PersistenceDenied => "knowledge.persistence.denied",
            Self::InvalidLink => "knowledge.link.invalid",
            Self::InvalidEvidence => "knowledge.evidence.invalid",
            Self::InvalidTemplate => "knowledge.template.invalid",
            Self::InvalidPath => "knowledge.path.invalid",
            Self::InvalidMarkdown => "knowledge.markdown.invalid",
            Self::ContentDrift => "knowledge.content.drift",
            Self::DuplicateRecord => "knowledge.record.duplicate",
        }
    }
}

impl std::fmt::Display for KnowledgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for KnowledgeError {}

/// Validates one complete canonical record against its closed kind schema.
pub fn validate_record(record: &KnowledgeRecord) -> Result<(), KnowledgeError> {
    if record.schema_version != KNOWLEDGE_SCHEMA_VERSION {
        return Err(KnowledgeError::UnsupportedSchema);
    }
    KnowledgeRecordId::parse(record.record_id.as_str())?;
    if record.title.is_empty()
        || record.title.len() > MAX_TITLE_BYTES
        || !valid_timestamp(&record.created_at)
        || !valid_timestamp(&record.updated_at)
        || record
            .last_verified_at
            .as_deref()
            .is_some_and(|value| !valid_timestamp(value))
        || record.fields.len() > MAX_FIELDS
        || record.links.len() > MAX_LINKS
        || record.evidence.len() > MAX_EVIDENCE
        || record.tags.len() > MAX_TAGS
    {
        return Err(KnowledgeError::InvalidMetadata);
    }
    if record.privacy == KnowledgePrivacy::HighlyRestricted
        || record.sensitivity != DataSensitivity::Durable
    {
        return Err(KnowledgeError::PersistenceDenied);
    }
    match record.retention.kind {
        KnowledgeRetentionKind::UntilExpiration => {
            if !record
                .retention
                .expires_at
                .as_deref()
                .is_some_and(valid_timestamp)
            {
                return Err(KnowledgeError::InvalidMetadata);
            }
        }
        KnowledgeRetentionKind::UntilSupersededOrDeleted | KnowledgeRetentionKind::UserHold => {
            if record.retention.expires_at.is_some() {
                return Err(KnowledgeError::InvalidMetadata);
            }
        }
    }
    let schema = knowledge_schema(record.kind);
    let mut names = BTreeSet::new();
    for field in &record.fields {
        if field.name.is_empty()
            || field.name.len() > MAX_FIELD_NAME_BYTES
            || field.value.is_empty()
            || field.value.len() > MAX_FIELD_VALUE_BYTES
            || !schema.permits(&field.name)
            || !names.insert(field.name.as_str())
        {
            return Err(KnowledgeError::InvalidField);
        }
    }
    if schema
        .required_fields
        .iter()
        .any(|required| !names.contains(required))
    {
        return Err(KnowledgeError::MissingRequiredField);
    }
    let mut links = BTreeSet::new();
    for link in &record.links {
        if link.target_id == record.record_id || !links.insert(link) {
            return Err(KnowledgeError::InvalidLink);
        }
    }
    let mut tags = BTreeSet::new();
    if record.tags.iter().any(|tag| {
        tag.is_empty()
            || tag.len() > MAX_TAG_BYTES
            || tag.starts_with('#')
            || tag.chars().any(char::is_whitespace)
            || !tags.insert(tag.as_str())
    }) {
        return Err(KnowledgeError::InvalidMetadata);
    }
    let mut evidence = BTreeSet::new();
    for item in &record.evidence {
        if item.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
            || item.source_id.is_empty()
            || item.object_id.is_empty()
            || !valid_sha256(&item.content_sha256)
            || !evidence.insert(item.evidence_id.as_str())
        {
            return Err(KnowledgeError::InvalidEvidence);
        }
    }
    Ok(())
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
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference,
    };

    use super::*;

    fn record(kind: KnowledgeRecordKind) -> KnowledgeRecord {
        let schema = knowledge_schema(kind);
        KnowledgeRecord {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse("knowledge-0001").expect("identity"),
            kind,
            title: "Synthetic record".to_owned(),
            privacy: KnowledgePrivacy::Private,
            sensitivity: DataSensitivity::Durable,
            retention: KnowledgeRetention {
                kind: KnowledgeRetentionKind::UntilSupersededOrDeleted,
                expires_at: None,
            },
            created_at: "2026-08-14T00:00:00Z".to_owned(),
            updated_at: "2026-08-14T00:00:00Z".to_owned(),
            last_verified_at: None,
            fields: schema
                .required_fields
                .iter()
                .map(|name| KnowledgeField {
                    name: (*name).to_owned(),
                    value: "fixture".to_owned(),
                })
                .collect(),
            links: Vec::new(),
            tags: vec!["fixture".to_owned()],
            evidence: vec![EvidenceReference {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw("evidence-0001"),
                kind: EvidenceKind::Document,
                source_id: "fixture-source".to_owned(),
                object_id: "fixture-object".to_owned(),
                fragment: None,
                content_sha256: "a".repeat(64),
                observed_revision: Some("fixture-v1".to_owned()),
            }],
        }
    }

    #[test]
    fn every_canonical_kind_has_one_valid_closed_record() {
        for kind in crate::knowledge_schemas().iter().map(|schema| schema.kind) {
            validate_record(&record(kind)).expect("valid canonical record");
        }
    }

    #[test]
    fn identity_is_path_independent_and_strict() {
        let identity = KnowledgeRecordId::parse("knowledge-person-001").expect("identity");
        assert_eq!(identity.as_str(), "knowledge-person-001");
        for invalid in ["", "person-001", "knowledge-UPPER", "knowledge-path/name"] {
            assert_eq!(
                KnowledgeRecordId::parse(invalid),
                Err(KnowledgeError::InvalidIdentity)
            );
        }
    }

    #[test]
    fn missing_unknown_duplicate_and_oversized_fields_fail_closed() {
        let mut value = record(KnowledgeRecordKind::Person);
        value.fields.clear();
        assert_eq!(
            validate_record(&value),
            Err(KnowledgeError::MissingRequiredField)
        );
        value = record(KnowledgeRecordKind::Person);
        value.fields.push(KnowledgeField {
            name: "unknown".to_owned(),
            value: "fixture".to_owned(),
        });
        assert_eq!(validate_record(&value), Err(KnowledgeError::InvalidField));
        value = record(KnowledgeRecordKind::Person);
        value.fields.push(value.fields[0].clone());
        assert_eq!(validate_record(&value), Err(KnowledgeError::InvalidField));
        value = record(KnowledgeRecordKind::Person);
        value.fields[0].value = "x".repeat(MAX_FIELD_VALUE_BYTES + 1);
        assert_eq!(validate_record(&value), Err(KnowledgeError::InvalidField));
    }

    #[test]
    fn restricted_operational_and_self_linked_records_never_enter_markdown() {
        let mut value = record(KnowledgeRecordKind::Project);
        value.privacy = KnowledgePrivacy::HighlyRestricted;
        assert_eq!(
            validate_record(&value),
            Err(KnowledgeError::PersistenceDenied)
        );
        value = record(KnowledgeRecordKind::Project);
        value.sensitivity = DataSensitivity::Operational;
        assert_eq!(
            validate_record(&value),
            Err(KnowledgeError::PersistenceDenied)
        );
        value = record(KnowledgeRecordKind::Project);
        value.links.push(KnowledgeLink {
            kind: KnowledgeLinkKind::Related,
            target_id: value.record_id.clone(),
        });
        assert_eq!(validate_record(&value), Err(KnowledgeError::InvalidLink));
    }

    #[test]
    fn malformed_retention_tags_and_evidence_fail_closed() {
        let mut value = record(KnowledgeRecordKind::Question);
        value.retention.kind = KnowledgeRetentionKind::UntilExpiration;
        assert_eq!(
            validate_record(&value),
            Err(KnowledgeError::InvalidMetadata)
        );
        value = record(KnowledgeRecordKind::Question);
        value.tags.push("bad tag".to_owned());
        assert_eq!(
            validate_record(&value),
            Err(KnowledgeError::InvalidMetadata)
        );
        value = record(KnowledgeRecordKind::Question);
        value.evidence[0].content_sha256 = "A".repeat(64);
        assert_eq!(
            validate_record(&value),
            Err(KnowledgeError::InvalidEvidence)
        );
    }
}
