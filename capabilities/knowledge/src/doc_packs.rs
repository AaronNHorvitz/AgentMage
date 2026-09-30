//! Offline documentation packs (Decision 0126).
//!
//! A documentation pack is a versioned, licensed set of Markdown or plain text
//! files that a person imports from bytes already on the machine. Import
//! checks the sealed manifest, the license against the person's allow-list and
//! every path, digest and bound, and refuses a replacement that is not an
//! explicit refresh of the current version. Nothing here opens a connection: a
//! refresh is the import of a newer version the person already holds. The
//! newest version is current; a superseded version stays searchable only as
//! history until its retention ends. A deletion removes a version's content
//! and index and leaves a content-free record. Indexing splits each file into
//! bounded fragments for the deterministic knowledge retrieval and withholds
//! every fragment the secret screen flags.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath, WorkspaceScopePath};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::secret_candidate;
use crate::{
    KnowledgeFileType, KnowledgeSourceAuthority, KnowledgeSourceDocument, KnowledgeSourceFragment,
    KnowledgeSourceFragmentKind, ObsidianSourceRange,
};

const SCHEMA_VERSION: u16 = 1;
const MAX_PACK_ID_BYTES: usize = 64;
const MAX_LABEL_BYTES: usize = 200;
const MAX_LICENSE_BYTES: usize = 64;
const MAX_FILES: usize = 10_000;
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PACK_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CATALOG_BYTES: u64 = 256 * 1024 * 1024;
const MAX_FRESH_DAYS: u32 = 3_650;
const MAX_RETENTION_DAYS: u32 = 3_650;
const MAX_FRAGMENT_BYTES: usize = 4 * 1024;
const MAX_VERSION_FRAGMENTS: u64 = 200_000;
const NOTE_KIND: &str = "documentation-pack";

/// Semantic version of a pack; later fields order within earlier ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
}

/// Closed media type of a pack file, fixed by its extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocPackMediaType {
    /// A `.md` file.
    Markdown,
    /// A `.txt` file.
    PlainText,
}

/// One file of a pack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackFile {
    /// Relative `/`-separated path inside the pack.
    pub path: String,
    /// Media type matching the extension.
    pub media_type: DocPackMediaType,
    /// Exact length in bytes.
    pub byte_len: u64,
    /// SHA-256 of the exact bytes.
    pub sha256: String,
}

/// Sealed manifest of one pack version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackManifest {
    /// Manifest schema version.
    pub schema_version: u16,
    /// Plain pack identity.
    pub pack_id: String,
    /// Pack version.
    pub version: DocPackVersion,
    /// Title shown to the person.
    pub title: String,
    /// Publisher shown to the person.
    pub publisher: String,
    /// License identifier, which the person's allow-list must hold.
    pub license: String,
    /// ISO date on which the content was obtained.
    pub retrieved_on: String,
    /// Days after `retrieved_on` during which the pack counts as fresh.
    pub fresh_for_days: u32,
    /// Files sorted by path.
    pub files: Vec<DocPackFile>,
    /// Sum of the file lengths.
    pub total_bytes: u64,
    /// SHA-256 of the canonical manifest with this field empty.
    pub manifest_sha256: String,
}

/// The person's import policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocPackPolicy {
    /// License identifiers the person accepts.
    pub allowed_licenses: BTreeSet<String>,
    /// Days a superseded version is kept before retention deletes it.
    pub superseded_retention_days: u32,
}

/// What an import is meant to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocPackImportIntent {
    /// The pack has no current version.
    New,
    /// A newer version replaces the named current version.
    Refresh {
        /// The current version it replaces.
        replaces: DocPackVersion,
    },
}

/// Content-free record of one import.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DocPackReceipt {
    /// Pack identity.
    pub pack_id: String,
    /// Imported version.
    pub version: DocPackVersion,
    /// Its manifest digest.
    pub manifest_sha256: String,
    /// Its license.
    pub license: String,
    /// The version it superseded, if any.
    pub superseded: Option<DocPackVersion>,
    /// Number of files.
    pub file_count: u64,
    /// Total bytes.
    pub total_bytes: u64,
    /// Indexed fragments.
    pub fragment_count: u64,
    /// Fragments withheld by the secret screen.
    pub withheld_fragment_count: u64,
    /// Fixed false: an import never uses the network.
    pub network_used: bool,
    /// SHA-256 of this receipt with this field empty.
    pub receipt_sha256: String,
}

/// Freshness of a version on a given day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DocPackFreshness {
    /// Still inside its freshness window.
    Fresh {
        /// Days until it becomes stale.
        days_left: u32,
    },
    /// Past its freshness window; a refresh is due.
    Stale {
        /// Days since it became stale.
        days_past: u32,
    },
}

/// Lifecycle state of a kept version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DocPackVersionState {
    /// The version searches return as current.
    Current,
    /// A newer version replaced it; it is history until retention ends.
    Superseded {
        /// ISO date it was superseded.
        superseded_on: String,
        /// ISO date on which retention deletes it.
        delete_on: String,
    },
}

/// Inspection of one kept version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DocPackVersionStatus {
    /// Version.
    pub version: DocPackVersion,
    /// Manifest digest.
    pub manifest_sha256: String,
    /// ISO date its content was obtained.
    pub retrieved_on: String,
    /// Lifecycle state.
    pub state: DocPackVersionState,
    /// Freshness on the inspected day.
    pub freshness: DocPackFreshness,
}

/// Inspection of one pack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DocPackStatus {
    /// Pack identity.
    pub pack_id: String,
    /// Current version, if one is kept.
    pub current: Option<DocPackVersion>,
    /// Every kept version, oldest first.
    pub versions: Vec<DocPackVersionStatus>,
}

/// Why a version was deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocPackDeletionReason {
    /// The person deleted it.
    Person,
    /// Its retention after supersession ended.
    Retention,
}

/// Content-free record of one deleted version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DocPackDeletion {
    /// Pack identity.
    pub pack_id: String,
    /// Deleted version.
    pub version: DocPackVersion,
    /// Its manifest digest.
    pub manifest_sha256: String,
    /// ISO date of deletion.
    pub deleted_on: String,
    /// Why.
    pub reason: DocPackDeletionReason,
}

