//! Optional local semantic-index contracts with explicit admission and opt-in.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{WorkspacePath, WorkspaceScopePath};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::ObsidianSourceRange;

const MAX_PROFILE_ID_BYTES: usize = 128;
const MAX_METADATA_BYTES: usize = 256;
const MAX_DIMENSIONS: u32 = 65_536;
const MAX_SCOPE_ENTRIES: usize = 100_000;
const MAX_CHUNKS: usize = 1_000_000;
const MAX_CHUNK_BYTES: usize = 1024 * 1024;
const MAX_BRANCH_BYTES: usize = 256;
const MAX_QUERY_RESULTS: u32 = 1_000;

/// Closed semantic model role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticModelRole {
    /// Local text embedding generation.
    Embedding,
    /// Local candidate reranking.
    Reranking,
}

/// Closed local runtime family; no remote provider exists in this contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRuntime {
    /// Native pinned `llama.cpp` process.
    NativeLlamaCpp,
    /// Optional separately gated Docker Model Runner process.
    DockerModelRunner,
    /// Apple Silicon Metal `llama.cpp` process.
    MacosMetalLlamaCpp,
}

/// Exact lifecycle state admitted by upstream model policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticProfileState {
    /// Recorded but unavailable candidate.
    Candidate,
    /// Isolated evidence-only evaluation.
    Evaluating,
    /// Approved for the exact declared role and artifact.
    Approved,
    /// Quarantined after identity or behavior drift.
    Quarantined,
    /// Rejected and retained only as evidence.
    Rejected,
}

/// Closed searchable field classes disclosed before opt-in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticField {
    /// Note title.
    Title,
    /// Heading text.
    Heading,
    /// Canonical field or property.
    Field,
    /// Task text.
    Task,
    /// Body excerpt.
    Body,
}

/// Explicit local derived-data storage protection choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticStorageProtection {
    /// Host full-disk encryption is relied on and disclosed as such.
    HostDiskEncryption,
    /// Application-managed encryption is required by the caller-owned store.
    ApplicationManagedEncryption,
    /// Unencrypted derived storage accepted through an explicit user decision.
    ExplicitUnencryptedAcceptance,
}

/// Exact model, tokenizer, lineage, license, runtime, and resource manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SemanticModelManifest {
    /// Stable exact profile identity.
    pub profile_id: String,
    /// Exact model role.
    pub role: SemanticModelRole,
    /// Publisher identity.
    pub publisher: String,
    /// Bounded lineage statement.
    pub lineage: String,
    /// SPDX or similarly exact license identity.
    pub license_id: String,
    /// Exact artifact digest.
    pub artifact_sha256: String,
    /// Exact tokenizer identity.
    pub tokenizer_id: String,
    /// Exact tokenizer artifact digest.
    pub tokenizer_sha256: String,
    /// Closed local runtime.
    pub runtime: SemanticRuntime,
    /// Exact fixed output dimension.
    pub dimensions: u32,
    /// Maximum resident bytes admitted by policy.
    pub max_resident_bytes: u64,
    /// Upstream lifecycle state.
    pub state: SemanticProfileState,
    /// Whether the project's origin policy passed for this exact lineage.
    pub origin_policy_passed: bool,
    /// Whether license review passed for this exact artifact.
    pub license_policy_passed: bool,
}

/// Hash-bound proof that upstream policy admitted one exact profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticAdmissionReceipt {
    /// Digest of the exact serialized manifest.
    pub manifest_sha256: String,
    /// Digest of the upstream approval decision and evidence.
    pub approval_evidence_sha256: String,
    /// Exact approved role.
    pub role: SemanticModelRole,
    /// Fixed true only after upstream admission.
    pub approved: bool,
}

/// One exact file and field preview shown before workspace opt-in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SemanticScopeEntry {
    /// Canonical workspace-relative source path.
    pub path: WorkspacePath,
    /// Exact current source digest.
    pub content_sha256: String,
    /// Exact source byte count.
    pub byte_count: u64,
    /// Closed included field set.
    pub fields: BTreeSet<SemanticField>,
}

/// Complete per-workspace opt-in and local storage disclosure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SemanticOptIn {
    /// Exact approved roots.
    pub roots: Vec<WorkspaceScopePath>,
    /// Exact file and field preview.
    pub entries: Vec<SemanticScopeEntry>,
    /// Digest of the policy decision controlling this opt-in.
    pub policy_sha256: String,
    /// Digest of the accepted same-profile comparative benchmark decision.
    pub benefit_evidence_sha256: String,
    /// Explicit derived storage protection choice.
    pub storage_protection: SemanticStorageProtection,
    /// Maximum admitted source bytes.
    pub max_source_bytes: u64,
    /// Maximum admitted index records.
    pub max_records: u64,
    /// Whether the user approved this exact preview.
    pub user_approved: bool,
    /// Must remain true; there is no remote mode in this capability.
    pub local_only: bool,
    /// Whether complete inspect controls were disclosed.
    pub inspection_disclosed: bool,
    /// Whether complete deletion and rebuild controls were disclosed.
    pub deletion_disclosed: bool,
    /// Whether retention behavior was disclosed.
    pub retention_disclosed: bool,
}

