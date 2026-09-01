//! Durable manual-frontier composition across recommendation, export, and local import.

use agentmage_kernel_contracts::{
    CapabilityGrant, FrontierTierEvidence, HandoffProhibitedAction, LocalHandoffReceipt,
};
use agentmage_kernel_engine::{
    filesystem_control::FilesystemPlan,
    frontier_import::{FrontierCurrentState, FrontierImportedArtifact},
    frontier_recommendation::FrontierPacketRequest,
    frontier_release::{
        FrontierPacketExportRequest, build_frontier_packet_export_plan,
        recovery::{
            DirectoryFrontierReleaseCheckpointStore, FrontierReleaseCheckpoint,
            FrontierReleasePhase, seal_frontier_release_checkpoint,
            verify_frontier_release_checkpoint,
        },
    },
    tooling::ToolRegistry,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    frontier_coordinator::{FrontierCoordinatorOutcome, FrontierRecommendationCoordinator},
    frontier_import_coordinator::{
        FrontierImportCheckpointPort, FrontierImportCoordinatorOutcome, FrontierStepRoute,
        coordinate_frontier_import,
    },
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure from the complete local manual-frontier transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierReleaseCoordinatorError {
    /// Measured recommendation, review, export, or import input failed closed.
    InvalidInput,
    /// Current packet, export, import, or policy evidence drifted from the durable transaction.
    StaleState,
    /// A prohibited external delivery surface could not produce a local denial receipt.
    DeliveryDenialFailure,
    /// A durable release checkpoint was unavailable, corrupt, forked, or could not be committed.
    CheckpointFailure,
}

impl FrontierReleaseCoordinatorError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "frontier.release-coordinator.input-invalid",
            Self::StaleState => "frontier.release-coordinator.state-stale",
            Self::DeliveryDenialFailure => "frontier.release-coordinator.delivery-denial-failed",
            Self::CheckpointFailure => "frontier.release-coordinator.checkpoint-failed",
        }
    }
}

impl std::fmt::Display for FrontierReleaseCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FrontierReleaseCoordinatorError {}

/// Every product-facing surface through which imported instructions might request delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierDeliverySurface {
    /// Native Chat population or submission.
    Chat,
    /// CLI launch, invocation, or submission.
    Cli,
    /// Model tool, connector, or workflow routing.
    ModelTools,
    /// Skill-triggered transfer.
    Skills,
    /// Scheduled or unattended transfer.
    Schedules,
    /// Delivery requested by imported or injected content.
    InjectedContent,
    /// Clipboard population.
    Clipboard,
    /// Visual Studio Code command or tab control.
    VisualStudioCode,
    /// Raw, local-runtime, or remote network path.
    Network,
}

impl FrontierDeliverySurface {
    /// Every closed Story 53 delivery-attack surface.
    pub const ALL: [Self; 9] = [
        Self::Chat,
        Self::Cli,
        Self::ModelTools,
        Self::Skills,
        Self::Schedules,
        Self::InjectedContent,
        Self::Clipboard,
        Self::VisualStudioCode,
        Self::Network,
    ];

    const fn prohibited_action(self) -> HandoffProhibitedAction {
        match self {
            Self::Chat => HandoffProhibitedAction::ChatPopulation,
            Self::Cli => HandoffProhibitedAction::CodexInvocation,
            Self::ModelTools | Self::Skills | Self::InjectedContent => {
                HandoffProhibitedAction::RoutedDelivery
            }
            Self::Schedules => HandoffProhibitedAction::ScheduledDelivery,
            Self::Clipboard => HandoffProhibitedAction::ClipboardWrite,
            Self::VisualStudioCode => HandoffProhibitedAction::TabActivation,
            Self::Network => HandoffProhibitedAction::NetworkCall,
        }
    }
}

/// Atomic persistence boundary implemented by the product's durable local store.
pub trait FrontierReleaseCheckpointPort {
    /// Loads the latest fully committed checkpoint for one transaction.
    fn load_latest(
        &mut self,
        transaction_id: &str,
    ) -> Result<Option<FrontierReleaseCheckpoint>, FrontierReleaseCoordinatorError>;

