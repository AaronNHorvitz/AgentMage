//! Hash-bound, authority-free declarative skill packages and bounded context composition.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::KNOWLEDGE_SCHEMA_VERSION;

const MAX_SKILLS: usize = 1_024;
const MAX_FILES: usize = 256;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CONTEXT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_ID_BYTES: usize = 128;

/// Closed data-only file kind accepted from one declarative skill package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclarativeAssetKind {
    /// Bounded prompt text proposed as untrusted model context.
    Prompt,
    /// Declarative JSON or textual schema data.
    Schema,
    /// Bounded non-executable example data.
    Example,
    /// Bounded text template data.
    Template,
}

/// Closed visible trust state for one exact package revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclarativeSkillTrustState {
    /// Package is known but cannot contribute context.
    Disabled,
    /// Package failed or awaits trust review and cannot contribute context.
    Quarantined,
    /// Exact hash-bound package was admitted for bounded data-only loading.
    Admitted,
}

/// Version and schema compatibility declared by one skill revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeclarativeSkillCompatibility {
    /// Minimum supported knowledge schema version.
    pub minimum_knowledge_schema: u16,
    /// Maximum supported knowledge schema version.
    pub maximum_knowledge_schema: u16,
    /// Minimum AgentMage capability version expressed as exact `major.minor.patch`.
    pub minimum_agentmage_version: String,
}

/// Authority-free requested scope for one declarative skill.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeclarativeSkillScope {
    /// Maximum aggregate context bytes this skill may contribute.
    pub max_context_bytes: u64,
    /// Exact knowledge record kinds named by lowercase wire value.
    pub knowledge_record_kinds: Vec<String>,
    /// Exact read-only workflow identities the package may support.
    pub workflow_ids: Vec<String>,
}

/// One declared package file and its exact content identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DeclarativeSkillFile {
    /// Canonical package-relative path.
    pub path: String,
    /// Closed data-only kind.
    pub kind: DeclarativeAssetKind,
    /// Semantic instruction key for prompts/templates, or a data key for schemas/examples.
    pub semantic_key: String,
    /// Exact UTF-8 byte count.
    pub bytes: u64,
    /// SHA-256 of exact file bytes.
    pub content_sha256: String,
}

/// Complete hash-bound declarative skill record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DeclarativeSkillManifest {
    /// Declarative-skill schema version.
    pub schema_version: u16,
    /// Stable skill identity.
    pub skill_id: String,
    /// Stable source identity without ambient path authority.
    pub source_id: String,
    /// Exact source revision or package provenance digest.
    pub source_sha256: String,
    /// Signer identity or bounded provenance statement.
    pub signer_or_provenance: String,
    /// SPDX-style license identifier or `Proprietary`.
    pub license: String,
    /// Exact semantic version.
    pub version: String,
    /// Supported product and knowledge schema range.
    pub compatibility: DeclarativeSkillCompatibility,
    /// Bounded user-visible purpose.
    pub purpose: String,
    /// Explicit precedence; larger values are considered first.
    pub precedence: u16,
    /// Complete sorted package file inventory.
    pub files: Vec<DeclarativeSkillFile>,
    /// Bounded read-only data scope.
    pub requested_scope: DeclarativeSkillScope,
    /// Visible trust state for this exact revision.
    pub trust_state: DeclarativeSkillTrustState,
    /// Digest of the complete ordered file descriptor set.
    pub package_sha256: String,
    /// Digest of every preceding manifest field.
    pub manifest_sha256: String,
}

/// Exact UTF-8 package asset supplied by a caller with no path or execution authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarativeSkillAsset {
    /// Canonical package-relative path matching the manifest.
    pub path: String,
    /// Exact data bytes.
    pub bytes: Vec<u8>,
}

/// One complete manifest and exact asset set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarativeSkillPackage {
    /// Hash-bound package manifest.
    pub manifest: DeclarativeSkillManifest,
    /// Complete assets in the same stable path order as the manifest.
    pub assets: Vec<DeclarativeSkillAsset>,
}

