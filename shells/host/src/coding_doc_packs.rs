//! Documentation packs through the development catalog host (Decision 0130).
//!
//! The catalog host owns one documentation catalog in its operational store.
//! For each operation it loads the stored catalog, restores and re-verifies
//! it through the knowledge component, applies retention for the host's
//! current day, performs the operation and commits the next state under the
//! revision it read. An import arrives in parts over the authenticated IPC
//! session: the sealed manifest, then each listed file's text in bounded
//! chunks, then a commit that checks every file and imports the version. One
//! import is staged at a time. Every answer is closed and content-free except
//! the cited text a search returns from documentation the person imported.
//! Nothing here opens a network connection or grants authority.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_capability_knowledge::{
    DocPackCatalog, DocPackCatalogState, DocPackDeletion, DocPackDeletionReason, DocPackError,
    DocPackFreshness, DocPackImportIntent, DocPackKeptState, DocPackManifest, DocPackPolicy,
    DocPackReceipt, DocPackSearchQuery, DocPackStatus, DocPackVersion, DocPackVersionState,
    parse_doc_pack_manifest,
};
use agentmage_kernel_engine::doc_pack_store::{
    DocPackCatalogContents, DocPackStoreError, DocPackVersionKey, DurableDocPackCatalog,
    MAX_DOC_PACK_DELETIONS, StoredDocPackDeletion, StoredDocPackDeletionReason, StoredDocPackFile,
    StoredDocPackVersion,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

/// Days a superseded version is kept before retention deletes it.
pub const DOC_PACK_RETENTION_DAYS: u32 = 30;
/// Most text one import chunk carries. The client reads only documentation
/// text, which has no control character other than a line feed, carriage
/// return or tab, and JSON escaping at most doubles such text, so a chunk the
/// client sends always fits one IPC frame (Decision 0132).
pub const MAX_DOC_PACK_CHUNK_BYTES: usize = 1024 * 1024;
/// Most licenses one import may allow.
pub const MAX_DOC_PACK_ALLOWED_LICENSES: usize = 8;
/// Most hits one search returns.
pub const MAX_DOC_PACK_SEARCH_HITS: u32 = 20;
const MAX_SEARCH_TERMS: usize = 16;
const MAX_SEARCH_TERM_BYTES: usize = 128;

/// An optional member that must still be present, as `null` when absent.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// One documentation pack request of a catalog client. Every member is
/// required and no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DocPackRequest {
    /// Stages an import of one sealed manifest version.
    ImportBegin {
        /// The sealed manifest.
        manifest: DocPackManifest,
        /// Licenses the person allows for this import.
        allowed_licenses: Vec<String>,
        /// The current version this import refreshes, if any.
        #[serde(deserialize_with = "required_option")]
        refresh: Option<DocPackVersion>,
    },
    /// Appends text to one listed file of the staged import.
    ImportChunk {
        /// File path as the manifest lists it.
        path: String,
        /// Byte offset this chunk starts at.
        offset: u64,
        /// The chunk's text.
        text: String,
    },
    /// Checks every staged file and imports the version.
    ImportCommit {},
    /// Inspects every kept pack.
    List {},
    /// Inspects one pack.
    Inspect {
        /// Pack identity.
        pack_id: String,
    },
    /// Deletes one version, or every version of one pack.
    Delete {
        /// Pack identity.
        pack_id: String,
        /// The version, or every version.
        #[serde(deserialize_with = "required_option")]
        version: Option<DocPackVersion>,
    },
    /// Searches the kept packs.
    Search {
        /// Case-insensitive terms, all required.
        terms: Vec<String>,
        /// One pack, or every pack.
        #[serde(deserialize_with = "required_option")]
        pack_id: Option<String>,
        /// Whether superseded versions may answer.
        include_history: bool,
    },
}

/// Why a version was deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocPackDeletionReasonView {
    /// The person deleted it.
    Person,
    /// Its retention after supersession ended.
    Retention,
}

/// One content-free deletion record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackDeletionView {
    /// Pack identity.
    pub pack_id: String,
    /// Deleted version.
    pub version: DocPackVersion,
    /// Its manifest digest.
    pub manifest_sha256: String,
    /// ISO date of deletion.
    pub deleted_on: String,
    /// Why.
    pub reason: DocPackDeletionReasonView,
}

/// The import receipt, field for field as the knowledge component seals it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackImportReceiptView {
    /// Pack identity.
    pub pack_id: String,
    /// Imported version.
    pub version: DocPackVersion,
    /// Its manifest digest.
    pub manifest_sha256: String,
    /// Its license.
    pub license: String,
    /// The version it superseded, if any.
    #[serde(deserialize_with = "required_option")]
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

/// One kept version on the inspected day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackVersionView {
    /// Version.
    pub version: DocPackVersion,
    /// Manifest digest.
    pub manifest_sha256: String,
    /// ISO date its content was obtained.
    pub retrieved_on: String,
    /// Whether searches return it as current.
    pub current: bool,
    /// ISO date it was superseded.
    #[serde(deserialize_with = "required_option")]
    pub superseded_on: Option<String>,
    /// ISO date on which retention deletes it.
    #[serde(deserialize_with = "required_option")]
    pub delete_on: Option<String>,
    /// Whether it is inside its freshness window.
    pub fresh: bool,
    /// Days left while fresh, or days past since it became stale.
    pub freshness_days: u32,
}