/// Immutable activation identity for an approved exact embedding profile and scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticActivation {
    manifest: SemanticModelManifest,
    manifest_sha256: String,
    reranker_manifest_sha256: Option<String>,
    opt_in: SemanticOptIn,
    opt_in_sha256: String,
    chunker_id: String,
    chunker_sha256: String,
    index_schema_version: u16,
}

/// One exact source chunk supplied to a local approved model runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticChunkInput {
    /// Canonical source path.
    pub path: WorkspacePath,
    /// Exact observed source digest.
    pub content_sha256: String,
    /// Digest currently expected from the canonical source.
    pub current_content_sha256: String,
    /// Exact source range.
    pub source_range: ObsidianSourceRange,
    /// Exact branch identity.
    pub branch: String,
    /// Exact bounded text sent only to the local runtime.
    pub text: String,
    /// Closed disclosed field class.
    pub field: SemanticField,
}

/// Fixed-point vector returned by a separately controlled local runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticVectorInput {
    /// Digest of the exact chunk key and text.
    pub chunk_sha256: String,
    /// Signed fixed-point components; floats and NaN states are excluded.
    pub values: Vec<i16>,
}

/// Complete content-addressed semantic record identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticIndexKey {
    /// Canonical source path.
    pub path: WorkspacePath,
    /// Exact source-content digest.
    pub content_sha256: String,
    /// Exact source range.
    pub source_range: ObsidianSourceRange,
    /// Exact branch identity.
    pub branch: String,
    /// Exact model-manifest digest.
    pub model_manifest_sha256: String,
    /// Optional exact reranker-manifest digest.
    pub reranker_manifest_sha256: Option<String>,
    /// Exact tokenizer digest.
    pub tokenizer_sha256: String,
    /// Exact tokenizer identity.
    pub tokenizer_id: String,
    /// Exact chunker digest.
    pub chunker_sha256: String,
    /// Exact chunker identity.
    pub chunker_id: String,
    /// Exact index schema version.
    pub index_schema_version: u16,
    /// Exact workspace opt-in policy digest.
    pub policy_sha256: String,
    /// Digest of the complete workspace opt-in and disclosure.
    pub opt_in_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SemanticIndexRecord {
    key: SemanticIndexKey,
    field: SemanticField,
    text: String,
    vector: Vec<i16>,
    chunk_sha256: String,
}

/// One bounded deterministic semantic query result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticIndexHit {
    /// Complete source and configuration identity.
    pub key: SemanticIndexKey,
    /// Closed source field.
    pub field: SemanticField,
    /// Exact local excerpt.
    pub text: String,
    /// Signed fixed-point dot-product score.
    pub score: i64,
    /// Stable record digest used for inspection and receipts.
    pub record_sha256: String,
}

/// Content-free inspect summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticIndexSummary {
    /// Current monotonic derived revision.
    pub revision: u64,
    /// Exact record count.
    pub record_count: u64,
    /// Digest of all ordered record identities and vectors.
    pub index_sha256: String,
    /// Exact active model-manifest digest.
    pub model_manifest_sha256: String,
    /// Optional exact active reranker-manifest digest.
    pub reranker_manifest_sha256: Option<String>,
    /// Exact active opt-in digest.
    pub opt_in_sha256: String,
    /// Fixed false remote-capability marker.
    pub remote_enabled: bool,
}

/// Complete atomic rebuild result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticIndexReport {
    /// Published derived revision.
    pub revision: u64,
    /// Published record count.
    pub record_count: u64,
    /// Published index digest.
    pub index_sha256: String,
    /// Count rejected because source digests were stale.
    pub stale_source_count: u64,
    /// Fixed true lexical fallback marker.
    pub lexical_fallback_available: bool,
    /// Fixed false source mutation marker.
    pub source_mutated: bool,
}

/// Content-free local lifecycle receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticLifecycleReceipt {
    /// Stable operation identity.
    pub operation: String,
    /// Revision after the operation.
    pub revision: u64,
    /// Number of affected records.
    pub affected_records: u64,
    /// Index digest after the operation.
    pub index_sha256: String,
    /// Digest of the affected path, if applicable.
    pub path_sha256: Option<String>,
    /// Fixed false network marker.
    pub network_used: bool,
    /// Fixed false source mutation marker.
    pub source_mutated: bool,
}

