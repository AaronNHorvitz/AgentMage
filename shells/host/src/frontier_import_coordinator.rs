//! Durable, authority-free host coordination for imported frontier proposals.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    FrontierImportDisposition, FrontierLocalFlowRequirements, FrontierReturnManifest,
    FrontierReturnedStepKind, FrontierRoundTripReceipt, ToolCall, WorkPacket,
};
use agentmage_kernel_engine::{
    frontier_import::{
        FrontierCurrentState, FrontierImportReport, FrontierImportedArtifact,
        build_frontier_round_trip_receipt, parse_frontier_return_manifest,
        revalidate_frontier_import, verify_frontier_round_trip_receipt,
    },
    frontier_import_recovery::{
        DirectoryFrontierImportCheckpointStore, FrontierImportCheckpoint, FrontierImportPhase,
        seal_frontier_import_checkpoint, verify_frontier_import_checkpoint,
    },
    task_classification::{TaskIntent, classify_task},
    tooling::{PreGrantDispatchDisposition, ProposalOrigin, ToolDispatcher, ToolRegistry},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_ROUTED_STEPS: usize = 64;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure from the frontier-import product transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierImportCoordinatorError {
    /// Imported or locally supplied material failed a closed contract.
    InvalidInput,
    /// The current repository, model, policy, permission, or request state changed.
    StaleState,
    /// An eligible step was not mapped exactly once into its required native flow.
    IncompleteRouting,
    /// A proposed tool did not pass the current native registry without authority.
    ToolDenied,
    /// A durable checkpoint was unavailable, corrupt, forked, or could not be committed.
    CheckpointFailure,
}

impl FrontierImportCoordinatorError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "frontier.import-coordinator.input-invalid",
            Self::StaleState => "frontier.import-coordinator.state-stale",
            Self::IncompleteRouting => "frontier.import-coordinator.routing-incomplete",
            Self::ToolDenied => "frontier.import-coordinator.tool-denied",
            Self::CheckpointFailure => "frontier.import-coordinator.checkpoint-failed",
        }
    }
}

impl std::fmt::Display for FrontierImportCoordinatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FrontierImportCoordinatorError {}

/// One trusted local proposal binding supplied independently of imported bytes.
pub struct FrontierStepRoute<'a> {
    /// Exact imported step identity.
    pub step_id: &'a str,
    /// Current native work packet used for deterministic classification.
    pub work_packet: &'a WorkPacket,
    /// Explicit current user intent used by the normal classification contract.
    pub intent: TaskIntent,
    /// Fresh locally constructed tool call for command, tool, or file proposals.
    pub tool_call: Option<&'a ToolCall>,
}

/// Closed native flow selected after current local revalidation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierNativeFlow {
    /// Human review of a decision or inert link.
    Review,
    /// Evidence-state assignment for a material claim.
    Evidence,
    /// Existing controlled-write preview and approval flow.
    ControlledWrite,
    /// Existing registered command flow.
    Command,
    /// Existing registered native tool flow.
    Tool,
    /// Existing trusted validation selection and sandbox flow.
    Validation,
}

/// Content-free ticket proving one proposal entered a native flow without authority or effects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierLocalFlowTicket {
    /// Exact imported step identity.
    pub step_id: String,
    /// Exact current work-packet digest.
    pub work_packet_sha256: String,
    /// Deterministic current task-classification digest.
    pub task_classification_sha256: String,
    /// Selected existing native flow.
    pub flow: FrontierNativeFlow,
    /// Optional exact fresh local tool-call digest.
    pub tool_call_sha256: Option<String>,
    /// Requirements inherited from the import report.
    pub requirements: FrontierLocalFlowRequirements,
    /// Fixed true when a future operation must obtain a fresh grant.
    pub fresh_grant_pending: bool,
    /// Fixed true when a file proposal still needs a current exact-preimage preview.
    pub exact_preimage_pending: bool,
    /// Fixed true when validation must run locally in the normal trusted sandbox.
    pub trusted_validation_pending: bool,
    /// Fixed true when local evidence assignment remains required.
    pub evidence_assignment_pending: bool,
    /// Fixed true when explicit user approval remains required.
    pub user_approval_pending: bool,
    /// Fixed false: a routing ticket is never authority.
    pub authority_granted: bool,
    /// Fixed zero: routing performs no effect.
    pub applied_effect_count: u32,
    /// Digest of this ticket with this field zeroed.
    pub ticket_sha256: String,
}

/// Atomic persistence boundary implemented by the product's durable local store.
pub trait FrontierImportCheckpointPort {
    /// Loads the latest fully committed checkpoint for one transaction.
    fn load_latest(
        &mut self,
        transaction_id: &str,
    ) -> Result<Option<FrontierImportCheckpoint>, FrontierImportCoordinatorError>;

