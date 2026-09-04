//! Pure contracts for separately threat-modeled, predeclared scheduled effects.

use serde::{Deserialize, Serialize};

/// Closed repeatability classification for scheduled effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledEffectClass {
    /// Append to one exact owned local log.
    AppendOwnedLog,
    /// Update one exact owned task record to a constrained desired state.
    UpdateOwnedTask,
    /// Ordinary fast-forward update of one dedicated task branch.
    UpdateDedicatedTaskBranch,
    /// Fixed-recipient or variable-content messaging requires live judgment.
    MessagingRequiresLiveApproval,
    /// Arbitrary shell always requires live approval and is not schedulable.
    ShellRequiresLiveApproval,
    /// Force/destructive operation is never schedulable.
    DestructiveProhibited,
}

impl ScheduledEffectClass {
    const fn predeclarable(self) -> bool {
        matches!(
            self,
            Self::AppendOwnedLog | Self::UpdateOwnedTask | Self::UpdateDedicatedTaskBranch
        )
    }
}

/// Model-independent control state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledAuthorityControl {
    /// Active after an exact reviewed preview.
    Active,
    /// Paused without destroying configuration.
    Paused,
    /// Revoked and unusable.
    Revoked,
    /// Expired and unusable.
    Expired,
    /// Emergency-stopped with descendants terminated.
    EmergencyStopped,
}

/// Exact unattended-authority grant and threat-model binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledAuthorityGrant {
    /// Stable schedule identity.
    pub schedule_id: String,
    /// Exact prior user configuration; never a live approval substitute.
    pub prior_configuration_sha256: String,
    /// Separately approved unattended-authority threat model.
    pub threat_model_sha256: String,
    /// Exact allowlist entry.
    pub allowlist_sha256: String,
    /// Closed effect class.
    pub effect: ScheduledEffectClass,
    /// Exact task template.
    pub task_template_sha256: String,
    /// Exact workspace.
    pub workspace_sha256: String,
    /// Dedicated worktree.
    pub worktree_sha256: String,
    /// Exact file-ownership manifest.
    pub ownership_sha256: String,
    /// Exact model profile.
    pub model_sha256: String,
    /// Exact tool set.
    pub tools_sha256: String,
    /// Exact destination set.
    pub destinations_sha256: String,
    /// Closed payload constraint.
    pub payload_constraints_sha256: String,
    /// Independent budget set.
    pub budgets_sha256: String,
    /// Exact stop conditions.
    pub stop_conditions_sha256: String,
    /// Exact dry-run result.
    pub dry_run_sha256: String,
    /// Exact activation preview.
    pub activation_preview_sha256: String,
    /// Exact activation approval, distinct from prior configuration.
    pub activation_approval_sha256: String,
    /// Exact idempotency key.
    pub idempotency_key_sha256: String,
    /// Expiry.
    pub expires_epoch_milliseconds: u64,
    /// Current time.
    pub now_epoch_milliseconds: u64,
    /// Independent control state.
    pub control: ScheduledAuthorityControl,
    /// The schedule did not create, edit, activate, renew, or broaden itself.
    pub self_modified: bool,
}

/// Mutable state captured both at approval and immediately before execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledAuthorityRefresh {
    /// Schedule record.
    pub schedule_sha256: String,
    /// Workspace state.
    pub workspace_sha256: String,
    /// Worktree state.
    pub worktree_sha256: String,
    /// Tool and policy state.
    pub policy_sha256: String,
    /// Destination state.
    pub destination_sha256: String,
    /// Payload-constraint state.
    pub payload_constraints_sha256: String,
    /// Effect precondition.
    pub precondition_sha256: String,
}

/// Inert exact activation/execution preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledAuthorityPreview {
    /// Schedule identity.
    pub schedule_id: String,
    /// Exact effect class.
    pub effect: ScheduledEffectClass,
    /// Allowlist entry.
    pub allowlist_sha256: String,
    /// Task template.
    pub task_template_sha256: String,
    /// Workspace.
    pub workspace_sha256: String,
    /// Worktree.
    pub worktree_sha256: String,
    /// Ownership.
    pub ownership_sha256: String,
    /// Destination.
    pub destinations_sha256: String,
    /// Payload constraints.
    pub payload_constraints_sha256: String,
    /// Dry-run evidence.
    pub dry_run_sha256: String,
    /// Activation preview.
    pub activation_preview_sha256: String,
    /// Activation approval.
    pub activation_approval_sha256: String,
    /// Idempotency key.
    pub idempotency_key_sha256: String,
    /// Refreshed precondition.
    pub precondition_sha256: String,
}

