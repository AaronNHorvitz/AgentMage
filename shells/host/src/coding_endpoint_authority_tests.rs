// SPDX-License-Identifier: MIT OR Apache-2.0
//! Component authority checks with real Linux descriptors; no agent/model workflow.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use agentmage_capability_read_only::{ReadOnlyToolKind, read_only_tool_definition};
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    grants::{GrantIssuer, SessionReadGrantRequest},
    policy::PolicyEvaluationContext,
    runtime_coordinator::seal_runtime_approval_challenge,
    runtime_loop::runtime_action_id,
    tooling::{Tool, ToolRegistry},
};
use agentmage_platform_linux::{
    LinuxAuthorizedWorkspace, LinuxPathAdapter, select_test_linux_workspace,
};

use super::*;

struct NativeWorkspace {
    held: LinuxAuthorizedWorkspace,
    root: PathBuf,
}
impl NativeWorkspace {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "agentmage-endpoint-policy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join("private")).unwrap();
        fs::write(root.join("src/lib.rs"), b"fn answer() -> u8 { 42 }\n").unwrap();
        fs::write(
            root.join("private/secret.txt"),
            b"synthetic private fixture\n",
        )
        .unwrap();
        let held = select_test_linux_workspace(
            &root,
            WorkspaceId::from_raw("endpoint-policy-workspace"),
            WorkspaceAuthorizationId::from_raw("endpoint-policy-authorization"),
            AdapterInstanceId::from_raw("endpoint-policy-adapter"),
        )
        .unwrap();
        Self { held, root }
    }
    fn target(&self, components: &[&str]) -> GrantTarget {
        let adapter = LinuxPathAdapter::new(self.held.adapter_instance_id().clone(), 1024);
        let path = WorkspacePath::new(self.held.workspace_id().clone(), components.iter().copied())
            .unwrap();
        let object = adapter
            .resolve(&self.held, &path, PathResolutionIntent::ReadFile)
            .unwrap();
        object.revalidate().unwrap();
        GrantTarget::held_object(&object).unwrap()
    }
}
impl Drop for NativeWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

struct Definition(ToolDefinition);
impl Tool for Definition {
    fn definition(&self) -> &ToolDefinition {
        &self.0
    }
    fn validate_arguments(&self, _: &[u8]) -> Vec<ValidationIssue> {
        Vec::new()
    }
}
fn registry(definition: ToolDefinition) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register_tool(Box::new(Definition(definition)))
        .unwrap();
    registry
}
fn read_registry() -> ToolRegistry {
    registry(read_only_tool_definition(ReadOnlyToolKind::ReadText))
}

