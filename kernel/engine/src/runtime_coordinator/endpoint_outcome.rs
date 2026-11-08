// SPDX-License-Identifier: Apache-2.0
//! Endpoint terminal framing using the existing outcome predicates. This is
//! descriptive provenance, never a verifier proof or an orchestration loop.

use agentmage_kernel_contracts::{RuntimeEndpointRunRequest, VersionedContract};
use serde::{Deserialize, Deserializer, Serialize};

use super::*;
use crate::runtime_loop::{RuntimeCompletionClaim, RuntimeCompletionProposal, RuntimeRunScope};

/// Provenance of exact endpoint model bytes, without a native manifest or
/// tokenizer assertion. The claim digest binds the accepted turn, complete
/// request/response descriptors, original run and exact schema-bound payload.
/// The citation digest binds the outcome's current ordered evidence. Neither
/// model output nor citations independently establish task success.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEndpointAnswerEvidence {
    /// Supported contract version.
    pub schema_version: u16,
    /// Digest of the exact kernel-bound endpoint completion claim.
    pub completion_claim_sha256: String,
    /// Digest of the complete ordered outcome evidence, including revisions.
    pub citations_sha256: String,
}

/// A distinct endpoint envelope around shared terminal observations. The
/// nested native `answer_evidence` must be null; endpoint provenance belongs
/// only here. `outcome.outcome_sha256` hashes this WHOLE envelope with that
/// field zeroed, so a terminal event must reference that envelope digest.
/// No native outcome, model admission, completion proof or resume is fabricated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEndpointOutcome {
    /// Shared terminal identity, counters, evidence, receipts and bounded output.
    pub outcome: RuntimeOutcome,
    /// Required only for a successful rendered endpoint answer; explicit null
    /// otherwise. Deserializing this field provides no accepted-turn authority.
    #[serde(deserialize_with = "required_option")]
    pub answer_evidence: Option<RuntimeEndpointAnswerEvidence>,
}

fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

impl VersionedContract for RuntimeEndpointOutcome {
    fn schema_version(&self) -> u16 {
        self.outcome.schema_version
    }
}

/// Seals endpoint terminal framing without converting the selected model to a
/// native profile. Successful rendered output requires the owning accepted
/// endpoint claim; the claim must still pass the coordinator's verifier registry
/// before that coordinator can decide success. This function grants no success
/// authority, effect, publication, persistence or network permission.
pub fn seal_runtime_endpoint_outcome(
    mut outcome: RuntimeOutcome,
    request: &RuntimeEndpointRunRequest,
    claim: Option<&RuntimeCompletionClaim<'_>>,
) -> Result<RuntimeEndpointOutcome, RuntimeCoordinatorError> {
    verify_runtime_endpoint_run_request(request)?;
    outcome.schema_version = CONTRACT_SCHEMA_VERSION;
    outcome.outcome_sha256 = ZERO_SHA256.to_owned();
    validate_base(&outcome, request)?;
    let answer_evidence = endpoint_answer(&outcome, request, claim)?;
    let mut envelope = RuntimeEndpointOutcome {
        outcome,
        answer_evidence,
    };
    envelope.outcome.outcome_sha256 = canonical_sha256(&envelope)?;
    Ok(envelope)
}

/// Verifies the whole envelope, original run and exact accepted claim. Wire
/// metadata alone cannot recreate a claim. Ephemeral composition must retain
/// the accepted turn while verifying a successful rendered answer; this API
/// promises neither replay-based authority nor durable reconstruction.
pub fn verify_runtime_endpoint_outcome(
    envelope: &RuntimeEndpointOutcome,
    request: &RuntimeEndpointRunRequest,
    claim: Option<&RuntimeCompletionClaim<'_>>,
) -> Result<(), RuntimeCoordinatorError> {
    verify_runtime_endpoint_run_request(request)?;
    validate(envelope, request, claim)?;
    let mut preimage = envelope.clone();
    preimage.outcome.outcome_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != envelope.outcome.outcome_sha256 {
        return Err(RuntimeCoordinatorError::DigestMismatch);
    }
    Ok(())
}

fn validate(
    envelope: &RuntimeEndpointOutcome,
    request: &RuntimeEndpointRunRequest,
    claim: Option<&RuntimeCompletionClaim<'_>>,
) -> Result<(), RuntimeCoordinatorError> {
    validate_base(&envelope.outcome, request)?;
    if envelope.answer_evidence != endpoint_answer(&envelope.outcome, request, claim)? {
        return Err(RuntimeCoordinatorError::OutcomeDenied);
    }
    Ok(())
}

fn validate_base(
    outcome: &RuntimeOutcome,
    request: &RuntimeEndpointRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    // These are the only currently composable endpoint turns. A failure
    // envelope must not implicitly enable a durable or controlled-write mode.
    if request.mode != RuntimeSessionMode::EphemeralReadOnly || request.event_cursor.is_some() {
        return Err(RuntimeCoordinatorError::OutcomeDenied);
    }
    validate_common_runtime_outcome_shape(
        outcome,
        &RuntimeRunShape::from(request),
        &request.limits,
    )?;
    if outcome.answer_evidence.is_some() {
        return Err(RuntimeCoordinatorError::OutcomeDenied);
    }
    Ok(())
}

fn endpoint_answer(
    outcome: &RuntimeOutcome,
    request: &RuntimeEndpointRunRequest,
    claim: Option<&RuntimeCompletionClaim<'_>>,
) -> Result<Option<RuntimeEndpointAnswerEvidence>, RuntimeCoordinatorError> {
    if !outcome.state.is_success() || outcome.output.is_none() {
        return if claim.is_none() {
            Ok(None)
        } else {
            Err(RuntimeCoordinatorError::OutcomeDenied)
        };
    }
    let scope =
        RuntimeRunScope::endpoint(request).map_err(|_| RuntimeCoordinatorError::OutcomeDenied)?;
    let claim = claim.ok_or(RuntimeCoordinatorError::OutcomeDenied)?;
    let turn = claim
        .endpoint_turn()
        .ok_or(RuntimeCoordinatorError::OutcomeDenied)?;
    let Some(RuntimeOutput::Inline { payload }) = outcome.output.as_ref() else {
        return Err(RuntimeCoordinatorError::OutcomeDenied);
    };
    if !claim.matches_scope(scope)
        || claim.payload() != Some(payload)
        || outcome.turn_count == 0
        || outcome.model_call_count != outcome.turn_count
        || turn.turn_id().as_str()
            != crate::runtime_loop::derived_id(
                "turn",
                request.run_id.as_str(),
                u64::from(outcome.turn_count),
            )
    {
        return Err(RuntimeCoordinatorError::OutcomeDenied);
    }
    Ok(Some(RuntimeEndpointAnswerEvidence {
        schema_version: CONTRACT_SCHEMA_VERSION,
        completion_claim_sha256: claim.proposal_sha256().to_owned(),
        citations_sha256: canonical_sha256(&outcome.evidence)?,
    }))
}
