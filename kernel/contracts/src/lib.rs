#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Interface-independent contracts shared across AgentMage components.

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "kernel-contracts";

#[cfg(test)]
mod tests {
    use super::COMPONENT_ID;

    #[test]
    fn component_identity_is_stable() {
        assert_eq!(COMPONENT_ID, "kernel-contracts");
    }
}