/// Observed result of a separately owned scheduled effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledAuthorityResult {
    /// Exact postcondition verified.
    Verified,
    /// No effect verified.
    NotApplied,
    /// Effect uncertain; retry prohibited.
    Unknown,
}

/// Stable fail-closed scheduled-authority error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduledAuthorityError {
    /// Grant identity, allowlist, activation, ownership, expiry, or control invalid.
    GrantDenied,
    /// Mutable state drifted after approval.
    StaleState,
    /// Effect requires live judgment or is prohibited.
    LiveApprovalRequired,
    /// Unknown effect requires reconciliation and prohibits retry.
    RetryBlocked,
}

/// Admits an inert scheduled-effect preview after exact dry-run and state refresh.
pub fn admit_scheduled_authority(
    grant: &ScheduledAuthorityGrant,
    approved: &ScheduledAuthorityRefresh,
    current: &ScheduledAuthorityRefresh,
) -> Result<ScheduledAuthorityPreview, ScheduledAuthorityError> {
    let hashes = [
        &grant.prior_configuration_sha256,
        &grant.threat_model_sha256,
        &grant.allowlist_sha256,
        &grant.task_template_sha256,
        &grant.workspace_sha256,
        &grant.worktree_sha256,
        &grant.ownership_sha256,
        &grant.model_sha256,
        &grant.tools_sha256,
        &grant.destinations_sha256,
        &grant.payload_constraints_sha256,
        &grant.budgets_sha256,
        &grant.stop_conditions_sha256,
        &grant.dry_run_sha256,
        &grant.activation_preview_sha256,
        &grant.activation_approval_sha256,
        &grant.idempotency_key_sha256,
    ];
    if !valid_id(&grant.schedule_id)
        || !hashes.into_iter().all(|value| valid_sha256(value))
        || grant.prior_configuration_sha256 == grant.activation_approval_sha256
        || grant.control != ScheduledAuthorityControl::Active
        || grant.self_modified
        || grant.now_epoch_milliseconds == 0
        || grant.now_epoch_milliseconds >= grant.expires_epoch_milliseconds
        || !valid_refresh(approved)
    {
        return Err(ScheduledAuthorityError::GrantDenied);
    }
    if !grant.effect.predeclarable() {
        return Err(ScheduledAuthorityError::LiveApprovalRequired);
    }
    if approved != current {
        return Err(ScheduledAuthorityError::StaleState);
    }
    Ok(ScheduledAuthorityPreview {
        schedule_id: grant.schedule_id.clone(),
        effect: grant.effect,
        allowlist_sha256: grant.allowlist_sha256.clone(),
        task_template_sha256: grant.task_template_sha256.clone(),
        workspace_sha256: grant.workspace_sha256.clone(),
        worktree_sha256: grant.worktree_sha256.clone(),
        ownership_sha256: grant.ownership_sha256.clone(),
        destinations_sha256: grant.destinations_sha256.clone(),
        payload_constraints_sha256: grant.payload_constraints_sha256.clone(),
        dry_run_sha256: grant.dry_run_sha256.clone(),
        activation_preview_sha256: grant.activation_preview_sha256.clone(),
        activation_approval_sha256: grant.activation_approval_sha256.clone(),
        idempotency_key_sha256: grant.idempotency_key_sha256.clone(),
        precondition_sha256: current.precondition_sha256.clone(),
    })
}

/// Accepts only certain terminal results; unknown effects block retry.
pub const fn reconcile_scheduled_authority(
    result: ScheduledAuthorityResult,
) -> Result<(), ScheduledAuthorityError> {
    match result {
        ScheduledAuthorityResult::Verified | ScheduledAuthorityResult::NotApplied => Ok(()),
        ScheduledAuthorityResult::Unknown => Err(ScheduledAuthorityError::RetryBlocked),
    }
}