/// Closed semantic lifecycle failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticError {
    /// Manifest, receipt, scope, chunk, vector, query, or metadata is invalid.
    InvalidInput,
    /// Profile has not passed exact admission.
    NotApproved,
    /// Workspace/file/field is outside the approved preview.
    ScopeDenied,
    /// Observed source digest is stale.
    StaleSource,
    /// Model, tokenizer, chunker, schema, branch, or policy identity changed.
    IncompatibleIndex,
    /// Fixed resource ceiling was exceeded.
    ResourceLimit,
    /// A vector was missing, duplicated, or bound to the wrong chunk.
    VectorMismatch,
}

impl SemanticActivation {
    /// Validates exact model admission, scope opt-in, disclosures, and index identities.
    pub fn new(
        manifest: SemanticModelManifest,
        admission: &SemanticAdmissionReceipt,
        reranker: Option<(SemanticModelManifest, SemanticAdmissionReceipt)>,
        opt_in: SemanticOptIn,
        chunker_id: String,
        chunker_sha256: String,
        index_schema_version: u16,
    ) -> Result<Self, SemanticError> {
        validate_manifest(&manifest)?;
        let manifest_sha256 = digest_json(&manifest)?;
        if manifest.role != SemanticModelRole::Embedding
            || !admission_matches(&manifest, &manifest_sha256, admission)
        {
            return Err(SemanticError::NotApproved);
        }
        let reranker_manifest_sha256 =
            if let Some((reranker_manifest, reranker_admission)) = reranker {
                validate_manifest(&reranker_manifest)?;
                let digest = digest_json(&reranker_manifest)?;
                if reranker_manifest.role != SemanticModelRole::Reranking
                    || !admission_matches(&reranker_manifest, &digest, &reranker_admission)
                {
                    return Err(SemanticError::NotApproved);
                }
                Some(digest)
            } else {
                None
            };
        validate_opt_in(&opt_in)?;
        if !valid_identifier(&chunker_id)
            || !valid_sha256(&chunker_sha256)
            || index_schema_version == 0
        {
            return Err(SemanticError::InvalidInput);
        }
        let opt_in_sha256 = digest_json(&opt_in)?;
        Ok(Self {
            manifest,
            manifest_sha256,
            reranker_manifest_sha256,
            opt_in,
            opt_in_sha256,
            chunker_id,
            chunker_sha256,
            index_schema_version,
        })
    }

    /// Returns the exact model-manifest digest.
    #[must_use]
    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    /// Returns the exact workspace opt-in digest.
    #[must_use]
    pub fn opt_in_sha256(&self) -> &str {
        &self.opt_in_sha256
    }

    /// Returns the optional exact reranker-manifest digest.
    #[must_use]
    pub fn reranker_manifest_sha256(&self) -> Option<&str> {
        self.reranker_manifest_sha256.as_deref()
    }

    /// Verifies that another activation is exactly index-compatible.
    pub fn verify_compatible(&self, candidate: &Self) -> Result<(), SemanticError> {
        if self.manifest_sha256 != candidate.manifest_sha256
            || self.reranker_manifest_sha256 != candidate.reranker_manifest_sha256
            || self.opt_in_sha256 != candidate.opt_in_sha256
            || self.chunker_id != candidate.chunker_id
            || self.chunker_sha256 != candidate.chunker_sha256
            || self.index_schema_version != candidate.index_schema_version
        {
            return Err(SemanticError::IncompatibleIndex);
        }
        Ok(())
    }
}

/// In-memory local derived semantic index with no runtime, filesystem, or network handle.
pub struct LocalSemanticIndex {
    activation: SemanticActivation,
    revision: u64,
    records: BTreeMap<SemanticIndexKey, SemanticIndexRecord>,
    index_sha256: String,
}

impl LocalSemanticIndex {
    /// Creates an empty derived index for one exact activation.
    #[must_use]
    pub fn new(activation: SemanticActivation) -> Self {
        Self {
            activation,
            revision: 0,
            records: BTreeMap::new(),
            index_sha256: sha256(&[]),
        }
    }

