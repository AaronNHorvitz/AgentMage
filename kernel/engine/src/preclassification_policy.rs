//! Deterministic deny-first policy checks that precede advisory classification.

use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    ActionId, ActionRisk, AuthorityClass, AutonomyLevel, BudgetState, CredentialClass,
    DeterministicPolicyFacts, DisclosureClass, ExactAuthorityState, ModelCapabilityStatus,
    NetworkRequirement, PathScopeState, PolicyDestinationClass, RepositoryState,
    StaticPolicyCheckKind, StaticPolicyCheckState, TaskId, to_canonical_json,
};
use sha2::{Digest, Sha256};

const STATIC_CHECK_ORDER: [StaticPolicyCheckKind; 4] = [
    StaticPolicyCheckKind::Secret,
    StaticPolicyCheckKind::Path,
    StaticPolicyCheckKind::ExecutableContent,
    StaticPolicyCheckKind::Destination,
];

/// Stable reason deterministic pre-classification policy denied or rejected facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreclassificationDenialReason {
    /// Required identities, hashes, fields, or static-check closure are malformed.
    InvalidFacts,
    /// Secret detection did not establish a clear result.
    Secret,
    /// Path checks or typed path scope did not establish an in-scope result.
    Path,
    /// Executable-content detection did not establish a clear or inapplicable result.
    ExecutableContent,
    /// Destination checks or typed destination identity did not establish eligibility.
    Destination,
    /// Repository state is conflicted or unknown.
    RepositoryState,
    /// Credential class is exposed, unknown, or lacks exact credential authority.
    CredentialClass,
    /// Disclosure class is unknown, public, or outside autonomy and authority.
    Disclosure,
    /// A state-changing operation is not mechanically reversible.
    Reversibility,
    /// Network requirement is unknown or outside autonomy and authority.
    Network,
    /// Budget is at, beyond, or missing an applicable ceiling.
    Budget,
    /// Exact current authority is absent, stale, consumed, pending, or uncertain.
    ExactAuthority,
    /// Requested operation exceeds the current autonomy ceiling.
    Autonomy,
    /// Required model role lacks current measured support.
    ModelCapability,
}

/// Content-free deterministic policy denial with stable check-order position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreclassificationDenial {
    reason: PreclassificationDenialReason,
    completed_checks: u8,
}

impl PreclassificationDenial {
    /// Returns the exact denial class.
    #[must_use]
    pub const fn reason(&self) -> PreclassificationDenialReason {
        self.reason
    }

    /// Returns the number of ordered checks completed before denial.
    #[must_use]
    pub const fn completed_checks(&self) -> u8 {
        self.completed_checks
    }
}

/// Opaque proof that one fact set passed every deterministic pre-classification check.
///
/// This type has no public constructor, clone implementation, serialization, grant, operation,
/// execution, or completion method. A later advisory boundary may only narrow this result.
#[derive(Debug, PartialEq, Eq)]
pub struct PreclassificationClearance {
    fact_set_sha256: String,
    policy_sha256: String,
    task_id: TaskId,
    action_id: ActionId,
    completed_checks: u8,
}

impl PreclassificationClearance {
    /// Returns the digest of the exact canonical deterministic fact set.
    #[must_use]
    pub fn fact_set_sha256(&self) -> &str {
        &self.fact_set_sha256
    }

    /// Returns the exact deterministic policy revision digest.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Returns the complete ordered deterministic check count.
    #[must_use]
    pub const fn completed_checks(&self) -> u8 {
        self.completed_checks
    }

    pub(crate) fn matches_advisory_identity(
        &self,
        task_id: &TaskId,
        action_id: Option<&ActionId>,
        input_sha256: &str,
    ) -> bool {
        self.task_id == *task_id
            && action_id == Some(&self.action_id)
            && self.fact_set_sha256 == input_sha256
    }