fn valid_refresh(value: &ScheduledAuthorityRefresh) -> bool {
    [
        &value.schedule_sha256,
        &value.workspace_sha256,
        &value.worktree_sha256,
        &value.policy_sha256,
        &value.destination_sha256,
        &value.payload_constraints_sha256,
        &value.precondition_sha256,
    ]
    .into_iter()
    .all(|value| valid_sha256(value))
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hash(c: char) -> String {
        c.to_string().repeat(64)
    }
    fn refresh() -> ScheduledAuthorityRefresh {
        ScheduledAuthorityRefresh {
            schedule_sha256: hash('1'),
            workspace_sha256: hash('2'),
            worktree_sha256: hash('3'),
            policy_sha256: hash('4'),
            destination_sha256: hash('5'),
            payload_constraints_sha256: hash('6'),
            precondition_sha256: hash('7'),
        }
    }
    fn grant(effect: ScheduledEffectClass) -> ScheduledAuthorityGrant {
        ScheduledAuthorityGrant {
            schedule_id: "schedule-1".into(),
            prior_configuration_sha256: hash('1'),
            threat_model_sha256: hash('2'),
            allowlist_sha256: hash('3'),
            effect,
            task_template_sha256: hash('4'),
            workspace_sha256: hash('5'),
            worktree_sha256: hash('6'),
            ownership_sha256: hash('7'),
            model_sha256: hash('8'),
            tools_sha256: hash('9'),
            destinations_sha256: hash('a'),
            payload_constraints_sha256: hash('b'),
            budgets_sha256: hash('c'),
            stop_conditions_sha256: hash('d'),
            dry_run_sha256: hash('e'),
            activation_preview_sha256: hash('f'),
            activation_approval_sha256: hash('0'),
            idempotency_key_sha256: hash('2'),
            expires_epoch_milliseconds: 2,
            now_epoch_milliseconds: 1,
            control: ScheduledAuthorityControl::Active,
            self_modified: false,
        }
    }
    #[test]
    fn sprint_91_only_allowlisted_repeatable_effects_are_predeclarable() {
        for effect in [
            ScheduledEffectClass::AppendOwnedLog,
            ScheduledEffectClass::UpdateOwnedTask,
            ScheduledEffectClass::UpdateDedicatedTaskBranch,
        ] {
            assert!(admit_scheduled_authority(&grant(effect), &refresh(), &refresh()).is_ok());
        }
    }
    #[test]
    fn sprint_91_live_judgment_and_destructive_effects_are_denied() {
        for effect in [
            ScheduledEffectClass::MessagingRequiresLiveApproval,
            ScheduledEffectClass::ShellRequiresLiveApproval,
            ScheduledEffectClass::DestructiveProhibited,
        ] {
            assert_eq!(
                admit_scheduled_authority(&grant(effect), &refresh(), &refresh()),
                Err(ScheduledAuthorityError::LiveApprovalRequired)
            );
        }
    }
    #[test]
    fn sprint_91_any_mutable_state_drift_stops_execution() {
        let approved = refresh();
        let mut current = approved.clone();
        current.policy_sha256 = hash('0');
        assert_eq!(
            admit_scheduled_authority(
                &grant(ScheduledEffectClass::AppendOwnedLog),
                &approved,
                &current
            ),
            Err(ScheduledAuthorityError::StaleState)
        );
    }
    #[test]
    fn sprint_91_prior_configuration_is_not_live_activation() {
        let mut value = grant(ScheduledEffectClass::UpdateOwnedTask);
        value.activation_approval_sha256 = value.prior_configuration_sha256.clone();
        assert_eq!(
            admit_scheduled_authority(&value, &refresh(), &refresh()),
            Err(ScheduledAuthorityError::GrantDenied)
        );
    }
    #[test]
    fn sprint_91_self_modification_and_inactive_controls_are_denied() {
        let mut value = grant(ScheduledEffectClass::UpdateOwnedTask);
        value.self_modified = true;
        assert_eq!(
            admit_scheduled_authority(&value, &refresh(), &refresh()),
            Err(ScheduledAuthorityError::GrantDenied)
        );
        value.self_modified = false;
        value.control = ScheduledAuthorityControl::EmergencyStopped;
        assert_eq!(
            admit_scheduled_authority(&value, &refresh(), &refresh()),
            Err(ScheduledAuthorityError::GrantDenied)
        );
    }
    #[test]
    fn sprint_91_unknown_effect_blocks_blind_retry() {
        assert_eq!(
            reconcile_scheduled_authority(ScheduledAuthorityResult::Unknown),
            Err(ScheduledAuthorityError::RetryBlocked)
        );
    }
}
