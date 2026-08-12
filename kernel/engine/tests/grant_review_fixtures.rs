use std::{collections::BTreeSet, env, fmt::Write as _, fs, path::PathBuf};

mod common;
use common::{preimage, scope, target};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, AuthorityTransactionId,
    ContractError, ContractPayload, CorrelationId, DataSensitivity, ErrorCategory, ErrorId,
    EvidenceId, EvidenceKind, EvidenceReference, GrantId, GrantNonce, GrantOperation,
    GrantSideEffect, GrantStatus, OperationAttemptId, OperationBinding, OperationOutcome, Receipt,
    ReceiptId, RequiredGrantTemplate, RetryDisposition, SchemaId, SchemaReference, SessionId,
    TaskId, ToolCall, ToolCallId, ToolDefinition, ToolId, ToolRiskLevel, to_canonical_json,
};
use agentmage_kernel_engine::{
    approval::render_approval_request,
    grants::{
        DerivedOperationGrantRequest, GrantConsumeError, GrantIssuer, SessionReadGrantRequest,
    },
    policy::{
        PolicyDenialScope, PolicyEngine, PolicyEvaluationContext, StrictLocalReadOnlyScope,
        ToolPolicyBinding,
    },
    tooling::{Tool, ToolRegistry},
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const EMIT_ENV: &str = "AGENTMAGE_EMIT_GRANT_REVIEW_FIXTURES";
const MARKER: &str = "AGENTMAGE_GRANT_REVIEW_FIXTURES=";
const APPROVAL_NAME: &str = "approval-preview.workspace-read.json";
const DECISION_NAME: &str = "policy-denial.argument-mismatch.json";
const RECEIPT_NAME: &str = "denial-receipt.argument-mismatch.json";
const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

struct FixtureTool {
    definition: ToolDefinition,
}

impl Tool for FixtureTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
}

#[derive(Serialize)]
struct PolicyDenialFixture {
    schema_version: u16,
    grant_id: GrantId,
    policy_sha256: String,
    allowed: bool,
    denial_scope: &'static str,
    code: &'static str,
    consume_code: &'static str,
    resulting_status: &'static str,
    resulting_revision: u32,
    resulting_use_count: u32,
}

#[derive(Serialize)]
struct FixtureRecord {
    name: &'static str,
    canonical_json: String,
}

fn hex_sha256(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn schema() -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw("fixture.input"),
        schema_version: 1,
        schema_sha256: "1".repeat(64),
    }
}

fn registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(FixtureTool {
            definition: ToolDefinition {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_id: ToolId::from_raw("fixture.read"),
                tool_version: "1.0.0".to_owned(),
                display_name: "Fixture reader".to_owned(),
                description: "Reads one synthetic fixture".to_owned(),
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
            },
        }))
        .expect("fixture tool must register");
    registry
}