    pub(crate) fn matches_reclassification_identity(
        &self,
        task_id: &TaskId,
        action_id: &ActionId,
        fact_set_sha256: &str,
    ) -> bool {
        self.task_id == *task_id
            && self.action_id == *action_id
            && self.fact_set_sha256 == fact_set_sha256
    }
}

/// Stateless deterministic gate that must run before advisory classification.
pub struct PreclassificationPolicyGate;

impl PreclassificationPolicyGate {
    /// Evaluates one exact fact set in fixed deny-first order.
    pub fn evaluate(
        facts: &DeterministicPolicyFacts,
    ) -> Result<PreclassificationClearance, PreclassificationDenial> {
        validate_facts(facts)?;

        for (index, (expected, check)) in STATIC_CHECK_ORDER
            .iter()
            .zip(&facts.static_checks)
            .enumerate()
        {
            debug_assert_eq!(expected, &check.kind);
            let admitted = match check.kind {
                StaticPolicyCheckKind::Path => {
                    check.state == StaticPolicyCheckState::Clear
                        && matches!(
                            facts.path_state,
                            PathScopeState::InScope | PathScopeState::NotApplicable
                        )
                }
                StaticPolicyCheckKind::ExecutableContent => matches!(
                    check.state,
                    StaticPolicyCheckState::Clear | StaticPolicyCheckState::NotApplicable
                ),
                StaticPolicyCheckKind::Destination => {
                    check.state == StaticPolicyCheckState::Clear
                        && facts.destination != PolicyDestinationClass::Unknown
                }
                _ => check.state == StaticPolicyCheckState::Clear,
            };
            if !admitted {
                return Err(denial(static_reason(check.kind), index));
            }
        }

        let mut completed = STATIC_CHECK_ORDER.len();
        if matches!(
            facts.repository_state,
            RepositoryState::Conflicted | RepositoryState::Unknown
        ) {
            return Err(denial(
                PreclassificationDenialReason::RepositoryState,
                completed,
            ));
        }
        completed += 1;
        if credential_denied(facts) {
            return Err(denial(
                PreclassificationDenialReason::CredentialClass,
                completed,
            ));
        }
        completed += 1;
        if disclosure_denied(facts) {
            return Err(denial(PreclassificationDenialReason::Disclosure, completed));
        }
        completed += 1;
        if !facts.reversible
            && matches!(
                facts.operation.authority_class(),
                AuthorityClass::LocalWrite
                    | AuthorityClass::RemoteWrite
                    | AuthorityClass::Deploy
                    | AuthorityClass::Admin
            )
        {
            return Err(denial(
                PreclassificationDenialReason::Reversibility,
                completed,
            ));
        }
        completed += 1;
        if network_denied(facts) {
            return Err(denial(PreclassificationDenialReason::Network, completed));
        }
        completed += 1;
        if facts.budget != BudgetState::Within {
            return Err(denial(PreclassificationDenialReason::Budget, completed));
        }
        completed += 1;
        if facts.exact_authority != ExactAuthorityState::CurrentExact {
            return Err(denial(
                PreclassificationDenialReason::ExactAuthority,
                completed,
            ));
        }
        completed += 1;
        if autonomy_denied(facts) {
            return Err(denial(PreclassificationDenialReason::Autonomy, completed));
        }
        completed += 1;
        if facts.model_capability_status != ModelCapabilityStatus::MeasuredSupported {
            return Err(denial(
                PreclassificationDenialReason::ModelCapability,
                completed,
            ));
        }
        completed += 1;

        let bytes = to_canonical_json(facts)
            .map_err(|_| denial(PreclassificationDenialReason::InvalidFacts, 0))?;
        Ok(PreclassificationClearance {
            fact_set_sha256: sha256_hex(&bytes),
            policy_sha256: facts.policy_sha256.clone(),
            task_id: facts.task_id.clone(),
            action_id: facts.action_id.clone(),
            completed_checks: u8::try_from(completed).expect("fixed check count fits u8"),
        })
    }
}

