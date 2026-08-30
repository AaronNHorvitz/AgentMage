//! Fail-closed admission of immutable workflow graphs and exact step policies.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalApprovalRequirement, CanonicalEffectClass,
    CanonicalIdempotencyRequirement, CanonicalStepExecutionPolicy,
    CanonicalVerificationRequirement, CanonicalWorkflowDefinition, CanonicalWorkflowLifecycle,
    to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::engineering_records::ValidateCanonicalRecord;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_POLICY_IDENTITIES: usize = 32;

/// Exact association between one closed workflow step and its execution policy.
#[derive(Clone, Copy, Debug)]
pub struct WorkflowStepPolicyBinding<'a> {
    /// Workflow step identity governed by the policy.
    pub step_id: &'a str,
    /// Immutable execution policy for that step.
    pub policy: &'a CanonicalStepExecutionPolicy,
}

/// A workflow definition that passed graph, integrity, and policy admission.
///
/// Construction is intentionally private. Possession proves only descriptive policy
/// admission; it grants no authority and cannot dispatch a side effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedWorkflowDefinition {
    workflow_id: String,
    workflow_version: u64,
    definition_sha256: String,
    ordered_step_ids: Vec<String>,
    entry_step_ids: Vec<String>,
    policy_sha256_by_step: BTreeMap<String, String>,
}

impl AdmittedWorkflowDefinition {
    /// Returns the admitted workflow identity.
    #[must_use]
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    /// Returns the admitted immutable workflow version.
    #[must_use]
    pub const fn workflow_version(&self) -> u64 {
        self.workflow_version
    }

    /// Returns the content digest verified during admission.
    #[must_use]
    pub fn definition_sha256(&self) -> &str {
        &self.definition_sha256
    }

    /// Returns the definition's deterministic dependency-safe order.
    #[must_use]
    pub fn ordered_step_ids(&self) -> &[String] {
        &self.ordered_step_ids
    }

    /// Returns exactly the dependency-free frontier, in definition order.
    #[must_use]
    pub fn entry_step_ids(&self) -> &[String] {
        &self.entry_step_ids
    }

    /// Returns the verified policy digest for an admitted step.
    #[must_use]
    pub fn policy_sha256(&self, step_id: &str) -> Option<&str> {
        self.policy_sha256_by_step.get(step_id).map(String::as_str)
    }
}

/// Stable content-free reason why a workflow definition was denied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowDefinitionAdmissionError {
    /// The existing canonical definition contract is invalid.
    InvalidDefinition,
    /// The definition digest does not match its exact canonical preimage.
    DefinitionIntegrity,
    /// Entry steps are not exactly the dependency-free frontier in definition order.
    EntryFrontierMismatch,
    /// A dependency occurs after the step that names it.
    StepOrderInvalid,
    /// The policy binding set is incomplete, duplicated, or names an unknown step.
    PolicySetMismatch,
    /// A policy has malformed identity, version, list, time, or digest fields.
    InvalidPolicy,
    /// A policy digest does not match its exact canonical preimage.
    PolicyIntegrity,
    /// Multiple workflow steps reuse a policy or plan-step identity.
    PolicyIdentityReuse,
    /// Policy effect, retry, verifier, or budget fields differ from the workflow step.
    PolicyStepMismatch,
    /// Preflight or postcondition policy is absent or internally inconsistent.
    PreflightOrPostconditionInvalid,
    /// Approval policy is too weak for the declared effect class.
    ApprovalPolicyInvalid,
    /// Idempotency policy is inconsistent with the declared effect class.
    IdempotencyPolicyInvalid,
}

impl std::fmt::Display for WorkflowDefinitionAdmissionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.code())
    }
}

impl std::error::Error for WorkflowDefinitionAdmissionError {}

impl WorkflowDefinitionAdmissionError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidDefinition => "workflow.definition.invalid",
            Self::DefinitionIntegrity => "workflow.definition.integrity",
            Self::EntryFrontierMismatch => "workflow.definition.entry_frontier",
            Self::StepOrderInvalid => "workflow.definition.step_order",
            Self::PolicySetMismatch => "workflow.definition.policy_set",
            Self::InvalidPolicy => "workflow.definition.policy_invalid",
            Self::PolicyIntegrity => "workflow.definition.policy_integrity",
            Self::PolicyIdentityReuse => "workflow.definition.policy_identity_reuse",
            Self::PolicyStepMismatch => "workflow.definition.policy_step_mismatch",
            Self::PreflightOrPostconditionInvalid => {
                "workflow.definition.preflight_or_postcondition"
            }
            Self::ApprovalPolicyInvalid => "workflow.definition.approval_policy",
            Self::IdempotencyPolicyInvalid => "workflow.definition.idempotency_policy",
        }
    }
}

