//! Fail-closed local rollout for the foundational artifact and workflow runtime.

use serde::{Deserialize, Serialize};

use crate::feature_activation::RuntimeFeatureActivation;

/// Current foundational-runtime rollout configuration version.
pub const FOUNDATIONAL_ROLLOUT_SCHEMA_VERSION: u16 = 1;

/// Explicit locally requested rollout mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationalRolloutRequest {
    /// Preserve the legacy zero-hidden-retry baseline with foundational features off.
    Disabled,
    /// Enable the implemented text/log and verified-workflow beta surfaces.
    Beta,
}

/// Closed local configuration for foundational-runtime rollout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationalRolloutConfig {
    /// Exact configuration schema version.
    pub schema_version: u16,
    /// Monotonic local configuration generation.
    pub generation: u64,
    /// Explicitly requested mode.
    pub requested_mode: FoundationalRolloutRequest,
    /// Exact affirmative acknowledgement for the beta limitations.
    pub beta_limitations_accepted: bool,
    /// One-shot request to return to the legacy baseline before activation.
    pub rollback_requested: bool,
}

impl FoundationalRolloutConfig {
    /// Migrates an absent legacy setting without changing legacy runtime behavior.
    #[must_use]
    pub const fn migrate_legacy_zero_hidden_retry() -> Self {
        Self {
            schema_version: FOUNDATIONAL_ROLLOUT_SCHEMA_VERSION,
            generation: 1,
            requested_mode: FoundationalRolloutRequest::Disabled,
            beta_limitations_accepted: false,
            rollback_requested: false,
        }
    }

    /// Returns a new explicit beta opt-in configuration.
    #[must_use]
    pub const fn beta_opt_in(generation: u64) -> Self {
        Self {
            schema_version: FOUNDATIONAL_ROLLOUT_SCHEMA_VERSION,
            generation,
            requested_mode: FoundationalRolloutRequest::Beta,
            beta_limitations_accepted: true,
            rollback_requested: false,
        }
    }

    /// Produces the next complete disabled generation for an exact rollback.
    pub const fn rollback(self) -> Result<Self, FoundationalRolloutError> {
        if self.schema_version != FOUNDATIONAL_ROLLOUT_SCHEMA_VERSION || self.generation == 0 {
            return Err(FoundationalRolloutError::InvalidConfiguration);
        }
        let Some(generation) = self.generation.checked_add(1) else {
            return Err(FoundationalRolloutError::GenerationExhausted);
        };
        Ok(Self {
            schema_version: FOUNDATIONAL_ROLLOUT_SCHEMA_VERSION,
            generation,
            requested_mode: FoundationalRolloutRequest::Disabled,
            beta_limitations_accepted: false,
            rollback_requested: false,
        })
    }
}

/// Fresh local emergency-disable observation evaluated before ordinary rollout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationalEmergencyDisable {
    /// No local emergency-disable material is present.
    Absent,
    /// Current locally verified material disables the foundational runtime.
    VerifiedActive,
    /// Material is present but cannot be verified; operation must stay blocked.
    InvalidPresent,
}

/// Effective local rollout state after precedence and validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationalRolloutMode {
    /// Legacy behavior is preserved and the new foundational surfaces are off.
    LegacyBaseline,
    /// The explicitly accepted beta is active.
    Beta,
    /// An explicit rollback selected the legacy baseline.
    RolledBack,
    /// Verified local emergency material disabled the foundational runtime.
    EmergencyDisabled,
    /// Invalid configuration or disable material failed closed.
    Blocked,
}

/// Stable, content-free reason for one rollout decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationalRolloutReason {
    /// An absent legacy setting migrated without behavioral change.
    LegacyBehaviorPreserved,
    /// A complete explicit beta opt-in was accepted.
    ExplicitBetaOptIn,
    /// Rollback took precedence before activation.
    ExplicitRollback,
    /// Current verified local emergency material took precedence.
    VerifiedEmergencyDisable,
    /// Present emergency material failed verification and therefore blocked activation.
    InvalidEmergencyMaterial,
    /// The closed rollout configuration was invalid.
    InvalidConfiguration,
}

impl FoundationalRolloutReason {
    /// Stable diagnostic reason code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::LegacyBehaviorPreserved => "foundational.rollout.legacy-preserved",
            Self::ExplicitBetaOptIn => "foundational.rollout.beta-opt-in",
            Self::ExplicitRollback => "foundational.rollout.rollback",
            Self::VerifiedEmergencyDisable => "foundational.rollout.emergency-disabled",
            Self::InvalidEmergencyMaterial => "foundational.rollout.emergency-invalid",
            Self::InvalidConfiguration => "foundational.rollout.configuration-invalid",
        }
    }
}

