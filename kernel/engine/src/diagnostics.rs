//! Kernel-owned redacted local doctor report construction.

use std::collections::BTreeMap;
use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, DiagnosticComponent, DiagnosticItem, DiagnosticObservation,
    DiagnosticState, DoctorReport,
};
use sha2::{Digest, Sha256};

const REPORT_KIND: &str = "agentmage.local-doctor.v1";
const MAX_CODE_BYTES: usize = 128;

/// Fail-closed reason a local doctor report cannot be produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticError {
    /// Two observations attempted to describe the same component.
    DuplicateComponent,
    /// A stable code or optional identity was malformed.
    InvalidObservation,
    /// Canonical report serialization failed.
    SerializationFailed,
}

impl DiagnosticError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DuplicateComponent => "diagnostic.component.duplicate",
            Self::InvalidObservation => "diagnostic.observation.invalid",
            Self::SerializationFailed => "diagnostic.serialization.failed",
        }
    }
}

/// Builds one deterministic report without probing the network or accepting raw content.
pub fn build_doctor_report(
    observations: Vec<DiagnosticObservation>,
) -> Result<DoctorReport, DiagnosticError> {
    let mut indexed = BTreeMap::new();
    for observation in observations {
        if !valid_code(&observation.reason_code)
            || observation
                .identity_sha256
                .as_deref()
                .is_some_and(|value| !valid_sha256(value))
        {
            return Err(DiagnosticError::InvalidObservation);
        }
        if indexed.insert(observation.component, observation).is_some() {
            return Err(DiagnosticError::DuplicateComponent);
        }
    }

    let items: Vec<_> = DiagnosticComponent::ALL
        .into_iter()
        .map(|component| item_for(component, indexed.remove(&component)))
        .collect();
    let overall_state = items
        .iter()
        .map(|item| item.state)
        .max_by_key(|state| severity(*state))
        .unwrap_or(DiagnosticState::Unavailable);
    let unsigned = UnsignedReport {
        schema_version: CONTRACT_SCHEMA_VERSION,
        report_kind: REPORT_KIND,
        overall_state,
        items: &items,
    };
    let bytes = serde_json::to_vec(&unsigned).map_err(|_| DiagnosticError::SerializationFailed)?;
    Ok(DoctorReport {
        schema_version: CONTRACT_SCHEMA_VERSION,
        report_kind: REPORT_KIND.to_owned(),
        overall_state,
        items,
        report_sha256: sha256_hex(&bytes),
    })
}

#[derive(serde::Serialize)]
struct UnsignedReport<'a> {
    schema_version: u16,
    report_kind: &'static str,
    overall_state: DiagnosticState,
    items: &'a [DiagnosticItem],
}

fn item_for(
    component: DiagnosticComponent,
    observation: Option<DiagnosticObservation>,
) -> DiagnosticItem {
    let Some(observation) = observation else {
        return DiagnosticItem {
            component,
            state: DiagnosticState::Unavailable,
            reason_code: "diagnostic.component.missing".to_owned(),
            remediation_code: remediation(component, DiagnosticState::Unavailable).to_owned(),
            identity_sha256: None,
        };
    };
    let (state, reason_code) = if observation.stale
        && matches!(
            observation.state,
            DiagnosticState::Healthy | DiagnosticState::Degraded
        ) {
        (
            DiagnosticState::Degraded,
            "diagnostic.observation.stale".to_owned(),
        )
    } else {
        (observation.state, observation.reason_code)
    };
    DiagnosticItem {
        component,
        state,
        reason_code,
        remediation_code: remediation(component, state).to_owned(),
        identity_sha256: observation.identity_sha256,
    }
}

const fn severity(state: DiagnosticState) -> u8 {
    match state {
        DiagnosticState::Healthy => 0,
        DiagnosticState::Degraded => 1,
        DiagnosticState::Unavailable => 2,
        DiagnosticState::Blocked => 3,
        DiagnosticState::Unsupported => 4,
        DiagnosticState::Quarantined => 5,
    }
}