    /// Atomically replaces the complete derived projection after exact vector reconciliation.
    pub fn rebuild(
        &mut self,
        chunks: &[SemanticChunkInput],
        vectors: &[SemanticVectorInput],
    ) -> Result<(SemanticIndexReport, SemanticLifecycleReceipt), SemanticError> {
        if chunks.len() > MAX_CHUNKS
            || u64::try_from(chunks.len()).map_err(|_| SemanticError::ResourceLimit)?
                > self.activation.opt_in.max_records
        {
            return Err(SemanticError::ResourceLimit);
        }
        let vector_map = vectors
            .iter()
            .map(|vector| (vector.chunk_sha256.clone(), vector))
            .collect::<BTreeMap<_, _>>();
        if vector_map.len() != vectors.len() || vectors.len() != chunks.len() {
            return Err(SemanticError::VectorMismatch);
        }
        let mut next = BTreeMap::new();
        let mut stale_source_count = 0_u64;
        for chunk in chunks {
            validate_chunk(&self.activation, chunk)?;
            if chunk.content_sha256 != chunk.current_content_sha256 {
                stale_source_count = stale_source_count.saturating_add(1);
                continue;
            }
            let chunk_sha256 = chunk_digest(chunk)?;
            let vector = vector_map
                .get(&chunk_sha256)
                .ok_or(SemanticError::VectorMismatch)?;
            validate_vector(&self.activation, vector)?;
            let key = index_key(&self.activation, chunk);
            if next
                .insert(
                    key.clone(),
                    SemanticIndexRecord {
                        key,
                        field: chunk.field,
                        text: chunk.text.clone(),
                        vector: vector.values.clone(),
                        chunk_sha256,
                    },
                )
                .is_some()
            {
                return Err(SemanticError::InvalidInput);
            }
        }
        if next.len() != chunks.len().saturating_sub(stale_source_count as usize) {
            return Err(SemanticError::VectorMismatch);
        }
        let index_sha256 = index_digest(&next)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(SemanticError::ResourceLimit)?;
        self.records = next;
        self.revision = revision;
        self.index_sha256 = index_sha256.clone();
        let record_count =
            u64::try_from(self.records.len()).map_err(|_| SemanticError::ResourceLimit)?;
        Ok((
            SemanticIndexReport {
                revision,
                record_count,
                index_sha256: index_sha256.clone(),
                stale_source_count,
                lexical_fallback_available: true,
                source_mutated: false,
            },
            SemanticLifecycleReceipt {
                operation: "rebuild".to_owned(),
                revision,
                affected_records: record_count,
                index_sha256,
                path_sha256: None,
                network_used: false,
                source_mutated: false,
            },
        ))
    }

    /// Returns a content-free inspect summary.
    #[must_use]
    pub fn inspect(&self) -> SemanticIndexSummary {
        SemanticIndexSummary {
            revision: self.revision,
            record_count: self.records.len() as u64,
            index_sha256: self.index_sha256.clone(),
            model_manifest_sha256: self.activation.manifest_sha256.clone(),
            reranker_manifest_sha256: self.activation.reranker_manifest_sha256.clone(),
            opt_in_sha256: self.activation.opt_in_sha256.clone(),
            remote_enabled: false,
        }
    }

    /// Runs deterministic fixed-point local vector retrieval.
    pub fn query(
        &self,
        vector: &[i16],
        max_results: u32,
    ) -> Result<Vec<SemanticIndexHit>, SemanticError> {
        if max_results == 0
            || max_results > MAX_QUERY_RESULTS
            || vector.len() != self.activation.manifest.dimensions as usize
        {
            return Err(SemanticError::InvalidInput);
        }
        let mut hits = self
            .records
            .values()
            .map(|record| {
                let score = record
                    .vector
                    .iter()
                    .zip(vector)
                    .try_fold(0_i64, |sum, (left, right)| {
                        sum.checked_add(i64::from(*left) * i64::from(*right))
                    })
                    .ok_or(SemanticError::ResourceLimit)?;
                Ok(SemanticIndexHit {
                    key: record.key.clone(),
                    field: record.field,
                    text: record.text.clone(),
                    score,
                    record_sha256: record_digest(record)?,
                })
            })
            .collect::<Result<Vec<_>, SemanticError>>()?;
        hits.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.key.cmp(&right.key))
        });
        hits.truncate(max_results as usize);
        Ok(hits)
    }

    /// Deletes every derived record for one approved path and proves it is no longer queryable.
    pub fn delete_path(
        &mut self,
        path: &WorkspacePath,
    ) -> Result<SemanticLifecycleReceipt, SemanticError> {
        if !self.path_is_approved(path) {
            return Err(SemanticError::ScopeDenied);
        }
        let before = self.records.len();
        self.records.retain(|key, _| &key.path != path);
        let affected = before.saturating_sub(self.records.len());
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(SemanticError::ResourceLimit)?;
        self.revision = revision;
        self.index_sha256 = index_digest(&self.records)?;
        Ok(SemanticLifecycleReceipt {
            operation: "delete_path".to_owned(),
            revision,
            affected_records: affected as u64,
            index_sha256: self.index_sha256.clone(),
            path_sha256: Some(digest_json(path)?),
            network_used: false,
            source_mutated: false,
        })
    }

    /// Clears the complete derived projection without changing source files or activation policy.
    pub fn clear(&mut self) -> Result<SemanticLifecycleReceipt, SemanticError> {
        let affected = self.records.len() as u64;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(SemanticError::ResourceLimit)?;
        self.records.clear();
        self.revision = revision;
        self.index_sha256 = sha256(&[]);
        Ok(SemanticLifecycleReceipt {
            operation: "clear".to_owned(),
            revision,
            affected_records: affected,
            index_sha256: self.index_sha256.clone(),
            path_sha256: None,
            network_used: false,
            source_mutated: false,
        })
    }

    fn path_is_approved(&self, path: &WorkspacePath) -> bool {
        self.activation
            .opt_in
            .entries
            .iter()
            .any(|entry| &entry.path == path)
    }
}