/// Searchable documents of one pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocPackIndex {
    /// One document per file of every kept version; superseded versions are
    /// marked historical.
    pub documents: Vec<KnowledgeSourceDocument>,
    /// Fragments indexed.
    pub fragment_count: u64,
    /// Fragments withheld by the secret screen.
    pub withheld_fragment_count: u64,
}

/// Content-free documentation pack failure. A failed operation changes
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocPackError {
    /// The manifest's shape, fields, paths, bounds or seal are invalid.
    ManifestInvalid,
    /// The person's allow-list does not hold the license.
    LicenseNotAllowed,
    /// The supplied files differ from the manifest.
    ContentMismatch,
    /// A file is not text without control characters.
    ContentInvalid,
    /// The intent does not name the pack's current state.
    IntentMismatch,
    /// The version is not newer than every kept version.
    NotNewer,
    /// This exact version is already kept.
    AlreadyImported,
    /// Another manifest with the same version is kept.
    Conflict,
    /// No such pack or version.
    NotFound,
    /// A pack or the catalog would exceed its bound.
    ResourceLimit,
    /// A date is malformed, precedes the content or goes backwards.
    InvalidDate,
    /// The policy is out of bounds.
    PolicyInvalid,
}

impl DocPackError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ManifestInvalid => "doc-pack.manifest-invalid",
            Self::LicenseNotAllowed => "doc-pack.license-not-allowed",
            Self::ContentMismatch => "doc-pack.content-mismatch",
            Self::ContentInvalid => "doc-pack.content-invalid",
            Self::IntentMismatch => "doc-pack.intent-mismatch",
            Self::NotNewer => "doc-pack.not-newer",
            Self::AlreadyImported => "doc-pack.already-imported",
            Self::Conflict => "doc-pack.conflict",
            Self::NotFound => "doc-pack.not-found",
            Self::ResourceLimit => "doc-pack.resource-limit",
            Self::InvalidDate => "doc-pack.invalid-date",
            Self::PolicyInvalid => "doc-pack.policy-invalid",
        }
    }
}

/// Seals a manifest after checking every field.
pub fn seal_doc_pack_manifest(
    mut manifest: DocPackManifest,
) -> Result<DocPackManifest, DocPackError> {
    manifest.manifest_sha256.clear();
    validate_manifest_fields(&manifest)?;
    manifest.manifest_sha256 = manifest_digest(&manifest)?;
    Ok(manifest)
}

/// Parses and verifies a manifest; unknown fields are refused.
pub fn parse_doc_pack_manifest(bytes: &[u8]) -> Result<DocPackManifest, DocPackError> {
    let manifest: DocPackManifest =
        serde_json::from_slice(bytes).map_err(|_| DocPackError::ManifestInvalid)?;
    verify_manifest(&manifest)?;
    Ok(manifest)
}

fn verify_manifest(manifest: &DocPackManifest) -> Result<(), DocPackError> {
    validate_manifest_fields(manifest)?;
    if manifest_digest(manifest)? != manifest.manifest_sha256 {
        return Err(DocPackError::ManifestInvalid);
    }
    Ok(())
}