/// Fixed authority ceiling for every declarative skill and composed context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SkillAuthorityCeiling {
    /// Filesystem authority is absent.
    pub filesystem: bool,
    /// Shell authority is absent.
    pub shell: bool,
    /// Secret authority is absent.
    pub secrets: bool,
    /// Network authority is absent.
    pub network: bool,
    /// Connector authority is absent.
    pub connectors: bool,
    /// Approval authority is absent.
    pub approvals: bool,
    /// Grant creation authority is absent.
    pub grant_creation: bool,
    /// Tool registration authority is absent.
    pub tool_registration: bool,
    /// Code execution authority is absent.
    pub code_execution: bool,
    /// Workspace-root expansion authority is absent.
    pub workspace_expansion: bool,
    /// Memory promotion authority is absent.
    pub memory_promotion: bool,
    /// Canonical or external write authority is absent.
    pub writes: bool,
}

impl SkillAuthorityCeiling {
    /// Returns the only valid declarative-skill authority state.
    #[must_use]
    pub const fn denied() -> Self {
        Self {
            filesystem: false,
            shell: false,
            secrets: false,
            network: false,
            connectors: false,
            approvals: false,
            grant_creation: false,
            tool_registration: false,
            code_execution: false,
            workspace_expansion: false,
            memory_promotion: false,
            writes: false,
        }
    }
}

/// One visible conflict between exact instruction contributions.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SkillInstructionConflict {
    /// Conflicting semantic key.
    pub semantic_key: String,
    /// Sorted skill identities contributing different bytes.
    pub skill_ids: Vec<String>,
    /// Sorted content digests that disagree.
    pub content_sha256: Vec<String>,
}

/// One bounded data-only context entry in effective precedence order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillContextEntry {
    /// Stable skill identity.
    pub skill_id: String,
    /// Exact package manifest digest.
    pub manifest_sha256: String,
    /// Source asset path.
    pub asset_path: String,
    /// Closed asset kind.
    pub kind: DeclarativeAssetKind,
    /// Semantic key.
    pub semantic_key: String,
    /// Visible precedence.
    pub precedence: u16,
    /// Exact bounded UTF-8 data.
    pub text: String,
    /// Exact asset digest.
    pub content_sha256: String,
}

/// Content-free receipt describing exact skill influence and permanent authority denial.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillInfluenceReceipt {
    /// Applied skill identities in effective order.
    pub skill_ids: Vec<String>,
    /// Applied exact manifest digests.
    pub manifest_sha256: Vec<String>,
    /// Included asset digests in effective order.
    pub included_content_sha256: Vec<String>,
    /// Visible conflicts omitted from effective context.
    pub conflicts: Vec<SkillInstructionConflict>,
    /// Exact total context bytes.
    pub context_bytes: u64,
    /// Digest of the complete effective context projection.
    pub context_sha256: String,
    /// Fixed authority denial.
    pub authority: SkillAuthorityCeiling,
}

/// Bounded effective declarative context plus its influence receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillContext {
    /// Effective conflict-free entries.
    pub entries: Vec<SkillContextEntry>,
    /// Complete influence receipt.
    pub receipt: SkillInfluenceReceipt,
    /// Fixed false marker: context is untrusted data, not a trusted instruction channel.
    pub trusted_instruction_channel: bool,
    /// Fixed false marker: composition executes nothing.
    pub executed: bool,
}

/// Stable content-free declarative-skill failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarativeSkillError {
    /// Manifest or asset metadata is malformed, duplicated, or internally inconsistent.
    InvalidManifest,
    /// Package version or knowledge schema range is unsupported.
    UnsupportedVersion,
    /// Exact source, package, manifest, or asset hashes do not match.
    HashMismatch,
    /// Package is disabled or quarantined.
    TrustDenied,
    /// File path, kind, bytes, or undeclared content is prohibited.
    ProhibitedContent,
    /// Package or composed context exceeds a fixed bound.
    ResourceLimit,
    /// Registry contains a duplicate stable identity.
    DuplicateIdentity,
}

