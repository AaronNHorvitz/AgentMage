// SPDX-License-Identifier: MIT OR Apache-2.0
//! Original-owner evidence at the existing independent completion boundary.

use super::*;
use agentmage_kernel_engine::runtime_loop::{
    RuntimeCompletionVerificationInput, RuntimeVerificationInput, RuntimeVerifierPort,
};
use agentmage_platform_linux::LinuxCodingValidationEvidence;

/// Explicit trusted opt-in to check coding completion using original records.
/// The implementation must independently check the sealed task's acceptance
/// criteria and correlate the original attempts with coordinator observations.
/// A parsed full pass or matching source sample alone cannot accept a task.
/// This port adds no default accepting implementation or model-authored checks.
pub trait CodingEndpointAcceptancePort {
    /// Exact verifier identity registered with the existing coordinator.
    fn verifier_id(&self) -> &VerifierId;

    /// Check the original claim and complete original validation history.
    /// Called with the original owner locked: do not reenter its preview or
    /// boundary. Prior consent must cover these records and whole-tree sampling.
    /// No original file content is disclosed to a model by this interface.
    fn verify_coding_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
        validation: LinuxCodingValidationEvidence<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure>;
}

/// Process-local adapter over the SAME native owner as strict edits and tests.
/// This does not restore authority, erase failed tools or grade a task itself.
/// The original registry still validates the independent verifier's candidate.
pub struct CodingEndpointOwnedVerifier<C, V> {
    startup: Arc<CodingEndpointPreparedStartup>,
    state: Arc<Mutex<State<C>>>,
    verifier: V,
}

impl<C: RuntimeClock + Clone> CodingEndpointEditPreviews<C> {
    /// Connect a caller-supplied independent checker before moving previews
    /// into the turn port. Only the combined fixed-test owner supports this;
    /// construction issues no evidence, reads no files and changes no phase.
    /// Existing development startup keeps its non-certifying verifier.
    pub fn completion_verifier<V: CodingEndpointAcceptancePort>(
        &self,
        verifier: V,
    ) -> Result<CodingEndpointOwnedVerifier<C, V>, RuntimePortFailure> {
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        if state.validation.is_none() {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(CodingEndpointOwnedVerifier {
            startup: self.startup.clone(),
            state: self.state.clone(),
            verifier,
        })
    }
}

impl<C, V: CodingEndpointAcceptancePort> RuntimeVerifierPort for CodingEndpointOwnedVerifier<C, V> {
    fn verifier_id(&self) -> &VerifierId {
        self.verifier.verifier_id()
    }

    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        // This adapter cannot reinterpret a native-profile request or silently
        // fall back to another verifier family.
        Err(RuntimePortFailure::Invalid)
    }

    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        if input.request.endpoint_request() != Some(self.startup.request())
            || !input.proposal.matches_scope(input.request)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let state = self
            .state
            .lock()
            .map_err(|_| RuntimePortFailure::Unavailable)?;
        state.control.remaining_ms().map_err(control_failure)?;
        if state.next != Next::Completion {
            return Err(RuntimePortFailure::Invalid);
        }
        let Some(Owner::Ready(repository)) = state.owner.as_ref() else {
            return Err(RuntimePortFailure::Invalid);
        };
        let checked = repository.with_completion_validation_evidence(|validation| {
            state.control.remaining_ms().map_err(control_failure)?;
            let checked = self.verifier.verify_coding_completion(input, validation);
            state.control.remaining_ms().map_err(control_failure)?;
            checked
        });
        state.control.remaining_ms().map_err(control_failure)?;
        checked.map_err(|_| RuntimePortFailure::Invalid)?
    }
}
