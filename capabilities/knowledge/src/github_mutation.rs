//! Pure GitHub mutation preview, grant, push, and reconciliation contracts.

use serde::{Deserialize, Serialize};

/// Separately granted GitHub mutation capability classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubMutationKind {
    /// Create or update one issue.
    Issue,
    /// Create or update one pull request.
    PullRequest,
    /// Publish one review.
    Review,
    /// Publish one comment or thread reply.
    Thread,
    /// Change one label.
    Label,
    /// Change one assignment.
    Assignment,
    /// Change one milestone.
    Milestone,
    /// Change one project field.
    ProjectField,
    /// Dispatch one workflow.
    Workflow,
    /// Create one exact signed local commit.
    Commit,
    /// Fast-forward one exact task branch.
    Push,
    /// Prepare one branch update without publishing it.
    BranchUpdate,
    /// Prepare one merge without performing it.
    Merge,
    /// Prepare or publish one release under its own grant.
    Release,
}

/// GitHub operations which this boundary never admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProhibitedGithubOperation {
    /// An implicit destination or refspec.
    ImplicitDestination,
    /// A configured push URL substituted for the canonical host.
    ConfiguredPushUrl,
    /// Multiple destinations or refspecs.
    MultipleDestinations,
    /// A default, protected, release, or tag reference.
    ProtectedOrTagRef,
    /// A ref deletion.
    Deletion,
    /// Push options or upstream configuration mutation.
    PushOptionOrUpstreamMutation,
    /// Recursive submodule publication.
    SubmoduleRecursion,
    /// All, branches, mirror, tags, or follow-tags expansion.
    ScopeExpansion,
    /// Force or any force-with-lease form.
    Force,
    /// Ruleset, protection, or permission bypass.
    Bypass,
    /// Automatic review, fix, merge, or release.
    AutomaticPublication,
    /// Repository administration, secret, or ruleset mutation.
    Administration,
}

/// Extra identities required for an ordinary single-ref fast-forward push.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubPushBinding {
    /// Full task-branch reference.
    pub full_ref: String,
    /// Expected remote object before the push.
    pub expected_old_object: String,
    /// Exact signed object after the push.
    pub expected_new_object: String,
    /// Exact staged diff.
    pub staged_diff_sha256: String,
    /// Exact commit message.
    pub message_sha256: String,
    /// Exact tree.
    pub tree_sha256: String,
    /// Exact parent.
    pub parent_sha256: String,
    /// Exact signer identity.
    pub signer_sha256: String,
    /// Signature was verified locally.
    pub signature_verified: bool,
    /// Separate approval for commit creation.
    pub commit_approval_sha256: String,
    /// Separate approval for network push.
    pub push_approval_sha256: String,
}

/// Exact current hosted and local state re-read before grant consumption.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRefresh {
    /// Hosted object or remote-ref state.
    pub hosted_state_sha256: String,
    /// Local branch and worktree state.
    pub local_state_sha256: String,
    /// Effective actor permissions.
    pub permissions_sha256: String,
    /// Branch protection state.
    pub protection_sha256: String,
    /// Repository rulesets.
    pub rulesets_sha256: String,
    /// Required review and check state.
    pub required_checks_sha256: String,
    /// Push-rule state.
    pub push_rules_sha256: String,
    /// Whether the actor possesses bypass capability; possession never authorizes use.
    pub bypass_capability_present: bool,
}

/// One exact locally constructed mutation request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubMutationRequest {
    /// Stable mutation identity.
    pub mutation_id: String,
    /// Canonical HTTPS host name.
    pub host: String,
    /// Canonical owner/repository identity.
    pub repository: String,
    /// Exact authenticated actor.
    pub actor_sha256: String,
    /// Narrow credential reference, never credential material.
    pub credential_sha256: String,
    /// One closed mutation class.
    pub kind: GithubMutationKind,
    /// Exact hosted object identity.
    pub object_sha256: String,
    /// Exact payload.
    pub payload_sha256: String,
    /// Exact expected hosted effect.
    pub expected_effect_sha256: String,
    /// Exact preview shown for approval.
    pub preview_sha256: String,
    /// Single-use network-write grant.
    pub grant_sha256: String,
    /// Unique idempotency key.
    pub idempotency_key_sha256: String,
    /// Absolute grant expiry in epoch milliseconds.
    pub expires_epoch_milliseconds: u64,
    /// Current time in epoch milliseconds.
    pub now_epoch_milliseconds: u64,
    /// State observed before approval.
    pub approved_refresh: HostedRefresh,
    /// State re-read immediately before submission.
    pub submission_refresh: HostedRefresh,
    /// Push-only identities; forbidden for every other class.
    pub push: Option<GithubPushBinding>,
    /// Closed list which must remain empty.
    pub prohibited_operations: Vec<ProhibitedGithubOperation>,
}

