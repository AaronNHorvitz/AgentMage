//! Fail-closed Windows workspace, worker, key, model, and provider contracts.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceReceipt {
    pub root_file_id_sha256: String,
    pub target_file_id_sha256: String,
    pub normalized_relative_sha256: String,
    pub handle_relative: bool,
    pub final_path_rechecked: bool,
    pub reparse_count: u32,
    pub hard_link_count: u32,
    pub cloud_placeholder_count: u32,
    pub namespace_escape_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestrictedWorker {
    pub workspace_handle_sha256: String,
    pub grant_sha256: String,
    pub scratch_limit_bytes: u64,
    pub process_limit: u32,
    pub wall_time_ms: u64,
    pub restricted_token: bool,
    pub job_object: bool,
    pub mitigations: bool,
    pub descendants_terminated: bool,
    pub residue_count: u32,
    pub network_authority_count: u32,
    pub ambient_authority_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtectedState {
    pub dpapi_provider_sha256: String,
    pub credential_reference_sha256: String,
    pub data_root_file_id_sha256: String,
    pub dpapi_protected: bool,
    pub credential_is_reference: bool,
    pub fixed_local_ntfs: bool,
    pub synchronized_or_remote: bool,
    pub secret_material_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeModelWorker {
    pub binary_sha256: String,
    pub signature_sha256: String,
    pub model_sha256: String,
    pub profile_sha256: String,
    pub cpu_profile: String,
    pub gpu_profile: String,
    pub signed: bool,
    pub listener_count: u32,
    pub authority_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderWorker {
    pub destination_sha256: String,
    pub account_sha256: String,
    pub capability_sha256: String,
    pub credential_reference_sha256: String,
    pub operation_sha256: String,
    pub byte_limit: u64,
    pub wall_time_ms: u64,
    pub confined: bool,
    pub removed: bool,
    pub residual_authority_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeBoundaryError {
    Workspace,
    Worker,
    State,
    Model,
    Provider,
}

pub fn validate_workspace(v: &WorkspaceReceipt) -> Result<(), RuntimeBoundaryError> {
    if [
        &v.root_file_id_sha256,
        &v.target_file_id_sha256,
        &v.normalized_relative_sha256,
    ]
    .into_iter()
    .any(|value| !valid_sha(value))
        || !v.handle_relative
        || !v.final_path_rechecked
        || v.reparse_count
            + v.hard_link_count
            + v.cloud_placeholder_count
            + v.namespace_escape_count
            != 0
    {
        return Err(RuntimeBoundaryError::Workspace);
    }
    Ok(())
}

pub fn validate_worker(v: &RestrictedWorker) -> Result<(), RuntimeBoundaryError> {
    if [&v.workspace_handle_sha256, &v.grant_sha256]
        .into_iter()
        .any(|value| !valid_sha(value))
        || v.scratch_limit_bytes == 0
        || v.scratch_limit_bytes > 1_073_741_824
        || v.process_limit == 0
        || v.process_limit > 64
        || v.wall_time_ms == 0
        || v.wall_time_ms > 3_600_000
        || !v.restricted_token
        || !v.job_object
        || !v.mitigations
        || !v.descendants_terminated
        || v.residue_count + v.network_authority_count + v.ambient_authority_count != 0
    {
        return Err(RuntimeBoundaryError::Worker);
    }
    Ok(())
}

pub fn validate_state(v: &ProtectedState) -> Result<(), RuntimeBoundaryError> {
    if [
        &v.dpapi_provider_sha256,
        &v.credential_reference_sha256,
        &v.data_root_file_id_sha256,
    ]
    .into_iter()
    .any(|value| !valid_sha(value))
        || !v.dpapi_protected
        || !v.credential_is_reference
        || !v.fixed_local_ntfs
        || v.synchronized_or_remote
        || v.secret_material_count != 0
    {
        return Err(RuntimeBoundaryError::State);
    }
    Ok(())
}

pub fn validate_model(v: &NativeModelWorker) -> Result<(), RuntimeBoundaryError> {
    if [
        &v.binary_sha256,
        &v.signature_sha256,
        &v.model_sha256,
        &v.profile_sha256,
    ]
    .into_iter()
    .any(|value| !valid_sha(value))
        || !valid_id(&v.cpu_profile)
        || !valid_id(&v.gpu_profile)
        || !v.signed
        || v.listener_count + v.authority_count != 0
    {
        return Err(RuntimeBoundaryError::Model);
    }
    Ok(())
}

pub fn validate_provider(v: &ProviderWorker) -> Result<(), RuntimeBoundaryError> {
    if [
        &v.destination_sha256,
        &v.account_sha256,
        &v.capability_sha256,
        &v.credential_reference_sha256,
        &v.operation_sha256,
    ]
    .into_iter()
    .any(|value| !valid_sha(value))
        || v.byte_limit == 0
        || v.byte_limit > 1_073_741_824
        || v.wall_time_ms == 0
        || v.wall_time_ms > 3_600_000
        || !v.confined
        || !v.removed
        || v.residual_authority_count != 0
    {
        return Err(RuntimeBoundaryError::Provider);
    }
    Ok(())
}

fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 128
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha() -> String {
        "a".repeat(64)
    }

    #[test]
    fn workspace_rejects_identity_and_namespace_escape() {
        let mut v = WorkspaceReceipt {
            root_file_id_sha256: sha(),
            target_file_id_sha256: sha(),
            normalized_relative_sha256: sha(),
            handle_relative: true,
            final_path_rechecked: true,
            reparse_count: 0,
            hard_link_count: 0,
            cloud_placeholder_count: 0,
            namespace_escape_count: 0,
        };
        assert_eq!(validate_workspace(&v), Ok(()));
        v.reparse_count = 1;
        assert_eq!(validate_workspace(&v), Err(RuntimeBoundaryError::Workspace));
    }

    #[test]
    fn restricted_worker_has_no_ambient_or_network_authority() {
        let mut v = RestrictedWorker {
            workspace_handle_sha256: sha(),
            grant_sha256: sha(),
            scratch_limit_bytes: 1,
            process_limit: 1,
            wall_time_ms: 1,
            restricted_token: true,
            job_object: true,
            mitigations: true,
            descendants_terminated: true,
            residue_count: 0,
            network_authority_count: 0,
            ambient_authority_count: 0,
        };
        assert_eq!(validate_worker(&v), Ok(()));
        v.ambient_authority_count = 1;
        assert_eq!(validate_worker(&v), Err(RuntimeBoundaryError::Worker));
    }

    #[test]
    fn state_and_model_fail_closed() {
        let state = ProtectedState {
            dpapi_provider_sha256: sha(),
            credential_reference_sha256: sha(),
            data_root_file_id_sha256: sha(),
            dpapi_protected: true,
            credential_is_reference: true,
            fixed_local_ntfs: true,
            synchronized_or_remote: false,
            secret_material_count: 0,
        };
        assert_eq!(validate_state(&state), Ok(()));
        let mut model = NativeModelWorker {
            binary_sha256: sha(),
            signature_sha256: sha(),
            model_sha256: sha(),
            profile_sha256: sha(),
            cpu_profile: "cpu-avx2".into(),
            gpu_profile: "gpu-none".into(),
            signed: true,
            listener_count: 0,
            authority_count: 0,
        };
        assert_eq!(validate_model(&model), Ok(()));
        model.listener_count = 1;
        assert_eq!(validate_model(&model), Err(RuntimeBoundaryError::Model));
    }

    #[test]
    fn provider_removal_is_bounded_and_complete() {
        let mut v = ProviderWorker {
            destination_sha256: sha(),
            account_sha256: sha(),
            capability_sha256: sha(),
            credential_reference_sha256: sha(),
            operation_sha256: sha(),
            byte_limit: 1,
            wall_time_ms: 1,
            confined: true,
            removed: true,
            residual_authority_count: 0,
        };
        assert_eq!(validate_provider(&v), Ok(()));
        v.removed = false;
        assert_eq!(validate_provider(&v), Err(RuntimeBoundaryError::Provider));
    }
}
