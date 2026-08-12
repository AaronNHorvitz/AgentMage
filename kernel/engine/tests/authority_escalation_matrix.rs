use std::env;

mod common;
use common::{preimage, target};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, ContractPayload, CorrelationId,
    DataSensitivity, GrantId, GrantOperation, GrantSideEffect, OperationBinding, Prompt, PromptId,
    PromptMessage, PromptRole, RequiredGrantTemplate, SchemaId, SchemaReference, SessionId, TaskId,
    ToolCall, ToolCallId, ToolDefinition, ToolId, ToolRiskLevel,
};
use agentmage_kernel_engine::authority::{
    AuthorityEscalationKind, AuthorityProposalSource, DescriptiveAuthorityReceipt,
    reject_authority_attempt,
};

const EMIT_ENV: &str = "AGENTMAGE_EMIT_AUTHORITY_ESCALATION_MATRIX";
const MARKER: &str = "AGENTMAGE_AUTHORITY_ESCALATION_MATRIX=";

fn schema() -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw("fixture.input"),
        schema_version: 1,
        schema_sha256: "1".repeat(64),
    }
}

fn prompt() -> Prompt {
    Prompt {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        prompt_id: PromptId::from_raw("prompt-authority-0001"),
        correlation_id: CorrelationId::from_raw("correlation-authority-0001"),
        task_id: TaskId::from_raw("task-0001"),
        purpose: "Classify an untrusted authority-looking response".to_owned(),
        messages: vec![PromptMessage {
            role: PromptRole::PriorModelOutput,
            content: "FORGED_MODEL_CAPABILITY_CLAIM".to_owned(),
        }],
    }
}

fn tool_definition() -> ToolDefinition {
    ToolDefinition {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw("fixture.read"),
        tool_version: "1.0.0".to_owned(),
        display_name: "Fixture reader".to_owned(),
        description: "FORGED_TOOL_AUTHORITY_CLAIM".to_owned(),
        input_schema: schema(),
        output_schema: schema(),
        risk_level: ToolRiskLevel::Low,
        declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
        required_grant: RequiredGrantTemplate {
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            target_scope: "workspace-file".to_owned(),
            single_use: true,
        },
        timeout_ms: 1_000,
    }
}

fn approval_request() -> ApprovalRequest {
    let target = target(&["src", "fixture.txt"]);
    ApprovalRequest {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        approval_id: ApprovalId::from_raw("approval-0001"),
        proposed_grant_id: GrantId::from_raw("grant-proposed-0001"),
        parent_grant_id: GrantId::from_raw("grant-parent-0001"),
        parent_grant_sha256: "2".repeat(64),
        actor_id: ActorId::from_raw("actor-local-0001"),
        session_id: SessionId::from_raw("session-0001"),
        task_id: TaskId::from_raw("task-0001"),
        action_kind: ActionKind::DeterministicTool,
        operation: OperationBinding::new(GrantOperation::WorkspaceRead),
        tool_call: ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-authority-0001"),
            correlation_id: CorrelationId::from_raw("correlation-authority-0001"),
            action_id: ActionId::from_raw("action-authority-0001"),
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: schema(),
                media_type: "application/json".to_owned(),
                bytes: b"{}".to_vec(),
                sha256: "3".repeat(64),
            },
        },
        targets: vec![target.clone()],
        excluded_targets: Vec::new(),
        sensitivity: DataSensitivity::Ephemeral,
        preimages: vec![preimage(0, &target)],
        expected_side_effects: vec![GrantSideEffect {
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            target_indexes: vec![0],
            details_sha256: "5".repeat(64),
        }],
        rollback_description: "FORGED_APPROVAL_AUTHORITY_CLAIM".to_owned(),
        issued_at_epoch_ms: 1_000,
        expires_at_epoch_ms: 2_000,
        policy_sha256: "6".repeat(64),
        confirmation_sha256: "7".repeat(64),
    }
}