    /// Atomically commits one new checkpoint generation.
    fn commit(
        &mut self,
        checkpoint: &FrontierReleaseCheckpoint,
    ) -> Result<(), FrontierReleaseCoordinatorError>;
}

impl FrontierReleaseCheckpointPort for DirectoryFrontierReleaseCheckpointStore {
    fn load_latest(
        &mut self,
        transaction_id: &str,
    ) -> Result<Option<FrontierReleaseCheckpoint>, FrontierReleaseCoordinatorError> {
        DirectoryFrontierReleaseCheckpointStore::load_latest(self, transaction_id)
            .map_err(|_| FrontierReleaseCoordinatorError::CheckpointFailure)
    }

    fn commit(
        &mut self,
        checkpoint: &FrontierReleaseCheckpoint,
    ) -> Result<(), FrontierReleaseCoordinatorError> {
        DirectoryFrontierReleaseCheckpointStore::commit(self, checkpoint)
            .map_err(|_| FrontierReleaseCoordinatorError::CheckpointFailure)
    }
}

/// Exact local recommendation render and authority-free controlled export plan.
#[derive(Clone, Debug)]
pub struct FrontierPreparedExport {
    /// Exact local-only recommendation and rendered packet.
    pub recommendation: FrontierCoordinatorOutcome,
    /// Controlled filesystem plan; it carries no write grant or approval.
    pub export_plan: FilesystemPlan,
    /// Latest durable release checkpoint.
    pub checkpoint: FrontierReleaseCheckpoint,
}

/// Complete local round trip through import revalidation and native proposal admission.
#[derive(Clone, Debug)]
pub struct FrontierReleaseOutcome {
    /// Exact local recommendation render and controlled export plan.
    pub prepared: FrontierPreparedExport,
    /// Authority-free imported-result outcome with pending native-flow tickets.
    pub imported: FrontierImportCoordinatorOutcome,
    /// Terminal durable release checkpoint.
    pub checkpoint: FrontierReleaseCheckpoint,
}

/// Product coordinator with no external client, endpoint, clipboard, or credential capability.
#[derive(Default)]
pub struct FrontierReleaseCoordinator {
    recommendation: FrontierRecommendationCoordinator,
}

