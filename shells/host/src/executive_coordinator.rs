//! Authority-free product composition for local executive-assistant projections.

use agentmage_capability_knowledge::built_in_executive_skill_pack;
use agentmage_kernel_contracts::{
    ExecutivePortfolioSnapshot, ExecutivePriorityRanking, ExecutivePrivacyClass,
    ExecutivePrivacyDecision, ExecutivePrivacyOperation, ExecutivePrivacyRequest, ExecutiveRecord,
    ExecutiveTracker, ExecutiveTrackerKind, ExecutiveView, ExecutiveViewKind,
};
use agentmage_kernel_engine::executive_assistant::{
    ExecutiveViewRequest, build_executive_view, build_portfolio_snapshot, build_priority_ranking,
    build_tracker, evaluate_executive_privacy,
};

/// Stable failure from the local executive-assistant product composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutiveCoordinatorError {
    /// Canonical records, privacy scope, or projection input failed closed.
    InvalidInput,
    /// The admitted built-in skill pack was unavailable or no longer authority-free.
    SkillPackInvalid,
    /// A projection attempted to claim an external effect or notification capability.
    AuthorityViolation,
}

impl ExecutiveCoordinatorError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "executive.coordinator.input-invalid",
            Self::SkillPackInvalid => "executive.coordinator.skill-pack-invalid",
            Self::AuthorityViolation => "executive.coordinator.authority-violation",
        }
    }
}

impl std::fmt::Display for ExecutiveCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ExecutiveCoordinatorError {}

/// Exact caller-owned identifiers and privacy scope for one local executive projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutiveCoordinatorRequest {
    /// Stable immutable portfolio snapshot identity.
    pub snapshot_id: String,
    /// Stable deterministic ranking identity.
    pub ranking_id: String,
    /// Stable tracker identity.
    pub tracker_id: String,
    /// Tracker class to render.
    pub tracker_kind: ExecutiveTrackerKind,
    /// Stable executive-view identity.
    pub view_id: String,
    /// View class to render.
    pub view_kind: ExecutiveViewKind,
    /// Exact privacy classes admitted by an already-authorized local read.
    pub admitted_privacy_classes: Vec<ExecutivePrivacyClass>,
}

/// One consistent source-backed local executive workspace with no effect authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutiveCoordinatorOutcome {
    /// Immutable canonical source snapshot shared by every projection.
    pub snapshot: ExecutivePortfolioSnapshot,
    /// Deterministic source-backed priority recommendation.
    pub ranking: ExecutivePriorityRanking,
    /// Deterministic local tracker without reminder or notification authority.
    pub tracker: ExecutiveTracker,
    /// Deterministic local executive view.
    pub view: ExecutiveView,
    /// Content-free privacy decisions for every current record.
    pub privacy: Vec<ExecutivePrivacyDecision>,
    /// Exact number of admitted authority-free built-in executive skills.
    pub admitted_skill_count: u32,
    /// Fixed false: composition cannot send, schedule, notify, write, or mutate a source.
    pub external_effect_allowed: bool,
}