/// Admits one immutable closed workflow and exactly one policy per step.
///
/// This is a pure validation boundary. It performs no I/O, consumes no grant, and
/// creates no execution or retry authority.
pub fn admit_closed_workflow(
    definition: &CanonicalWorkflowDefinition,
    bindings: &[WorkflowStepPolicyBinding<'_>],
) -> Result<AdmittedWorkflowDefinition, WorkflowDefinitionAdmissionError> {
    definition
        .validate_canonical()
        .map_err(|_| WorkflowDefinitionAdmissionError::InvalidDefinition)?;
    if !record_integrity(definition, |candidate| {
        candidate.definition_sha256 = ZERO_SHA256.to_owned();
    }) {
        return Err(WorkflowDefinitionAdmissionError::DefinitionIntegrity);
    }

    let step_by_id = definition
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.step_id.as_str(), (index, step)))
        .collect::<BTreeMap<_, _>>();
    let expected_entries = definition
        .steps
        .iter()
        .filter(|step| step.depends_on.is_empty())
        .map(|step| step.step_id.as_str())
        .collect::<Vec<_>>();
    if !definition
        .entry_step_ids
        .iter()
        .map(String::as_str)
        .eq(expected_entries)
    {
        return Err(WorkflowDefinitionAdmissionError::EntryFrontierMismatch);
    }
    for (step_index, step) in definition.steps.iter().enumerate() {
        if step.depends_on.iter().any(|dependency| {
            step_by_id
                .get(dependency.as_str())
                .is_none_or(|(dependency_index, _)| *dependency_index >= step_index)
        }) {
            return Err(WorkflowDefinitionAdmissionError::StepOrderInvalid);
        }
        if step.model_role.is_none() && step.tool_id.is_none() {
            return Err(WorkflowDefinitionAdmissionError::InvalidDefinition);
        }
    }

    if bindings.len() != definition.steps.len() {
        return Err(WorkflowDefinitionAdmissionError::PolicySetMismatch);
    }
    let mut binding_by_step = BTreeMap::new();
    for binding in bindings {
        if !step_by_id.contains_key(binding.step_id)
            || binding_by_step
                .insert(binding.step_id, binding.policy)
                .is_some()
        {
            return Err(WorkflowDefinitionAdmissionError::PolicySetMismatch);
        }
    }

    let mut policy_ids = BTreeSet::new();
    let mut plan_step_ids = BTreeSet::new();
    let mut policy_sha256_by_step = BTreeMap::new();
    let mut plan_identity: Option<(&str, u32)> = None;
    for step in &definition.steps {
        let policy = binding_by_step[step.step_id.as_str()];
        validate_policy_shape(policy)?;
        if !record_integrity(policy, |candidate| {
            candidate.policy_sha256 = ZERO_SHA256.to_owned();
        }) {
            return Err(WorkflowDefinitionAdmissionError::PolicyIntegrity);
        }
        if !policy_ids.insert(policy.policy_id.as_str())
            || !plan_step_ids.insert(policy.plan_step_id.as_str())
        {
            return Err(WorkflowDefinitionAdmissionError::PolicyIdentityReuse);
        }
        match plan_identity {
            None => plan_identity = Some((policy.plan_id.as_str(), policy.plan_revision)),
            Some((plan_id, revision))
                if plan_id != policy.plan_id.as_str() || revision != policy.plan_revision =>
            {
                return Err(WorkflowDefinitionAdmissionError::PolicyStepMismatch);
            }
            Some(_) => {}
        }
        if policy.effect_class != step.effect_class
            || policy.retry_class != step.retry_class
            || policy.required_verifier_ids != step.verifier_ids
            || policy.budgets != step.budgets
        {
            return Err(WorkflowDefinitionAdmissionError::PolicyStepMismatch);
        }
        validate_effect_policy(policy)?;
        policy_sha256_by_step.insert(step.step_id.clone(), policy.policy_sha256.clone());
    }

    Ok(AdmittedWorkflowDefinition {
        workflow_id: definition.workflow_id.clone(),
        workflow_version: definition.workflow_version,
        definition_sha256: definition.definition_sha256.clone(),
        ordered_step_ids: definition
            .steps
            .iter()
            .map(|step| step.step_id.clone())
            .collect(),
        entry_step_ids: definition.entry_step_ids.clone(),
        policy_sha256_by_step,
    })
}

/// Returns whether a lifecycle value is one of the closed absorbing terminal states.
#[must_use]
pub const fn workflow_lifecycle_is_terminal(state: CanonicalWorkflowLifecycle) -> bool {
    matches!(
        state,
        CanonicalWorkflowLifecycle::Succeeded
            | CanonicalWorkflowLifecycle::NoOp
            | CanonicalWorkflowLifecycle::Blocked
            | CanonicalWorkflowLifecycle::Denied
            | CanonicalWorkflowLifecycle::Failed
            | CanonicalWorkflowLifecycle::Cancelled
            | CanonicalWorkflowLifecycle::TimedOut
            | CanonicalWorkflowLifecycle::ResourceExhausted
            | CanonicalWorkflowLifecycle::Uncertain
    )
}

