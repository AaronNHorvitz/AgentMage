#![forbid(unsafe_code)]
//! Versioned, non-effecting Windows platform boundary.

use agentmage_kernel_contracts::PLATFORM_ADAPTER_API_VERSION;

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
        REQUIRED_NATIVE_CONTROLS, SHARED_ADAPTER_API_VERSION, WINDOWS_PLATFORM_CONTRACT_VERSION,
        WindowsEnrollmentBlocker, enrollment_status,
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
}
