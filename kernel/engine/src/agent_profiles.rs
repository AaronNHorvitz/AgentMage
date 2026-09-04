//! Declarative specialist-agent profiles, admission, lint, and synthetic evaluation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

const MAX_ITEMS: usize = 64;
const MAX_TEXT_BYTES: usize = 512;

/// Closed package lifecycle. Registration never enables a profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProfileLifecycle {
    /// Editable and inadmissible.
    Draft,
    /// Validated but unavailable for execution.
    Disabled,
    /// Separately reviewed and enabled.
    Enabled,
    /// Available with explicit limitations.
    Degraded,
    /// Rejected after a security or integrity failure.
    Quarantined,
    /// Retained only for migration/history.
    Retired,
}

/// Closed compatibility disposition for one requested requirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapabilityState {
    /// Available in the injected synthetic environment.
    Available,
    /// Available only with an explicit limitation.
    Degraded,
    /// Declared but not yet tested.
    Untested,
    /// Policy denies it.
    Denied,
    /// No compatible implementation exists.
    Incompatible,
}

/// Complete declarative profile. It carries ceilings and requests, never grants.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProfileDefinition {
    /// Contract schema.
    pub schema_version: u32,
    /// Stable `AG-NN` identity.
    pub profile_id: String,
    /// Semantic package version.
    pub version: String,
    /// Owning organization or user identity.
    pub owner: String,
    /// Canonical profile name.
    pub name: String,
    /// Catalog family.
    pub family: String,
    /// Bounded purpose.
    pub purpose: String,
    /// Accepted work classes.
    pub accepted_tasks: Vec<String>,
    /// Explicit prohibited work.
    pub prohibited_tasks: Vec<String>,
    /// Model profiles this role may request.
    pub compatible_models: Vec<String>,
    /// Codec profiles this role may request.
    pub compatible_codecs: Vec<String>,
    /// Tools requested from the shared dispatcher.
    pub requested_tools: Vec<String>,
    /// Roots requested from the shared authority intersection.
    pub requested_roots: Vec<String>,
    /// Provider-object classes requested, without connections or credentials.
    pub provider_object_classes: Vec<String>,
    /// Maximum authority ceiling.
    pub authority_ceiling: String,
    /// Input contract identity.
    pub input_schema: String,
    /// Output contract identity.
    pub output_schema: String,
    /// Evidence requirements.
    pub evidence: Vec<String>,
    /// Citation requirements.
    pub citations: Vec<String>,
    /// Bounded context items.
    pub context_item_limit: u32,
    /// Bounded turns.
    pub turn_limit: u32,
    /// Bounded output bytes.
    pub output_byte_limit: u32,
    /// Bounded elapsed milliseconds.
    pub elapsed_milliseconds: u64,
    /// Approval contract identity.
    pub approval: String,
    /// Completion predicate identity.
    pub completion: String,
    /// Cancellation predicate identity.
    pub cancellation: String,
    /// Stop-condition identities.
    pub stop_conditions: Vec<String>,
    /// Retention contract identity.
    pub retention: String,
    /// Exact source digest.
    pub source_sha256: String,
    /// Detached signature digest over the reviewed source.
    pub signature_sha256: String,
    /// Package lifecycle.
    pub lifecycle: AgentProfileLifecycle,
    /// Immutable source was reviewed independently where required.
    pub independent_review: bool,
    /// User accepted the exact definition and synthetic result.
    pub user_review: bool,
    /// Profile attempts to approve itself.
    pub self_approval: bool,
    /// Profile attempts to modify or enable itself.
    pub self_modification: bool,
    /// Profile attempts recursive child creation.
    pub recursive_spawn: bool,
}

/// Stable fail-closed profile error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentProfileError {
    /// Shape, identity, digest, bounds, or required contract invalid.
    InvalidDefinition,
    /// Definition contradicts or obscures its declared contract.
    InstructionRejected,
    /// Identity/version already exists.
    Duplicate,
    /// Package is not enabled by all deterministic prerequisites and user review.
    Disabled,
    /// Synthetic environment is not completely fake and isolated.
    NonSyntheticEnvironment,
}

