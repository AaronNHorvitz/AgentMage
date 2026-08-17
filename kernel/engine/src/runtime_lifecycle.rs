//! Cross-domain runtime lifecycle planning without aggregated authority.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    GrantOperation, RuntimeEventRetention, RuntimeEventRetentionKind, SessionId, TaskId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: u16 = 1;
const MAX_TARGETS: usize = 256;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure returned before any lifecycle owner is invoked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeLifecycleError {
    /// The request is malformed, oversized, noncanonical, or digest-invalid.
    InvalidRequest,
    /// Fresh domain observations do not permit the requested lifecycle transition.
    UnsafeTargetState,
    /// Safe mode intentionally prohibits the requested action.
    SafeModeDenied,
    /// Canonical request or plan encoding failed.
    Serialization,
}

impl RuntimeLifecycleError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "runtime_lifecycle.request.invalid",
            Self::UnsafeTargetState => "runtime_lifecycle.target.unsafe",
            Self::SafeModeDenied => "runtime_lifecycle.safe_mode.denied",
            Self::Serialization => "runtime_lifecycle.serialization.failed",
        }
    }
}

/// Independently governed runtime data domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLifecycleDomain {
    /// Canonical content-minimized execution events.
    Journal,
    /// Optional persisted user-visible conversation projection.
    Transcript,
    /// Optional content-free local measurements.
    Metrics,
    /// Immutable content-addressed runtime payloads.
    Artifact,
    /// AgentMage-owned isolated repository worktrees.
    Worktree,
    /// Atomic session checkpoints and resume bindings.
    Checkpoint,
}

impl RuntimeLifecycleDomain {
    /// Every closed lifecycle domain.
    pub const ALL: [Self; 6] = [
        Self::Journal,
        Self::Transcript,
        Self::Metrics,
        Self::Artifact,
        Self::Worktree,
        Self::Checkpoint,
    ];
}

/// Existing component that remains authoritative for one lifecycle domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLifecycleOwner {
    /// `runtime_journal` and its encrypted operational-store tables.
    RuntimeJournal,
    /// `conversation_library` and `conversation_archive`.
    ConversationLibrary,
    /// Optional content-free local metric projection.
    LocalMetrics,
    /// `runtime_artifact` metadata and the platform payload store.
    RuntimeArtifactStore,
    /// `repository_safety` and the owned-worktree registry.
    RepositorySafety,
    /// `operational_store` checkpoint and resume-binding owner.
    OperationalStore,
}

/// Closed lifecycle action requested across one or more independent domains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLifecycleAction {
    /// Admit a new record into its selected optional or canonical projection.
    Collect,
    /// Assign or strengthen one exact retention rule.
    Retain,
    /// Delete one exact released target through its existing owner.
    Delete,
    /// Export one exact bounded domain projection to a separately approved destination.
    Export,
    /// Produce content-minimized local diagnostic facts without mutation.
    Diagnose,
}

impl RuntimeLifecycleAction {
    /// Every closed lifecycle action.
    pub const ALL: [Self; 5] = [
        Self::Collect,
        Self::Retain,
        Self::Delete,
        Self::Export,
        Self::Diagnose,
    ];
}

/// Runtime operating mode applied before ordinary lifecycle planning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeLifecycleMode {
    /// Ordinary verified operation under the current exact policy.
    Normal,
    /// Recovery-only operation with optional execution and cleanup disabled.
    SafeMode,
}

/// Descriptive authority prerequisite; this value is never a grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "operation", rename_all = "snake_case")]
pub enum RuntimeLifecycleAuthorityRequirement {
    /// Kernel-owned correctness persistence under the already admitted run.
    KernelOwned,
    /// Explicit local opt-in is required for an optional projection.
    ProjectionOptIn,
    /// The existing owner must validate its exact protected preview and user decision.
    ProtectedDomainApproval,
    /// One exact separately issued grant is required immediately before the effect.
    ExactGrant(GrantOperation),
}

