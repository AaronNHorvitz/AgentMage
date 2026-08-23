//! Kernel-enforced ceilings for Verified Chat operating modes.

use agentmage_kernel_contracts::EngineeringSessionMode;

/// Closed operation families understood by the Engineering Runtime mode boundary.
///
/// A permitted family is only a ceiling. It never creates a grant, approval, path lease,
/// credential, tool permit, model qualification, or effect authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringModeOperation {
    /// Capture or retrieve exact session-owned context.
    Context,
    /// Parse exact captured bytes into provenance-bound derivatives.
    ArtifactIngestion,
    /// Invoke an independently qualified model without giving it effect authority.
    ModelInference,
    /// Persist or revise a non-authoritative planning artifact.
    PlanArtifact,
    /// Request a separately authorized local effect.
    LocalEffect,
    /// Request a separately authorized remote effect.
    RemoteEffect,
    /// Coordinate separately leased Team workers and serialized integration.
    TeamCampaign,
    /// Expand capability, policy, or authority.
    CapabilityExpansion,
}

/// Stable mode-ceiling denial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringModeError {
    /// The immutable session mode does not permit this operation family.
    Denied,
}

impl EngineeringModeError {
    /// Returns the content-free denial code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Denied => "engineering.mode.denied",
        }
    }
}

/// Enforces the immutable session mode as a narrowing-only operation ceiling.
pub const fn enforce_engineering_mode(
    mode: EngineeringSessionMode,
    operation: EngineeringModeOperation,
) -> Result<(), EngineeringModeError> {
    let permitted = match mode {
        EngineeringSessionMode::Ask => matches!(
            operation,
            EngineeringModeOperation::Context
                | EngineeringModeOperation::ArtifactIngestion
                | EngineeringModeOperation::ModelInference
        ),
        EngineeringSessionMode::Plan => matches!(
            operation,
            EngineeringModeOperation::Context
                | EngineeringModeOperation::ArtifactIngestion
                | EngineeringModeOperation::ModelInference
                | EngineeringModeOperation::PlanArtifact
        ),
        EngineeringSessionMode::Agent => matches!(
            operation,
            EngineeringModeOperation::Context
                | EngineeringModeOperation::ArtifactIngestion
                | EngineeringModeOperation::ModelInference
                | EngineeringModeOperation::PlanArtifact
                | EngineeringModeOperation::LocalEffect
                | EngineeringModeOperation::RemoteEffect
        ),
        EngineeringSessionMode::Team => {
            !matches!(operation, EngineeringModeOperation::CapabilityExpansion)
        }
    };
    if permitted {
        Ok(())
    } else {
        Err(EngineeringModeError::Denied)
    }
}

#[cfg(test)]
mod tests {
    use super::{EngineeringModeOperation as Operation, enforce_engineering_mode};
    use agentmage_kernel_contracts::EngineeringSessionMode as Mode;

    #[test]
    fn ask_and_plan_are_effect_free_kernel_policies() {
        for mode in [Mode::Ask, Mode::Plan] {
            assert!(enforce_engineering_mode(mode, Operation::Context).is_ok());
            assert!(enforce_engineering_mode(mode, Operation::ArtifactIngestion).is_ok());
            assert!(enforce_engineering_mode(mode, Operation::ModelInference).is_ok());
            assert!(enforce_engineering_mode(mode, Operation::LocalEffect).is_err());
            assert!(enforce_engineering_mode(mode, Operation::RemoteEffect).is_err());
            assert!(enforce_engineering_mode(mode, Operation::TeamCampaign).is_err());
            assert!(enforce_engineering_mode(mode, Operation::CapabilityExpansion).is_err());
        }
        assert!(enforce_engineering_mode(Mode::Ask, Operation::PlanArtifact).is_err());
        assert!(enforce_engineering_mode(Mode::Plan, Operation::PlanArtifact).is_ok());
    }

    #[test]
    fn agent_and_team_labels_still_cannot_expand_authority() {
        assert!(enforce_engineering_mode(Mode::Agent, Operation::LocalEffect).is_ok());
        assert!(enforce_engineering_mode(Mode::Agent, Operation::RemoteEffect).is_ok());
        assert!(enforce_engineering_mode(Mode::Agent, Operation::TeamCampaign).is_err());
        assert!(enforce_engineering_mode(Mode::Agent, Operation::CapabilityExpansion).is_err());

        assert!(enforce_engineering_mode(Mode::Team, Operation::TeamCampaign).is_ok());
        assert!(enforce_engineering_mode(Mode::Team, Operation::CapabilityExpansion).is_err());
    }
}