/// Content-free exact preview. Possession grants no network authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubMutationPreview {
    /// Stable mutation identity.
    pub mutation_id: String,
    /// Canonical host.
    pub host: String,
    /// Canonical repository.
    pub repository: String,
    /// Exact actor.
    pub actor_sha256: String,
    /// Closed mutation class.
    pub kind: GithubMutationKind,
    /// Exact object.
    pub object_sha256: String,
    /// Exact payload.
    pub payload_sha256: String,
    /// Exact expected effect.
    pub expected_effect_sha256: String,
    /// Exact displayed preview.
    pub preview_sha256: String,
    /// Exact grant.
    pub grant_sha256: String,
    /// Exact idempotency key.
    pub idempotency_key_sha256: String,
    /// Submission state digest.
    pub submission_state_sha256: String,
    /// Push-only binding.
    pub push: Option<GithubPushBinding>,
}

/// Closed terminal observation classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubMutationResult {
    /// Exact expected effect occurred once.
    VerifiedChanged,
    /// Fresh observation proves no effect.
    VerifiedUnchanged,
    /// A subset of the requested effect may exist.
    Partial,
    /// Transport ended without a certain effect disposition.
    Unknown,
}

/// Complete read/write receipt for one attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubMutationReceipt {
    /// Exact mutation identity.
    pub mutation_id: String,
    /// Exact idempotency identity.
    pub idempotency_key_sha256: String,
    /// Terminal result class.
    pub result: GithubMutationResult,
    /// Freshly observed remote state after the attempt.
    pub observed_state_sha256: String,
    /// Exact expected effect when verified changed.
    pub observed_effect_sha256: Option<String>,
    /// Whether external state changed.
    pub external_state_changed: bool,
    /// Whether exactly one effect was observed.
    pub exactly_once: bool,
    /// Whether owned transport work terminated.
    pub terminated: bool,
    /// Optional rollback or compensation plan digest.
    pub recovery_sha256: Option<String>,
}

/// Stable fail-closed admission or reconciliation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GithubMutationError {
    /// Identity, state, grant, expiry, or prohibited-operation failure.
    RequestDenied,
    /// Push signature, reference, or distinct-approval failure.
    PushDenied,
    /// Receipt contradicts the request or terminal result.
    ReceiptDenied,
    /// Unknown or partial effects require fresh reconciliation and prohibit retry.
    RetryBlocked,
}

/// Builds an inert exact preview after all mutable state has been re-read.
pub fn admit_github_mutation(
    request: &GithubMutationRequest,
) -> Result<GithubMutationPreview, GithubMutationError> {
    let hashes = [
        &request.actor_sha256,
        &request.credential_sha256,
        &request.object_sha256,
        &request.payload_sha256,
        &request.expected_effect_sha256,
        &request.preview_sha256,
        &request.grant_sha256,
        &request.idempotency_key_sha256,
    ];
    if !valid_id(&request.mutation_id)
        || !valid_host(&request.host)
        || !valid_repository(&request.repository)
        || !hashes.into_iter().all(|value| valid_sha256(value))
        || request.now_epoch_milliseconds == 0
        || request.now_epoch_milliseconds >= request.expires_epoch_milliseconds
        || request.approved_refresh != request.submission_refresh
        || !valid_refresh(&request.submission_refresh)
        || !request.prohibited_operations.is_empty()
    {
        return Err(GithubMutationError::RequestDenied);
    }
    match (&request.kind, &request.push) {
        (GithubMutationKind::Push, Some(push)) if valid_push(push) => {}
        (GithubMutationKind::Push, _) | (_, Some(_)) => {
            return Err(GithubMutationError::PushDenied);
        }
        _ => {}
    }
    Ok(GithubMutationPreview {
        mutation_id: request.mutation_id.clone(),
        host: request.host.clone(),
        repository: request.repository.clone(),
        actor_sha256: request.actor_sha256.clone(),
        kind: request.kind,
        object_sha256: request.object_sha256.clone(),
        payload_sha256: request.payload_sha256.clone(),
        expected_effect_sha256: request.expected_effect_sha256.clone(),
        preview_sha256: request.preview_sha256.clone(),
        grant_sha256: request.grant_sha256.clone(),
        idempotency_key_sha256: request.idempotency_key_sha256.clone(),
        submission_state_sha256: request.submission_refresh.hosted_state_sha256.clone(),
        push: request.push.clone(),
    })
}