/// Fresh content-free state of one exact lifecycle target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLifecycleTarget {
    /// Independently governed target domain.
    pub domain: RuntimeLifecycleDomain,
    /// Digest of the stable target identity; never a path or transcript value.
    pub target_identity_sha256: String,
    /// Exact current domain revision, or zero before initial collection.
    pub expected_revision: u64,
    /// Digest of the complete current content-free state, or zeroes before collection.
    pub state_sha256: String,
    /// Whether the existing domain owner freshly verified target integrity.
    pub integrity_verified: bool,
    /// Whether expiration/release and all holds permit deletion.
    pub retention_released: bool,
    /// Whether every child, reference, process, or resume dependency is resolved.
    pub dependents_resolved: bool,
    /// Whether owned-worktree cleanup eligibility was freshly verified.
    pub worktree_cleanup_verified: bool,
}

/// Exact sealed request for lifecycle planning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLifecycleRequest {
    /// Lifecycle planner schema version.
    pub schema_version: u16,
    /// Stable request identity.
    pub request_id: String,
    /// Owning session.
    pub session_id: SessionId,
    /// Owning task.
    pub task_id: TaskId,
    /// Digest of the exact current policy revision.
    pub policy_sha256: String,
    /// Current operating mode.
    pub mode: RuntimeLifecycleMode,
    /// One closed action applied independently to each target.
    pub action: RuntimeLifecycleAction,
    /// Canonically sorted unique target observations.
    pub targets: Vec<RuntimeLifecycleTarget>,
    /// Retention assignment required for collection or retention changes.
    pub retention: Option<RuntimeEventRetention>,
    /// Digest of an exact separately controlled export destination.
    pub export_destination_sha256: Option<String>,
    /// Explicit current local opt-in for optional transcript or metrics collection.
    pub optional_projection_enabled: bool,
    /// Trusted planning time.
    pub occurred_at_epoch_ms: u64,
    /// Digest of this request with this field set to zeroes.
    pub request_sha256: String,
}

/// One owner-specific lifecycle step carrying no authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLifecyclePlannedOperation {
    /// Exact target domain.
    pub domain: RuntimeLifecycleDomain,
    /// Existing component that must execute and revalidate this step.
    pub owner: RuntimeLifecycleOwner,
    /// Digest of the exact target identity.
    pub target_identity_sha256: String,
    /// Exact expected owner revision.
    pub expected_revision: u64,
    /// Descriptive prerequisite that cannot be converted into authority.
    pub authority_requirement: RuntimeLifecycleAuthorityRequirement,
    /// Whether an explicit protected user confirmation is required.
    pub requires_user_confirmation: bool,
    /// Fixed false marker: a plan cannot execute or mutate its target.
    pub effect_applied: bool,
}

/// Canonical non-authoritative cross-domain lifecycle plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLifecyclePlan {
    /// Exact sealed request from which this plan was derived.
    pub request: RuntimeLifecycleRequest,
    /// One independent owner-specific operation per target.
    pub operations: Vec<RuntimeLifecyclePlannedOperation>,
    /// Fixed marker showing whether safe-mode restrictions were applied.
    pub safe_mode: bool,
    /// Fixed false marker: no operation authority is present.
    pub carries_authority: bool,
    /// Digest of this plan with this field set to zeroes.
    pub plan_sha256: String,
}

