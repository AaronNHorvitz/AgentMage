#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Bounded read-only capability-pack scaffold.

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "capability-read-only";

/// Returns the identity of the contracts implemented by this capability pack.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

#[cfg(test)]
mod tests {
    use super::{COMPONENT_ID, contract_component_id};

    #[test]
    fn capability_depends_only_on_contracts() {
        assert_eq!(COMPONENT_ID, "capability-read-only");
        assert_eq!(contract_component_id(), "kernel-contracts");
    }
}