/// Synthetic dependency state supplied by a deterministic test harness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntheticAgentEnvironment {
    /// Files are fake.
    pub fake_files: bool,
    /// Tools are fake.
    pub fake_tools: bool,
    /// Models are fake.
    pub fake_models: bool,
    /// Connectors are fake.
    pub fake_connectors: bool,
    /// Grants are fake and authority-free.
    pub fake_grants: bool,
    /// Requirements available to the harness.
    pub available: BTreeSet<String>,
    /// Requirements explicitly denied by policy.
    pub denied: BTreeSet<String>,
    /// Requirements measured with limitations.
    pub degraded: BTreeSet<String>,
    /// Requirements present but untested.
    pub untested: BTreeSet<String>,
}

/// Attributable, effect-free result of a synthetic profile evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSyntheticReport {
    /// Profile evaluated.
    pub profile_id: String,
    /// State per declared requirement.
    pub requirements: BTreeMap<String, AgentCapabilityState>,
    /// Real external or user-data effects, always zero.
    pub real_effect_count: u32,
    /// Registration/evaluation cannot enable a profile.
    pub enabled: bool,
}

/// Immutable registry keyed by exact identity and version.
#[derive(Default)]
pub struct AgentProfileRegistry {
    profiles: BTreeMap<(String, String), AgentProfileDefinition>,
}

impl AgentProfileRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a valid definition without granting or enabling anything.
    pub fn register(&mut self, profile: AgentProfileDefinition) -> Result<(), AgentProfileError> {
        validate_agent_profile(&profile)?;
        let key = (profile.profile_id.clone(), profile.version.clone());
        if self.profiles.contains_key(&key) {
            return Err(AgentProfileError::Duplicate);
        }
        self.profiles.insert(key, profile);
        Ok(())
    }

    /// Selects only a separately enabled exact version.
    pub fn select(
        &self,
        profile_id: &str,
        version: &str,
    ) -> Result<&AgentProfileDefinition, AgentProfileError> {
        let profile = self
            .profiles
            .get(&(profile_id.to_owned(), version.to_owned()))
            .ok_or(AgentProfileError::Disabled)?;
        if profile.lifecycle != AgentProfileLifecycle::Enabled
            || !profile.user_review
            || (requires_independent_review(&profile.profile_id) && !profile.independent_review)
        {
            return Err(AgentProfileError::Disabled);
        }
        Ok(profile)
    }

    /// Returns definitions in stable identity/version order.
    #[must_use]
    pub fn profiles(&self) -> Vec<&AgentProfileDefinition> {
        self.profiles.values().collect()
    }
}

/// Validates one definition without interpreting prose as authority.
pub fn validate_agent_profile(profile: &AgentProfileDefinition) -> Result<(), AgentProfileError> {
    let lists = [
        &profile.accepted_tasks,
        &profile.prohibited_tasks,
        &profile.compatible_models,
        &profile.compatible_codecs,
        &profile.evidence,
        &profile.citations,
        &profile.stop_conditions,
    ];
    let texts = [
        &profile.version,
        &profile.owner,
        &profile.name,
        &profile.family,
        &profile.purpose,
        &profile.authority_ceiling,
        &profile.input_schema,
        &profile.output_schema,
        &profile.approval,
        &profile.completion,
        &profile.cancellation,
        &profile.retention,
    ];
    if profile.schema_version != 1
        || !valid_profile_id(&profile.profile_id)
        || texts.iter().any(|value| !valid_text(value))
        || lists
            .iter()
            .any(|items| items.is_empty() || !valid_items(items))
        || !valid_items(&profile.requested_tools)
        || !valid_items(&profile.requested_roots)
        || !valid_items(&profile.provider_object_classes)
        || !valid_sha256(&profile.source_sha256)
        || !valid_sha256(&profile.signature_sha256)
        || profile.source_sha256 == profile.signature_sha256
        || profile.context_item_limit == 0
        || profile.turn_limit == 0
        || profile.output_byte_limit == 0
        || profile.elapsed_milliseconds == 0
        || profile.self_approval
        || profile.self_modification
        || profile.recursive_spawn
    {
        return Err(AgentProfileError::InvalidDefinition);
    }
    if profile
        .accepted_tasks
        .iter()
        .any(|item| profile.prohibited_tasks.contains(item))
        || vague_completion(&profile.completion)
    {
        return Err(AgentProfileError::InstructionRejected);
    }
    if profile.lifecycle == AgentProfileLifecycle::Enabled
        && (!profile.user_review
            || (requires_independent_review(&profile.profile_id) && !profile.independent_review))
    {
        return Err(AgentProfileError::Disabled);
    }
    Ok(())
}

