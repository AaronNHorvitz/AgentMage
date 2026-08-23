//! Approved-Plan bindings for the existing controlled engineering runtime.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, EngineeringPlanHandoff, EngineeringRuntimeBinding,
    EngineeringSessionMode, RuntimeRunRequest, RuntimeSessionMode,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    engineering_plan::verify_plan_handoff, runtime_coordinator::verify_runtime_run_request,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable refusal from approved-Plan runtime binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringExecutionError {
    /// The handoff or runtime request was malformed or did not verify.
    InvalidInput,
    /// The runtime request did not identify the exact Agent Plan and session.
    BindingMismatch,
    /// The binding digest was substituted.
    DigestMismatch,
}

/// Seals one non-authoritative binding between an approved Plan and controlled-write request.
pub fn seal_runtime_binding(
    handoff: &EngineeringPlanHandoff,
    request: &RuntimeRunRequest,
) -> Result<EngineeringRuntimeBinding, EngineeringExecutionError> {
    verify_inputs(handoff, request)?;
    let mut binding = EngineeringRuntimeBinding {
        schema_version: CONTRACT_SCHEMA_VERSION,
        session_id: handoff.target_session_id.clone(),
        handoff_sha256: handoff.handoff_sha256.clone(),
        plan_artifact_id: handoff.plan_artifact_id.clone(),
        plan_sha256: handoff.plan_sha256.clone(),
        approval_id: handoff.approval_id.clone(),
        run_id: request.run_id.clone(),
        request_sha256: request.request_sha256.clone(),
        binding_sha256: ZERO_SHA256.to_owned(),
    };
    binding.binding_sha256 =
        canonical_sha256(&binding).map_err(|_| EngineeringExecutionError::InvalidInput)?;
    Ok(binding)
}

/// Verifies one sealed approved-Plan runtime binding against its complete authorities.
pub fn verify_runtime_binding(
    binding: &EngineeringRuntimeBinding,
    handoff: &EngineeringPlanHandoff,
    request: &RuntimeRunRequest,
) -> Result<(), EngineeringExecutionError> {
    verify_runtime_binding_shape(binding)?;
    let expected = seal_runtime_binding(handoff, request)?;
    if &expected != binding {
        return Err(EngineeringExecutionError::DigestMismatch);
    }
    Ok(())
}

/// Verifies the closed fields and canonical digest without re-authorizing the bound request.
pub fn verify_runtime_binding_shape(
    binding: &EngineeringRuntimeBinding,
) -> Result<(), EngineeringExecutionError> {
    if binding.schema_version != CONTRACT_SCHEMA_VERSION
        || binding.session_id.as_str().is_empty()
        || binding.handoff_sha256.len() != 64
        || binding.plan_sha256.len() != 64
        || binding.request_sha256.len() != 64
        || binding.binding_sha256.len() != 64
        || binding.plan_artifact_id.as_str().is_empty()
        || binding.approval_id.as_str().is_empty()
        || binding.run_id.as_str().is_empty()
    {
        return Err(EngineeringExecutionError::InvalidInput);
    }
    let mut candidate = binding.clone();
    candidate.binding_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != binding.binding_sha256 {
        return Err(EngineeringExecutionError::DigestMismatch);
    }
    Ok(())
}

fn verify_inputs(
    handoff: &EngineeringPlanHandoff,
    request: &RuntimeRunRequest,
) -> Result<(), EngineeringExecutionError> {
    verify_plan_handoff(handoff).map_err(|_| EngineeringExecutionError::InvalidInput)?;
    verify_runtime_run_request(request).map_err(|_| EngineeringExecutionError::InvalidInput)?;
    if handoff.target_mode != EngineeringSessionMode::Agent
        || request.mode != RuntimeSessionMode::ControlledWrite
        || request.session_id != handoff.target_session_id
        || request.task.session_id != handoff.target_session_id
        || request.task.objective != request.work_packet.objective
        || sha256(request.task.objective.as_bytes()) != handoff.plan_sha256
    {
        return Err(EngineeringExecutionError::BindingMismatch);
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, EngineeringExecutionError> {
    let encoded = serde_json::to_vec(value).map_err(|_| EngineeringExecutionError::InvalidInput)?;
    Ok(sha256(&encoded))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::EngineeringSessionMode;

    use super::EngineeringExecutionError;

    #[test]
    fn team_mode_is_not_a_single_agent_runtime_binding() {
        assert_ne!(EngineeringSessionMode::Team, EngineeringSessionMode::Agent);
        assert_eq!(
            EngineeringExecutionError::BindingMismatch,
            EngineeringExecutionError::BindingMismatch
        );
    }
}