/// Seals, validates, and plans one cross-domain lifecycle request.
pub fn plan_runtime_lifecycle(
    mut request: RuntimeLifecycleRequest,
) -> Result<RuntimeLifecyclePlan, RuntimeLifecycleError> {
    request.request_sha256 = ZERO_SHA256.to_owned();
    validate_request(&request, true)?;
    request.request_sha256 = canonical_sha256(&request)?;

    let operations = request
        .targets
        .iter()
        .map(|target| RuntimeLifecyclePlannedOperation {
            domain: target.domain,
            owner: owner(target.domain),
            target_identity_sha256: target.target_identity_sha256.clone(),
            expected_revision: target.expected_revision,
            authority_requirement: authority_requirement(request.action, target.domain),
            requires_user_confirmation: matches!(
                request.action,
                RuntimeLifecycleAction::Retain
                    | RuntimeLifecycleAction::Delete
                    | RuntimeLifecycleAction::Export
            ),
            effect_applied: false,
        })
        .collect();
    let mut plan = RuntimeLifecyclePlan {
        safe_mode: request.mode == RuntimeLifecycleMode::SafeMode,
        request,
        operations,
        carries_authority: false,
        plan_sha256: ZERO_SHA256.to_owned(),
    };
    plan.plan_sha256 = canonical_sha256(&plan)?;
    Ok(plan)
}

/// Verifies a retained lifecycle plan and every owner/authority projection.
pub fn verify_runtime_lifecycle_plan(
    plan: &RuntimeLifecyclePlan,
) -> Result<(), RuntimeLifecycleError> {
    validate_request(&plan.request, false)?;
    if plan.carries_authority
        || plan.safe_mode != (plan.request.mode == RuntimeLifecycleMode::SafeMode)
        || plan.operations.len() != plan.request.targets.len()
        || !valid_sha256(&plan.plan_sha256)
    {
        return Err(RuntimeLifecycleError::InvalidRequest);
    }
    for (operation, target) in plan.operations.iter().zip(&plan.request.targets) {
        if operation.domain != target.domain
            || operation.owner != owner(target.domain)
            || operation.target_identity_sha256 != target.target_identity_sha256
            || operation.expected_revision != target.expected_revision
            || operation.authority_requirement
                != authority_requirement(plan.request.action, target.domain)
            || operation.requires_user_confirmation
                != matches!(
                    plan.request.action,
                    RuntimeLifecycleAction::Retain
                        | RuntimeLifecycleAction::Delete
                        | RuntimeLifecycleAction::Export
                )
            || operation.effect_applied
        {
            return Err(RuntimeLifecycleError::InvalidRequest);
        }
    }
    let mut candidate = plan.clone();
    candidate.plan_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != plan.plan_sha256 {
        return Err(RuntimeLifecycleError::InvalidRequest);
    }
    Ok(())
}

fn validate_request(
    request: &RuntimeLifecycleRequest,
    unsealed: bool,
) -> Result<(), RuntimeLifecycleError> {
    if request.schema_version != SCHEMA_VERSION
        || !valid_id(&request.request_id)
        || !valid_id(request.session_id.as_str())
        || !valid_id(request.task_id.as_str())
        || !valid_sha256(&request.policy_sha256)
        || request.targets.is_empty()
        || request.targets.len() > MAX_TARGETS
        || request.occurred_at_epoch_ms == 0
        || (unsealed && request.request_sha256 != ZERO_SHA256)
        || (!unsealed && !valid_sha256(&request.request_sha256))
    {
        return Err(RuntimeLifecycleError::InvalidRequest);
    }
    if !unsealed {
        let mut candidate = request.clone();
        candidate.request_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&candidate)? != request.request_sha256 {
            return Err(RuntimeLifecycleError::InvalidRequest);
        }
    }
    let mut identities = BTreeSet::new();
    let mut previous = None;
    for target in &request.targets {
        let key = (target.domain, target.target_identity_sha256.as_str());
        if !valid_sha256(&target.target_identity_sha256)
            || !valid_sha256(&target.state_sha256)
            || !identities.insert(key)
            || previous.is_some_and(|prior| prior >= key)
            || (request.action == RuntimeLifecycleAction::Collect
                && (target.expected_revision != 0 || target.state_sha256 != ZERO_SHA256))
            || (request.action != RuntimeLifecycleAction::Collect && target.expected_revision == 0)
        {
            return Err(RuntimeLifecycleError::InvalidRequest);
        }
        previous = Some(key);
        validate_target_state(request.action, target)?;
    }
    validate_action_fields(request)?;
    validate_safe_mode(request)
}