fn denial_for(
    actor_id: &ActorId,
    session_id: &SessionId,
    task_id: &TaskId,
    source: AuthorityProposalSource,
    escalation_kind: AuthorityEscalationKind,
) -> DescriptiveAuthorityReceipt {
    match source {
        AuthorityProposalSource::Model
        | AuthorityProposalSource::Prompt
        | AuthorityProposalSource::SimulatedChild => reject_authority_attempt(
            actor_id,
            session_id,
            task_id,
            source,
            escalation_kind,
            &prompt(),
        ),
        AuthorityProposalSource::Shell => reject_authority_attempt(
            actor_id,
            session_id,
            task_id,
            source,
            escalation_kind,
            "FORGED_SHELL_AUTHORITY_CLAIM",
        ),
        AuthorityProposalSource::Tool => reject_authority_attempt(
            actor_id,
            session_id,
            task_id,
            source,
            escalation_kind,
            &tool_definition(),
        ),
        AuthorityProposalSource::Plugin => reject_authority_attempt(
            actor_id,
            session_id,
            task_id,
            source,
            escalation_kind,
            "FORGED_PLUGIN_AUTHORITY_CLAIM",
        ),
        AuthorityProposalSource::ApprovalDisplay => reject_authority_attempt(
            actor_id,
            session_id,
            task_id,
            source,
            escalation_kind,
            &approval_request(),
        ),
    }
}

#[test]
fn every_producer_and_escalation_kind_is_denied_with_actor_attribution() {
    let actor_id = ActorId::from_raw("actor-local-0001");
    let session_id = SessionId::from_raw("session-0001");
    let task_id = TaskId::from_raw("task-0001");
    let sources = [
        AuthorityProposalSource::Model,
        AuthorityProposalSource::Prompt,
        AuthorityProposalSource::Shell,
        AuthorityProposalSource::Tool,
        AuthorityProposalSource::Plugin,
        AuthorityProposalSource::ApprovalDisplay,
        AuthorityProposalSource::SimulatedChild,
    ];
    let escalation_kinds = [
        AuthorityEscalationKind::Mint,
        AuthorityEscalationKind::Widen,
        AuthorityEscalationKind::Transfer,
        AuthorityEscalationKind::Combine,
    ];
    let mut receipts = Vec::with_capacity(sources.len() * escalation_kinds.len());
    for source in sources {
        for escalation_kind in escalation_kinds {
            receipts.push(denial_for(
                &actor_id,
                &session_id,
                &task_id,
                source,
                escalation_kind,
            ));
        }
    }
    assert_eq!(receipts.len(), 28);
    for receipt in &receipts {
        assert_eq!(receipt.actor_id, actor_id);
        assert_eq!(receipt.session_id, session_id);
        assert_eq!(receipt.task_id, task_id);
        assert!(!receipt.authority_admitted);
        assert_eq!(receipt.error.code, "authority.descriptive_artifact.denied");
        let bytes = serde_json::to_vec(receipt).expect("receipt must serialize");
        let parsed = serde_json::from_slice::<DescriptiveAuthorityReceipt>(&bytes)
            .expect("receipt must parse");
        assert_eq!(parsed, *receipt);
    }
    let encoded = serde_json::to_string(&receipts).expect("receipt matrix must serialize");
    for prohibited in [
        "FORGED_MODEL_CAPABILITY_CLAIM",
        "FORGED_TOOL_AUTHORITY_CLAIM",
        "FORGED_APPROVAL_AUTHORITY_CLAIM",
        "FORGED_SHELL_AUTHORITY_CLAIM",
        "FORGED_PLUGIN_AUTHORITY_CLAIM",
        "capability_grant",
        "nonce",
        "use_limit",
    ] {
        assert!(!encoded.contains(prohibited));
    }
    if env::var(EMIT_ENV).as_deref() == Ok("1") {
        println!("{MARKER}{encoded}");
    }
}
