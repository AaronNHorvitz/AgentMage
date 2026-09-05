//! Metadata-only credential references and exact, bounded resolution contracts.
#![allow(missing_docs)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CredentialStore {
    LinuxSecretService,
    WindowsCredentialManager,
    WindowsDpapi,
    MacOsKeychainRetained,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionFlow {
    SystemBrowserOauth,
    DeviceOauth,
    ShortLivedToken,
    SshAgent,
    CertificateReference,
    ScopedManualToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialLifecycle {
    Active,
    Expired,
    Refreshing,
    Rotating,
    Revoked,
    Deleted,
    Disconnected,
    EmergencyDisabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProhibitedSurface {
    Prompts,
    ModelContext,
    Chat,
    Files,
    Arguments,
    Environment,
    Logs,
    Receipts,
    Diagnostics,
    Exports,
    Crashes,
    Snapshots,
    Residue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreAttestation {
    pub store: CredentialStore,
    pub platform_supported: bool,
    pub identity_verified: bool,
    pub unlocked: bool,
    pub access_control_verified: bool,
    pub cryptographic_provider_verified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CredentialReference {
    pub reference_id: String,
    pub store: CredentialStore,
    pub provider: String,
    pub host: String,
    pub tenant: String,
    pub account: String,
    pub scopes: BTreeSet<String>,
    pub acquisition: AcquisitionFlow,
    pub created_unix_ms: u64,
    pub expires_unix_ms: u64,
    pub rotation: u64,
    pub lifecycle: CredentialLifecycle,
    pub support_metadata_digest: String,
    pub secret_value_count: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionRequest {
    pub worker_id: String,
    pub authorized_worker_id: String,
    pub provider: String,
    pub host: String,
    pub tenant: String,
    pub account: String,
    pub operation: String,
    pub authorized_operation: String,
    pub scopes: BTreeSet<String>,
    pub grant_id: String,
    pub redirect_uri: String,
    pub authorized_redirect_uri: String,
    pub proxy: Option<String>,
    pub authorized_proxy: Option<String>,
    pub now_unix_ms: u64,
    pub lifetime_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolutionReceipt {
    pub reference_id: String,
    pub worker_id: String,
    pub operation: String,
    pub expires_unix_ms: u64,
    pub status: &'static str,
    pub secret_value_count: u8,
    pub worker_memory_cleared: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreResult {
    pub restored_reference_ids: Vec<String>,
    pub raw_credential_count: u32,
    pub usable_credential_count: u32,
    pub deterministic_reauthentication_required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanaryObservation {
    pub surface: ProhibitedSurface,
    pub canary_sha256: String,
    pub raw_value_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialError {
    Invalid,
    StoreUnverified,
    Mismatch,
    Inactive,
    Expired,
    Lifetime,
    Disclosure,
    IncompleteScan,
}

fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn validate_store(value: &StoreAttestation) -> Result<(), CredentialError> {
    if value.platform_supported
        && value.identity_verified
        && value.unlocked
        && value.access_control_verified
        && value.cryptographic_provider_verified
    {
        Ok(())
    } else {
        Err(CredentialError::StoreUnverified)
    }
}

pub fn validate_reference(value: &CredentialReference) -> Result<(), CredentialError> {
    if !identifier(&value.reference_id)
        || !identifier(&value.provider)
        || !identifier(&value.host)
        || !identifier(&value.tenant)
        || !identifier(&value.account)
        || value.scopes.is_empty()
        || value.created_unix_ms >= value.expires_unix_ms
        || !digest(&value.support_metadata_digest)
        || value.secret_value_count != 0
    {
        return Err(CredentialError::Invalid);
    }
    Ok(())
}

pub fn resolve(
    store: &StoreAttestation,
    reference: &CredentialReference,
    request: &ResolutionRequest,
) -> Result<ResolutionReceipt, CredentialError> {
    validate_store(store)?;
    validate_reference(reference)?;
    if store.store != reference.store
        || request.worker_id != request.authorized_worker_id
        || request.provider != reference.provider
        || request.host != reference.host
        || request.tenant != reference.tenant
        || request.account != reference.account
        || request.operation != request.authorized_operation
        || request.scopes != reference.scopes
        || !identifier(&request.grant_id)
        || request.redirect_uri != request.authorized_redirect_uri
        || request.proxy != request.authorized_proxy
    {
        return Err(CredentialError::Mismatch);
    }
    if reference.lifecycle != CredentialLifecycle::Active {
        return Err(CredentialError::Inactive);
    }
    if request.now_unix_ms >= reference.expires_unix_ms {
        return Err(CredentialError::Expired);
    }
    if request.lifetime_ms == 0
        || request
            .now_unix_ms
            .checked_add(request.lifetime_ms)
            .is_none_or(|end| end > reference.expires_unix_ms)
    {
        return Err(CredentialError::Lifetime);
    }
    Ok(ResolutionReceipt {
        reference_id: reference.reference_id.clone(),
        worker_id: request.worker_id.clone(),
        operation: request.operation.clone(),
        expires_unix_ms: request.now_unix_ms + request.lifetime_ms,
        status: "resolved-and-cleared",
        secret_value_count: 0,
        worker_memory_cleared: true,
    })
}

pub fn scan_canaries(values: &[CanaryObservation]) -> Result<(), CredentialError> {
    let surfaces: BTreeSet<_> = values.iter().map(|value| value.surface).collect();
    if surfaces.len() != 13 || values.len() != 13 {
        return Err(CredentialError::IncompleteScan);
    }
    if values
        .iter()
        .any(|value| !digest(&value.canary_sha256) || value.raw_value_count != 0)
    {
        return Err(CredentialError::Disclosure);
    }
    Ok(())
}

pub fn restore_references(reference_ids: &[String]) -> Result<RestoreResult, CredentialError> {
    if reference_ids.is_empty() || reference_ids.iter().any(|value| !identifier(value)) {
        return Err(CredentialError::Invalid);
    }
    Ok(RestoreResult {
        restored_reference_ids: reference_ids.to_vec(),
        raw_credential_count: 0,
        usable_credential_count: 0,
        deterministic_reauthentication_required: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> StoreAttestation {
        StoreAttestation {
            store: CredentialStore::LinuxSecretService,
            platform_supported: true,
            identity_verified: true,
            unlocked: true,
            access_control_verified: true,
            cryptographic_provider_verified: true,
        }
    }
    fn reference() -> CredentialReference {
        CredentialReference {
            reference_id: "credential-1".into(),
            store: CredentialStore::LinuxSecretService,
            provider: "provider".into(),
            host: "api.example.invalid".into(),
            tenant: "tenant".into(),
            account: "account".into(),
            scopes: BTreeSet::from(["read".into()]),
            acquisition: AcquisitionFlow::SystemBrowserOauth,
            created_unix_ms: 1,
            expires_unix_ms: 10_000,
            rotation: 1,
            lifecycle: CredentialLifecycle::Active,
            support_metadata_digest: "a".repeat(64),
            secret_value_count: 0,
        }
    }
    fn request() -> ResolutionRequest {
        ResolutionRequest {
            worker_id: "worker".into(),
            authorized_worker_id: "worker".into(),
            provider: "provider".into(),
            host: "api.example.invalid".into(),
            tenant: "tenant".into(),
            account: "account".into(),
            operation: "read".into(),
            authorized_operation: "read".into(),
            scopes: BTreeSet::from(["read".into()]),
            grant_id: "grant".into(),
            redirect_uri: "http://127.0.0.1/callback".into(),
            authorized_redirect_uri: "http://127.0.0.1/callback".into(),
            proxy: None,
            authorized_proxy: None,
            now_unix_ms: 100,
            lifetime_ms: 100,
        }
    }
    #[test]
    fn exact_resolution_returns_metadata_and_clears_memory() {
        let receipt = resolve(&store(), &reference(), &request()).unwrap();
        assert_eq!(receipt.secret_value_count, 0);
        assert!(receipt.worker_memory_cleared);
    }
    #[test]
    fn every_identity_mismatch_fails_closed() {
        let mut value = request();
        value.account = "wrong".into();
        assert_eq!(
            resolve(&store(), &reference(), &value),
            Err(CredentialError::Mismatch)
        );
    }
    #[test]
    fn unverified_store_and_terminal_reference_fail_closed() {
        let mut attestation = store();
        attestation.access_control_verified = false;
        assert_eq!(
            resolve(&attestation, &reference(), &request()),
            Err(CredentialError::StoreUnverified)
        );
        let mut credential = reference();
        credential.lifecycle = CredentialLifecycle::Revoked;
        assert_eq!(
            resolve(&store(), &credential, &request()),
            Err(CredentialError::Inactive)
        );
    }
    #[test]
    fn restore_requires_reauthentication_and_canary_scan_is_complete() {
        let restored = restore_references(&["credential-1".into()]).unwrap();
        assert_eq!(
            (
                restored.raw_credential_count,
                restored.usable_credential_count
            ),
            (0, 0)
        );
        assert!(restored.deterministic_reauthentication_required);
        let observations = [
            ProhibitedSurface::Prompts,
            ProhibitedSurface::ModelContext,
            ProhibitedSurface::Chat,
            ProhibitedSurface::Files,
            ProhibitedSurface::Arguments,
            ProhibitedSurface::Environment,
            ProhibitedSurface::Logs,
            ProhibitedSurface::Receipts,
            ProhibitedSurface::Diagnostics,
            ProhibitedSurface::Exports,
            ProhibitedSurface::Crashes,
            ProhibitedSurface::Snapshots,
            ProhibitedSurface::Residue,
        ]
        .map(|surface| CanaryObservation {
            surface,
            canary_sha256: "b".repeat(64),
            raw_value_count: 0,
        });
        assert_eq!(scan_canaries(&observations), Ok(()));
    }
}
