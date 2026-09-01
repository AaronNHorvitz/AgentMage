//! Authority-free product composition for source-preserving meeting workflows.

use agentmage_capability_knowledge::built_in_meeting_skill_pack;
use agentmage_kernel_contracts::{
    MeetingCloseout, MeetingContinuityRecord, MeetingContinuityUpdate, MeetingMinutes,
    MeetingPlanDraft, MeetingProjectionPrecondition, MeetingTranscriptCleanup,
};
use agentmage_kernel_engine::meeting_continuity::{
    MeetingContinuityError, admit_meeting_projection, build_meeting_closeout,
    build_meeting_continuity, verify_meeting_minutes, verify_meeting_plan_draft,
    verify_transcript_cleanup,
};

/// Stable failure from the local meeting-workflow composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetingCoordinatorError {
    /// The caller cancelled before any projection began.
    Cancelled,
    /// A required already-approved local dependency is unavailable.
    DependencyUnavailable,
    /// A sealed record, identity, source set, or continuity input failed closed.
    InvalidInput,
    /// The admitted built-in meeting skill pack is unavailable or incomplete.
    SkillPackInvalid,
    /// A supplied or derived record claimed an external effect.
    AuthorityViolation,
}

impl MeetingCoordinatorError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "meeting.coordinator.cancelled",
            Self::DependencyUnavailable => "meeting.coordinator.dependency-unavailable",
            Self::InvalidInput => "meeting.coordinator.input-invalid",
            Self::SkillPackInvalid => "meeting.coordinator.skill-pack-invalid",
            Self::AuthorityViolation => "meeting.coordinator.authority-violation",
        }
    }
}

impl std::fmt::Display for MeetingCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for MeetingCoordinatorError {}

impl From<MeetingContinuityError> for MeetingCoordinatorError {
    fn from(error: MeetingContinuityError) -> Self {
        match error {
            MeetingContinuityError::Cancelled => Self::Cancelled,
            MeetingContinuityError::DependencyUnavailable => Self::DependencyUnavailable,
            MeetingContinuityError::InvalidInput
            | MeetingContinuityError::IntegrityFailure
            | MeetingContinuityError::StaleRecord => Self::InvalidInput,
        }
    }
}

/// Caller-owned identities and explicit state for one local meeting composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingCoordinatorRequest {
    /// Explicit dependency and sticky-cancellation state.
    pub precondition: MeetingProjectionPrecondition,
    /// Stable identity for the next immutable continuity record.
    pub continuity_record_id: String,
    /// Stable recurring-meeting series identity.
    pub series_id: String,
    /// Sorted exact source-backed updates to prior continuity items.
    pub continuity_updates: Vec<MeetingContinuityUpdate>,
    /// Local follow-up draft retained for user review only.
    pub follow_up_draft: String,
}

/// One source-consistent local meeting workspace with no effect authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetingCoordinatorOutcome {
    /// Exact verified local agenda or meeting-request draft.
    pub plan: MeetingPlanDraft,
    /// Exact verified verbatim-preserving cleanup proposal.
    pub cleanup: MeetingTranscriptCleanup,
    /// Exact verified local minutes proposal.
    pub minutes: MeetingMinutes,
    /// Immutable source-linked recurring continuity record.
    pub continuity: MeetingContinuityRecord,
    /// Content-free counts derived from the exact minutes and continuity record.
    pub closeout: MeetingCloseout,
    /// Exact number of admitted authority-free built-in meeting skills.
    pub admitted_skill_count: u32,
    /// Fixed false: composition cannot invite, assign, notify, send, schedule, or mutate sources.
    pub external_effect_allowed: bool,
}

