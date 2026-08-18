//! Verified promoted knowledge workflows over native Chat and CLI-compatible contracts.

use agentmage_capability_knowledge::{
    KnowledgeRecord, KnowledgeRetrievalMode, KnowledgeWorkflow, KnowledgeWorkflowEvidence,
    KnowledgeWorkflowResult, run_read_only_knowledge_workflow,
};
use serde::{Deserialize, Serialize};

use crate::headless::{
    ClientCancellation, ClientCommand, ClientContentChannel, KnowledgeClientCommand,
    KnowledgeRetrievalClientMode, KnowledgeWorkflowClient, ThinClientError, ThinClientEvent,
    ThinClientEventKind, ThinClientRequest, ThinClientTransport, parse_thin_client_request,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Content-minimized result shared by native Chat and CLI-compatible clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeWorkflowProjection {
    /// Exact promoted workflow wire identity.
    pub workflow: KnowledgeWorkflowClient,
    /// Explicit retrieval path used by the trusted local host.
    pub retrieval_mode: KnowledgeRetrievalClientMode,
    /// Stable canonical source identities in deterministic order.
    pub source_record_ids: Vec<String>,
    /// Complete canonical source-record digests in the same order.
    pub source_record_sha256: Vec<String>,
    /// Current source citations retained by the workflow.
    pub citation_sha256: Vec<String>,
    /// Digest of the exact retrieval result projection.
    pub retrieval_result_sha256: String,
    /// Digest of the derived task projection when this workflow uses tasks.
    pub task_view_sha256: Option<String>,
    /// Digest of the authority-free skill context.
    pub skill_context_sha256: String,
    /// Digest of the complete interface-neutral result.
    pub result_sha256: String,
    /// Fixed zero for the v0.2 promoted workflow family.
    pub proposed_write_count: u64,
    /// Fixed false for the v0.2 promoted workflow family.
    pub source_files_changed: bool,
}

impl KnowledgeWorkflowProjection {
    fn from_result(
        workflow: KnowledgeWorkflowClient,
        retrieval_mode: KnowledgeRetrievalClientMode,
        result: KnowledgeWorkflowResult,
    ) -> Result<Self, ThinClientError> {
        if !result.interface_neutral
            || result.proposed_write_count != 0
            || result.source_files_changed
            || result.workflow != map_workflow(workflow)
            || result.evidence.retrieval_mode != map_retrieval_mode(retrieval_mode)
        {
            return Err(ThinClientError::TransportFailed);
        }
        Ok(Self {
            workflow,
            retrieval_mode,
            source_record_ids: result
                .source_record_ids
                .iter()
                .map(|identity| identity.as_str().to_owned())
                .collect(),
            source_record_sha256: result.source_record_sha256,
            citation_sha256: result.evidence.citation_sha256,
            retrieval_result_sha256: result.evidence.retrieval_result_sha256,
            task_view_sha256: result.task_view.map(|view| view.view_sha256),
            skill_context_sha256: result.skill_context.receipt.context_sha256,
            result_sha256: result.result_sha256,
            proposed_write_count: result.proposed_write_count,
            source_files_changed: result.source_files_changed,
        })
    }
}

/// One authenticated local transport for a preselected canonical record and evidence set.
pub struct KnowledgeWorkflowTransport {
    now_epoch_ms: u64,
    records: Vec<KnowledgeRecord>,
    evidence: KnowledgeWorkflowEvidence,
}

impl KnowledgeWorkflowTransport {
    /// Creates a transport without reading files, selecting evidence, or granting authority.
    #[must_use]
    pub const fn new(
        now_epoch_ms: u64,
        records: Vec<KnowledgeRecord>,
        evidence: KnowledgeWorkflowEvidence,
    ) -> Self {
        Self {
            now_epoch_ms,
            records,
            evidence,
        }
    }
}

impl ThinClientTransport for KnowledgeWorkflowTransport {
    fn exchange(
        &mut self,
        request: &[u8],
        cancellation: &ClientCancellation,
    ) -> Result<Vec<Vec<u8>>, ThinClientError> {
        let request = parse_thin_client_request(request, self.now_epoch_ms)?;
        if cancellation.is_cancelled() || request.cancellation_id != cancellation.cancellation_id()
        {
            return Err(ThinClientError::Cancelled);
        }
        let ClientCommand::Knowledge {
            action:
                KnowledgeClientCommand::Run {
                    workflow,
                    retrieval_mode,
                },
        } = request.command
        else {
            return Err(ThinClientError::InvalidValue);
        };
        if self.evidence.retrieval_mode != map_retrieval_mode(retrieval_mode) {
            return Err(ThinClientError::TransportFailed);
        }
        let result = run_read_only_knowledge_workflow(
            map_workflow(workflow),
            &self.records,
            self.evidence.clone(),
        )
        .map_err(|_| ThinClientError::TransportFailed)?;
        if cancellation.is_cancelled() {
            return Err(ThinClientError::Cancelled);
        }
        let projection =
            KnowledgeWorkflowProjection::from_result(workflow, retrieval_mode, result)?;
        let content = serde_json::to_string(&projection).map_err(|_| ThinClientError::Malformed)?;
        let content_sha256 = sha256(content.as_bytes());
        encode_stream(
            &request,
            vec![
                ThinClientEventKind::Started,
                ThinClientEventKind::Content {
                    channel: ClientContentChannel::Citation,
                    text: content,
                    text_sha256: content_sha256,
                },
                ThinClientEventKind::Receipt {
                    receipt_id: "knowledge-workflow-receipt".to_owned(),
                    receipt_sha256: projection.result_sha256.clone(),
                    outcome: "succeeded".to_owned(),
                },
                ThinClientEventKind::Completed {
                    final_state_sha256: projection.result_sha256,
                },
            ],
        )
    }
}

const fn map_workflow(workflow: KnowledgeWorkflowClient) -> KnowledgeWorkflow {
    match workflow {
        KnowledgeWorkflowClient::DailySetup => KnowledgeWorkflow::DailySetup,
        KnowledgeWorkflowClient::DailyBriefing => KnowledgeWorkflow::DailyBriefing,
        KnowledgeWorkflowClient::IssueIntake => KnowledgeWorkflow::IssueIntake,
        KnowledgeWorkflowClient::Handoff => KnowledgeWorkflow::Handoff,
        KnowledgeWorkflowClient::MeetingCleanup => KnowledgeWorkflow::MeetingCleanup,
        KnowledgeWorkflowClient::RepositoryLearning => KnowledgeWorkflow::RepositoryLearning,
        KnowledgeWorkflowClient::PlainWorkspaceSteward => KnowledgeWorkflow::PlainWorkspaceSteward,
        KnowledgeWorkflowClient::ObsidianVaultSteward => KnowledgeWorkflow::ObsidianVaultSteward,
    }
}

const fn map_retrieval_mode(mode: KnowledgeRetrievalClientMode) -> KnowledgeRetrievalMode {
    match mode {
        KnowledgeRetrievalClientMode::Lexical => KnowledgeRetrievalMode::Lexical,
        KnowledgeRetrievalClientMode::ApprovedLocalSemantic => {
            KnowledgeRetrievalMode::ApprovedLocalSemantic
        }
    }
}

fn encode_stream(
    request: &ThinClientRequest,
    kinds: Vec<ThinClientEventKind>,
) -> Result<Vec<Vec<u8>>, ThinClientError> {
    let mut total = 0_u64;
    let mut previous = ZERO_SHA256.to_owned();
    kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| {
            let mut candidate_total = total
                .checked_add(512)
                .ok_or(ThinClientError::SizeExceeded)?;
            loop {
                let event = ThinClientEvent {
                    schema_version: 0,
                    request_id: request.request_id.clone(),
                    stream_id: "knowledge-workflow-stream".to_owned(),
                    sequence: index as u64,
                    kernel_request_sha256: request.kernel_request_sha256.clone(),
                    policy_sha256: request.policy_sha256.clone(),
                    cumulative_output_bytes: candidate_total,
                    kind: kind.clone(),
                    previous_event_sha256: previous.clone(),
                    event_sha256: ZERO_SHA256.to_owned(),
                }
                .seal()?;
                let bytes = serde_json::to_vec(&event).map_err(|_| ThinClientError::Malformed)?;
                let exact = total
                    .checked_add(bytes.len() as u64)
                    .ok_or(ThinClientError::SizeExceeded)?;
                if exact == candidate_total {
                    previous = event.event_sha256;
                    total = exact;
                    return Ok(bytes);
                }
                candidate_total = exact;
            }
        })
        .collect()
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use agentmage_capability_knowledge::{
        KNOWLEDGE_SCHEMA_VERSION, KnowledgeField, KnowledgePrivacy, KnowledgeRecordId,
        KnowledgeRecordKind, KnowledgeRetention, KnowledgeRetentionKind,
    };
    use agentmage_kernel_contracts::{DataSensitivity, GrantOperation};

    use super::*;
    use crate::headless::{
        ClientAuthority, ClientExitCode, ClientStatusSnapshot, PredeclaredClientGrant,
        ThinKernelClient, kernel_operation_sha256,
    };

    const NOW: u64 = 1_786_944_000_000;

    fn record() -> KnowledgeRecord {
        KnowledgeRecord {
            schema_version: KNOWLEDGE_SCHEMA_VERSION,
            record_id: KnowledgeRecordId::parse("knowledge-task-client-workflow")
                .expect("record identity"),
            kind: KnowledgeRecordKind::Task,
            title: "Verify client workflow parity".to_owned(),
            privacy: KnowledgePrivacy::Private,
            sensitivity: DataSensitivity::Durable,
            retention: KnowledgeRetention {
                kind: KnowledgeRetentionKind::UntilSupersededOrDeleted,
                expires_at: None,
            },
            created_at: "2026-08-18T00:00:00Z".to_owned(),
            updated_at: "2026-08-18T00:00:00Z".to_owned(),
            last_verified_at: None,
            fields: vec![
                KnowledgeField {
                    name: "next_action".to_owned(),
                    value: "Run both first-party client paths".to_owned(),
                },
                KnowledgeField {
                    name: "owner".to_owned(),
                    value: "user".to_owned(),
                },
                KnowledgeField {
                    name: "priority".to_owned(),
                    value: "high".to_owned(),
                },
                KnowledgeField {
                    name: "project".to_owned(),
                    value: "AgentMage".to_owned(),
                },
                KnowledgeField {
                    name: "status".to_owned(),
                    value: "open".to_owned(),
                },
            ],
            links: Vec::new(),
            tags: vec!["workflow".to_owned()],
            evidence: Vec::new(),
        }
    }

    fn evidence(mode: KnowledgeRetrievalClientMode) -> KnowledgeWorkflowEvidence {
        KnowledgeWorkflowEvidence {
            retrieval_mode: map_retrieval_mode(mode),
            citation_sha256: vec!["a".repeat(64), "b".repeat(64)],
            retrieval_result_sha256: "c".repeat(64),
            current_and_nonconflicting: true,
        }
    }

    fn request(
        surface: crate::headless::ClientSurface,
        workflow: KnowledgeWorkflowClient,
        mode: KnowledgeRetrievalClientMode,
        ordinal: usize,
    ) -> ThinClientRequest {
        let command = ClientCommand::Knowledge {
            action: KnowledgeClientCommand::Run {
                workflow,
                retrieval_mode: mode,
            },
        };
        let status = ClientStatusSnapshot {
            workspace_id: "workspace-knowledge-client".to_owned(),
            model_profile_id: None,
            permission_profile_id: "permission-read-only".to_owned(),
            conversation_id: None,
            plan_step_id: None,
            writable_roots: Vec::new(),
            offline: true,
            status_sha256: ZERO_SHA256.to_owned(),
        }
        .seal()
        .expect("status");
        let policy_sha256 = "d".repeat(64);
        let operation_sha256 =
            kernel_operation_sha256(&status.workspace_id, &command, &policy_sha256)
                .expect("operation digest");
        let request_id = format!("knowledge-client-request-{ordinal}");
        ThinClientRequest {
            schema_version: 0,
            request_id: request_id.clone(),
            surface,
            status,
            command,
            authority: ClientAuthority::Predeclared {
                grant: PredeclaredClientGrant {
                    grant_id: format!("knowledge-client-grant-{ordinal}"),
                    operation: GrantOperation::WorkspaceRead,
                    policy_sha256: policy_sha256.clone(),
                    arguments_sha256: operation_sha256,
                    nonce_sha256: "e".repeat(64),
                    issued_at_epoch_ms: NOW - 1,
                    expires_at_epoch_ms: NOW + 1,
                    single_use: true,
                },
            },
            policy_sha256,
            cancellation_id: format!("knowledge-client-cancel-{ordinal}"),
            max_event_bytes: 1024 * 1024,
            max_output_bytes: 4 * 1024 * 1024,
            resume: None,
            kernel_request_sha256: ZERO_SHA256.to_owned(),
            request_sha256: ZERO_SHA256.to_owned(),
        }
        .seal(NOW)
        .expect("request")
    }

    fn execute(
        surface: crate::headless::ClientSurface,
        workflow: KnowledgeWorkflowClient,
        mode: KnowledgeRetrievalClientMode,
        ordinal: usize,
        records: &[KnowledgeRecord],
    ) -> KnowledgeWorkflowProjection {
        let request = request(surface, workflow, mode, ordinal);
        let cancellation = ClientCancellation::new(&request.cancellation_id).expect("cancellation");
        let transport = KnowledgeWorkflowTransport::new(NOW, records.to_vec(), evidence(mode));
        let mut client = ThinKernelClient::new(surface, transport);
        let stream = client
            .execute(&request, NOW, &cancellation)
            .expect("verified knowledge stream");
        assert_eq!(stream.exit_code, ClientExitCode::Success);
        assert_eq!(stream.events.len(), 4);
        let ThinClientEventKind::Content {
            channel: ClientContentChannel::Citation,
            text,
            ..
        } = &stream.events[1].kind
        else {
            panic!("expected canonical citation projection");
        };
        let projection: KnowledgeWorkflowProjection =
            serde_json::from_str(text).expect("projection");
        let ThinClientEventKind::Completed { final_state_sha256 } = &stream.events[3].kind else {
            panic!("expected terminal completion");
        };
        assert_eq!(final_state_sha256, &projection.result_sha256);
        projection
    }

    #[test]
    fn s_028_it01_every_promoted_workflow_has_native_chat_and_cli_evidence_parity() {
        let records = vec![record()];
        let before = serde_json::to_vec(&records).expect("source snapshot");
        let workflows = [
            KnowledgeWorkflowClient::DailySetup,
            KnowledgeWorkflowClient::DailyBriefing,
            KnowledgeWorkflowClient::IssueIntake,
            KnowledgeWorkflowClient::Handoff,
            KnowledgeWorkflowClient::MeetingCleanup,
            KnowledgeWorkflowClient::RepositoryLearning,
            KnowledgeWorkflowClient::PlainWorkspaceSteward,
            KnowledgeWorkflowClient::ObsidianVaultSteward,
        ];
        let modes = [
            KnowledgeRetrievalClientMode::Lexical,
            KnowledgeRetrievalClientMode::ApprovedLocalSemantic,
        ];
        for (workflow_index, workflow) in workflows.into_iter().enumerate() {
            let mut mode_results = Vec::new();
            for (mode_index, mode) in modes.into_iter().enumerate() {
                let ordinal = workflow_index * 10 + mode_index * 2;
                let native_chat = execute(
                    crate::headless::ClientSurface::NativeChat,
                    workflow,
                    mode,
                    ordinal,
                    &records,
                );
                let cli = execute(
                    crate::headless::ClientSurface::InteractiveCli,
                    workflow,
                    mode,
                    ordinal + 1,
                    &records,
                );
                assert_eq!(native_chat, cli);
                assert_eq!(native_chat.proposed_write_count, 0);
                assert!(!native_chat.source_files_changed);
                mode_results.push(native_chat);
            }
            assert_eq!(
                mode_results[0].source_record_ids,
                mode_results[1].source_record_ids
            );
            assert_eq!(
                mode_results[0].source_record_sha256,
                mode_results[1].source_record_sha256
            );
            assert_eq!(
                mode_results[0].citation_sha256,
                mode_results[1].citation_sha256
            );
            assert_eq!(
                mode_results[0].task_view_sha256,
                mode_results[1].task_view_sha256
            );
            assert_eq!(
                mode_results[0].skill_context_sha256,
                mode_results[1].skill_context_sha256
            );
        }
        assert_eq!(
            serde_json::to_vec(&records).expect("source unchanged"),
            before
        );
    }

    #[test]
    fn retrieval_substitution_fails_before_projection() {
        let records = vec![record()];
        let request = request(
            crate::headless::ClientSurface::NativeChat,
            KnowledgeWorkflowClient::DailySetup,
            KnowledgeRetrievalClientMode::ApprovedLocalSemantic,
            100,
        );
        let cancellation = ClientCancellation::new(&request.cancellation_id).expect("cancellation");
        let transport = KnowledgeWorkflowTransport::new(
            NOW,
            records,
            evidence(KnowledgeRetrievalClientMode::Lexical),
        );
        let mut client =
            ThinKernelClient::new(crate::headless::ClientSurface::NativeChat, transport);
        assert_eq!(
            client.execute(&request, NOW, &cancellation),
            Err(ThinClientError::TransportFailed)
        );
    }
}
