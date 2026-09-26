//! Single-use research freshness proof inside the existing authority coordinator.
//! This additional restriction grants no network permission or native admission.

use agentmage_kernel_contracts::GrantOperation;

use crate::authority_transaction::{EffectAuthorization, EffectDriver, EffectLaunch};
use crate::research_fetch::PublicGetWorkerPacket;
use crate::research_journal::ResearchJournalError;

/// Native research drivers are reached through the existing authority coordinator,
/// after the canonical owner performs freshness checks inside its store lock.
pub trait ResearchEffectDriver {
    /// Executes one attempt with two distinct proofs. The driver must additionally
    /// verify exact tool/binding/root, trusted launch time and native confinement.
    fn execute_research(
        &mut self,
        authorization: EffectAuthorization<'_>,
        dispatch: ResearchDispatch<'_>,
    ) -> EffectLaunch;
}

/// Owner-only preflight material, filled after canonical full-plan/head/clock checks.
/// No public constructor, serialization or clone may be added.
pub(crate) struct FreshResearchDispatch {
    pub(crate) task_id: String,
    pub(crate) operation_id: String,
    pub(crate) request_sha256: String,
    pub(crate) reservation_sha256: String,
    pub(crate) checked_at_epoch_ms: u64,
}

/// Borrowed only for one synchronous coordinator-to-driver invocation. Keeping
/// strings from this proof does not let an adapter construct another proof.
///
/// There is no public constructor or transferable owner material:
/// ```compile_fail
/// use agentmage_kernel_engine::research_dispatch::ResearchDispatch;
/// let _ = ResearchDispatch { material: &() };
/// ```
/// The invocation lifetime cannot be extended to retain authority for later:
/// ```compile_fail
/// use agentmage_kernel_engine::research_dispatch::ResearchDispatch;
/// fn retain(proof: ResearchDispatch<'_>) -> ResearchDispatch<'static> { proof }
/// ```
/// A borrowed proof cannot be duplicated:
/// ```compile_fail
/// use agentmage_kernel_engine::research_dispatch::ResearchDispatch;
/// fn duplicate(proof: ResearchDispatch<'_>) { let _ = proof.clone(); }
/// ```
/// The research interface alone cannot bypass canonical preflight by becoming
/// an ordinary effect driver:
/// ```compile_fail
/// use agentmage_kernel_engine::authority_transaction::{EffectAuthorization, EffectDriver};
/// use agentmage_kernel_engine::research_dispatch::ResearchEffectDriver;
/// fn bypass(driver: &mut impl ResearchEffectDriver, permit: EffectAuthorization<'_>) {
///     EffectDriver::execute(driver, permit);
/// }
/// ```
pub struct ResearchDispatch<'a> {
    material: &'a FreshResearchDispatch,
}

impl std::fmt::Debug for ResearchDispatch<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResearchDispatch")
            .field("reservation_sha256", &self.material.reservation_sha256)
            .finish_non_exhaustive()
    }
}

impl ResearchDispatch<'_> {
    /// Checks exact packet plus the original interval and the most recent trusted
    /// preflight clock. This is not a substitute for the consumed-grant binding.
    #[must_use]
    pub fn matches_packet_at(&self, packet: &PublicGetWorkerPacket, now_epoch_ms: u64) -> bool {
        self.material.task_id == packet.task_id()
            && self.material.operation_id == packet.request().operation_id
            && self.material.request_sha256 == packet.sha256()
            && now_epoch_ms >= self.material.checked_at_epoch_ms
            && now_epoch_ms >= packet.prepared_at_epoch_ms()
            && now_epoch_ms < packet.deadline_epoch_ms()
    }

    /// Exact immutable spent-accounting identity for native effect material.
    #[must_use]
    pub fn reservation_sha256(&self) -> &str {
        &self.material.reservation_sha256
    }
}

/// Private bridge, not another scheduler or permission evaluator. The canonical
/// owner alone arms it inside the existing begin-effect critical section.
pub(crate) struct ResearchDispatchAdapter<'a, D> {
    driver: &'a mut D,
    fresh: Option<FreshResearchDispatch>,
    armed: bool,
}

impl<'a, D> ResearchDispatchAdapter<'a, D> {
    pub(crate) fn new(driver: &'a mut D) -> Self {
        Self {
            driver,
            fresh: None,
            armed: false,
        }
    }

    pub(crate) fn arm_once(
        &mut self,
        fresh: FreshResearchDispatch,
    ) -> Result<(), ResearchJournalError> {
        if self.armed {
            return Err(ResearchJournalError::Binding);
        }
        self.armed = true;
        self.fresh = Some(fresh);
        Ok(())
    }
}

impl<D: ResearchEffectDriver> EffectDriver for ResearchDispatchAdapter<'_, D> {
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        let Some(fresh) = self.fresh.take() else {
            return EffectLaunch::failed();
        };
        if authorization.operation().operation() != GrantOperation::NetworkAccess
            || authorization.task_id().as_str() != fresh.task_id
            || authorization.call().tool_call_id.as_str() != fresh.operation_id
            || authorization.expected_side_effects().len() != 1
            || authorization.expected_side_effects()[0].details_sha256 != fresh.request_sha256
        {
            return EffectLaunch::failed();
        }
        self.driver
            .execute_research(authorization, ResearchDispatch { material: &fresh })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::authority_transaction::EffectResult;
    use agentmage_kernel_contracts::{OperationOutcome, StateChange};

    // Shared only with canonical-store tests: a synthetic observer, never native I/O.
    pub(crate) struct RecordingResearchDriver<'a> {
        pub(crate) packet: &'a PublicGetWorkerPacket,
        pub(crate) now: u64,
        pub(crate) calls: usize,
        pub(crate) reservation: Option<String>,
    }

    impl ResearchEffectDriver for RecordingResearchDriver<'_> {
        fn execute_research(
            &mut self,
            authorization: EffectAuthorization<'_>,
            dispatch: ResearchDispatch<'_>,
        ) -> EffectLaunch {
            self.calls += 1;
            assert!(authorization.grant_live_at(self.now));
            assert!(dispatch.matches_packet_at(self.packet, self.now));
            assert!(!dispatch.matches_packet_at(self.packet, self.now - 1));
            assert!(!dispatch.matches_packet_at(self.packet, self.packet.deadline_epoch_ms()));
            self.reservation = Some(dispatch.reservation_sha256().to_owned());
            EffectLaunch::completed(EffectResult::from_redacted_material(
                OperationOutcome::Succeeded,
                b"synthetic-canonical-research-dispatch",
                StateChange::NotChanged,
            ))
        }
    }

    #[test]
    fn adapter_cannot_be_rearmed_even_after_its_material_is_taken() {
        fn fresh() -> FreshResearchDispatch {
            FreshResearchDispatch {
                task_id: "task-1".into(),
                operation_id: "call-1".into(),
                request_sha256: "1".repeat(64),
                reservation_sha256: "2".repeat(64),
                checked_at_epoch_ms: 100,
            }
        }
        let mut driver = ();
        let mut adapter = ResearchDispatchAdapter::new(&mut driver);
        assert!(adapter.fresh.is_none());
        adapter.arm_once(fresh()).unwrap();
        assert_eq!(
            adapter.arm_once(fresh()),
            Err(ResearchJournalError::Binding)
        );
        assert!(adapter.fresh.take().is_some());
        assert_eq!(
            adapter.arm_once(fresh()),
            Err(ResearchJournalError::Binding)
        );
    }
}