/// Composes exact sealed meeting records into one consistent local-only workspace.
pub fn coordinate_meeting_workspace(
    plan: MeetingPlanDraft,
    cleanup: MeetingTranscriptCleanup,
    minutes: MeetingMinutes,
    prior_continuity: Option<&MeetingContinuityRecord>,
    request: MeetingCoordinatorRequest,
) -> Result<MeetingCoordinatorOutcome, MeetingCoordinatorError> {
    admit_meeting_projection(request.precondition)?;

    let packages =
        built_in_meeting_skill_pack().map_err(|_| MeetingCoordinatorError::SkillPackInvalid)?;
    if packages.len() != 6 {
        return Err(MeetingCoordinatorError::SkillPackInvalid);
    }

    verify_meeting_plan_draft(&plan)?;
    verify_transcript_cleanup(&cleanup)?;
    verify_meeting_minutes(&minutes)?;

    if plan.meeting_id != cleanup.meeting_id
        || plan.meeting_id != minutes.meeting_id
        || plan.sources != cleanup.sources
        || plan.sources != minutes.sources
        || plan.participants != minutes.participants
    {
        return Err(MeetingCoordinatorError::InvalidInput);
    }

    let continuity = build_meeting_continuity(
        request.continuity_record_id,
        request.series_id,
        &minutes,
        prior_continuity,
        &request.continuity_updates,
        request.follow_up_draft,
    )?;
    let closeout = build_meeting_closeout(&minutes, &continuity)?;

    if !plan.proposal_only
        || plan.external_effects_performed
        || plan.calendar_effect_performed
        || !cleanup.proposal_only
        || cleanup.source_mutation_performed
        || !minutes.proposal_only
        || minutes.commitment_effect_performed
        || minutes.communication_effect_performed
        || minutes.calendar_effect_performed
        || !continuity.proposal_only
        || continuity.communication_effect_performed
        || continuity.source_mutation_performed
        || closeout.external_effects_performed
    {
        return Err(MeetingCoordinatorError::AuthorityViolation);
    }

    Ok(MeetingCoordinatorOutcome {
        plan,
        cleanup,
        minutes,
        continuity,
        closeout,
        admitted_skill_count: packages.len() as u32,
        external_effect_allowed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ExecutiveEvidenceState, ExecutivePrivacyClass,
        ExecutiveSourceReference, ExecutiveSourceStore, MeetingAttendanceState, MeetingAttendee,
        MeetingDraftKind, MeetingFieldState, MeetingInvitationState, MeetingMinutesItem,
        MeetingMinutesItemKind, MeetingPlanItem, MeetingTextSourceKind, MeetingTranscriptSegment,
    };
    use agentmage_kernel_engine::meeting_continuity::{
        seal_meeting_minutes, seal_meeting_plan_draft, seal_transcript_cleanup,
    };

    use super::*;

    fn source() -> ExecutiveSourceReference {
        ExecutiveSourceReference {
            source_id: "source-1".to_owned(),
            object_id: "object-1".to_owned(),
            fragment: Some("meeting".to_owned()),
            content_sha256: "a".repeat(64),
            observed_revision: Some("revision-1".to_owned()),
            store: ExecutiveSourceStore::PlainFolder,
        }
    }

    fn attendee() -> MeetingAttendee {
        MeetingAttendee {
            participant_id: "person-1".to_owned(),
            display_name: "Person One".to_owned(),
            invitation_state: MeetingInvitationState::Accepted,
            invitation_evidence_state: ExecutiveEvidenceState::Confirmed,
            attendance_state: MeetingAttendanceState::Attended,
            attendance_evidence_state: ExecutiveEvidenceState::Confirmed,
            source_ids: vec!["source-1".to_owned()],
        }
    }

    fn plan() -> MeetingPlanDraft {
        seal_meeting_plan_draft(MeetingPlanDraft {
            schema_version: CONTRACT_SCHEMA_VERSION,
            draft_id: "plan-1".to_owned(),
            meeting_id: "meeting-1".to_owned(),
            kind: MeetingDraftKind::Agenda,
            purpose: "Review the exact local evidence.".to_owned(),
            participants: vec![attendee()],
            topics: vec![MeetingPlanItem {
                item_id: "topic-1".to_owned(),
                text: "Review evidence.".to_owned(),
                source_ids: vec!["source-1".to_owned()],
            }],
            decision_needs: Vec::new(),
            preparation_items: Vec::new(),
            expected_outputs: Vec::new(),
            sources: vec![source()],
            draft_sha256: String::new(),
            proposal_only: true,
            external_effects_performed: false,
            calendar_effect_performed: false,
        })
        .expect("sealed plan")
    }

    fn cleanup() -> MeetingTranscriptCleanup {
        seal_transcript_cleanup(MeetingTranscriptCleanup {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cleanup_id: "cleanup-1".to_owned(),
            meeting_id: "meeting-1".to_owned(),
            segments: vec![MeetingTranscriptSegment {
                segment_id: "segment-1".to_owned(),
                source_kind: MeetingTextSourceKind::LiveNote,
                verbatim_text: "Review evidence".to_owned(),
                cleaned_text: "Review evidence.".to_owned(),
                started_at: None,
                ended_at: None,
                speaker_label: None,
                attribution_confidence_bps: 0,
                attribution_evidence_state: ExecutiveEvidenceState::Unknown,
                unclear_markers: Vec::new(),
                source_ids: vec!["source-1".to_owned()],
            }],
            sources: vec![source()],
            cleanup_sha256: String::new(),
            proposal_only: true,
            source_mutation_performed: false,
        })
        .expect("sealed cleanup")
    }

    fn minutes() -> MeetingMinutes {
        seal_meeting_minutes(MeetingMinutes {
            schema_version: CONTRACT_SCHEMA_VERSION,
            minutes_id: "minutes-1".to_owned(),
            meeting_id: "meeting-1".to_owned(),
            participants: vec![attendee()],
            items: vec![MeetingMinutesItem {
                item_id: "action-1".to_owned(),
                kind: MeetingMinutesItemKind::Action,
                text: "Investigate the open question.".to_owned(),
                evidence_state: ExecutiveEvidenceState::Confirmed,
                owner: None,
                owner_state: MeetingFieldState::Unknown,
                due_date: None,
                due_date_state: MeetingFieldState::Unknown,
                carried_from_item_id: None,
                source_ids: vec!["source-1".to_owned()],
            }],
            sources: vec![source()],
            privacy_class: ExecutivePrivacyClass::Private,
            minutes_sha256: String::new(),
            proposal_only: true,
            commitment_effect_performed: false,
            communication_effect_performed: false,
            calendar_effect_performed: false,
        })
        .expect("sealed minutes")
    }

    fn request() -> MeetingCoordinatorRequest {
        MeetingCoordinatorRequest {
            precondition: MeetingProjectionPrecondition {
                dependencies_ready: true,
                cancellation_requested: false,
            },
            continuity_record_id: "continuity-1".to_owned(),
            series_id: "series-1".to_owned(),
            continuity_updates: Vec::new(),
            follow_up_draft: "Review this local follow-up draft.".to_owned(),
        }
    }

    #[test]
    fn story_55_product_composition_binds_all_records_and_six_skills_without_effects() {
        let outcome = coordinate_meeting_workspace(plan(), cleanup(), minutes(), None, request())
            .expect("meeting workspace");
        assert_eq!(outcome.admitted_skill_count, 6);
        assert_eq!(outcome.closeout.action_count, 1);
        assert_eq!(outcome.closeout.unknown_owner_action_count, 1);
        assert_eq!(outcome.continuity.items.len(), 1);
        assert!(!outcome.external_effect_allowed);
        assert!(!outcome.continuity.communication_effect_performed);
        assert!(!outcome.continuity.source_mutation_performed);
    }

    #[test]
    fn story_55_product_composition_rejects_cross_meeting_and_cross_source_records() {
        let mut wrong_meeting = cleanup();
        wrong_meeting.meeting_id = "meeting-2".to_owned();
        assert_eq!(
            coordinate_meeting_workspace(plan(), wrong_meeting, minutes(), None, request()),
            Err(MeetingCoordinatorError::InvalidInput)
        );

        let mut wrong_source = cleanup();
        wrong_source.sources[0].fragment = Some("different".to_owned());
        wrong_source = seal_transcript_cleanup(MeetingTranscriptCleanup {
            cleanup_sha256: String::new(),
            ..wrong_source
        })
        .expect("resealed cleanup");
        assert_eq!(
            coordinate_meeting_workspace(plan(), wrong_source, minutes(), None, request()),
            Err(MeetingCoordinatorError::InvalidInput)
        );
    }

    #[test]
    fn story_55_product_composition_honors_sticky_cancellation_and_dependency_failure() {
        let mut cancelled = request();
        cancelled.precondition.cancellation_requested = true;
        assert_eq!(
            coordinate_meeting_workspace(plan(), cleanup(), minutes(), None, cancelled),
            Err(MeetingCoordinatorError::Cancelled)
        );

        let mut unavailable = request();
        unavailable.precondition.dependencies_ready = false;
        assert_eq!(
            coordinate_meeting_workspace(plan(), cleanup(), minutes(), None, unavailable),
            Err(MeetingCoordinatorError::DependencyUnavailable)
        );
    }
}