impl FrontierReleaseCoordinator {
    /// Creates an empty local-only release coordinator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            recommendation: FrontierRecommendationCoordinator::new(),
        }
    }

    /// Creates one exact reviewed packet and user-initiated controlled local export plan.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_export(
        &mut self,
        transaction_id: &str,
        evidence: &FrontierTierEvidence,
        packet_request: FrontierPacketRequest,
        preview_id: String,
        now_ms: u64,
        expires_at_ms: u64,
        confirmation_sha256: &str,
        non_public_acknowledged: bool,
        render_attempt_id: String,
        recommendation_receipt_id: String,
        export_parent: &CapabilityGrant,
        export_request: FrontierPacketExportRequest,
        checkpoints: &mut dyn FrontierReleaseCheckpointPort,
    ) -> Result<FrontierPreparedExport, FrontierReleaseCoordinatorError> {
        let mut latest = load_latest(checkpoints, transaction_id)?;
        let preview = self
            .recommendation
            .preview(
                evidence,
                packet_request,
                preview_id.clone(),
                now_ms,
                expires_at_ms,
            )
            .map_err(|_| FrontierReleaseCoordinatorError::InvalidInput)?;
        if preview.review.confirmation_sha256 != confirmation_sha256 {
            return Err(FrontierReleaseCoordinatorError::InvalidInput);
        }
        let recommendation = self
            .recommendation
            .render_and_record(
                &preview_id,
                confirmation_sha256,
                now_ms,
                non_public_acknowledged,
                render_attempt_id,
                recommendation_receipt_id,
                false,
                None,
            )
            .map_err(|_| FrontierReleaseCoordinatorError::InvalidInput)?;
        if recommendation.rendered.receipt.external_delivery_attempted
            || recommendation.recommendation.external_delivery_attempted
        {
            return Err(FrontierReleaseCoordinatorError::InvalidInput);
        }
        let packet_sha256 = recommendation.rendered.manifest.packet_sha256.clone();
        let recommendation_sha256 = digest(&recommendation.recommendation)?;
        require_binding(&latest, &packet_sha256, &recommendation_sha256, None, None)?;
        advance_checkpoint(
            checkpoints,
            &mut latest,
            transaction_id,
            FrontierReleasePhase::PacketRendered,
            &packet_sha256,
            &recommendation_sha256,
            None,
            None,
        )?;

        let export_plan = build_frontier_packet_export_plan(
            export_parent,
            &recommendation.rendered,
            export_request,
        )
        .map_err(|_| FrontierReleaseCoordinatorError::InvalidInput)?;
        let export_plan_sha256 = export_plan.plan_sha256().to_owned();
        require_binding(
            &latest,
            &packet_sha256,
            &recommendation_sha256,
            Some(&export_plan_sha256),
            None,
        )?;
        advance_checkpoint(
            checkpoints,
            &mut latest,
            transaction_id,
            FrontierReleasePhase::ExportPlanned,
            &packet_sha256,
            &recommendation_sha256,
            Some(export_plan_sha256),
            None,
        )?;
        let checkpoint = latest.ok_or(FrontierReleaseCoordinatorError::CheckpointFailure)?;
        Ok(FrontierPreparedExport {
            recommendation,
            export_plan,
            checkpoint,
        })
    }

    /// Imports caller-supplied returned bytes and routes proposals through current native gates.
    #[allow(clippy::too_many_arguments)]
    pub fn complete_import(
        &mut self,
        transaction_id: &str,
        prepared: FrontierPreparedExport,
        import_transaction_id: &str,
        import_receipt_id: String,
        manifest_bytes: &[u8],
        artifacts: &[FrontierImportedArtifact],
        current: &FrontierCurrentState,
        routes: &[FrontierStepRoute<'_>],
        registry: &ToolRegistry,
        capability_feedback: Vec<String>,
        release_checkpoints: &mut dyn FrontierReleaseCheckpointPort,
        import_checkpoints: &mut dyn FrontierImportCheckpointPort,
    ) -> Result<FrontierReleaseOutcome, FrontierReleaseCoordinatorError> {
        let mut latest = load_latest(release_checkpoints, transaction_id)?;
        let packet_sha256 = prepared
            .recommendation
            .rendered
            .manifest
            .packet_sha256
            .clone();
        let recommendation_sha256 = digest(&prepared.recommendation.recommendation)?;
        let export_plan_sha256 = prepared.export_plan.plan_sha256().to_owned();
        require_binding(
            &latest,
            &packet_sha256,
            &recommendation_sha256,
            Some(&export_plan_sha256),
            None,
        )?;
        if latest
            .as_ref()
            .is_none_or(|checkpoint| checkpoint.phase < FrontierReleasePhase::ExportPlanned)
        {
            return Err(FrontierReleaseCoordinatorError::StaleState);
        }
        let imported = coordinate_frontier_import(
            import_transaction_id,
            import_receipt_id,
            manifest_bytes,
            artifacts,
            current,
            routes,
            registry,
            None,
            capability_feedback,
            import_checkpoints,
        )
        .map_err(|_| FrontierReleaseCoordinatorError::InvalidInput)?;
        let import_receipt_sha256 = imported.receipt.receipt_sha256.clone();
        require_binding(
            &latest,
            &packet_sha256,
            &recommendation_sha256,
            Some(&export_plan_sha256),
            Some(&import_receipt_sha256),
        )?;
        advance_checkpoint(
            release_checkpoints,
            &mut latest,
            transaction_id,
            FrontierReleasePhase::ImportCompleted,
            &packet_sha256,
            &recommendation_sha256,
            Some(export_plan_sha256),
            Some(import_receipt_sha256),
        )?;
        let checkpoint = latest.ok_or(FrontierReleaseCoordinatorError::CheckpointFailure)?;
        Ok(FrontierReleaseOutcome {
            prepared,
            imported,
            checkpoint,
        })
    }

    /// Converts a delivery attempt on any named product surface into a local denial receipt.
    pub fn deny_delivery_surface(
        &self,
        surface: FrontierDeliverySurface,
        attempt_id: String,
        packet_id: Option<String>,
    ) -> Result<LocalHandoffReceipt, FrontierReleaseCoordinatorError> {
        self.recommendation
            .deny_delivery(attempt_id, packet_id, surface.prohibited_action())
            .map_err(|_| FrontierReleaseCoordinatorError::DeliveryDenialFailure)
    }
}

