#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

/// Authority-reducing application of advisory classifier output.
pub mod advisory_policy;
pub mod agent_ceiling;
/// One-active-step plan history, progress, interruption, and response validation.
pub mod agent_progress;
pub mod agent_proposal;
pub mod agent_restart;
/// Bounded single-agent observe, plan, action-proposal, and review control loop.
pub mod agent_runtime;
pub mod agent_state;
pub mod agent_verifier;
/// Deterministic non-authoritative approval-display construction.
pub mod approval;
/// Metadata-only attachment classification and exact workspace path resolution.
pub mod attachment;
/// Sealed classification and denial of descriptive artifacts as authority.
pub mod authority;
/// Kernel-owned authority-transaction ordering and recovery contract.
pub mod authority_transaction;
/// Deterministic material-claim proof and truthful final-response construction.
pub mod claim_evidence;
/// Versioned, fail-closed configuration loading and recovery.
pub mod configuration;
/// Deterministic bounded context, checked summaries, checkpoints, and drift gates.
pub mod context_management;
/// Separately keyed private conversation archives and controlled lifecycle operations.
pub mod conversation_archive;
/// Encrypted canonical conversation records and immutable turn timelines.
pub mod conversation_library;
/// Kernel-owned redacted local doctor report construction.
pub mod diagnostics;
/// Exact-preview redacted evidence bundles derived from canonical conversation state.
pub mod evidence_bundle;
/// Fresh citation resolution, complete answer ledgers, and keyed receipt integrity.
pub mod evidence_reconciliation;
/// Deterministic assignment of explicit evidence states to material claims.
pub mod evidence_state;
/// Kernel-only session and operation grant issuance.
pub mod grants;
/// Deterministic local-only manual handoff construction and denial.
pub mod handoff;
/// Hash-bound discovery, reading, and narrowing-only trust for untrusted instructions.
pub mod instruction_provenance;
pub mod model_codec;
/// Candidate-neutral native-picker projection and stale-selection refusal.
pub mod model_discovery;
pub mod model_response;
/// Candidate-neutral exact-profile admission and local runtime orchestration.
pub mod model_runtime;
/// Explicit exact-profile selection, deterministic-first dispatch, and resource control.
pub mod model_selection;
/// Encrypted canonical operational state and crash recovery.
pub mod operational_store;
/// Deterministic pre-persistence classification, minimization, and receipts.
pub mod persistence;
/// Fail-closed platform capability detection and adapter activation.
pub mod platform_startup;
/// Deterministic deny-first capability-grant policy evaluation.
pub mod policy;
/// Deterministic static and typed-fact checks before advisory classification.
pub mod preclassification_policy;
/// Cancellation trees and lossless typed failure propagation.
pub mod propagation;
/// Bounded concise reasoning records and deterministic verification gates.
pub mod reasoning;
/// Continuous content reclassification before successive trust boundaries.
pub mod reclassification;
/// Stateful resource budgets and explicit sticky stop conditions.
pub mod run_control;
/// Bounded, configuration-bound session environment and provenance capture.
pub mod session_environment;
/// Strict-local endpoint policy, storage admission, and content-free attempt ledger.
pub mod strict_local;
/// Deterministic descriptive task intent, complexity, and risk classification.
pub mod task_classification;
/// Exact tool registration, call validation, and pre-grant dispatch denial.
pub mod tooling;
/// Bounded work-packet validation, revision history, and plan adaptation.
pub mod work_packet;
/// Exact-preimage shadow changes, review previews, and bounded write grants.
pub mod write_approval;

#[cfg(test)]
mod s012_it01;
#[cfg(test)]
mod s012_rt01;
#[cfg(test)]
mod s012_ut02;
#[cfg(test)]
mod test_target;

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "kernel-engine";

/// Returns the identity of the contract package used by the kernel.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

#[cfg(test)]
mod tests {
    use super::{COMPONENT_ID, contract_component_id};

    #[test]
    fn dependency_points_toward_contracts() {
        assert_eq!(COMPONENT_ID, "kernel-engine");
        assert_eq!(contract_component_id(), "kernel-contracts");
    }
}
