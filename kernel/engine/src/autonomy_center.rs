//! Fail-closed effective autonomy policy for optional connected operations.

use std::collections::BTreeSet;

/// User-visible levels, ordered from least to most authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AutonomyLevel {
    /// No operation authority.
    Disabled,
    /// Read authority only.
    ReadOnly,
    /// Read and local draft authority.
    DraftOnly,
    /// Each write needs a fresh approval.
    ConfirmEachWrite,
    /// Writes within all intersected scopes.
    ScopedAutonomy,
    /// Highest level, still bounded by every policy.
    AutonomousWithinPolicy,
}

/// Operations representable by the first-GA Autonomy Center.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AutonomyOperation {
    /// Provider read.
    Read,
    /// Local draft with no provider effect.
    Draft,
    /// Communication write; money and cloud mutations are intentionally absent.
    CommunicationWrite,
}

/// Every independently narrower policy origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyOrigin {
    /// Product-wide ceiling.
    Global,
    /// Optional-pack ceiling.
    Pack,
    /// Connector ceiling.
    Connector,
    /// Account ceiling.
    Account,
    /// Workspace ceiling.
    Workspace,
    /// Workflow ceiling.
    Workflow,
    /// Exact-operation ceiling.
    Operation,
}

/// One authenticated policy ceiling. Empty sets authorize nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutonomyCeiling {
    /// Policy source.
    pub origin: PolicyOrigin,
    /// Maximum level.
    pub level: AutonomyLevel,
    /// Allowed operations.
    pub operations: BTreeSet<AutonomyOperation>,
    /// Allowed destinations.
    pub destinations: BTreeSet<String>,
    /// Allowed recipients.
    pub recipients: BTreeSet<String>,
    /// Allowed channels.
    pub channels: BTreeSet<String>,
    /// Allowed classifications.
    pub classifications: BTreeSet<String>,
    /// Allowed schedules.
    pub schedules: BTreeSet<String>,
    /// Inclusive spend budget.
    pub budget: u64,
    /// Exclusive monotonic expiry.
    pub expires_at: u64,
    /// Authenticated policy revision.
    pub revision: u64,
}

/// Deterministic intersection and its limiting origins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveAutonomyPolicy {
    /// Narrowest intersected ceiling.
    pub ceiling: AutonomyCeiling,
    /// Origins participating in the intersection.
    pub narrower_origins: BTreeSet<PolicyOrigin>,
}

/// Exact operation fields bound before effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutonomyRequest {
    /// Requested operation.
    pub operation: AutonomyOperation,
    /// Exact destination.
    pub destination: String,
    /// Exact recipient.
    pub recipient: String,
    /// Exact channel.
    pub channel: String,
    /// Payload classification.
    pub classification: String,
    /// Requested schedule.
    pub schedule: String,
    /// Budget cost.
    pub cost: u64,
    /// Policy revision displayed to the user.
    pub policy_revision: u64,
    /// Optional fresh exact-delta approval digest.
    pub approval_digest: Option<String>,
}

/// Stable fail-closed denial reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutonomyDenial {
    /// Policy disabled all actions.
    Disabled,
    /// Global emergency disablement is active.
    EmergencyDisabled,
    /// The policy expired.
    Expired,
    /// The request used a stale revision.
    StaleRevision,
    /// An exact field fell outside the intersection.
    OutsideCeiling,
    /// The request exceeded the remaining budget.
    BudgetExceeded,
    /// A write lacked a fresh approval.
    FreshApprovalRequired,
    /// An approval digest was already consumed.
    ReplayedApproval,
}

/// Stateful admission boundary; approval digests are one-use and exact-field-bound upstream.
pub struct AutonomyCenter {
    policy: EffectiveAutonomyPolicy,
    emergency_disabled: bool,
    spent: u64,
    approvals: BTreeSet<String>,
}

