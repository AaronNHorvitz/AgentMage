//! Closed dependency classification and fail-closed degradation evaluation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

const MAX_DEPENDENCIES: usize = 256;

/// Runtime dependency family requiring an explicit lifecycle disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeDependencyKind {
    /// Runtime implementation or worker process.
    Runtime,
    /// Source parser or extractor.
    Parser,
    /// Exact model profile.
    Model,
    /// Model protocol codec.
    Codec,
    /// Local or remote model endpoint.
    Endpoint,
    /// Native or mediated tool.
    Tool,
    /// Deterministic or admitted verifier.
    Verifier,
    /// Durable state or artifact store.
    Store,
    /// Native Chat, Verified Chat, CLI, headless, or another client feature.
    ClientFeature,
    /// Optional capability not represented by a narrower family.
    OptionalCapability,
}

/// Workflow-local dependency requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyRequirement {
    /// Loss prevents the affected workflow from executing.
    Required,
    /// Loss permits only an explicitly visible reduced-capability result.
    Optional,
    /// Loss permits only one fresh, explicitly qualified replacement.
    Substitutable,
}

/// Trusted observation of an exact dependency identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyHealth {
    /// Exact expected identity is current and usable.
    Available,
    /// Dependency is absent.
    Missing,
    /// Dependency returned a deterministic failure.
    Failed,
    /// Dependency output or protocol was malformed.
    Malformed,
    /// Identity or qualification evidence is stale.
    Stale,
    /// Operator or policy disabled the dependency.
    Disabled,
    /// Security or integrity policy quarantined the dependency.
    Quarantined,
}

/// User-visible dependency state after policy evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyDegradationState {
    /// Exact dependency or qualified substitute is ready.
    Ready,
    /// Required loss blocks execution.
    Blocked,
    /// Optional loss visibly reduces capability.
    Degraded,
    /// No valid replacement exists for a substitutable dependency.
    Unavailable,
    /// Dependency is explicitly disabled.
    Disabled,
    /// Dependency is quarantined and cannot be selected.
    Quarantined,
}

impl DependencyDegradationState {
    const fn severity(self) -> u8 {
        match self {
            Self::Ready => 0,
            Self::Degraded => 1,
            Self::Disabled => 2,
            Self::Unavailable => 3,
            Self::Blocked => 4,
            Self::Quarantined => 5,
        }
    }
}

/// Security and completion ceiling that a replacement may not weaken.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyPolicyEnvelope {
    /// Exact data-disclosure policy digest.
    pub data_policy_sha256: String,
    /// Maximum authority classes available to this dependency.
    pub authority_classes: BTreeSet<String>,
    /// Monotonic security strength; a substitute must be equal or greater.
    pub security_strength: u16,
    /// Monotonic verification strength; a substitute must be equal or greater.
    pub verification_strength: u16,
    /// Exact completion-policy digest.
    pub completion_policy_sha256: String,
}

/// One exact dependency admitted for a workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDependency {
    /// Stable dependency identity.
    pub dependency_id: String,
    /// Closed dependency family.
    pub kind: RuntimeDependencyKind,
    /// Workflow-local requirement class.
    pub requirement: DependencyRequirement,
    /// Exact expected implementation or protocol version.
    pub version: String,
    /// Digest of the exact expected implementation, artifact, or configuration.
    pub identity_sha256: String,
    /// Exact approved substitution-policy digest only for a substitutable dependency.
    pub substitution_policy_sha256: Option<String>,
    /// Policy ceiling the dependency must preserve.
    pub policy: DependencyPolicyEnvelope,
}

/// Current trusted observation for one dependency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDependencyObservation {
    /// Observed dependency identity.
    pub dependency_id: String,
    /// Exact observed version.
    pub version: String,
    /// Exact observed identity digest.
    pub identity_sha256: String,
    /// Current lifecycle/health state.
    pub health: DependencyHealth,
    /// Monotonic qualification generation observed now.
    pub qualification_generation: u64,
}

/// Explicit policy authorizing one fresh replacement selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualifiedDependencySubstitution {
    /// Unavailable dependency being replaced.
    pub dependency_id: String,
    /// Explicitly selected registered replacement.
    pub substitute_dependency_id: String,
    /// Stable qualification record identity.
    pub qualification_id: String,
    /// Exact nonzero substitution-policy digest.
    pub policy_sha256: String,
    /// Qualification generation bound by the decision.
    pub qualification_generation: u64,
    /// Whether current trusted qualification admitted the selection.
    pub qualified: bool,
    /// Whether the route/replacement is visible to the caller.
    pub user_visible: bool,
    /// Stable content-free selection reason.
    pub reason_code: String,
}

/// Visible result for one classified dependency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyDegradationResult {
    /// Original dependency identity.
    pub dependency_id: String,
    /// Original requirement class.
    pub requirement: DependencyRequirement,
    /// Resulting visible lifecycle state.
    pub state: DependencyDegradationState,
    /// Whether this dependency disposition permits the workflow to proceed.
    pub execution_permitted: bool,
    /// Selected replacement only after fresh qualification.
    pub selected_dependency_id: Option<String>,
    /// Stable content-free reason.
    pub reason_code: String,
}