fn generated_fixtures() -> Vec<FixtureRecord> {
    let registry = registry();
    let actor_id = ActorId::from_raw("actor-local-0001");
    let session_id = SessionId::from_raw("session-0001");
    let task_id = TaskId::from_raw("task-0001");
    let action_id = ActionId::from_raw("action-0001");
    let operation_target = target(&["src", "fixture.txt"]);
    let policy = PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
        revision: 1,
        actors: BTreeSet::from([actor_id.clone()]),
        tasks: BTreeSet::from([task_id.clone()]),
        actions: BTreeSet::from([action_id.clone()]),
        tools: BTreeSet::from([ToolPolicyBinding {
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
        }]),
        targets: BTreeSet::from([operation_target.clone()]),
    })
    .expect("strict fixture policy must build");
    let mut issuer = GrantIssuer::new();
    let parent = issuer
        .issue_session_read(SessionReadGrantRequest {
            grant_id: GrantId::from_raw("grant-parent-0001"),
            actor_id: actor_id.clone(),
            session_id: session_id.clone(),
            task_id: task_id.clone(),
            targets: vec![scope(&[])],
            excluded_targets: vec![scope(&["private"])],
            sensitivity: DataSensitivity::Ephemeral,
            issued_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 60_000,
            nonce: GrantNonce::from_raw("nonce-parent-0001"),
            maximum_derived_operations: 1,
            preview_sha256: "a".repeat(64),
            policy_sha256: policy.policy_sha256().to_owned(),
        })
        .expect("session parent must issue");
    let parent_sha256 = issuer
        .revision_hash(&parent.grant_id, parent.revision)
        .expect("parent revision hash")
        .to_owned();
    let argument_bytes = br#"{"path":["src","fixture.txt"]}"#.to_vec();
    let argument_sha256 = hex_sha256(&argument_bytes);
    let tool_call = ToolCall {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("call-0001"),
        correlation_id: CorrelationId::from_raw("correlation-0001"),
        action_id: action_id.clone(),
        tool_id: ToolId::from_raw("fixture.read"),
        tool_version: "1.0.0".to_owned(),
        arguments: ContractPayload {
            schema: schema(),
            media_type: "application/json".to_owned(),
            bytes: argument_bytes,
            sha256: argument_sha256.clone(),
        },
    };
    let preimages = vec![preimage(0, &operation_target)];
    let effects = vec![GrantSideEffect {
        operation: OperationBinding::new(GrantOperation::WorkspaceRead),
        target_indexes: vec![0],
        details_sha256: "4".repeat(64),
    }];
    let approval = render_approval_request(
        &registry,
        ApprovalRequest {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            approval_id: ApprovalId::from_raw("approval-0001"),
            proposed_grant_id: GrantId::from_raw("grant-operation-0001"),
            parent_grant_id: parent.grant_id.clone(),
            parent_grant_sha256: parent_sha256,
            actor_id: actor_id.clone(),
            session_id: session_id.clone(),
            task_id: task_id.clone(),
            action_kind: ActionKind::DeterministicTool,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            tool_call: tool_call.clone(),
            targets: vec![operation_target.clone()],
            excluded_targets: parent.excluded_targets.clone(),
            sensitivity: DataSensitivity::Ephemeral,
            preimages: preimages.clone(),
            expected_side_effects: effects.clone(),
            rollback_description: "No state change is permitted".to_owned(),
            issued_at_epoch_ms: 2_000,
            expires_at_epoch_ms: 30_000,
            policy_sha256: policy.policy_sha256().to_owned(),
            confirmation_sha256: "untrusted-candidate".to_owned(),
        },
    )
    .expect("approval preview must render");
    let approval_bytes = to_canonical_json(&approval).expect("approval must serialize");
    let operation = issuer
        .derive_operation(
            &parent.grant_id,
            DerivedOperationGrantRequest {
                grant_id: approval.proposed_grant_id.clone(),
                approval_id: approval.approval_id.clone(),
                action_id: action_id.clone(),
                action_kind: approval.action_kind,
                operation: approval.operation,
                tool_id: tool_call.tool_id.clone(),
                tool_version: tool_call.tool_version.clone(),
                targets: approval.targets.clone(),
                argument_sha256: argument_sha256.clone(),
                preimages: preimages.clone(),
                expected_side_effects: effects.clone(),
                rollback_description: approval.rollback_description.clone(),
                issued_at_epoch_ms: approval.issued_at_epoch_ms,
                expires_at_epoch_ms: approval.expires_at_epoch_ms,
                nonce: GrantNonce::from_raw("nonce-operation-0001"),
                preview_sha256: approval.confirmation_sha256.clone(),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("operation grant must derive");
    let operation_sha256 =
        hex_sha256(&to_canonical_json(&operation).expect("operation grant must serialize"));
    let mut context = PolicyEvaluationContext {
        actor_id,
        session_id: session_id.clone(),
        task_id: task_id.clone(),
        action_id: action_id.clone(),
        action_kind: ActionKind::DeterministicTool,
        tool_id: tool_call.tool_id.clone(),
        tool_version: tool_call.tool_version.clone(),
        targets: approval.targets.clone(),
        argument_sha256: "f".repeat(64),
        preimages,
        expected_side_effects: effects,
        preview_sha256: approval.confirmation_sha256.clone(),
        now_epoch_ms: 3_000,
        network_scope: None,
        credential_scope: None,
        publication_scope: None,
    };
    let decision = policy.evaluate(&issuer, &operation, &context);
    assert_eq!(decision.denial_scope, Some(PolicyDenialScope::Argument));
    assert_eq!(decision.code, "policy.deny.argument");
    assert_eq!(
        issuer.consume_for_execution(&operation.grant_id, &policy, &context),
        Err(GrantConsumeError::PolicyDenied(PolicyDenialScope::Argument))
    );
    let invalidated = issuer
        .current(&operation.grant_id)
        .expect("denied operation remains retained");
    assert_eq!(invalidated.status, GrantStatus::Invalidated);
    assert_eq!(invalidated.use_count, 0);
    let denial = PolicyDenialFixture {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        grant_id: operation.grant_id.clone(),
        policy_sha256: decision.policy_sha256,
        allowed: decision.allowed,
        denial_scope: "argument",
        code: decision.code,
        consume_code: GrantConsumeError::PolicyDenied(PolicyDenialScope::Argument).code(),
        resulting_status: "invalidated",
        resulting_revision: invalidated.revision,
        resulting_use_count: invalidated.use_count,
    };
    let denial_bytes = serde_json::to_vec(&denial).expect("denial fixture must serialize");
    let error = ContractError {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        error_id: ErrorId::from_raw("error-policy-denial-0001"),
        code: decision.code.to_owned(),
        category: ErrorCategory::Policy,
        message: "Current policy denied the operation at the argument boundary".to_owned(),
        field_path: vec!["argument_sha256".to_owned()],
        retry: RetryDisposition::Never,
        caused_by: None,
    };
    let mut receipt = Receipt {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        receipt_id: ReceiptId::from_raw("receipt-policy-denial-0001"),
        sequence: 1,
        correlation_id: tool_call.correlation_id.clone(),
        authority_transaction_id: AuthorityTransactionId::from_raw("transaction-0001"),
        operation_attempt_id: OperationAttemptId::from_raw("attempt-0001"),
        approval_id: approval.approval_id.clone(),
        grant_id: operation.grant_id.clone(),
        session_id,
        task_id,
        action_id,
        tool_call_id: Some(tool_call.tool_call_id),
        operation: operation.operation,
        outcome: OperationOutcome::Denied,
        operation_sha256,
        evidence: vec![EvidenceReference {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw("evidence-policy-denial-0001"),
            kind: EvidenceKind::Decision,
            source_id: "agentmage-kernel-policy".to_owned(),
            object_id: operation.grant_id.as_str().to_owned(),
            fragment: Some(decision.code.to_owned()),
            content_sha256: hex_sha256(&denial_bytes),
            observed_revision: Some("policy-revision-1".to_owned()),
        }],
        error: Some(error),
        previous_receipt_sha256: ZERO_HASH.to_owned(),
        receipt_sha256: ZERO_HASH.to_owned(),
        occurred_at: "2026-08-10T00:00:03Z".to_owned(),
    };
    receipt.receipt_sha256 =
        hex_sha256(&to_canonical_json(&receipt).expect("receipt preimage must serialize"));
    let receipt_bytes = to_canonical_json(&receipt).expect("receipt must serialize");
    context.argument_sha256 = argument_sha256;
    assert_eq!(
        issuer.consume_for_execution(&operation.grant_id, &policy, &context),
        Err(GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant))
    );

    vec![
        FixtureRecord {
            name: APPROVAL_NAME,
            canonical_json: String::from_utf8(approval_bytes).expect("approval JSON must be UTF-8"),
        },
        FixtureRecord {
            name: DECISION_NAME,
            canonical_json: String::from_utf8(denial_bytes).expect("denial JSON must be UTF-8"),
        },
        FixtureRecord {
            name: RECEIPT_NAME,
            canonical_json: String::from_utf8(receipt_bytes).expect("receipt JSON must be UTF-8"),
        },
    ]
}

