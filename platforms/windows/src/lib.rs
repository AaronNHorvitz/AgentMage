#![deny(unsafe_op_in_unsafe_fn)]
//! Versioned Windows platform boundary with independently gated native controls.

#[cfg(target_os = "windows")]
mod native_identity;

#[cfg(target_os = "windows")]
pub use native_identity::{
    WindowsNativeIdentityError, WindowsProcessIdentity, observe_windows_process_identity,
};

use agentmage_kernel_contracts::PLATFORM_ADAPTER_API_VERSION;

mod delivery_contract;
pub use delivery_contract::*;
mod runtime_boundary;
pub use runtime_boundary::*;

/// Stable component identity used by architecture and package manifests.
pub const COMPONENT_ID: &str = "platform-windows";

/// First Windows-specific contract version enrolled under Decision 0020.
pub const WINDOWS_PLATFORM_CONTRACT_VERSION: u16 = 1;

/// Exact shared adapter API that this Windows contract must preserve.
pub const SHARED_ADAPTER_API_VERSION: u16 = PLATFORM_ADAPTER_API_VERSION;

/// Native controls required before a Windows adapter may be activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WindowsNativeControl {
    PackageIdentity,
    StandardUserIdentity,
    AuthenticatedNamedPipe,
    HandleRelativeNtfs,
    ReparseAndHardLinkDenial,
    RestrictedWorkerToken,
    JobObjectLimits,
    DpapiKeyProtection,
    DurableAuthorityStore,
    NativeModelRuntime,
    PackageLifecycle,
    RemovalReconciliation,
}

/// Native controls with implemented source and a dedicated Windows execution path.
///
/// This is not an enrollment or release list. Each control still requires its
/// complete hostile matrix and release evidence before enrollment can change.
pub const IMPLEMENTED_NATIVE_CONTROL_SOURCES: [WindowsNativeControl; 1] =
    [WindowsNativeControl::StandardUserIdentity];

/// Complete ordered control inventory for contract version 1.
pub const REQUIRED_NATIVE_CONTROLS: [WindowsNativeControl; 12] = [
    WindowsNativeControl::PackageIdentity,
    WindowsNativeControl::StandardUserIdentity,
    WindowsNativeControl::AuthenticatedNamedPipe,
    WindowsNativeControl::HandleRelativeNtfs,
    WindowsNativeControl::ReparseAndHardLinkDenial,
    WindowsNativeControl::RestrictedWorkerToken,
    WindowsNativeControl::JobObjectLimits,
    WindowsNativeControl::DpapiKeyProtection,
    WindowsNativeControl::DurableAuthorityStore,
    WindowsNativeControl::NativeModelRuntime,
    WindowsNativeControl::PackageLifecycle,
    WindowsNativeControl::RemovalReconciliation,
];

/// Fail-closed reason that prevents a scaffold from becoming an adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsEnrollmentBlocker {
    NativeImplementationUnavailable,
    NativeEvidenceUnavailable,
}

impl WindowsEnrollmentBlocker {
    /// Stable content-free blocker code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NativeImplementationUnavailable => {
                "windows.enrollment.native_implementation_unavailable"
            }
            Self::NativeEvidenceUnavailable => "windows.enrollment.native_evidence_unavailable",
        }
    }
}

/// Returns the current explicit blocker; this crate never substitutes another platform.
pub const fn enrollment_status() -> Result<(), WindowsEnrollmentBlocker> {
    Err(WindowsEnrollmentBlocker::NativeImplementationUnavailable)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        IMPLEMENTED_NATIVE_CONTROL_SOURCES, REQUIRED_NATIVE_CONTROLS, SHARED_ADAPTER_API_VERSION,
        WINDOWS_PLATFORM_CONTRACT_VERSION, WindowsEnrollmentBlocker, WindowsNativeControl,
        enrollment_status,
    };

    #[test]
    fn versioned_boundary_preserves_shared_adapter_contract() {
        assert_eq!(WINDOWS_PLATFORM_CONTRACT_VERSION, 1);
        assert_eq!(SHARED_ADAPTER_API_VERSION, 1);
    }

    #[test]
    fn required_native_controls_are_closed_and_unique() {
        assert_eq!(REQUIRED_NATIVE_CONTROLS.len(), 12);
        assert_eq!(
            REQUIRED_NATIVE_CONTROLS
                .into_iter()
                .collect::<BTreeSet<_>>()
                .len(),
            REQUIRED_NATIVE_CONTROLS.len()
        );
    }

    #[test]
    fn scaffold_cannot_enroll_or_substitute_linux_evidence() {
        let blocker = enrollment_status().expect_err("native implementation is absent");
        assert_eq!(
            blocker,
            WindowsEnrollmentBlocker::NativeImplementationUnavailable
        );
        assert_eq!(
            blocker.code(),
            "windows.enrollment.native_implementation_unavailable"
        );
    }

    #[test]
    fn partial_native_source_never_implies_platform_enrollment() {
        assert_eq!(
            IMPLEMENTED_NATIVE_CONTROL_SOURCES,
            [WindowsNativeControl::StandardUserIdentity]
        );
        assert!(IMPLEMENTED_NATIVE_CONTROL_SOURCES.len() < REQUIRED_NATIVE_CONTROLS.len());
        assert!(enrollment_status().is_err());
    }
}