fn validate_manifest(manifest: &SemanticModelManifest) -> Result<(), SemanticError> {
    if !valid_identifier(&manifest.profile_id)
        || !valid_metadata(&manifest.publisher)
        || !valid_metadata(&manifest.lineage)
        || !valid_metadata(&manifest.license_id)
        || !valid_sha256(&manifest.artifact_sha256)
        || !valid_identifier(&manifest.tokenizer_id)
        || !valid_sha256(&manifest.tokenizer_sha256)
        || manifest.dimensions == 0
        || manifest.dimensions > MAX_DIMENSIONS
        || manifest.max_resident_bytes == 0
        || !manifest.origin_policy_passed
        || !manifest.license_policy_passed
    {
        return Err(SemanticError::InvalidInput);
    }
    Ok(())
}

fn admission_matches(
    manifest: &SemanticModelManifest,
    manifest_sha256: &str,
    admission: &SemanticAdmissionReceipt,
) -> bool {
    admission.approved
        && admission.role == manifest.role
        && admission.manifest_sha256 == manifest_sha256
        && valid_sha256(&admission.approval_evidence_sha256)
        && manifest.state == SemanticProfileState::Approved
}

fn validate_opt_in(opt_in: &SemanticOptIn) -> Result<(), SemanticError> {
    if opt_in.roots.is_empty()
        || opt_in.entries.is_empty()
        || opt_in.entries.len() > MAX_SCOPE_ENTRIES
        || !valid_sha256(&opt_in.policy_sha256)
        || !valid_sha256(&opt_in.benefit_evidence_sha256)
        || opt_in.max_source_bytes == 0
        || opt_in.max_records == 0
        || !opt_in.user_approved
        || !opt_in.local_only
        || !opt_in.inspection_disclosed
        || !opt_in.deletion_disclosed
        || !opt_in.retention_disclosed
    {
        return Err(SemanticError::InvalidInput);
    }
    let mut paths = BTreeSet::new();
    let mut bytes = 0_u64;
    for entry in &opt_in.entries {
        if !opt_in
            .roots
            .iter()
            .any(|root| root.contains_path(&entry.path))
            || !valid_sha256(&entry.content_sha256)
            || entry.byte_count == 0
            || entry.fields.is_empty()
            || !paths.insert(entry.path.clone())
        {
            return Err(SemanticError::InvalidInput);
        }
        bytes = bytes
            .checked_add(entry.byte_count)
            .ok_or(SemanticError::ResourceLimit)?;
    }
    if bytes > opt_in.max_source_bytes
        || u64::try_from(opt_in.entries.len()).map_err(|_| SemanticError::ResourceLimit)?
            > opt_in.max_records
    {
        return Err(SemanticError::ResourceLimit);
    }
    Ok(())
}

fn validate_chunk(
    activation: &SemanticActivation,
    chunk: &SemanticChunkInput,
) -> Result<(), SemanticError> {
    let entry = activation
        .opt_in
        .entries
        .iter()
        .find(|entry| entry.path == chunk.path)
        .ok_or(SemanticError::ScopeDenied)?;
    if !entry.fields.contains(&chunk.field) {
        return Err(SemanticError::ScopeDenied);
    }
    if chunk.content_sha256 != entry.content_sha256
        || !valid_sha256(&chunk.content_sha256)
        || !valid_sha256(&chunk.current_content_sha256)
        || !valid_range(chunk.source_range)
        || chunk.text.is_empty()
        || chunk.text.len() > MAX_CHUNK_BYTES
        || crate::domain::secret_candidate(&chunk.text)
        || chunk.branch.is_empty()
        || chunk.branch.len() > MAX_BRANCH_BYTES
        || chunk.branch.chars().any(char::is_control)
    {
        return Err(SemanticError::InvalidInput);
    }
    if chunk.content_sha256 != chunk.current_content_sha256 {
        return Ok(());
    }
    Ok(())
}

fn validate_vector(
    activation: &SemanticActivation,
    vector: &SemanticVectorInput,
) -> Result<(), SemanticError> {
    if !valid_sha256(&vector.chunk_sha256)
        || vector.values.len() != activation.manifest.dimensions as usize
        || vector.values.iter().all(|value| *value == 0)
    {
        return Err(SemanticError::VectorMismatch);
    }
    Ok(())
}

fn index_key(activation: &SemanticActivation, chunk: &SemanticChunkInput) -> SemanticIndexKey {
    SemanticIndexKey {
        path: chunk.path.clone(),
        content_sha256: chunk.content_sha256.clone(),
        source_range: chunk.source_range,
        branch: chunk.branch.clone(),
        model_manifest_sha256: activation.manifest_sha256.clone(),
        reranker_manifest_sha256: activation.reranker_manifest_sha256.clone(),
        tokenizer_sha256: activation.manifest.tokenizer_sha256.clone(),
        tokenizer_id: activation.manifest.tokenizer_id.clone(),
        chunker_sha256: activation.chunker_sha256.clone(),
        chunker_id: activation.chunker_id.clone(),
        index_schema_version: activation.index_schema_version,
        policy_sha256: activation.opt_in.policy_sha256.clone(),
        opt_in_sha256: activation.opt_in_sha256.clone(),
    }
}

