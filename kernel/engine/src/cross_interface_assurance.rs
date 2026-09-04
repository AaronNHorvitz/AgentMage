//! Interface-invariant authority, isolation, recovery, and privacy assurance.

use std::collections::{BTreeMap, BTreeSet};

/// Closed interface and extension boundary family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterfaceBoundary {
    /// Visual Studio Code thin client.
    VisualStudioCode,
    /// Command-line thin client.
    Cli,
    /// Native desktop thin client.
    Desktop,
    /// Structured JSON boundary.
    Json,
    /// Software development kit boundary.
    Sdk,
    /// Agent Client Protocol boundary.
    Acp,
    /// Admitted package boundary.
    Package,
    /// Lifecycle hook boundary.
    Hook,
    /// Model Context Protocol boundary.
    Mcp,
    /// Connector boundary.
    Connector,
    /// Browser boundary.
    Browser,
    /// Schedule boundary.
    Schedule,
    /// Child-agent boundary.
    ChildAgent,
}

/// Exact authority result observed at one boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundaryResult {
    /// Boundary under test.
    pub boundary: InterfaceBoundary,
    /// Exact attributable actor.
    pub actor_sha256: String,
    /// Exact current grant.
    pub grant_sha256: String,
    /// Exact current precondition.
    pub precondition_sha256: String,
    /// Exact terminal receipt.
    pub receipt_sha256: String,
    /// Stable policy decision shared by every boundary.
    pub decision: String,
    /// Cancellation propagated to descendants.
    pub cancellation_propagated: bool,
    /// Sandbox remained enforced.
    pub sandbox_enforced: bool,
    /// Offline policy remained enforced.
    pub offline_enforced: bool,
    /// Any effect escaped the exact grant.
    pub unauthorized_effect_count: u32,
}

/// Complete cross-interface scenario.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityParityScenario {
    /// Exact scenario identity.
    pub scenario_sha256: String,
    /// Results keyed by every closed boundary.
    pub results: BTreeMap<InterfaceBoundary, BoundaryResult>,
}

/// Seven independently bound isolation identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IsolationScope {
    /// Account identity.
    pub account_sha256: String,
    /// Workspace identity.
    pub workspace_sha256: String,
    /// Project identity.
    pub project_sha256: String,
    /// Agent identity.
    pub agent_sha256: String,
    /// Worktree identity.
    pub worktree_sha256: String,
    /// Connector identity.
    pub connector_sha256: String,
    /// Transport identity.
    pub transport_sha256: String,
}

/// Privacy-canary observation spanning a closed persistence/export path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyCanaryResult {
    /// Closed path class.
    pub path_class: String,
    /// Canary digest, never canary bytes.
    pub canary_sha256: String,
    /// Unauthorized persistence occurrences.
    pub unauthorized_persistence_count: u32,
    /// Unauthorized disclosure occurrences.
    pub unauthorized_disclosure_count: u32,
    /// Cleanup was verified.
    pub cleanup_verified: bool,
    /// Classification and minimization passed.
    pub classification_and_minimization_passed: bool,
    /// Encryption or explicit non-persistence passed.
    pub encryption_or_nonpersistence_passed: bool,
    /// Retention and redaction passed.
    pub retention_and_redaction_passed: bool,
}

/// Stable assurance refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssuranceError {
    /// Boundary closure or identity shape is invalid.
    InvalidRecord,
    /// Interface decisions differ for one scenario.
    ParityFailure,
    /// An authority, sandbox, offline, or cancellation bypass occurred.
    AuthorityBypass,
    /// Isolation identities were pooled or reused.
    IsolationFailure,
    /// A privacy canary persisted or escaped.
    PrivacyFailure,
}

/// Return the exact thirteen boundary families.
#[must_use]
pub fn all_boundaries() -> BTreeSet<InterfaceBoundary> {
    [
        InterfaceBoundary::VisualStudioCode,
        InterfaceBoundary::Cli,
        InterfaceBoundary::Desktop,
        InterfaceBoundary::Json,
        InterfaceBoundary::Sdk,
        InterfaceBoundary::Acp,
        InterfaceBoundary::Package,
        InterfaceBoundary::Hook,
        InterfaceBoundary::Mcp,
        InterfaceBoundary::Connector,
        InterfaceBoundary::Browser,
        InterfaceBoundary::Schedule,
        InterfaceBoundary::ChildAgent,
    ]
    .into_iter()
    .collect()
}

/// Validate exact parity without granting or executing any effect.
pub fn validate_authority_parity(value: &AuthorityParityScenario) -> Result<(), AssuranceError> {
    if !valid_sha256(&value.scenario_sha256)
        || value.results.keys().copied().collect::<BTreeSet<_>>() != all_boundaries()
        || value.results.iter().any(|(boundary, result)| {
            result.boundary != *boundary
                || ![
                    &result.actor_sha256,
                    &result.grant_sha256,
                    &result.precondition_sha256,
                    &result.receipt_sha256,
                ]
                .into_iter()
                .all(|digest| valid_sha256(digest))
                || !valid_id(&result.decision)
        })
    {
        return Err(AssuranceError::InvalidRecord);
    }
    let decisions = value
        .results
        .values()
        .map(|result| result.decision.as_str())
        .collect::<BTreeSet<_>>();
    if decisions.len() != 1 {
        return Err(AssuranceError::ParityFailure);
    }
    if value.results.values().any(|result| {
        !result.cancellation_propagated
            || !result.sandbox_enforced
            || !result.offline_enforced
            || result.unauthorized_effect_count != 0
    }) {
        return Err(AssuranceError::AuthorityBypass);
    }
    Ok(())
}