// Direct endpoint framing: no native request, coding profile or model fixture.
fn request(
    workspace: &LinuxAuthorizedWorkspace,
    registry: &ToolRegistry,
) -> RuntimeEndpointRunRequest {
    let task_id = TaskId::from_raw("endpoint-policy-task");
    let session_id = SessionId::from_raw("endpoint-policy-session");
    let objective = "Read the selected source with exact approval";
    let check = "Ground observations in current source";
    let catalog_id = ToolCatalogId::from_raw("endpoint-policy-catalog");
    let visible_tools = runtime_tool_references(registry).unwrap();
    seal_runtime_endpoint_run_request(RuntimeEndpointRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("endpoint-policy-run"),
        session_id: session_id.clone(),
        mode: RuntimeSessionMode::EphemeralReadOnly,
        task: Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: task_id.clone(),
            session_id,
            objective: objective.into(),
            acceptance_criteria: vec![check.into()],
            constraints: vec!["Read only; no network".into()],
            status: TaskStatus::Ready,
        },
        work_packet: WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("endpoint-policy-packet"),
            task_id,
            revision: 1,
            objective: objective.into(),
            reason: "Exercise the kernel authority boundary".into(),
            owner: "synthetic-local-actor".into(),
            authoritative_evidence: vec![],
            mutable_files: vec![],
            protected_files: vec!["private".into()],
            expected_output: "Grounded source observations".into(),
            acceptance_checks: vec![check.into()],
            required_evidence: vec![EvidenceKind::ToolOutput],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 5,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 5,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 4,
                },
            ],
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
                description: format!("Stop for {kind:?}"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No workspace changes".into(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-11-08".into(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: vec![],
            superseding_work: None,
            validation_issues: vec![],
            plan_id: Some(PlanId::from_raw("endpoint-policy-plan")),
            state: WorkPacketState::Active,
        },
        workspace_id: workspace.workspace_id().clone(),
        workspace_snapshot_sha256: "a".repeat(64),
        repository_snapshot_id: RepositorySnapshotId::from_raw("endpoint-policy-snapshot"),
        repository_snapshot_sha256: "b".repeat(64),
        model_selection: EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic-model".into(),
        },
        context_budget: RuntimeEndpointContextBudget {
            max_messages: 8,
            max_content_bytes: 4096,
            max_serialized_messages_bytes: 8192,
            max_request_body_bytes: 16384,
        },
        max_output_tokens: 128,
        tool_catalog_sha256: runtime_tool_catalog_sha256(&catalog_id, &visible_tools).unwrap(),
        tool_catalog_id: catalog_id,
        visible_tools,
        policy_id: PolicyId::from_raw("unbound-startup-policy"),
        policy_sha256: "c".repeat(64),
        limits: RuntimeRunLimits {
            max_turns: 5,
            max_model_calls: 5,
            max_tool_calls: 4,
            max_repeated_tool_calls: 2,
            max_tool_call_depth: 1,
            max_no_progress_turns: 2,
            max_context_refreshes: 5,
            max_events: 128,
            max_elapsed_ms: 60000,
            max_output_bytes: 4096,
        },
        event_cursor: None,
        request_sha256: "0".repeat(64),
    })
    .unwrap()
}
fn bind(
    request: RuntimeEndpointRunRequest,
    workspace: &LinuxAuthorizedWorkspace,
    registry: &ToolRegistry,
    exclusions: &[&str],
) -> Result<BoundEndpointCodingPolicy, CodingApprovalError> {
    bind_endpoint_coding_policy(EndpointCodingPolicyRequest {
        request,
        actor_id: &ActorId::from_raw("endpoint-policy-actor"),
        workspace,
        registry,
        excluded_scopes: exclusions
            .iter()
            .map(|path| {
                WorkspaceScopePath::new(workspace.workspace_id().clone(), path.split('/')).unwrap()
            })
            .collect(),
    })
}

#[test]
fn endpoint_policy_preserves_original_framing_and_canonicalizes_all_exclusions() {
    let workspace = NativeWorkspace::new();
    let registry = read_registry();
    let original = request(&workspace.held, &registry);
    let bound = bind(
        original.clone(),
        &workspace.held,
        &registry,
        &["private", "generated", "private"],
    )
    .unwrap();
    let mut expected = original;
    expected.policy_id = bound.policy.policy_id().clone();
    expected.policy_sha256 = bound.policy.engine().policy_sha256().into();
    assert_eq!(
        bound.request,
        seal_runtime_endpoint_run_request(expected).unwrap()
    );
    verify_runtime_endpoint_run_request(&bound.request).unwrap();
    assert!(bound.policy.binds_run(
        &ActorId::from_raw("endpoint-policy-actor"),
        &bound.request.task.task_id,
        &bound.request.run_id,
        4
    ));
    assert_eq!(bound.policy.excluded_targets().len(), 2);
    assert!(
        !bound
            .policy
            .admits_action(&runtime_action_id(&bound.request.run_id, 0))
    );
    assert!(
        bound
            .policy
            .admits_action(&runtime_action_id(&bound.request.run_id, 4))
    );
    assert!(
        !bound
            .policy
            .admits_action(&runtime_action_id(&bound.request.run_id, 5))
    );
    let rebound = bind(
        bound.request.clone(),
        &workspace.held,
        &registry,
        &["generated", "private"],
    )
    .unwrap();
    assert_eq!(rebound.request, bound.request);
    let json = serde_json::to_value(&bound.request).unwrap();
    assert!(json.get("model_profile").is_none());
    assert!(json.get("verified_input_tokens").is_none());
}