/// Returns creation-wizard gaps without filling or broadening them.
#[must_use]
pub fn creation_wizard_gaps(profile: &AgentProfileDefinition) -> Vec<&'static str> {
    let mut gaps = Vec::new();
    if profile.purpose.trim().is_empty() {
        gaps.push("purpose");
    }
    if profile.prohibited_tasks.is_empty() {
        gaps.push("prohibitions");
    }
    if profile.evidence.is_empty() {
        gaps.push("evidence");
    }
    if profile.requested_tools.is_empty() && profile.requested_roots.is_empty() {
        gaps.push("permissions");
    }
    if profile.stop_conditions.is_empty() {
        gaps.push("stop_conditions");
    }
    gaps
}

/// Evaluates a definition only against fully synthetic dependencies.
pub fn synthetic_dry_run(
    profile: &AgentProfileDefinition,
    environment: &SyntheticAgentEnvironment,
) -> Result<AgentSyntheticReport, AgentProfileError> {
    validate_agent_profile(profile)?;
    if !environment.fake_files
        || !environment.fake_tools
        || !environment.fake_models
        || !environment.fake_connectors
        || !environment.fake_grants
    {
        return Err(AgentProfileError::NonSyntheticEnvironment);
    }
    let requirements = profile
        .requested_tools
        .iter()
        .chain(profile.requested_roots.iter())
        .chain(profile.provider_object_classes.iter())
        .chain(profile.compatible_models.iter())
        .map(|requirement| {
            let state = if environment.denied.contains(requirement) {
                AgentCapabilityState::Denied
            } else if environment.degraded.contains(requirement) {
                AgentCapabilityState::Degraded
            } else if environment.untested.contains(requirement) {
                AgentCapabilityState::Untested
            } else if environment.available.contains(requirement) {
                AgentCapabilityState::Available
            } else {
                AgentCapabilityState::Incompatible
            };
            (requirement.clone(), state)
        })
        .collect();
    Ok(AgentSyntheticReport {
        profile_id: profile.profile_id.clone(),
        requirements,
        real_effect_count: 0,
        enabled: false,
    })
}

fn requires_independent_review(profile_id: &str) -> bool {
    matches!(
        profile_id,
        "AG-10" | "AG-11" | "AG-35" | "AG-36" | "AG-37" | "AG-38" | "AG-39" | "AG-40" | "AG-41"
    )
}

fn vague_completion(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "done" | "looks good" | "complete"
    )
}

fn valid_profile_id(value: &str) -> bool {
    value.len() == 5
        && value.starts_with("AG-")
        && value[3..]
            .parse::<u8>()
            .is_ok_and(|number| (1..=49).contains(&number))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_TEXT_BYTES
}