fn validate_manifest_fields(manifest: &DocPackManifest) -> Result<(), DocPackError> {
    let invalid = DocPackError::ManifestInvalid;
    if manifest.schema_version != SCHEMA_VERSION
        || !plain_pack_id(&manifest.pack_id)
        || !label(&manifest.title)
        || !label(&manifest.publisher)
        || manifest.license.is_empty()
        || manifest.license.len() > MAX_LICENSE_BYTES
        || !manifest
            .license
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        || manifest.fresh_for_days == 0
        || manifest.fresh_for_days > MAX_FRESH_DAYS
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES
        || manifest
            .files
            .windows(2)
            .any(|pair| pair[0].path >= pair[1].path)
    {
        return Err(invalid);
    }
    day_number(&manifest.retrieved_on).map_err(|_| invalid)?;
    let mut total = 0_u64;
    for file in &manifest.files {
        let expected = if file.path.ends_with(".md") {
            DocPackMediaType::Markdown
        } else if file.path.ends_with(".txt") {
            DocPackMediaType::PlainText
        } else {
            return Err(invalid);
        };
        pack_path(&manifest.pack_id, manifest.version, &file.path)?;
        if file.media_type != expected
            || file.byte_len == 0
            || file.byte_len > MAX_FILE_BYTES
            || !valid_sha256(&file.sha256)
        {
            return Err(invalid);
        }
        total = total.checked_add(file.byte_len).ok_or(invalid)?;
    }
    if total != manifest.total_bytes || total > MAX_PACK_BYTES {
        return Err(invalid);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KeptVersion {
    manifest: DocPackManifest,
    contents: BTreeMap<String, Vec<u8>>,
    superseded_on: Option<i64>,
}

/// In-memory catalog of imported packs with one owner.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocPackCatalog {
    packs: BTreeMap<String, BTreeMap<DocPackVersion, KeptVersion>>,
    deletions: Vec<DocPackDeletion>,
    total_bytes: u64,
    last_day: i64,
}

impl DocPackCatalog {
    /// An empty catalog.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            packs: BTreeMap::new(),
            deletions: Vec::new(),
            total_bytes: 0,
            last_day: 0,
        }
    }

    /// Imports one pack version from the supplied file bytes, keyed by path.
    pub fn import(
        &mut self,
        policy: &DocPackPolicy,
        manifest: &DocPackManifest,
        contents: &BTreeMap<String, Vec<u8>>,
        intent: DocPackImportIntent,
        today: &str,
    ) -> Result<DocPackReceipt, DocPackError> {
        validate_policy(policy)?;
        verify_manifest(manifest)?;
        let day = self.day(today)?;
        if day < day_number(&manifest.retrieved_on)? {
            return Err(DocPackError::InvalidDate);
        }
        if !policy.allowed_licenses.contains(&manifest.license) {
            return Err(DocPackError::LicenseNotAllowed);
        }
        if contents.len() != manifest.files.len() {
            return Err(DocPackError::ContentMismatch);
        }
        for file in &manifest.files {
            let bytes = contents
                .get(&file.path)
                .ok_or(DocPackError::ContentMismatch)?;
            if bytes.len() as u64 != file.byte_len || sha256_hex(bytes) != file.sha256 {
                return Err(DocPackError::ContentMismatch);
            }
            let text = std::str::from_utf8(bytes).map_err(|_| DocPackError::ContentInvalid)?;
            if text
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
            {
                return Err(DocPackError::ContentInvalid);
            }
        }
        let kept = self.packs.get(&manifest.pack_id);
        if let Some(existing) = kept.and_then(|versions| versions.get(&manifest.version)) {
            return Err(
                if existing.manifest.manifest_sha256 == manifest.manifest_sha256 {
                    DocPackError::AlreadyImported
                } else {
                    DocPackError::Conflict
                },
            );
        }
        let current = kept.and_then(current_version);
        match (intent, current) {
            (DocPackImportIntent::New, None) => {}
            (DocPackImportIntent::Refresh { replaces }, Some(current)) if replaces == current => {}
            _ => return Err(DocPackError::IntentMismatch),
        }
        if kept
            .and_then(|versions| versions.keys().next_back())
            .is_some_and(|newest| *newest >= manifest.version)
        {
            return Err(DocPackError::NotNewer);
        }
        let total_bytes = self
            .total_bytes
            .checked_add(manifest.total_bytes)
            .filter(|total| *total <= MAX_CATALOG_BYTES)
            .ok_or(DocPackError::ResourceLimit)?;
        let version = KeptVersion {
            manifest: manifest.clone(),
            contents: contents.clone(),
            superseded_on: None,
        };
        let (_, fragment_count, withheld_fragment_count) = version_documents(&version)?;
        let mut receipt = DocPackReceipt {
            pack_id: manifest.pack_id.clone(),
            version: manifest.version,
            manifest_sha256: manifest.manifest_sha256.clone(),
            license: manifest.license.clone(),
            superseded: current,
            file_count: manifest.files.len() as u64,
            total_bytes: manifest.total_bytes,
            fragment_count,
            withheld_fragment_count,
            network_used: false,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = sha256_json(&receipt)?;
        let versions = self.packs.entry(manifest.pack_id.clone()).or_default();
        if let Some(current) = current
            && let Some(replaced) = versions.get_mut(&current)
        {
            replaced.superseded_on = Some(day);
        }
        versions.insert(manifest.version, version);
        self.total_bytes = total_bytes;
        self.last_day = day;
        Ok(receipt)
    }

    /// Inspects every kept version of one pack on a given day.
    pub fn inspect(
        &self,
        policy: &DocPackPolicy,
        pack_id: &str,
        today: &str,
    ) -> Result<DocPackStatus, DocPackError> {
        validate_policy(policy)?;
        let day = self.day(today)?;
        let versions = self.packs.get(pack_id).ok_or(DocPackError::NotFound)?;
        let mut statuses = Vec::with_capacity(versions.len());
        for (version, kept) in versions {
            let fresh_until =
                day_number(&kept.manifest.retrieved_on)? + i64::from(kept.manifest.fresh_for_days);
            let freshness = if day < fresh_until {
                DocPackFreshness::Fresh {
                    days_left: u32::try_from(fresh_until - day).unwrap_or(u32::MAX),
                }
            } else {
                DocPackFreshness::Stale {
                    days_past: u32::try_from(day - fresh_until).unwrap_or(u32::MAX),
                }
            };
            let state = match kept.superseded_on {
                None => DocPackVersionState::Current,
                Some(superseded_on) => DocPackVersionState::Superseded {
                    superseded_on: day_text(superseded_on)?,
                    delete_on: day_text(
                        superseded_on + i64::from(policy.superseded_retention_days),
                    )?,
                },
            };
            statuses.push(DocPackVersionStatus {
                version: *version,
                manifest_sha256: kept.manifest.manifest_sha256.clone(),
                retrieved_on: kept.manifest.retrieved_on.clone(),
                state,
                freshness,
            });
        }
        Ok(DocPackStatus {
            pack_id: pack_id.to_owned(),
            current: current_version(versions),
            versions: statuses,
        })
    }

    /// Deletes every superseded version whose retention ended. The current
    /// version is never deleted by retention.
    pub fn apply_retention(
        &mut self,
        policy: &DocPackPolicy,
        today: &str,
    ) -> Result<Vec<DocPackDeletion>, DocPackError> {
        validate_policy(policy)?;
        let day = self.day(today)?;
        let retention = i64::from(policy.superseded_retention_days);
        let expired = self
            .packs
            .iter()
            .flat_map(|(pack_id, versions)| {
                versions
                    .iter()
                    .filter(move |(_, kept)| {
                        kept.superseded_on
                            .is_some_and(|superseded_on| superseded_on + retention <= day)
                    })
                    .map(move |(version, _)| (pack_id.clone(), *version))
            })
            .collect::<Vec<_>>();
        let deleted = expired
            .into_iter()
            .map(|(pack_id, version)| {
                self.remove(&pack_id, version, day, DocPackDeletionReason::Retention)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.last_day = day;
        Ok(deleted)
    }

    /// Deletes one kept version, or every kept version of the pack. Deleting
    /// the current version leaves the pack without one; no older version
    /// becomes current.
    pub fn delete(
        &mut self,
        pack_id: &str,
        version: Option<DocPackVersion>,
        today: &str,
    ) -> Result<Vec<DocPackDeletion>, DocPackError> {
        let day = self.day(today)?;
        let versions = self.packs.get(pack_id).ok_or(DocPackError::NotFound)?;
        let targets = match version {
            Some(version) if versions.contains_key(&version) => vec![version],
            Some(_) => return Err(DocPackError::NotFound),
            None => versions.keys().copied().collect(),
        };
        let deleted = targets
            .into_iter()
            .map(|version| self.remove(pack_id, version, day, DocPackDeletionReason::Person))
            .collect::<Result<Vec<_>, _>>()?;
        self.last_day = day;
        Ok(deleted)
    }

    /// Builds the searchable documents of one pack.
    pub fn index(&self, pack_id: &str) -> Result<DocPackIndex, DocPackError> {
        let versions = self.packs.get(pack_id).ok_or(DocPackError::NotFound)?;
        let mut index = DocPackIndex {
            documents: Vec::new(),
            fragment_count: 0,
            withheld_fragment_count: 0,
        };
        for kept in versions.values() {
            let (documents, fragments, withheld) = version_documents(kept)?;
            index.documents.extend(documents);
            index.fragment_count += fragments;
            index.withheld_fragment_count += withheld;
        }
        Ok(index)
    }

    /// Every deletion so far, in order.
    #[must_use]
    pub fn deletions(&self) -> &[DocPackDeletion] {
        &self.deletions
    }

    /// Bytes held across every kept version.
    #[must_use]
    pub const fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    fn remove(
        &mut self,
        pack_id: &str,
        version: DocPackVersion,
        day: i64,
        reason: DocPackDeletionReason,
    ) -> Result<DocPackDeletion, DocPackError> {
        let versions = self.packs.get_mut(pack_id).ok_or(DocPackError::NotFound)?;
        let kept = versions.remove(&version).ok_or(DocPackError::NotFound)?;
        if versions.is_empty() {
            self.packs.remove(pack_id);
        }
        self.total_bytes -= kept.manifest.total_bytes;
        let deletion = DocPackDeletion {
            pack_id: pack_id.to_owned(),
            version,
            manifest_sha256: kept.manifest.manifest_sha256,
            deleted_on: day_text(day)?,
            reason,
        };
        self.deletions.push(deletion.clone());
        Ok(deletion)
    }

    /// Parses a day that must not precede the last day a change used.
    fn day(&self, today: &str) -> Result<i64, DocPackError> {
        let day = day_number(today)?;
        if day < self.last_day {
            return Err(DocPackError::InvalidDate);
        }
        Ok(day)
    }
}

fn current_version(versions: &BTreeMap<DocPackVersion, KeptVersion>) -> Option<DocPackVersion> {
    versions
        .iter()
        .find_map(|(version, kept)| kept.superseded_on.is_none().then_some(*version))
}

fn validate_policy(policy: &DocPackPolicy) -> Result<(), DocPackError> {
    if policy.superseded_retention_days > MAX_RETENTION_DAYS
        || policy.allowed_licenses.iter().any(|license| {
            license.is_empty()
                || license.len() > MAX_LICENSE_BYTES
                || !license
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        })
    {
        return Err(DocPackError::PolicyInvalid);
    }
    Ok(())
}

/// The workspace identity under which one pack version is searched.
fn pack_workspace(pack_id: &str, version: DocPackVersion) -> WorkspaceId {
    WorkspaceId::from_raw(format!(
        "doc-pack:{pack_id}:{}.{}.{}",
        version.major, version.minor, version.patch
    ))
}

fn pack_path(
    pack_id: &str,
    version: DocPackVersion,
    path: &str,
) -> Result<WorkspacePath, DocPackError> {
    WorkspacePath::new(pack_workspace(pack_id, version), path.split('/'))
        .map_err(|_| DocPackError::ManifestInvalid)
}

/// Splits every file of one version into bounded fragments.
fn version_documents(
    kept: &KeptVersion,
) -> Result<(Vec<KnowledgeSourceDocument>, u64, u64), DocPackError> {
    let manifest = &kept.manifest;
    let root = WorkspaceScopePath::new(
        pack_workspace(&manifest.pack_id, manifest.version),
        Vec::<String>::new(),
    )
    .map_err(|_| DocPackError::ManifestInvalid)?;
    let mut documents = Vec::with_capacity(manifest.files.len());
    let mut fragment_count = 0_u64;
    let mut withheld_count = 0_u64;
    for file in &manifest.files {
        let bytes = kept
            .contents
            .get(&file.path)
            .ok_or(DocPackError::ContentMismatch)?;
        let text = std::str::from_utf8(bytes).map_err(|_| DocPackError::ContentInvalid)?;
        let markdown = file.media_type == DocPackMediaType::Markdown;
        let mut fragments = Vec::new();
        for fragment in split_fragments(text, markdown) {
            if secret_candidate(&fragment.text) {
                withheld_count += 1;
            } else {
                fragments.push(fragment);
            }
        }
        fragment_count += fragments.len() as u64;
        if fragment_count + withheld_count > MAX_VERSION_FRAGMENTS {
            return Err(DocPackError::ResourceLimit);
        }
        documents.push(KnowledgeSourceDocument {
            root: root.clone(),
            path: pack_path(&manifest.pack_id, manifest.version, &file.path)?,
            file_type: if markdown {
                KnowledgeFileType::Markdown
            } else {
                KnowledgeFileType::Text
            },
            authority: KnowledgeSourceAuthority::DirectEvidence,
            content_sha256: file.sha256.clone(),
            current_content_sha256: file.sha256.clone(),
            verified_on: manifest.retrieved_on.clone(),
            source_date: manifest.retrieved_on.clone(),
            note_kind: Some(NOTE_KIND.to_owned()),
            historical: kept.superseded_on.is_some(),
            denied: false,
            fragments,
        });
    }
    Ok((documents, fragment_count, withheld_count))
}

/// Splits text into paragraphs of consecutive non-blank lines. A Markdown
/// heading line is its own fragment. A paragraph longer than the fragment
/// bound is cut at line ends, and a longer line at character boundaries.
fn split_fragments(text: &str, markdown: bool) -> Vec<KnowledgeSourceFragment> {
    let mut fragments = Vec::new();
    let mut paragraph: Vec<(u32, &str)> = Vec::new();
    for (index, raw) in text.split('\n').enumerate() {
        let line_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.trim().is_empty() {
            flush_paragraph(&mut paragraph, &mut fragments);
        } else if markdown && heading(line) {
            flush_paragraph(&mut paragraph, &mut fragments);
            push_line_pieces(
                line_number,
                line,
                KnowledgeSourceFragmentKind::Heading,
                &mut fragments,
            );
        } else {
            let length = paragraph
                .iter()
                .map(|(_, text)| text.len() + 1)
                .sum::<usize>();
            if !paragraph.is_empty() && length + line.len() > MAX_FRAGMENT_BYTES {
                flush_paragraph(&mut paragraph, &mut fragments);
            }
            if line.len() > MAX_FRAGMENT_BYTES {
                push_line_pieces(
                    line_number,
                    line,
                    KnowledgeSourceFragmentKind::Body,
                    &mut fragments,
                );
            } else {
                paragraph.push((line_number, line));
            }
        }
    }
    flush_paragraph(&mut paragraph, &mut fragments);
    fragments
}

fn heading(line: &str) -> bool {
    let marks = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6).contains(&marks) && line[marks..].starts_with(' ')
}

fn flush_paragraph(paragraph: &mut Vec<(u32, &str)>, fragments: &mut Vec<KnowledgeSourceFragment>) {
    let (Some((first, _)), Some((last, last_text))) = (paragraph.first(), paragraph.last()) else {
        return;
    };
    fragments.push(KnowledgeSourceFragment {
        kind: KnowledgeSourceFragmentKind::Body,
        text: paragraph
            .iter()
            .map(|(_, text)| *text)
            .collect::<Vec<_>>()
            .join("\n"),
        source_range: ObsidianSourceRange {
            start_line: *first,
            start_column: 1,
            end_line: *last,
            end_column: column(last_text.len()),
        },
        fact_key: None,
    });
    paragraph.clear();
}

/// Pushes one line as fragments of at most the bound, cut at character
/// boundaries.
fn push_line_pieces(
    line_number: u32,
    line: &str,
    kind: KnowledgeSourceFragmentKind,
    fragments: &mut Vec<KnowledgeSourceFragment>,
) {
    let mut start = 0;
    while start < line.len() {
        let mut end = (start + MAX_FRAGMENT_BYTES).min(line.len());
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        fragments.push(KnowledgeSourceFragment {
            kind,
            text: line[start..end].to_owned(),
            source_range: ObsidianSourceRange {
                start_line: line_number,
                start_column: column(start),
                end_line: line_number,
                end_column: column(end),
            },
            fact_key: None,
        });
        start = end;
    }
}

/// One-based column of a zero-based byte offset.
fn column(offset: usize) -> u32 {
    u32::try_from(offset + 1).unwrap_or(u32::MAX)
}

fn plain_pack_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PACK_ID_BYTES
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

fn label(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_LABEL_BYTES
        && !value.chars().any(char::is_control)
        && !secret_candidate(value)
}

/// Days since 0000-03-01 of an ISO `YYYY-MM-DD` date.
fn day_number(value: &str) -> Result<i64, DocPackError> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return Err(DocPackError::InvalidDate);
    }
    let number = |range: std::ops::Range<usize>| -> i64 {
        value[range]
            .bytes()
            .fold(0, |total, byte| total * 10 + i64::from(byte - b'0'))
    };
    let (year, month, day) = (number(0..4), number(5..7), number(8..10));
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err(DocPackError::InvalidDate),
    };
    if year == 0 || day == 0 || day > days_in_month {
        return Err(DocPackError::InvalidDate);
    }
    let shifted_year = if month <= 2 { year - 1 } else { year };
    let shifted_month = (month + 9) % 12;
    Ok(365 * shifted_year + shifted_year / 4 - shifted_year / 100
        + shifted_year / 400
        + (153 * shifted_month + 2) / 5
        + day
        - 1)
}