fn load_latest(
    checkpoints: &mut dyn FrontierReleaseCheckpointPort,
    transaction_id: &str,
) -> Result<Option<FrontierReleaseCheckpoint>, FrontierReleaseCoordinatorError> {
    let latest = checkpoints.load_latest(transaction_id)?;
    if let Some(checkpoint) = &latest {
        verify_frontier_release_checkpoint(checkpoint)
            .map_err(|_| FrontierReleaseCoordinatorError::CheckpointFailure)?;
    }
    Ok(latest)
}

#[allow(clippy::too_many_arguments)]
fn advance_checkpoint(
    checkpoints: &mut dyn FrontierReleaseCheckpointPort,
    latest: &mut Option<FrontierReleaseCheckpoint>,
    transaction_id: &str,
    phase: FrontierReleasePhase,
    packet_sha256: &str,
    recommendation_sha256: &str,
    export_plan_sha256: Option<String>,
    import_receipt_sha256: Option<String>,
) -> Result<(), FrontierReleaseCoordinatorError> {
    if latest
        .as_ref()
        .is_some_and(|checkpoint| checkpoint.phase >= phase)
    {
        return Ok(());
    }
    let generation = latest
        .as_ref()
        .map_or(1, |checkpoint| checkpoint.generation.saturating_add(1));
    let checkpoint = seal_frontier_release_checkpoint(FrontierReleaseCheckpoint {
        transaction_id: transaction_id.to_owned(),
        generation,
        phase,
        packet_sha256: packet_sha256.to_owned(),
        recommendation_sha256: recommendation_sha256.to_owned(),
        export_plan_sha256,
        import_receipt_sha256,
        previous_checkpoint_sha256: latest
            .as_ref()
            .map(|checkpoint| checkpoint.checkpoint_sha256.clone()),
        external_delivery: false,
        execution_authority: false,
        applied_effect_count: 0,
        checkpoint_sha256: ZERO_SHA256.to_owned(),
    })
    .map_err(|_| FrontierReleaseCoordinatorError::CheckpointFailure)?;
    checkpoints.commit(&checkpoint)?;
    *latest = Some(checkpoint);
    Ok(())
}

fn require_binding(
    latest: &Option<FrontierReleaseCheckpoint>,
    packet_sha256: &str,
    recommendation_sha256: &str,
    export_plan_sha256: Option<&str>,
    import_receipt_sha256: Option<&str>,
) -> Result<(), FrontierReleaseCoordinatorError> {
    let Some(checkpoint) = latest else {
        return Ok(());
    };
    if checkpoint.packet_sha256 != packet_sha256
        || checkpoint.recommendation_sha256 != recommendation_sha256
        || matches!(
            (checkpoint.export_plan_sha256.as_deref(), export_plan_sha256),
            (Some(expected), Some(actual)) if expected != actual
        )
        || matches!(
            (
                checkpoint.import_receipt_sha256.as_deref(),
                import_receipt_sha256,
            ),
            (Some(expected), Some(actual)) if expected != actual
        )
    {
        return Err(FrontierReleaseCoordinatorError::StaleState);
    }
    Ok(())
}