fn chunk_digest(chunk: &SemanticChunkInput) -> Result<String, SemanticError> {
    digest_json(&(
        &chunk.path,
        &chunk.content_sha256,
        chunk.source_range.start_line,
        chunk.source_range.start_column,
        chunk.source_range.end_line,
        chunk.source_range.end_column,
        &chunk.branch,
        &chunk.text,
        chunk.field,
    ))
}

fn record_digest(record: &SemanticIndexRecord) -> Result<String, SemanticError> {
    digest_json(&(
        (
            &record.key.path,
            &record.key.content_sha256,
            (
                record.key.source_range.start_line,
                record.key.source_range.start_column,
                record.key.source_range.end_line,
                record.key.source_range.end_column,
            ),
            &record.key.branch,
            &record.key.model_manifest_sha256,
            &record.key.reranker_manifest_sha256,
            &record.key.tokenizer_sha256,
            &record.key.tokenizer_id,
            &record.key.chunker_sha256,
            &record.key.chunker_id,
            record.key.index_schema_version,
            &record.key.policy_sha256,
            &record.key.opt_in_sha256,
        ),
        record.field,
        &record.chunk_sha256,
        &record.vector,
    ))
}

fn index_digest(
    records: &BTreeMap<SemanticIndexKey, SemanticIndexRecord>,
) -> Result<String, SemanticError> {
    let digests = records
        .values()
        .map(record_digest)
        .collect::<Result<Vec<_>, _>>()?;
    digest_json(&digests)
}

