#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

/// One-active-step plan history, progress, interruption, and response validation.
pub mod agent_progress;
pub mod agent_proposal;
/// Bounded single-agent observe, plan, action-proposal, and review control loop.
pub mod agent_runtime;
pub mod agent_state;
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
/// Kernel-only session and operation grant issuance.
pub mod grants;
/// Encrypted canonical operational state and crash recovery.
pub mod operational_store;
/// Deterministic pre-persistence classification, minimization, and receipts.
pub mod persistence;
/// Fail-closed platform capability detection and adapter activation.
pub mod platform_startup;
/// Deterministic deny-first capability-grant policy evaluation.
pub mod policy;
/// Cancellation trees and lossless typed failure propagation.
pub mod propagation;
/// Bounded concise reasoning records and deterministic verification gates.
pub mod reasoning;
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
