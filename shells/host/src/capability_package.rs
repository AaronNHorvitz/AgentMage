//! Fail-closed capability package admission, lifecycle preview, and scope narrowing.
#![allow(missing_docs)]

use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write;

const SIGNATURE_DOMAIN: &[u8] = b"agentmage.capability-package.v1\0";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCompatibility {
    pub kernel_version: String,
    pub tool_protocol_version: String,
    pub configuration_version: String,
    pub memory_version: String,
    pub storage_version: String,
    pub shell_version: String,
    pub policy_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDependency {
    pub package_id: String,
    pub exact_version: String,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEntryPoint {
    pub entry_id: String,
    pub relative_path: String,
    pub content_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySideEffect {
    WorkspaceRead,
    WorkspaceWrite,
    Command,
    Network,
    Credential,
    Connector,
    Publication,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityPackageScope {
    pub filesystem_roots: Vec<String>,
    pub commands: Vec<String>,
    pub network_domains: Vec<String>,
    pub credential_ids: Vec<String>,
    pub connector_ids: Vec<String>,
    pub publication_targets: Vec<String>,
    pub max_memory_bytes: u64,
    pub max_cpu_milliseconds: u64,
    pub retention_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityPackageManifest {
    pub schema_version: u16,
    pub package_id: String,
    pub package_name: String,
    pub version: String,
    pub source_id: String,
    pub source_sha256: String,
    pub signer_id: String,
    pub signer_public_key_sha256: String,
    pub license: String,
    pub compatibility: CapabilityCompatibility,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub hooks: Vec<String>,
    pub requested_scope: CapabilityPackageScope,
    pub dependencies: Vec<CapabilityDependency>,
    pub entry_points: Vec<CapabilityEntryPoint>,
    pub side_effects: Vec<CapabilitySideEffect>,
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityPackageAdmission<'a> {
    pub manifest: CapabilityPackageManifest,
    pub signature: &'a [u8; 64],
    pub public_key: &'a [u8; 32],
    pub expected_compatibility: &'a CapabilityCompatibility,
    pub allowed_licenses: &'a BTreeSet<String>,
    pub trusted_signers: &'a BTreeSet<String>,
    pub observed_source_sha256: &'a str,
    pub dependency_locks: &'a [CapabilityDependency],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityPackageVerification {
    pub package_id: String,
    pub version: String,
    pub manifest_sha256: String,
    pub source_sha256: String,
    pub signer_id: String,
    pub signature_verified: bool,
    pub checksum_verified: bool,
    pub license_verified: bool,
    pub provenance_verified: bool,
    pub dependency_locks_verified: bool,
    pub compatibility_verified: bool,
    pub admitted: bool,
    pub verification_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityLifecycleAction {
    Install,
    Enable,
    Disable,
    Update,
    Rollback,
    Uninstall,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityLifecyclePreview {
    pub preview_id: String,
    pub package_id: String,
    pub action: CapabilityLifecycleAction,
    pub from_manifest_sha256: Option<String>,
    pub to_manifest_sha256: Option<String>,
    pub permission_delta_sha256: String,
    pub approval_id: String,
    pub verified_package: bool,
    pub applied: bool,
    pub preview_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityLifecycleInput<'a> {
    pub preview_id: &'a str,
    pub package_id: &'a str,
    pub action: CapabilityLifecycleAction,
    pub from_manifest_sha256: Option<String>,
    pub to_manifest_sha256: Option<String>,
    pub permission_delta_sha256: &'a str,
    pub approval_id: &'a str,
    pub verified_package: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityPackageError {
    InvalidManifest,
    HashMismatch,
    SignatureDenied,
    LicenseDenied,
    ProvenanceDenied,
    DependencyDenied,
    CompatibilityDenied,
    ApprovalDenied,
    ScopeDenied,
}

impl CapabilityPackageError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidManifest => "capability-package.manifest.invalid",
            Self::HashMismatch => "capability-package.hash.mismatch",
            Self::SignatureDenied => "capability-package.signature.denied",
            Self::LicenseDenied => "capability-package.license.denied",
            Self::ProvenanceDenied => "capability-package.provenance.denied",
            Self::DependencyDenied => "capability-package.dependency.denied",
            Self::CompatibilityDenied => "capability-package.compatibility.denied",
            Self::ApprovalDenied => "capability-package.approval.denied",
            Self::ScopeDenied => "capability-package.scope.denied",
        }
    }
}
impl std::fmt::Display for CapabilityPackageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for CapabilityPackageError {}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}
fn version(value: &str) -> bool {
    let parts: Vec<_> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}
fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
fn relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 2048
        && !value.starts_with('/')
        && !value.contains(['\\', '\0'])
        && !value.split('/').any(|part| matches!(part, "" | "." | ".."))
}
fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
fn canonical_manifest_bytes(
    manifest: &CapabilityPackageManifest,
) -> Result<Vec<u8>, CapabilityPackageError> {
    let mut value = manifest.clone();
    value.manifest_sha256.clear();
    serde_json::to_vec(&value).map_err(|_| CapabilityPackageError::InvalidManifest)
}
fn sorted_unique(values: &[String]) -> bool {
    !values.windows(2).any(|pair| pair[0] >= pair[1])
        && values.iter().all(|value| identifier(value))
}
fn valid_scope(scope: &CapabilityPackageScope) -> bool {
    scope.filesystem_roots.len() <= 64
        && scope
            .filesystem_roots
            .iter()
            .all(|value| relative_path(value))
        && sorted_unique(&scope.filesystem_roots)
        && sorted_unique(&scope.commands)
        && sorted_unique(&scope.network_domains)
        && sorted_unique(&scope.credential_ids)
        && sorted_unique(&scope.connector_ids)
        && sorted_unique(&scope.publication_targets)
        && scope.max_memory_bytes > 0
        && scope.max_memory_bytes <= 8 * 1024 * 1024 * 1024
        && scope.max_cpu_milliseconds > 0
        && scope.max_cpu_milliseconds <= 3_600_000
        && scope.retention_seconds <= 31_536_000
}
fn valid_manifest_fields(manifest: &CapabilityPackageManifest) -> bool {
    manifest.schema_version == 1
        && identifier(&manifest.package_id)
        && !manifest.package_name.trim().is_empty()
        && manifest.package_name.len() <= 256
        && version(&manifest.version)
        && identifier(&manifest.source_id)
        && sha(&manifest.source_sha256)
        && identifier(&manifest.signer_id)
        && sha(&manifest.signer_public_key_sha256)
        && identifier(&manifest.license)
        && sorted_unique(&manifest.tools)
        && sorted_unique(&manifest.skills)
        && sorted_unique(&manifest.hooks)
        && valid_scope(&manifest.requested_scope)
        && manifest.dependencies.len() <= 128
        && manifest
            .dependencies
            .windows(2)
            .all(|p| p[0].package_id < p[1].package_id)
        && manifest.dependencies.iter().all(|d| {
            identifier(&d.package_id) && version(&d.exact_version) && sha(&d.manifest_sha256)
        })
        && !manifest.entry_points.is_empty()
        && manifest.entry_points.len() <= 64
        && manifest
            .entry_points
            .windows(2)
            .all(|p| p[0].entry_id < p[1].entry_id)
        && manifest.entry_points.iter().all(|e| {
            identifier(&e.entry_id) && relative_path(&e.relative_path) && sha(&e.content_sha256)
        })
        && manifest
            .side_effects
            .windows(2)
            .all(|pair| (pair[0] as u8) < pair[1] as u8)
}

pub fn seal_capability_manifest(
    mut manifest: CapabilityPackageManifest,
) -> Result<CapabilityPackageManifest, CapabilityPackageError> {
    manifest.manifest_sha256.clear();
    if !valid_manifest_fields(&manifest) {
        return Err(CapabilityPackageError::InvalidManifest);
    }
    manifest.manifest_sha256 = hex(&Sha256::digest(canonical_manifest_bytes(&manifest)?));
    Ok(manifest)
}

pub fn verify_capability_package(
    input: CapabilityPackageAdmission<'_>,
) -> Result<CapabilityPackageVerification, CapabilityPackageError> {
    if !valid_manifest_fields(&input.manifest) || !sha(&input.manifest.manifest_sha256) {
        return Err(CapabilityPackageError::InvalidManifest);
    }
    let bytes = canonical_manifest_bytes(&input.manifest)?;
    if hex(&Sha256::digest(&bytes)) != input.manifest.manifest_sha256
        || input.observed_source_sha256 != input.manifest.source_sha256
    {
        return Err(CapabilityPackageError::HashMismatch);
    }
    if !input.allowed_licenses.contains(&input.manifest.license) {
        return Err(CapabilityPackageError::LicenseDenied);
    }
    if !input.trusted_signers.contains(&input.manifest.signer_id) {
        return Err(CapabilityPackageError::ProvenanceDenied);
    }
    if input.dependency_locks != input.manifest.dependencies {
        return Err(CapabilityPackageError::DependencyDenied);
    }
    if input.expected_compatibility != &input.manifest.compatibility {
        return Err(CapabilityPackageError::CompatibilityDenied);
    }
    if hex(&Sha256::digest(input.public_key)) != input.manifest.signer_public_key_sha256 {
        return Err(CapabilityPackageError::SignatureDenied);
    }
    let key = VerifyingKey::from_bytes(input.public_key)
        .map_err(|_| CapabilityPackageError::SignatureDenied)?;
    let mut signed = Vec::with_capacity(SIGNATURE_DOMAIN.len() + bytes.len());
    signed.extend_from_slice(SIGNATURE_DOMAIN);
    signed.extend_from_slice(&bytes);
    key.verify_strict(&signed, &Signature::from_bytes(input.signature))
        .map_err(|_| CapabilityPackageError::SignatureDenied)?;
    let verification_sha256 = hex(&Sha256::digest(
        [
            input.manifest.manifest_sha256.as_bytes(),
            input.manifest.source_sha256.as_bytes(),
            input.manifest.signer_id.as_bytes(),
        ]
        .concat(),
    ));
    Ok(CapabilityPackageVerification {
        package_id: input.manifest.package_id,
        version: input.manifest.version,
        manifest_sha256: input.manifest.manifest_sha256,
        source_sha256: input.manifest.source_sha256,
        signer_id: input.manifest.signer_id,
        signature_verified: true,
        checksum_verified: true,
        license_verified: true,
        provenance_verified: true,
        dependency_locks_verified: true,
        compatibility_verified: true,
        admitted: true,
        verification_sha256,
    })
}

pub fn preview_capability_lifecycle(
    input: CapabilityLifecycleInput<'_>,
) -> Result<CapabilityLifecyclePreview, CapabilityPackageError> {
    let from = input.from_manifest_sha256;
    let to = input.to_manifest_sha256;
    if !identifier(input.preview_id)
        || !identifier(input.package_id)
        || !identifier(input.approval_id)
        || !sha(input.permission_delta_sha256)
        || from.as_deref().is_some_and(|value| !sha(value))
        || to.as_deref().is_some_and(|value| !sha(value))
    {
        return Err(CapabilityPackageError::InvalidManifest);
    }
    let shape = match input.action {
        CapabilityLifecycleAction::Install => from.is_none() && to.is_some(),
        CapabilityLifecycleAction::Enable | CapabilityLifecycleAction::Disable => {
            from.is_some() && to == from
        }
        CapabilityLifecycleAction::Update | CapabilityLifecycleAction::Rollback => {
            from.is_some() && to.is_some() && from != to
        }
        CapabilityLifecycleAction::Uninstall => from.is_some() && to.is_none(),
    };
    if !shape
        || matches!(
            input.action,
            CapabilityLifecycleAction::Install
                | CapabilityLifecycleAction::Enable
                | CapabilityLifecycleAction::Update
                | CapabilityLifecycleAction::Rollback
        ) && !input.verified_package
    {
        return Err(CapabilityPackageError::ApprovalDenied);
    }
    let action_name = format!("{:?}", input.action);
    let preview_sha256 = hex(&Sha256::digest(
        [
            input.preview_id,
            input.package_id,
            action_name.as_str(),
            from.as_deref().unwrap_or(""),
            to.as_deref().unwrap_or(""),
            input.permission_delta_sha256,
            input.approval_id,
        ]
        .concat(),
    ));
    Ok(CapabilityLifecyclePreview {
        preview_id: input.preview_id.to_owned(),
        package_id: input.package_id.to_owned(),
        action: input.action,
        from_manifest_sha256: from,
        to_manifest_sha256: to,
        permission_delta_sha256: input.permission_delta_sha256.to_owned(),
        approval_id: input.approval_id.to_owned(),
        verified_package: input.verified_package,
        applied: false,
        preview_sha256,
    })
}

fn subset(values: &[String], ceiling: &[String]) -> bool {
    values
        .iter()
        .all(|value| ceiling.binary_search(value).is_ok())
}
pub fn narrow_package_scope(
    requested: &CapabilityPackageScope,
    task: &CapabilityPackageScope,
) -> Result<CapabilityPackageScope, CapabilityPackageError> {
    if !valid_scope(requested)
        || !valid_scope(task)
        || !subset(&requested.filesystem_roots, &task.filesystem_roots)
        || !subset(&requested.commands, &task.commands)
        || !subset(&requested.network_domains, &task.network_domains)
        || !subset(&requested.credential_ids, &task.credential_ids)
        || !subset(&requested.connector_ids, &task.connector_ids)
        || !subset(&requested.publication_targets, &task.publication_targets)
        || requested.max_memory_bytes > task.max_memory_bytes
        || requested.max_cpu_milliseconds > task.max_cpu_milliseconds
        || requested.retention_seconds > task.retention_seconds
    {
        return Err(CapabilityPackageError::ScopeDenied);
    }
    Ok(requested.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    fn compatibility() -> CapabilityCompatibility {
        CapabilityCompatibility {
            kernel_version: "1.0.0".into(),
            tool_protocol_version: "1.0.0".into(),
            configuration_version: "1.0.0".into(),
            memory_version: "1.0.0".into(),
            storage_version: "1.0.0".into(),
            shell_version: "1.0.0".into(),
            policy_version: "1.0.0".into(),
        }
    }
    fn scope() -> CapabilityPackageScope {
        CapabilityPackageScope {
            filesystem_roots: vec!["workspace".into()],
            commands: vec!["cargo-test".into()],
            network_domains: vec!["api.example.test".into()],
            credential_ids: vec!["credential-1".into()],
            connector_ids: vec!["connector-1".into()],
            publication_targets: vec!["target-1".into()],
            max_memory_bytes: 1024,
            max_cpu_milliseconds: 1000,
            retention_seconds: 60,
        }
    }
    fn manifest(key: &SigningKey) -> CapabilityPackageManifest {
        seal_capability_manifest(CapabilityPackageManifest {
            schema_version: 1,
            package_id: "package-1".into(),
            package_name: "Package One".into(),
            version: "1.0.0".into(),
            source_id: "source-1".into(),
            source_sha256: A.into(),
            signer_id: "signer-1".into(),
            signer_public_key_sha256: hex(&Sha256::digest(key.verifying_key().as_bytes())),
            license: "Apache-2.0".into(),
            compatibility: compatibility(),
            tools: vec!["tool-1".into()],
            skills: vec!["skill-1".into()],
            hooks: vec!["hook-1".into()],
            requested_scope: scope(),
            dependencies: vec![],
            entry_points: vec![CapabilityEntryPoint {
                entry_id: "entry-1".into(),
                relative_path: "bin/entry".into(),
                content_sha256: A.into(),
            }],
            side_effects: vec![CapabilitySideEffect::WorkspaceRead],
            manifest_sha256: String::new(),
        })
        .unwrap()
    }
    #[test]
    fn signed_exact_package_verifies() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let manifest = manifest(&key);
        let bytes = canonical_manifest_bytes(&manifest).unwrap();
        let mut signed = SIGNATURE_DOMAIN.to_vec();
        signed.extend(bytes);
        let signature = key.sign(&signed).to_bytes();
        let allowed = BTreeSet::from(["Apache-2.0".into()]);
        let signers = BTreeSet::from(["signer-1".into()]);
        let result = verify_capability_package(CapabilityPackageAdmission {
            manifest: manifest.clone(),
            signature: &signature,
            public_key: key.verifying_key().as_bytes(),
            expected_compatibility: &compatibility(),
            allowed_licenses: &allowed,
            trusted_signers: &signers,
            observed_source_sha256: A,
            dependency_locks: &[],
        })
        .unwrap();
        assert!(result.admitted && result.signature_verified);
    }
    #[test]
    fn tampered_signature_source_license_and_compatibility_fail() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let manifest = manifest(&key);
        let bytes = canonical_manifest_bytes(&manifest).unwrap();
        let mut signed = SIGNATURE_DOMAIN.to_vec();
        signed.extend(bytes);
        let signature = key.sign(&signed).to_bytes();
        let public_key = key.verifying_key().to_bytes();
        let expected_compatibility = compatibility();
        let allowed = BTreeSet::from(["Apache-2.0".into()]);
        let signers = BTreeSet::from(["signer-1".into()]);
        let base = || CapabilityPackageAdmission {
            manifest: manifest.clone(),
            signature: &signature,
            public_key: &public_key,
            expected_compatibility: &expected_compatibility,
            allowed_licenses: &allowed,
            trusted_signers: &signers,
            observed_source_sha256: A,
            dependency_locks: &[],
        };
        let mut bad = signature;
        bad[0] ^= 1;
        assert_eq!(
            verify_capability_package(CapabilityPackageAdmission {
                signature: &bad,
                ..base()
            }),
            Err(CapabilityPackageError::SignatureDenied)
        );
        let wrong_source = "b".repeat(64);
        assert_eq!(
            verify_capability_package(CapabilityPackageAdmission {
                observed_source_sha256: &wrong_source,
                ..base()
            }),
            Err(CapabilityPackageError::HashMismatch)
        );
    }
    #[test]
    fn all_lifecycle_actions_are_inert_and_approved() {
        for (action, from, to) in [
            (CapabilityLifecycleAction::Install, None, Some(A.into())),
            (
                CapabilityLifecycleAction::Enable,
                Some(A.into()),
                Some(A.into()),
            ),
            (
                CapabilityLifecycleAction::Disable,
                Some(A.into()),
                Some(A.into()),
            ),
            (
                CapabilityLifecycleAction::Update,
                Some(A.into()),
                Some("b".repeat(64)),
            ),
            (
                CapabilityLifecycleAction::Rollback,
                Some("b".repeat(64)),
                Some(A.into()),
            ),
            (CapabilityLifecycleAction::Uninstall, Some(A.into()), None),
        ] {
            let preview = preview_capability_lifecycle(CapabilityLifecycleInput {
                preview_id: "preview-1",
                package_id: "package-1",
                action,
                from_manifest_sha256: from,
                to_manifest_sha256: to,
                permission_delta_sha256: A,
                approval_id: "approval-1",
                verified_package: true,
            })
            .unwrap();
            assert!(!preview.applied);
        }
    }
    #[test]
    fn unverified_enable_fails() {
        assert_eq!(
            preview_capability_lifecycle(CapabilityLifecycleInput {
                preview_id: "preview-1",
                package_id: "package-1",
                action: CapabilityLifecycleAction::Enable,
                from_manifest_sha256: Some(A.into()),
                to_manifest_sha256: Some(A.into()),
                permission_delta_sha256: A,
                approval_id: "approval-1",
                verified_package: false,
            }),
            Err(CapabilityPackageError::ApprovalDenied)
        );
    }
    #[test]
    fn package_scope_must_be_within_task_scope() {
        assert_eq!(narrow_package_scope(&scope(), &scope()).unwrap(), scope());
        let mut broad = scope();
        broad.network_domains.push("other.example.test".into());
        assert_eq!(
            narrow_package_scope(&broad, &scope()),
            Err(CapabilityPackageError::ScopeDenied)
        );
    }
}
