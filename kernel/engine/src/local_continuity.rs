//! Encrypted local snapshots and transitive audit-checkpoint invalidation.
#![allow(missing_docs)]
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotRootKind {
    Local,
    CloudSync,
    Network,
    Remote,
    Placeholder,
    Linked,
    Raced,
    Unsupported,
    Uncertain,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotKind {
    Full,
    DeduplicatedIncremental,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotManifest {
    pub schema_version: u16,
    pub snapshot_id: String,
    pub release_id: String,
    pub source_schema_id: String,
    pub domains: BTreeSet<String>,
    pub classifications: BTreeMap<String, String>,
    pub exclusions: BTreeMap<String, String>,
    pub retention_policy_id: String,
    pub chunk_digests: Vec<String>,
    pub integrity_root_sha256: String,
    pub encryption_metadata_sha256: String,
    pub required_reference_digests: BTreeSet<String>,
    pub root_kind: SnapshotRootKind,
    pub encrypted: bool,
    pub authenticated: bool,
    pub verified_object_count: u32,
    pub declared_object_count: u32,
    pub complete: bool,
    pub atomic_publication: bool,
    pub opaque_identifier: bool,
    pub raw_credential_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotPlan {
    pub kind: SnapshotKind,
    pub memory_limit: u64,
    pub disk_limit: u64,
    pub cpu_limit: u16,
    pub cancellation_id: String,
    pub cleanup_owner: String,
    pub progress_schema_id: String,
    pub expiry_unix_ms: u64,
    pub cryptographic_deletion: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreAdmission {
    pub manifest_verified: bool,
    pub key_verified: bool,
    pub objects_verified: bool,
    pub hashes_verified: bool,
    pub compatible: bool,
    pub space_verified: bool,
    pub ownership_verified: bool,
    pub domains_verified: bool,
    pub rollback_verified: bool,
    pub staging_verified: bool,
    pub migration_verified: bool,
    pub exact_swap_confirmed: bool,
    pub canonical_mutation_count_before_confirmation: u32,
    pub connected_accounts_reauthentication: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditCheckpoint {
    pub schema_version: u16,
    pub checkpoint_id: String,
    pub generation: u64,
    pub prior_generation_retained: bool,
    pub atomic_complete: bool,
    pub reverse_index_verified: bool,
    pub identity_digests: BTreeMap<String, String>,
    pub raw_secret_count: u32,
    pub retained_source_payload_count: u32,
    pub ambient_path_count: u32,
    pub mutable_summary_fact_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumeIdentity {
    pub repository: String,
    pub worktree: String,
    pub scope: String,
    pub parser: String,
    pub model: String,
    pub runtime: String,
    pub policy: String,
    pub record_schema: String,
    pub resource: String,
    pub checkpoint: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidationResult {
    pub stale_ids: BTreeSet<String>,
    pub replacement_ids: BTreeSet<String>,
    pub broader_rescan: bool,
    pub surviving_stale_current_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuityError {
    Invalid,
    UnsafeRoot,
    Incomplete,
    Plaintext,
    RestoreDenied,
    CheckpointDenied,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 4096
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_snapshot(v: &SnapshotManifest) -> Result<(), ContinuityError> {
    if v.schema_version != 1
        || !id(&v.snapshot_id)
        || !id(&v.release_id)
        || !id(&v.source_schema_id)
        || v.domains.is_empty()
        || v.retention_policy_id.is_empty()
        || v.chunk_digests.is_empty()
        || v.chunk_digests.iter().any(|x| !digest(x))
        || !digest(&v.integrity_root_sha256)
        || !digest(&v.encryption_metadata_sha256)
        || v.required_reference_digests.iter().any(|x| !digest(x))
    {
        return Err(ContinuityError::Invalid);
    }
    if v.root_kind != SnapshotRootKind::Local {
        return Err(ContinuityError::UnsafeRoot);
    }
    if !v.encrypted || !v.authenticated || v.raw_credential_count != 0 {
        return Err(ContinuityError::Plaintext);
    }
    if !v.complete
        || !v.atomic_publication
        || !v.opaque_identifier
        || v.verified_object_count != v.declared_object_count
    {
        return Err(ContinuityError::Incomplete);
    }
    Ok(())
}
pub fn validate_plan(v: &SnapshotPlan) -> Result<(), ContinuityError> {
    if v.memory_limit == 0
        || v.disk_limit == 0
        || v.cpu_limit == 0
        || v.cpu_limit > 100
        || !id(&v.cancellation_id)
        || !id(&v.cleanup_owner)
        || !id(&v.progress_schema_id)
        || v.expiry_unix_ms == 0
        || !v.cryptographic_deletion
    {
        Err(ContinuityError::Invalid)
    } else {
        Ok(())
    }
}
pub fn admit_restore(v: &RestoreAdmission) -> Result<(), ContinuityError> {
    if v.manifest_verified
        && v.key_verified
        && v.objects_verified
        && v.hashes_verified
        && v.compatible
        && v.space_verified
        && v.ownership_verified
        && v.domains_verified
        && v.rollback_verified
        && v.staging_verified
        && v.migration_verified
        && v.exact_swap_confirmed
        && v.canonical_mutation_count_before_confirmation == 0
        && v.connected_accounts_reauthentication
    {
        Ok(())
    } else {
        Err(ContinuityError::RestoreDenied)
    }
}
pub fn validate_checkpoint(v: &AuditCheckpoint) -> Result<(), ContinuityError> {
    let required = BTreeSet::from([
        "audit",
        "repository",
        "scope",
        "parser",
        "model",
        "runtime",
        "policy",
        "queue",
        "census",
        "graph",
        "packet",
        "card",
        "contradiction",
        "finding",
        "resource",
        "completion",
    ]);
    if v.schema_version != 1
        || !id(&v.checkpoint_id)
        || v.generation == 0
        || !v.prior_generation_retained
        || !v.atomic_complete
        || !v.reverse_index_verified
        || v.identity_digests
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != required
        || v.identity_digests.values().any(|x| !digest(x))
        || v.raw_secret_count != 0
        || v.retained_source_payload_count != 0
        || v.ambient_path_count != 0
        || v.mutable_summary_fact_count != 0
    {
        Err(ContinuityError::CheckpointDenied)
    } else {
        Ok(())
    }
}
pub fn resume_matches(saved: &ResumeIdentity, current: &ResumeIdentity) -> bool {
    saved == current
}
pub fn invalidate(
    changed: &BTreeSet<String>,
    reverse: &BTreeMap<String, BTreeSet<String>>,
    relationships_complete: bool,
) -> InvalidationResult {
    let mut stale = changed.clone();
    let mut queue: VecDeque<_> = changed.iter().cloned().collect();
    while let Some(item) = queue.pop_front() {
        if let Some(next) = reverse.get(&item) {
            for dependent in next {
                if stale.insert(dependent.clone()) {
                    queue.push_back(dependent.clone())
                }
            }
        }
    }
    InvalidationResult {
        replacement_ids: stale.clone(),
        stale_ids: stale,
        broader_rescan: !relationships_complete,
        surviving_stale_current_count: 0,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsafe_roots_are_denied() {
        for root in [
            SnapshotRootKind::CloudSync,
            SnapshotRootKind::Network,
            SnapshotRootKind::Remote,
            SnapshotRootKind::Placeholder,
            SnapshotRootKind::Linked,
            SnapshotRootKind::Raced,
            SnapshotRootKind::Unsupported,
            SnapshotRootKind::Uncertain,
        ] {
            let mut v = manifest();
            v.root_kind = root;
            assert_eq!(validate_snapshot(&v), Err(ContinuityError::UnsafeRoot))
        }
    }
    #[test]
    fn complete_encrypted_snapshot_passes() {
        assert_eq!(validate_snapshot(&manifest()), Ok(()))
    }
    #[test]
    fn invalidation_is_transitive_and_fail_broad() {
        let reverse = BTreeMap::from([
            ("file".into(), BTreeSet::from(["symbol".into()])),
            ("symbol".into(), BTreeSet::from(["report".into()])),
        ]);
        let r = invalidate(&BTreeSet::from(["file".into()]), &reverse, false);
        assert_eq!(r.stale_ids.len(), 3);
        assert!(r.broader_rescan);
        assert_eq!(r.surviving_stale_current_count, 0)
    }
    fn manifest() -> SnapshotManifest {
        SnapshotManifest {
            schema_version: 1,
            snapshot_id: "opaque".into(),
            release_id: "release".into(),
            source_schema_id: "schema".into(),
            domains: BTreeSet::from(["work".into()]),
            classifications: BTreeMap::from([("work".into(), "private".into())]),
            exclusions: BTreeMap::new(),
            retention_policy_id: "retain".into(),
            chunk_digests: vec!["a".repeat(64)],
            integrity_root_sha256: "b".repeat(64),
            encryption_metadata_sha256: "c".repeat(64),
            required_reference_digests: BTreeSet::new(),
            root_kind: SnapshotRootKind::Local,
            encrypted: true,
            authenticated: true,
            verified_object_count: 1,
            declared_object_count: 1,
            complete: true,
            atomic_publication: true,
            opaque_identifier: true,
            raw_credential_count: 0,
        }
    }
}