/// Builds one internally consistent executive workspace from already-approved local records.
pub fn coordinate_executive_workspace(
    current_records: Vec<ExecutiveRecord>,
    history: Vec<ExecutiveRecord>,
    request: ExecutiveCoordinatorRequest,
) -> Result<ExecutiveCoordinatorOutcome, ExecutiveCoordinatorError> {
    let packages =
        built_in_executive_skill_pack().map_err(|_| ExecutiveCoordinatorError::SkillPackInvalid)?;
    if packages.len() != 8 {
        return Err(ExecutiveCoordinatorError::SkillPackInvalid);
    }
    let snapshot = build_portfolio_snapshot(request.snapshot_id, current_records, history)
        .map_err(|_| ExecutiveCoordinatorError::InvalidInput)?;
    let ranking = build_priority_ranking(
        request.ranking_id,
        &snapshot.current_records,
        &request.admitted_privacy_classes,
    )
    .map_err(|_| ExecutiveCoordinatorError::InvalidInput)?;
    let tracker = build_tracker(
        request.tracker_id,
        request.tracker_kind,
        &snapshot.current_records,
        &request.admitted_privacy_classes,
    )
    .map_err(|_| ExecutiveCoordinatorError::InvalidInput)?;
    let view = build_executive_view(
        &snapshot,
        ExecutiveViewRequest {
            view_id: request.view_id,
            kind: request.view_kind,
            admitted_privacy_classes: request.admitted_privacy_classes,
            prior_snapshot: None,
        },
    )
    .map_err(|_| ExecutiveCoordinatorError::InvalidInput)?;
    let privacy = snapshot
        .current_records
        .iter()
        .map(|record| {
            evaluate_executive_privacy(&ExecutivePrivacyRequest {
                record_id: record.record_id.clone(),
                privacy_class: record.privacy_class,
                operation: ExecutivePrivacyOperation::Retrieve,
                exact_source: true,
                explicit_user_approval: matches!(
                    record.privacy_class,
                    ExecutivePrivacyClass::Confidential | ExecutivePrivacyClass::HighlyRestricted
                ),
                retention_days: None,
            })
        })
        .collect::<Vec<_>>();
    if !ranking.proposal_only
        || ranking.external_effect_allowed
        || tracker.notification_allowed
        || !view.proposal_only
        || view.external_effect_allowed
        || snapshot.source_mutation_allowed
        || privacy.iter().any(|decision| decision.effect_performed)
    {
        return Err(ExecutiveCoordinatorError::AuthorityViolation);
    }
    Ok(ExecutiveCoordinatorOutcome {
        snapshot,
        ranking,
        tracker,
        view,
        privacy,
        admitted_skill_count: packages.len() as u32,
        external_effect_allowed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ExecutiveDueWindow, ExecutiveEvidenceState, ExecutiveRecordKind,
        ExecutiveRecordStatus, ExecutiveSourceReference, ExecutiveSourceStore,
    };

    use super::*;

    fn record(id: &str, privacy_class: ExecutivePrivacyClass) -> ExecutiveRecord {
        ExecutiveRecord {
            schema_version: CONTRACT_SCHEMA_VERSION,
            record_id: id.to_owned(),
            kind: ExecutiveRecordKind::Task,
            title: format!("Title {id}"),
            summary: format!("Summary for {id}"),
            status: ExecutiveRecordStatus::Active,
            evidence_state: ExecutiveEvidenceState::Confirmed,
            owner: Some("Owner One".to_owned()),
            counterparty: None,
            project_id: Some("project-one".to_owned()),
            due_date: Some("2026-09-02".to_owned()),
            due_window: ExecutiveDueWindow::WithinSevenDays,
            estimated_effort_minutes: Some(30),
            urgency_bps: 7_000,
            importance_bps: 8_000,
            user_preference_bps: 5_000,
            consequence_bps: 6_000,
            schedule_conflict: false,
            dependency_ids: Vec::new(),
            fields: Vec::new(),
            sources: vec![ExecutiveSourceReference {
                source_id: format!("source-{id}"),
                object_id: format!("object-{id}"),
                fragment: Some("heading-1".to_owned()),
                content_sha256: "1".repeat(64),
                observed_revision: Some("revision-1".to_owned()),
                store: ExecutiveSourceStore::PlainFolder,
            }],
            privacy_class,
            supersedes_record_id: None,
        }
    }

    fn request(classes: Vec<ExecutivePrivacyClass>) -> ExecutiveCoordinatorRequest {
        ExecutiveCoordinatorRequest {
            snapshot_id: "executive-snapshot-native".to_owned(),
            ranking_id: "executive-ranking-native".to_owned(),
            tracker_id: "executive-reminders-native".to_owned(),
            tracker_kind: ExecutiveTrackerKind::Reminders,
            view_id: "executive-start-native".to_owned(),
            view_kind: ExecutiveViewKind::StartOfCycle,
            admitted_privacy_classes: classes,
        }
    }

    #[test]
    fn story_54_native_composition_binds_every_projection_to_one_source_snapshot() {
        let outcome = coordinate_executive_workspace(
            vec![record("task-one", ExecutivePrivacyClass::Ordinary)],
            Vec::new(),
            request(vec![ExecutivePrivacyClass::Ordinary]),
        )
        .expect("coordinated executive workspace");
        assert_eq!(outcome.ranking.entries.len(), 1);
        assert_eq!(outcome.tracker.entries.len(), 1);
        assert_eq!(outcome.view.items.len(), 1);
        assert_eq!(
            outcome.view.snapshot_sha256,
            outcome.snapshot.snapshot_sha256
        );
        assert_eq!(outcome.admitted_skill_count, 8);
        assert!(!outcome.external_effect_allowed);
        assert!(!outcome.tracker.notification_allowed);
        assert!(
            outcome
                .privacy
                .iter()
                .all(|decision| !decision.effect_performed)
        );
    }

    #[test]
    fn story_54_privacy_scope_fails_closed_without_partial_cross_class_views() {
        assert_eq!(
            coordinate_executive_workspace(
                vec![
                    record("task-ordinary", ExecutivePrivacyClass::Ordinary),
                    record("task-confidential", ExecutivePrivacyClass::Confidential),
                ],
                Vec::new(),
                request(vec![ExecutivePrivacyClass::Ordinary]),
            ),
            Err(ExecutiveCoordinatorError::InvalidInput)
        );
    }

    #[test]
    fn story_54_duplicate_privacy_scope_and_changes_since_without_prior_fail_closed() {
        assert_eq!(
            coordinate_executive_workspace(
                vec![record("task-one", ExecutivePrivacyClass::Ordinary)],
                Vec::new(),
                request(vec![
                    ExecutivePrivacyClass::Ordinary,
                    ExecutivePrivacyClass::Ordinary,
                ]),
            ),
            Err(ExecutiveCoordinatorError::InvalidInput)
        );
        let mut changed = request(vec![ExecutivePrivacyClass::Ordinary]);
        changed.view_kind = ExecutiveViewKind::ChangesSince;
        assert_eq!(
            coordinate_executive_workspace(
                vec![record("task-one", ExecutivePrivacyClass::Ordinary)],
                Vec::new(),
                changed,
            ),
            Err(ExecutiveCoordinatorError::InvalidInput)
        );
    }
}