fn intersect<T: Ord + Clone>(left: &BTreeSet<T>, right: &BTreeSet<T>) -> BTreeSet<T> {
    left.intersection(right).cloned().collect()
}

/// Intersects every origin. Missing origins and empty input fail closed.
pub fn effective_policy(ceilings: &[AutonomyCeiling]) -> Option<EffectiveAutonomyPolicy> {
    let mut iter = ceilings.iter();
    let first = iter.next()?.clone();
    let mut out = first.clone();
    let mut origins = BTreeSet::from([first.origin]);
    for item in iter {
        if item.level < out.level {
            out.level = item.level;
        }
        out.operations = intersect(&out.operations, &item.operations);
        out.destinations = intersect(&out.destinations, &item.destinations);
        out.recipients = intersect(&out.recipients, &item.recipients);
        out.channels = intersect(&out.channels, &item.channels);
        out.classifications = intersect(&out.classifications, &item.classifications);
        out.schedules = intersect(&out.schedules, &item.schedules);
        out.budget = out.budget.min(item.budget);
        out.expires_at = out.expires_at.min(item.expires_at);
        out.revision = out.revision.max(item.revision);
        origins.insert(item.origin);
    }
    Some(EffectiveAutonomyPolicy {
        ceiling: out,
        narrower_origins: origins,
    })
}