/// The ISO date of a day number.
fn day_text(day: i64) -> Result<String, DocPackError> {
    let mut year = (10_000 * day + 14_780) / 3_652_425;
    let mut remainder = day - (365 * year + year / 4 - year / 100 + year / 400);
    if remainder < 0 {
        year -= 1;
        remainder = day - (365 * year + year / 4 - year / 100 + year / 400);
    }
    let shifted_month = (100 * remainder + 52) / 3_060;
    let month = (shifted_month + 2) % 12 + 1;
    let year = year + (shifted_month + 2) / 12;
    let day_of_month = remainder - (shifted_month * 306 + 5) / 10 + 1;
    if !(1..=9_999).contains(&year) {
        return Err(DocPackError::InvalidDate);
    }
    Ok(format!("{year:04}-{month:02}-{day_of_month:02}"))
}

fn manifest_digest(manifest: &DocPackManifest) -> Result<String, DocPackError> {
    let mut canonical = manifest.clone();
    canonical.manifest_sha256.clear();
    sha256_json(&canonical).map_err(|_| DocPackError::ManifestInvalid)
}

fn sha256_json(value: &impl Serialize) -> Result<String, DocPackError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| DocPackError::ManifestInvalid)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KnowledgeContextQuery, retrieve_knowledge};

    const V1: DocPackVersion = DocPackVersion {
        major: 1,
        minor: 0,
        patch: 0,
    };
    const V2: DocPackVersion = DocPackVersion {
        major: 1,
        minor: 1,
        patch: 0,
    };
    // Synthetic canary, assembled so no complete token appears in source.
    const CANARY: &str = concat!("gh", "p_", "DOCPACKCANARYaaaaaaaaaaaaaaaaaaaaaaaaaa");

    fn policy() -> DocPackPolicy {
        DocPackPolicy {
            allowed_licenses: BTreeSet::from(["CC-BY-4.0".to_owned(), "MIT".to_owned()]),
            superseded_retention_days: 30,
        }
    }

    fn pack(
        version: DocPackVersion,
        retrieved_on: &str,
        files: &[(&str, &str)],
    ) -> (DocPackManifest, BTreeMap<String, Vec<u8>>) {
        let contents = files
            .iter()
            .map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec()))
            .collect::<BTreeMap<_, _>>();
        let manifest = seal_doc_pack_manifest(DocPackManifest {
            schema_version: SCHEMA_VERSION,
            pack_id: "build-tool-guide".to_owned(),
            version,
            title: "Build tool guide".to_owned(),
            publisher: "Example publisher".to_owned(),
            license: "CC-BY-4.0".to_owned(),
            retrieved_on: retrieved_on.to_owned(),
            fresh_for_days: 30,
            files: contents
                .iter()
                .map(|(path, bytes)| DocPackFile {
                    path: path.clone(),
                    media_type: if path.ends_with(".md") {
                        DocPackMediaType::Markdown
                    } else {
                        DocPackMediaType::PlainText
                    },
                    byte_len: bytes.len() as u64,
                    sha256: sha256_hex(bytes),
                })
                .collect(),
            total_bytes: contents.values().map(|bytes| bytes.len() as u64).sum(),
            manifest_sha256: String::new(),
        })
        .unwrap();
        (manifest, contents)
    }

    fn guide(
        version: DocPackVersion,
        retrieved_on: &str,
    ) -> (DocPackManifest, BTreeMap<String, Vec<u8>>) {
        let cache = format!(
            "# Build cache\n\nThe cache keeps compiled units.\nClear it with the clean step.\n\n## Credentials\nUse token {CANARY} here.\n"
        );
        pack(
            version,
            retrieved_on,
            &[
                ("guide/cache.md", &cache),
                ("notes.txt", "Offline notes about the cache.\n"),
            ],
        )
    }

    type ManifestChange = Box<dyn Fn(&mut DocPackManifest)>;

    fn reseal(
        manifest: &DocPackManifest,
        change: impl FnOnce(&mut DocPackManifest),
    ) -> Result<DocPackManifest, DocPackError> {
        let mut changed = manifest.clone();
        change(&mut changed);
        seal_doc_pack_manifest(changed)
    }

    #[test]
    fn an_import_checks_the_seal_license_paths_digests_and_content_and_changes_nothing_on_failure()
    {
        let (manifest, contents) = guide(V1, "2026-01-01");
        // Paths, media types, order, bounds, identity, labels and dates are
        // refused before sealing.
        let refused: Vec<ManifestChange> = vec![
            Box::new(|value| value.files[0].path = "../escape.md".to_owned()),
            Box::new(|value| value.files[0].path = "/absolute.md".to_owned()),
            Box::new(|value| value.files[0].path = "guide//cache.md".to_owned()),
            Box::new(|value| value.files[0].path = "guide/./cache.md".to_owned()),
            Box::new(|value| value.files[0].path = "guide/cache.html".to_owned()),
            Box::new(|value| value.files[0].media_type = DocPackMediaType::PlainText),
            Box::new(|value| value.files.swap(0, 1)),
            Box::new(|value| value.files[1] = value.files[0].clone()),
            Box::new(|value| value.files[0].byte_len = 0),
            Box::new(|value| value.files[0].sha256 = "A".repeat(64)),
            Box::new(|value| value.total_bytes += 1),
            Box::new(|value| value.files.clear()),
            Box::new(|value| value.pack_id = "Build-Guide".to_owned()),
            Box::new(|value| value.title = format!("Guide {CANARY}")),
            Box::new(|value| value.publisher = "line\nbreak".to_owned()),
            Box::new(|value| value.license = "MIT OR Apache-2.0".to_owned()),
            Box::new(|value| value.retrieved_on = "2026-02-30".to_owned()),
            Box::new(|value| value.fresh_for_days = 0),
            Box::new(|value| value.schema_version = 2),
        ];
        for change in refused {
            assert_eq!(
                reseal(&manifest, change),
                Err(DocPackError::ManifestInvalid)
            );
        }
        let mut catalog = DocPackCatalog::new();
        let mut tampered = manifest.clone();
        tampered.title = "Another title".to_owned();
        let mut changed = contents.clone();
        changed.insert(
            "notes.txt".to_owned(),
            b"Offline notes about the cachE.\n".to_vec(),
        );
        let mut missing = contents.clone();
        missing.remove("notes.txt");
        let mut extra = contents.clone();
        extra.insert("zz.md".to_owned(), b"x".to_vec());
        let (binary, binary_contents) = pack(V1, "2026-01-01", &[("bin.txt", "a\u{0}b")]);
        let (unlicensed, unlicensed_contents) = (
            reseal(&manifest, |value| {
                value.license = "Proprietary-1".to_owned()
            })
            .unwrap(),
            contents.clone(),
        );
        let cases = [
            (
                &tampered,
                &contents,
                "2026-01-02",
                DocPackError::ManifestInvalid,
            ),
            (
                &unlicensed,
                &unlicensed_contents,
                "2026-01-02",
                DocPackError::LicenseNotAllowed,
            ),
            (
                &manifest,
                &changed,
                "2026-01-02",
                DocPackError::ContentMismatch,
            ),
            (
                &manifest,
                &missing,
                "2026-01-02",
                DocPackError::ContentMismatch,
            ),
            (
                &manifest,
                &extra,
                "2026-01-02",
                DocPackError::ContentMismatch,
            ),
            (
                &binary,
                &binary_contents,
                "2026-01-02",
                DocPackError::ContentInvalid,
            ),
            (
                &manifest,
                &contents,
                "2025-12-31",
                DocPackError::InvalidDate,
            ),
            (&manifest, &contents, "2026-1-02", DocPackError::InvalidDate),
        ];
        for (manifest, contents, today, error) in cases {
            assert_eq!(
                catalog.import(
                    &policy(),
                    manifest,
                    contents,
                    DocPackImportIntent::New,
                    today
                ),
                Err(error)
            );
            assert_eq!(catalog, DocPackCatalog::new());
        }
        let mut bad_policy = policy();
        bad_policy.superseded_retention_days = MAX_RETENTION_DAYS + 1;
        assert_eq!(
            catalog.import(
                &bad_policy,
                &manifest,
                &contents,
                DocPackImportIntent::New,
                "2026-01-02"
            ),
            Err(DocPackError::PolicyInvalid)
        );
        // Invalid UTF-8 is refused as content.
        let mut invalid_utf8 = contents.clone();
        invalid_utf8.insert("notes.txt".to_owned(), vec![0xff; 31]);
        let invalid_manifest = reseal(&manifest, |value| {
            value.files[1].sha256 = sha256_hex(&[0xff; 31]);
            value.files[1].byte_len = 31;
            value.total_bytes = value.files[0].byte_len + 31;
        })
        .unwrap();
        assert_eq!(
            catalog.import(
                &policy(),
                &invalid_manifest,
                &invalid_utf8,
                DocPackImportIntent::New,
                "2026-01-02"
            ),
            Err(DocPackError::ContentInvalid)
        );
        // A manifest file is parsed closed and verified.
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(parse_doc_pack_manifest(&bytes), Ok(manifest.clone()));
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["source_url"] = serde_json::Value::from("https://example.test/");
        assert_eq!(
            parse_doc_pack_manifest(&serde_json::to_vec(&value).unwrap()),
            Err(DocPackError::ManifestInvalid)
        );
        assert_eq!(
            parse_doc_pack_manifest(&serde_json::to_vec(&tampered).unwrap()),
            Err(DocPackError::ManifestInvalid)
        );
    }

    #[test]
    fn a_refresh_must_name_the_current_version_and_supersede_it() {
        let mut catalog = DocPackCatalog::new();
        let (first, first_contents) = guide(V1, "2026-01-01");
        let receipt = catalog
            .import(
                &policy(),
                &first,
                &first_contents,
                DocPackImportIntent::New,
                "2026-01-02",
            )
            .unwrap();
        assert_eq!(
            (receipt.version, receipt.superseded, receipt.file_count),
            (V1, None, 2)
        );
        assert!(!receipt.network_used);
        let mut unsealed = receipt.clone();
        unsealed.receipt_sha256.clear();
        assert_eq!(receipt.receipt_sha256, sha256_json(&unsealed).unwrap());
        assert_eq!(
            catalog.import(
                &policy(),
                &first,
                &first_contents,
                DocPackImportIntent::New,
                "2026-01-02"
            ),
            Err(DocPackError::AlreadyImported)
        );
        let (other_first, other_contents) =
            pack(V1, "2026-01-01", &[("other.md", "Other text.\n")]);
        assert_eq!(
            catalog.import(
                &policy(),
                &other_first,
                &other_contents,
                DocPackImportIntent::Refresh { replaces: V1 },
                "2026-01-02"
            ),
            Err(DocPackError::Conflict)
        );
        let (second, second_contents) = guide(V2, "2026-02-01");
        let (older, older_contents) = guide(
            DocPackVersion {
                major: 0,
                minor: 9,
                patch: 0,
            },
            "2026-02-01",
        );
        for (manifest, contents, intent, error) in [
            (
                &second,
                &second_contents,
                DocPackImportIntent::New,
                DocPackError::IntentMismatch,
            ),
            (
                &second,
                &second_contents,
                DocPackImportIntent::Refresh { replaces: V2 },
                DocPackError::IntentMismatch,
            ),
            (
                &older,
                &older_contents,
                DocPackImportIntent::Refresh { replaces: V1 },
                DocPackError::NotNewer,
            ),
        ] {
            assert_eq!(
                catalog.import(&policy(), manifest, contents, intent, "2026-02-01"),
                Err(error)
            );
        }
        let refreshed = catalog
            .import(
                &policy(),
                &second,
                &second_contents,
                DocPackImportIntent::Refresh { replaces: V1 },
                "2026-02-01",
            )
            .unwrap();
        assert_eq!(refreshed.superseded, Some(V1));
        let status = catalog
            .inspect(&policy(), "build-tool-guide", "2026-02-01")
            .unwrap();
        assert_eq!(status.current, Some(V2));
        assert_eq!(
            status
                .versions
                .iter()
                .map(|version| (version.version, version.state.clone()))
                .collect::<Vec<_>>(),
            vec![
                (
                    V1,
                    DocPackVersionState::Superseded {
                        superseded_on: "2026-02-01".to_owned(),
                        delete_on: "2026-03-03".to_owned(),
                    }
                ),
                (V2, DocPackVersionState::Current),
            ]
        );
        assert_eq!(
            catalog.total_bytes(),
            first.total_bytes + second.total_bytes
        );
        assert_eq!(
            catalog.inspect(&policy(), "absent-pack", "2026-02-01"),
            Err(DocPackError::NotFound)
        );
    }

    #[test]
    fn catalog_bytes_and_version_fragments_are_bounded() {
        let (manifest, contents) = guide(V1, "2026-01-01");
        let mut catalog = DocPackCatalog::new();
        catalog.total_bytes = MAX_CATALOG_BYTES - manifest.total_bytes + 1;
        let before = catalog.clone();
        assert_eq!(
            catalog.import(
                &policy(),
                &manifest,
                &contents,
                DocPackImportIntent::New,
                "2026-01-01"
            ),
            Err(DocPackError::ResourceLimit)
        );
        assert_eq!(catalog, before);
        catalog.total_bytes -= 1;
        assert!(
            catalog
                .import(
                    &policy(),
                    &manifest,
                    &contents,
                    DocPackImportIntent::New,
                    "2026-01-01"
                )
                .is_ok()
        );
        let many = "a\n\n".repeat(usize::try_from(MAX_VERSION_FRAGMENTS).unwrap() + 1);
        let (large, large_contents) = pack(V1, "2026-01-01", &[("many.txt", &many)]);
        let mut catalog = DocPackCatalog::new();
        assert_eq!(
            catalog.import(
                &policy(),
                &large,
                &large_contents,
                DocPackImportIntent::New,
                "2026-01-01"
            ),
            Err(DocPackError::ResourceLimit)
        );
        assert_eq!(catalog, DocPackCatalog::new());
    }

    #[test]
    fn freshness_is_inspected_on_the_given_day() {
        let mut catalog = DocPackCatalog::new();
        let (manifest, contents) = guide(V1, "2028-02-01");
        catalog
            .import(
                &policy(),
                &manifest,
                &contents,
                DocPackImportIntent::New,
                "2028-02-01",
            )
            .unwrap();
        for (today, freshness) in [
            ("2028-02-01", DocPackFreshness::Fresh { days_left: 30 }),
            ("2028-03-01", DocPackFreshness::Fresh { days_left: 1 }),
            ("2028-03-02", DocPackFreshness::Stale { days_past: 0 }),
            ("2029-03-02", DocPackFreshness::Stale { days_past: 365 }),
        ] {
            let status = catalog
                .inspect(&policy(), "build-tool-guide", today)
                .unwrap();
            assert_eq!(status.versions[0].freshness, freshness, "{today}");
        }
        assert_eq!(
            catalog.inspect(&policy(), "build-tool-guide", "2028-01-31"),
            Err(DocPackError::InvalidDate)
        );
        for date in [
            "2028-02-29",
            "2000-02-29",
            "1999-12-31",
            "2026-03-01",
            "0001-01-01",
        ] {
            assert_eq!(day_text(day_number(date).unwrap()).unwrap(), date);
        }
        for date in [
            "2027-02-29",
            "1900-02-29",
            "2026-13-01",
            "2026-00-10",
            "0000-01-01",
        ] {
            assert_eq!(day_number(date), Err(DocPackError::InvalidDate), "{date}");
        }
    }

    #[test]
    fn retention_deletes_only_superseded_versions_and_deletion_keeps_no_content() {
        let mut catalog = DocPackCatalog::new();
        let (first, first_contents) = guide(V1, "2026-01-01");
        let (second, second_contents) = guide(V2, "2026-01-15");
        catalog
            .import(
                &policy(),
                &first,
                &first_contents,
                DocPackImportIntent::New,
                "2026-01-01",
            )
            .unwrap();
        catalog
            .import(
                &policy(),
                &second,
                &second_contents,
                DocPackImportIntent::Refresh { replaces: V1 },
                "2026-02-01",
            )
            .unwrap();
        assert!(
            catalog
                .apply_retention(&policy(), "2026-03-02")
                .unwrap()
                .is_empty()
        );
        let expired = catalog.apply_retention(&policy(), "2026-03-03").unwrap();
        assert_eq!(
            expired
                .iter()
                .map(|deletion| (deletion.version, deletion.reason))
                .collect::<Vec<_>>(),
            vec![(V1, DocPackDeletionReason::Retention)]
        );
        // The current version stays, however stale.
        assert!(
            catalog
                .apply_retention(&policy(), "2030-01-01")
                .unwrap()
                .is_empty()
        );
        let status = catalog
            .inspect(&policy(), "build-tool-guide", "2030-01-01")
            .unwrap();
        assert_eq!(status.versions.len(), 1);
        assert!(matches!(
            status.versions[0].freshness,
            DocPackFreshness::Stale { .. }
        ));
        // Dates never go backwards.
        assert_eq!(
            catalog.apply_retention(&policy(), "2029-12-31"),
            Err(DocPackError::InvalidDate)
        );
        assert_eq!(
            catalog.delete("build-tool-guide", Some(V1), "2030-01-01"),
            Err(DocPackError::NotFound)
        );
        let deleted = catalog
            .delete("build-tool-guide", None, "2030-01-02")
            .unwrap();
        assert_eq!(deleted[0].reason, DocPackDeletionReason::Person);
        assert_eq!(catalog.total_bytes(), 0);
        assert_eq!(
            catalog.index("build-tool-guide"),
            Err(DocPackError::NotFound)
        );
        let records = serde_json::to_string(catalog.deletions()).unwrap();
        assert_eq!(catalog.deletions().len(), 2);
        assert!(!records.contains("compiled units") && !records.contains("guide/cache.md"));
        // Once no version is kept, the pack may start again.
        assert!(
            catalog
                .import(
                    &policy(),
                    &first,
                    &first_contents,
                    DocPackImportIntent::New,
                    "2030-01-02"
                )
                .is_ok()
        );
    }

    #[test]
    fn indexing_bounds_fragments_withholds_secrets_and_keeps_history_out_of_current_searches() {
        let mut catalog = DocPackCatalog::new();
        let (first, first_contents) = guide(V1, "2026-01-01");
        let receipt = catalog
            .import(
                &policy(),
                &first,
                &first_contents,
                DocPackImportIntent::New,
                "2026-01-01",
            )
            .unwrap();
        assert_eq!(
            (receipt.fragment_count, receipt.withheld_fragment_count),
            (4, 1)
        );
        let index = catalog.index("build-tool-guide").unwrap();
        let cache = &index.documents[0];
        assert_eq!(
            cache
                .fragments
                .iter()
                .map(|fragment| (fragment.kind, fragment.source_range, fragment.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    KnowledgeSourceFragmentKind::Heading,
                    ObsidianSourceRange {
                        start_line: 1,
                        start_column: 1,
                        end_line: 1,
                        end_column: 14
                    },
                    "# Build cache",
                ),
                (
                    KnowledgeSourceFragmentKind::Body,
                    ObsidianSourceRange {
                        start_line: 3,
                        start_column: 1,
                        end_line: 4,
                        end_column: 30
                    },
                    "The cache keeps compiled units.\nClear it with the clean step.",
                ),
                (
                    KnowledgeSourceFragmentKind::Heading,
                    ObsidianSourceRange {
                        start_line: 6,
                        start_column: 1,
                        end_line: 6,
                        end_column: 15
                    },
                    "## Credentials",
                ),
            ]
        );
        assert!(!format!("{index:?}").contains(CANARY));
        assert_eq!(
            (cache.file_type, cache.historical),
            (KnowledgeFileType::Markdown, false)
        );
        assert_eq!(index.documents[1].file_type, KnowledgeFileType::Text);
        // A long line is cut at character boundaries within the bound.
        let long = "\u{e9}".repeat(MAX_FRAGMENT_BYTES);
        let pieces = split_fragments(&long, false);
        assert_eq!(pieces.len(), 2);
        assert!(
            pieces
                .iter()
                .all(|piece| piece.text.len() <= MAX_FRAGMENT_BYTES)
        );
        assert_eq!(
            pieces[1].source_range.start_column,
            pieces[0].source_range.end_column
        );
        // A paragraph longer than the bound is cut at a line end.
        let paragraph = vec!["x".repeat(49); 100].join("\n");
        let cut = split_fragments(&paragraph, false);
        assert_eq!(cut.len(), 2);
        assert!(cut[0].text.len() <= MAX_FRAGMENT_BYTES && !cut[0].text.ends_with('\n'));
        assert_eq!(
            cut[1].source_range.start_line,
            cut[0].source_range.end_line + 1
        );
        // Searches reach the current version; a superseded one only as history.
        let (second, second_contents) = pack(
            V2,
            "2026-02-01",
            &[(
                "guide/cache.md",
                "# Build cache\n\nThe cache now lives per workspace.\n",
            )],
        );
        catalog
            .import(
                &policy(),
                &second,
                &second_contents,
                DocPackImportIntent::Refresh { replaces: V1 },
                "2026-02-01",
            )
            .unwrap();
        let index = catalog.index("build-tool-guide").unwrap();
        let query = |include_historical| KnowledgeContextQuery {
            terms: vec!["cache".to_owned()],
            phrases: Vec::new(),
            roots: index
                .documents
                .iter()
                .map(|document| document.root.clone())
                .collect(),
            date_from: None,
            date_to: None,
            as_of_date: "2026-02-01".to_owned(),
            file_types: BTreeSet::from([KnowledgeFileType::Markdown, KnowledgeFileType::Text]),
            authorities: BTreeSet::from([KnowledgeSourceAuthority::DirectEvidence]),
            include_historical,
            max_results: 20,
            max_context_bytes: 64 * 1024,
        };
        let current = retrieve_knowledge(&query(false), &index.documents).unwrap();
        assert!(!current.hits.is_empty());
        assert!(
            current
                .hits
                .iter()
                .all(|hit| hit.path.workspace_id().as_str() == "doc-pack:build-tool-guide:1.1.0")
        );
        let history = retrieve_knowledge(&query(true), &index.documents).unwrap();
        assert!(
            history
                .hits
                .iter()
                .any(|hit| hit.path.workspace_id().as_str() == "doc-pack:build-tool-guide:1.0.0")
        );
        assert!(!history.semantic_components_used);
    }
}
