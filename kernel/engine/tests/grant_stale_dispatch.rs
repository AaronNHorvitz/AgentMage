use std::{collections::BTreeSet, env, fmt::Write as _};

mod common;
use common::{preimage, scope, target};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ActorId, ApprovalId, ApprovalRequest, CapabilityGrant, ContractPayload,
    CorrelationId, DataSensitivity, GrantId, GrantNonce, GrantOperation, GrantSideEffect,
    GrantStatus, OperationBinding, RequiredGrantTemplate, SchemaId, SchemaReference, SessionId,
    TaskId, ToolCall, ToolCallId, ToolDefinition, ToolId, ToolRiskLevel,
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

const EMIT_ENV: &str = "AGENTMAGE_EMIT_GRANT_STALE_DISPATCH";
const MARKER: &str = "AGENTMAGE_GRANT_STALE_DISPATCH=";

#[derive(Clone, Copy)]
enum Mutation {
    Policy,
    Preimage,
    Task,
    Preview,
}

impl Mutation {
    const ALL: [Self; 4] = [Self::Policy, Self::Preimage, Self::Task, Self::Preview];

    const fn name(self) -> &'static str {
        match self {
            Self::Policy => "policy",
            Self::Preimage => "preimage",
            Self::Task => "task",
            Self::Preview => "preview",
        }
    }

    const fn expected_scope(self) -> PolicyDenialScope {
        match self {
            Self::Policy => PolicyDenialScope::Grant,
            Self::Preimage => PolicyDenialScope::Preimage,
            Self::Task => PolicyDenialScope::Task,
            Self::Preview => PolicyDenialScope::Preview,
        }
    }
}

#[derive(Serialize)]
struct StaleDispatchTrace {
    schema_version: u16,
    mutation: &'static str,
    approval_confirmation_matches_grant: bool,
    final_boundary: &'static str,
    denial_scope: &'static str,
    denial_code: &'static str,
    atomic_consumption_count: u32,
    worker_start_count: u32,
    effect_count: u32,
    terminal_status: &'static str,
    terminal_revision: u32,
    terminal_use_count: u32,
    exact_replay_allowed: bool,
}

struct FixtureTool {
    definition: ToolDefinition,
}

impl Tool for FixtureTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
}

struct Fixture {
    issuer: GrantIssuer,
    policy: PolicyEngine,
    grant: CapabilityGrant,
    context: PolicyEvaluationContext,
    approval_confirmation_sha256: String,
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

fn policy(revision: u32) -> PolicyEngine {
    PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
        revision,
        actors: BTreeSet::from([ActorId::from_raw("actor-local-0001")]),
        tasks: BTreeSet::from([TaskId::from_raw("task-0001")]),
        actions: BTreeSet::from([ActionId::from_raw("action-0001")]),
        tools: BTreeSet::from([ToolPolicyBinding {
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
        }]),
        targets: BTreeSet::from([target(&["src", "fixture.txt"])]),
    })
    .expect("strict fixture policy must build")
}