/// Reconciles an attempt; unknown or partial effects can never authorize a blind retry.
pub fn reconcile_github_mutation(
    request: &GithubMutationRequest,
    preview: &GithubMutationPreview,
    receipt: &GithubMutationReceipt,
) -> Result<GithubMutationResult, GithubMutationError> {
    if admit_github_mutation(request)? != *preview
        || receipt.mutation_id != request.mutation_id
        || receipt.idempotency_key_sha256 != request.idempotency_key_sha256
        || !valid_sha256(&receipt.observed_state_sha256)
        || receipt
            .observed_effect_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || receipt
            .recovery_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || !receipt.terminated
    {
        return Err(GithubMutationError::ReceiptDenied);
    }
    match receipt.result {
        GithubMutationResult::VerifiedChanged
            if receipt.external_state_changed
                && receipt.exactly_once
                && receipt.observed_effect_sha256.as_deref()
                    == Some(request.expected_effect_sha256.as_str()) =>
        {
            Ok(receipt.result)
        }
        GithubMutationResult::VerifiedUnchanged
            if !receipt.external_state_changed
                && !receipt.exactly_once
                && receipt.observed_effect_sha256.is_none() =>
        {
            Ok(receipt.result)
        }
        GithubMutationResult::Partial | GithubMutationResult::Unknown => {
            Err(GithubMutationError::RetryBlocked)
        }
        _ => Err(GithubMutationError::ReceiptDenied),
    }
}

fn valid_refresh(value: &HostedRefresh) -> bool {
    [
        &value.hosted_state_sha256,
        &value.local_state_sha256,
        &value.permissions_sha256,
        &value.protection_sha256,
        &value.rulesets_sha256,
        &value.required_checks_sha256,
        &value.push_rules_sha256,
    ]
    .into_iter()
    .all(|value| valid_sha256(value))
}