/// Complete closed evaluation for one workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDependencyDisposition {
    /// Most severe visible dependency state.
    pub state: DependencyDegradationState,
    /// Whether execution may proceed under the exact evaluated set.
    pub execution_permitted: bool,
    /// One result for every registered dependency, in registry order.
    pub dependencies: Vec<DependencyDegradationResult>,
}

/// Stable fail-closed dependency-evaluation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyDegradationError {
    /// Registry or observation fields are invalid.
    InvalidInput,
    /// A dependency or observation identity is duplicated or unaccounted.
    InventoryMismatch,
    /// A substitution is absent, stale, unqualified, or weakens policy.
    SubstitutionDenied,
}

impl DependencyDegradationError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "runtime.dependency.input-invalid",
            Self::InventoryMismatch => "runtime.dependency.inventory-mismatch",
            Self::SubstitutionDenied => "runtime.dependency.substitution-denied",
        }
    }
}

impl std::fmt::Display for DependencyDegradationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DependencyDegradationError {}

/// Evaluates every registered dependency without omission or implicit fallback.
pub fn evaluate_workflow_dependencies(
    dependencies: &[RuntimeDependency],
    observations: &[RuntimeDependencyObservation],
    substitutions: &[QualifiedDependencySubstitution],
) -> Result<WorkflowDependencyDisposition, DependencyDegradationError> {
    if dependencies.is_empty() || dependencies.len() > MAX_DEPENDENCIES {
        return Err(DependencyDegradationError::InvalidInput);
    }
    let registry = unique_dependencies(dependencies)?;
    let observed = unique_observations(observations, &registry)?;
    let policies = unique_substitutions(substitutions, &registry)?;
    let mut results = Vec::with_capacity(dependencies.len());
    for dependency in dependencies {
        let observation = observed[dependency.dependency_id.as_str()];
        let exact_available = observation.health == DependencyHealth::Available
            && observation.version == dependency.version
            && observation.identity_sha256 == dependency.identity_sha256
            && observation.qualification_generation > 0;
        let item = if exact_available {
            if policies.contains_key(dependency.dependency_id.as_str()) {
                return Err(DependencyDegradationError::SubstitutionDenied);
            }
            result(
                dependency,
                DependencyDegradationState::Ready,
                None,
                "runtime.dependency.ready",
            )
        } else {
            evaluate_loss(dependency, observation, &registry, &observed, &policies)?
        };
        results.push(item);
    }
    let state = results
        .iter()
        .map(|item| item.state)
        .max_by_key(|state| state.severity())
        .unwrap_or(DependencyDegradationState::Blocked);
    Ok(WorkflowDependencyDisposition {
        state,
        execution_permitted: results.iter().all(|item| item.execution_permitted),
        dependencies: results,
    })
}

fn evaluate_loss(
    dependency: &RuntimeDependency,
    observation: &RuntimeDependencyObservation,
    registry: &BTreeMap<&str, &RuntimeDependency>,
    observations: &BTreeMap<&str, &RuntimeDependencyObservation>,
    substitutions: &BTreeMap<&str, &QualifiedDependencySubstitution>,
) -> Result<DependencyDegradationResult, DependencyDegradationError> {
    let direct = match observation.health {
        DependencyHealth::Disabled => Some((
            DependencyDegradationState::Disabled,
            "runtime.dependency.disabled",
        )),
        DependencyHealth::Quarantined => Some((
            DependencyDegradationState::Quarantined,
            "runtime.dependency.quarantined",
        )),
        _ => None,
    };
    if let Some((state, code)) = direct {
        if substitutions.contains_key(dependency.dependency_id.as_str()) {
            return Err(DependencyDegradationError::SubstitutionDenied);
        }
        return Ok(result(dependency, state, None, code));
    }
    match dependency.requirement {
        DependencyRequirement::Required => {
            deny_unexpected_substitution(dependency, substitutions)?;
            Ok(result(
                dependency,
                DependencyDegradationState::Blocked,
                None,
                "runtime.dependency.required-unavailable",
            ))
        }
        DependencyRequirement::Optional => {
            deny_unexpected_substitution(dependency, substitutions)?;
            Ok(result(
                dependency,
                DependencyDegradationState::Degraded,
                None,
                "runtime.dependency.optional-unavailable",
            ))
        }
        DependencyRequirement::Substitutable => {
            let Some(policy) = substitutions.get(dependency.dependency_id.as_str()) else {
                return Ok(result(
                    dependency,
                    DependencyDegradationState::Unavailable,
                    None,
                    "runtime.dependency.substitute-unavailable",
                ));
            };
            let substitute = registry
                .get(policy.substitute_dependency_id.as_str())
                .ok_or(DependencyDegradationError::SubstitutionDenied)?;
            let current = observations
                .get(policy.substitute_dependency_id.as_str())
                .ok_or(DependencyDegradationError::SubstitutionDenied)?;
            if substitute.kind != dependency.kind
                || substitute.dependency_id == dependency.dependency_id
                || !policy.qualified
                || !policy.user_visible
                || !valid_identifier(&policy.qualification_id)
                || !valid_code(&policy.reason_code)
                || !valid_sha256(&policy.policy_sha256)
                || dependency.substitution_policy_sha256.as_deref()
                    != Some(policy.policy_sha256.as_str())
                || current.health != DependencyHealth::Available
                || current.version != substitute.version
                || current.identity_sha256 != substitute.identity_sha256
                || policy.qualification_generation == 0
                || policy.qualification_generation != current.qualification_generation
                || !preserves_policy(&dependency.policy, &substitute.policy)
            {
                return Err(DependencyDegradationError::SubstitutionDenied);
            }
            Ok(result(
                dependency,
                DependencyDegradationState::Ready,
                Some(substitute.dependency_id.clone()),
                &policy.reason_code,
            ))
        }
    }
}