/// One kept pack on the inspected day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackStatusView {
    /// Pack identity.
    pub pack_id: String,
    /// Current version, if one is kept.
    #[serde(deserialize_with = "required_option")]
    pub current: Option<DocPackVersion>,
    /// Every kept version, oldest first.
    pub versions: Vec<DocPackVersionView>,
}

/// One cited fragment a search found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocPackHitView {
    /// Pack identity.
    pub pack_id: String,
    /// Version.
    pub version: DocPackVersion,
    /// Whether that version is superseded.
    pub historical: bool,
    /// File path inside the pack.
    pub path: String,
    /// First line.
    pub start_line: u32,
    /// Last line.
    pub end_line: u32,
    /// Whether the fragment is a heading.
    pub heading: bool,
    /// The bounded exact cited text.
    pub text: String,
    /// Citation digest.
    pub citation_sha256: String,
}

/// Content-free reason a documentation pack request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocPackRefusal {
    /// The manifest's shape, fields, paths, bounds or seal are invalid.
    ManifestInvalid,
    /// The import does not allow the manifest's license.
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
    /// The host's day is earlier than the catalog's last change.
    InvalidDate,
    /// The allowed licenses or the retention policy are out of bounds.
    PolicyInvalid,
    /// The search terms are empty, too many or too long.
    QueryInvalid,
    /// The stored catalog does not restore.
    StateInvalid,
    /// No import is staged.
    NotStaged,
    /// A chunk names no listed file, another offset or more than its file.
    ChunkInvalid,
    /// The host has no clock reading.
    ClockUnavailable,
    /// The store could not be opened, read or written.
    StoreUnavailable,
    /// The next catalog exceeds a store bound.
    StoreLimit,
    /// The catalog changed underneath this operation.
    StoreConflict,
    /// The stored catalog failed its integrity check.
    StoreIntegrity,
}

impl DocPackRefusal {
    /// Stable content-free code.
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
            Self::QueryInvalid => "doc-pack.query-invalid",
            Self::StateInvalid => "doc-pack.state-invalid",
            Self::NotStaged => "doc-pack.not-staged",
            Self::ChunkInvalid => "doc-pack.chunk-invalid",
            Self::ClockUnavailable => "doc-pack.clock-unavailable",
            Self::StoreUnavailable => "doc-pack.store-unavailable",
            Self::StoreLimit => "doc-pack.store-limit",
            Self::StoreConflict => "doc-pack.store-conflict",
            Self::StoreIntegrity => "doc-pack.store-integrity",
        }
    }

    const fn of(error: DocPackError) -> Self {
        match error {
            DocPackError::ManifestInvalid => Self::ManifestInvalid,
            DocPackError::LicenseNotAllowed => Self::LicenseNotAllowed,
            DocPackError::ContentMismatch => Self::ContentMismatch,
            DocPackError::ContentInvalid => Self::ContentInvalid,
            DocPackError::IntentMismatch => Self::IntentMismatch,
            DocPackError::NotNewer => Self::NotNewer,
            DocPackError::AlreadyImported => Self::AlreadyImported,
            DocPackError::Conflict => Self::Conflict,
            DocPackError::NotFound => Self::NotFound,
            DocPackError::ResourceLimit => Self::ResourceLimit,
            DocPackError::InvalidDate => Self::InvalidDate,
            DocPackError::PolicyInvalid => Self::PolicyInvalid,
            DocPackError::QueryInvalid => Self::QueryInvalid,
            DocPackError::StateInvalid => Self::StateInvalid,
        }
    }

    const fn of_store(error: DocPackStoreError) -> Self {
        match error {
            DocPackStoreError::ResourceLimit => Self::StoreLimit,
            DocPackStoreError::Stale => Self::StoreConflict,
            DocPackStoreError::Integrity => Self::StoreIntegrity,
            DocPackStoreError::InvalidInput
            | DocPackStoreError::InvalidChange
            | DocPackStoreError::Storage
            | DocPackStoreError::Unavailable => Self::StoreUnavailable,
        }
    }
}

/// One documentation pack answer of the catalog host. Every member is
/// required and no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DocPackAnswer {
    /// The manifest verified and the import is staged.
    ImportStaged {
        /// Pack identity.
        pack_id: String,
        /// Version.
        version: DocPackVersion,
        /// Manifest digest.
        manifest_sha256: String,
    },
    /// A chunk was appended.
    ChunkAccepted {
        /// File path.
        path: String,
        /// Bytes of that file received so far.
        received_bytes: u64,
    },
    /// The version was imported.
    Imported {
        /// Its sealed receipt.
        receipt: DocPackImportReceiptView,
        /// Versions retention deleted first.
        retention: Vec<DocPackDeletionView>,
    },
    /// Every kept pack.
    Listed {
        /// Each pack on the host's day.
        packs: Vec<DocPackStatusView>,
        /// Versions retention deleted first.
        retention: Vec<DocPackDeletionView>,
    },
    /// One kept pack.
    Inspected {
        /// The pack on the host's day.
        status: DocPackStatusView,
        /// Versions retention deleted first.
        retention: Vec<DocPackDeletionView>,
    },
    /// The person's deletion.
    Deleted {
        /// Every version deleted.
        deletions: Vec<DocPackDeletionView>,
        /// Versions retention deleted first.
        retention: Vec<DocPackDeletionView>,
    },
    /// What a search found.
    Found {
        /// Ranked hits.
        hits: Vec<DocPackHitView>,
        /// Matching fragments not shown.
        omitted: u64,
        /// Versions retention deleted first.
        retention: Vec<DocPackDeletionView>,
    },
    /// The request was refused and changed nothing.
    Refused {
        /// Why.
        refusal: DocPackRefusal,
    },
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// The ISO date in UTC of a Unix epoch time in milliseconds.
#[must_use]
pub fn iso_date_of_epoch_ms(epoch_ms: u64) -> Option<String> {
    let days = i64::try_from(epoch_ms / 86_400_000).ok()?;
    // Civil date of a day count since 1970-01-01 (proleptic Gregorian).
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (1..=9_999)
        .contains(&year)
        .then(|| format!("{year:04}-{month:02}-{day:02}"))
}