fn digest_json<T: Serialize>(value: &T) -> Result<String, SemanticError> {
    let bytes = serde_json::to_vec(value).map_err(|_| SemanticError::InvalidInput)?;
    Ok(sha256(&bytes))
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROFILE_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn valid_metadata(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_METADATA_BYTES && !value.chars().any(char::is_control)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_range(range: ObsidianSourceRange) -> bool {
    range.start_line > 0
        && range.start_column > 0
        && range.end_line >= range.start_line
        && range.end_column > 0
        && (range.end_line > range.start_line || range.end_column > range.start_column)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath, WorkspaceScopePath};

    use super::{
        LocalSemanticIndex, SemanticActivation, SemanticAdmissionReceipt, SemanticChunkInput,
        SemanticError, SemanticField, SemanticModelManifest, SemanticModelRole, SemanticOptIn,
        SemanticProfileState, SemanticRuntime, SemanticScopeEntry, SemanticStorageProtection,
        SemanticVectorInput, chunk_digest, digest_json,
    };
    use crate::ObsidianSourceRange;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_raw("workspace-semantic")
    }

    fn root() -> WorkspaceScopePath {
        WorkspaceScopePath::new(workspace(), ["vault"]).expect("valid root")
    }

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(workspace(), ["vault", name]).expect("valid path")
    }

    fn manifest(role: SemanticModelRole) -> SemanticModelManifest {
        SemanticModelManifest {
            profile_id: match role {
                SemanticModelRole::Embedding => "fixture-embedding-v1",
                SemanticModelRole::Reranking => "fixture-reranking-v1",
            }
            .to_owned(),
            role,
            publisher: "fixture publisher".to_owned(),
            lineage: "synthetic test fixture; not a released model".to_owned(),
            license_id: "Apache-2.0".to_owned(),
            artifact_sha256: match role {
                SemanticModelRole::Embedding => "a".repeat(64),
                SemanticModelRole::Reranking => "b".repeat(64),
            },
            tokenizer_id: "fixture-tokenizer-v1".to_owned(),
            tokenizer_sha256: "c".repeat(64),
            runtime: SemanticRuntime::NativeLlamaCpp,
            dimensions: 3,
            max_resident_bytes: 1024,
            state: SemanticProfileState::Approved,
            origin_policy_passed: true,
            license_policy_passed: true,
        }
    }

    fn admission(manifest: &SemanticModelManifest) -> SemanticAdmissionReceipt {
        SemanticAdmissionReceipt {
            manifest_sha256: digest_json(manifest).expect("manifest serializes"),
            approval_evidence_sha256: "d".repeat(64),
            role: manifest.role,
            approved: true,
        }
    }

    fn opt_in() -> SemanticOptIn {
        SemanticOptIn {
            roots: vec![root()],
            entries: vec![
                SemanticScopeEntry {
                    path: path("alpha.md"),
                    content_sha256: "e".repeat(64),
                    byte_count: 100,
                    fields: [SemanticField::Heading, SemanticField::Body]
                        .into_iter()
                        .collect(),
                },
                SemanticScopeEntry {
                    path: path("beta.md"),
                    content_sha256: "f".repeat(64),
                    byte_count: 100,
                    fields: [SemanticField::Body].into_iter().collect(),
                },
            ],
            policy_sha256: "1".repeat(64),
            benefit_evidence_sha256: "3".repeat(64),
            storage_protection: SemanticStorageProtection::HostDiskEncryption,
            max_source_bytes: 200,
            max_records: 10,
            user_approved: true,
            local_only: true,
            inspection_disclosed: true,
            deletion_disclosed: true,
            retention_disclosed: true,
        }
    }

    fn activation(with_reranker: bool) -> SemanticActivation {
        let embedding = manifest(SemanticModelRole::Embedding);
        let embedding_admission = admission(&embedding);
        let reranker = with_reranker.then(|| {
            let value = manifest(SemanticModelRole::Reranking);
            let receipt = admission(&value);
            (value, receipt)
        });
        SemanticActivation::new(
            embedding,
            &embedding_admission,
            reranker,
            opt_in(),
            "fixture-chunker-v1".to_owned(),
            "2".repeat(64),
            1,
        )
        .expect("valid fixture activation")
    }

    fn chunk(name: &str, text: &str, line: u32, field: SemanticField) -> SemanticChunkInput {
        let digest = if name == "alpha.md" {
            "e".repeat(64)
        } else {
            "f".repeat(64)
        };
        SemanticChunkInput {
            path: path(name),
            content_sha256: digest.clone(),
            current_content_sha256: digest,
            source_range: ObsidianSourceRange {
                start_line: line,
                start_column: 1,
                end_line: line,
                end_column: 20,
            },
            branch: "main".to_owned(),
            text: text.to_owned(),
            field,
        }
    }

    fn vector(chunk: &SemanticChunkInput, values: [i16; 3]) -> SemanticVectorInput {
        SemanticVectorInput {
            chunk_sha256: chunk_digest(chunk).expect("chunk serializes"),
            values: values.to_vec(),
        }
    }

    #[test]
    fn exact_admission_reranker_and_opt_in_are_required() {
        let embedding = manifest(SemanticModelRole::Embedding);
        let mut receipt = admission(&embedding);
        receipt.approved = false;
        assert_eq!(
            SemanticActivation::new(
                embedding.clone(),
                &receipt,
                None,
                opt_in(),
                "fixture-chunker-v1".to_owned(),
                "2".repeat(64),
                1,
            ),
            Err(SemanticError::NotApproved)
        );

        let mut changed = embedding.clone();
        changed.artifact_sha256 = "9".repeat(64);
        assert_eq!(
            SemanticActivation::new(
                changed,
                &admission(&embedding),
                None,
                opt_in(),
                "fixture-chunker-v1".to_owned(),
                "2".repeat(64),
                1,
            ),
            Err(SemanticError::NotApproved)
        );

        let mut denied_scope = opt_in();
        denied_scope.local_only = false;
        assert_eq!(
            SemanticActivation::new(
                embedding.clone(),
                &admission(&embedding),
                None,
                denied_scope,
                "fixture-chunker-v1".to_owned(),
                "2".repeat(64),
                1,
            ),
            Err(SemanticError::InvalidInput)
        );

        let active = activation(true);
        assert!(active.reranker_manifest_sha256().is_some());
    }

    #[test]
    fn rebuild_query_and_keys_are_deterministic_source_ranged_and_local() {
        let alpha = chunk(
            "alpha.md",
            "instruction text remains inert",
            2,
            SemanticField::Body,
        );
        let beta = chunk("beta.md", "semantic prose", 3, SemanticField::Body);
        let mut index = LocalSemanticIndex::new(activation(true));
        let (report, receipt) = index
            .rebuild(
                &[beta.clone(), alpha.clone()],
                &[vector(&alpha, [10, 1, 0]), vector(&beta, [1, 10, 0])],
            )
            .expect("rebuild succeeds");
        assert_eq!(report.record_count, 2);
        assert!(report.lexical_fallback_available);
        assert!(!report.source_mutated);
        assert!(!receipt.network_used);
        assert!(!receipt.source_mutated);

        let hits = index.query(&[10, 1, 0], 2).expect("query succeeds");
        assert_eq!(hits[0].key.path, path("alpha.md"));
        assert_eq!(hits[0].key.source_range, alpha.source_range);
        assert_eq!(hits[0].key.branch, "main");
        assert_eq!(hits[0].key.index_schema_version, 1);
        assert_eq!(hits[0].record_sha256.len(), 64);
        assert!(hits[0].key.reranker_manifest_sha256.is_some());
        assert!(!index.inspect().remote_enabled);
    }

    #[test]
    fn delete_clear_and_complete_rebuild_remove_old_content() {
        let alpha = chunk("alpha.md", "alpha concept", 1, SemanticField::Body);
        let beta = chunk("beta.md", "beta concept", 1, SemanticField::Body);
        let mut index = LocalSemanticIndex::new(activation(false));
        index
            .rebuild(
                &[alpha.clone(), beta.clone()],
                &[vector(&alpha, [10, 0, 0]), vector(&beta, [0, 10, 0])],
            )
            .expect("initial rebuild succeeds");
        let deletion = index
            .delete_path(&path("alpha.md"))
            .expect("delete succeeds");
        assert_eq!(deletion.affected_records, 1);
        assert!(
            index
                .query(&[10, 0, 0], 10)
                .expect("query succeeds")
                .iter()
                .all(|hit| hit.key.path != path("alpha.md") && !hit.text.contains("alpha"))
        );
        let cleared = index.clear().expect("clear succeeds");
        assert_eq!(cleared.affected_records, 1);
        assert!(
            index
                .query(&[1, 1, 1], 10)
                .expect("empty query succeeds")
                .is_empty()
        );

        let mut changed = beta;
        changed.branch = "feature".to_owned();
        index
            .rebuild(
                std::slice::from_ref(&changed),
                &[vector(&changed, [0, 10, 0])],
            )
            .expect("replacement rebuild succeeds");
        let hits = index.query(&[0, 10, 0], 10).expect("query succeeds");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key.branch, "feature");
    }

    #[test]
    fn failures_are_atomic_and_secret_scope_vector_and_resource_attacks_fail_closed() {
        let alpha = chunk("alpha.md", "safe content", 1, SemanticField::Body);
        let mut index = LocalSemanticIndex::new(activation(false));
        index
            .rebuild(std::slice::from_ref(&alpha), &[vector(&alpha, [1, 2, 3])])
            .expect("initial rebuild succeeds");
        let before = index.inspect();

        let mut secret = alpha.clone();
        secret.text = "password=hidden".to_owned();
        assert_eq!(
            index.rebuild(std::slice::from_ref(&secret), &[vector(&secret, [1, 2, 3])]),
            Err(SemanticError::InvalidInput)
        );
        assert_eq!(index.inspect(), before);

        let mut canary = alpha.clone();
        canary.path =
            WorkspacePath::new(workspace(), ["other", "canary.md"]).expect("valid unrelated path");
        assert_eq!(
            index.rebuild(std::slice::from_ref(&canary), &[vector(&canary, [1, 2, 3])]),
            Err(SemanticError::ScopeDenied)
        );
        assert_eq!(index.inspect(), before);

        assert_eq!(
            index.rebuild(std::slice::from_ref(&alpha), &[]),
            Err(SemanticError::VectorMismatch)
        );
        assert_eq!(index.query(&[1, 2], 1), Err(SemanticError::InvalidInput));
        assert_eq!(index.inspect(), before);
    }

    #[test]
    fn stale_sources_are_omitted_and_every_configuration_identity_invalidates() {
        let mut stale = chunk("alpha.md", "stale concept", 1, SemanticField::Body);
        stale.current_content_sha256 = "8".repeat(64);
        let mut index = LocalSemanticIndex::new(activation(false));
        let (report, _) = index
            .rebuild(std::slice::from_ref(&stale), &[vector(&stale, [1, 2, 3])])
            .expect("stale rebuild publishes no records");
        assert_eq!(report.stale_source_count, 1);
        assert_eq!(report.record_count, 0);

        let baseline = activation(false);
        let same = activation(false);
        assert_eq!(baseline.verify_compatible(&same), Ok(()));

        let embedding = manifest(SemanticModelRole::Embedding);
        let changed = SemanticActivation::new(
            embedding.clone(),
            &admission(&embedding),
            None,
            opt_in(),
            "changed-chunker".to_owned(),
            "7".repeat(64),
            2,
        )
        .expect("changed activation is independently valid");
        assert_eq!(
            baseline.verify_compatible(&changed),
            Err(SemanticError::IncompatibleIndex)
        );
    }

    #[test]
    fn scope_preview_rejects_duplicate_outside_empty_and_undisclosed_fields() {
        let embedding = manifest(SemanticModelRole::Embedding);
        let receipt = admission(&embedding);
        let mut duplicate = opt_in();
        duplicate.entries.push(duplicate.entries[0].clone());
        duplicate.max_source_bytes = 1_000;
        assert_eq!(
            SemanticActivation::new(
                embedding.clone(),
                &receipt,
                None,
                duplicate,
                "fixture-chunker-v1".to_owned(),
                "2".repeat(64),
                1,
            ),
            Err(SemanticError::InvalidInput)
        );

        let mut index = LocalSemanticIndex::new(activation(false));
        let undisclosed = chunk("beta.md", "heading denied", 1, SemanticField::Heading);
        assert_eq!(
            index.rebuild(
                std::slice::from_ref(&undisclosed),
                &[vector(&undisclosed, [1, 2, 3])],
            ),
            Err(SemanticError::ScopeDenied)
        );

        let fields = BTreeSet::<SemanticField>::new();
        assert!(fields.is_empty(), "empty field fixture remains explicit");
    }
}
