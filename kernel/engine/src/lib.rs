#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

/// Sealed classification and denial of descriptive artifacts as authority.
pub mod authority;
/// Versioned, fail-closed configuration loading and recovery.
pub mod configuration;
/// Kernel-only session and operation grant issuance.
pub mod grants;
/// Deterministic deny-first capability-grant policy evaluation.
pub mod policy;
/// Cancellation trees and lossless typed failure propagation.
pub mod propagation;
/// Stateful resource budgets and explicit sticky stop conditions.
pub mod run_control;
/// Exact tool registration, call validation, and pre-grant dispatch denial.
pub mod tooling;
/// Bounded work-packet validation, revision history, and plan adaptation.
pub mod work_packet;

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