/// Complete authority-neutral decision produced before registration or work acceptance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundationalRolloutDecision {
    /// Effective rollout state.
    pub mode: FoundationalRolloutMode,
    /// Stable decision reason.
    pub reason: FoundationalRolloutReason,
    /// Exact evaluated local configuration generation.
    pub generation: u64,
    /// Derived feature activation; this value grants no authority.
    pub features: RuntimeFeatureActivation,
    /// Fixed false: no decision permits an automatic or hidden retry.
    pub hidden_retry_permitted: bool,
    /// Fixed true: any eligible retry remains a fresh supervised attempt.
    pub retry_requires_fresh_attempt: bool,
    /// Fixed false: rollout diagnostics have no automatic remote transport.
    pub automatic_remote_telemetry: bool,
    /// Fixed false: this configuration carries no network authority.
    pub network_authority: bool,
}

impl FoundationalRolloutDecision {
    /// Whether foundational tool registration and supervision may be composed.
    #[must_use]
    pub const fn foundational_features_enabled(self) -> bool {
        matches!(self.mode, FoundationalRolloutMode::Beta)
    }
}

/// Release phase attached to an explicit local diagnostic review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationalDiagnosticPhase {
    /// Candidate rehearsal before any release claim.
    Candidate,
    /// Explicit local review after an independently established release.
    PostRelease,
}

/// Content-free local rollout diagnostic; it contains no paths, prompts, or identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationalRolloutDiagnostic {
    /// Exact diagnostic schema version.
    pub schema_version: u16,
    /// Review phase selected locally by the operator.
    pub phase: FoundationalDiagnosticPhase,
    /// Effective rollout mode.
    pub mode: FoundationalRolloutMode,
    /// Stable reason code family.
    pub reason: FoundationalRolloutReason,
    /// Exact evaluated configuration generation.
    pub generation: u64,
    /// Whether the foundational surfaces are active.
    pub foundational_features_enabled: bool,
    /// Fixed false: hidden retries are never part of rollout.
    pub hidden_retry_permitted: bool,
    /// Fixed false: no automatic remote diagnostic transport exists.
    pub automatic_remote_telemetry: bool,
    /// Fixed false: diagnostics carry no network authority.
    pub network_authority: bool,
    /// Fixed true: any export remains separately previewed and locally approved.
    pub reviewed_local_export_required: bool,
}

/// Stable rollout construction failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundationalRolloutError {
    /// The version, generation, or explicit beta acknowledgement is invalid.
    InvalidConfiguration,
    /// No next monotonic generation can be represented.
    GenerationExhausted,
}

impl FoundationalRolloutError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidConfiguration => "foundational.rollout.configuration-invalid",
            Self::GenerationExhausted => "foundational.rollout.generation-exhausted",
        }
    }
}

impl std::fmt::Display for FoundationalRolloutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FoundationalRolloutError {}

/// Evaluates local rollout state before feature registration or work acceptance.
#[must_use]
pub const fn evaluate_foundational_rollout(
    config: FoundationalRolloutConfig,
    emergency: FoundationalEmergencyDisable,
) -> FoundationalRolloutDecision {
    let disabled = RuntimeFeatureActivation::foundational_disabled();
    let base = FoundationalRolloutDecision {
        mode: FoundationalRolloutMode::Blocked,
        reason: FoundationalRolloutReason::InvalidConfiguration,
        generation: config.generation,
        features: disabled,
        hidden_retry_permitted: false,
        retry_requires_fresh_attempt: true,
        automatic_remote_telemetry: false,
        network_authority: false,
    };
    if config.schema_version != FOUNDATIONAL_ROLLOUT_SCHEMA_VERSION || config.generation == 0 {
        return base;
    }
    if matches!(config.requested_mode, FoundationalRolloutRequest::Disabled)
        && config.beta_limitations_accepted
        && !config.rollback_requested
    {
        return base;
    }
    match emergency {
        FoundationalEmergencyDisable::InvalidPresent => FoundationalRolloutDecision {
            reason: FoundationalRolloutReason::InvalidEmergencyMaterial,
            ..base
        },
        FoundationalEmergencyDisable::VerifiedActive => FoundationalRolloutDecision {
            mode: FoundationalRolloutMode::EmergencyDisabled,
            reason: FoundationalRolloutReason::VerifiedEmergencyDisable,
            ..base
        },
        FoundationalEmergencyDisable::Absent if config.rollback_requested => {
            FoundationalRolloutDecision {
                mode: FoundationalRolloutMode::RolledBack,
                reason: FoundationalRolloutReason::ExplicitRollback,
                ..base
            }
        }
        FoundationalEmergencyDisable::Absent => match config.requested_mode {
            FoundationalRolloutRequest::Disabled => FoundationalRolloutDecision {
                mode: FoundationalRolloutMode::LegacyBaseline,
                reason: FoundationalRolloutReason::LegacyBehaviorPreserved,
                ..base
            },
            FoundationalRolloutRequest::Beta if config.beta_limitations_accepted => {
                FoundationalRolloutDecision {
                    mode: FoundationalRolloutMode::Beta,
                    reason: FoundationalRolloutReason::ExplicitBetaOptIn,
                    features: RuntimeFeatureActivation::current(),
                    ..base
                }
            }
            FoundationalRolloutRequest::Beta => base,
        },
    }
}

