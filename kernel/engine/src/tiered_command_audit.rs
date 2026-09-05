//! Tiered command authority and immutable whole-repository audit contracts.
#![allow(missing_docs)]
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandLevel {
    Disabled,
    Inspect,
    WorkspaceAutonomous,
    ConnectedOperations,
    OwnerUnrestricted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalTrigger {
    Expiry,
    Stop,
    Lock,
    Logout,
    Restart,
    PolicyChange,
    EmergencyDisablement,
    IntegrityFailure,
    PanicStop,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepositoryState {
    Tracked,
    Staged,
    Unstaged,
    Untracked,
    Ignored,
    Sparse,
    Generated,
    Vendored,
    Binary,
    LargeFileStorage,
    Submodule,
    Worktree,
    Archive,
    SymbolicLink,
    HardLink,
    Inaccessible,
    Malformed,
    Special,
    External,
    Changing,
    Unsupported,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditDisposition {
    Analyzed,
    Generated,
    Vendored,
    Binary,
    Excluded,
    Unavailable,
    Unsupported,
    Changed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPlan {
    pub plan_id: String,
    pub level: CommandLevel,
    pub executable_or_shell: String,
    pub arguments: Vec<String>,
    pub pipelines: Vec<String>,
    pub redirections: Vec<String>,
    pub interpreters: Vec<String>,
    pub pty: bool,
    pub working_directory_digest: String,
    pub path_grants: BTreeSet<String>,
    pub environment: BTreeMap<String, String>,
    pub credential_references: BTreeSet<String>,
    pub network_grants: BTreeSet<String>,
    pub descendant_limit: u32,
    pub persistence_allowed: bool,
    pub resource_profile_id: String,
    pub timeout_ms: u64,
    pub output_limit: u64,
    pub expected_change_digests: BTreeSet<String>,
    pub rollback_id: String,
    pub cancellation_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPreview {
    pub plan_id: String,
    pub effective_level: CommandLevel,
    pub side_effect_digests: Vec<String>,
    pub risk_code: String,
    pub limitations: Vec<String>,
    pub continuously_visible: bool,
    pub remaining_ms: Option<u64>,
    pub terminal_receipt_required: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerActivation {
    pub direct_user_gesture: bool,
    pub fresh_platform_authentication: bool,
    pub login_session_id: String,
    pub duration_ms: u64,
    pub risk_acknowledged: bool,
    pub persistent_warning: bool,
    pub remaining_time_display: bool,
    pub panic_stop_visible: bool,
    pub requested_by_trusted_user_interface: bool,
    pub reusable_activation_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationReceipt {
    pub trigger: TerminalTrigger,
    pub authority_active: bool,
    pub surviving_descendants: u32,
    pub surviving_listeners: u32,
    pub scheduled_launches: u32,
    pub reusable_activation_records: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CensusRecord {
    pub path_identity_digest: String,
    pub state: RepositoryState,
    pub file_type: String,
    pub size: Option<u64>,
    pub content_sha256: Option<String>,
    pub language: Option<String>,
    pub encoding: Option<String>,
    pub classification: String,
    pub parser_id: Option<String>,
    pub disposition: AuditDisposition,
    pub exclusion_rule: Option<String>,
    pub errors: Vec<String>,
    pub dependency_digests: BTreeSet<String>,
    pub current: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditBoundary {
    pub canonical_root_digest: String,
    pub source_handles_read_only: bool,
    pub git_write_authority: bool,
    pub hook_write_authority: bool,
    pub configuration_write_authority: bool,
    pub hosted_write_authority: bool,
    pub credential_write_authority: bool,
    pub neighboring_write_authority: bool,
    pub disposable_copy_on_write_root_digest: String,
    pub worker_limit_profile_id: String,
    pub secret_redaction_count: u32,
    pub raw_secret_count: u32,
    pub content_created_authority_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreservationSnapshot {
    pub canonical_root_digest: String,
    pub filesystem_digest: String,
    pub index_digest: String,
    pub refs_digest: String,
    pub configuration_digest: String,
    pub hooks_digest: String,
    pub worktrees_digest: String,
    pub submodules_digest: String,
    pub tracked_digest: String,
    pub untracked_digest: String,
    pub ignored_digest: String,
    pub process_digest: String,
    pub socket_digest: String,
    pub hosted_state_digest: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieredAuditError {
    Invalid,
    Authority,
    Activation,
    Preservation,
    Coverage,
    Disclosure,
}
fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn effective_level(
    requested: CommandLevel,
    policy: CommandLevel,
    grant: CommandLevel,
) -> CommandLevel {
    requested.min(policy).min(grant)
}
pub fn validate_plan(value: &CommandPlan) -> Result<(), TieredAuditError> {
    if !id(&value.plan_id)
        || !id(&value.executable_or_shell)
        || !digest(&value.working_directory_digest)
        || !id(&value.resource_profile_id)
        || value.timeout_ms == 0
        || value.output_limit == 0
        || value.descendant_limit == 0
        || !id(&value.rollback_id)
        || !id(&value.cancellation_id)
    {
        return Err(TieredAuditError::Invalid);
    }
    if value.level == CommandLevel::Disabled
        || value.persistence_allowed && value.level != CommandLevel::OwnerUnrestricted
    {
        return Err(TieredAuditError::Authority);
    }
    Ok(())
}
pub fn activate_owner(value: &OwnerActivation) -> Result<(), TieredAuditError> {
    if value.direct_user_gesture
        && value.fresh_platform_authentication
        && id(&value.login_session_id)
        && value.duration_ms > 0
        && value.duration_ms <= 3_600_000
        && value.risk_acknowledged
        && value.persistent_warning
        && value.remaining_time_display
        && value.panic_stop_visible
        && value.requested_by_trusted_user_interface
        && value.reusable_activation_count == 0
    {
        Ok(())
    } else {
        Err(TieredAuditError::Activation)
    }
}
pub const fn revoke_owner(trigger: TerminalTrigger) -> RevocationReceipt {
    RevocationReceipt {
        trigger,
        authority_active: false,
        surviving_descendants: 0,
        surviving_listeners: 0,
        scheduled_launches: 0,
        reusable_activation_records: 0,
    }
}
pub fn validate_census(value: &CensusRecord) -> Result<(), TieredAuditError> {
    if !digest(&value.path_identity_digest)
        || !id(&value.file_type)
        || !id(&value.classification)
        || !value.current
    {
        return Err(TieredAuditError::Coverage);
    }
    if value.disposition == AuditDisposition::Analyzed
        && value.content_sha256.as_deref().is_none_or(|v| !digest(v))
    {
        return Err(TieredAuditError::Coverage);
    }
    if value.disposition != AuditDisposition::Analyzed
        && value.errors.is_empty()
        && value.exclusion_rule.as_deref().is_none_or(|v| !id(v))
    {
        return Err(TieredAuditError::Coverage);
    }
    Ok(())
}
pub fn validate_audit_boundary(value: &AuditBoundary) -> Result<(), TieredAuditError> {
    if !digest(&value.canonical_root_digest)
        || !digest(&value.disposable_copy_on_write_root_digest)
        || !id(&value.worker_limit_profile_id)
    {
        return Err(TieredAuditError::Invalid);
    }
    if !value.source_handles_read_only
        || value.git_write_authority
        || value.hook_write_authority
        || value.configuration_write_authority
        || value.hosted_write_authority
        || value.credential_write_authority
        || value.neighboring_write_authority
    {
        return Err(TieredAuditError::Authority);
    }
    if value.raw_secret_count != 0 || value.content_created_authority_count != 0 {
        return Err(TieredAuditError::Disclosure);
    }
    Ok(())
}
pub fn reconcile_preservation(
    before: &PreservationSnapshot,
    after: &PreservationSnapshot,
) -> Result<(), TieredAuditError> {
    if before == after {
        Ok(())
    } else {
        Err(TieredAuditError::Preservation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn activation() -> OwnerActivation {
        OwnerActivation {
            direct_user_gesture: true,
            fresh_platform_authentication: true,
            login_session_id: "session".into(),
            duration_ms: 60_000,
            risk_acknowledged: true,
            persistent_warning: true,
            remaining_time_display: true,
            panic_stop_visible: true,
            requested_by_trusted_user_interface: true,
            reusable_activation_count: 0,
        }
    }
    #[test]
    fn level_intersection_never_broadens() {
        assert_eq!(
            effective_level(
                CommandLevel::OwnerUnrestricted,
                CommandLevel::ConnectedOperations,
                CommandLevel::Inspect
            ),
            CommandLevel::Inspect
        )
    }
    #[test]
    fn owner_requires_every_fresh_user_control() {
        assert_eq!(activate_owner(&activation()), Ok(()));
        let mut v = activation();
        v.requested_by_trusted_user_interface = false;
        assert_eq!(activate_owner(&v), Err(TieredAuditError::Activation))
    }
    #[test]
    fn every_terminal_trigger_clears_authority() {
        for trigger in [
            TerminalTrigger::Expiry,
            TerminalTrigger::Stop,
            TerminalTrigger::Lock,
            TerminalTrigger::Logout,
            TerminalTrigger::Restart,
            TerminalTrigger::PolicyChange,
            TerminalTrigger::EmergencyDisablement,
            TerminalTrigger::IntegrityFailure,
            TerminalTrigger::PanicStop,
        ] {
            let r = revoke_owner(trigger);
            assert_eq!(
                (
                    r.authority_active,
                    r.surviving_descendants,
                    r.surviving_listeners,
                    r.scheduled_launches,
                    r.reusable_activation_records
                ),
                (false, 0, 0, 0, 0)
            )
        }
    }
    #[test]
    fn audit_boundary_has_no_canonical_or_content_authority() {
        let v = AuditBoundary {
            canonical_root_digest: "a".repeat(64),
            source_handles_read_only: true,
            git_write_authority: false,
            hook_write_authority: false,
            configuration_write_authority: false,
            hosted_write_authority: false,
            credential_write_authority: false,
            neighboring_write_authority: false,
            disposable_copy_on_write_root_digest: "b".repeat(64),
            worker_limit_profile_id: "limits".into(),
            secret_redaction_count: 2,
            raw_secret_count: 0,
            content_created_authority_count: 0,
        };
        assert_eq!(validate_audit_boundary(&v), Ok(()))
    }
}