fn validate_facts(facts: &DeterministicPolicyFacts) -> Result<(), PreclassificationDenial> {
    let valid_static_checks = facts.static_checks.len() == STATIC_CHECK_ORDER.len()
        && facts
            .static_checks
            .iter()
            .map(|check| check.kind)
            .eq(STATIC_CHECK_ORDER)
        && facts
            .static_checks
            .iter()
            .all(|check| valid_sha256(&check.observation_sha256));
    if facts.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        || !valid_id(&facts.fact_set_id)
        || !valid_id(facts.policy_id.as_str())
        || !valid_sha256(&facts.policy_sha256)
        || !valid_id(&facts.actor_id)
        || !valid_id(facts.session_id.as_str())
        || !valid_id(facts.task_id.as_str())
        || !valid_id(facts.action_id.as_str())
        || !valid_sha256(&facts.source_sha256)
        || !valid_sha256(&facts.destination_sha256)
        || !valid_sha256(&facts.path_sha256)
        || !risk_matches(facts)
        || !valid_static_checks
    {
        return Err(denial(PreclassificationDenialReason::InvalidFacts, 0));
    }
    Ok(())
}

fn static_reason(kind: StaticPolicyCheckKind) -> PreclassificationDenialReason {
    match kind {
        StaticPolicyCheckKind::Secret => PreclassificationDenialReason::Secret,
        StaticPolicyCheckKind::Path => PreclassificationDenialReason::Path,
        StaticPolicyCheckKind::ExecutableContent => {
            PreclassificationDenialReason::ExecutableContent
        }
        StaticPolicyCheckKind::Destination => PreclassificationDenialReason::Destination,
    }
}

fn credential_denied(facts: &DeterministicPolicyFacts) -> bool {
    match facts.credential_class {
        CredentialClass::None => false,
        CredentialClass::Brokered | CredentialClass::PlatformStore => {
            facts.exact_authority != ExactAuthorityState::CurrentExact
        }
        CredentialClass::Environment | CredentialClass::File | CredentialClass::Unknown => true,
    }
}

fn disclosure_denied(facts: &DeterministicPolicyFacts) -> bool {
    match facts.disclosure {
        DisclosureClass::None | DisclosureClass::LocalOnly => false,
        DisclosureClass::SameTenant => !connected(facts.autonomy),
        DisclosureClass::External => {
            !connected(facts.autonomy) || facts.exact_authority != ExactAuthorityState::CurrentExact
        }
        DisclosureClass::Public | DisclosureClass::Unknown => true,
    }
}

fn network_denied(facts: &DeterministicPolicyFacts) -> bool {
    match facts.network {
        NetworkRequirement::None | NetworkRequirement::Loopback => false,
        NetworkRequirement::LocalNetwork | NetworkRequirement::Internet => {
            !connected(facts.autonomy) || facts.exact_authority != ExactAuthorityState::CurrentExact
        }
        NetworkRequirement::Unknown => true,
    }
}

fn autonomy_denied(facts: &DeterministicPolicyFacts) -> bool {
    match facts.autonomy {
        AutonomyLevel::Disabled => true,
        AutonomyLevel::Inspect => !matches!(
            facts.operation.authority_class(),
            AuthorityClass::Observe | AuthorityClass::Draft
        ),
        AutonomyLevel::WorkspaceAutonomous => matches!(
            facts.operation.authority_class(),
            AuthorityClass::RemoteWrite
                | AuthorityClass::Deploy
                | AuthorityClass::Secrets
                | AuthorityClass::Admin
        ),
        AutonomyLevel::ConnectedOperations | AutonomyLevel::Owner => false,
    }
}

const fn connected(autonomy: AutonomyLevel) -> bool {
    matches!(
        autonomy,
        AutonomyLevel::ConnectedOperations | AutonomyLevel::Owner
    )
}

