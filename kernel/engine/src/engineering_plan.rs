//! Exact, non-authoritative Verified Chat Plan approval bindings.

use agentmage_kernel_contracts::{
    ActorId, ApprovalId, CONTRACT_SCHEMA_VERSION, EngineeringPlanApproval, EngineeringPlanHandoff,
    EngineeringSessionMode, RuntimeArtifactId, SessionId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable Plan approval validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringPlanError {
    /// Identity, digest, time, or seal is invalid.
    InvalidApproval,
}

impl EngineeringPlanError {
    /// Returns one content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidApproval => "engineering.plan.approval.invalid",
        }
    }
}

/// Seals one explicit approval of exact Plan bytes.
pub fn seal_plan_approval(
    approval_id: ApprovalId,
    session_id: SessionId,
    plan_artifact_id: RuntimeArtifactId,
    plan_sha256: String,
    approved_by: ActorId,
    approved_at_epoch_ms: u64,
) -> Result<EngineeringPlanApproval, EngineeringPlanError> {
    let mut approval = EngineeringPlanApproval {
        schema_version: CONTRACT_SCHEMA_VERSION,
        approval_id,
        session_id,
        plan_artifact_id,
        plan_sha256,
        approved_by,
        approved_at_epoch_ms,
        approval_sha256: ZERO_SHA256.to_owned(),
    };
    validate_fields(&approval)?;
    approval.approval_sha256 = canonical_sha256(&approval)?;
    Ok(approval)
}

/// Revalidates one retained Plan approval without trusting its claimed digest.
pub fn verify_plan_approval(
    approval: &EngineeringPlanApproval,
) -> Result<(), EngineeringPlanError> {
    validate_fields(approval)?;
    let mut candidate = approval.clone();
    candidate.approval_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != approval.approval_sha256 {
        return Err(EngineeringPlanError::InvalidApproval);
    }
    Ok(())
}

/// Seals one approved-Plan handoff into a distinct Agent or Team session.
#[allow(clippy::too_many_arguments)]
pub fn seal_plan_handoff(
    source_session_id: SessionId,
    target_session_id: SessionId,
    target_mode: EngineeringSessionMode,
    plan_artifact_id: RuntimeArtifactId,
    plan_sha256: String,
    approval_id: ApprovalId,
    approval_sha256: String,
    initiated_by: ActorId,
    initiated_at_epoch_ms: u64,
) -> Result<EngineeringPlanHandoff, EngineeringPlanError> {
    let mut handoff = EngineeringPlanHandoff {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_session_id,
        target_session_id,
        target_mode,
        plan_artifact_id,
        plan_sha256,
        approval_id,
        approval_sha256,
        initiated_by,
        initiated_at_epoch_ms,
        handoff_sha256: ZERO_SHA256.to_owned(),
    };
    validate_handoff_fields(&handoff)?;
    handoff.handoff_sha256 = canonical_sha256(&handoff)?;
    Ok(handoff)
}

/// Revalidates one retained approved-Plan handoff and exact digest.
pub fn verify_plan_handoff(handoff: &EngineeringPlanHandoff) -> Result<(), EngineeringPlanError> {
    validate_handoff_fields(handoff)?;
    let mut candidate = handoff.clone();
    candidate.handoff_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != handoff.handoff_sha256 {
        return Err(EngineeringPlanError::InvalidApproval);
    }
    Ok(())
}

fn validate_fields(approval: &EngineeringPlanApproval) -> Result<(), EngineeringPlanError> {
    if approval.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(approval.approval_id.as_str())
        || !valid_identifier(approval.session_id.as_str())
        || !valid_identifier(approval.plan_artifact_id.as_str())
        || !valid_identifier(approval.approved_by.as_str())
        || !valid_sha256(&approval.plan_sha256)
        || !valid_sha256(&approval.approval_sha256)
        || approval.approved_at_epoch_ms == 0
    {
        return Err(EngineeringPlanError::InvalidApproval);
    }
    Ok(())
}

fn validate_handoff_fields(handoff: &EngineeringPlanHandoff) -> Result<(), EngineeringPlanError> {
    if handoff.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(handoff.source_session_id.as_str())
        || !valid_identifier(handoff.target_session_id.as_str())
        || handoff.source_session_id == handoff.target_session_id
        || !matches!(
            handoff.target_mode,
            EngineeringSessionMode::Agent | EngineeringSessionMode::Team
        )
        || !valid_identifier(handoff.plan_artifact_id.as_str())
        || !valid_sha256(&handoff.plan_sha256)
        || !valid_identifier(handoff.approval_id.as_str())
        || !valid_sha256(&handoff.approval_sha256)
        || !valid_identifier(handoff.initiated_by.as_str())
        || handoff.initiated_at_epoch_ms == 0
        || !valid_sha256(&handoff.handoff_sha256)
    {
        return Err(EngineeringPlanError::InvalidApproval);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, EngineeringPlanError> {
    let bytes = serde_json::to_vec(value).map_err(|_| EngineeringPlanError::InvalidApproval)?;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::{seal_plan_approval, seal_plan_handoff, verify_plan_approval, verify_plan_handoff};
    use agentmage_kernel_contracts::{ActorId, ApprovalId, RuntimeArtifactId, SessionId};

    #[test]
    fn approval_binds_every_exact_identity_and_byte_digest() {
        let approval = seal_plan_approval(
            ApprovalId::from_raw("approval-plan-0001"),
            SessionId::from_raw("session-plan-0001"),
            RuntimeArtifactId::from_raw("artifact-plan-0001"),
            "a".repeat(64),
            ActorId::from_raw("user-aaron"),
            100,
        )
        .unwrap();
        verify_plan_approval(&approval).unwrap();
        for mutate in [
            |value: &mut agentmage_kernel_contracts::EngineeringPlanApproval| {
                value.plan_sha256 = "b".repeat(64);
            },
            |value: &mut agentmage_kernel_contracts::EngineeringPlanApproval| {
                value.plan_artifact_id = RuntimeArtifactId::from_raw("artifact-substituted");
            },
        ] {
            let mut candidate = approval.clone();
            mutate(&mut candidate);
            assert!(verify_plan_approval(&candidate).is_err());
        }
    }

    #[test]
    fn handoff_accepts_only_distinct_agent_or_team_targets() {
        let handoff = seal_plan_handoff(
            SessionId::from_raw("session-plan-0001"),
            SessionId::from_raw("session-agent-0001"),
            agentmage_kernel_contracts::EngineeringSessionMode::Agent,
            RuntimeArtifactId::from_raw("artifact-plan-0001"),
            "a".repeat(64),
            ApprovalId::from_raw("approval-plan-0001"),
            "b".repeat(64),
            ActorId::from_raw("user-aaron"),
            200,
        )
        .unwrap();
        verify_plan_handoff(&handoff).unwrap();
        let mut substituted = handoff.clone();
        substituted.target_mode = agentmage_kernel_contracts::EngineeringSessionMode::Plan;
        assert!(verify_plan_handoff(&substituted).is_err());
        assert!(
            seal_plan_handoff(
                SessionId::from_raw("session-same"),
                SessionId::from_raw("session-same"),
                agentmage_kernel_contracts::EngineeringSessionMode::Team,
                RuntimeArtifactId::from_raw("artifact-plan-0001"),
                "a".repeat(64),
                ApprovalId::from_raw("approval-plan-0001"),
                "b".repeat(64),
                ActorId::from_raw("user-aaron"),
                200,
            )
            .is_err()
        );
    }
}