fn validate_target_state(
    action: RuntimeLifecycleAction,
    target: &RuntimeLifecycleTarget,
) -> Result<(), RuntimeLifecycleError> {
    match action {
        RuntimeLifecycleAction::Collect | RuntimeLifecycleAction::Diagnose => Ok(()),
        RuntimeLifecycleAction::Retain | RuntimeLifecycleAction::Export => target
            .integrity_verified
            .then_some(())
            .ok_or(RuntimeLifecycleError::UnsafeTargetState),
        RuntimeLifecycleAction::Delete => {
            if !target.integrity_verified
                || !target.retention_released
                || !target.dependents_resolved
                || (target.domain == RuntimeLifecycleDomain::Worktree
                    && !target.worktree_cleanup_verified)
            {
                Err(RuntimeLifecycleError::UnsafeTargetState)
            } else {
                Ok(())
            }
        }
    }
}

fn validate_action_fields(request: &RuntimeLifecycleRequest) -> Result<(), RuntimeLifecycleError> {
    match request.action {
        RuntimeLifecycleAction::Collect | RuntimeLifecycleAction::Retain => {
            let retention = request
                .retention
                .as_ref()
                .ok_or(RuntimeLifecycleError::InvalidRequest)?;
            if request.export_destination_sha256.is_some()
                || !valid_retention(retention, request.occurred_at_epoch_ms)
            {
                return Err(RuntimeLifecycleError::InvalidRequest);
            }
            if request.action == RuntimeLifecycleAction::Collect
                && request.targets.iter().any(|target| {
                    matches!(
                        target.domain,
                        RuntimeLifecycleDomain::Transcript | RuntimeLifecycleDomain::Metrics
                    )
                })
                && !request.optional_projection_enabled
            {
                return Err(RuntimeLifecycleError::InvalidRequest);
            }
        }
        RuntimeLifecycleAction::Export => {
            if request.retention.is_some()
                || !request
                    .export_destination_sha256
                    .as_deref()
                    .is_some_and(valid_sha256)
            {
                return Err(RuntimeLifecycleError::InvalidRequest);
            }
        }
        RuntimeLifecycleAction::Delete | RuntimeLifecycleAction::Diagnose => {
            if request.retention.is_some() || request.export_destination_sha256.is_some() {
                return Err(RuntimeLifecycleError::InvalidRequest);
            }
        }
    }
    Ok(())
}

fn validate_safe_mode(request: &RuntimeLifecycleRequest) -> Result<(), RuntimeLifecycleError> {
    if request.mode == RuntimeLifecycleMode::Normal {
        return Ok(());
    }
    match request.action {
        RuntimeLifecycleAction::Diagnose | RuntimeLifecycleAction::Export => Ok(()),
        RuntimeLifecycleAction::Retain
            if request
                .retention
                .as_ref()
                .is_some_and(|retention| retention.kind == RuntimeEventRetentionKind::UserHold) =>
        {
            Ok(())
        }
        RuntimeLifecycleAction::Collect
        | RuntimeLifecycleAction::Retain
        | RuntimeLifecycleAction::Delete => Err(RuntimeLifecycleError::SafeModeDenied),
    }
}

const fn owner(domain: RuntimeLifecycleDomain) -> RuntimeLifecycleOwner {
    match domain {
        RuntimeLifecycleDomain::Journal => RuntimeLifecycleOwner::RuntimeJournal,
        RuntimeLifecycleDomain::Transcript => RuntimeLifecycleOwner::ConversationLibrary,
        RuntimeLifecycleDomain::Metrics => RuntimeLifecycleOwner::LocalMetrics,
        RuntimeLifecycleDomain::Artifact => RuntimeLifecycleOwner::RuntimeArtifactStore,
        RuntimeLifecycleDomain::Worktree => RuntimeLifecycleOwner::RepositorySafety,
        RuntimeLifecycleDomain::Checkpoint => RuntimeLifecycleOwner::OperationalStore,
    }
}