fn risk_matches(facts: &DeterministicPolicyFacts) -> bool {
    let expected = match facts.operation.authority_class() {
        AuthorityClass::Observe | AuthorityClass::Draft => ActionRisk::Minimal,
        AuthorityClass::LocalWrite if facts.reversible => ActionRisk::Controlled,
        AuthorityClass::LocalWrite | AuthorityClass::RemoteWrite | AuthorityClass::Execute => {
            ActionRisk::Elevated
        }
        AuthorityClass::Deploy | AuthorityClass::Secrets | AuthorityClass::Admin => {
            ActionRisk::Critical
        }
    };
    facts.action_risk == expected
}

fn denial(reason: PreclassificationDenialReason, completed: usize) -> PreclassificationDenial {
    PreclassificationDenial {
        reason,
        completed_checks: u8::try_from(completed).expect("fixed check count fits u8"),
    }
}

fn sha256_hex(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{PreclassificationDenialReason, PreclassificationPolicyGate, STATIC_CHECK_ORDER};
    use agentmage_kernel_contracts::{
        ActionId, ActionRisk, AutonomyLevel, BudgetState, CredentialClass, DataSensitivity,
        DeterministicPolicyFacts, DisclosureClass, ExactAuthorityState, GrantOperation,
        ModelCapabilityRole, ModelCapabilityStatus, NetworkRequirement, OperationBinding,
        PathScopeState, PolicyDestinationClass, PolicyId, PolicySourceClass, RepositoryState,
        SessionId, StaticPolicyCheck, StaticPolicyCheckKind, StaticPolicyCheckState, TaskId,
    };

    const SHA256: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn facts() -> DeterministicPolicyFacts {
        DeterministicPolicyFacts {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            fact_set_id: "facts-0001".to_owned(),
            policy_id: PolicyId::from_raw("policy-0001"),
            policy_sha256: SHA256.to_owned(),
            actor_id: "actor-0001".to_owned(),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: ActionId::from_raw("action-0001"),
            autonomy: AutonomyLevel::Inspect,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            source: PolicySourceClass::LocalWorkspace,
            source_sha256: SHA256.to_owned(),
            destination: PolicyDestinationClass::UserDisplay,
            destination_sha256: SHA256.to_owned(),
            path_state: PathScopeState::InScope,
            path_sha256: SHA256.to_owned(),
            repository_state: RepositoryState::Clean,
            credential_class: CredentialClass::None,
            data_sensitivity: DataSensitivity::Ephemeral,
            action_risk: ActionRisk::Minimal,
            model_capability_role: ModelCapabilityRole::CodeGeneration,
            model_capability_status: ModelCapabilityStatus::MeasuredSupported,
            reversible: true,
            network: NetworkRequirement::None,
            disclosure: DisclosureClass::LocalOnly,
            budget: BudgetState::Within,
            exact_authority: ExactAuthorityState::CurrentExact,
            static_checks: STATIC_CHECK_ORDER
                .into_iter()
                .map(|kind| StaticPolicyCheck {
                    kind,
                    state: if kind == StaticPolicyCheckKind::ExecutableContent {
                        StaticPolicyCheckState::NotApplicable
                    } else {
                        StaticPolicyCheckState::Clear
                    },
                    observation_sha256: SHA256.to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn task_12_2_2_2_exact_facts_clear_all_checks_deterministically() {
        let first = PreclassificationPolicyGate::evaluate(&facts()).expect("facts clear");
        let repeated = PreclassificationPolicyGate::evaluate(&facts()).expect("facts clear");
        assert_eq!(first, repeated);
        assert_eq!(first.completed_checks(), 13);
        assert_eq!(first.policy_sha256(), SHA256);
        assert_eq!(first.fact_set_sha256().len(), 64);
    }

    #[test]
    fn task_12_2_2_2_static_checks_run_first_in_fixed_order() {
        let reasons = [
            PreclassificationDenialReason::Secret,
            PreclassificationDenialReason::Path,
            PreclassificationDenialReason::ExecutableContent,
            PreclassificationDenialReason::Destination,
        ];
        for (index, reason) in reasons.into_iter().enumerate() {
            for state in [
                StaticPolicyCheckState::Detected,
                StaticPolicyCheckState::Unknown,
            ] {
                let mut candidate = facts();
                candidate.static_checks[index].state = state;
                let denied = PreclassificationPolicyGate::evaluate(&candidate)
                    .expect_err("static hazard denies");
                assert_eq!(denied.reason(), reason);
                assert_eq!(denied.completed_checks(), index as u8);
            }
        }

        let mut path = facts();
        path.path_state = PathScopeState::OutOfScope;
        assert_eq!(
            PreclassificationPolicyGate::evaluate(&path)
                .expect_err("path denies")
                .reason(),
            PreclassificationDenialReason::Path
        );
        let mut destination = facts();
        destination.destination = PolicyDestinationClass::Unknown;
        assert_eq!(
            PreclassificationPolicyGate::evaluate(&destination)
                .expect_err("destination denies")
                .reason(),
            PreclassificationDenialReason::Destination
        );
    }

    #[test]
    fn task_12_2_2_2_typed_policy_checks_follow_static_checks_in_order() {
        let mut cases = Vec::new();

        let mut repository = facts();
        repository.repository_state = RepositoryState::Unknown;
        cases.push((
            repository,
            PreclassificationDenialReason::RepositoryState,
            4,
        ));

        let mut credential = facts();
        credential.credential_class = CredentialClass::File;
        cases.push((
            credential,
            PreclassificationDenialReason::CredentialClass,
            5,
        ));

        let mut disclosure = facts();
        disclosure.disclosure = DisclosureClass::Public;
        cases.push((disclosure, PreclassificationDenialReason::Disclosure, 6));

        let mut irreversible = facts();
        irreversible.operation = OperationBinding::new(GrantOperation::WorkspaceWrite);
        irreversible.action_risk = ActionRisk::Elevated;
        irreversible.reversible = false;
        irreversible.autonomy = AutonomyLevel::WorkspaceAutonomous;
        cases.push((
            irreversible,
            PreclassificationDenialReason::Reversibility,
            7,
        ));

        let mut network = facts();
        network.network = NetworkRequirement::Unknown;
        cases.push((network, PreclassificationDenialReason::Network, 8));

        let mut budget = facts();
        budget.budget = BudgetState::AtLimit;
        cases.push((budget, PreclassificationDenialReason::Budget, 9));

        let mut authority = facts();
        authority.exact_authority = ExactAuthorityState::Consumed;
        cases.push((authority, PreclassificationDenialReason::ExactAuthority, 10));

        let mut autonomy = facts();
        autonomy.autonomy = AutonomyLevel::Disabled;
        cases.push((autonomy, PreclassificationDenialReason::Autonomy, 11));

        let mut capability = facts();
        capability.model_capability_status = ModelCapabilityStatus::MeasuredUnsupported;
        cases.push((
            capability,
            PreclassificationDenialReason::ModelCapability,
            12,
        ));

        for (candidate, expected, completed) in cases {
            let denied =
                PreclassificationPolicyGate::evaluate(&candidate).expect_err("typed fact denies");
            assert_eq!(denied.reason(), expected);
            assert_eq!(denied.completed_checks(), completed);
        }
    }

    #[test]
    fn task_12_2_2_2_malformed_missing_duplicate_or_reordered_facts_are_inert() {
        let mut candidates = Vec::new();
        let mut missing = facts();
        missing.static_checks.pop();
        candidates.push(missing);
        let mut duplicate = facts();
        duplicate.static_checks[1].kind = StaticPolicyCheckKind::Secret;
        candidates.push(duplicate);
        let mut reordered = facts();
        reordered.static_checks.swap(0, 1);
        candidates.push(reordered);
        let mut malformed_hash = facts();
        malformed_hash.source_sha256 = "invalid".to_owned();
        candidates.push(malformed_hash);
        let mut inconsistent_risk = facts();
        inconsistent_risk.action_risk = ActionRisk::Critical;
        candidates.push(inconsistent_risk);

        for candidate in candidates {
            let denied =
                PreclassificationPolicyGate::evaluate(&candidate).expect_err("invalid facts deny");
            assert_eq!(denied.reason(), PreclassificationDenialReason::InvalidFacts);
            assert_eq!(denied.completed_checks(), 0);
        }
    }

    #[test]
    fn task_12_2_2_2_applicable_safe_edge_states_remain_eligible() {
        let mut dirty_read = facts();
        dirty_read.repository_state = RepositoryState::Dirty;
        assert!(PreclassificationPolicyGate::evaluate(&dirty_read).is_ok());

        let mut connected = facts();
        connected.autonomy = AutonomyLevel::ConnectedOperations;
        connected.network = NetworkRequirement::Internet;
        connected.disclosure = DisclosureClass::SameTenant;
        connected.credential_class = CredentialClass::Brokered;
        assert!(PreclassificationPolicyGate::evaluate(&connected).is_ok());
    }

    #[test]
    fn task_12_2_2_2_denial_occurs_before_any_semantic_classifier_call() {
        let mut semantic_classifier_calls = 0_u32;
        let mut denied = facts();
        denied.static_checks[0].state = StaticPolicyCheckState::Detected;
        if PreclassificationPolicyGate::evaluate(&denied).is_ok() {
            semantic_classifier_calls += 1;
        }
        assert_eq!(semantic_classifier_calls, 0);

        if PreclassificationPolicyGate::evaluate(&facts()).is_ok() {
            semantic_classifier_calls += 1;
        }
        assert_eq!(semantic_classifier_calls, 1);
    }

    #[test]
    fn d027_s12_policy_5120_seeded_fact_mutations_are_deterministic() {
        const DIMENSION_COUNT: usize = 16;
        const MUTATIONS_PER_DIMENSION: usize = 320;
        const MUTATION_COUNT: usize = DIMENSION_COUNT * MUTATIONS_PER_DIMENSION;

        let autonomy_values = [
            AutonomyLevel::Disabled,
            AutonomyLevel::Inspect,
            AutonomyLevel::WorkspaceAutonomous,
            AutonomyLevel::ConnectedOperations,
            AutonomyLevel::Owner,
        ];
        let source_values = [
            PolicySourceClass::LocalWorkspace,
            PolicySourceClass::Attachment,
            PolicySourceClass::ToolOutput,
            PolicySourceClass::Connector,
            PolicySourceClass::Network,
            PolicySourceClass::Generated,
            PolicySourceClass::Unknown,
        ];
        let destination_values = [
            PolicyDestinationClass::None,
            PolicyDestinationClass::LocalWorkspace,
            PolicyDestinationClass::LocalStore,
            PolicyDestinationClass::LocalProcess,
            PolicyDestinationClass::UserDisplay,
            PolicyDestinationClass::RemoteService,
            PolicyDestinationClass::Export,
            PolicyDestinationClass::Unknown,
        ];
        let path_values = [
            PathScopeState::InScope,
            PathScopeState::OutOfScope,
            PathScopeState::Ambiguous,
            PathScopeState::Missing,
            PathScopeState::NotApplicable,
        ];
        let repository_values = [
            RepositoryState::Clean,
            RepositoryState::Dirty,
            RepositoryState::Conflicted,
            RepositoryState::Detached,
            RepositoryState::Unborn,
            RepositoryState::NotRepository,
            RepositoryState::Unknown,
        ];
        let credential_values = [
            CredentialClass::None,
            CredentialClass::Brokered,
            CredentialClass::Environment,
            CredentialClass::File,
            CredentialClass::PlatformStore,
            CredentialClass::Unknown,
        ];
        let sensitivity_values = [
            DataSensitivity::Ephemeral,
            DataSensitivity::Operational,
            DataSensitivity::Durable,
            DataSensitivity::Restricted,
        ];
        let network_values = [
            NetworkRequirement::None,
            NetworkRequirement::Loopback,
            NetworkRequirement::LocalNetwork,
            NetworkRequirement::Internet,
            NetworkRequirement::Unknown,
        ];
        let disclosure_values = [
            DisclosureClass::None,
            DisclosureClass::LocalOnly,
            DisclosureClass::SameTenant,
            DisclosureClass::External,
            DisclosureClass::Public,
            DisclosureClass::Unknown,
        ];
        let budget_values = [
            BudgetState::Within,
            BudgetState::AtLimit,
            BudgetState::Exceeded,
            BudgetState::Unknown,
        ];
        let authority_values = [
            ExactAuthorityState::None,
            ExactAuthorityState::Pending,
            ExactAuthorityState::CurrentExact,
            ExactAuthorityState::Stale,
            ExactAuthorityState::Consumed,
            ExactAuthorityState::Uncertain,
        ];

        let mut seed = 0xd027_5120_5eed_u64;
        let mut dimension_counts = [0_usize; DIMENSION_COUNT];
        let mut clearance_count = 0_usize;
        let mut denial_count = 0_usize;

        for index in 0..MUTATION_COUNT {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let sample = usize::try_from(seed % 10_000).expect("bounded sample fits usize");
            let dimension = index % DIMENSION_COUNT;
            dimension_counts[dimension] += 1;
            let mut candidate = facts();
            match dimension {
                0 => candidate.actor_id = format!("actor-seeded-{sample:04}"),
                1 => candidate.session_id = SessionId::from_raw(format!("session-{sample:04}")),
                2 => candidate.task_id = TaskId::from_raw(format!("task-{sample:04}")),
                3 => candidate.autonomy = autonomy_values[sample % autonomy_values.len()],
                4 => {
                    candidate.operation = OperationBinding::new(
                        GrantOperation::ALL[sample % GrantOperation::ALL.len()],
                    );
                }
                5 => candidate.source = source_values[sample % source_values.len()],
                6 => {
                    candidate.destination = destination_values[sample % destination_values.len()];
                }
                7 => candidate.path_state = path_values[sample % path_values.len()],
                8 => {
                    candidate.repository_state =
                        repository_values[sample % repository_values.len()];
                }
                9 => {
                    candidate.credential_class =
                        credential_values[sample % credential_values.len()];
                }
                10 => {
                    candidate.data_sensitivity =
                        sensitivity_values[sample % sensitivity_values.len()];
                }
                11 => candidate.reversible = sample % 2 == 0,
                12 => candidate.network = network_values[sample % network_values.len()],
                13 => {
                    candidate.disclosure = disclosure_values[sample % disclosure_values.len()];
                }
                14 => candidate.budget = budget_values[sample % budget_values.len()],
                15 => {
                    candidate.exact_authority = authority_values[sample % authority_values.len()];
                }
                _ => unreachable!("dimension is reduced modulo 16"),
            }

            let first = PreclassificationPolicyGate::evaluate(&candidate);
            let repeated = PreclassificationPolicyGate::evaluate(&candidate);
            assert_eq!(first, repeated, "decision drift at mutation {index}");
            if first.is_ok() {
                clearance_count += 1;
            } else {
                denial_count += 1;
            }
        }

        assert_eq!(dimension_counts, [MUTATIONS_PER_DIMENSION; DIMENSION_COUNT]);
        assert_eq!(clearance_count + denial_count, MUTATION_COUNT);
        assert!(clearance_count > 0);
        assert!(denial_count > 0);
    }
}