/// Validate seven exact and distinct isolation identities.
pub fn validate_isolation(value: &IsolationScope) -> Result<(), AssuranceError> {
    let values = [
        &value.account_sha256,
        &value.workspace_sha256,
        &value.project_sha256,
        &value.agent_sha256,
        &value.worktree_sha256,
        &value.connector_sha256,
        &value.transport_sha256,
    ];
    if !values.iter().all(|digest| valid_sha256(digest))
        || values
            .iter()
            .map(|value| value.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != values.len()
    {
        return Err(AssuranceError::IsolationFailure);
    }
    Ok(())
}

/// Validate one content-free privacy-canary result.
pub fn validate_privacy_canary(value: &PrivacyCanaryResult) -> Result<(), AssuranceError> {
    let allowed = [
        "audit",
        "cache",
        "credential",
        "export",
        "input",
        "output",
        "screenshot",
        "transcript",
    ];
    if !allowed.contains(&value.path_class.as_str()) || !valid_sha256(&value.canary_sha256) {
        return Err(AssuranceError::InvalidRecord);
    }
    if value.unauthorized_persistence_count != 0
        || value.unauthorized_disclosure_count != 0
        || !value.cleanup_verified
        || !value.classification_and_minimization_passed
        || !value.encryption_or_nonpersistence_passed
        || !value.retention_and_redaction_passed
    {
        return Err(AssuranceError::PrivacyFailure);
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
    fn h(seed: u8) -> String {
        format!("{seed:02x}").repeat(32)
    }
    fn scenario() -> AuthorityParityScenario {
        let results = all_boundaries()
            .into_iter()
            .enumerate()
            .map(|(i, boundary)| {
                (
                    boundary,
                    BoundaryResult {
                        boundary,
                        actor_sha256: h(i as u8 + 1),
                        grant_sha256: h(i as u8 + 20),
                        precondition_sha256: h(i as u8 + 40),
                        receipt_sha256: h(i as u8 + 60),
                        decision: "deny-unsupported-native-effect".into(),
                        cancellation_propagated: true,
                        sandbox_enforced: true,
                        offline_enforced: true,
                        unauthorized_effect_count: 0,
                    },
                )
            })
            .collect();
        AuthorityParityScenario {
            scenario_sha256: h(99),
            results,
        }
    }
    #[test]
    fn thirteen_boundaries_have_identical_fail_closed_authority() {
        assert_eq!(validate_authority_parity(&scenario()), Ok(()));
    }
    #[test]
    fn missing_boundary_decision_drift_and_bypass_fail() {
        let mut v = scenario();
        v.results.remove(&InterfaceBoundary::Sdk);
        assert_eq!(
            validate_authority_parity(&v),
            Err(AssuranceError::InvalidRecord)
        );
        v = scenario();
        v.results.get_mut(&InterfaceBoundary::Cli).unwrap().decision = "allow".into();
        assert_eq!(
            validate_authority_parity(&v),
            Err(AssuranceError::ParityFailure)
        );
        v = scenario();
        v.results
            .get_mut(&InterfaceBoundary::Package)
            .unwrap()
            .unauthorized_effect_count = 1;
        assert_eq!(
            validate_authority_parity(&v),
            Err(AssuranceError::AuthorityBypass)
        );
    }
    #[test]
    fn isolation_rejects_identity_pooling() {
        let mut v = IsolationScope {
            account_sha256: h(1),
            workspace_sha256: h(2),
            project_sha256: h(3),
            agent_sha256: h(4),
            worktree_sha256: h(5),
            connector_sha256: h(6),
            transport_sha256: h(7),
        };
        assert_eq!(validate_isolation(&v), Ok(()));
        v.transport_sha256 = v.account_sha256.clone();
        assert_eq!(
            validate_isolation(&v),
            Err(AssuranceError::IsolationFailure)
        );
    }
    #[test]
    fn privacy_canary_requires_zero_escape_and_complete_cleanup() {
        let mut v = PrivacyCanaryResult {
            path_class: "transcript".into(),
            canary_sha256: h(8),
            unauthorized_persistence_count: 0,
            unauthorized_disclosure_count: 0,
            cleanup_verified: true,
            classification_and_minimization_passed: true,
            encryption_or_nonpersistence_passed: true,
            retention_and_redaction_passed: true,
        };
        assert_eq!(validate_privacy_canary(&v), Ok(()));
        v.unauthorized_disclosure_count = 1;
        assert_eq!(
            validate_privacy_canary(&v),
            Err(AssuranceError::PrivacyFailure)
        );
    }
}