const fn authority_requirement(
    action: RuntimeLifecycleAction,
    domain: RuntimeLifecycleDomain,
) -> RuntimeLifecycleAuthorityRequirement {
    match action {
        RuntimeLifecycleAction::Collect => match domain {
            RuntimeLifecycleDomain::Transcript | RuntimeLifecycleDomain::Metrics => {
                RuntimeLifecycleAuthorityRequirement::ProjectionOptIn
            }
            RuntimeLifecycleDomain::Journal
            | RuntimeLifecycleDomain::Artifact
            | RuntimeLifecycleDomain::Worktree
            | RuntimeLifecycleDomain::Checkpoint => {
                RuntimeLifecycleAuthorityRequirement::KernelOwned
            }
        },
        RuntimeLifecycleAction::Retain => {
            RuntimeLifecycleAuthorityRequirement::ProtectedDomainApproval
        }
        RuntimeLifecycleAction::Delete => match domain {
            RuntimeLifecycleDomain::Worktree => {
                RuntimeLifecycleAuthorityRequirement::ExactGrant(GrantOperation::GitWorktreeRemove)
            }
            RuntimeLifecycleDomain::Journal
            | RuntimeLifecycleDomain::Transcript
            | RuntimeLifecycleDomain::Metrics
            | RuntimeLifecycleDomain::Artifact
            | RuntimeLifecycleDomain::Checkpoint => {
                RuntimeLifecycleAuthorityRequirement::ProtectedDomainApproval
            }
        },
        RuntimeLifecycleAction::Export => {
            RuntimeLifecycleAuthorityRequirement::ExactGrant(GrantOperation::WorkspaceWrite)
        }
        RuntimeLifecycleAction::Diagnose => match domain {
            RuntimeLifecycleDomain::Worktree => {
                RuntimeLifecycleAuthorityRequirement::ExactGrant(GrantOperation::WorkspaceRead)
            }
            RuntimeLifecycleDomain::Journal
            | RuntimeLifecycleDomain::Transcript
            | RuntimeLifecycleDomain::Metrics
            | RuntimeLifecycleDomain::Artifact
            | RuntimeLifecycleDomain::Checkpoint => {
                RuntimeLifecycleAuthorityRequirement::ExactGrant(GrantOperation::DatabaseRead)
            }
        },
    }
}

