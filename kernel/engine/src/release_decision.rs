//! Fail-closed superseding release-decision evaluation.
#![allow(missing_docs)]
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReleaseBlockerClass {
    Platform,
    Provider,
    Authority,
    Credential,
    Disclosure,
    Backup,
    Restore,
    Model,
    AuditCoverage,
    ReadOnly,
    Reconciliation,
    Security,
    Accessibility,
    Recovery,
    Support,
    Removal,
    Evidence,
    Fuzzing,
    Reviewer,
    UserApproval,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateState {
    Passed,
    Failed,
    Skipped,
    Stale,
    Unavailable,
    Flaky,
    Quarantined,
    Suppressed,
    Unreconciled,
    Unreviewed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseGate {
    pub gate_id: String,
    pub blocker_class: ReleaseBlockerClass,
    pub state: GateState,
    pub evidence_sha256: Option<String>,
    pub reason_code: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseDecision {
    pub release_id: String,
    pub gates: Vec<ReleaseGate>,
    pub required_classes: BTreeSet<ReleaseBlockerClass>,
    pub independent_reproduction_complete: bool,
    pub explicit_user_approval: bool,
    pub manifests_signed: bool,
    pub package_publication_allowed: bool,
    pub ga_closed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseDecisionError {
    Invalid,
    MissingClass,
    BlockingState,
    ApprovalAbsent,
    FalsePublication,
}
fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
pub fn evaluate(value: &ReleaseDecision) -> Result<(), ReleaseDecisionError> {
    if !id(&value.release_id) || value.gates.is_empty() || value.required_classes.is_empty() {
        return Err(ReleaseDecisionError::Invalid);
    }
    let represented = value
        .gates
        .iter()
        .map(|gate| gate.blocker_class)
        .collect::<BTreeSet<_>>();
    if !value.required_classes.is_subset(&represented) {
        return Err(ReleaseDecisionError::MissingClass);
    }
    for gate in &value.gates {
        if !id(&gate.gate_id) {
            return Err(ReleaseDecisionError::Invalid);
        }
        if gate.state == GateState::Passed {
            if gate
                .evidence_sha256
                .as_deref()
                .is_none_or(|item| !digest(item))
            {
                return Err(ReleaseDecisionError::Invalid);
            }
        } else if gate.reason_code.as_deref().is_none_or(|item| !id(item)) {
            return Err(ReleaseDecisionError::Invalid);
        }
    }
    let blocking = value
        .gates
        .iter()
        .any(|gate| gate.state != GateState::Passed);
    if blocking && (value.package_publication_allowed || value.ga_closed) {
        return Err(ReleaseDecisionError::BlockingState);
    }
    let approved = value.independent_reproduction_complete
        && value.explicit_user_approval
        && value.manifests_signed;
    if (value.package_publication_allowed || value.ga_closed) && !approved {
        return Err(ReleaseDecisionError::ApprovalAbsent);
    }
    if value.package_publication_allowed != value.ga_closed {
        return Err(ReleaseDecisionError::FalsePublication);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_non_pass_state_blocks_publication() {
        for state in [
            GateState::Failed,
            GateState::Skipped,
            GateState::Stale,
            GateState::Unavailable,
            GateState::Flaky,
            GateState::Quarantined,
            GateState::Suppressed,
            GateState::Unreconciled,
            GateState::Unreviewed,
        ] {
            let mut value = decision(state);
            assert_eq!(evaluate(&value), Ok(()));
            value.ga_closed = true;
            value.package_publication_allowed = true;
            assert_eq!(evaluate(&value), Err(ReleaseDecisionError::BlockingState));
        }
    }
    #[test]
    fn approval_and_signatures_are_mandatory() {
        let mut value = decision(GateState::Passed);
        value.gates[0].reason_code = None;
        value.gates[0].evidence_sha256 = Some("a".repeat(64));
        value.ga_closed = true;
        value.package_publication_allowed = true;
        assert_eq!(evaluate(&value), Err(ReleaseDecisionError::ApprovalAbsent));
    }
    fn decision(state: GateState) -> ReleaseDecision {
        ReleaseDecision {
            release_id: "v1".into(),
            gates: vec![ReleaseGate {
                gate_id: "platform".into(),
                blocker_class: ReleaseBlockerClass::Platform,
                state,
                evidence_sha256: None,
                reason_code: Some("not-pass".into()),
            }],
            required_classes: BTreeSet::from([ReleaseBlockerClass::Platform]),
            independent_reproduction_complete: false,
            explicit_user_approval: false,
            manifests_signed: false,
            package_publication_allowed: false,
            ga_closed: false,
        }
    }
}
