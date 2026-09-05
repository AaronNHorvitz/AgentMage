//! Client-side-encrypted, namespace-exact cloud continuity contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackupProvider {
    S3Compatible,
    OneDrive,
    GoogleDrive,
    Box,
    Dropbox,
    LaterConformance,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectState {
    NotStarted,
    InProgress,
    Succeeded,
    FailedKnown,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderContract {
    pub provider: BackupProvider,
    pub account: String,
    pub host: String,
    pub container: String,
    pub prefix: String,
    pub object_methods: BTreeSet<String>,
    pub client_side_encryption_required: bool,
    pub metadata_limit: u64,
    pub byte_limit: u64,
    pub rate_limit: u64,
    pub versioning: bool,
    pub retention_policy: String,
    pub deletion_supported: bool,
    pub restore_supported: bool,
    pub removal_supported: bool,
    pub observer_schema_count: u32,
    pub observer_credential_reuse_count: u32,
    pub observer_write_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferPlan {
    pub completed_snapshot_sha256: String,
    pub opaque_object_keys: bool,
    pub plaintext_object_count: u32,
    pub raw_credential_count: u32,
    pub multipart_limit: u32,
    pub resumable: bool,
    pub integrity_sha256: String,
    pub idempotency_key: String,
    pub cancellation_id: String,
    pub expected_account: String,
    pub expected_prefix: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRequest {
    pub host: String,
    pub tls_identity: String,
    pub dns_evidence: String,
    pub proxy: Option<String>,
    pub expected_proxy: Option<String>,
    pub redirect_count: u8,
    pub account: String,
    pub namespace: String,
    pub object_identity: String,
    pub remote_version: String,
    pub credential_reference: String,
    pub effect_state: EffectState,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reconciliation {
    pub observed_state: EffectState,
    pub remote_version: String,
    pub object_count: u32,
    pub duplicate_count: u32,
    pub missing_count: u32,
    pub retention_consistent: bool,
    pub deletion_consistent: bool,
    pub retry_admitted: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudContinuityError {
    Invalid,
    Plaintext,
    Namespace,
    ObserverCrossover,
    UnknownEffect,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 4096
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_provider(v: &ProviderContract) -> Result<(), CloudContinuityError> {
    if !id(&v.account)
        || !id(&v.host)
        || !id(&v.container)
        || !id(&v.prefix)
        || v.object_methods.is_empty()
        || !v.client_side_encryption_required
        || v.metadata_limit == 0
        || v.byte_limit == 0
        || v.rate_limit == 0
        || !id(&v.retention_policy)
        || !v.deletion_supported
        || !v.restore_supported
        || !v.removal_supported
    {
        return Err(CloudContinuityError::Invalid);
    }
    if v.observer_schema_count != 0
        || v.observer_credential_reuse_count != 0
        || v.observer_write_count != 0
    {
        return Err(CloudContinuityError::ObserverCrossover);
    }
    Ok(())
}
pub fn validate_transfer(v: &TransferPlan) -> Result<(), CloudContinuityError> {
    if !digest(&v.completed_snapshot_sha256)
        || !digest(&v.integrity_sha256)
        || !id(&v.idempotency_key)
        || !id(&v.cancellation_id)
        || !id(&v.expected_account)
        || !id(&v.expected_prefix)
        || v.multipart_limit == 0
        || !v.resumable
        || !v.opaque_object_keys
    {
        return Err(CloudContinuityError::Invalid);
    }
    if v.plaintext_object_count != 0 || v.raw_credential_count != 0 {
        return Err(CloudContinuityError::Plaintext);
    }
    Ok(())
}
pub fn validate_effect(
    provider: &ProviderContract,
    plan: &TransferPlan,
    v: &EffectRequest,
) -> Result<(), CloudContinuityError> {
    validate_provider(provider)?;
    validate_transfer(plan)?;
    if v.host != provider.host
        || !id(&v.tls_identity)
        || !id(&v.dns_evidence)
        || v.proxy != v.expected_proxy
        || v.redirect_count > 3
        || v.account != provider.account
        || v.account != plan.expected_account
        || v.namespace != provider.prefix
        || v.namespace != plan.expected_prefix
        || !id(&v.object_identity)
        || !id(&v.remote_version)
        || !id(&v.credential_reference)
    {
        return Err(CloudContinuityError::Namespace);
    }
    if v.effect_state == EffectState::Unknown {
        return Err(CloudContinuityError::UnknownEffect);
    }
    Ok(())
}
pub fn reconcile(v: &Reconciliation) -> Result<(), CloudContinuityError> {
    if v.observed_state == EffectState::Unknown && v.retry_admitted {
        return Err(CloudContinuityError::UnknownEffect);
    }
    if !id(&v.remote_version)
        || v.duplicate_count > 0
        || v.missing_count > 0
        || !v.retention_consistent
        || !v.deletion_consistent
    {
        return Err(CloudContinuityError::Invalid);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn provider() -> ProviderContract {
        ProviderContract {
            provider: BackupProvider::S3Compatible,
            account: "account".into(),
            host: "storage.example.invalid".into(),
            container: "bucket".into(),
            prefix: "backup/".into(),
            object_methods: BTreeSet::from(["put".into(), "get".into(), "delete".into()]),
            client_side_encryption_required: true,
            metadata_limit: 1000,
            byte_limit: 10000,
            rate_limit: 100,
            versioning: true,
            retention_policy: "retain".into(),
            deletion_supported: true,
            restore_supported: true,
            removal_supported: true,
            observer_schema_count: 0,
            observer_credential_reuse_count: 0,
            observer_write_count: 0,
        }
    }
    #[test]
    fn observer_is_structurally_separate() {
        assert_eq!(validate_provider(&provider()), Ok(()));
        let mut v = provider();
        v.observer_write_count = 1;
        assert_eq!(
            validate_provider(&v),
            Err(CloudContinuityError::ObserverCrossover)
        )
    }
    #[test]
    fn unknown_effect_cannot_retry() {
        let v = Reconciliation {
            observed_state: EffectState::Unknown,
            remote_version: "v1".into(),
            object_count: 1,
            duplicate_count: 0,
            missing_count: 0,
            retention_consistent: true,
            deletion_consistent: true,
            retry_admitted: true,
        };
        assert_eq!(reconcile(&v), Err(CloudContinuityError::UnknownEffect))
    }
}
