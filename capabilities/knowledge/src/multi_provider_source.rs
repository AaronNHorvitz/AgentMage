//! Provider-neutral Azure Repos and GitLab source/review contracts.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceProvider {
    AzureRepos,
    GitLabCloud,
    GitLabSelfHosted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReadKind {
    Repository,
    ProjectOrGroup,
    Reference,
    Commit,
    Tree,
    Blob,
    Tag,
    Release,
    PolicyOrProtection,
    PullOrMergeRequest,
    Review,
    Thread,
    Status,
    Check,
    Permission,
    Pipeline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceEffect {
    BranchPush,
    PullOrMergeRequest,
    Comment,
    Review,
    Thread,
    Label,
    Reviewer,
    DraftState,
    Closure,
    MergePreparation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewLineState {
    Exact,
    Moved,
    Deleted,
    Stale,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderSourceSnapshot {
    pub provider: SourceProvider,
    pub canonical_host: String,
    pub tenant_or_group: String,
    pub project_id: String,
    pub repository_id: String,
    pub account_id: String,
    pub base_revision: String,
    pub head_revision: String,
    pub review_object_id: String,
    pub object_revision: String,
    pub policy_sha256: String,
    pub permissions_sha256: String,
    pub instructions_sha256: String,
    pub worktree_manifest_sha256: String,
    pub provider_extensions: BTreeMap<String, String>,
    pub isolated_worktree: bool,
    pub active_checkout_changed: bool,
    pub untrusted_content: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEffectPlan {
    pub plan_id: String,
    pub provider: SourceProvider,
    pub canonical_host: String,
    pub tenant_or_group: String,
    pub project_id: String,
    pub repository_id: String,
    pub account_id: String,
    pub effect: SourceEffect,
    pub base_revision: String,
    pub head_revision: String,
    pub object_revision: String,
    pub policy_sha256: String,
    pub permissions_sha256: String,
    pub payload_sha256: String,
    pub expected_postcondition_sha256: String,
    pub idempotency_sha256: String,
    pub recipients: BTreeSet<String>,
    pub hidden_effect_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitApproval {
    pub tree_sha256: String,
    pub diff_sha256: String,
    pub message_sha256: String,
    pub parent_revision: String,
    pub signer_identity_sha256: String,
    pub signature_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushApproval {
    pub canonical_host: String,
    pub repository_id: String,
    pub full_ref: String,
    pub old_revision: String,
    pub new_revision: String,
    pub credential_domain_sha256: String,
    pub policy_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedSourceEffect {
    pub plan_id: String,
    pub effect: SourceEffect,
    pub payload_sha256: String,
    pub expected_postcondition_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiProviderSourceError {
    InvalidRecord,
    ProviderNamespaceMismatch,
    ActiveCheckoutRisk,
    StaleApproval,
    HiddenEffect,
    CommitPushConflation,
}

pub fn validate_source_snapshot(
    snapshot: &ProviderSourceSnapshot,
) -> Result<(), MultiProviderSourceError> {
    let ids = [
        &snapshot.canonical_host,
        &snapshot.tenant_or_group,
        &snapshot.project_id,
        &snapshot.repository_id,
        &snapshot.account_id,
        &snapshot.base_revision,
        &snapshot.head_revision,
        &snapshot.review_object_id,
        &snapshot.object_revision,
    ];
    let hashes = [
        &snapshot.policy_sha256,
        &snapshot.permissions_sha256,
        &snapshot.instructions_sha256,
        &snapshot.worktree_manifest_sha256,
    ];
    if ids.into_iter().any(|value| !valid_id(value))
        || hashes.into_iter().any(|value| !valid_sha(value))
        || !snapshot.isolated_worktree
        || snapshot.active_checkout_changed
        || !snapshot.untrusted_content
    {
        return Err(MultiProviderSourceError::ActiveCheckoutRisk);
    }
    let prefix = format!("{}.", provider_wire(snapshot.provider));
    if snapshot
        .provider_extensions
        .iter()
        .any(|(key, value)| !key.starts_with(&prefix) || !valid_id(key) || !valid_sha(value))
    {
        return Err(MultiProviderSourceError::ProviderNamespaceMismatch);
    }
    Ok(())
}

pub fn admit_source_effect(
    snapshot: &ProviderSourceSnapshot,
    plan: &SourceEffectPlan,
    commit: Option<&CommitApproval>,
    push: Option<&PushApproval>,
) -> Result<AdmittedSourceEffect, MultiProviderSourceError> {
    validate_source_snapshot(snapshot)?;
    if plan.provider != snapshot.provider
        || plan.canonical_host != snapshot.canonical_host
        || plan.tenant_or_group != snapshot.tenant_or_group
        || plan.project_id != snapshot.project_id
        || plan.repository_id != snapshot.repository_id
        || plan.account_id != snapshot.account_id
        || plan.base_revision != snapshot.base_revision
        || plan.head_revision != snapshot.head_revision
        || plan.object_revision != snapshot.object_revision
        || plan.policy_sha256 != snapshot.policy_sha256
        || plan.permissions_sha256 != snapshot.permissions_sha256
    {
        return Err(MultiProviderSourceError::StaleApproval);
    }
    if !valid_id(&plan.plan_id)
        || [
            &plan.payload_sha256,
            &plan.expected_postcondition_sha256,
            &plan.idempotency_sha256,
        ]
        .into_iter()
        .any(|value| !valid_sha(value))
        || plan.recipients.iter().any(|value| !valid_id(value))
    {
        return Err(MultiProviderSourceError::InvalidRecord);
    }
    if plan.hidden_effect_count != 0 {
        return Err(MultiProviderSourceError::HiddenEffect);
    }
    if plan.effect == SourceEffect::BranchPush {
        let (commit, push) = commit
            .zip(push)
            .ok_or(MultiProviderSourceError::CommitPushConflation)?;
        if [
            &commit.tree_sha256,
            &commit.diff_sha256,
            &commit.message_sha256,
            &commit.signer_identity_sha256,
            &commit.signature_sha256,
            &push.credential_domain_sha256,
        ]
        .into_iter()
        .any(|value| !valid_sha(value))
            || commit.parent_revision != snapshot.base_revision
            || push.canonical_host != snapshot.canonical_host
            || push.repository_id != snapshot.repository_id
            || !push.full_ref.starts_with("refs/")
            || push.old_revision != snapshot.base_revision
            || push.new_revision != snapshot.head_revision
            || push.policy_sha256 != snapshot.policy_sha256
        {
            return Err(MultiProviderSourceError::CommitPushConflation);
        }
    } else if commit.is_some() || push.is_some() {
        return Err(MultiProviderSourceError::CommitPushConflation);
    }
    Ok(AdmittedSourceEffect {
        plan_id: plan.plan_id.clone(),
        effect: plan.effect,
        payload_sha256: plan.payload_sha256.clone(),
        expected_postcondition_sha256: plan.expected_postcondition_sha256.clone(),
    })
}

fn provider_wire(provider: SourceProvider) -> &'static str {
    match provider {
        SourceProvider::AzureRepos => "azure-repos",
        SourceProvider::GitLabCloud => "gitlab-cloud",
        SourceProvider::GitLabSelfHosted => "gitlab-self-hosted",
    }
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> ProviderSourceSnapshot {
        let h = "a".repeat(64);
        ProviderSourceSnapshot {
            provider: SourceProvider::GitLabCloud,
            canonical_host: "gitlab.com".into(),
            tenant_or_group: "group-1".into(),
            project_id: "project-1".into(),
            repository_id: "repo-1".into(),
            account_id: "account-1".into(),
            base_revision: "base-1".into(),
            head_revision: "head-1".into(),
            review_object_id: "mr-1".into(),
            object_revision: "revision-1".into(),
            policy_sha256: h.clone(),
            permissions_sha256: h.clone(),
            instructions_sha256: h.clone(),
            worktree_manifest_sha256: h.clone(),
            provider_extensions: BTreeMap::from([("gitlab-cloud.approval".into(), h)]),
            isolated_worktree: true,
            active_checkout_changed: false,
            untrusted_content: true,
        }
    }
    fn plan(snapshot: &ProviderSourceSnapshot, effect: SourceEffect) -> SourceEffectPlan {
        let h = "b".repeat(64);
        SourceEffectPlan {
            plan_id: "plan-1".into(),
            provider: snapshot.provider,
            canonical_host: snapshot.canonical_host.clone(),
            tenant_or_group: snapshot.tenant_or_group.clone(),
            project_id: snapshot.project_id.clone(),
            repository_id: snapshot.repository_id.clone(),
            account_id: snapshot.account_id.clone(),
            effect,
            base_revision: snapshot.base_revision.clone(),
            head_revision: snapshot.head_revision.clone(),
            object_revision: snapshot.object_revision.clone(),
            policy_sha256: snapshot.policy_sha256.clone(),
            permissions_sha256: snapshot.permissions_sha256.clone(),
            payload_sha256: h.clone(),
            expected_postcondition_sha256: h.clone(),
            idempotency_sha256: h,
            recipients: BTreeSet::from(["reviewer-1".into()]),
            hidden_effect_count: 0,
        }
    }
    #[test]
    fn namespaced_snapshot_preserves_active_checkout() {
        assert!(validate_source_snapshot(&snapshot()).is_ok());
        let mut changed = snapshot();
        changed.active_checkout_changed = true;
        assert_eq!(
            validate_source_snapshot(&changed),
            Err(MultiProviderSourceError::ActiveCheckoutRisk)
        );
    }
    #[test]
    fn hosted_effect_fails_on_stale_policy_or_hidden_effect() {
        let snapshot = snapshot();
        let mut plan = plan(&snapshot, SourceEffect::Comment);
        assert!(admit_source_effect(&snapshot, &plan, None, None).is_ok());
        plan.policy_sha256 = "c".repeat(64);
        assert_eq!(
            admit_source_effect(&snapshot, &plan, None, None),
            Err(MultiProviderSourceError::StaleApproval)
        );
        plan.policy_sha256 = snapshot.policy_sha256.clone();
        plan.hidden_effect_count = 1;
        assert_eq!(
            admit_source_effect(&snapshot, &plan, None, None),
            Err(MultiProviderSourceError::HiddenEffect)
        );
    }
    #[test]
    fn branch_push_requires_distinct_signed_commit_and_push_records() {
        let snapshot = snapshot();
        let plan = plan(&snapshot, SourceEffect::BranchPush);
        assert_eq!(
            admit_source_effect(&snapshot, &plan, None, None),
            Err(MultiProviderSourceError::CommitPushConflation)
        );
        let h = "d".repeat(64);
        let commit = CommitApproval {
            tree_sha256: h.clone(),
            diff_sha256: h.clone(),
            message_sha256: h.clone(),
            parent_revision: snapshot.base_revision.clone(),
            signer_identity_sha256: h.clone(),
            signature_sha256: h.clone(),
        };
        let push = PushApproval {
            canonical_host: snapshot.canonical_host.clone(),
            repository_id: snapshot.repository_id.clone(),
            full_ref: "refs/heads/task".into(),
            old_revision: snapshot.base_revision.clone(),
            new_revision: snapshot.head_revision.clone(),
            credential_domain_sha256: h,
            policy_sha256: snapshot.policy_sha256.clone(),
        };
        assert!(admit_source_effect(&snapshot, &plan, Some(&commit), Some(&push)).is_ok());
    }
}
