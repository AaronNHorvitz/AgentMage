//! Platform-neutral Windows package, IPC, and KVM evidence contracts.
#![allow(missing_docs)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageManifest {
    pub windows_build: String,
    pub vscode_version: String,
    pub sdk_version: String,
    pub package_identity_sha256: String,
    pub certificate_sha256: String,
    pub timestamp_sha256: String,
    pub component_manifest_sha256: String,
    pub dependency_manifest_sha256: String,
    pub per_user: bool,
    pub reproducible: bool,
    pub offline_verifiable: bool,
    pub service_count: u32,
    pub driver_count: u32,
    pub scheduled_task_count: u32,
    pub system_write_count: u32,
    pub policy_weakening_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpcPeer {
    pub user_sid_sha256: String,
    pub logon_session_sha256: String,
    pub integrity_sha256: String,
    pub executable_sha256: String,
    pub package_sha256: String,
    pub protocol_version: u16,
    pub sequence: u64,
    pub message_bytes: u32,
    pub challenge_sha256: String,
    pub replay_sha256: String,
    pub cancellation_sha256: String,
    pub pipe_acl_sha256: String,
    pub handle_inheritable: bool,
    pub authority_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KvmImageManifest {
    pub edition: String,
    pub build: String,
    pub source_sha256: String,
    pub image_digest: String,
    pub uefi_sha256: String,
    pub secure_boot: bool,
    pub virtual_tpm: bool,
    pub hardware_sha256: String,
    pub update_sha256: String,
    pub vscode_sha256: String,
    pub toolchain_sha256: String,
    pub standard_user_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayPlan {
    pub image_digest: String,
    pub source_revision: String,
    pub overlay_sha256: String,
    pub firmware_sha256: String,
    pub exposed_repository_credentials: u32,
    pub unrelated_host_paths: u32,
    pub dirty_guest: bool,
    pub undeclared_secrets: u32,
    pub evidence_retained_before_destroy: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsContractError {
    InvalidPackage,
    InvalidPeer,
    InvalidImage,
    InvalidOverlay,
}
pub fn validate_package(v: &PackageManifest) -> Result<(), WindowsContractError> {
    if !valid_id(&v.windows_build)
        || !valid_id(&v.vscode_version)
        || !valid_id(&v.sdk_version)
        || [
            &v.package_identity_sha256,
            &v.certificate_sha256,
            &v.timestamp_sha256,
            &v.component_manifest_sha256,
            &v.dependency_manifest_sha256,
        ]
        .into_iter()
        .any(|x| !valid_sha(x))
        || !v.per_user
        || !v.reproducible
        || !v.offline_verifiable
        || v.service_count
            + v.driver_count
            + v.scheduled_task_count
            + v.system_write_count
            + v.policy_weakening_count
            != 0
    {
        return Err(WindowsContractError::InvalidPackage);
    }
    Ok(())
}
pub fn validate_peer(v: &IpcPeer) -> Result<(), WindowsContractError> {
    if [
        &v.user_sid_sha256,
        &v.logon_session_sha256,
        &v.integrity_sha256,
        &v.executable_sha256,
        &v.package_sha256,
        &v.challenge_sha256,
        &v.replay_sha256,
        &v.cancellation_sha256,
        &v.pipe_acl_sha256,
    ]
    .into_iter()
    .any(|x| !valid_sha(x))
        || v.protocol_version != 1
        || v.sequence == 0
        || v.message_bytes == 0
        || v.message_bytes > 16 * 1024 * 1024
        || v.handle_inheritable
        || v.authority_count != 0
    {
        return Err(WindowsContractError::InvalidPeer);
    }
    Ok(())
}
pub fn validate_image(v: &KvmImageManifest) -> Result<(), WindowsContractError> {
    if !valid_id(&v.edition)
        || !valid_id(&v.build)
        || !valid_digest(&v.image_digest)
        || [
            &v.source_sha256,
            &v.uefi_sha256,
            &v.hardware_sha256,
            &v.update_sha256,
            &v.vscode_sha256,
            &v.toolchain_sha256,
            &v.standard_user_sha256,
        ]
        .into_iter()
        .any(|x| !valid_sha(x))
        || !v.secure_boot
        || !v.virtual_tpm
    {
        return Err(WindowsContractError::InvalidImage);
    }
    Ok(())
}
pub fn validate_overlay(v: &OverlayPlan) -> Result<(), WindowsContractError> {
    if !valid_digest(&v.image_digest)
        || v.source_revision.len() != 40
        || !v
            .source_revision
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || !valid_sha(&v.overlay_sha256)
        || !valid_sha(&v.firmware_sha256)
        || v.exposed_repository_credentials != 0
        || v.unrelated_host_paths != 0
        || v.dirty_guest
        || v.undeclared_secrets != 0
        || !v.evidence_retained_before_destroy
    {
        return Err(WindowsContractError::InvalidOverlay);
    }
    Ok(())
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 128
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn valid_digest(v: &str) -> bool {
    v.starts_with("sha256:") && valid_sha(&v[7..])
}
#[cfg(test)]
mod tests {
    use super::*;
    fn package() -> PackageManifest {
        let h = "a".repeat(64);
        PackageManifest {
            windows_build: "26100".into(),
            vscode_version: "1.100.0".into(),
            sdk_version: "10.0.26100".into(),
            package_identity_sha256: h.clone(),
            certificate_sha256: h.clone(),
            timestamp_sha256: h.clone(),
            component_manifest_sha256: h.clone(),
            dependency_manifest_sha256: h,
            per_user: true,
            reproducible: true,
            offline_verifiable: true,
            service_count: 0,
            driver_count: 0,
            scheduled_task_count: 0,
            system_write_count: 0,
            policy_weakening_count: 0,
        }
    }
    #[test]
    fn package_is_per_user_and_authority_free() {
        assert_eq!(validate_package(&package()), Ok(()));
    }
    #[test]
    fn system_effects_fail_closed() {
        let mut p = package();
        p.service_count = 1;
        assert_eq!(
            validate_package(&p),
            Err(WindowsContractError::InvalidPackage)
        );
    }
    #[test]
    fn peer_replay_and_handle_bounds_are_exact() {
        let h = "b".repeat(64);
        let mut p = IpcPeer {
            user_sid_sha256: h.clone(),
            logon_session_sha256: h.clone(),
            integrity_sha256: h.clone(),
            executable_sha256: h.clone(),
            package_sha256: h.clone(),
            protocol_version: 1,
            sequence: 1,
            message_bytes: 1,
            challenge_sha256: h.clone(),
            replay_sha256: h.clone(),
            cancellation_sha256: h.clone(),
            pipe_acl_sha256: h,
            handle_inheritable: false,
            authority_count: 0,
        };
        assert_eq!(validate_peer(&p), Ok(()));
        p.handle_inheritable = true;
        assert_eq!(validate_peer(&p), Err(WindowsContractError::InvalidPeer));
    }
    #[test]
    fn dirty_or_credentialed_overlay_is_refused() {
        let h = "c".repeat(64);
        let mut p = OverlayPlan {
            image_digest: format!("sha256:{h}"),
            source_revision: "d".repeat(40),
            overlay_sha256: h.clone(),
            firmware_sha256: h,
            exposed_repository_credentials: 0,
            unrelated_host_paths: 0,
            dirty_guest: false,
            undeclared_secrets: 0,
            evidence_retained_before_destroy: true,
        };
        assert_eq!(validate_overlay(&p), Ok(()));
        p.dirty_guest = true;
        assert_eq!(
            validate_overlay(&p),
            Err(WindowsContractError::InvalidOverlay)
        );
    }
}
