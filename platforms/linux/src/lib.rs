#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Fedora and Ubuntu platform-adapter scaffold.

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "platform-linux";

/// Returns the identity of the contracts implemented by this adapter.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

#[cfg(test)]
mod tests {
    use super::{COMPONENT_ID, contract_component_id};

    #[test]
    fn adapter_depends_only_on_contracts() {
        assert_eq!(COMPONENT_ID, "platform-linux");
        assert_eq!(contract_component_id(), "kernel-contracts");
    }
}
