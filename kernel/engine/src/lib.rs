#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Security-authoritative AgentMage kernel scaffold.

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