fn valid_push(value: &GithubPushBinding) -> bool {
    value.full_ref.starts_with("refs/heads/agentmage/")
        && !value.full_ref.contains("..")
        && value.expected_old_object != value.expected_new_object
        && value.signature_verified
        && value.commit_approval_sha256 != value.push_approval_sha256
        && [
            &value.expected_old_object,
            &value.expected_new_object,
            &value.staged_diff_sha256,
            &value.message_sha256,
            &value.tree_sha256,
            &value.parent_sha256,
            &value.signer_sha256,
            &value.commit_approval_sha256,
            &value.push_approval_sha256,
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

fn valid_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && !value.contains(['/', ':', '@'])
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn valid_repository(value: &str) -> bool {
    value.len() <= 256
        && value.split_once('/').is_some_and(|(owner, repository)| {
            valid_id(owner) && valid_id(repository) && !repository.ends_with(".git")
        })
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
    fn refresh() -> HostedRefresh {
        HostedRefresh {
            hosted_state_sha256: hash('1'),
            local_state_sha256: hash('2'),
            permissions_sha256: hash('3'),
            protection_sha256: hash('4'),
            rulesets_sha256: hash('5'),
            required_checks_sha256: hash('6'),
            push_rules_sha256: hash('7'),
            bypass_capability_present: false,
        }
    }
    fn request(kind: GithubMutationKind) -> GithubMutationRequest {
        GithubMutationRequest {
            mutation_id: "mutation-1".into(),
            host: "github.com".into(),
            repository: "owner/repository".into(),
            actor_sha256: hash('a'),
            credential_sha256: hash('b'),
            kind,
            object_sha256: hash('c'),
            payload_sha256: hash('d'),
            expected_effect_sha256: hash('e'),
            preview_sha256: hash('f'),
            grant_sha256: hash('8'),
            idempotency_key_sha256: hash('9'),
            expires_epoch_milliseconds: 2,
            now_epoch_milliseconds: 1,
            approved_refresh: refresh(),
            submission_refresh: refresh(),
            push: None,
            prohibited_operations: vec![],
        }
    }
    fn push() -> GithubPushBinding {
        GithubPushBinding {
            full_ref: "refs/heads/agentmage/task-1".into(),
            expected_old_object: hash('a'),
            expected_new_object: hash('b'),
            staged_diff_sha256: hash('c'),
            message_sha256: hash('d'),
            tree_sha256: hash('e'),
            parent_sha256: hash('f'),
            signer_sha256: hash('1'),
            signature_verified: true,
            commit_approval_sha256: hash('2'),
            push_approval_sha256: hash('3'),
        }
    }
    #[test]
    fn sprint_85_all_mutation_classes_create_exact_inert_previews() {
        for kind in [
            GithubMutationKind::Issue,
            GithubMutationKind::PullRequest,
            GithubMutationKind::Review,
            GithubMutationKind::Thread,
            GithubMutationKind::Label,
            GithubMutationKind::Assignment,
            GithubMutationKind::Milestone,
            GithubMutationKind::ProjectField,
            GithubMutationKind::Workflow,
            GithubMutationKind::Commit,
            GithubMutationKind::BranchUpdate,
            GithubMutationKind::Merge,
            GithubMutationKind::Release,
        ] {
            assert!(admit_github_mutation(&request(kind)).is_ok());
        }
    }
    #[test]
    fn sprint_85_state_drift_expiry_and_prohibited_operations_fail_closed() {
        let mut value = request(GithubMutationKind::Issue);
        value.submission_refresh.hosted_state_sha256 = hash('0');
        assert_eq!(
            admit_github_mutation(&value),
            Err(GithubMutationError::RequestDenied)
        );
        value.submission_refresh = value.approved_refresh.clone();
        value.now_epoch_milliseconds = value.expires_epoch_milliseconds;
        assert_eq!(
            admit_github_mutation(&value),
            Err(GithubMutationError::RequestDenied)
        );
        value.now_epoch_milliseconds = 1;
        value.prohibited_operations = vec![ProhibitedGithubOperation::Force];
        assert_eq!(
            admit_github_mutation(&value),
            Err(GithubMutationError::RequestDenied)
        );
    }
    #[test]
    fn sprint_85_push_requires_signature_task_ref_and_distinct_approvals() {
        let mut value = request(GithubMutationKind::Push);
        value.push = Some(push());
        assert!(admit_github_mutation(&value).is_ok());
        value.push.as_mut().expect("push").signature_verified = false;
        assert_eq!(
            admit_github_mutation(&value),
            Err(GithubMutationError::PushDenied)
        );
    }
    #[test]
    fn sprint_86_verified_effect_must_be_exactly_once() {
        let value = request(GithubMutationKind::Issue);
        let preview = admit_github_mutation(&value).expect("preview");
        let receipt = GithubMutationReceipt {
            mutation_id: value.mutation_id.clone(),
            idempotency_key_sha256: value.idempotency_key_sha256.clone(),
            result: GithubMutationResult::VerifiedChanged,
            observed_state_sha256: hash('0'),
            observed_effect_sha256: Some(value.expected_effect_sha256.clone()),
            external_state_changed: true,
            exactly_once: true,
            terminated: true,
            recovery_sha256: None,
        };
        assert_eq!(
            reconcile_github_mutation(&value, &preview, &receipt),
            Ok(GithubMutationResult::VerifiedChanged)
        );
    }
    #[test]
    fn sprint_86_unknown_and_partial_results_block_retry() {
        let value = request(GithubMutationKind::Issue);
        let preview = admit_github_mutation(&value).expect("preview");
        for result in [GithubMutationResult::Unknown, GithubMutationResult::Partial] {
            let receipt = GithubMutationReceipt {
                mutation_id: value.mutation_id.clone(),
                idempotency_key_sha256: value.idempotency_key_sha256.clone(),
                result,
                observed_state_sha256: hash('0'),
                observed_effect_sha256: None,
                external_state_changed: false,
                exactly_once: false,
                terminated: true,
                recovery_sha256: Some(hash('1')),
            };
            assert_eq!(
                reconcile_github_mutation(&value, &preview, &receipt),
                Err(GithubMutationError::RetryBlocked)
            );
        }
    }
    #[test]
    fn sprint_86_receipt_identity_and_effect_drift_fail_closed() {
        let value = request(GithubMutationKind::Issue);
        let preview = admit_github_mutation(&value).expect("preview");
        let receipt = GithubMutationReceipt {
            mutation_id: value.mutation_id.clone(),
            idempotency_key_sha256: hash('0'),
            result: GithubMutationResult::VerifiedChanged,
            observed_state_sha256: hash('1'),
            observed_effect_sha256: Some(hash('2')),
            external_state_changed: true,
            exactly_once: true,
            terminated: true,
            recovery_sha256: None,
        };
        assert_eq!(
            reconcile_github_mutation(&value, &preview, &receipt),
            Err(GithubMutationError::ReceiptDenied)
        );
    }
}
