#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

/// Deterministic non-authoritative approval-display construction.
pub mod approval;
/// Sealed classification and denial of descriptive artifacts as authority.
pub mod authority;
/// Kernel-owned authority-transaction ordering and recovery contract.
pub mod authority_transaction;
/// Versioned, fail-closed configuration loading and recovery.
pub mod configuration;
/// Kernel-mediated configuration filesystem effects.
pub mod configuration_effect;
/// Kernel-only session and operation grant issuance.
pub mod grants;
/// Encrypted canonical operational state and crash recovery.
pub mod operational_store;
/// Fail-closed platform capability detection and adapter activation.
pub mod platform_startup;
/// Deterministic deny-first capability-grant policy evaluation.
pub mod policy;
/// Cancellation trees and lossless typed failure propagation.
pub mod propagation;
/// Stateful resource budgets and explicit sticky stop conditions.
pub mod run_control;
/// Strict-local endpoint policy, storage admission, and content-free attempt ledger.
pub mod strict_local;
/// Exact tool registration, call validation, and pre-grant dispatch denial.
pub mod tooling;
/// Bounded work-packet validation, revision history, and plan adaptation.
pub mod work_packet;

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