fn version_text(version: DocPackVersion) -> String {
    format!("{}.{}.{}", version.major, version.minor, version.patch)
}

/// Parses `MAJOR.MINOR.PATCH` with plain decimal parts.
#[must_use]
pub fn parse_doc_pack_version(value: &str) -> Option<DocPackVersion> {
    let mut parts = value.split('.');
    let mut part = || {
        parts
            .next()
            .filter(|part| {
                !part.is_empty()
                    && part.len() <= 10
                    && part.bytes().all(|byte| byte.is_ascii_digit())
                    && (part.len() == 1 || !part.starts_with('0'))
            })
            .and_then(|part| part.parse::<u32>().ok())
    };
    let version = DocPackVersion {
        major: part()?,
        minor: part()?,
        patch: part()?,
    };
    parts.next().is_none().then_some(version)
}

const fn deletion_reason(reason: DocPackDeletionReason) -> DocPackDeletionReasonView {
    match reason {
        DocPackDeletionReason::Person => DocPackDeletionReasonView::Person,
        DocPackDeletionReason::Retention => DocPackDeletionReasonView::Retention,
    }
}

fn deletion_view(deletion: &DocPackDeletion) -> DocPackDeletionView {
    DocPackDeletionView {
        pack_id: deletion.pack_id.clone(),
        version: deletion.version,
        manifest_sha256: deletion.manifest_sha256.clone(),
        deleted_on: deletion.deleted_on.clone(),
        reason: deletion_reason(deletion.reason),
    }
}

fn receipt_view(receipt: &DocPackReceipt) -> DocPackImportReceiptView {
    DocPackImportReceiptView {
        pack_id: receipt.pack_id.clone(),
        version: receipt.version,
        manifest_sha256: receipt.manifest_sha256.clone(),
        license: receipt.license.clone(),
        superseded: receipt.superseded,
        file_count: receipt.file_count,
        total_bytes: receipt.total_bytes,
        fragment_count: receipt.fragment_count,
        withheld_fragment_count: receipt.withheld_fragment_count,
        network_used: receipt.network_used,
        receipt_sha256: receipt.receipt_sha256.clone(),
    }
}

fn status_view(status: &DocPackStatus) -> DocPackStatusView {
    DocPackStatusView {
        pack_id: status.pack_id.clone(),
        current: status.current,
        versions: status
            .versions
            .iter()
            .map(|version| {
                let (superseded_on, delete_on) = match &version.state {
                    DocPackVersionState::Current => (None, None),
                    DocPackVersionState::Superseded {
                        superseded_on,
                        delete_on,
                    } => (Some(superseded_on.clone()), Some(delete_on.clone())),
                };
                let (fresh, freshness_days) = match version.freshness {
                    DocPackFreshness::Fresh { days_left } => (true, days_left),
                    DocPackFreshness::Stale { days_past } => (false, days_past),
                };
                DocPackVersionView {
                    version: version.version,
                    manifest_sha256: version.manifest_sha256.clone(),
                    retrieved_on: version.retrieved_on.clone(),
                    current: superseded_on.is_none(),
                    superseded_on,
                    delete_on,
                    fresh,
                    freshness_days,
                }
            })
            .collect(),
    }
}

/// The digest a receipt must carry: its encoding with the digest empty, as
/// the knowledge component seals it.
#[must_use]
pub fn doc_pack_receipt_sha256(receipt: &DocPackImportReceiptView) -> Option<String> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    serde_json::to_vec(&unsigned)
        .ok()
        .map(|bytes| sha256_hex(&bytes))
}

/// Keeps an import receipt only when it names exactly the manifest the client
/// sent, recomputes and used no network.
#[must_use]
pub fn verify_doc_pack_receipt(receipt: &DocPackImportReceiptView, sent: &DocPackManifest) -> bool {
    receipt.pack_id == sent.pack_id
        && receipt.version == sent.version
        && receipt.manifest_sha256 == sent.manifest_sha256
        && receipt.license == sent.license
        && receipt.file_count == sent.files.len() as u64
        && receipt.total_bytes == sent.total_bytes
        && !receipt.network_used
        && doc_pack_receipt_sha256(receipt).as_deref() == Some(receipt.receipt_sha256.as_str())
}