impl std::fmt::Display for DeclarativeSkillError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidManifest => "declarative_skill.manifest.invalid",
            Self::UnsupportedVersion => "declarative_skill.version.unsupported",
            Self::HashMismatch => "declarative_skill.hash.mismatch",
            Self::TrustDenied => "declarative_skill.trust.denied",
            Self::ProhibitedContent => "declarative_skill.content.prohibited",
            Self::ResourceLimit => "declarative_skill.resource.exceeded",
            Self::DuplicateIdentity => "declarative_skill.identity.duplicate",
        })
    }
}

impl std::error::Error for DeclarativeSkillError {}

/// Authority-free registry of exact declarative skill manifests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclarativeSkillRegistry {
    manifests: Vec<DeclarativeSkillManifest>,
}

impl DeclarativeSkillRegistry {
    /// Validates and registers exact manifests in stable skill-identity order.
    pub fn build(manifests: Vec<DeclarativeSkillManifest>) -> Result<Self, DeclarativeSkillError> {
        if manifests.len() > MAX_SKILLS {
            return Err(DeclarativeSkillError::ResourceLimit);
        }
        let mut manifests = manifests;
        manifests.sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
        if manifests
            .windows(2)
            .any(|pair| pair[0].skill_id == pair[1].skill_id)
        {
            return Err(DeclarativeSkillError::DuplicateIdentity);
        }
        for manifest in &manifests {
            validate_manifest(manifest)?;
        }
        Ok(Self { manifests })
    }

    /// Returns all exact manifests in stable identity order.
    #[must_use]
    pub fn manifests(&self) -> &[DeclarativeSkillManifest] {
        &self.manifests
    }

    /// Returns one exact manifest without loading any package bytes.
    #[must_use]
    pub fn manifest(&self, skill_id: &str) -> Option<&DeclarativeSkillManifest> {
        self.manifests
            .binary_search_by_key(&skill_id, |manifest| manifest.skill_id.as_str())
            .ok()
            .map(|index| &self.manifests[index])
    }

    /// Returns a new registry without one selected skill; no files are deleted.
    pub fn without(&self, skill_id: &str) -> Result<Self, DeclarativeSkillError> {
        let Some(index) = self
            .manifests
            .iter()
            .position(|manifest| manifest.skill_id == skill_id)
        else {
            return Err(DeclarativeSkillError::InvalidManifest);
        };
        let mut manifests = self.manifests.clone();
        manifests.remove(index);
        Self::build(manifests)
    }
}