fn validate_policy_shape(
    policy: &CanonicalStepExecutionPolicy,
) -> Result<(), WorkflowDefinitionAdmissionError> {
    let identities = [
        policy.policy_id.as_str(),
        policy.plan_id.as_str(),
        policy.plan_step_id.as_str(),
        policy.preflight_policy_id.as_str(),
        policy.side_effect_policy_id.as_str(),
        policy.approval_policy_id.as_str(),
        policy.idempotency_policy_id.as_str(),
        policy.verifier_policy_id.as_str(),
        policy.retry_policy_id.as_str(),
        policy.budget_policy_id.as_str(),
        policy.diagnostic_policy_id.as_str(),
    ];
    if policy.schema_version != CONTRACT_SCHEMA_VERSION
        || policy.plan_revision == 0
        || identities
            .iter()
            .any(|identity| !valid_identifier(identity))
        || !valid_timestamp(&policy.recorded_at)
        || !valid_sha256(&policy.policy_sha256)
    {
        return Err(WorkflowDefinitionAdmissionError::InvalidPolicy);
    }
    if !valid_required_identity_list(&policy.required_preflight_ids)
        || !valid_required_identity_list(&policy.required_verifier_ids)
    {
        return Err(WorkflowDefinitionAdmissionError::PreflightOrPostconditionInvalid);
    }
    match (
        policy.verification_requirement,
        policy.deferral_reason_code.as_deref(),
    ) {
        (CanonicalVerificationRequirement::VerifierEvidenceRequired, None) => {}
        (CanonicalVerificationRequirement::PolicyDeferred, Some(reason))
            if valid_identifier(reason) => {}
        _ => return Err(WorkflowDefinitionAdmissionError::PreflightOrPostconditionInvalid),
    }
    if [
        policy.budgets.turns,
        policy.budgets.tokens,
        policy.budgets.duration_ms,
        policy.budgets.tool_calls,
        policy.budgets.attempts,
        policy.budgets.no_progress_events,
        policy.budgets.output_bytes,
        policy.budgets.memory_bytes,
    ]
    .contains(&0)
    {
        return Err(WorkflowDefinitionAdmissionError::InvalidPolicy);
    }
    Ok(())
}

fn validate_effect_policy(
    policy: &CanonicalStepExecutionPolicy,
) -> Result<(), WorkflowDefinitionAdmissionError> {
    if !policy
        .effect_class
        .permitted_retry_classes()
        .contains(&policy.retry_class)
    {
        return Err(WorkflowDefinitionAdmissionError::PolicyStepMismatch);
    }
    if policy.effect_class.requires_approval()
        && policy.approval_requirement != CanonicalApprovalRequirement::RequiredPerAttempt
    {
        return Err(WorkflowDefinitionAdmissionError::ApprovalPolicyInvalid);
    }
    let idempotency_valid = match policy.effect_class {
        CanonicalEffectClass::ReadOnly => {
            policy.idempotency_key_requirement == CanonicalIdempotencyRequirement::NotApplicable
        }
        CanonicalEffectClass::IdempotentWrite | CanonicalEffectClass::Conditional => matches!(
            policy.idempotency_key_requirement,
            CanonicalIdempotencyRequirement::Required
                | CanonicalIdempotencyRequirement::VerifiedDesiredState
        ),
        CanonicalEffectClass::NonIdempotent
        | CanonicalEffectClass::Destructive
        | CanonicalEffectClass::External
        | CanonicalEffectClass::Unknown => {
            policy.idempotency_key_requirement
                == CanonicalIdempotencyRequirement::VerifiedDesiredState
        }
    };
    if !idempotency_valid {
        return Err(WorkflowDefinitionAdmissionError::IdempotencyPolicyInvalid);
    }
    Ok(())
}

fn record_integrity<T: Clone + agentmage_kernel_contracts::VersionedContract>(
    value: &T,
    zero_digest: impl FnOnce(&mut T),
) -> bool {
    let mut candidate = value.clone();
    zero_digest(&mut candidate);
    let Ok(bytes) = to_canonical_json(&candidate) else {
        return false;
    };
    let mut actual = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(actual, "{byte:02x}");
    }
    let expected = if let Ok(bytes) = to_canonical_json(value) {
        let Ok(serialized) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return false;
        };
        serialized
            .get(if serialized.get("definition_sha256").is_some() {
                "definition_sha256"
            } else {
                "policy_sha256"
            })
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    } else {
        return false;
    };
    actual == expected
}

fn valid_required_identity_list(values: &[String]) -> bool {
    !values.is_empty()
        && values.len() <= MAX_POLICY_IDENTITIES
        && values.iter().all(|value| valid_identifier(value))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
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

fn valid_timestamp(value: &str) -> bool {
    value.len() >= 20 && value.len() <= 64 && value.is_ascii() && value.ends_with('Z')
}