/// The catalog's knowledge state as the store keeps it.
fn store_contents(state: DocPackCatalogState) -> Result<DocPackCatalogContents, DocPackRefusal> {
    let versions = state
        .versions
        .into_iter()
        .map(|kept| {
            let manifest_json =
                serde_json::to_vec(&kept.manifest).map_err(|_| DocPackRefusal::StoreUnavailable)?;
            Ok(StoredDocPackVersion {
                key: DocPackVersionKey {
                    pack_id: kept.manifest.pack_id.clone(),
                    major: kept.manifest.version.major,
                    minor: kept.manifest.version.minor,
                    patch: kept.manifest.version.patch,
                },
                manifest_sha256: kept.manifest.manifest_sha256.clone(),
                manifest_json,
                superseded_on: kept.superseded_on,
                files: kept
                    .contents
                    .into_iter()
                    .map(|(path, content)| StoredDocPackFile {
                        path,
                        sha256: sha256_hex(&content),
                        content,
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, DocPackRefusal>>()?;
    let deletions = state
        .deletions
        .iter()
        .map(|deletion| StoredDocPackDeletion {
            key: DocPackVersionKey {
                pack_id: deletion.pack_id.clone(),
                major: deletion.version.major,
                minor: deletion.version.minor,
                patch: deletion.version.patch,
            },
            manifest_sha256: deletion.manifest_sha256.clone(),
            deleted_on: deletion.deleted_on.clone(),
            reason: match deletion.reason {
                DocPackDeletionReason::Person => StoredDocPackDeletionReason::Person,
                DocPackDeletionReason::Retention => StoredDocPackDeletionReason::Retention,
            },
        })
        .collect();
    Ok(DocPackCatalogContents {
        versions,
        deletions,
        last_changed_on: state.last_changed_on,
    })
}

/// The knowledge state of what the store keeps. Each stored manifest must
/// parse closed, verify its seal and name its row's pack, version and digest.
fn knowledge_state(
    contents: DocPackCatalogContents,
) -> Result<DocPackCatalogState, DocPackRefusal> {
    let versions = contents
        .versions
        .into_iter()
        .map(|stored| {
            let manifest = parse_doc_pack_manifest(&stored.manifest_json)
                .map_err(|_| DocPackRefusal::StoreIntegrity)?;
            if manifest.pack_id != stored.key.pack_id
                || manifest.version
                    != (DocPackVersion {
                        major: stored.key.major,
                        minor: stored.key.minor,
                        patch: stored.key.patch,
                    })
                || manifest.manifest_sha256 != stored.manifest_sha256
            {
                return Err(DocPackRefusal::StoreIntegrity);
            }
            Ok(DocPackKeptState {
                manifest,
                contents: stored
                    .files
                    .into_iter()
                    .map(|file| (file.path, file.content))
                    .collect(),
                superseded_on: stored.superseded_on,
            })
        })
        .collect::<Result<Vec<_>, DocPackRefusal>>()?;
    let deletions = contents
        .deletions
        .into_iter()
        .map(|deletion| DocPackDeletion {
            pack_id: deletion.key.pack_id,
            version: DocPackVersion {
                major: deletion.key.major,
                minor: deletion.key.minor,
                patch: deletion.key.patch,
            },
            manifest_sha256: deletion.manifest_sha256,
            deleted_on: deletion.deleted_on,
            reason: match deletion.reason {
                StoredDocPackDeletionReason::Person => DocPackDeletionReason::Person,
                StoredDocPackDeletionReason::Retention => DocPackDeletionReason::Retention,
            },
        })
        .collect();
    Ok(DocPackCatalogState {
        versions,
        deletions,
        last_changed_on: contents.last_changed_on,
    })
}

fn policy(allowed_licenses: BTreeSet<String>) -> DocPackPolicy {
    DocPackPolicy {
        allowed_licenses,
        superseded_retention_days: DOC_PACK_RETENTION_DAYS,
    }
}

struct StagedImport {
    manifest: DocPackManifest,
    allowed_licenses: BTreeSet<String>,
    intent: DocPackImportIntent,
    received: BTreeMap<String, Vec<u8>>,
}

/// The catalog host's documentation pack owner for one session.
pub struct DocPackOwner {
    staged: Option<StagedImport>,
    /// Most deletion records and kept versions together after an import.
    /// Retention and deletion turn a kept version into one deletion record,
    /// so an import inside this bound leaves room for every later deletion
    /// and reads never wait on a commit the store refuses (Decision 0132).
    history_limit: usize,
}

impl Default for DocPackOwner {
    fn default() -> Self {
        Self {
            staged: None,
            history_limit: MAX_DOC_PACK_DELETIONS,
        }
    }
}

impl std::fmt::Debug for DocPackOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DocPackOwner")
            .field("staged", &self.staged.is_some())
            .finish()
    }
}

impl DocPackOwner {
    /// Answers one request. `open` opens the store for an operation that
    /// needs it; `today` is the host's ISO date, if it has one.
    pub fn answer(
        &mut self,
        request: DocPackRequest,
        open: &mut dyn FnMut() -> Result<DurableDocPackCatalog, DocPackRefusal>,
        today: Option<&str>,
    ) -> DocPackAnswer {
        match self.decide(request, open, today) {
            Ok(answer) => answer,
            Err(refusal) => DocPackAnswer::Refused { refusal },
        }
    }

    fn decide(
        &mut self,
        request: DocPackRequest,
        open: &mut dyn FnMut() -> Result<DurableDocPackCatalog, DocPackRefusal>,
        today: Option<&str>,
    ) -> Result<DocPackAnswer, DocPackRefusal> {
        match request {
            DocPackRequest::ImportBegin {
                manifest,
                allowed_licenses,
                refresh,
            } => self.begin(manifest, allowed_licenses, refresh),
            DocPackRequest::ImportChunk { path, offset, text } => self.chunk(&path, offset, &text),
            DocPackRequest::ImportCommit {} => {
                let staged = self.staged.take().ok_or(DocPackRefusal::NotStaged)?;
                let today = today.ok_or(DocPackRefusal::ClockUnavailable)?;
                let history_limit = self.history_limit;
                change_catalog(open, today, |catalog, retention| {
                    if catalog.deletions().len() + catalog.kept_version_count() >= history_limit {
                        return Err(DocPackRefusal::StoreLimit);
                    }
                    let receipt = catalog
                        .import(
                            &policy(staged.allowed_licenses.clone()),
                            &staged.manifest,
                            &staged.received,
                            staged.intent,
                            today,
                        )
                        .map_err(DocPackRefusal::of)?;
                    let answer = DocPackAnswer::Imported {
                        receipt: receipt_view(&receipt),
                        retention,
                    };
                    Ok((answer, true))
                })
            }
            DocPackRequest::List {} => {
                let today = today.ok_or(DocPackRefusal::ClockUnavailable)?;
                change_catalog(open, today, |catalog, retention| {
                    let packs = catalog
                        .pack_ids()
                        .iter()
                        .map(|pack_id| {
                            catalog
                                .inspect(&policy(BTreeSet::new()), pack_id, today)
                                .map(|status| status_view(&status))
                                .map_err(DocPackRefusal::of)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok((DocPackAnswer::Listed { packs, retention }, false))
                })
            }
            DocPackRequest::Inspect { pack_id } => {
                let today = today.ok_or(DocPackRefusal::ClockUnavailable)?;
                change_catalog(open, today, |catalog, retention| {
                    let status = catalog
                        .inspect(&policy(BTreeSet::new()), &pack_id, today)
                        .map_err(DocPackRefusal::of)?;
                    let answer = DocPackAnswer::Inspected {
                        status: status_view(&status),
                        retention,
                    };
                    Ok((answer, false))
                })
            }
            DocPackRequest::Delete { pack_id, version } => {
                let today = today.ok_or(DocPackRefusal::ClockUnavailable)?;
                change_catalog(open, today, |catalog, retention| {
                    let deletions = catalog
                        .delete(&pack_id, version, today)
                        .map_err(DocPackRefusal::of)?;
                    let answer = DocPackAnswer::Deleted {
                        deletions: deletions.iter().map(deletion_view).collect(),
                        retention,
                    };
                    Ok((answer, true))
                })
            }
            DocPackRequest::Search {
                terms,
                pack_id,
                include_history,
            } => {
                if terms.is_empty()
                    || terms.len() > MAX_SEARCH_TERMS
                    || terms.iter().any(|term| {
                        term.is_empty()
                            || term.len() > MAX_SEARCH_TERM_BYTES
                            || term.chars().any(char::is_whitespace)
                    })
                {
                    return Err(DocPackRefusal::QueryInvalid);
                }
                let today = today.ok_or(DocPackRefusal::ClockUnavailable)?;
                change_catalog(open, today, |catalog, retention| {
                    let found = catalog
                        .search(
                            &DocPackSearchQuery {
                                terms: terms.clone(),
                                pack_id: pack_id.clone(),
                                include_history,
                                max_results: MAX_DOC_PACK_SEARCH_HITS,
                            },
                            today,
                        )
                        .map_err(DocPackRefusal::of)?;
                    let answer = DocPackAnswer::Found {
                        hits: found
                            .hits
                            .into_iter()
                            .map(|hit| DocPackHitView {
                                pack_id: hit.pack_id,
                                version: hit.version,
                                historical: hit.historical,
                                path: hit.path,
                                start_line: hit.start_line,
                                end_line: hit.end_line,
                                heading: hit.heading,
                                text: hit.text,
                                citation_sha256: hit.citation_sha256,
                            })
                            .collect(),
                        omitted: found.omitted_count,
                        retention,
                    };
                    Ok((answer, false))
                })
            }
        }
    }

    /// Stages an import after verifying the manifest's seal and that the
    /// person allows its license. Any earlier staged import is discarded.
    fn begin(
        &mut self,
        manifest: DocPackManifest,
        allowed_licenses: Vec<String>,
        refresh: Option<DocPackVersion>,
    ) -> Result<DocPackAnswer, DocPackRefusal> {
        self.staged = None;
        let bytes = serde_json::to_vec(&manifest).map_err(|_| DocPackRefusal::ManifestInvalid)?;
        if parse_doc_pack_manifest(&bytes).ok().as_ref() != Some(&manifest) {
            return Err(DocPackRefusal::ManifestInvalid);
        }
        let allowed = allowed_licenses.iter().cloned().collect::<BTreeSet<_>>();
        if allowed.is_empty()
            || allowed_licenses.len() > MAX_DOC_PACK_ALLOWED_LICENSES
            || allowed.len() != allowed_licenses.len()
        {
            return Err(DocPackRefusal::PolicyInvalid);
        }
        if !allowed.contains(&manifest.license) {
            return Err(DocPackRefusal::LicenseNotAllowed);
        }
        let answer = DocPackAnswer::ImportStaged {
            pack_id: manifest.pack_id.clone(),
            version: manifest.version,
            manifest_sha256: manifest.manifest_sha256.clone(),
        };
        self.staged = Some(StagedImport {
            received: manifest
                .files
                .iter()
                .map(|file| (file.path.clone(), Vec::new()))
                .collect(),
            manifest,
            allowed_licenses: allowed,
            intent: refresh.map_or(DocPackImportIntent::New, |replaces| {
                DocPackImportIntent::Refresh { replaces }
            }),
        });
        Ok(answer)
    }

    /// Appends one chunk at the exact offset of a listed file, never past its
    /// length. A refused chunk discards the staged import.
    fn chunk(
        &mut self,
        path: &str,
        offset: u64,
        text: &str,
    ) -> Result<DocPackAnswer, DocPackRefusal> {
        let mut staged = self.staged.take().ok_or(DocPackRefusal::NotStaged)?;
        let length = staged
            .manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.byte_len)
            .ok_or(DocPackRefusal::ChunkInvalid)?;
        let received = staged
            .received
            .get_mut(path)
            .ok_or(DocPackRefusal::ChunkInvalid)?;
        let next = (received.len() as u64).checked_add(text.len() as u64);
        if text.is_empty()
            || text.len() > MAX_DOC_PACK_CHUNK_BYTES
            || offset != received.len() as u64
            || next.is_none_or(|next| next > length)
        {
            return Err(DocPackRefusal::ChunkInvalid);
        }
        received.extend_from_slice(text.as_bytes());
        let received_bytes = received.len() as u64;
        self.staged = Some(staged);
        Ok(DocPackAnswer::ChunkAccepted {
            path: path.to_owned(),
            received_bytes,
        })
    }
}

/// Restores the stored catalog, applies retention for `today`, runs one
/// operation and commits the next state under the revision it read. The
/// operation says whether it changed the catalog; nothing is committed when
/// neither it nor retention did, so a refused operation changes nothing.
fn change_catalog(
    open: &mut dyn FnMut() -> Result<DurableDocPackCatalog, DocPackRefusal>,
    today: &str,
    operation: impl FnOnce(
        &mut DocPackCatalog,
        Vec<DocPackDeletionView>,
    ) -> Result<(DocPackAnswer, bool), DocPackRefusal>,
) -> Result<DocPackAnswer, DocPackRefusal> {
    let store = open()?;
    let stored = store.load().map_err(DocPackRefusal::of_store)?;
    let revision = stored.revision;
    let mut catalog =
        DocPackCatalog::restore(knowledge_state(stored.contents)?).map_err(DocPackRefusal::of)?;
    let retention = catalog
        .apply_retention(&policy(BTreeSet::new()), today)
        .map_err(DocPackRefusal::of)?;
    let retained = !retention.is_empty();
    let (answer, changed) = operation(&mut catalog, retention.iter().map(deletion_view).collect())?;
    if changed || retained {
        let next = store_contents(catalog.state().map_err(DocPackRefusal::of)?)?;
        store
            .commit(revision, &next)
            .map_err(DocPackRefusal::of_store)?;
    }
    Ok(answer)
}

/// Text of a cited fragment for a terminal: each line escaped, so no control,
/// format or line separator character reaches the terminal.
fn escaped_lines(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split('\n').map(|line| line.escape_debug().to_string())
}

fn render_deletion(deletion: &DocPackDeletionView, json: bool) -> String {
    if json {
        format!(
            "{}\n",
            serde_json::json!({
                "type": "doc_pack_deletion",
                "deletion": deletion,
            })
        )
    } else {
        format!(
            "doc pack deleted: {} {} ({}) on {}; manifest {}\n",
            deletion.pack_id,
            version_text(deletion.version),
            match deletion.reason {
                DocPackDeletionReasonView::Person => "person",
                DocPackDeletionReasonView::Retention => "retention",
            },
            deletion.deleted_on,
            &deletion.manifest_sha256[..deletion.manifest_sha256.len().min(12)]
        )
    }
}

fn render_status(status: &DocPackStatusView, json: bool) -> String {
    if json {
        return format!(
            "{}\n",
            serde_json::json!({"type": "doc_pack_status", "status": status})
        );
    }
    let mut output = format!(
        "doc pack {}: current {}\n",
        status.pack_id,
        status
            .current
            .map_or_else(|| "none".to_owned(), version_text)
    );
    for version in &status.versions {
        let state = match (&version.superseded_on, &version.delete_on) {
            (Some(superseded_on), Some(delete_on)) => {
                format!("superseded on {superseded_on}, deleted on {delete_on}")
            }
            _ => "current".to_owned(),
        };
        let freshness = if version.fresh {
            format!("fresh, {} days left", version.freshness_days)
        } else {
            format!("stale for {} days", version.freshness_days)
        };
        let _ = writeln!(
            output,
            "  {} {state}; retrieved {}; {freshness}; manifest {}",
            version_text(version.version),
            version.retrieved_on,
            &version.manifest_sha256[..version.manifest_sha256.len().min(12)]
        );
    }
    output
}

/// Standard output for one answer, in text lines or JSON rows. A refusal is
/// written to standard error by the caller.
#[must_use]
pub fn render_doc_pack_answer(answer: &DocPackAnswer, json: bool) -> String {
    let mut output = String::new();
    let retention = match answer {
        DocPackAnswer::Imported { retention, .. }
        | DocPackAnswer::Listed { retention, .. }
        | DocPackAnswer::Inspected { retention, .. }
        | DocPackAnswer::Deleted { retention, .. }
        | DocPackAnswer::Found { retention, .. } => retention.as_slice(),
        _ => &[],
    };
    for deletion in retention {
        output.push_str(&render_deletion(deletion, json));
    }
    match answer {
        DocPackAnswer::Imported { receipt, .. } => {
            if json {
                let _ = writeln!(
                    output,
                    "{}",
                    serde_json::json!({"type": "doc_pack_import", "receipt": receipt})
                );
            } else {
                let _ = writeln!(
                    output,
                    "doc pack imported: {} {}; {} files, {} bytes, {} fragments, {} withheld; license {}; supersedes {}; no network; receipt {}",
                    receipt.pack_id,
                    version_text(receipt.version),
                    receipt.file_count,
                    receipt.total_bytes,
                    receipt.fragment_count,
                    receipt.withheld_fragment_count,
                    receipt.license,
                    receipt
                        .superseded
                        .map_or_else(|| "none".to_owned(), version_text),
                    &receipt.receipt_sha256[..receipt.receipt_sha256.len().min(12)]
                );
            }
        }
        DocPackAnswer::Listed { packs, .. } => {
            for status in packs {
                output.push_str(&render_status(status, json));
            }
            if json {
                let _ = writeln!(
                    output,
                    "{}",
                    serde_json::json!({"type": "doc_pack_list", "packs": packs.len()})
                );
            } else if packs.is_empty() {
                output.push_str("doc packs: none kept\n");
            }
        }
        DocPackAnswer::Inspected { status, .. } => output.push_str(&render_status(status, json)),
        DocPackAnswer::Deleted { deletions, .. } => {
            for deletion in deletions {
                output.push_str(&render_deletion(deletion, json));
            }
        }
        DocPackAnswer::Found { hits, omitted, .. } => {
            if json {
                for hit in hits {
                    let _ = writeln!(
                        output,
                        "{}",
                        serde_json::json!({"type": "doc_pack_hit", "hit": hit})
                    );
                }
                let _ = writeln!(
                    output,
                    "{}",
                    serde_json::json!({"type": "doc_pack_search", "hits": hits.len(), "omitted": omitted})
                );
            } else {
                for (index, hit) in hits.iter().enumerate() {
                    let _ = writeln!(
                        output,
                        "doc pack hit {}: {} {}{} {}:{}-{}; citation {}",
                        index + 1,
                        hit.pack_id,
                        version_text(hit.version),
                        if hit.historical { " (history)" } else { "" },
                        hit.path.escape_debug(),
                        hit.start_line,
                        hit.end_line,
                        &hit.citation_sha256[..hit.citation_sha256.len().min(12)]
                    );
                    for line in escaped_lines(&hit.text) {
                        let _ = writeln!(output, "  | {line}");
                    }
                }
                let _ = writeln!(
                    output,
                    "doc pack search: {} hits; {omitted} more not shown",
                    hits.len()
                );
            }
        }
        DocPackAnswer::ImportStaged { .. }
        | DocPackAnswer::ChunkAccepted { .. }
        | DocPackAnswer::Refused { .. } => {}
    }
    output
}

/// One refusal line for standard error.
#[must_use]
pub fn render_doc_pack_refusal(refusal: DocPackRefusal, json: bool) -> String {
    if json {
        format!(
            "{}\n",
            serde_json::json!({"type": "doc_pack_refused", "code": refusal.code()})
        )
    } else {
        format!("doc pack refused: {}\n", refusal.code())
    }
}

/// A documentation pack operation the development CLI asks the catalog host
/// for instead of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocPackCommand {
    /// Imports the pack in an absolute directory.
    Import {
        /// Directory holding `manifest.json` and the listed files.
        directory: std::path::PathBuf,
        /// Licenses the person allows for this import.
        allowed_licenses: Vec<String>,
        /// The current version the import refreshes, if any.
        refresh: Option<DocPackVersion>,
    },
    /// Inspects every kept pack.
    List,
    /// Inspects one pack.
    Inspect {
        /// Pack identity.
        pack_id: String,
    },
    /// Deletes one version, or every version of one pack.
    Delete {
        /// Pack identity.
        pack_id: String,
        /// The version, or every version.
        version: Option<DocPackVersion>,
    },
    /// Searches the kept packs.
    Search {
        /// Terms, all required.
        terms: Vec<String>,
        /// One pack, or every pack.
        pack_id: Option<String>,
        /// Whether superseded versions may answer.
        include_history: bool,
    },
}

/// Whether a value is a plain pack identity: a lowercase letter, then
/// lowercase letters, digits, dots and hyphens, at most 64 bytes.
#[must_use]
pub fn plain_doc_pack_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

/// Whether a value is a license identifier an import may allow.
#[must_use]
pub fn doc_pack_license_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

/// Search terms of one command-line value: whitespace-separated, at least
/// one, at most 16 of at most 128 bytes each.
#[must_use]
pub fn doc_pack_search_terms(value: &str) -> Option<Vec<String>> {
    let terms = value
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    (!terms.is_empty()
        && terms.len() <= MAX_SEARCH_TERMS
        && terms.iter().all(|term| term.len() <= MAX_SEARCH_TERM_BYTES))
    .then_some(terms)
}

/// The closed process exit class of a refusal.
#[must_use]
pub const fn doc_pack_refusal_exit(refusal: DocPackRefusal) -> crate::headless::ClientExitCode {
    use crate::headless::ClientExitCode;
    match refusal {
        DocPackRefusal::ManifestInvalid
        | DocPackRefusal::ContentMismatch
        | DocPackRefusal::ContentInvalid
        | DocPackRefusal::QueryInvalid
        | DocPackRefusal::ChunkInvalid
        | DocPackRefusal::NotStaged => ClientExitCode::InvalidInput,
        DocPackRefusal::LicenseNotAllowed
        | DocPackRefusal::IntentMismatch
        | DocPackRefusal::NotNewer
        | DocPackRefusal::AlreadyImported
        | DocPackRefusal::Conflict
        | DocPackRefusal::NotFound
        | DocPackRefusal::InvalidDate
        | DocPackRefusal::PolicyInvalid => ClientExitCode::PolicyDenied,
        DocPackRefusal::ResourceLimit | DocPackRefusal::StoreLimit => ClientExitCode::ResourceBound,
        DocPackRefusal::StateInvalid
        | DocPackRefusal::ClockUnavailable
        | DocPackRefusal::StoreUnavailable
        | DocPackRefusal::StoreConflict
        | DocPackRefusal::StoreIntegrity => ClientExitCode::ServiceUnavailable,
    }
}

/// A pack the client read from a directory: its manifest and every listed
/// file's text, keyed by path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocPackSource {
    /// The sealed manifest.
    pub manifest: DocPackManifest,
    /// Each listed file's text.
    pub files: BTreeMap<String, String>,
}

/// The requests that import `source`: the beginning, each file's text in
/// chunks cut at character boundaries, and the commit.
#[must_use]
pub fn doc_pack_import_requests(
    source: &DocPackSource,
    allowed_licenses: &[String],
    refresh: Option<DocPackVersion>,
) -> Vec<DocPackRequest> {
    let mut requests = vec![DocPackRequest::ImportBegin {
        manifest: source.manifest.clone(),
        allowed_licenses: allowed_licenses.to_vec(),
        refresh,
    }];
    for (path, text) in &source.files {
        let mut start = 0;
        while start < text.len() {
            let mut end = (start + MAX_DOC_PACK_CHUNK_BYTES).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            requests.push(DocPackRequest::ImportChunk {
                path: path.clone(),
                offset: start as u64,
                text: text[start..end].to_owned(),
            });
            start = end;
        }
    }
    requests.push(DocPackRequest::ImportCommit {});
    requests
}

#[cfg(target_os = "linux")]
mod source {
    use std::io::Read as _;
    use std::os::fd::OwnedFd;
    use std::path::Path;

    use rustix::fs::{Mode, OFlags, openat};

    use super::{DocPackSource, parse_doc_pack_manifest};

    const MANIFEST_NAME: &str = "manifest.json";
    const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

    fn open_at(
        directory: &OwnedFd,
        name: &str,
        flags: OFlags,
    ) -> Result<OwnedFd, super::DocPackRefusal> {
        openat(
            directory,
            name,
            flags | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| super::DocPackRefusal::ContentMismatch)
    }

    /// Reads exactly `length` bytes of one regular file, refusing links,
    /// other file types and a file of another length.
    fn read_exact(file: OwnedFd, length: u64) -> Result<Vec<u8>, super::DocPackRefusal> {
        let metadata =
            rustix::fs::fstat(&file).map_err(|_| super::DocPackRefusal::ContentMismatch)?;
        if rustix::fs::FileType::from_raw_mode(metadata.st_mode)
            != rustix::fs::FileType::RegularFile
            || u64::try_from(metadata.st_size).ok() != Some(length)
        {
            return Err(super::DocPackRefusal::ContentMismatch);
        }
        let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
        std::fs::File::from(file)
            .take(length + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| super::DocPackRefusal::ContentMismatch)?;
        if bytes.len() as u64 != length {
            return Err(super::DocPackRefusal::ContentMismatch);
        }
        Ok(bytes)
    }

    /// Reads a pack from an absolute directory: `manifest.json` and each
    /// listed file, opening every path component without following links.
    /// Every file must be text.
    pub fn read_doc_pack_source(directory: &Path) -> Result<DocPackSource, super::DocPackRefusal> {
        if !directory.is_absolute() {
            return Err(super::DocPackRefusal::ContentMismatch);
        }
        let root = rustix::fs::open(
            directory,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| super::DocPackRefusal::ContentMismatch)?;
        let manifest_file = open_at(&root, MANIFEST_NAME, OFlags::RDONLY | OFlags::NONBLOCK)?;
        let length = rustix::fs::fstat(&manifest_file)
            .ok()
            .filter(|metadata| {
                rustix::fs::FileType::from_raw_mode(metadata.st_mode)
                    == rustix::fs::FileType::RegularFile
            })
            .and_then(|metadata| u64::try_from(metadata.st_size).ok())
            .filter(|length| *length > 0 && *length <= MAX_MANIFEST_BYTES)
            .ok_or(super::DocPackRefusal::ManifestInvalid)?;
        let manifest = parse_doc_pack_manifest(&read_exact(manifest_file, length)?)
            .map_err(|_| super::DocPackRefusal::ManifestInvalid)?;
        let mut files = std::collections::BTreeMap::new();
        for file in &manifest.files {
            let mut components = file.path.split('/').collect::<Vec<_>>();
            let name = components
                .pop()
                .ok_or(super::DocPackRefusal::ManifestInvalid)?;
            let mut parent = root
                .try_clone()
                .map_err(|_| super::DocPackRefusal::ContentMismatch)?;
            for component in components {
                parent = open_at(&parent, component, OFlags::RDONLY | OFlags::DIRECTORY)?;
            }
            // A FIFO or device opens without blocking and is then refused as
            // not a regular file.
            let bytes = read_exact(
                open_at(&parent, name, OFlags::RDONLY | OFlags::NONBLOCK)?,
                file.byte_len,
            )?;
            // Text the host would refuse at commit is refused here, before
            // any host is launched (Decision 0132).
            let text = String::from_utf8(bytes)
                .ok()
                .filter(|text| agentmage_capability_knowledge::doc_pack_text_allowed(text))
                .ok_or(super::DocPackRefusal::ContentInvalid)?;
            files.insert(file.path.clone(), text);
        }
        Ok(DocPackSource { manifest, files })
    }
}

#[cfg(target_os = "linux")]
pub use source::read_doc_pack_source;

#[cfg(all(
    test,
    target_os = "linux",
    feature = "source-artifacts",
    feature = "workflow-supervisor"
))]
#[path = "coding_doc_packs_tests.rs"]
mod tests;