/// Computes package and manifest digests for an otherwise complete candidate manifest.
pub fn seal_declarative_skill_manifest(
    mut manifest: DeclarativeSkillManifest,
) -> Result<DeclarativeSkillManifest, DeclarativeSkillError> {
    manifest.package_sha256.clear();
    manifest.manifest_sha256.clear();
    validate_manifest_fields(&manifest)?;
    manifest.package_sha256 = package_digest(&manifest.files)?;
    manifest.manifest_sha256 = manifest_digest(&manifest)?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Composes admitted exact packages into bounded conflict-free untrusted context.
pub fn compose_skill_context(
    registry: &DeclarativeSkillRegistry,
    packages: &[DeclarativeSkillPackage],
    max_context_bytes: u64,
) -> Result<SkillContext, DeclarativeSkillError> {
    if packages.len() > MAX_SKILLS
        || max_context_bytes == 0
        || max_context_bytes > MAX_CONTEXT_BYTES
    {
        return Err(DeclarativeSkillError::ResourceLimit);
    }
    let mut loaded = Vec::new();
    let mut seen = BTreeSet::new();
    for package in packages {
        if !seen.insert(package.manifest.skill_id.as_str()) {
            return Err(DeclarativeSkillError::DuplicateIdentity);
        }
        let registered = registry
            .manifest(&package.manifest.skill_id)
            .ok_or(DeclarativeSkillError::TrustDenied)?;
        if registered != &package.manifest {
            return Err(DeclarativeSkillError::HashMismatch);
        }
        loaded.extend(load_package(package)?);
    }
    let conflicts = instruction_conflicts(&loaded);
    let conflicting_keys = conflicts
        .iter()
        .map(|conflict| conflict.semantic_key.as_str())
        .collect::<BTreeSet<_>>();
    loaded.retain(|entry| !conflicting_keys.contains(entry.semantic_key.as_str()));
    loaded.sort_by(|left, right| {
        right
            .precedence
            .cmp(&left.precedence)
            .then_with(|| left.skill_id.cmp(&right.skill_id))
            .then_with(|| left.asset_path.cmp(&right.asset_path))
    });
    let context_bytes = loaded.iter().try_fold(0_u64, |total, entry| {
        total
            .checked_add(entry.text.len() as u64)
            .ok_or(DeclarativeSkillError::ResourceLimit)
    })?;
    if context_bytes > max_context_bytes {
        return Err(DeclarativeSkillError::ResourceLimit);
    }
    let context_sha256 = context_digest(&loaded, &conflicts)?;
    let receipt = SkillInfluenceReceipt {
        skill_ids: loaded.iter().map(|entry| entry.skill_id.clone()).collect(),
        manifest_sha256: loaded
            .iter()
            .map(|entry| entry.manifest_sha256.clone())
            .collect(),
        included_content_sha256: loaded
            .iter()
            .map(|entry| entry.content_sha256.clone())
            .collect(),
        conflicts,
        context_bytes,
        context_sha256,
        authority: SkillAuthorityCeiling::denied(),
    };
    Ok(SkillContext {
        entries: loaded,
        receipt,
        trusted_instruction_channel: false,
        executed: false,
    })
}

fn load_package(
    package: &DeclarativeSkillPackage,
) -> Result<Vec<SkillContextEntry>, DeclarativeSkillError> {
    validate_manifest(&package.manifest)?;
    if package.manifest.trust_state != DeclarativeSkillTrustState::Admitted {
        return Err(DeclarativeSkillError::TrustDenied);
    }
    if package.assets.len() != package.manifest.files.len() {
        return Err(DeclarativeSkillError::ProhibitedContent);
    }
    let mut entries = Vec::with_capacity(package.assets.len());
    let mut package_bytes = 0_u64;
    for (asset, declared) in package.assets.iter().zip(&package.manifest.files) {
        if asset.path != declared.path
            || asset.bytes.len() as u64 != declared.bytes
            || sha256(&asset.bytes) != declared.content_sha256
        {
            return Err(DeclarativeSkillError::HashMismatch);
        }
        package_bytes = package_bytes
            .checked_add(declared.bytes)
            .ok_or(DeclarativeSkillError::ResourceLimit)?;
        if prohibited_asset(&declared.path, &asset.bytes) {
            return Err(DeclarativeSkillError::ProhibitedContent);
        }
        let text = String::from_utf8(asset.bytes.clone())
            .map_err(|_| DeclarativeSkillError::ProhibitedContent)?;
        entries.push(SkillContextEntry {
            skill_id: package.manifest.skill_id.clone(),
            manifest_sha256: package.manifest.manifest_sha256.clone(),
            asset_path: asset.path.clone(),
            kind: declared.kind,
            semantic_key: declared.semantic_key.clone(),
            precedence: package.manifest.precedence,
            text,
            content_sha256: declared.content_sha256.clone(),
        });
    }
    if package_bytes > package.manifest.requested_scope.max_context_bytes
        || package_bytes > MAX_PACKAGE_BYTES
    {
        return Err(DeclarativeSkillError::ResourceLimit);
    }
    Ok(entries)
}

fn validate_manifest(manifest: &DeclarativeSkillManifest) -> Result<(), DeclarativeSkillError> {
    validate_manifest_fields(manifest)?;
    if manifest.package_sha256 != package_digest(&manifest.files)?
        || manifest.manifest_sha256 != manifest_digest(manifest)?
    {
        return Err(DeclarativeSkillError::HashMismatch);
    }
    Ok(())
}

fn validate_manifest_fields(
    manifest: &DeclarativeSkillManifest,
) -> Result<(), DeclarativeSkillError> {
    if manifest.schema_version != 1
        || !valid_prefixed_id(&manifest.skill_id, "skill-")
        || !valid_id(&manifest.source_id)
        || !valid_sha256_or_empty(&manifest.source_sha256)
        || !valid_text(&manifest.signer_or_provenance)
        || !valid_license(&manifest.license)
        || !valid_semver(&manifest.version)
        || !valid_text(&manifest.purpose)
        || manifest.precedence == 0
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES
        || manifest.requested_scope.max_context_bytes == 0
        || manifest.requested_scope.max_context_bytes > MAX_PACKAGE_BYTES
        || manifest.requested_scope.knowledge_record_kinds.len() > 64
        || manifest.requested_scope.workflow_ids.len() > 64
        || !valid_sha256_or_empty(&manifest.package_sha256)
        || !valid_sha256_or_empty(&manifest.manifest_sha256)
    {
        return Err(DeclarativeSkillError::InvalidManifest);
    }
    if manifest.compatibility.minimum_knowledge_schema > KNOWLEDGE_SCHEMA_VERSION
        || manifest.compatibility.maximum_knowledge_schema < KNOWLEDGE_SCHEMA_VERSION
        || manifest.compatibility.minimum_knowledge_schema
            > manifest.compatibility.maximum_knowledge_schema
        || !valid_semver(&manifest.compatibility.minimum_agentmage_version)
    {
        return Err(DeclarativeSkillError::UnsupportedVersion);
    }
    if manifest
        .requested_scope
        .knowledge_record_kinds
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
        || manifest
            .requested_scope
            .knowledge_record_kinds
            .iter()
            .any(|value| !valid_scope_code(value))
        || manifest
            .requested_scope
            .workflow_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || manifest
            .requested_scope
            .workflow_ids
            .iter()
            .any(|value| !valid_scope_code(value))
    {
        return Err(DeclarativeSkillError::InvalidManifest);
    }
    let mut total = 0_u64;
    for (index, file) in manifest.files.iter().enumerate() {
        if !valid_relative_path(&file.path)
            || !valid_scope_code(&file.semantic_key)
            || file.bytes == 0
            || file.bytes > MAX_FILE_BYTES
            || !valid_sha256(&file.content_sha256)
            || index > 0 && manifest.files[index - 1].path >= file.path
            || !extension_matches(file.kind, &file.path)
        {
            return Err(DeclarativeSkillError::InvalidManifest);
        }
        total = total
            .checked_add(file.bytes)
            .ok_or(DeclarativeSkillError::ResourceLimit)?;
    }
    if total > MAX_PACKAGE_BYTES || total > manifest.requested_scope.max_context_bytes {
        return Err(DeclarativeSkillError::ResourceLimit);
    }
    Ok(())
}

fn instruction_conflicts(entries: &[SkillContextEntry]) -> Vec<SkillInstructionConflict> {
    let mut grouped: BTreeMap<&str, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    for entry in entries.iter().filter(|entry| {
        matches!(
            entry.kind,
            DeclarativeAssetKind::Prompt | DeclarativeAssetKind::Template
        )
    }) {
        let group = grouped.entry(&entry.semantic_key).or_default();
        group.0.insert(entry.skill_id.clone());
        group.1.insert(entry.content_sha256.clone());
    }
    grouped
        .into_iter()
        .filter(|(_, (_, hashes))| hashes.len() > 1)
        .map(
            |(semantic_key, (skill_ids, content_sha256))| SkillInstructionConflict {
                semantic_key: semantic_key.to_owned(),
                skill_ids: skill_ids.into_iter().collect(),
                content_sha256: content_sha256.into_iter().collect(),
            },
        )
        .collect()
}

fn context_digest(
    entries: &[SkillContextEntry],
    conflicts: &[SkillInstructionConflict],
) -> Result<String, DeclarativeSkillError> {
    let entry_projection = entries
        .iter()
        .map(|entry| {
            (
                entry.skill_id.as_str(),
                entry.manifest_sha256.as_str(),
                entry.asset_path.as_str(),
                entry.kind,
                entry.semantic_key.as_str(),
                entry.precedence,
                entry.content_sha256.as_str(),
                entry.text.len() as u64,
            )
        })
        .collect::<Vec<_>>();
    let conflict_projection = conflicts
        .iter()
        .map(|conflict| {
            (
                conflict.semantic_key.as_str(),
                &conflict.skill_ids,
                &conflict.content_sha256,
            )
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&(
        "agentmage-declarative-skill-context-v1",
        entry_projection,
        conflict_projection,
        SkillAuthorityCeiling::denied(),
    ))
    .map(|bytes| sha256(&bytes))
    .map_err(|_| DeclarativeSkillError::InvalidManifest)
}

fn package_digest(files: &[DeclarativeSkillFile]) -> Result<String, DeclarativeSkillError> {
    serde_json::to_vec(&("agentmage-declarative-skill-package-v1", files))
        .map(|bytes| sha256(&bytes))
        .map_err(|_| DeclarativeSkillError::InvalidManifest)
}

fn manifest_digest(manifest: &DeclarativeSkillManifest) -> Result<String, DeclarativeSkillError> {
    #[derive(Serialize)]
    struct Unsigned<'a> {
        schema_version: u16,
        skill_id: &'a str,
        source_id: &'a str,
        source_sha256: &'a str,
        signer_or_provenance: &'a str,
        license: &'a str,
        version: &'a str,
        compatibility: &'a DeclarativeSkillCompatibility,
        purpose: &'a str,
        precedence: u16,
        files: &'a [DeclarativeSkillFile],
        requested_scope: &'a DeclarativeSkillScope,
        trust_state: DeclarativeSkillTrustState,
        package_sha256: &'a str,
    }
    serde_json::to_vec(&Unsigned {
        schema_version: manifest.schema_version,
        skill_id: &manifest.skill_id,
        source_id: &manifest.source_id,
        source_sha256: &manifest.source_sha256,
        signer_or_provenance: &manifest.signer_or_provenance,
        license: &manifest.license,
        version: &manifest.version,
        compatibility: &manifest.compatibility,
        purpose: &manifest.purpose,
        precedence: manifest.precedence,
        files: &manifest.files,
        requested_scope: &manifest.requested_scope,
        trust_state: manifest.trust_state,
        package_sha256: &manifest.package_sha256,
    })
    .map(|bytes| sha256(&bytes))
    .map_err(|_| DeclarativeSkillError::InvalidManifest)
}

fn prohibited_asset(path: &str, bytes: &[u8]) -> bool {
    if bytes.contains(&0)
        || bytes.starts_with(b"#!")
        || bytes
            .windows(10)
            .any(|window| window.eq_ignore_ascii_case(b"<tool_call"))
        || bytes
            .windows(b"agentmage_hidden_tool".len())
            .any(|window| window.eq_ignore_ascii_case(b"agentmage_hidden_tool"))
    {
        return true;
    }
    let lowered = path.to_ascii_lowercase();
    [
        ".sh", ".bash", ".zsh", ".fish", ".ps1", ".bat", ".cmd", ".exe", ".dll", ".so", ".dylib",
        ".py", ".js", ".ts", ".rs", ".wasm",
    ]
    .iter()
    .any(|extension| lowered.ends_with(extension))
}

fn extension_matches(kind: DeclarativeAssetKind, path: &str) -> bool {
    match kind {
        DeclarativeAssetKind::Prompt => path.ends_with(".prompt.md"),
        DeclarativeAssetKind::Schema => path.ends_with(".schema.json"),
        DeclarativeAssetKind::Example => path.ends_with(".example.md"),
        DeclarativeAssetKind::Template => path.ends_with(".template.md"),
    }
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.starts_with('/')
        && !value.starts_with('~')
        && !value.contains('\\')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn valid_prefixed_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix) && valid_id(value)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_scope_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(|character| character == '\0')
}