const fn remediation(component: DiagnosticComponent, state: DiagnosticState) -> &'static str {
    if matches!(state, DiagnosticState::Healthy) {
        return "diagnostic.remediation.none";
    }
    match component {
        DiagnosticComponent::Package => "diagnostic.remediation.verify-package",
        DiagnosticComponent::Platform => "diagnostic.remediation.verify-platform",
        DiagnosticComponent::Model => "diagnostic.remediation.review-model-profile",
        DiagnosticComponent::Runtime => "diagnostic.remediation.restart-runtime",
        DiagnosticComponent::HardwareFit => "diagnostic.remediation.review-hardware-fit",
        DiagnosticComponent::OfflineBoundary => "diagnostic.remediation.restore-offline-boundary",
        DiagnosticComponent::SandboxHelper => "diagnostic.remediation.verify-sandbox-helper",
        DiagnosticComponent::WorkspaceGrant => "diagnostic.remediation.review-workspace-grant",
        DiagnosticComponent::Capabilities => "diagnostic.remediation.verify-capabilities",
        DiagnosticComponent::RepositoryMap => "diagnostic.remediation.rebuild-repository-map",
        DiagnosticComponent::EncryptedStore => "diagnostic.remediation.recover-encrypted-store",
        DiagnosticComponent::ReceiptChain => "diagnostic.remediation.verify-receipt-chain",
        DiagnosticComponent::Recovery => "diagnostic.remediation.resume-or-discard-session",
    }
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{DiagnosticComponent, DiagnosticObservation, DiagnosticState};

    use super::{DiagnosticError, build_doctor_report};

    fn observation(
        component: DiagnosticComponent,
        state: DiagnosticState,
    ) -> DiagnosticObservation {
        DiagnosticObservation {
            component,
            state,
            reason_code: "diagnostic.fixture.observed".to_owned(),
            identity_sha256: Some("a".repeat(64)),
            stale: false,
        }
    }

    #[test]
    fn complete_report_is_stable_ordered_and_content_free() {
        let observations = DiagnosticComponent::ALL
            .into_iter()
            .rev()
            .map(|component| observation(component, DiagnosticState::Healthy))
            .collect();
        let first = build_doctor_report(observations).expect("complete report");
        let second = build_doctor_report(
            DiagnosticComponent::ALL
                .into_iter()
                .map(|component| observation(component, DiagnosticState::Healthy))
                .collect(),
        )
        .expect("same report");
        assert_eq!(first, second);
        assert_eq!(first.overall_state, DiagnosticState::Healthy);
        assert_eq!(first.items.len(), DiagnosticComponent::ALL.len());
        assert_eq!(
            first
                .items
                .iter()
                .map(|item| item.component)
                .collect::<Vec<_>>(),
            DiagnosticComponent::ALL
        );
        let encoded = serde_json::to_string(&first).expect("report JSON");
        for prohibited in [
            "prompt",
            "content",
            "credential",
            "environment",
            "hostname",
            "username",
            "/home/",
        ] {
            assert!(!encoded.contains(prohibited), "{prohibited}");
        }
    }

    #[test]
    fn missing_stale_and_failed_states_are_precise() {
        let report = build_doctor_report(vec![
            observation(DiagnosticComponent::Package, DiagnosticState::Healthy),
            DiagnosticObservation {
                stale: true,
                ..observation(DiagnosticComponent::RepositoryMap, DiagnosticState::Healthy)
            },
            observation(DiagnosticComponent::Model, DiagnosticState::Quarantined),
        ])
        .expect("partial report");
        assert_eq!(report.overall_state, DiagnosticState::Quarantined);
        assert_eq!(report.items[0].state, DiagnosticState::Healthy);
        assert_eq!(report.items[2].state, DiagnosticState::Quarantined);
        assert_eq!(report.items[9].state, DiagnosticState::Degraded);
        assert_eq!(report.items[1].state, DiagnosticState::Unavailable);
    }

    #[test]
    fn duplicate_malformed_and_canary_bearing_values_fail_closed() {
        let duplicate = observation(DiagnosticComponent::Package, DiagnosticState::Healthy);
        assert_eq!(
            build_doctor_report(vec![duplicate.clone(), duplicate]),
            Err(DiagnosticError::DuplicateComponent)
        );
        for value in [
            "Contains Uppercase",
            "diagnostic.canary=/home/private",
            "diagnostic.canary\nsecret",
        ] {
            let mut malformed = observation(DiagnosticComponent::Package, DiagnosticState::Healthy);
            malformed.reason_code = value.to_owned();
            assert_eq!(
                build_doctor_report(vec![malformed]),
                Err(DiagnosticError::InvalidObservation)
            );
        }
    }

    #[test]
    fn every_declared_state_survives_without_state_aliasing() {
        let states = [
            DiagnosticState::Healthy,
            DiagnosticState::Degraded,
            DiagnosticState::Blocked,
            DiagnosticState::Unavailable,
            DiagnosticState::Quarantined,
            DiagnosticState::Unsupported,
        ];
        for state in states {
            let report =
                build_doctor_report(vec![observation(DiagnosticComponent::Package, state)])
                    .expect("state report");
            assert_eq!(report.items[0].state, state);
        }
    }
}
