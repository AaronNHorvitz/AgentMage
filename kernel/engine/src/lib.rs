#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

/// Versioned, fail-closed configuration loading and recovery.
pub mod configuration;
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