fn valid_license(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'+'))
}

fn valid_semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}

fn valid_sha256_or_empty(value: &str) -> bool {
    value.is_empty() || valid_sha256(value)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(
        skill_id: &str,
        semantic_key: &str,
        text: &str,
        precedence: u16,
        trust_state: DeclarativeSkillTrustState,
    ) -> DeclarativeSkillPackage {
        let path = format!("prompts/{semantic_key}.prompt.md");
        let asset = DeclarativeSkillAsset {
            path: path.clone(),
            bytes: text.as_bytes().to_vec(),
        };
        let manifest = seal_declarative_skill_manifest(DeclarativeSkillManifest {
            schema_version: 1,
            skill_id: skill_id.to_owned(),
            source_id: "agentmage-built-in-skill-pack".to_owned(),
            source_sha256: sha256(b"built-in-source"),
            signer_or_provenance: "AgentMage source repository".to_owned(),
            license: "Apache-2.0".to_owned(),
            version: "0.2.0".to_owned(),
            compatibility: DeclarativeSkillCompatibility {
                minimum_knowledge_schema: 1,
                maximum_knowledge_schema: 1,
                minimum_agentmage_version: "0.2.0".to_owned(),
            },
            purpose: "Read-only knowledge fixture".to_owned(),
            precedence,
            files: vec![DeclarativeSkillFile {
                path,
                kind: DeclarativeAssetKind::Prompt,
                semantic_key: semantic_key.to_owned(),
                bytes: text.len() as u64,
                content_sha256: sha256(text.as_bytes()),
            }],
            requested_scope: DeclarativeSkillScope {
                max_context_bytes: 16 * 1024,
                knowledge_record_kinds: vec!["task".to_owned()],
                workflow_ids: vec!["daily-setup".to_owned()],
            },
            trust_state,
            package_sha256: String::new(),
            manifest_sha256: String::new(),
        })
        .expect("manifest seals");
        DeclarativeSkillPackage {
            manifest,
            assets: vec![asset],
        }
    }

    #[test]
    fn admitted_packages_compose_in_visible_precedence_with_zero_authority() {
        let lower = package(
            "skill-daily-setup",
            "daily_setup",
            "Summarize source-backed tasks.",
            10,
            DeclarativeSkillTrustState::Admitted,
        );
        let higher = package(
            "skill-issue-intake",
            "issue_intake",
            "Classify the provided issue without writing.",
            20,
            DeclarativeSkillTrustState::Admitted,
        );
        let registry =
            DeclarativeSkillRegistry::build(vec![lower.manifest.clone(), higher.manifest.clone()])
                .expect("registry");
        let context = compose_skill_context(&registry, &[lower, higher], 64 * 1024)
            .expect("context composes");
        assert_eq!(context.entries[0].skill_id, "skill-issue-intake");
        assert_eq!(context.entries[1].skill_id, "skill-daily-setup");
        assert_eq!(context.receipt.authority, SkillAuthorityCeiling::denied());
        assert!(!context.trusted_instruction_channel);
        assert!(!context.executed);
        assert!(context.receipt.conflicts.is_empty());
    }

    #[test]
    fn conflicting_instruction_keys_are_visible_and_contribute_no_context() {
        let first = package(
            "skill-conflict-a",
            "shared_policy",
            "Use source A.",
            10,
            DeclarativeSkillTrustState::Admitted,
        );
        let second = package(
            "skill-conflict-b",
            "shared_policy",
            "Use source B.",
            20,
            DeclarativeSkillTrustState::Admitted,
        );
        let registry =
            DeclarativeSkillRegistry::build(vec![first.manifest.clone(), second.manifest.clone()])
                .expect("registry");
        let context = compose_skill_context(&registry, &[first, second], 64 * 1024)
            .expect("conflict remains reviewable");
        assert!(context.entries.is_empty());
        assert_eq!(context.receipt.conflicts.len(), 1);
        assert_eq!(context.receipt.conflicts[0].semantic_key, "shared_policy");
        assert_eq!(context.receipt.context_bytes, 0);
    }

    #[test]
    fn disabled_quarantined_tampered_and_unregistered_packages_fail_closed() {
        for state in [
            DeclarativeSkillTrustState::Disabled,
            DeclarativeSkillTrustState::Quarantined,
        ] {
            let candidate = package("skill-denied", "denied", "Denied", 10, state);
            let registry = DeclarativeSkillRegistry::build(vec![candidate.manifest.clone()])
                .expect("registry retains disabled package");
            assert_eq!(
                compose_skill_context(&registry, &[candidate], 1024),
                Err(DeclarativeSkillError::TrustDenied)
            );
        }
        let candidate = package(
            "skill-tampered",
            "tampered",
            "Original",
            10,
            DeclarativeSkillTrustState::Admitted,
        );
        let registry =
            DeclarativeSkillRegistry::build(vec![candidate.manifest.clone()]).expect("registry");
        let mut tampered = candidate.clone();
        tampered.assets[0].bytes = b"Replacement".to_vec();
        assert_eq!(
            compose_skill_context(&registry, &[tampered], 1024),
            Err(DeclarativeSkillError::HashMismatch)
        );
        assert_eq!(
            compose_skill_context(
                &DeclarativeSkillRegistry::build(Vec::new()).expect("empty registry"),
                &[candidate],
                1024,
            ),
            Err(DeclarativeSkillError::TrustDenied)
        );
    }

    #[test]
    fn hidden_tool_executable_path_expansion_and_excessive_context_are_rejected() {
        for (skill_id, semantic_key, text) in [
            (
                "skill-hidden-tool",
                "hidden_tool",
                "agentmage_hidden_tool execute now",
            ),
            ("skill-shebang", "shebang", "#!/bin/sh\necho unsafe"),
        ] {
            let candidate = package(
                skill_id,
                semantic_key,
                text,
                10,
                DeclarativeSkillTrustState::Admitted,
            );
            let registry = DeclarativeSkillRegistry::build(vec![candidate.manifest.clone()])
                .expect("registry");
            assert_eq!(
                compose_skill_context(&registry, &[candidate], 64 * 1024),
                Err(DeclarativeSkillError::ProhibitedContent)
            );
        }
        let mut traversal = package(
            "skill-traversal",
            "traversal",
            "Read only",
            10,
            DeclarativeSkillTrustState::Admitted,
        )
        .manifest;
        traversal.files[0].path = "../outside.prompt.md".to_owned();
        traversal.package_sha256.clear();
        traversal.manifest_sha256.clear();
        assert_eq!(
            seal_declarative_skill_manifest(traversal),
            Err(DeclarativeSkillError::InvalidManifest)
        );
        let candidate = package(
            "skill-budget",
            "budget",
            "This text exceeds the selected composition budget.",
            10,
            DeclarativeSkillTrustState::Admitted,
        );
        let registry =
            DeclarativeSkillRegistry::build(vec![candidate.manifest.clone()]).expect("registry");
        assert_eq!(
            compose_skill_context(&registry, &[candidate], 8),
            Err(DeclarativeSkillError::ResourceLimit)
        );
    }

    #[test]
    fn registry_rejects_duplicates_and_removal_is_manifest_only() {
        let candidate = package(
            "skill-removable",
            "removable",
            "Read only",
            10,
            DeclarativeSkillTrustState::Admitted,
        );
        assert_eq!(
            DeclarativeSkillRegistry::build(vec![
                candidate.manifest.clone(),
                candidate.manifest.clone(),
            ]),
            Err(DeclarativeSkillError::DuplicateIdentity)
        );
        let registry = DeclarativeSkillRegistry::build(vec![candidate.manifest]).expect("registry");
        let removed = registry
            .without("skill-removable")
            .expect("manifest removed");
        assert!(removed.manifests().is_empty());
    }
}