#[test]
fn endpoint_policy_refuses_tampering_catalog_substitution_and_unsupported_modes() {
    let workspace = NativeWorkspace::new();
    let registry = read_registry();
    let original = request(&workspace.held, &registry);
    let mut changed = original.clone();
    changed.task.objective.push_str(" altered");
    assert!(bind(changed, &workspace.held, &registry, &[]).is_err());
    let mut changed = original.clone();
    changed.workspace_id = WorkspaceId::from_raw("other-workspace");
    assert!(
        bind(
            seal_runtime_endpoint_run_request(changed).unwrap(),
            &workspace.held,
            &registry,
            &[]
        )
        .is_err()
    );
    for mode in [
        RuntimeSessionMode::DurableReadOnly,
        RuntimeSessionMode::ControlledWrite,
    ] {
        let mut changed = original.clone();
        changed.mode = mode;
        changed.work_packet.budgets.extend(
            [
                (BudgetResource::ToolCallDepth, 1),
                (BudgetResource::InputBytes, 81920),
                (BudgetResource::OutputBytes, 4096),
                (BudgetResource::ElapsedMilliseconds, 60000),
                (BudgetResource::MemoryBytes, 1),
                (BudgetResource::DiskBytes, 4096),
                (BudgetResource::ProcessCount, 1),
            ]
            .into_iter()
            .map(|(resource, limit)| BudgetLimit { resource, limit }),
        );
        assert!(
            bind(
                seal_runtime_endpoint_run_request(changed).unwrap(),
                &workspace.held,
                &registry,
                &[]
            )
            .is_err()
        );
    }
    let mut changed = original.clone();
    changed.tool_catalog_sha256 = "f".repeat(64);
    assert!(seal_runtime_endpoint_run_request(changed.clone()).is_err());
    assert!(bind(changed, &workspace.held, &registry, &[]).is_err());
    let mut changed = original.clone();
    changed.visible_tools[0].definition_sha256 = "f".repeat(64);
    changed.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&changed.tool_catalog_id, &changed.visible_tools).unwrap();
    assert!(
        bind(
            seal_runtime_endpoint_run_request(changed).unwrap(),
            &workspace.held,
            &registry,
            &[]
        )
        .is_err()
    );
    let mut replacement = read_only_tool_definition(ReadOnlyToolKind::ReadText);
    replacement.display_name.push_str(" changed");
    let substitute = super::tests::registry(replacement);
    assert!(bind(original.clone(), &workspace.held, &substitute, &[]).is_err());
    let empty = ToolRegistry::new();
    assert!(
        bind(
            request(&workspace.held, &empty),
            &workspace.held,
            &empty,
            &[]
        )
        .is_err()
    );
    let mut changed = original;
    changed.event_cursor = Some(RuntimeEventCursor {
        run_id: changed.run_id.clone(),
        event_id: RuntimeEventId::from_raw("endpoint-policy-prior-event"),
        sequence: 1,
        event_sha256: "e".repeat(64),
    });
    assert!(seal_runtime_endpoint_run_request(changed.clone()).is_err());
    assert!(bind(changed, &workspace.held, &registry, &[]).is_err());
}

#[test]
fn endpoint_policy_denies_non_read_catalog_even_when_its_digest_is_exact() {
    let workspace = NativeWorkspace::new();
    let mut definition = read_only_tool_definition(ReadOnlyToolKind::ReadText);
    definition.required_grant.operation = OperationBinding::new(GrantOperation::WorkspaceWrite);
    definition.declared_effects = vec![OperationBinding::new(GrantOperation::WorkspaceWrite)];
    let registry = registry(definition);
    assert!(
        bind(
            request(&workspace.held, &registry),
            &workspace.held,
            &registry,
            &[]
        )
        .is_err()
    );
}

#[test]
fn endpoint_policy_native_targets_use_kernel_approval_and_single_use_grants() {
    native_grant_cycle(false);
}

#[test]
fn endpoint_policy_failed_native_grant_consumption_cannot_be_retried() {
    native_grant_cycle(true);
}

