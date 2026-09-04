//! Safe-mode capability closure and content-free maintenance diagnostics.

use std::collections::BTreeSet;

/// Optional authority domains that safe mode must disable together.
pub const OPTIONAL_CAPABILITIES: [&str; 8] = [
    "browser",
    "child-agent",
    "connector",
    "executable",
    "mcp",
    "network",
    "package",
    "schedule",
];

/// One startup integrity and recovery observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryObservation {
    /// Canonical-state digest.
    pub canonical_state_sha256: String,
    /// Last-known-good digest.
    pub last_known_good_sha256: String,
    /// Database integrity passed.
    pub database_valid: bool,
    /// Index integrity passed or the index can be rebuilt.
    pub indexes_valid_or_rebuildable: bool,
    /// Orphan cleanup completed without deleting canonical state.
    pub orphan_cleanup_complete: bool,
    /// Every optional capability is disabled.
    pub disabled_capabilities: BTreeSet<String>,
    /// Diagnostics remain available.
    pub diagnostics_available: bool,
    /// Evidence and recovery remain available.
    pub evidence_and_recovery_available: bool,
}

/// Content-free diagnostic receipt spanning all later capability classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaintenanceDiagnostic {
    /// Stable diagnostic identity.
    pub diagnostic_id: String,
    /// Closed subsystem name.
    pub subsystem: String,
    /// Stable result code.
    pub result_code: String,
    /// Digest of redacted details.
    pub redacted_detail_sha256: String,
    /// Whether raw content was retained.
    pub raw_content_present: bool,
    /// Whether a credential or secret was retained.
    pub secret_present: bool,
    /// Whether the diagnostic grants authority.
    pub authority_granted: bool,
}

/// Stable maintenance recovery disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryDisposition {
    /// Current state is valid and may remain active.
    KeepCurrent,
    /// Rebuild non-canonical indexes before leaving safe mode.
    RebuildIndexes,
    /// Restore the exact last-known-good candidate after separate approval.
    ProposeLastKnownGood,
    /// Remain in safe mode because no verified state is available.
    BlockedSafeMode,
}

/// Safe-mode validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafeModeError {
    /// Required identity or diagnostic shape is invalid.
    InvalidRecord,
    /// An optional authority remains enabled.
    CapabilityEnabled,
    /// Raw or secret content, or authority, entered diagnostics.
    DiagnosticLeak,
}

/// Validate safe-mode closure and choose a non-mutating recovery proposal.
pub fn evaluate_recovery(
    value: &RecoveryObservation,
) -> Result<RecoveryDisposition, SafeModeError> {
    let required = OPTIONAL_CAPABILITIES
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if !valid_sha256(&value.canonical_state_sha256)
        || !valid_sha256(&value.last_known_good_sha256)
        || !value.diagnostics_available
        || !value.evidence_and_recovery_available
    {
        return Err(SafeModeError::InvalidRecord);
    }
    if value.disabled_capabilities != required {
        return Err(SafeModeError::CapabilityEnabled);
    }
    Ok(
        if value.database_valid
            && value.indexes_valid_or_rebuildable
            && value.orphan_cleanup_complete
        {
            RecoveryDisposition::KeepCurrent
        } else if value.database_valid && !value.indexes_valid_or_rebuildable {
            RecoveryDisposition::RebuildIndexes
        } else if value.canonical_state_sha256 != value.last_known_good_sha256 {
            RecoveryDisposition::ProposeLastKnownGood
        } else {
            RecoveryDisposition::BlockedSafeMode
        },
    )
}

/// Validate one content-free diagnostic receipt.
pub fn validate_diagnostic(value: &MaintenanceDiagnostic) -> Result<(), SafeModeError> {
    let allowed = [
        "child-agent",
        "connector",
        "desktop",
        "mcp",
        "package",
        "schedule",
    ];
    if !valid_id(&value.diagnostic_id)
        || !allowed.contains(&value.subsystem.as_str())
        || !valid_id(&value.result_code)
        || !valid_sha256(&value.redacted_detail_sha256)
    {
        return Err(SafeModeError::InvalidRecord);
    }
    if value.raw_content_present || value.secret_present || value.authority_granted {
        return Err(SafeModeError::DiagnosticLeak);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation() -> RecoveryObservation {
        RecoveryObservation {
            canonical_state_sha256: "1".repeat(64),
            last_known_good_sha256: "2".repeat(64),
            database_valid: true,
            indexes_valid_or_rebuildable: true,
            orphan_cleanup_complete: true,
            disabled_capabilities: OPTIONAL_CAPABILITIES
                .into_iter()
                .map(str::to_owned)
                .collect(),
            diagnostics_available: true,
            evidence_and_recovery_available: true,
        }
    }
    #[test]
    fn safe_mode_disables_every_optional_capability() {
        assert_eq!(
            evaluate_recovery(&observation()),
            Ok(RecoveryDisposition::KeepCurrent)
        );
    }
    #[test]
    fn enabled_capability_and_corrupt_state_fail_or_propose_recovery() {
        let mut value = observation();
        value.disabled_capabilities.remove("network");
        assert_eq!(
            evaluate_recovery(&value),
            Err(SafeModeError::CapabilityEnabled)
        );
        value = observation();
        value.database_valid = false;
        assert_eq!(
            evaluate_recovery(&value),
            Ok(RecoveryDisposition::ProposeLastKnownGood)
        );
    }
    #[test]
    fn diagnostic_is_content_free_and_non_authoritative() {
        let mut value = MaintenanceDiagnostic {
            diagnostic_id: "diag-1".into(),
            subsystem: "connector".into(),
            result_code: "credential-revoked".into(),
            redacted_detail_sha256: "a".repeat(64),
            raw_content_present: false,
            secret_present: false,
            authority_granted: false,
        };
        assert_eq!(validate_diagnostic(&value), Ok(()));
        value.secret_present = true;
        assert_eq!(
            validate_diagnostic(&value),
            Err(SafeModeError::DiagnosticLeak)
        );
    }
}