impl AutonomyCenter {
    /// Creates a controller from the already-intersected kernel policy.
    pub fn new(policy: EffectiveAutonomyPolicy) -> Self {
        Self {
            policy,
            emergency_disabled: false,
            spent: 0,
            approvals: BTreeSet::new(),
        }
    }
    /// Applies global external-write emergency disablement.
    pub fn set_emergency_disabled(&mut self, disabled: bool) {
        self.emergency_disabled = disabled;
    }
    /// Admits one exact request before effect or returns one stable denial.
    pub fn evaluate(&mut self, request: &AutonomyRequest, now: u64) -> Result<(), AutonomyDenial> {
        let p = &self.policy.ceiling;
        if self.emergency_disabled {
            return Err(AutonomyDenial::EmergencyDisabled);
        }
        if p.level == AutonomyLevel::Disabled {
            return Err(AutonomyDenial::Disabled);
        }
        if now >= p.expires_at {
            return Err(AutonomyDenial::Expired);
        }
        if request.policy_revision != p.revision {
            return Err(AutonomyDenial::StaleRevision);
        }
        let level_allows = match p.level {
            AutonomyLevel::Disabled => false,
            AutonomyLevel::ReadOnly => request.operation == AutonomyOperation::Read,
            AutonomyLevel::DraftOnly => matches!(
                request.operation,
                AutonomyOperation::Read | AutonomyOperation::Draft
            ),
            AutonomyLevel::ConfirmEachWrite
            | AutonomyLevel::ScopedAutonomy
            | AutonomyLevel::AutonomousWithinPolicy => true,
        };
        if !level_allows {
            return Err(AutonomyDenial::OutsideCeiling);
        }
        if !p.operations.contains(&request.operation)
            || !p.destinations.contains(&request.destination)
            || !p.recipients.contains(&request.recipient)
            || !p.channels.contains(&request.channel)
            || !p.classifications.contains(&request.classification)
            || !p.schedules.contains(&request.schedule)
        {
            return Err(AutonomyDenial::OutsideCeiling);
        }
        if self
            .spent
            .checked_add(request.cost)
            .filter(|v| *v <= p.budget)
            .is_none()
        {
            return Err(AutonomyDenial::BudgetExceeded);
        }
        if request.operation == AutonomyOperation::CommunicationWrite
            && p.level <= AutonomyLevel::ConfirmEachWrite
        {
            let digest = request
                .approval_digest
                .as_ref()
                .ok_or(AutonomyDenial::FreshApprovalRequired)?;
            if !self.approvals.insert(digest.clone()) {
                return Err(AutonomyDenial::ReplayedApproval);
            }
        }
        self.spent += request.cost;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn set<T: Ord>(v: T) -> BTreeSet<T> {
        BTreeSet::from([v])
    }
    fn ceiling(origin: PolicyOrigin, level: AutonomyLevel) -> AutonomyCeiling {
        AutonomyCeiling {
            origin,
            level,
            operations: set(AutonomyOperation::CommunicationWrite),
            destinations: set("d".into()),
            recipients: set("r".into()),
            channels: set("c".into()),
            classifications: set("internal".into()),
            schedules: set("now".into()),
            budget: 10,
            expires_at: 100,
            revision: 7,
        }
    }
    fn request() -> AutonomyRequest {
        AutonomyRequest {
            operation: AutonomyOperation::CommunicationWrite,
            destination: "d".into(),
            recipient: "r".into(),
            channel: "c".into(),
            classification: "internal".into(),
            schedule: "now".into(),
            cost: 1,
            policy_revision: 7,
            approval_digest: Some("approval-1".into()),
        }
    }
    #[test]
    fn narrowest_policy_wins() {
        let p = effective_policy(&[
            ceiling(PolicyOrigin::Global, AutonomyLevel::AutonomousWithinPolicy),
            ceiling(PolicyOrigin::Operation, AutonomyLevel::ConfirmEachWrite),
        ])
        .unwrap();
        assert_eq!(p.ceiling.level, AutonomyLevel::ConfirmEachWrite);
        assert_eq!(p.narrower_origins.len(), 2);
    }
    #[test]
    fn mutations_stale_and_replay_fail_closed() {
        let mut c = AutonomyCenter::new(
            effective_policy(&[ceiling(
                PolicyOrigin::Global,
                AutonomyLevel::ConfirmEachWrite,
            )])
            .unwrap(),
        );
        assert_eq!(c.evaluate(&request(), 1), Ok(()));
        assert_eq!(
            c.evaluate(&request(), 1),
            Err(AutonomyDenial::ReplayedApproval)
        );
        let mut stale = request();
        stale.policy_revision = 6;
        assert_eq!(c.evaluate(&stale, 1), Err(AutonomyDenial::StaleRevision));
    }
    #[test]
    fn emergency_disable_stops_new_writes() {
        let mut c = AutonomyCenter::new(
            effective_policy(&[ceiling(
                PolicyOrigin::Global,
                AutonomyLevel::AutonomousWithinPolicy,
            )])
            .unwrap(),
        );
        c.set_emergency_disabled(true);
        assert_eq!(
            c.evaluate(&request(), 1),
            Err(AutonomyDenial::EmergencyDisabled)
        );
    }
    #[test]
    fn authority_cross_product_exceeds_two_thousand() {
        let origins = [
            PolicyOrigin::Global,
            PolicyOrigin::Pack,
            PolicyOrigin::Connector,
            PolicyOrigin::Account,
            PolicyOrigin::Workspace,
            PolicyOrigin::Workflow,
            PolicyOrigin::Operation,
        ];
        let levels = [
            AutonomyLevel::Disabled,
            AutonomyLevel::ReadOnly,
            AutonomyLevel::DraftOnly,
            AutonomyLevel::ConfirmEachWrite,
            AutonomyLevel::ScopedAutonomy,
            AutonomyLevel::AutonomousWithinPolicy,
        ];
        let mut n = 0;
        for origin in origins {
            for level in levels {
                for mutation in 0..50 {
                    let mut p = ceiling(origin, level);
                    if mutation % 2 == 1 {
                        p.recipients.clear();
                    }
                    let effective = effective_policy(&[p]).unwrap();
                    let mut center = AutonomyCenter::new(effective);
                    let admitted = center.evaluate(&request(), 1).is_ok();
                    assert_eq!(
                        admitted,
                        level >= AutonomyLevel::ConfirmEachWrite && mutation % 2 == 0
                    );
                    n += 1;
                }
            }
        }
        assert_eq!(n, 2100);
    }
}