fn deny_unexpected_substitution(
    dependency: &RuntimeDependency,
    substitutions: &BTreeMap<&str, &QualifiedDependencySubstitution>,
) -> Result<(), DependencyDegradationError> {
    if substitutions.contains_key(dependency.dependency_id.as_str()) {
        Err(DependencyDegradationError::SubstitutionDenied)
    } else {
        Ok(())
    }
}

fn preserves_policy(
    required: &DependencyPolicyEnvelope,
    candidate: &DependencyPolicyEnvelope,
) -> bool {
    valid_envelope(required)
        && valid_envelope(candidate)
        && required.data_policy_sha256 == candidate.data_policy_sha256
        && required.completion_policy_sha256 == candidate.completion_policy_sha256
        && candidate
            .authority_classes
            .is_subset(&required.authority_classes)
        && candidate.security_strength >= required.security_strength
        && candidate.verification_strength >= required.verification_strength
}

fn unique_dependencies(
    values: &[RuntimeDependency],
) -> Result<BTreeMap<&str, &RuntimeDependency>, DependencyDegradationError> {
    let mut result = BTreeMap::new();
    for value in values {
        let substitution_policy_valid = match value.requirement {
            DependencyRequirement::Substitutable => value
                .substitution_policy_sha256
                .as_deref()
                .is_some_and(valid_sha256),
            DependencyRequirement::Required | DependencyRequirement::Optional => {
                value.substitution_policy_sha256.is_none()
            }
        };
        if !valid_identifier(&value.dependency_id)
            || value.version.is_empty()
            || value.version.len() > 128
            || !valid_sha256(&value.identity_sha256)
            || !valid_envelope(&value.policy)
            || !substitution_policy_valid
            || result.insert(value.dependency_id.as_str(), value).is_some()
        {
            return Err(DependencyDegradationError::InvalidInput);
        }
    }
    Ok(result)
}

fn unique_observations<'a>(
    values: &'a [RuntimeDependencyObservation],
    registry: &BTreeMap<&str, &RuntimeDependency>,
) -> Result<BTreeMap<&'a str, &'a RuntimeDependencyObservation>, DependencyDegradationError> {
    if values.len() != registry.len() {
        return Err(DependencyDegradationError::InventoryMismatch);
    }
    let mut result = BTreeMap::new();
    for value in values {
        if !registry.contains_key(value.dependency_id.as_str())
            || value.version.is_empty()
            || !valid_sha256(&value.identity_sha256)
            || result.insert(value.dependency_id.as_str(), value).is_some()
        {
            return Err(DependencyDegradationError::InventoryMismatch);
        }
    }
    Ok(result)
}

fn unique_substitutions<'a>(
    values: &'a [QualifiedDependencySubstitution],
    registry: &BTreeMap<&str, &RuntimeDependency>,
) -> Result<BTreeMap<&'a str, &'a QualifiedDependencySubstitution>, DependencyDegradationError> {
    let mut result = BTreeMap::new();
    for value in values {
        if !registry.contains_key(value.dependency_id.as_str())
            || !registry.contains_key(value.substitute_dependency_id.as_str())
            || result.insert(value.dependency_id.as_str(), value).is_some()
        {
            return Err(DependencyDegradationError::InventoryMismatch);
        }
    }
    Ok(result)
}

fn result(
    dependency: &RuntimeDependency,
    state: DependencyDegradationState,
    selected_dependency_id: Option<String>,
    reason_code: &str,
) -> DependencyDegradationResult {
    DependencyDegradationResult {
        dependency_id: dependency.dependency_id.clone(),
        requirement: dependency.requirement,
        state,
        execution_permitted: matches!(
            state,
            DependencyDegradationState::Ready | DependencyDegradationState::Degraded
        ) || dependency.requirement == DependencyRequirement::Optional,
        selected_dependency_id,
        reason_code: reason_code.to_owned(),
    }
}

fn valid_envelope(value: &DependencyPolicyEnvelope) -> bool {
    valid_sha256(&value.data_policy_sha256)
        && valid_sha256(&value.completion_policy_sha256)
        && value.security_strength > 0
        && value.verification_strength > 0
        && value.authority_classes.len() <= 64
        && value
            .authority_classes
            .iter()
            .all(|item| valid_identifier(item))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_code(value: &str) -> bool {
    valid_identifier(value) && value.contains('.')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