    /// Atomically commits one new checkpoint generation.
    fn commit(
        &mut self,
        checkpoint: &FrontierImportCheckpoint,
    ) -> Result<(), FrontierImportCoordinatorError>;
}

impl FrontierImportCheckpointPort for DirectoryFrontierImportCheckpointStore {
    fn load_latest(
        &mut self,
        transaction_id: &str,
    ) -> Result<Option<FrontierImportCheckpoint>, FrontierImportCoordinatorError> {
        DirectoryFrontierImportCheckpointStore::load_latest(self, transaction_id)
            .map_err(|_| FrontierImportCoordinatorError::CheckpointFailure)
    }

    fn commit(
        &mut self,
        checkpoint: &FrontierImportCheckpoint,
    ) -> Result<(), FrontierImportCoordinatorError> {
        DirectoryFrontierImportCheckpointStore::commit(self, checkpoint)
            .map_err(|_| FrontierImportCoordinatorError::CheckpointFailure)
    }
}

/// Complete no-effect outcome of one imported-result transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontierImportCoordinatorOutcome {
    /// Pure local revalidation report.
    pub report: FrontierImportReport,
    /// Native-flow tickets; none grants or applies anything.
    pub tickets: Vec<FrontierLocalFlowTicket>,
    /// Terminal no-effect round-trip receipt.
    pub receipt: FrontierRoundTripReceipt,
    /// Final durable checkpoint.
    pub checkpoint: FrontierImportCheckpoint,
}