fn valid_items(items: &[String]) -> bool {
    items.len() <= MAX_ITEMS
        && items.iter().all(|item| valid_text(item))
        && items.iter().collect::<BTreeSet<_>>().len() == items.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str) -> AgentProfileDefinition {
        AgentProfileDefinition {
            schema_version: 1,
            profile_id: id.to_owned(),
            version: "1.0.0".to_owned(),
            owner: "local-user".to_owned(),
            name: "Test role".to_owned(),
            family: "engineering".to_owned(),
            purpose: "Produce one bounded proposal".to_owned(),
            accepted_tasks: vec!["analyze".to_owned()],
            prohibited_tasks: vec!["authorize".to_owned()],
            compatible_models: vec!["fake-model".to_owned()],
            compatible_codecs: vec!["text-v1".to_owned()],
            requested_tools: vec!["fake-read".to_owned()],
            requested_roots: vec!["fake-root".to_owned()],
            provider_object_classes: vec!["fake-object".to_owned()],
            authority_ceiling: "observe-only".to_owned(),
            input_schema: "work-packet-v1".to_owned(),
            output_schema: "proposal-v1".to_owned(),
            evidence: vec!["citations".to_owned()],
            citations: vec!["source-lines".to_owned()],
            context_item_limit: 16,
            turn_limit: 4,
            output_byte_limit: 4096,
            elapsed_milliseconds: 10_000,
            approval: "exact-user-review".to_owned(),
            completion: "verified-proposal-v1".to_owned(),
            cancellation: "runtime-cancellation-v1".to_owned(),
            stop_conditions: vec!["budget-exhausted".to_owned()],
            retention: "ephemeral-v1".to_owned(),
            source_sha256: "a".repeat(64),
            signature_sha256: "b".repeat(64),
            lifecycle: AgentProfileLifecycle::Disabled,
            independent_review: false,
            user_review: false,
            self_approval: false,
            self_modification: false,
            recursive_spawn: false,
        }
    }

    #[test]
    fn registration_is_inert_and_exact() {
        let mut registry = AgentProfileRegistry::new();
        registry.register(profile("AG-01")).unwrap();
        assert_eq!(registry.profiles().len(), 1);
        assert_eq!(
            registry.select("AG-01", "1.0.0"),
            Err(AgentProfileError::Disabled)
        );
        assert_eq!(
            registry.register(profile("AG-01")),
            Err(AgentProfileError::Duplicate)
        );
    }

    #[test]
    fn invalid_and_self_expanding_definitions_fail_closed() {
        let mut candidate = profile("AG-50");
        assert_eq!(
            validate_agent_profile(&candidate),
            Err(AgentProfileError::InvalidDefinition)
        );
        candidate = profile("AG-07");
        candidate.self_modification = true;
        assert_eq!(
            validate_agent_profile(&candidate),
            Err(AgentProfileError::InvalidDefinition)
        );
        candidate = profile("AG-07");
        candidate.completion = "done".to_owned();
        assert_eq!(
            validate_agent_profile(&candidate),
            Err(AgentProfileError::InstructionRejected)
        );
    }

    #[test]
    fn review_roles_require_independent_review_and_every_role_requires_user_review() {
        let mut candidate = profile("AG-10");
        candidate.lifecycle = AgentProfileLifecycle::Enabled;
        candidate.user_review = true;
        assert_eq!(
            validate_agent_profile(&candidate),
            Err(AgentProfileError::Disabled)
        );
        candidate.independent_review = true;
        assert_eq!(validate_agent_profile(&candidate), Ok(()));
    }

    #[test]
    fn wizard_surfaces_missing_contracts() {
        let mut candidate = profile("AG-02");
        candidate.purpose.clear();
        candidate.prohibited_tasks.clear();
        candidate.evidence.clear();
        candidate.requested_tools.clear();
        candidate.requested_roots.clear();
        candidate.stop_conditions.clear();
        assert_eq!(
            creation_wizard_gaps(&candidate),
            vec![
                "purpose",
                "prohibitions",
                "evidence",
                "permissions",
                "stop_conditions"
            ]
        );
    }

    #[test]
    fn synthetic_run_reports_all_five_states_and_no_effect_or_enablement() {
        let mut candidate = profile("AG-25");
        candidate.requested_tools.extend([
            "degraded".to_owned(),
            "untested".to_owned(),
            "denied".to_owned(),
            "missing".to_owned(),
        ]);
        let environment = SyntheticAgentEnvironment {
            fake_files: true,
            fake_tools: true,
            fake_models: true,
            fake_connectors: true,
            fake_grants: true,
            available: ["fake-read", "fake-root", "fake-object", "fake-model"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            denied: ["denied"].into_iter().map(str::to_owned).collect(),
            degraded: ["degraded"].into_iter().map(str::to_owned).collect(),
            untested: ["untested"].into_iter().map(str::to_owned).collect(),
        };
        let report = synthetic_dry_run(&candidate, &environment).unwrap();
        assert_eq!(report.real_effect_count, 0);
        assert!(!report.enabled);
        assert!(
            AgentCapabilityState::all_for_test()
                .iter()
                .all(|state| report.requirements.values().any(|value| value == state))
        );
    }

    #[test]
    fn a_non_synthetic_dependency_blocks_the_run() {
        let environment = SyntheticAgentEnvironment {
            fake_files: false,
            fake_tools: true,
            fake_models: true,
            fake_connectors: true,
            fake_grants: true,
            available: BTreeSet::new(),
            denied: BTreeSet::new(),
            degraded: BTreeSet::new(),
            untested: BTreeSet::new(),
        };
        assert_eq!(
            synthetic_dry_run(&profile("AG-01"), &environment),
            Err(AgentProfileError::NonSyntheticEnvironment)
        );
    }

    impl AgentCapabilityState {
        fn all_for_test() -> [Self; 5] {
            [
                Self::Available,
                Self::Degraded,
                Self::Untested,
                Self::Denied,
                Self::Incompatible,
            ]
        }
    }
}
