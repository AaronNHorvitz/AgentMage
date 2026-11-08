// SPDX-License-Identifier: Apache-2.0
//! Model-independent, inert completion inputs for the existing verifier boundary.

use agentmage_kernel_contracts::{
    ClosedModelProposal, ContractPayload, EndpointModelFinish, ModelProposalKind, ProposalId,
    SchemaReference,
};

use super::{
    RuntimePortFailure, RuntimeRunScope, endpoint::RuntimeEndpointTurnResult, sha256,
    valid_output_payload,
};

mod sealed {
    pub trait Sealed {}
}

/// Descriptive completion fields only. Inspecting them never admits a proposal,
/// proves an effect, grants authority or establishes deterministic completion.
pub trait RuntimeCompletionProposal: sealed::Sealed {
    /// Stable identity of the complete inert proposal.
    fn proposal_id(&self) -> &ProposalId;
    /// Digest binding the original complete proposal, not just its payload.
    fn proposal_sha256(&self) -> &str;
    /// The model's inert proposal class.
    fn kind(&self) -> ModelProposalKind;
    /// Exact model-authored payload, if any.
    fn payload(&self) -> Option<&ContractPayload>;
    /// Whether the proposal also contains an inert tool candidate.
    fn has_tool_call(&self) -> bool;
}

impl sealed::Sealed for ClosedModelProposal {}
impl RuntimeCompletionProposal for ClosedModelProposal {
    fn proposal_id(&self) -> &ProposalId {
        &self.proposal_id
    }
    fn proposal_sha256(&self) -> &str {
        &self.proposal_sha256
    }
    fn kind(&self) -> ModelProposalKind {
        self.kind
    }
    fn payload(&self) -> Option<&ContractPayload> {
        self.payload.as_ref()
    }
    fn has_tool_call(&self) -> bool {
        self.tool_call.is_some()
    }
}
impl<P: RuntimeCompletionProposal + ?Sized> sealed::Sealed for &P {}
impl<P: RuntimeCompletionProposal + ?Sized> RuntimeCompletionProposal for &P {
    fn proposal_id(&self) -> &ProposalId {
        (**self).proposal_id()
    }
    fn proposal_sha256(&self) -> &str {
        (**self).proposal_sha256()
    }
    fn kind(&self) -> ModelProposalKind {
        (**self).kind()
    }
    fn payload(&self) -> Option<&ContractPayload> {
        (**self).payload()
    }
    fn has_tool_call(&self) -> bool {
        (**self).has_tool_call()
    }
}

enum Claim<'a> {
    Native(&'a ClosedModelProposal),
    Endpoint {
        turn: &'a RuntimeEndpointTurnResult,
        proposal_id: ProposalId,
        proposal_sha256: String,
        payload: ContractPayload,
    },
}

/// An original native proposal or an endpoint completion bound to an accepted
/// explicit turn and its exact decoded stream bytes. No deserializer or native
/// profile/codec conversion exists. This claim is still entirely untrusted.
pub struct RuntimeCompletionClaim<'a>(Claim<'a>);

impl<'a> RuntimeCompletionClaim<'a> {
    /// Borrows the original proposal. Native model admission remains with the
    /// coordinator; wrapping an arbitrary proposal provides no admission proof.
    #[must_use]
    pub const fn native(proposal: &'a ClosedModelProposal) -> Self {
        Self(Claim::Native(proposal))
    }