fn native_grant_cycle(reject_first: bool) {
    let workspace = NativeWorkspace::new();
    let registry = read_registry();
    let bound = bind(
        request(&workspace.held, &registry),
        &workspace.held,
        &registry,
        &["private"],
    )
    .unwrap();
    let policy = &bound.policy;
    let actor = ActorId::from_raw("endpoint-policy-actor");
    let definition = registry.list_tools()[0];
    let bytes = br#"{"schema_version":1,"paths":[["src","lib.rs"]],"query":null,"byte_offset":0,"byte_count":24,"encoding":"utf8","limits":{"files":64,"depth":16,"total_bytes":8388608,"file_bytes":4194304,"matches":10000},"call_depth":0}"#.to_vec();
    let call = ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("endpoint-policy-call"),
        correlation_id: CorrelationId::from_raw("endpoint-policy-correlation"),
        action_id: runtime_action_id(&bound.request.run_id, 1),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".into(),
            sha256: sha256(&bytes),
            bytes,
        },
    };
    let mut issuer = GrantIssuer::new();
    let parent = issuer
        .issue_session_read(SessionReadGrantRequest {
            grant_id: GrantId::from_raw("endpoint-policy-parent"),
            actor_id: actor.clone(),
            session_id: bound.request.session_id.clone(),
            task_id: bound.request.task.task_id.clone(),
            targets: policy.parent_targets().to_vec(),
            excluded_targets: policy.excluded_targets().to_vec(),
            sensitivity: DataSensitivity::Ephemeral,
            issued_at_epoch_ms: 1000,
            expires_at_epoch_ms: 20000,
            nonce: GrantNonce::from_raw("endpoint-policy-parent-nonce"),
            maximum_derived_operations: 4,
            preview_sha256: "d".repeat(64),
            policy_sha256: policy.engine().policy_sha256().into(),
        })
        .unwrap();
    let approval = render_coding_approval_request(
        &registry,
        CodingApprovalRequest {
            parent: &parent,
            approval_id: ApprovalId::from_raw("endpoint-policy-approval"),
            proposed_grant_id: GrantId::from_raw("endpoint-policy-child"),
            call: &call,
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            targets: vec![workspace.target(&["src", "lib.rs"])],
            operation_plan_sha256: &"e".repeat(64),
            issued_at_epoch_ms: 2000,
            expires_at_epoch_ms: 10000,
        },
    )
    .unwrap();
    // Excluded native objects cannot enter even the original protected preview.
    assert!(
        render_coding_approval_request(
            &registry,
            CodingApprovalRequest {
                parent: &parent,
                approval_id: ApprovalId::from_raw("excluded-approval"),
                proposed_grant_id: GrantId::from_raw("excluded-child"),
                call: &call,
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                targets: vec![workspace.target(&["private", "secret.txt"])],
                operation_plan_sha256: &"e".repeat(64),
                issued_at_epoch_ms: 2000,
                expires_at_epoch_ms: 10000,
            }
        )
        .is_err()
    );
    let challenge = seal_runtime_approval_challenge(RuntimeApprovalChallenge {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: bound.request.run_id.clone(),
        task_id: bound.request.task.task_id.clone(),
        turn_id: RuntimeTurnId::from_raw("endpoint-policy-turn"),
        operation_id: RuntimeOperationId::from_raw("endpoint-policy-operation"),
        tool_call_id: call.tool_call_id.clone(),
        approval_id: approval.approval_id.clone(),
        proposed_grant_id: approval.proposed_grant_id.clone(),
        operation: GrantOperation::WorkspaceRead,
        presentation: RuntimeApprovalPresentation {
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            display_name: definition.display_name.clone(),
            risk_level: definition.risk_level,
            target_scope: definition.required_grant.target_scope.clone(),
            arguments: call.arguments.clone(),
            declared_effects: definition.declared_effects.clone(),
            single_use: definition.required_grant.single_use,
            timeout_ms: definition.timeout_ms,
        },
        preview_sha256: approval.confirmation_sha256.clone(),
        expires_at_epoch_ms: approval.expires_at_epoch_ms,
        challenge_sha256: "0".repeat(64),
    })
    .unwrap();
    let response = RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: challenge.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Allow,
        challenge_sha256: challenge.challenge_sha256.clone(),
        grant_id: Some(challenge.proposed_grant_id.clone()),
        selection: None,
    };
    verify_coding_decision(&approval, &challenge, &response, 3000).unwrap();
    let mut denied = response.clone();
    denied.disposition = RuntimeApprovalDisposition::Deny;
    denied.grant_id = None;
    assert!(verify_coding_decision(&approval, &challenge, &denied, 3000).is_err());
    let mut foreign = response.clone();
    foreign.grant_id = Some(GrantId::from_raw("other-grant"));
    assert!(verify_coding_decision(&approval, &challenge, &foreign, 3000).is_err());
    let mut foreign = response.clone();
    foreign.challenge_sha256 = "f".repeat(64);
    assert!(verify_coding_decision(&approval, &challenge, &foreign, 3000).is_err());
    let child = issuer
        .derive_operation(
            &parent.grant_id,
            operation_grant_request(
                &approval,
                GrantNonce::from_raw("endpoint-policy-child-nonce"),
                3000,
                9000,
            )
            .unwrap(),
        )
        .unwrap();
    let context = PolicyEvaluationContext {
        actor_id: actor,
        session_id: child.session_id.clone(),
        task_id: bound.request.task.task_id.clone(),
        action_id: call.action_id.clone(),
        action_kind: ActionKind::DeterministicTool,
        tool_id: call.tool_id.clone(),
        tool_version: call.tool_version.clone(),
        targets: child.targets.clone(),
        argument_sha256: child.argument_sha256.clone(),
        preimages: child.preimages.clone(),
        expected_side_effects: child.expected_side_effects.clone(),
        preview_sha256: child.preview_sha256.clone(),
        now_epoch_ms: 4000,
        network_scope: None,
        credential_scope: None,
        publication_scope: None,
    };
    for alter in 0..12 {
        let mut invalid = context.clone();
        match alter {
            0 => invalid.action_id = runtime_action_id(&bound.request.run_id, 5),
            1 => invalid.actor_id = ActorId::from_raw("other-actor"),
            2 => invalid.session_id = SessionId::from_raw("other-session"),
            3 => invalid.argument_sha256 = "f".repeat(64),
            4 => invalid.now_epoch_ms = 9000,
            5 => invalid.targets = vec![workspace.target(&["private", "secret.txt"])],
            6 => invalid.preimages.clear(),
            7 => invalid.network_scope = Some("https:example.invalid:443".into()),
            8 => invalid.credential_scope = Some("credential:synthetic".into()),
            9 => invalid.publication_scope = Some("destination:synthetic".into()),
            10 => invalid.task_id = TaskId::from_raw("other-task"),
            _ => invalid.tool_version = "9.9.9".into(),
        }
        assert!(
            !policy.engine().evaluate(&issuer, &child, &invalid).allowed,
            "probe {alter}"
        );
        assert_eq!(
            issuer.current(&child.grant_id).unwrap().status,
            GrantStatus::Issued
        );
    }
    if reject_first {
        let mut invalid = context.clone();
        invalid.argument_sha256 = "f".repeat(64);
        assert!(
            issuer
                .consume_for_execution(&child.grant_id, policy.engine(), &invalid)
                .is_err()
        );
        assert_eq!(
            issuer.current(&child.grant_id).unwrap().status,
            GrantStatus::Invalidated
        );
        assert!(
            issuer
                .consume_for_execution(&child.grant_id, policy.engine(), &context)
                .is_err()
        );
        assert_eq!(
            issuer.current(&child.grant_id).unwrap().status,
            GrantStatus::Invalidated
        );
        return;
    }
    let consumed = issuer
        .consume_for_execution(&child.grant_id, policy.engine(), &context)
        .unwrap();
    assert_eq!(consumed.consumed_revision, child.revision + 1);
    assert!(
        issuer
            .consume_for_execution(&child.grant_id, policy.engine(), &context)
            .is_err()
    );
    assert_eq!(
        issuer.current(&child.grant_id).unwrap().status,
        GrantStatus::Consumed
    );
    assert_eq!(
        fs::read(workspace.root.join("src/lib.rs")).unwrap(),
        b"fn answer() -> u8 { 42 }\n"
    );
}