fn fixture() -> Fixture {
    let registry = registry();
    let policy = policy(1);
    let actor_id = ActorId::from_raw("actor-local-0001");
    let session_id = SessionId::from_raw("session-0001");
    let task_id = TaskId::from_raw("task-0001");
    let action_id = ActionId::from_raw("action-0001");
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
            preview_sha256: "2".repeat(64),
            policy_sha256: policy.policy_sha256().to_owned(),
        })
        .expect("session parent must issue");
    let parent_sha256 = issuer
        .revision_hash(&parent.grant_id, parent.revision)
        .expect("parent hash")
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
    let operation_target = target(&["src", "fixture.txt"]);
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
            targets: vec![operation_target],
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
    .expect("approval must render");
    let grant = issuer
        .derive_operation(
            &parent.grant_id,
            DerivedOperationGrantRequest {
                grant_id: approval.proposed_grant_id,
                approval_id: approval.approval_id,
                action_id: action_id.clone(),
                action_kind: approval.action_kind,
                operation: approval.operation,
                tool_id: tool_call.tool_id.clone(),
                tool_version: tool_call.tool_version.clone(),
                targets: approval.targets.clone(),
                argument_sha256: argument_sha256.clone(),
                preimages: preimages.clone(),
                expected_side_effects: effects.clone(),
                rollback_description: approval.rollback_description,
                issued_at_epoch_ms: approval.issued_at_epoch_ms,
                expires_at_epoch_ms: approval.expires_at_epoch_ms,
                nonce: GrantNonce::from_raw("nonce-operation-0001"),
                preview_sha256: approval.confirmation_sha256.clone(),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("operation grant must derive");
    let context = PolicyEvaluationContext {
        actor_id,
        session_id,
        task_id,
        action_id,
        action_kind: ActionKind::DeterministicTool,
        tool_id: tool_call.tool_id,
        tool_version: tool_call.tool_version,
        targets: grant.targets.clone(),
        argument_sha256: grant.argument_sha256.clone(),
        preimages: grant.preimages.clone(),
        expected_side_effects: grant.expected_side_effects.clone(),
        preview_sha256: grant.preview_sha256.clone(),
        now_epoch_ms: 3_000,
        network_scope: None,
        credential_scope: None,
        publication_scope: None,
    };
    Fixture {
        issuer,
        policy,
        grant,
        context,
        approval_confirmation_sha256: approval.confirmation_sha256,
    }
}

fn scope_name(scope: PolicyDenialScope) -> &'static str {
    match scope {
        PolicyDenialScope::Grant => "grant",
        PolicyDenialScope::Task => "task",
        PolicyDenialScope::Preimage => "preimage",
        PolicyDenialScope::Preview => "preview",
        _ => panic!("unexpected stale-dispatch scope"),
    }
}

fn run_mutation(mutation: Mutation) -> StaleDispatchTrace {
    let mut fixture = fixture();
    assert_eq!(
        fixture.approval_confirmation_sha256,
        fixture.grant.preview_sha256
    );
    let exact_context = fixture.context.clone();
    let changed_policy;
    let selected_policy = if matches!(mutation, Mutation::Policy) {
        changed_policy = policy(2);
        &changed_policy
    } else {
        match mutation {
            Mutation::Preimage => {
                fixture.context.preimages[0].content_sha256 = "a".repeat(64);
            }
            Mutation::Task => {
                fixture.context.task_id = TaskId::from_raw("task-changed-0001");
            }
            Mutation::Preview => {
                fixture.context.preview_sha256 = "b".repeat(64);
            }
            Mutation::Policy => unreachable!(),
        }
        &fixture.policy
    };
    let expected_scope = mutation.expected_scope();
    assert_eq!(
        fixture.issuer.consume_for_execution(
            &fixture.grant.grant_id,
            selected_policy,
            &fixture.context,
        ),
        Err(GrantConsumeError::PolicyDenied(expected_scope))
    );
    assert_eq!(
        fixture.issuer.consume_for_execution(
            &fixture.grant.grant_id,
            &fixture.policy,
            &exact_context,
        ),
        Err(GrantConsumeError::PolicyDenied(PolicyDenialScope::Grant))
    );
    let retained = fixture
        .issuer
        .current(&fixture.grant.grant_id)
        .expect("invalidated grant must remain");
    assert_eq!(retained.status, GrantStatus::Invalidated);
    assert_eq!(retained.revision, 2);
    assert_eq!(retained.use_count, 0);
    StaleDispatchTrace {
        schema_version: 1,
        mutation: mutation.name(),
        approval_confirmation_matches_grant: true,
        final_boundary: "GrantIssuer::consume_for_execution",
        denial_scope: scope_name(expected_scope),
        denial_code: expected_scope.code(),
        atomic_consumption_count: 0,
        worker_start_count: 0,
        effect_count: 0,
        terminal_status: "invalidated",
        terminal_revision: retained.revision,
        terminal_use_count: retained.use_count,
        exact_replay_allowed: false,
    }
}

#[test]
fn post_approval_changes_fail_at_final_consumption_before_worker_start() {
    let traces = Mutation::ALL.map(run_mutation);
    if env::var(EMIT_ENV).as_deref() == Ok("1") {
        println!(
            "{MARKER}{}",
            serde_json::to_string(&traces).expect("stale-dispatch traces must serialize")
        );
    }
}