/// Builds one local content-free review record without export or transport side effects.
#[must_use]
pub const fn review_foundational_rollout_locally(
    decision: FoundationalRolloutDecision,
    phase: FoundationalDiagnosticPhase,
) -> FoundationalRolloutDiagnostic {
    FoundationalRolloutDiagnostic {
        schema_version: 1,
        phase,
        mode: decision.mode,
        reason: decision.reason,
        generation: decision.generation,
        foundational_features_enabled: decision.foundational_features_enabled(),
        hidden_retry_permitted: decision.hidden_retry_permitted,
        automatic_remote_telemetry: decision.automatic_remote_telemetry,
        network_authority: decision.network_authority,
        reviewed_local_export_required: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_migration_preserves_zero_hidden_retry_and_requires_beta_opt_in() {
        let migrated = FoundationalRolloutConfig::migrate_legacy_zero_hidden_retry();
        let legacy = evaluate_foundational_rollout(migrated, FoundationalEmergencyDisable::Absent);
        assert_eq!(legacy.mode, FoundationalRolloutMode::LegacyBaseline);
        assert!(!legacy.foundational_features_enabled());
        assert!(!legacy.hidden_retry_permitted && legacy.retry_requires_fresh_attempt);
        let unknown = r#"{"schema_version":1,"generation":1,"requested_mode":"disabled","beta_limitations_accepted":false,"rollback_requested":false,"prompt":"secret"}"#;
        assert!(serde_json::from_str::<FoundationalRolloutConfig>(unknown).is_err());

        let mut contradictory = migrated;
        contradictory.beta_limitations_accepted = true;
        assert_eq!(
            evaluate_foundational_rollout(contradictory, FoundationalEmergencyDisable::Absent).mode,
            FoundationalRolloutMode::Blocked
        );

        let mut incomplete = FoundationalRolloutConfig::beta_opt_in(2);
        incomplete.beta_limitations_accepted = false;
        assert_eq!(
            evaluate_foundational_rollout(incomplete, FoundationalEmergencyDisable::Absent).mode,
            FoundationalRolloutMode::Blocked
        );
        let beta = evaluate_foundational_rollout(
            FoundationalRolloutConfig::beta_opt_in(2),
            FoundationalEmergencyDisable::Absent,
        );
        assert_eq!(beta.mode, FoundationalRolloutMode::Beta);
        assert!(beta.foundational_features_enabled());
        assert!(!beta.hidden_retry_permitted && beta.retry_requires_fresh_attempt);
    }

    #[test]
    fn rollback_and_emergency_disable_take_precedence_without_registration() {
        let mut rollback = FoundationalRolloutConfig::beta_opt_in(3);
        rollback.rollback_requested = true;
        let rolled_back =
            evaluate_foundational_rollout(rollback, FoundationalEmergencyDisable::Absent);
        assert_eq!(rolled_back.mode, FoundationalRolloutMode::RolledBack);
        assert!(!rolled_back.foundational_features_enabled());

        for emergency in [
            FoundationalEmergencyDisable::VerifiedActive,
            FoundationalEmergencyDisable::InvalidPresent,
        ] {
            let decision =
                evaluate_foundational_rollout(FoundationalRolloutConfig::beta_opt_in(3), emergency);
            assert!(!decision.foundational_features_enabled());
            assert!(!decision.features.artifact_ingress);
            assert!(!decision.features.retrieval);
            assert!(!decision.features.workflow_supervision);
        }
    }

    #[test]
    fn rollback_is_complete_monotonic_and_reenrollment_is_explicit() {
        let beta = FoundationalRolloutConfig::beta_opt_in(8);
        let disabled = beta.rollback().expect("rollback generation");
        assert_eq!(disabled.generation, 9);
        assert_eq!(
            disabled.requested_mode,
            FoundationalRolloutRequest::Disabled
        );
        assert!(!disabled.beta_limitations_accepted && !disabled.rollback_requested);
        assert_eq!(
            FoundationalRolloutConfig::beta_opt_in(u64::MAX).rollback(),
            Err(FoundationalRolloutError::GenerationExhausted)
        );
    }

    #[test]
    fn post_release_review_is_content_free_local_and_has_no_transport_authority() {
        let decision = evaluate_foundational_rollout(
            FoundationalRolloutConfig::beta_opt_in(12),
            FoundationalEmergencyDisable::Absent,
        );
        let diagnostic =
            review_foundational_rollout_locally(decision, FoundationalDiagnosticPhase::PostRelease);
        let encoded = serde_json::to_value(diagnostic).expect("closed diagnostic serializes");
        let fields = encoded
            .as_object()
            .expect("diagnostic object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            fields,
            [
                "schema_version",
                "phase",
                "mode",
                "reason",
                "generation",
                "foundational_features_enabled",
                "hidden_retry_permitted",
                "automatic_remote_telemetry",
                "network_authority",
                "reviewed_local_export_required",
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(diagnostic.mode, FoundationalRolloutMode::Beta);
        assert!(!diagnostic.hidden_retry_permitted);
        assert!(!diagnostic.automatic_remote_telemetry);
        assert!(!diagnostic.network_authority);
        assert!(diagnostic.reviewed_local_export_required);
    }
}