    /// Binds an ordinarily stopped accepted response as one inert completion
    /// payload. A token-limited response remains an observation, even if its
    /// text happens to be valid JSON; it cannot assert completed generation.
    /// The trusted caller chooses the schema; model text cannot select it.
    /// No text is decoded as instructions, tools, evidence or a native profile.
    pub fn endpoint(
        scope: RuntimeRunScope<'_>,
        turn: &'a RuntimeEndpointTurnResult,
        response_text: &str,
        schema: SchemaReference,
    ) -> Result<Self, RuntimePortFailure> {
        let run = scope
            .endpoint_request()
            .ok_or(RuntimePortFailure::Invalid)?;
        let result = turn.result();
        if result.finish != EndpointModelFinish::Stop
            || !turn.run().binds(run)
            || result.request.runtime_model_binding() != scope.model_binding()
            || result.response_text_bytes != response_text.len()
            || result.response_text_sha256 != sha256(response_text.as_bytes())
        {
            return Err(RuntimePortFailure::Invalid);
        }
        // Bound size and validate the schema before copying untrusted text.
        if response_text.is_empty()
            || response_text.len() as u64 > scope.limits().max_output_bytes
            || !super::valid_identifier(schema.schema_id.as_str())
            || schema.schema_version == 0
            || !super::valid_sha256(&schema.schema_sha256)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let payload = ContractPayload {
            schema,
            media_type: "application/json".into(),
            bytes: response_text.as_bytes().to_vec(),
            sha256: result.response_text_sha256.clone(),
        };
        if !valid_output_payload(&payload, scope.limits().max_output_bytes) {
            return Err(RuntimePortFailure::Invalid);
        }
        #[derive(serde::Serialize)]
        struct Binding<'a> {
            version: u16,
            kind: &'static str,
            turn: &'a RuntimeEndpointTurnResult,
            payload: &'a ContractPayload,
        }
        let bytes = serde_json::to_vec(&Binding {
            version: 1,
            kind: "endpoint_completion_candidate",
            turn,
            payload: &payload,
        })
        .map_err(|_| RuntimePortFailure::Invalid)?;
        let proposal_sha256 = sha256(&bytes);
        let proposal_id = ProposalId::from_raw(format!("endpoint-completion:{proposal_sha256}"));
        Ok(Self(Claim::Endpoint {
            turn,
            proposal_id,
            proposal_sha256,
            payload,
        }))
    }

    /// Original native proposal only, absent for every endpoint claim.
    #[must_use]
    pub const fn native_proposal(&self) -> Option<&'a ClosedModelProposal> {
        match self.0 {
            Claim::Native(proposal) => Some(proposal),
            Claim::Endpoint { .. } => None,
        }
    }

    pub(crate) const fn endpoint_turn(&self) -> Option<&'a RuntimeEndpointTurnResult> {
        match self.0 {
            Claim::Endpoint { turn, .. } => Some(turn),
            Claim::Native(_) => None,
        }
    }

    /// Checks the request family and exact model/run binding before verifier
    /// entry. This is identity binding, never proposal or effect admission.
    #[must_use]
    pub fn matches_scope(&self, scope: RuntimeRunScope<'_>) -> bool {
        match &self.0 {
            Claim::Native(proposal) => scope.native_request().is_ok_and(|request| {
                proposal.profile_id == request.model_profile.profile_id
                    && proposal.codec_id == request.model_profile.codec.codec_id
            }),
            Claim::Endpoint { turn, .. } => scope.endpoint_request().is_some_and(|run| {
                turn.run().binds(run)
                    && turn.result().request.runtime_model_binding() == scope.model_binding()
            }),
        }
    }
}

impl sealed::Sealed for RuntimeCompletionClaim<'_> {}
impl RuntimeCompletionProposal for RuntimeCompletionClaim<'_> {
    fn proposal_id(&self) -> &ProposalId {
        match &self.0 {
            Claim::Native(p) => &p.proposal_id,
            Claim::Endpoint { proposal_id, .. } => proposal_id,
        }
    }
    fn proposal_sha256(&self) -> &str {
        match &self.0 {
            Claim::Native(p) => &p.proposal_sha256,
            Claim::Endpoint {
                proposal_sha256, ..
            } => proposal_sha256,
        }
    }
    fn kind(&self) -> ModelProposalKind {
        match &self.0 {
            Claim::Native(p) => p.kind,
            Claim::Endpoint { .. } => ModelProposalKind::CompletionCandidate,
        }
    }
    fn payload(&self) -> Option<&ContractPayload> {
        match &self.0 {
            Claim::Native(p) => p.payload.as_ref(),
            Claim::Endpoint { payload, .. } => Some(payload),
        }
    }
    fn has_tool_call(&self) -> bool {
        match &self.0 {
            Claim::Native(p) => p.tool_call.is_some(),
            Claim::Endpoint { .. } => false,
        }
    }
}
