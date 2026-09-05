//! Cross-pack effect isolation, hostile-input accounting, and removal reconciliation.
#![allow(missing_docs)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityPack {
    Productivity,
    Communications,
    Finance,
    CloudObserver,
    Delivery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProhibitedAuthorityFamily {
    MoneyMovement,
    FinancialAdministration,
    CloudMutation,
    CloudExecution,
    SecretAccess,
    IdentityAdministration,
    PolicyAdministration,
    ContentCreatedAuthority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampaignState {
    Idle,
    Queued,
    InFlight,
    Uncertain,
    Synchronizing,
    Stale,
    Crashed,
    PartiallyRemoved,
    Removed,
    RestoredStrictLocal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossPackCampaignCase {
    pub case_id: String,
    pub autonomy_level: String,
    pub packs: BTreeSet<CapabilityPack>,
    pub provider_ids: BTreeSet<String>,
    pub account_ids: BTreeSet<String>,
    pub recipient_ids: BTreeSet<String>,
    pub destination_ids: BTreeSet<String>,
    pub workflow_id: String,
    pub state: CampaignState,
    pub declared_effect_ids: BTreeSet<String>,
    pub observed_effect_ids: BTreeSet<String>,
    pub probed_prohibited_families: BTreeSet<ProhibitedAuthorityFamily>,
    pub observed_prohibited_families: BTreeSet<ProhibitedAuthorityFamily>,
    pub uncertainty_codes: BTreeSet<String>,
    pub postcondition_sha256: String,
    pub evidence_sha256: String,
    pub unauthorized_disclosure_count: u32,
    pub duplicate_effect_count: u32,
    pub false_completion_count: u32,
    pub hidden_blocker_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackRemovalInventory {
    pub packs: BTreeSet<CapabilityPack>,
    pub state: CampaignState,
    pub retained_data_policy_id: String,
    pub credential_count: u32,
    pub cache_count: u32,
    pub cursor_count: u32,
    pub event_count: u32,
    pub webhook_count: u32,
    pub schedule_count: u32,
    pub index_count: u32,
    pub process_count: u32,
    pub socket_count: u32,
    pub listener_count: u32,
    pub network_scope_count: u32,
    pub grant_count: u32,
    pub undeclared_retained_data_count: u32,
    pub strict_local_network_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CampaignReconciliation {
    pub campaign_id: String,
    pub corpus_version: String,
    pub seed_sha256: String,
    pub environment_sha256: String,
    pub raw_results_sha256: String,
    pub recomputed_summary_sha256: String,
    pub recorded_summary_sha256: String,
    pub failure_count: u32,
    pub skip_count: u32,
    pub suppression_count: u32,
    pub quarantine_count: u32,
    pub hidden_blocker_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossPackError {
    Invalid,
    UndeclaredEffect,
    ProhibitedAuthority,
    Residue,
    Unreconciled,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn validate_campaign_case(value: &CrossPackCampaignCase) -> Result<(), CrossPackError> {
    if !id(&value.case_id)
        || !id(&value.autonomy_level)
        || value.packs.is_empty()
        || value.provider_ids.is_empty()
        || value.account_ids.is_empty()
        || !id(&value.workflow_id)
        || !digest(&value.postcondition_sha256)
        || !digest(&value.evidence_sha256)
        || value.probed_prohibited_families.len() != 8
    {
        return Err(CrossPackError::Invalid);
    }
    if value.declared_effect_ids != value.observed_effect_ids
        || value.duplicate_effect_count != 0
        || value.false_completion_count != 0
    {
        return Err(CrossPackError::UndeclaredEffect);
    }
    if !value.observed_prohibited_families.is_empty() || value.unauthorized_disclosure_count != 0 {
        return Err(CrossPackError::ProhibitedAuthority);
    }
    if value.hidden_blocker_count != 0 {
        return Err(CrossPackError::Invalid);
    }
    if matches!(value.state, CampaignState::Uncertain | CampaignState::Stale)
        && value.uncertainty_codes.is_empty()
    {
        return Err(CrossPackError::Invalid);
    }
    Ok(())
}

pub fn validate_removal(value: &PackRemovalInventory) -> Result<(), CrossPackError> {
    if value.packs.is_empty()
        || !id(&value.retained_data_policy_id)
        || !matches!(
            value.state,
            CampaignState::Removed | CampaignState::RestoredStrictLocal
        )
    {
        return Err(CrossPackError::Invalid);
    }
    if [
        value.credential_count,
        value.cache_count,
        value.cursor_count,
        value.event_count,
        value.webhook_count,
        value.schedule_count,
        value.index_count,
        value.process_count,
        value.socket_count,
        value.listener_count,
        value.network_scope_count,
        value.grant_count,
        value.undeclared_retained_data_count,
        value.strict_local_network_count,
    ]
    .into_iter()
    .any(|count| count != 0)
    {
        return Err(CrossPackError::Residue);
    }
    Ok(())
}

pub fn validate_reconciliation(value: &CampaignReconciliation) -> Result<(), CrossPackError> {
    if !id(&value.campaign_id)
        || !id(&value.corpus_version)
        || !digest(&value.seed_sha256)
        || !digest(&value.environment_sha256)
        || !digest(&value.raw_results_sha256)
        || !digest(&value.recomputed_summary_sha256)
        || value.recomputed_summary_sha256 != value.recorded_summary_sha256
        || [
            value.failure_count,
            value.skip_count,
            value.suppression_count,
            value.quarantine_count,
            value.hidden_blocker_count,
        ]
        .into_iter()
        .any(|count| count != 0)
    {
        return Err(CrossPackError::Unreconciled);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prohibited() -> BTreeSet<ProhibitedAuthorityFamily> {
        BTreeSet::from([
            ProhibitedAuthorityFamily::MoneyMovement,
            ProhibitedAuthorityFamily::FinancialAdministration,
            ProhibitedAuthorityFamily::CloudMutation,
            ProhibitedAuthorityFamily::CloudExecution,
            ProhibitedAuthorityFamily::SecretAccess,
            ProhibitedAuthorityFamily::IdentityAdministration,
            ProhibitedAuthorityFamily::PolicyAdministration,
            ProhibitedAuthorityFamily::ContentCreatedAuthority,
        ])
    }

    fn campaign() -> CrossPackCampaignCase {
        CrossPackCampaignCase {
            case_id: "case-1".into(),
            autonomy_level: "observe".into(),
            packs: BTreeSet::from([CapabilityPack::Finance, CapabilityPack::CloudObserver]),
            provider_ids: BTreeSet::from(["provider".into()]),
            account_ids: BTreeSet::from(["account".into()]),
            recipient_ids: BTreeSet::new(),
            destination_ids: BTreeSet::new(),
            workflow_id: "workflow-1".into(),
            state: CampaignState::Idle,
            declared_effect_ids: BTreeSet::new(),
            observed_effect_ids: BTreeSet::new(),
            probed_prohibited_families: prohibited(),
            observed_prohibited_families: BTreeSet::new(),
            uncertainty_codes: BTreeSet::new(),
            postcondition_sha256: "a".repeat(64),
            evidence_sha256: "b".repeat(64),
            unauthorized_disclosure_count: 0,
            duplicate_effect_count: 0,
            false_completion_count: 0,
            hidden_blocker_count: 0,
        }
    }

    #[test]
    fn declared_cross_pack_case_passes() {
        assert_eq!(validate_campaign_case(&campaign()), Ok(()));
    }

    #[test]
    fn money_or_cloud_authority_is_rejected() {
        let mut value = campaign();
        value
            .observed_prohibited_families
            .insert(ProhibitedAuthorityFamily::MoneyMovement);
        assert_eq!(
            validate_campaign_case(&value),
            Err(CrossPackError::ProhibitedAuthority)
        );
    }

    #[test]
    fn removal_and_strict_local_have_zero_residue() {
        let value = PackRemovalInventory {
            packs: BTreeSet::from([CapabilityPack::Productivity]),
            state: CampaignState::RestoredStrictLocal,
            retained_data_policy_id: "policy".into(),
            credential_count: 0,
            cache_count: 0,
            cursor_count: 0,
            event_count: 0,
            webhook_count: 0,
            schedule_count: 0,
            index_count: 0,
            process_count: 0,
            socket_count: 0,
            listener_count: 0,
            network_scope_count: 0,
            grant_count: 0,
            undeclared_retained_data_count: 0,
            strict_local_network_count: 0,
        };
        assert_eq!(validate_removal(&value), Ok(()));
    }

    #[test]
    fn summaries_must_recompute_exactly() {
        let value = CampaignReconciliation {
            campaign_id: "campaign".into(),
            corpus_version: "1".into(),
            seed_sha256: "a".repeat(64),
            environment_sha256: "b".repeat(64),
            raw_results_sha256: "c".repeat(64),
            recomputed_summary_sha256: "d".repeat(64),
            recorded_summary_sha256: "d".repeat(64),
            failure_count: 0,
            skip_count: 0,
            suppression_count: 0,
            quarantine_count: 0,
            hidden_blocker_count: 0,
        };
        assert_eq!(validate_reconciliation(&value), Ok(()));
    }
}