fn digest<T: Serialize>(value: &T) -> Result<String, FrontierReleaseCoordinatorError> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| FrontierReleaseCoordinatorError::InvalidInput)?;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a string cannot fail");
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use agentmage_kernel_contracts::{
        AuthorityClass, BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION, CapabilityGrant,
        DataSensitivity, EvidenceKind, FrontierAcceptanceState, FrontierClarificationClass,
        FrontierReturnInput, FrontierReturnKind, FrontierReturnManifest, FrontierReturnedStep,
        FrontierReturnedStepKind, FrontierTaskTier, GrantTarget, HandoffDisclosureEntry,
        HandoffEntryDisposition, HandoffEntryKind, HandoffSensitivity, LocalHandoffOutcome, PlanId,
        RollbackPlan, StopCondition, StopConditionKind, TaskId, WorkPacket, WorkPacketId,
        WorkPacketState, WorkspaceId, WorkspacePath,
    };
    use agentmage_kernel_engine::{
        filesystem_control::NewDestinationDraft,
        frontier_import::{
            recovery::{FrontierImportCheckpoint, verify_frontier_import_checkpoint},
            seal_frontier_return_manifest,
        },
        frontier_recommendation::{
            FrontierPacketEvidence, FrontierPacketEvidenceRole, build_frontier_packet_preview,
            decide_frontier_tier,
        },
        frontier_release::recovery::DirectoryFrontierReleaseCheckpointStore,
        handoff::seal_handoff_entry,
        task_classification::TaskIntent,
    };
    use serde_json::json;

    use super::*;
    use crate::frontier_import_coordinator::FrontierImportCoordinatorError;

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[derive(Clone, Default)]
    struct RecordingReleaseCheckpoints {
        values: Vec<FrontierReleaseCheckpoint>,
        fail_after: Option<FrontierReleasePhase>,
    }

    impl FrontierReleaseCheckpointPort for RecordingReleaseCheckpoints {
        fn load_latest(
            &mut self,
            _transaction_id: &str,
        ) -> Result<Option<FrontierReleaseCheckpoint>, FrontierReleaseCoordinatorError> {
            Ok(self.values.last().cloned())
        }

        fn commit(
            &mut self,
            checkpoint: &FrontierReleaseCheckpoint,
        ) -> Result<(), FrontierReleaseCoordinatorError> {
            verify_frontier_release_checkpoint(checkpoint)
                .map_err(|_| FrontierReleaseCoordinatorError::CheckpointFailure)?;
            self.values.push(checkpoint.clone());
            if self.fail_after == Some(checkpoint.phase) {
                return Err(FrontierReleaseCoordinatorError::CheckpointFailure);
            }
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct RecordingImportCheckpoints {
        values: Vec<FrontierImportCheckpoint>,
    }

    impl FrontierImportCheckpointPort for RecordingImportCheckpoints {
        fn load_latest(
            &mut self,
            _transaction_id: &str,
        ) -> Result<Option<FrontierImportCheckpoint>, FrontierImportCoordinatorError> {
            Ok(self.values.last().cloned())
        }

        fn commit(
            &mut self,
            checkpoint: &FrontierImportCheckpoint,
        ) -> Result<(), FrontierImportCoordinatorError> {
            verify_frontier_import_checkpoint(checkpoint)
                .map_err(|_| FrontierImportCoordinatorError::CheckpointFailure)?;
            self.values.push(checkpoint.clone());
            Ok(())
        }
    }

    fn tier_evidence() -> FrontierTierEvidence {
        FrontierTierEvidence {
            schema_version: CONTRACT_SCHEMA_VERSION,
            decision_id: "frontier-release-decision".to_owned(),
            deterministic_available: false,
            deterministic_succeeded: false,
            local_model_attempted: true,
            local_acceptance_check_id: "check.release-result".to_owned(),
            local_acceptance_state: FrontierAcceptanceState::Failed,
            validation_failure_count: 1,
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
            display_source: "src/release.rs".to_owned(),
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

    fn packet_request(evidence: &FrontierTierEvidence) -> FrontierPacketRequest {
        FrontierPacketRequest {
            tier_evidence: evidence.clone(),
            decision: decide_frontier_tier(evidence).expect("measured recommendation"),
            packet_id: "frontier-release-packet".to_owned(),
            workspace_state_sha256: SHA_A.to_owned(),
            policy_sha256: SHA_A.to_owned(),
            redaction_policy_sha256: SHA_A.to_owned(),
            objective: "Review the failed local acceptance check".to_owned(),
            acceptance_checks: vec!["Return one locally reviewable decision".to_owned()],
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
            required_output_contract: "One JSON decision with citations".to_owned(),
        }
    }

    fn parent() -> CapabilityGrant {
        serde_json::from_value(json!({
            "schema_version": 2,
            "grant_id": "grant-frontier-release-parent",
            "revision": 0,
            "grant_class": "session_read",
            "actor_id": "actor-local",
            "approval_id": null,
            "session_id": "session-frontier-release",
            "task_id": "task-frontier-release",
            "action_id": null,
            "action_kind": null,
            "operation": {"taxonomy_version": 1, "operation": "workspace_read", "authority_class": "observe"},
            "tool_id": null,
            "tool_version": null,
            "targets": [{
                "target_kind": "workspace_scope",
                "path": {"workspace_id": "workspace-frontier-release", "components": []},
                "authorization_id": "authorization-frontier-release",
                "adapter_instance_id": "adapter-frontier-release",
                "platform": "deterministic_fake"
            }],
            "excluded_targets": [],
            "sensitivity": "restricted",
            "argument_sha256": "1".repeat(64),
            "preimages": [],
            "expected_side_effects": [],
            "rollback_description": "No read effect",
            "issued_at_epoch_ms": 1000,
            "expires_at_epoch_ms": 100000,
            "nonce": "nonce-frontier-release-parent",
            "use_limit": 10,
            "use_count": 0,
            "parent_grant_id": null,
            "parent_grant_sha256": null,
            "preview_sha256": "2".repeat(64),
            "policy_sha256": "3".repeat(64),
            "status": "issued"
        }))
        .expect("parent grant")
    }

    fn export_request() -> FrontierPacketExportRequest {
        let identity: [u8; 32] = Sha256::digest(b"exports").into();
        let parent: GrantTarget = serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-frontier-release", "components": ["exports"]},
            "authorization_id": "authorization-frontier-release",
            "adapter_instance_id": "adapter-frontier-release",
            "platform": "deterministic_fake",
            "object_kind": "directory",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": identity
            },
            "preimage": null
        }))
        .expect("destination parent");
        FrontierPacketExportRequest {
            plan_id: "frontier-release-export-plan".to_owned(),
            operation_id: "frontier-release-export-create".to_owned(),
            observed_at_epoch_ms: 2_000,
            destination: NewDestinationDraft {
                parent,
                path: WorkspacePath::new(
                    WorkspaceId::from_raw("workspace-frontier-release"),
                    ["exports", "frontier-packet.md"],
                )
                .expect("destination path"),
                observed_sibling_names: Vec::new(),
            },
            user_initiated: true,
        }
    }

    fn prepared(
        coordinator: &mut FrontierReleaseCoordinator,
        checkpoints: &mut dyn FrontierReleaseCheckpointPort,
    ) -> Result<FrontierPreparedExport, FrontierReleaseCoordinatorError> {
        let evidence = tier_evidence();
        let request = packet_request(&evidence);
        let preview =
            build_frontier_packet_preview(&request, "frontier-release-preview".to_owned(), 3_000)
                .expect("preview identity");
        coordinator.prepare_export(
            "frontier-release-transaction",
            &evidence,
            request,
            "frontier-release-preview".to_owned(),
            2_000,
            3_000,
            &preview.review.confirmation_sha256,
            false,
            "frontier-release-render-attempt".to_owned(),
            "frontier-release-recommendation-receipt".to_owned(),
            &parent(),
            export_request(),
            checkpoints,
        )
    }

    fn import_manifest() -> Vec<u8> {
        let mut manifest = FrontierReturnManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            import_id: "frontier-release-import".to_owned(),
            request_packet_sha256: SHA_A.to_owned(),
            base_workspace_state_sha256: SHA_A.to_owned(),
            base_model_state_sha256: SHA_A.to_owned(),
            base_policy_sha256: SHA_A.to_owned(),
            tier: FrontierTaskTier::FrontierRecommended,
            result_kind: FrontierReturnKind::Decision,
            rationale: "Return one untrusted decision for local review only".to_owned(),
            inputs: vec![FrontierReturnInput {
                input_id: "input-release-packet".to_owned(),
                input_sha256: SHA_A.to_owned(),
            }],
            artifacts: Vec::new(),
            citations: Vec::new(),
            steps: vec![FrontierReturnedStep {
                step_id: "step-release-decision".to_owned(),
                kind: FrontierReturnedStepKind::Decision,
                rationale: "Review this decision under current local policy".to_owned(),
                artifact_ids: Vec::new(),
                citation_ids: Vec::new(),
                acceptance_checks: vec!["Review returned decision locally".to_owned()],
                proposed_operation: None,
                approval_requirements: Vec::new(),
            }],
            acceptance_checks: vec!["Review returned decision locally".to_owned()],
            approval_requirements: Vec::new(),
            remaining_steps: vec!["Choose whether to act through a fresh local flow".to_owned()],
            external_content_untrusted: true,
            authority_granted: false,
            completion_credit: false,
            outbound_network_required: false,
            manifest_sha256: String::new(),
        };
        seal_frontier_return_manifest(&mut manifest).expect("seal import manifest");
        serde_json::to_vec(&manifest).expect("manifest JSON")
    }

    fn current() -> FrontierCurrentState {
        FrontierCurrentState {
            request_packet_sha256: SHA_A.to_owned(),
            workspace_state_sha256: SHA_A.to_owned(),
            model_state_sha256: SHA_A.to_owned(),
            policy_sha256: SHA_A.to_owned(),
            permissions_sha256: SHA_A.to_owned(),
            permissions_checked: true,
            citations: Vec::new(),
        }
    }

    fn route_packet() -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("frontier-release-work-packet"),
            task_id: TaskId::from_raw("frontier-release-local-review"),
            revision: 1,
            objective: "Review one imported decision locally".to_owned(),
            reason: "Imported decisions carry no authority".to_owned(),
            owner: "local-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: Vec::new(),
            expected_output: "One authority-free review ticket".to_owned(),
            acceptance_checks: vec!["Review returned decision locally".to_owned()],
            required_evidence: vec![EvidenceKind::Decision],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![BudgetLimit {
                resource: BudgetResource::PlanSteps,
                limit: 1,
            }],
            stop_conditions: [
                StopConditionKind::AcceptanceSatisfied,
                StopConditionKind::UserDecisionRequired,
                StopConditionKind::PolicyDenied,
                StopConditionKind::Error,
                StopConditionKind::Cancelled,
                StopConditionKind::BudgetExhausted,
                StopConditionKind::UncertainResult,
            ]
            .into_iter()
            .map(|kind| StopCondition {
                kind,
                description: format!("Stop at {kind:?}"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No imported operation is applied".to_owned(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-09-01".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw("frontier-release-plan")),
            state: WorkPacketState::Active,
        }
    }

    fn complete(
        coordinator: &mut FrontierReleaseCoordinator,
        prepared: FrontierPreparedExport,
        release_checkpoints: &mut dyn FrontierReleaseCheckpointPort,
        import_checkpoints: &mut dyn FrontierImportCheckpointPort,
    ) -> Result<FrontierReleaseOutcome, FrontierReleaseCoordinatorError> {
        let packet = route_packet();
        let routes = [FrontierStepRoute {
            step_id: "step-release-decision",
            work_packet: &packet,
            intent: TaskIntent::Review,
            tool_call: None,
        }];
        coordinator.complete_import(
            "frontier-release-transaction",
            prepared,
            "frontier-release-import-transaction",
            "frontier-release-import-receipt".to_owned(),
            &import_manifest(),
            &[],
            &current(),
            &routes,
            &ToolRegistry::new(),
            vec!["frontier.feedback.manual-round-trip".to_owned()],
            release_checkpoints,
            import_checkpoints,
        )
    }

    #[test]
    fn story_53_manual_round_trip_remains_local_authority_free_and_optional() {
        let mut coordinator = FrontierReleaseCoordinator::new();
        let mut release_checkpoints = RecordingReleaseCheckpoints::default();
        let prepared =
            prepared(&mut coordinator, &mut release_checkpoints).expect("prepare export");
        assert_eq!(
            prepared.checkpoint.phase,
            FrontierReleasePhase::ExportPlanned
        );
        assert_eq!(prepared.export_plan.operations().len(), 1);
        assert!(!prepared.recommendation.rendered.manifest.delivered);

        let mut import_checkpoints = RecordingImportCheckpoints::default();
        let outcome = complete(
            &mut coordinator,
            prepared,
            &mut release_checkpoints,
            &mut import_checkpoints,
        )
        .expect("complete local round trip");
        assert_eq!(
            outcome.checkpoint.phase,
            FrontierReleasePhase::ImportCompleted
        );
        assert_eq!(outcome.imported.tickets.len(), 1);
        assert!(!outcome.imported.tickets[0].authority_granted);
        assert_eq!(outcome.imported.tickets[0].applied_effect_count, 0);
        assert_eq!(outcome.imported.receipt.applied_effect_count, 0);
        assert_eq!(outcome.imported.receipt.duplicate_effect_count, 0);
        assert_eq!(release_checkpoints.values.len(), 3);
        assert_eq!(import_checkpoints.values.len(), 4);
    }

    #[test]
    fn story_53_every_release_phase_recovers_without_delivery_or_duplicate_effect() {
        for phase in [
            FrontierReleasePhase::PacketRendered,
            FrontierReleasePhase::ExportPlanned,
            FrontierReleasePhase::ImportCompleted,
        ] {
            let mut coordinator = FrontierReleaseCoordinator::new();
            let mut release_checkpoints = RecordingReleaseCheckpoints {
                fail_after: Some(phase),
                ..RecordingReleaseCheckpoints::default()
            };
            let mut import_checkpoints = RecordingImportCheckpoints::default();
            let first = prepared(&mut coordinator, &mut release_checkpoints);
            let prepared = if phase == FrontierReleasePhase::PacketRendered
                || phase == FrontierReleasePhase::ExportPlanned
            {
                assert!(matches!(
                    first,
                    Err(FrontierReleaseCoordinatorError::CheckpointFailure)
                ));
                release_checkpoints.fail_after = None;
                prepared(&mut coordinator, &mut release_checkpoints)
                    .unwrap_or_else(|error| panic!("resume export after {phase:?}: {error:?}"))
            } else {
                first.expect("prepare before import interruption")
            };
            if phase == FrontierReleasePhase::ImportCompleted {
                assert!(matches!(
                    complete(
                        &mut coordinator,
                        prepared.clone(),
                        &mut release_checkpoints,
                        &mut import_checkpoints,
                    ),
                    Err(FrontierReleaseCoordinatorError::CheckpointFailure)
                ));
                release_checkpoints.fail_after = None;
            }
            let outcome = complete(
                &mut coordinator,
                prepared,
                &mut release_checkpoints,
                &mut import_checkpoints,
            )
            .expect("resume complete round trip");
            assert_eq!(outcome.imported.receipt.applied_effect_count, 0);
            assert_eq!(outcome.imported.receipt.duplicate_effect_count, 0);
            assert!(!outcome.checkpoint.external_delivery);
        }
    }

    #[test]
    fn story_53_all_delivery_surfaces_fail_to_local_denial_receipts() {
        let coordinator = FrontierReleaseCoordinator::new();
        for (index, surface) in FrontierDeliverySurface::ALL.into_iter().enumerate() {
            let receipt = coordinator
                .deny_delivery_surface(
                    surface,
                    format!("frontier-release-denial-{index}"),
                    Some("frontier-release-packet".to_owned()),
                )
                .expect("local denial");
            assert_eq!(receipt.outcome, LocalHandoffOutcome::Denied);
            assert!(!receipt.external_delivery_attempted);
        }
    }

    #[test]
    fn story_53_directory_reopen_rejects_tampering_and_preserves_exact_phase() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agentmage-frontier-release-{}-{unique}",
            std::process::id()
        ));
        let mut coordinator = FrontierReleaseCoordinator::new();
        let mut store = DirectoryFrontierReleaseCheckpointStore::open(&root).expect("open store");
        let prepared = prepared(&mut coordinator, &mut store).expect("durable export");
        drop(store);
        let reopened = DirectoryFrontierReleaseCheckpointStore::open(&root).expect("reopen store");
        assert_eq!(
            reopened
                .load_latest("frontier-release-transaction")
                .expect("load chain")
                .expect("checkpoint"),
            prepared.checkpoint
        );
        let terminal = std::fs::read_dir(&root)
            .expect("read directory")
            .map(|entry| entry.expect("entry").path())
            .max()
            .expect("checkpoint file");
        let mut bytes = std::fs::read(&terminal).expect("checkpoint bytes");
        *bytes.last_mut().expect("non-empty checkpoint") ^= 1;
        std::fs::write(&terminal, bytes).expect("tamper fixture");
        assert!(
            reopened
                .load_latest("frontier-release-transaction")
                .is_err()
        );
        std::fs::remove_dir_all(root).expect("remove fixture directory");
    }
}
