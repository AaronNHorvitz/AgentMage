//! Bounded host coordination for manual, local-only frontier recommendations.

use std::collections::BTreeMap;

use agentmage_kernel_contracts::{
    FrontierRecommendationReceipt, FrontierTierEvidence, HandoffProhibitedAction,
    LocalHandoffReceipt, RenderedHandoff,
};
use agentmage_kernel_engine::{
    frontier_recommendation::{
        FrontierPacketPreview, FrontierPacketRequest, build_frontier_packet_preview,
        decide_frontier_tier, record_frontier_recommendation, render_frontier_packet,
    },
    handoff::{cancel_handoff, deny_handoff_action},
};

const MAX_PENDING_FRONTIER_REVIEWS: usize = 4;

/// Stable failure from the local frontier coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierCoordinatorError {
    /// Current measured evidence, packet material, or confirmation failed closed.
    InvalidRequest,
    /// The bounded number of pending exact reviews has been reached.
    CapacityExceeded,
    /// The requested one-use review does not exist or was already consumed.
    ReviewUnavailable,
}

impl FrontierCoordinatorError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "frontier.coordinator.request-invalid",
            Self::CapacityExceeded => "frontier.coordinator.capacity-exceeded",
            Self::ReviewUnavailable => "frontier.coordinator.review-unavailable",
        }
    }
}

impl std::fmt::Display for FrontierCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FrontierCoordinatorError {}

/// One completed local render and content-free recommendation record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierCoordinatorOutcome {
    /// Exact reviewed packet rendered only into the caller's local response.
    pub rendered: RenderedHandoff,
    /// Content-free recommendation bookkeeping receipt.
    pub recommendation: FrontierRecommendationReceipt,
}

#[derive(Clone)]
struct PendingFrontierReview {
    request: FrontierPacketRequest,
    preview: FrontierPacketPreview,
}

/// Memory-only owner of bounded, one-use frontier reviews.
#[derive(Default)]
pub struct FrontierRecommendationCoordinator {
    pending: BTreeMap<String, PendingFrontierReview>,
}