fn valid_retention(retention: &RuntimeEventRetention, now_epoch_ms: u64) -> bool {
    match retention.kind {
        RuntimeEventRetentionKind::Ephemeral => false,
        RuntimeEventRetentionKind::Session | RuntimeEventRetentionKind::UserHold => {
            retention.expires_at_epoch_ms.is_none()
        }
        RuntimeEventRetentionKind::UntilExpiration => retention
            .expires_at_epoch_ms
            .is_some_and(|expires| expires > now_epoch_ms),
    }
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, RuntimeLifecycleError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RuntimeLifecycleError::Serialization)?;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut encoded, "{byte:02x}")
            .expect("writing a SHA-256 digest to a String cannot fail");
    }
    Ok(encoded)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
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

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn target(domain: RuntimeLifecycleDomain, ordinal: usize) -> RuntimeLifecycleTarget {
        RuntimeLifecycleTarget {
            domain,
            target_identity_sha256: format!("{ordinal:064x}"),
            expected_revision: 1,
            state_sha256: HASH_A.to_owned(),
            integrity_verified: true,
            retention_released: true,
            dependents_resolved: true,
            worktree_cleanup_verified: domain == RuntimeLifecycleDomain::Worktree,
        }
    }

    fn request(
        action: RuntimeLifecycleAction,
        domains: &[RuntimeLifecycleDomain],
    ) -> RuntimeLifecycleRequest {
        let mut targets = domains
            .iter()
            .enumerate()
            .map(|(index, domain)| target(*domain, index + 1))
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| {
            (left.domain, left.target_identity_sha256.as_str())
                .cmp(&(right.domain, right.target_identity_sha256.as_str()))
        });
        if action == RuntimeLifecycleAction::Collect {
            for target in &mut targets {
                target.expected_revision = 0;
                target.state_sha256 = ZERO_SHA256.to_owned();
                target.integrity_verified = false;
            }
        }
        RuntimeLifecycleRequest {
            schema_version: SCHEMA_VERSION,
            request_id: "lifecycle-request-0001".to_owned(),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            policy_sha256: HASH_B.to_owned(),
            mode: RuntimeLifecycleMode::Normal,
            action,
            targets,
            retention: matches!(
                action,
                RuntimeLifecycleAction::Collect | RuntimeLifecycleAction::Retain
            )
            .then_some(RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            }),
            export_destination_sha256: (action == RuntimeLifecycleAction::Export)
                .then(|| HASH_B.to_owned()),
            optional_projection_enabled: true,
            occurred_at_epoch_ms: 1_000,
            request_sha256: ZERO_SHA256.to_owned(),
        }
    }

    #[test]
    fn story_50_2_every_domain_and_action_retains_one_exact_owner() {
        for action in RuntimeLifecycleAction::ALL {
            let plan = plan_runtime_lifecycle(request(action, &RuntimeLifecycleDomain::ALL))
                .expect("matrix plans");
            verify_runtime_lifecycle_plan(&plan).expect("matrix verifies");
            assert_eq!(plan.operations.len(), RuntimeLifecycleDomain::ALL.len());
            assert!(!plan.carries_authority);
            for (operation, domain) in plan.operations.iter().zip(RuntimeLifecycleDomain::ALL) {
                assert_eq!(operation.domain, domain);
                assert_eq!(operation.owner, owner(domain));
                assert_eq!(
                    operation.authority_requirement,
                    authority_requirement(action, domain)
                );
                assert!(!operation.effect_applied);
            }
        }
    }

    #[test]
    fn story_50_2_optional_transcript_and_metric_collection_requires_current_opt_in() {
        for domain in [
            RuntimeLifecycleDomain::Transcript,
            RuntimeLifecycleDomain::Metrics,
        ] {
            let mut value = request(RuntimeLifecycleAction::Collect, &[domain]);
            value.optional_projection_enabled = false;
            assert_eq!(
                plan_runtime_lifecycle(value),
                Err(RuntimeLifecycleError::InvalidRequest)
            );
        }
        for domain in [
            RuntimeLifecycleDomain::Journal,
            RuntimeLifecycleDomain::Artifact,
            RuntimeLifecycleDomain::Worktree,
            RuntimeLifecycleDomain::Checkpoint,
        ] {
            let mut value = request(RuntimeLifecycleAction::Collect, &[domain]);
            value.optional_projection_enabled = false;
            let plan = plan_runtime_lifecycle(value).expect("canonical collection plans");
            assert_eq!(
                plan.operations[0].authority_requirement,
                RuntimeLifecycleAuthorityRequirement::KernelOwned
            );
        }
    }

    #[test]
    fn story_50_2_safe_mode_allows_only_diagnostics_export_and_preserving_hold() {
        for action in [
            RuntimeLifecycleAction::Diagnose,
            RuntimeLifecycleAction::Export,
        ] {
            let mut value = request(action, &[RuntimeLifecycleDomain::Journal]);
            value.mode = RuntimeLifecycleMode::SafeMode;
            let plan = plan_runtime_lifecycle(value).expect("safe recovery action plans");
            assert!(plan.safe_mode);
        }

        let mut hold = request(
            RuntimeLifecycleAction::Retain,
            &[RuntimeLifecycleDomain::Artifact],
        );
        hold.mode = RuntimeLifecycleMode::SafeMode;
        hold.retention = Some(RuntimeEventRetention {
            kind: RuntimeEventRetentionKind::UserHold,
            expires_at_epoch_ms: None,
        });
        assert!(plan_runtime_lifecycle(hold).is_ok());

        for action in [
            RuntimeLifecycleAction::Collect,
            RuntimeLifecycleAction::Delete,
            RuntimeLifecycleAction::Retain,
        ] {
            let mut value = request(action, &[RuntimeLifecycleDomain::Journal]);
            value.mode = RuntimeLifecycleMode::SafeMode;
            assert_eq!(
                plan_runtime_lifecycle(value),
                Err(RuntimeLifecycleError::SafeModeDenied)
            );
        }
    }

    #[test]
    fn story_50_2_deletion_requires_release_dependencies_and_worktree_cleanup() {
        for field in ["integrity", "retention", "dependents", "worktree"] {
            let mut value = request(
                RuntimeLifecycleAction::Delete,
                &[RuntimeLifecycleDomain::Worktree],
            );
            match field {
                "integrity" => value.targets[0].integrity_verified = false,
                "retention" => value.targets[0].retention_released = false,
                "dependents" => value.targets[0].dependents_resolved = false,
                "worktree" => value.targets[0].worktree_cleanup_verified = false,
                _ => unreachable!(),
            }
            assert_eq!(
                plan_runtime_lifecycle(value),
                Err(RuntimeLifecycleError::UnsafeTargetState)
            );
        }
    }

    #[test]
    fn story_50_2_targets_are_unique_canonical_and_never_share_aggregate_authority() {
        let mut value = request(
            RuntimeLifecycleAction::Delete,
            &[
                RuntimeLifecycleDomain::Artifact,
                RuntimeLifecycleDomain::Checkpoint,
            ],
        );
        let plan = plan_runtime_lifecycle(value.clone()).expect("independent operations plan");
        assert_eq!(plan.operations.len(), 2);
        assert!(plan.operations.iter().all(|operation| matches!(
            operation.authority_requirement,
            RuntimeLifecycleAuthorityRequirement::ProtectedDomainApproval
        )));
        assert!(!plan.carries_authority);

        value.targets.push(value.targets[0].clone());
        assert_eq!(
            plan_runtime_lifecycle(value),
            Err(RuntimeLifecycleError::InvalidRequest)
        );
    }

    #[test]
    fn story_50_2_plan_mutation_and_field_confusion_fail_closed() {
        let plan = plan_runtime_lifecycle(request(
            RuntimeLifecycleAction::Export,
            &[RuntimeLifecycleDomain::Artifact],
        ))
        .expect("export plans");
        let mut mutations = Vec::new();

        let mut owner_mutation = plan.clone();
        owner_mutation.operations[0].owner = RuntimeLifecycleOwner::RepositorySafety;
        mutations.push(owner_mutation);

        let mut authority_mutation = plan.clone();
        authority_mutation.operations[0].authority_requirement =
            RuntimeLifecycleAuthorityRequirement::KernelOwned;
        mutations.push(authority_mutation);

        let mut effect_mutation = plan.clone();
        effect_mutation.operations[0].effect_applied = true;
        mutations.push(effect_mutation);

        let mut request_mutation = plan.clone();
        request_mutation.request.policy_sha256 = HASH_A.to_owned();
        request_mutation.plan_sha256 = ZERO_SHA256.to_owned();
        request_mutation.plan_sha256 = canonical_sha256(&request_mutation).expect("plan hashes");
        mutations.push(request_mutation);

        let mut digest_mutation = plan;
        digest_mutation.plan_sha256 = HASH_A.to_owned();
        mutations.push(digest_mutation);

        for mutation in mutations {
            assert_eq!(
                verify_runtime_lifecycle_plan(&mutation),
                Err(RuntimeLifecycleError::InvalidRequest)
            );
        }
    }
}