/// Executes one restartable import transaction through existing local admission contracts.
#[allow(clippy::too_many_arguments)]
pub fn coordinate_frontier_import(
    transaction_id: &str,
    receipt_id: String,
    manifest_bytes: &[u8],
    artifacts: &[FrontierImportedArtifact],
    current: &FrontierCurrentState,
    routes: &[FrontierStepRoute<'_>],
    registry: &ToolRegistry,
    re_escalation_reason: Option<String>,
    capability_feedback: Vec<String>,
    checkpoints: &mut dyn FrontierImportCheckpointPort,
) -> Result<FrontierImportCoordinatorOutcome, FrontierImportCoordinatorError> {
    if !valid_identifier(transaction_id) || routes.len() > MAX_ROUTED_STEPS {
        return Err(FrontierImportCoordinatorError::InvalidInput);
    }
    let mut latest = checkpoints.load_latest(transaction_id)?;
    if let Some(checkpoint) = &latest {
        verify_checkpoint(checkpoint)?;
    }

    let manifest = parse_frontier_return_manifest(manifest_bytes, &current.request_packet_sha256)
        .map_err(|error| match error.code() {
        "frontier.import.request-mismatch" => FrontierImportCoordinatorError::StaleState,
        _ => FrontierImportCoordinatorError::InvalidInput,
    })?;
    require_resume_binding(&latest, &manifest, current, None, None)?;
    advance_checkpoint(
        checkpoints,
        &mut latest,
        transaction_id,
        FrontierImportPhase::Parsed,
        &manifest,
        None,
        None,
        None,
    )?;

    let report = revalidate_frontier_import(&manifest, artifacts, current)
        .map_err(|_| FrontierImportCoordinatorError::InvalidInput)?;
    if !report.base_state_current {
        return Err(FrontierImportCoordinatorError::StaleState);
    }
    require_resume_binding(&latest, &manifest, current, Some(&report), None)?;
    advance_checkpoint(
        checkpoints,
        &mut latest,
        transaction_id,
        FrontierImportPhase::Revalidated,
        &manifest,
        Some(&report),
        None,
        None,
    )?;

    let tickets = route_steps(&manifest, &report, routes, registry)?;
    let tickets_sha256 = digest(&("agentmage-frontier-local-flow-tickets-v1", &tickets))?;
    require_resume_binding(
        &latest,
        &manifest,
        current,
        Some(&report),
        Some(&tickets_sha256),
    )?;
    advance_checkpoint(
        checkpoints,
        &mut latest,
        transaction_id,
        FrontierImportPhase::Routed,
        &manifest,
        Some(&report),
        Some(tickets_sha256.clone()),
        None,
    )?;

    let receipt = build_frontier_round_trip_receipt(
        receipt_id,
        current.request_packet_sha256.clone(),
        &report,
        re_escalation_reason,
        capability_feedback,
    )
    .map_err(|_| FrontierImportCoordinatorError::InvalidInput)?;
    verify_frontier_round_trip_receipt(&receipt, &report)
        .map_err(|_| FrontierImportCoordinatorError::InvalidInput)?;
    if let Some(checkpoint) = latest
        .as_ref()
        .filter(|checkpoint| checkpoint.phase == FrontierImportPhase::Completed)
    {
        if checkpoint.receipt_sha256.as_deref() != Some(&receipt.receipt_sha256) {
            return Err(FrontierImportCoordinatorError::StaleState);
        }
        return Ok(FrontierImportCoordinatorOutcome {
            report,
            tickets,
            receipt,
            checkpoint: checkpoint.clone(),
        });
    }
    advance_checkpoint(
        checkpoints,
        &mut latest,
        transaction_id,
        FrontierImportPhase::Completed,
        &manifest,
        Some(&report),
        Some(tickets_sha256),
        Some(receipt.receipt_sha256.clone()),
    )?;
    let checkpoint = latest.ok_or(FrontierImportCoordinatorError::CheckpointFailure)?;
    Ok(FrontierImportCoordinatorOutcome {
        report,
        tickets,
        receipt,
        checkpoint,
    })
}

fn route_steps(
    manifest: &FrontierReturnManifest,
    report: &FrontierImportReport,
    routes: &[FrontierStepRoute<'_>],
    registry: &ToolRegistry,
) -> Result<Vec<FrontierLocalFlowTicket>, FrontierImportCoordinatorError> {
    let eligible = report
        .step_outcomes
        .iter()
        .filter(|outcome| outcome.disposition == FrontierImportDisposition::ProposalEligible)
        .map(|outcome| outcome.step_id.as_str())
        .collect::<BTreeSet<_>>();
    let route_ids = routes
        .iter()
        .map(|route| route.step_id)
        .collect::<BTreeSet<_>>();
    if route_ids.len() != routes.len() || route_ids != eligible {
        return Err(FrontierImportCoordinatorError::IncompleteRouting);
    }
    let steps = manifest
        .steps
        .iter()
        .map(|step| (step.step_id.as_str(), step))
        .collect::<BTreeMap<_, _>>();
    let outcomes = report
        .step_outcomes
        .iter()
        .map(|outcome| (outcome.step_id.as_str(), outcome))
        .collect::<BTreeMap<_, _>>();
    let mut tickets = Vec::with_capacity(routes.len());
    for route in routes {
        let step = steps
            .get(route.step_id)
            .ok_or(FrontierImportCoordinatorError::IncompleteRouting)?;
        let outcome = outcomes
            .get(route.step_id)
            .ok_or(FrontierImportCoordinatorError::IncompleteRouting)?;
        if !step
            .acceptance_checks
            .iter()
            .all(|check| route.work_packet.acceptance_checks.contains(check))
        {
            return Err(FrontierImportCoordinatorError::InvalidInput);
        }
        let classification = classify_task(route.work_packet, route.intent)
            .map_err(|_| FrontierImportCoordinatorError::InvalidInput)?;
        let flow = flow_for(step.kind);
        let tool_call_sha256 = match (step.kind, route.tool_call) {
            (
                FrontierReturnedStepKind::FileProposal
                | FrontierReturnedStepKind::CommandProposal
                | FrontierReturnedStepKind::ToolProposal,
                Some(call),
            ) => {
                let operation = step
                    .proposed_operation
                    .ok_or(FrontierImportCoordinatorError::InvalidInput)?;
                if classification.required_authority_class()
                    != operation.operation().authority_class()
                {
                    return Err(FrontierImportCoordinatorError::InvalidInput);
                }
                let definition = registry
                    .validate_arguments(call)
                    .map_err(|_| FrontierImportCoordinatorError::ToolDenied)?;
                if definition.required_grant.operation != operation {
                    return Err(FrontierImportCoordinatorError::ToolDenied);
                }
                let dispatch = ToolDispatcher::new(registry).dispatch(ProposalOrigin::Model, call);
                if dispatch.disposition != PreGrantDispatchDisposition::GrantRequired {
                    return Err(FrontierImportCoordinatorError::ToolDenied);
                }
                Some(digest(&("agentmage-frontier-local-tool-call-v1", call))?)
            }
            (
                FrontierReturnedStepKind::FileProposal
                | FrontierReturnedStepKind::CommandProposal
                | FrontierReturnedStepKind::ToolProposal,
                None,
            ) => return Err(FrontierImportCoordinatorError::IncompleteRouting),
            (_, Some(_)) => return Err(FrontierImportCoordinatorError::InvalidInput),
            (_, None) => None,
        };
        let work_packet_sha256 = digest(&(
            "agentmage-frontier-current-work-packet-v1",
            route.work_packet,
        ))?;
        let mut ticket = FrontierLocalFlowTicket {
            step_id: route.step_id.to_owned(),
            work_packet_sha256,
            task_classification_sha256: classification.sha256().to_owned(),
            flow,
            tool_call_sha256,
            requirements: outcome.local_requirements.clone(),
            fresh_grant_pending: outcome.local_requirements.fresh_grant_required,
            exact_preimage_pending: outcome.local_requirements.exact_write_preview_required,
            trusted_validation_pending: outcome.local_requirements.trusted_validation_required,
            evidence_assignment_pending: outcome.local_requirements.evidence_assignment_required,
            user_approval_pending: outcome.local_requirements.user_approval_required,
            authority_granted: false,
            applied_effect_count: 0,
            ticket_sha256: ZERO_SHA256.to_owned(),
        };
        ticket.ticket_sha256 = ticket_digest(&ticket)?;
        verify_ticket(&ticket)?;
        tickets.push(ticket);
    }
    tickets.sort_by(|left, right| left.step_id.cmp(&right.step_id));
    Ok(tickets)
}

const fn flow_for(kind: FrontierReturnedStepKind) -> FrontierNativeFlow {
    match kind {
        FrontierReturnedStepKind::Decision | FrontierReturnedStepKind::LinkReference => {
            FrontierNativeFlow::Review
        }
        FrontierReturnedStepKind::Claim => FrontierNativeFlow::Evidence,
        FrontierReturnedStepKind::FileProposal => FrontierNativeFlow::ControlledWrite,
        FrontierReturnedStepKind::CommandProposal => FrontierNativeFlow::Command,
        FrontierReturnedStepKind::ToolProposal => FrontierNativeFlow::Tool,
        FrontierReturnedStepKind::TestResult => FrontierNativeFlow::Validation,
    }
}

fn verify_ticket(ticket: &FrontierLocalFlowTicket) -> Result<(), FrontierImportCoordinatorError> {
    let operation_flow = matches!(
        ticket.flow,
        FrontierNativeFlow::ControlledWrite
            | FrontierNativeFlow::Command
            | FrontierNativeFlow::Tool
    );
    if !valid_identifier(&ticket.step_id)
        || !valid_sha256(&ticket.work_packet_sha256)
        || !valid_sha256(&ticket.task_classification_sha256)
        || ticket
            .tool_call_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || operation_flow != ticket.tool_call_sha256.is_some()
        || ticket.authority_granted
        || ticket.applied_effect_count != 0
        || !valid_sha256(&ticket.ticket_sha256)
        || ticket.ticket_sha256 != ticket_digest(ticket)?
    {
        return Err(FrontierImportCoordinatorError::InvalidInput);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn advance_checkpoint(
    port: &mut dyn FrontierImportCheckpointPort,
    latest: &mut Option<FrontierImportCheckpoint>,
    transaction_id: &str,
    phase: FrontierImportPhase,
    manifest: &FrontierReturnManifest,
    report: Option<&FrontierImportReport>,
    tickets_sha256: Option<String>,
    receipt_sha256: Option<String>,
) -> Result<(), FrontierImportCoordinatorError> {
    if latest
        .as_ref()
        .is_some_and(|checkpoint| checkpoint.phase >= phase)
    {
        return Ok(());
    }
    let generation = latest
        .as_ref()
        .map_or(1, |checkpoint| checkpoint.generation + 1);
    let previous_checkpoint_sha256 = latest
        .as_ref()
        .map(|checkpoint| checkpoint.checkpoint_sha256.clone());
    let checkpoint = seal_frontier_import_checkpoint(FrontierImportCheckpoint {
        transaction_id: transaction_id.to_owned(),
        generation,
        phase,
        request_packet_sha256: manifest.request_packet_sha256.clone(),
        manifest_sha256: manifest.manifest_sha256.clone(),
        current_state_sha256: report.map(|value| value.current_state_sha256.clone()),
        report_sha256: report.map(|value| value.report_sha256.clone()),
        tickets_sha256,
        receipt_sha256,
        previous_checkpoint_sha256,
        execution_authority: false,
        applied_effect_count: 0,
        checkpoint_sha256: ZERO_SHA256.to_owned(),
    })
    .map_err(|_| FrontierImportCoordinatorError::CheckpointFailure)?;
    port.commit(&checkpoint)?;
    *latest = Some(checkpoint);
    Ok(())
}

fn require_resume_binding(
    latest: &Option<FrontierImportCheckpoint>,
    manifest: &FrontierReturnManifest,
    current: &FrontierCurrentState,
    report: Option<&FrontierImportReport>,
    tickets_sha256: Option<&str>,
) -> Result<(), FrontierImportCoordinatorError> {
    let Some(checkpoint) = latest else {
        return Ok(());
    };
    let matches = checkpoint.request_packet_sha256 == current.request_packet_sha256
        && checkpoint.manifest_sha256 == manifest.manifest_sha256
        && report.is_none_or(|report| {
            checkpoint
                .current_state_sha256
                .as_deref()
                .is_none_or(|value| report.current_state_sha256 == value)
                && checkpoint
                    .report_sha256
                    .as_deref()
                    .is_none_or(|value| report.report_sha256 == value)
        })
        && tickets_sha256.is_none_or(|tickets_sha256| {
            checkpoint
                .tickets_sha256
                .as_deref()
                .is_none_or(|value| tickets_sha256 == value)
        });
    if matches {
        Ok(())
    } else {
        Err(FrontierImportCoordinatorError::StaleState)
    }
}

fn verify_checkpoint(
    checkpoint: &FrontierImportCheckpoint,
) -> Result<(), FrontierImportCoordinatorError> {
    verify_frontier_import_checkpoint(checkpoint)
        .map_err(|_| FrontierImportCoordinatorError::CheckpointFailure)
}

fn ticket_digest(
    ticket: &FrontierLocalFlowTicket,
) -> Result<String, FrontierImportCoordinatorError> {
    let mut unsigned = ticket.clone();
    unsigned.ticket_sha256 = ZERO_SHA256.to_owned();
    digest(&("agentmage-frontier-local-flow-ticket-v1", unsigned))
}

fn digest<T: Serialize>(value: &T) -> Result<String, FrontierImportCoordinatorError> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| FrontierImportCoordinatorError::InvalidInput)?;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(&bytes) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a string cannot fail");
    }
    Ok(output)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ActionId, AuthorityClass, BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION,
        ContractPayload, CorrelationId, DataSensitivity, EvidenceKind,
        FrontierReturnArtifactDeclaration, FrontierReturnArtifactKind, FrontierReturnInput,
        FrontierReturnKind, FrontierReturnedStep, FrontierTaskTier, GrantOperation,
        OperationBinding, PlanId, RequiredGrantTemplate, RollbackPlan, SchemaId, SchemaReference,
        StopCondition, StopConditionKind, TaskId, ToolCallId, ToolDefinition, ToolId,
        ToolRiskLevel, WorkPacketId, WorkPacketState,
    };
    use agentmage_kernel_engine::{
        frontier_import::seal_frontier_return_manifest,
        tooling::{Tool, ToolRegistry},
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[derive(Clone, Default)]
    struct RecordingCheckpoints {
        values: Vec<FrontierImportCheckpoint>,
        fail_after: Option<FrontierImportPhase>,
        failed: bool,
    }

    impl FrontierImportCheckpointPort for RecordingCheckpoints {
        fn load_latest(
            &mut self,
            transaction_id: &str,
        ) -> Result<Option<FrontierImportCheckpoint>, FrontierImportCoordinatorError> {
            Ok(self
                .values
                .iter()
                .rev()
                .find(|checkpoint| checkpoint.transaction_id == transaction_id)
                .cloned())
        }

        fn commit(
            &mut self,
            checkpoint: &FrontierImportCheckpoint,
        ) -> Result<(), FrontierImportCoordinatorError> {
            if let Some(previous) = self.values.last()
                && (checkpoint.generation != previous.generation + 1
                    || checkpoint.previous_checkpoint_sha256.as_deref()
                        != Some(&previous.checkpoint_sha256))
            {
                return Err(FrontierImportCoordinatorError::CheckpointFailure);
            }
            self.values.push(checkpoint.clone());
            if self.fail_after == Some(checkpoint.phase) && !self.failed {
                self.failed = true;
                return Err(FrontierImportCoordinatorError::CheckpointFailure);
            }
            Ok(())
        }
    }

    struct FixtureTool {
        definition: ToolDefinition,
    }

    impl Tool for FixtureTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }
    }

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn schema(identity: &str) -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw(identity),
            schema_version: 1,
            schema_sha256: hash('a'),
        }
    }

    fn registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        for (identity, operation) in [
            ("fixture.frontier-read", GrantOperation::WorkspaceRead),
            ("fixture.frontier-write", GrantOperation::WorkspaceWrite),
        ] {
            let operation = OperationBinding::new(operation);
            registry
                .register_tool(Box::new(FixtureTool {
                    definition: ToolDefinition {
                        schema_version: CONTRACT_SCHEMA_VERSION,
                        tool_id: ToolId::from_raw(identity),
                        tool_version: "1.0.0".to_owned(),
                        display_name: "Frontier fixture reader".to_owned(),
                        description: "Validates one inert local frontier fixture".to_owned(),
                        input_schema: schema(&format!("{identity}.input")),
                        output_schema: schema("fixture.frontier-read.output"),
                        risk_level: ToolRiskLevel::Low,
                        declared_effects: vec![operation],
                        required_grant: RequiredGrantTemplate {
                            operation,
                            target_scope: "workspace-file".to_owned(),
                            single_use: true,
                        },
                        timeout_ms: 1_000,
                    },
                }))
                .expect("fixture tool registers");
        }
        registry
    }

    fn tool_call(identity: &str, suffix: &str) -> ToolCall {
        let bytes = br#"{"path":"fixture.txt"}"#.to_vec();
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(format!("frontier-tool-call-{suffix}")),
            correlation_id: CorrelationId::from_raw(format!("frontier-correlation-{suffix}")),
            action_id: ActionId::from_raw(format!("frontier-action-{suffix}")),
            tool_id: ToolId::from_raw(identity),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: schema(&format!("{identity}.input")),
                media_type: "application/json".to_owned(),
                sha256: raw_sha256(&bytes),
                bytes,
            },
        }
    }

    fn raw_sha256(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            use std::fmt::Write as _;
            write!(&mut output, "{byte:02x}").expect("write hash");
        }
        output
    }

    fn manifest() -> FrontierReturnManifest {
        let read = OperationBinding::new(GrantOperation::WorkspaceRead);
        let write = OperationBinding::new(GrantOperation::WorkspaceWrite);
        let artifact_bytes = b"--- old\n+++ new\n";
        let mut manifest = FrontierReturnManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            import_id: "frontier-import-native-0001".to_owned(),
            request_packet_sha256: hash('1'),
            base_workspace_state_sha256: hash('2'),
            base_model_state_sha256: hash('3'),
            base_policy_sha256: hash('4'),
            tier: FrontierTaskTier::FrontierRecommended,
            result_kind: FrontierReturnKind::Artifact,
            rationale: "Route the proposal through current local contracts only.".to_owned(),
            inputs: vec![FrontierReturnInput {
                input_id: "input-0001".to_owned(),
                input_sha256: hash('5'),
            }],
            artifacts: vec![FrontierReturnArtifactDeclaration {
                artifact_id: "artifact-patch-0001".to_owned(),
                kind: FrontierReturnArtifactKind::Patch,
                media_type: "text/x-diff".to_owned(),
                display_path: "proposal/change.diff".to_owned(),
                byte_length: artifact_bytes.len() as u64,
                content_sha256: raw_sha256(artifact_bytes),
            }],
            citations: Vec::new(),
            steps: vec![
                FrontierReturnedStep {
                    step_id: "step-file-0001".to_owned(),
                    kind: FrontierReturnedStepKind::FileProposal,
                    rationale: "Require a current exact-preimage write preview.".to_owned(),
                    artifact_ids: vec!["artifact-patch-0001".to_owned()],
                    citation_ids: Vec::new(),
                    acceptance_checks: vec!["Exact preimage review passes.".to_owned()],
                    proposed_operation: Some(write),
                    approval_requirements: vec![write],
                },
                FrontierReturnedStep {
                    step_id: "step-test-0001".to_owned(),
                    kind: FrontierReturnedStepKind::TestResult,
                    rationale: "Require a fresh trusted local validation.".to_owned(),
                    artifact_ids: Vec::new(),
                    citation_ids: Vec::new(),
                    acceptance_checks: vec!["Fresh validation passes.".to_owned()],
                    proposed_operation: None,
                    approval_requirements: Vec::new(),
                },
                FrontierReturnedStep {
                    step_id: "step-tool-0001".to_owned(),
                    kind: FrontierReturnedStepKind::ToolProposal,
                    rationale: "Enter the registered native tool path.".to_owned(),
                    artifact_ids: Vec::new(),
                    citation_ids: Vec::new(),
                    acceptance_checks: vec!["Registered tool validation passes.".to_owned()],
                    proposed_operation: Some(read),
                    approval_requirements: vec![read],
                },
            ],
            acceptance_checks: vec!["Every proposal reenters local policy.".to_owned()],
            approval_requirements: vec![read, write],
            remaining_steps: vec!["Obtain fresh local approvals and evidence.".to_owned()],
            external_content_untrusted: true,
            authority_granted: false,
            completion_credit: false,
            outbound_network_required: false,
            manifest_sha256: String::new(),
        };
        seal_frontier_return_manifest(&mut manifest).expect("seal manifest");
        manifest
    }

    fn current() -> FrontierCurrentState {
        FrontierCurrentState {
            request_packet_sha256: hash('1'),
            workspace_state_sha256: hash('2'),
            model_state_sha256: hash('3'),
            policy_sha256: hash('4'),
            permissions_sha256: hash('6'),
            permissions_checked: true,
            citations: Vec::new(),
        }
    }

    fn packet(suffix: &str, authority: AuthorityClass, acceptance_check: &str) -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw(format!("frontier-packet-{suffix}")),
            task_id: TaskId::from_raw(format!("frontier-task-{suffix}")),
            revision: 1,
            objective: "Re-evaluate one imported proposal locally".to_owned(),
            reason: "Imported content carries no authority".to_owned(),
            owner: "local-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["fixture.txt".to_owned()],
            expected_output: "One authority-free native-flow ticket".to_owned(),
            acceptance_checks: vec![acceptance_check.to_owned()],
            required_evidence: vec![EvidenceKind::Validation],
            required_capability_class: authority,
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
            plan_id: Some(PlanId::from_raw(format!("frontier-plan-{suffix}"))),
            state: WorkPacketState::Active,
        }
    }

    fn run(
        current: &FrontierCurrentState,
        checkpoints: &mut dyn FrontierImportCheckpointPort,
    ) -> Result<FrontierImportCoordinatorOutcome, FrontierImportCoordinatorError> {
        let manifest = manifest();
        let encoded = serde_json::to_vec(&manifest).expect("manifest JSON");
        let test_packet = packet(
            "test-0001",
            AuthorityClass::Observe,
            "Fresh validation passes.",
        );
        let tool_packet = packet(
            "tool-0001",
            AuthorityClass::Observe,
            "Registered tool validation passes.",
        );
        let file_packet = packet(
            "file-0001",
            AuthorityClass::LocalWrite,
            "Exact preimage review passes.",
        );
        let file_call = tool_call("fixture.frontier-write", "file-0001");
        let tool_call = tool_call("fixture.frontier-read", "read-0001");
        let routes = [
            FrontierStepRoute {
                step_id: "step-file-0001",
                work_packet: &file_packet,
                intent: TaskIntent::Change,
                tool_call: Some(&file_call),
            },
            FrontierStepRoute {
                step_id: "step-test-0001",
                work_packet: &test_packet,
                intent: TaskIntent::Review,
                tool_call: None,
            },
            FrontierStepRoute {
                step_id: "step-tool-0001",
                work_packet: &tool_packet,
                intent: TaskIntent::Change,
                tool_call: Some(&tool_call),
            },
        ];
        coordinate_frontier_import(
            "frontier-transaction-0001",
            "frontier-round-trip-0001".to_owned(),
            &encoded,
            &[FrontierImportedArtifact {
                artifact_id: "artifact-patch-0001".to_owned(),
                bytes: b"--- old\n+++ new\n".to_vec(),
            }],
            current,
            &routes,
            &registry(),
            None,
            vec!["frontier.feedback.local-validation-required".to_owned()],
            checkpoints,
        )
    }

    #[test]
    fn story_52_accepted_steps_enter_native_flows_with_every_authority_gate_pending() {
        let mut checkpoints = RecordingCheckpoints::default();
        let outcome = run(&current(), &mut checkpoints).expect("coordinate import");
        assert_eq!(checkpoints.values.len(), 4);
        assert_eq!(outcome.checkpoint.phase, FrontierImportPhase::Completed);
        assert_eq!(outcome.tickets.len(), 3);
        let file = &outcome.tickets[0];
        assert_eq!(file.flow, FrontierNativeFlow::ControlledWrite);
        assert!(file.fresh_grant_pending);
        assert!(file.exact_preimage_pending);
        assert!(file.trusted_validation_pending);
        assert!(file.user_approval_pending);
        assert!(!file.authority_granted);
        let validation = &outcome.tickets[1];
        assert_eq!(validation.flow, FrontierNativeFlow::Validation);
        assert!(validation.trusted_validation_pending);
        assert!(!validation.authority_granted);
        let tool = &outcome.tickets[2];
        assert_eq!(tool.flow, FrontierNativeFlow::Tool);
        assert!(tool.fresh_grant_pending);
        assert!(tool.user_approval_pending);
        assert!(tool.tool_call_sha256.is_some());
        assert!(!tool.authority_granted);
        assert_eq!(outcome.report.grant_count, 0);
        assert_eq!(outcome.report.tool_call_count, 0);
        assert_eq!(outcome.receipt.applied_effect_count, 0);
        assert_eq!(outcome.receipt.duplicate_effect_count, 0);

        let repeated = run(&current(), &mut checkpoints).expect("idempotent completed resume");
        assert_eq!(repeated.receipt, outcome.receipt);
        assert_eq!(checkpoints.values.len(), 4);
    }

    #[test]
    fn story_52_every_checkpoint_boundary_resumes_without_partial_or_duplicate_effect() {
        for phase in [
            FrontierImportPhase::Parsed,
            FrontierImportPhase::Revalidated,
            FrontierImportPhase::Routed,
            FrontierImportPhase::Completed,
        ] {
            let mut checkpoints = RecordingCheckpoints {
                fail_after: Some(phase),
                ..RecordingCheckpoints::default()
            };
            assert_eq!(
                run(&current(), &mut checkpoints),
                Err(FrontierImportCoordinatorError::CheckpointFailure)
            );
            checkpoints.fail_after = None;
            let outcome = run(&current(), &mut checkpoints).expect("resume exact transaction");
            assert_eq!(outcome.receipt.applied_effect_count, 0);
            assert_eq!(outcome.receipt.duplicate_effect_count, 0);
            assert_eq!(outcome.checkpoint.phase, FrontierImportPhase::Completed);
            assert_eq!(checkpoints.values.len(), 4);
        }
    }

    #[test]
    fn story_52_restart_refuses_state_drift_tamper_and_incomplete_routing() {
        let mut interrupted = RecordingCheckpoints {
            fail_after: Some(FrontierImportPhase::Routed),
            ..RecordingCheckpoints::default()
        };
        assert!(run(&current(), &mut interrupted).is_err());
        interrupted.fail_after = None;
        for mutate in [
            |state: &mut FrontierCurrentState| state.workspace_state_sha256 = hash('7'),
            |state: &mut FrontierCurrentState| state.model_state_sha256 = hash('7'),
            |state: &mut FrontierCurrentState| state.policy_sha256 = hash('7'),
            |state: &mut FrontierCurrentState| state.permissions_checked = false,
        ] {
            let mut changed = current();
            mutate(&mut changed);
            let mut checkpoint_copy = interrupted.clone();
            assert_eq!(
                run(&changed, &mut checkpoint_copy),
                Err(FrontierImportCoordinatorError::StaleState)
            );
        }

        let mut tampered = interrupted.clone();
        tampered
            .values
            .last_mut()
            .expect("checkpoint")
            .report_sha256 = Some(hash('9'));
        assert_eq!(
            run(&current(), &mut tampered),
            Err(FrontierImportCoordinatorError::CheckpointFailure)
        );

        let value = manifest();
        let encoded = serde_json::to_vec(&value).expect("manifest JSON");
        let mut empty = RecordingCheckpoints::default();
        assert_eq!(
            coordinate_frontier_import(
                "frontier-transaction-empty",
                "frontier-round-trip-empty".to_owned(),
                &encoded,
                &[FrontierImportedArtifact {
                    artifact_id: "artifact-patch-0001".to_owned(),
                    bytes: b"--- old\n+++ new\n".to_vec(),
                }],
                &current(),
                &[],
                &registry(),
                None,
                Vec::new(),
                &mut empty,
            ),
            Err(FrontierImportCoordinatorError::IncompleteRouting)
        );
    }

    #[test]
    fn story_52_directory_store_reopens_exact_chain_and_rejects_tampering() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agentmage-frontier-import-{}-{unique}",
            std::process::id()
        ));
        let mut store = DirectoryFrontierImportCheckpointStore::open(&root).expect("open store");
        let outcome = run(&current(), &mut store).expect("durable import");
        drop(store);
        let mut reopened =
            DirectoryFrontierImportCheckpointStore::open(&root).expect("reopen store");
        assert_eq!(
            reopened
                .load_latest("frontier-transaction-0001")
                .expect("load chain")
                .expect("checkpoint"),
            outcome.checkpoint
        );

        let terminal = fs::read_dir(&root)
            .expect("read checkpoint directory")
            .map(|entry| entry.expect("checkpoint entry").path())
            .find(|path| {
                path.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .contains(&outcome.checkpoint.checkpoint_sha256)
                })
            })
            .expect("terminal checkpoint file");
        let mut bytes = fs::read(&terminal).expect("read terminal checkpoint");
        let index = bytes.len() / 2;
        bytes[index] ^= 1;
        fs::write(&terminal, bytes).expect("tamper test checkpoint");
        assert_eq!(
            FrontierImportCheckpointPort::load_latest(&mut reopened, "frontier-transaction-0001",),
            Err(FrontierImportCoordinatorError::CheckpointFailure)
        );
        fs::remove_dir_all(&root).expect("remove exact test directory");
    }
}