#[test]
fn approval_and_denial_review_fixtures_match_kernel_behavior() {
    let fixtures = generated_fixtures();
    if env::var(EMIT_ENV).as_deref() == Ok("1") {
        println!(
            "{MARKER}{}",
            serde_json::to_string(&fixtures).expect("fixture bundle must serialize")
        );
        return;
    }

    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/grants/v1");
    let historical_sha256 = [
        (
            APPROVAL_NAME,
            "d718203e6f1f7b9b3c8cd9d763eb878e91bdb98ae583e36f37b7a90cd8132707",
        ),
        (
            DECISION_NAME,
            "68729e0b489b301ffccfe807234106817430d6b7055656f2d39f52d20cc64f96",
        ),
        (
            RECEIPT_NAME,
            "2ca3d6508641ae6657ba5492256013283db17069c4d2fa6df145a0b6df940d41",
        ),
    ];
    for fixture in fixtures {
        let retained = fs::read(fixture_root.join(fixture.name))
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", fixture.name));
        let expected_historical = historical_sha256
            .iter()
            .find_map(|(name, digest)| (*name == fixture.name).then_some(*digest))
            .expect("fixture has historical identity");
        assert_eq!(
            hex_sha256(&retained),
            expected_historical,
            "{}",
            fixture.name
        );
        assert_ne!(
            retained,
            fixture.canonical_json.as_bytes(),
            "{}",
            fixture.name
        );

        let current: serde_json::Value = serde_json::from_str(&fixture.canonical_json)
            .expect("current fixture must be canonical JSON");
        if fixture.name == APPROVAL_NAME {
            assert_eq!(current["approval_id"], "approval-0001");
            assert_eq!(current["operation"]["taxonomy_version"], 1);
            assert_eq!(current["operation"]["authority_class"], "observe");
        }
        if fixture.name == RECEIPT_NAME {
            assert_eq!(current["approval_id"], "approval-0001");
            assert_eq!(current["operation"]["taxonomy_version"], 1);
        }
    }
}
