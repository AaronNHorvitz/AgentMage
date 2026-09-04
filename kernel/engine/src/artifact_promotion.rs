//! Digest-first artifact registry observation and promotion admission.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactProvider {
    OciDistribution,
    GithubContainerRegistry,
    AzureContainerRegistry,
    Artifactory,
    Nexus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactSnapshot {
    pub provider: ArtifactProvider,
    pub canonical_host: String,
    pub repository: String,
    pub immutable_digest: String,
    pub observed_label: String,
    pub observed_epoch_milliseconds: u64,
    pub media_type: String,
    pub size_bytes: u64,
    pub platforms: BTreeSet<String>,
    pub deleted: bool,
    pub signature_sha256: String,
    pub provenance_sha256: String,
    pub permissions_sha256: String,
    pub content_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactPromotionPlan {
    pub plan_id: String,
    pub source_provider: ArtifactProvider,
    pub source_host: String,
    pub source_repository: String,
    pub source_digest: String,
    pub source_mapping_epoch_milliseconds: u64,
    pub target_provider: ArtifactProvider,
    pub target_host: String,
    pub target_repository: String,
    pub target_labels: BTreeSet<String>,
    pub metadata: BTreeMap<String, String>,
    pub retention_sha256: String,
    pub signature_sha256: String,
    pub provenance_sha256: String,
    pub expected_content_sha256: String,
    pub expected_size_bytes: u64,
    pub collision_disposition: String,
    pub idempotency_sha256: String,
    pub grant_sha256: String,
    pub delete_count: u32,
    pub retention_change_count: u32,
    pub signing_key_use_count: u32,
    pub administration_count: u32,
    pub mutable_tag_replacement_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDownloadObservation {
    pub immutable_digest: String,
    pub content_sha256: String,
    pub size_bytes: u64,
    pub unsafe_archive_entry_count: u32,
    pub decompressed_bytes: u64,
    pub cancelled: bool,
    pub quarantined: bool,
    pub cleanup_complete: bool,
    pub model_binary_access_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedArtifactPromotion {
    pub plan_id: String,
    pub source_digest: String,
    pub target_repository: String,
    pub idempotency_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactPromotionError {
    InvalidRecord,
    DigestMismatch,
    StaleMapping,
    UnsafeArtifact,
    HiddenEffect,
}

pub fn admit_artifact_download(
    snapshot: &ArtifactSnapshot,
    observation: &ArtifactDownloadObservation,
    maximum_decompressed_bytes: u64,
) -> Result<(), ArtifactPromotionError> {
    validate_snapshot(snapshot)?;
    if observation.immutable_digest != snapshot.immutable_digest
        || observation.content_sha256 != snapshot.content_sha256
        || observation.size_bytes != snapshot.size_bytes
    {
        return Err(ArtifactPromotionError::DigestMismatch);
    }
    if observation.unsafe_archive_entry_count != 0
        || observation.decompressed_bytes > maximum_decompressed_bytes
        || observation.cancelled
        || observation.quarantined
        || !observation.cleanup_complete
        || observation.model_binary_access_count != 0
    {
        return Err(ArtifactPromotionError::UnsafeArtifact);
    }
    Ok(())
}

pub fn admit_artifact_promotion(
    snapshot: &ArtifactSnapshot,
    plan: &ArtifactPromotionPlan,
    prior_keys: &BTreeSet<String>,
) -> Result<AdmittedArtifactPromotion, ArtifactPromotionError> {
    validate_snapshot(snapshot)?;
    if plan.source_provider != snapshot.provider
        || plan.source_host != snapshot.canonical_host
        || plan.source_repository != snapshot.repository
        || plan.source_digest != snapshot.immutable_digest
        || plan.source_mapping_epoch_milliseconds != snapshot.observed_epoch_milliseconds
        || plan.signature_sha256 != snapshot.signature_sha256
        || plan.provenance_sha256 != snapshot.provenance_sha256
        || plan.expected_content_sha256 != snapshot.content_sha256
        || plan.expected_size_bytes != snapshot.size_bytes
    {
        return Err(ArtifactPromotionError::StaleMapping);
    }
    if !valid_id(&plan.plan_id)
        || !valid_id(&plan.target_host)
        || !valid_id(&plan.target_repository)
        || plan.target_labels.is_empty()
        || plan.target_labels.iter().any(|v| !valid_id(v))
        || plan
            .metadata
            .iter()
            .any(|(k, v)| !valid_id(k) || !valid_sha(v))
        || [
            &plan.retention_sha256,
            &plan.idempotency_sha256,
            &plan.grant_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
        || !matches!(
            plan.collision_disposition.as_str(),
            "deny" | "identical-noop"
        )
    {
        return Err(ArtifactPromotionError::InvalidRecord);
    }
    if plan.delete_count != 0
        || plan.retention_change_count != 0
        || plan.signing_key_use_count != 0
        || plan.administration_count != 0
        || plan.mutable_tag_replacement_count != 0
    {
        return Err(ArtifactPromotionError::HiddenEffect);
    }
    if prior_keys.contains(&plan.idempotency_sha256) {
        return Err(ArtifactPromotionError::HiddenEffect);
    }
    Ok(AdmittedArtifactPromotion {
        plan_id: plan.plan_id.clone(),
        source_digest: plan.source_digest.clone(),
        target_repository: plan.target_repository.clone(),
        idempotency_sha256: plan.idempotency_sha256.clone(),
    })
}

fn validate_snapshot(v: &ArtifactSnapshot) -> Result<(), ArtifactPromotionError> {
    if !valid_id(&v.canonical_host)
        || !valid_id(&v.repository)
        || !valid_id(&v.observed_label)
        || !v.immutable_digest.starts_with("sha256:")
        || !valid_sha(&v.immutable_digest[7..])
        || v.observed_epoch_milliseconds == 0
        || !valid_id(&v.media_type)
        || v.size_bytes == 0
        || v.platforms.is_empty()
        || v.platforms.iter().any(|x| !valid_id(x))
        || v.deleted
        || [
            &v.signature_sha256,
            &v.provenance_sha256,
            &v.permissions_sha256,
            &v.content_sha256,
        ]
        .into_iter()
        .any(|x| !valid_sha(x))
    {
        return Err(ArtifactPromotionError::InvalidRecord);
    }
    Ok(())
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 512
        && v.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/' | b'+')
        })
}
fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snap() -> ArtifactSnapshot {
        let h = "a".repeat(64);
        ArtifactSnapshot {
            provider: ArtifactProvider::OciDistribution,
            canonical_host: "registry.example.test".into(),
            repository: "org/image".into(),
            immutable_digest: format!("sha256:{h}"),
            observed_label: "latest".into(),
            observed_epoch_milliseconds: 1,
            media_type: "application/vnd.oci.image.manifest.v1+json".into(),
            size_bytes: 10,
            platforms: BTreeSet::from(["linux/amd64".into()]),
            deleted: false,
            signature_sha256: h.clone(),
            provenance_sha256: h.clone(),
            permissions_sha256: h.clone(),
            content_sha256: h,
        }
    }
    fn plan(s: &ArtifactSnapshot) -> ArtifactPromotionPlan {
        let h = "b".repeat(64);
        ArtifactPromotionPlan {
            plan_id: "plan-1".into(),
            source_provider: s.provider,
            source_host: s.canonical_host.clone(),
            source_repository: s.repository.clone(),
            source_digest: s.immutable_digest.clone(),
            source_mapping_epoch_milliseconds: s.observed_epoch_milliseconds,
            target_provider: ArtifactProvider::Artifactory,
            target_host: "target.example.test".into(),
            target_repository: "release/image".into(),
            target_labels: BTreeSet::from(["release-1".into()]),
            metadata: BTreeMap::from([("build".into(), h.clone())]),
            retention_sha256: h.clone(),
            signature_sha256: s.signature_sha256.clone(),
            provenance_sha256: s.provenance_sha256.clone(),
            expected_content_sha256: s.content_sha256.clone(),
            expected_size_bytes: s.size_bytes,
            collision_disposition: "deny".into(),
            idempotency_sha256: h.clone(),
            grant_sha256: h,
            delete_count: 0,
            retention_change_count: 0,
            signing_key_use_count: 0,
            administration_count: 0,
            mutable_tag_replacement_count: 0,
        }
    }
    #[test]
    fn exact_digest_promotion_is_admitted_once() {
        let s = snap();
        let p = plan(&s);
        assert!(admit_artifact_promotion(&s, &p, &BTreeSet::new()).is_ok());
        assert!(
            admit_artifact_promotion(&s, &p, &BTreeSet::from([p.idempotency_sha256.clone()]))
                .is_err()
        );
    }
    #[test]
    fn changed_mapping_or_hidden_effect_is_denied() {
        let s = snap();
        let mut p = plan(&s);
        p.source_mapping_epoch_milliseconds = 2;
        assert_eq!(
            admit_artifact_promotion(&s, &p, &BTreeSet::new()),
            Err(ArtifactPromotionError::StaleMapping)
        );
        p.source_mapping_epoch_milliseconds = 1;
        p.delete_count = 1;
        assert_eq!(
            admit_artifact_promotion(&s, &p, &BTreeSet::new()),
            Err(ArtifactPromotionError::HiddenEffect)
        );
    }
    #[test]
    fn downloads_are_digest_bounded_and_binary_inert() {
        let s = snap();
        let mut o = ArtifactDownloadObservation {
            immutable_digest: s.immutable_digest.clone(),
            content_sha256: s.content_sha256.clone(),
            size_bytes: 10,
            unsafe_archive_entry_count: 0,
            decompressed_bytes: 10,
            cancelled: false,
            quarantined: false,
            cleanup_complete: true,
            model_binary_access_count: 0,
        };
        assert!(admit_artifact_download(&s, &o, 10).is_ok());
        o.model_binary_access_count = 1;
        assert_eq!(
            admit_artifact_download(&s, &o, 10),
            Err(ArtifactPromotionError::UnsafeArtifact)
        );
    }
}