impl FrontierRecommendationCoordinator {
    /// Creates an empty coordinator with no ambient model, filesystem, or network capability.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
        }
    }

    /// Recomputes the measured tier and prepares one exact local-only review.
    pub fn preview(
        &mut self,
        evidence: &FrontierTierEvidence,
        request: FrontierPacketRequest,
        preview_id: String,
        now_ms: u64,
        expires_at_ms: u64,
    ) -> Result<FrontierPacketPreview, FrontierCoordinatorError> {
        self.expire(now_ms);
        if self.pending.len() >= MAX_PENDING_FRONTIER_REVIEWS {
            return Err(FrontierCoordinatorError::CapacityExceeded);
        }
        if now_ms == 0
            || expires_at_ms <= now_ms
            || request.tier_evidence != *evidence
            || request.decision
                != decide_frontier_tier(evidence)
                    .map_err(|_| FrontierCoordinatorError::InvalidRequest)?
            || self.pending.contains_key(&preview_id)
        {
            return Err(FrontierCoordinatorError::InvalidRequest);
        }
        let preview = build_frontier_packet_preview(&request, preview_id.clone(), expires_at_ms)
            .map_err(|_| FrontierCoordinatorError::InvalidRequest)?;
        self.pending.insert(
            preview_id,
            PendingFrontierReview {
                request,
                preview: preview.clone(),
            },
        );
        Ok(preview)
    }

    /// Consumes one exact review, renders locally, and records no delivery effect.
    #[allow(clippy::too_many_arguments)]
    pub fn render_and_record(
        &mut self,
        preview_id: &str,
        confirmation_sha256: &str,
        now_ms: u64,
        non_public_acknowledged: bool,
        attempt_id: String,
        receipt_id: String,
        destination_recording_requested: bool,
        user_recorded_destination: Option<String>,
    ) -> Result<FrontierCoordinatorOutcome, FrontierCoordinatorError> {
        let pending = self
            .pending
            .remove(preview_id)
            .ok_or(FrontierCoordinatorError::ReviewUnavailable)?;
        let rendered = render_frontier_packet(
            &pending.request,
            &pending.preview,
            confirmation_sha256,
            now_ms,
            non_public_acknowledged,
            attempt_id,
        )
        .map_err(|_| FrontierCoordinatorError::InvalidRequest)?;
        let recommendation = record_frontier_recommendation(
            receipt_id,
            &pending.request.decision,
            &pending.preview,
            destination_recording_requested,
            user_recorded_destination,
        )
        .map_err(|_| FrontierCoordinatorError::InvalidRequest)?;
        if rendered.receipt.external_delivery_attempted
            || rendered.manifest.delivered
            || recommendation.external_delivery_attempted
        {
            return Err(FrontierCoordinatorError::InvalidRequest);
        }
        Ok(FrontierCoordinatorOutcome {
            rendered,
            recommendation,
        })
    }

    /// Consumes a pending review and returns one local cancellation receipt.
    pub fn cancel(
        &mut self,
        preview_id: &str,
        attempt_id: String,
    ) -> Result<LocalHandoffReceipt, FrontierCoordinatorError> {
        let pending = self
            .pending
            .remove(preview_id)
            .ok_or(FrontierCoordinatorError::ReviewUnavailable)?;
        cancel_handoff(attempt_id, pending.request.packet_id)
            .map_err(|_| FrontierCoordinatorError::InvalidRequest)
    }

    /// Produces a local denial receipt for a prohibited delivery action.
    pub fn deny_delivery(
        &self,
        attempt_id: String,
        packet_id: Option<String>,
        action: HandoffProhibitedAction,
    ) -> Result<LocalHandoffReceipt, FrontierCoordinatorError> {
        deny_handoff_action(attempt_id, packet_id, action)
            .map_err(|_| FrontierCoordinatorError::InvalidRequest)
    }

    fn expire(&mut self, now_ms: u64) {
        self.pending
            .retain(|_, pending| pending.preview.review.expires_at_ms > now_ms);
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, FrontierAcceptanceState, FrontierClarificationClass,
        HandoffDisclosureEntry, HandoffEntryDisposition, HandoffEntryKind, HandoffSensitivity,
        LocalHandoffOutcome,
    };
    use agentmage_kernel_engine::handoff::seal_handoff_entry;

    use super::*;
    use agentmage_kernel_engine::frontier_recommendation::{
        FrontierPacketEvidence, FrontierPacketEvidenceRole,
    };

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn tier_evidence() -> FrontierTierEvidence {
        FrontierTierEvidence {
            schema_version: CONTRACT_SCHEMA_VERSION,
            decision_id: "frontier-native-decision".to_owned(),
            deterministic_available: false,
            deterministic_succeeded: false,
            local_model_attempted: true,
            local_acceptance_check_id: "check.native-result".to_owned(),
            local_acceptance_state: FrontierAcceptanceState::Failed,
            validation_failure_count: 0,
            contradiction_count: 0,
            verification_rejected: false,
            budget_exhausted: false,
            clarification: FrontierClarificationClass::None,
            evidence_sha256: vec![SHA_A.to_owned(), SHA_B.to_owned()],
        }
    }

    fn entry(id: &str) -> HandoffDisclosureEntry {
        seal_handoff_entry(HandoffDisclosureEntry {
            entry_id: id.to_owned(),
            source_id: format!("source-{id}"),
            display_source: "src/module.rs".to_owned(),
            fragment: "lines:1-3".to_owned(),
            excerpt: "bounded current evidence".to_owned(),
            content_sha256: SHA_A.to_owned(),
            kind: HandoffEntryKind::SourceExcerpt,
            sensitivity: HandoffSensitivity::Public,
            disposition: HandoffEntryDisposition::Include,
            redactions: Vec::new(),
            hidden: false,
            related: true,
            entry_sha256: String::new(),
        })
        .expect("sealed entry")
    }

    fn request(evidence: &FrontierTierEvidence) -> FrontierPacketRequest {
        FrontierPacketRequest {
            tier_evidence: evidence.clone(),
            decision: decide_frontier_tier(evidence).expect("measured recommendation"),
            packet_id: "frontier-native-packet".to_owned(),
            workspace_state_sha256: SHA_A.to_owned(),
            policy_sha256: SHA_A.to_owned(),
            redaction_policy_sha256: SHA_A.to_owned(),
            objective: "Review the failed local acceptance check".to_owned(),
            acceptance_checks: vec!["Return cited findings".to_owned()],
            constraints: vec!["No external action".to_owned()],
            authority_boundary: vec!["Manual user transfer only".to_owned()],
            evidence: [
                (FrontierPacketEvidenceRole::CurrentState, "entry-current"),
                (FrontierPacketEvidenceRole::Citation, "entry-citation"),
                (FrontierPacketEvidenceRole::Receipt, "entry-receipt"),
            ]
            .into_iter()
            .map(|(role, id)| FrontierPacketEvidence {
                role,
                entry: entry(id),
                authority_object: false,
            })
            .collect(),
            exclusions: vec!["Credentials and unrelated files".to_owned()],
            unresolved_questions: vec!["Which local assumption failed?".to_owned()],
            required_output_contract: "JSON findings with citations".to_owned(),
        }
    }

    #[test]
    fn measured_native_preview_renders_records_and_never_delivers() {
        let evidence = tier_evidence();
        let mut coordinator = FrontierRecommendationCoordinator::new();
        let preview = coordinator
            .preview(
                &evidence,
                request(&evidence),
                "preview-native".to_owned(),
                10,
                20,
            )
            .expect("native preview");
        let outcome = coordinator
            .render_and_record(
                "preview-native",
                &preview.review.confirmation_sha256,
                19,
                false,
                "attempt-native".to_owned(),
                "receipt-native".to_owned(),
                true,
                Some("User selected separate interface".to_owned()),
            )
            .expect("one local render");
        assert_eq!(
            outcome.rendered.packet_markdown,
            preview.review.packet_markdown
        );
        assert!(!outcome.rendered.manifest.delivered);
        assert!(!outcome.rendered.receipt.external_delivery_attempted);
        assert!(!outcome.recommendation.external_delivery_attempted);
        assert_eq!(
            coordinator.render_and_record(
                "preview-native",
                &preview.review.confirmation_sha256,
                19,
                false,
                "attempt-replay".to_owned(),
                "receipt-replay".to_owned(),
                false,
                None,
            ),
            Err(FrontierCoordinatorError::ReviewUnavailable)
        );
    }

    #[test]
    fn stale_measured_state_mutation_expiry_and_bad_confirmation_fail_closed() {
        let evidence = tier_evidence();
        let mut coordinator = FrontierRecommendationCoordinator::new();
        let mut drifted = evidence.clone();
        drifted.evidence_sha256[0] = SHA_B.to_owned();
        assert_eq!(
            coordinator.preview(
                &drifted,
                request(&evidence),
                "preview-drift".to_owned(),
                10,
                20,
            ),
            Err(FrontierCoordinatorError::InvalidRequest)
        );
        let preview = coordinator
            .preview(
                &evidence,
                request(&evidence),
                "preview-expire".to_owned(),
                10,
                20,
            )
            .expect("preview");
        assert_eq!(
            coordinator.render_and_record(
                "preview-expire",
                SHA_B,
                19,
                false,
                "attempt-invalid".to_owned(),
                "receipt-invalid".to_owned(),
                false,
                None,
            ),
            Err(FrontierCoordinatorError::InvalidRequest)
        );
        assert_eq!(
            coordinator.render_and_record(
                "preview-expire",
                &preview.review.confirmation_sha256,
                19,
                false,
                "attempt-replay".to_owned(),
                "receipt-replay".to_owned(),
                false,
                None,
            ),
            Err(FrontierCoordinatorError::ReviewUnavailable)
        );
    }

    #[test]
    fn cancellation_and_every_delivery_class_remain_local_denials() {
        let evidence = tier_evidence();
        let mut coordinator = FrontierRecommendationCoordinator::new();
        coordinator
            .preview(
                &evidence,
                request(&evidence),
                "preview-cancel".to_owned(),
                10,
                20,
            )
            .expect("preview");
        let cancelled = coordinator
            .cancel("preview-cancel", "attempt-cancel".to_owned())
            .expect("local cancel");
        assert_eq!(cancelled.outcome, LocalHandoffOutcome::Cancelled);
        assert!(!cancelled.external_delivery_attempted);

        for (index, action) in HandoffProhibitedAction::ALL.into_iter().enumerate() {
            let denied = coordinator
                .deny_delivery(
                    format!("attempt-deny-{index}"),
                    Some("frontier-native-packet".to_owned()),
                    action,
                )
                .expect("local denial");
            assert_eq!(denied.outcome, LocalHandoffOutcome::Denied);
            assert_eq!(denied.prohibited_action, Some(action));
            assert!(!denied.external_delivery_attempted);
        }
    }
}
