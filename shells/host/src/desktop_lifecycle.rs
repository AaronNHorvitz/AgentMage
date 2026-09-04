//! Effect-free desktop status, recovery, protocol, and local-package contracts.
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopStatusSnapshot {
    pub snapshot_id: String,
    pub model_sha256: String,
    pub runtime_sha256: String,
    pub context_sha256: String,
    pub memory_sha256: String,
    pub plan_sha256: String,
    pub tools_sha256: String,
    pub resources_sha256: String,
    pub audit_sha256: String,
    pub offline: bool,
    pub display_only: bool,
    pub status_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopWriterLease {
    pub lease_id: String,
    pub conversation_id: String,
    pub owner_instance_id: String,
    pub generation: u64,
    pub acquired_at_epoch_ms: u64,
    pub released: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopCheckpointPhase {
    Prepared,
    CanonicalCommitted,
    ReceiptCommitted,
    Complete,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopRecoveryCheckpoint {
    pub checkpoint_id: String,
    pub conversation_id: String,
    pub phase: DesktopCheckpointPhase,
    pub journal_sequence: u64,
    pub canonical_state_sha256: String,
    pub effect_sha256: Option<String>,
    pub receipt_sha256: Option<String>,
    pub clean_shutdown: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopRecoveryMode {
    ResumeProjection,
    ReadOnlySafeMode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopRecoveryDecision {
    pub checkpoint_id: String,
    pub mode: DesktopRecoveryMode,
    pub canonical_state_unchanged: bool,
    pub duplicate_effect_possible: bool,
    pub writer_lock_reacquired: bool,
    pub reason_code: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopLocalAssetKind {
    Font,
    Icon,
    Theme,
    Help,
    UpdateMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopLocalAsset {
    pub asset_id: String,
    pub kind: DesktopLocalAssetKind,
    pub relative_path: String,
    pub content_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopPackageContract {
    pub package_id: String,
    pub target_platform: String,
    pub assets: Vec<DesktopLocalAsset>,
    pub telemetry_enabled: bool,
    pub advertisements_enabled: bool,
    pub remote_assets_enabled: bool,
    pub automatic_cloud_checks_enabled: bool,
    pub network_required_for_startup: bool,
    pub signed_package_present: bool,
    pub installed_package_tested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopProtocolOperation {
    Message,
    EventCursor,
    Cancel,
    ApprovalResponse,
    DeepLink,
    FilePicker,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopProtocolProjection {
    pub projection_id: String,
    pub operation: DesktopProtocolOperation,
    pub request_sha256: String,
    pub kernel_protocol_forwarded: bool,
    pub direct_file_access: bool,
    pub direct_model_access: bool,
    pub direct_key_access: bool,
    pub direct_tool_access: bool,
    pub hidden_network: bool,
    pub unsafe_open: bool,
    pub drag_drop_escape: bool,
    pub clipboard_leakage: bool,
    pub approval_bypass: bool,
    pub authority_granted: bool,
    pub effect_applied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopLifecycleError {
    InvalidInput,
    WriterConflict,
    BoundaryViolation,
}

impl DesktopLifecycleError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "desktop-lifecycle.input.invalid",
            Self::WriterConflict => "desktop-lifecycle.writer.conflict",
            Self::BoundaryViolation => "desktop-lifecycle.boundary.violation",
        }
    }
}
impl std::fmt::Display for DesktopLifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for DesktopLifecycleError {}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 2_048
        && !value.starts_with('/')
        && !value.contains(['\\', '\0'])
        && !value
            .split('/')
            .any(|component| matches!(component, "" | "." | ".."))
}

pub fn validate_status_snapshot(
    snapshot: &DesktopStatusSnapshot,
) -> Result<(), DesktopLifecycleError> {
    if !identifier(&snapshot.snapshot_id)
        || ![
            &snapshot.model_sha256,
            &snapshot.runtime_sha256,
            &snapshot.context_sha256,
            &snapshot.memory_sha256,
            &snapshot.plan_sha256,
            &snapshot.tools_sha256,
            &snapshot.resources_sha256,
            &snapshot.audit_sha256,
            &snapshot.status_sha256,
        ]
        .into_iter()
        .all(|value| sha256(value))
        || !snapshot.display_only
    {
        return Err(DesktopLifecycleError::BoundaryViolation);
    }
    Ok(())
}

pub fn admit_writer_lease(
    active: Option<&DesktopWriterLease>,
    candidate: DesktopWriterLease,
) -> Result<DesktopWriterLease, DesktopLifecycleError> {
    if !identifier(&candidate.lease_id)
        || !identifier(&candidate.conversation_id)
        || !identifier(&candidate.owner_instance_id)
        || candidate.generation == 0
        || candidate.acquired_at_epoch_ms == 0
        || candidate.released
    {
        return Err(DesktopLifecycleError::InvalidInput);
    }
    if active
        .is_some_and(|lease| !lease.released && lease.conversation_id == candidate.conversation_id)
    {
        return Err(DesktopLifecycleError::WriterConflict);
    }
    Ok(candidate)
}

pub fn recover_desktop_session(
    checkpoint: &DesktopRecoveryCheckpoint,
    writer_lock_reacquired: bool,
) -> Result<DesktopRecoveryDecision, DesktopLifecycleError> {
    if !identifier(&checkpoint.checkpoint_id)
        || !identifier(&checkpoint.conversation_id)
        || checkpoint.journal_sequence == 0
        || !sha256(&checkpoint.canonical_state_sha256)
        || checkpoint
            .effect_sha256
            .as_deref()
            .is_some_and(|value| !sha256(value))
        || checkpoint
            .receipt_sha256
            .as_deref()
            .is_some_and(|value| !sha256(value))
        || checkpoint.receipt_sha256.is_some() && checkpoint.effect_sha256.is_none()
    {
        return Err(DesktopLifecycleError::InvalidInput);
    }
    let complete = checkpoint.phase == DesktopCheckpointPhase::Complete
        && checkpoint.clean_shutdown
        && checkpoint.effect_sha256.is_some() == checkpoint.receipt_sha256.is_some();
    let safe_mode = !complete || !writer_lock_reacquired;
    Ok(DesktopRecoveryDecision {
        checkpoint_id: checkpoint.checkpoint_id.clone(),
        mode: if safe_mode {
            DesktopRecoveryMode::ReadOnlySafeMode
        } else {
            DesktopRecoveryMode::ResumeProjection
        },
        canonical_state_unchanged: true,
        duplicate_effect_possible: false,
        writer_lock_reacquired,
        reason_code: if safe_mode {
            "desktop-recovery.safe-mode".to_owned()
        } else {
            "desktop-recovery.clean-resume".to_owned()
        },
    })
}

pub fn validate_package_contract(
    package: &DesktopPackageContract,
) -> Result<(), DesktopLifecycleError> {
    let kinds: BTreeSet<_> = package.assets.iter().map(|asset| asset.kind).collect();
    let required = BTreeSet::from([
        DesktopLocalAssetKind::Font,
        DesktopLocalAssetKind::Icon,
        DesktopLocalAssetKind::Theme,
        DesktopLocalAssetKind::Help,
        DesktopLocalAssetKind::UpdateMetadata,
    ]);
    if !identifier(&package.package_id)
        || !matches!(
            package.target_platform.as_str(),
            "linux-x86_64" | "macos-arm64"
        )
        || package.assets.is_empty()
        || package.assets.len() > 256
        || kinds != required
        || package.assets.iter().any(|asset| {
            !identifier(&asset.asset_id)
                || !relative_path(&asset.relative_path)
                || !sha256(&asset.content_sha256)
        })
        || package.telemetry_enabled
        || package.advertisements_enabled
        || package.remote_assets_enabled
        || package.automatic_cloud_checks_enabled
        || package.network_required_for_startup
        || package.signed_package_present
        || package.installed_package_tested
    {
        return Err(DesktopLifecycleError::BoundaryViolation);
    }
    Ok(())
}

pub fn validate_protocol_projection(
    projection: &DesktopProtocolProjection,
) -> Result<(), DesktopLifecycleError> {
    if !identifier(&projection.projection_id) || !sha256(&projection.request_sha256) {
        return Err(DesktopLifecycleError::InvalidInput);
    }
    if !projection.kernel_protocol_forwarded
        || projection.direct_file_access
        || projection.direct_model_access
        || projection.direct_key_access
        || projection.direct_tool_access
        || projection.hidden_network
        || projection.unsafe_open
        || projection.drag_drop_escape
        || projection.clipboard_leakage
        || projection.approval_bypass
        || projection.authority_granted
        || projection.effect_applied
    {
        return Err(DesktopLifecycleError::BoundaryViolation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn status() -> DesktopStatusSnapshot {
        DesktopStatusSnapshot {
            snapshot_id: "status-1".to_owned(),
            model_sha256: SHA.to_owned(),
            runtime_sha256: SHA.to_owned(),
            context_sha256: SHA.to_owned(),
            memory_sha256: SHA.to_owned(),
            plan_sha256: SHA.to_owned(),
            tools_sha256: SHA.to_owned(),
            resources_sha256: SHA.to_owned(),
            audit_sha256: SHA.to_owned(),
            offline: true,
            display_only: true,
            status_sha256: SHA.to_owned(),
        }
    }

    fn checkpoint(
        phase: DesktopCheckpointPhase,
        clean_shutdown: bool,
    ) -> DesktopRecoveryCheckpoint {
        DesktopRecoveryCheckpoint {
            checkpoint_id: "checkpoint-1".to_owned(),
            conversation_id: "conversation-1".to_owned(),
            phase,
            journal_sequence: 1,
            canonical_state_sha256: SHA.to_owned(),
            effect_sha256: None,
            receipt_sha256: None,
            clean_shutdown,
        }
    }

    #[test]
    fn status_is_display_only_and_digest_bound() {
        assert_eq!(validate_status_snapshot(&status()), Ok(()));
        let mut invalid = status();
        invalid.display_only = false;
        assert_eq!(
            validate_status_snapshot(&invalid),
            Err(DesktopLifecycleError::BoundaryViolation)
        );
    }

    #[test]
    fn only_one_writer_lease_is_admitted() {
        let lease = DesktopWriterLease {
            lease_id: "lease-1".to_owned(),
            conversation_id: "conversation-1".to_owned(),
            owner_instance_id: "desktop-1".to_owned(),
            generation: 1,
            acquired_at_epoch_ms: 1,
            released: false,
        };
        assert_eq!(admit_writer_lease(None, lease.clone()).unwrap(), lease);
        let candidate = DesktopWriterLease {
            lease_id: "lease-2".to_owned(),
            owner_instance_id: "desktop-2".to_owned(),
            ..lease.clone()
        };
        assert_eq!(
            admit_writer_lease(Some(&lease), candidate),
            Err(DesktopLifecycleError::WriterConflict)
        );
    }

    #[test]
    fn clean_complete_checkpoint_can_resume() {
        let decision =
            recover_desktop_session(&checkpoint(DesktopCheckpointPhase::Complete, true), true)
                .unwrap();
        assert_eq!(decision.mode, DesktopRecoveryMode::ResumeProjection);
        assert!(decision.canonical_state_unchanged);
        assert!(!decision.duplicate_effect_possible);
    }

    #[test]
    fn uncertain_or_locked_checkpoint_enters_read_only_safe_mode() {
        for (phase, clean, lock) in [
            (DesktopCheckpointPhase::Uncertain, false, true),
            (DesktopCheckpointPhase::Complete, true, false),
        ] {
            let decision = recover_desktop_session(&checkpoint(phase, clean), lock).unwrap();
            assert_eq!(decision.mode, DesktopRecoveryMode::ReadOnlySafeMode);
            assert!(!decision.duplicate_effect_possible);
        }
    }

    fn assets() -> Vec<DesktopLocalAsset> {
        [
            DesktopLocalAssetKind::Font,
            DesktopLocalAssetKind::Icon,
            DesktopLocalAssetKind::Theme,
            DesktopLocalAssetKind::Help,
            DesktopLocalAssetKind::UpdateMetadata,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, kind)| DesktopLocalAsset {
            asset_id: format!("asset-{index}"),
            kind,
            relative_path: format!("assets/{index}.bin"),
            content_sha256: SHA.to_owned(),
        })
        .collect()
    }

    #[test]
    fn package_contract_requires_all_local_asset_classes() {
        let package = DesktopPackageContract {
            package_id: "desktop-package-1".to_owned(),
            target_platform: "linux-x86_64".to_owned(),
            assets: assets(),
            telemetry_enabled: false,
            advertisements_enabled: false,
            remote_assets_enabled: false,
            automatic_cloud_checks_enabled: false,
            network_required_for_startup: false,
            signed_package_present: false,
            installed_package_tested: false,
        };
        assert_eq!(validate_package_contract(&package), Ok(()));
        let mut invalid = package;
        invalid.remote_assets_enabled = true;
        assert_eq!(
            validate_package_contract(&invalid),
            Err(DesktopLifecycleError::BoundaryViolation)
        );
    }

    #[test]
    fn package_contract_cannot_claim_signing_or_installation() {
        let mut package = DesktopPackageContract {
            package_id: "desktop-package-1".to_owned(),
            target_platform: "macos-arm64".to_owned(),
            assets: assets(),
            telemetry_enabled: false,
            advertisements_enabled: false,
            remote_assets_enabled: false,
            automatic_cloud_checks_enabled: false,
            network_required_for_startup: false,
            signed_package_present: true,
            installed_package_tested: false,
        };
        assert!(validate_package_contract(&package).is_err());
        package.signed_package_present = false;
        package.installed_package_tested = true;
        assert!(validate_package_contract(&package).is_err());
    }

    #[test]
    fn all_protocol_operations_are_kernel_forwarded_and_inert() {
        for operation in [
            DesktopProtocolOperation::Message,
            DesktopProtocolOperation::EventCursor,
            DesktopProtocolOperation::Cancel,
            DesktopProtocolOperation::ApprovalResponse,
            DesktopProtocolOperation::DeepLink,
            DesktopProtocolOperation::FilePicker,
        ] {
            let projection = DesktopProtocolProjection {
                projection_id: "projection-1".to_owned(),
                operation,
                request_sha256: SHA.to_owned(),
                kernel_protocol_forwarded: true,
                direct_file_access: false,
                direct_model_access: false,
                direct_key_access: false,
                direct_tool_access: false,
                hidden_network: false,
                unsafe_open: false,
                drag_drop_escape: false,
                clipboard_leakage: false,
                approval_bypass: false,
                authority_granted: false,
                effect_applied: false,
            };
            assert_eq!(validate_protocol_projection(&projection), Ok(()));
        }
    }

    #[test]
    fn direct_access_or_effect_is_rejected() {
        let mut projection = DesktopProtocolProjection {
            projection_id: "projection-1".to_owned(),
            operation: DesktopProtocolOperation::FilePicker,
            request_sha256: SHA.to_owned(),
            kernel_protocol_forwarded: true,
            direct_file_access: true,
            direct_model_access: false,
            direct_key_access: false,
            direct_tool_access: false,
            hidden_network: false,
            unsafe_open: false,
            drag_drop_escape: false,
            clipboard_leakage: false,
            approval_bypass: false,
            authority_granted: false,
            effect_applied: false,
        };
        assert_eq!(
            validate_protocol_projection(&projection),
            Err(DesktopLifecycleError::BoundaryViolation)
        );
        projection.direct_file_access = false;
        projection.effect_applied = true;
        assert_eq!(
            validate_protocol_projection(&projection),
            Err(DesktopLifecycleError::BoundaryViolation)
        );
    }
}
