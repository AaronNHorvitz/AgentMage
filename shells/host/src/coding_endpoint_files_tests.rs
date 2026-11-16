// SPDX-License-Identifier: MIT OR Apache-2.0
//! Real-descriptor and durable-authority components; no model score or native admission claim.

use super::*;
use crate::{
    coding_authority::{EndpointCodingPolicyRequest, bind_endpoint_coding_policy},
    linux_coding_runtime::LinuxCodingRuntimeError,
    runtime_tools::workspace_file_read_runtime_registry,
};
use agentmage_capability_read_only::{ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyRequest};
use agentmage_kernel_engine::{
    operational_store::{OperationalStoreKeyError, OperationalStoreKeyProvider},
    runtime_coordinator::{
        runtime_tool_catalog_sha256, seal_runtime_approval_challenge,
        seal_runtime_endpoint_run_request,
    },
    runtime_loop::{runtime_action_id, runtime_tool_references},
};
use agentmage_platform_linux::{open_test_linux_authority, select_test_linux_workspace};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Key;
impl OperationalStoreKeyProvider for Key {
    fn with_key<T>(
        &mut self,
        operation: impl FnOnce(&[u8]) -> T,
    ) -> Result<T, OperationalStoreKeyError> {
        Ok(operation(&[29; 32]))
    }
}
#[derive(Default)]
struct Ids(u32);
impl CodingIdentitySource for Ids {
    fn next(&mut self, prefix: &str) -> Result<String, LinuxCodingRuntimeError> {
        self.0 += 1;
        Ok(format!("{prefix}-{}", self.0))
    }
}
struct Fixture {
    root: PathBuf,
    boundary: EndpointFileGrants<Ids>,
    request: RuntimeEndpointRunRequest,
    definition: ToolDefinition,
    call: ToolCall,
    operation: RuntimeOperationId,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
impl Fixture {
    fn new() -> Self {
        Self::modified(|_| {}).unwrap()
    }
    fn modified(
        change: impl FnOnce(&mut FileGrantsInput<Ids>),
    ) -> Result<Self, RuntimePortFailure> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "agentmage-endpoint-files-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("workspace")).unwrap();
        fs::create_dir(root.join("workspace/src")).unwrap();
        fs::create_dir(root.join("workspace/private")).unwrap();
        fs::write(
            root.join("workspace/src/lib.rs"),
            b"pub fn answer() -> u8 { 42 }\n",
        )
        .unwrap();
        fs::write(
            root.join("workspace/private/secret"),
            b"synthetic excluded content",
        )
        .unwrap();
        fs::create_dir(root.join("state")).unwrap();
        fs::set_permissions(root.join("state"), fs::Permissions::from_mode(0o700)).unwrap();
        let workspace = Arc::new(
            select_test_linux_workspace(
                &root.join("workspace"),
                WorkspaceId::from_raw("endpoint-policy-workspace"),
                WorkspaceAuthorizationId::from_raw("endpoint-files-authorization"),
                AdapterInstanceId::from_raw("endpoint-files-adapter"),
            )
            .unwrap(),
        );
        let registry = workspace_file_read_runtime_registry().unwrap();
        let original = request(&workspace, &registry);
        let actor = ActorId::from_raw("endpoint-files-actor");
        let bound = bind_endpoint_coding_policy(EndpointCodingPolicyRequest {
            request: original,
            actor_id: &actor,
            workspace: &*workspace,
            registry: &registry,
            excluded_scopes: vec![
                WorkspaceScopePath::new(workspace.workspace_id().clone(), ["private"]).unwrap(),
            ],
        })
        .unwrap();
        let request = bound.request.clone();
        let authority = open_test_linux_authority(&root.join("state"), &mut Key, 1).unwrap();
        // Native descriptors and durable authority only. This component
        // owner cannot launch a worker or satisfy platform admission.
        let mut input = FileGrantsInput {
            bound,
            workspace,
            authority,
            registry,
            actor_id: actor,
            identities: Ids::default(),
        };
        change(&mut input);
        let boundary = match EndpointFileGrants::new(input) {
            Ok(boundary) => boundary,
            Err(error) => {
                fs::remove_dir_all(&root).unwrap();
                return Err(error);
            }
        };
        let definition = read_only_tool_definition(ReadOnlyToolKind::ReadText);
        let call = call(&request, &definition, &["src", "lib.rs"], 1);
        Ok(Self {
            root,
            boundary,
            request,
            definition,
            call,
            operation: RuntimeOperationId::from_raw("endpoint-files-operation"),
        })
    }
    fn evaluate(&mut self) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.boundary.evaluate_file(
            RuntimeRunScope::endpoint(&self.request).unwrap(),
            &self.operation,
            &self.definition,
            &self.call,
            1000,
        )
    }
    fn challenge(&mut self) -> RuntimeApprovalChallenge {
        let RuntimePermissionEvaluation::Ask {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
        } = self.evaluate().unwrap()
        else {
            panic!("expected exact ask")
        };
        seal_runtime_approval_challenge(RuntimeApprovalChallenge {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: self.request.run_id.clone(),
            task_id: self.request.task.task_id.clone(),
            turn_id: RuntimeTurnId::from_raw("endpoint-files-turn"),
            operation_id: self.operation.clone(),
            tool_call_id: self.call.tool_call_id.clone(),
            approval_id,
            proposed_grant_id: grant_id,
            operation: GrantOperation::WorkspaceRead,
            presentation: RuntimeApprovalPresentation {
                tool_id: self.definition.tool_id.clone(),
                tool_version: self.definition.tool_version.clone(),
                display_name: self.definition.display_name.clone(),
                risk_level: self.definition.risk_level,
                target_scope: self.definition.required_grant.target_scope.clone(),
                arguments: self.call.arguments.clone(),
                declared_effects: self.definition.declared_effects.clone(),
                single_use: true,
                timeout_ms: self.definition.timeout_ms,
            },
            preview_sha256,
            expires_at_epoch_ms,
            challenge_sha256: "0".repeat(64),
        })
        .unwrap()
    }
    fn response(
        &self,
        challenge: &RuntimeApprovalChallenge,
        disposition: RuntimeApprovalDisposition,
    ) -> RuntimeApprovalResponse {
        RuntimeApprovalResponse {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: challenge.run_id.clone(),
            approval_id: challenge.approval_id.clone(),
            challenge_sha256: challenge.challenge_sha256.clone(),
            disposition,
            grant_id: (disposition == RuntimeApprovalDisposition::Allow)
                .then(|| challenge.proposed_grant_id.clone()),
            selection: None,
        }
    }

    fn resolve(
        &mut self,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.boundary.resolve_file(
            RuntimeRunScope::endpoint(&self.request).unwrap(),
            challenge,
            response,
            &self.definition,
            &self.call,
            1100,
        )
    }
}
fn call(
    request: &RuntimeEndpointRunRequest,
    definition: &ToolDefinition,
    path: &[&str],
    seq: u32,
) -> ToolCall {
    let bytes = serde_json::to_vec(&ReadOnlyRequest {
        schema_version: 1,
        paths: vec![path.iter().map(|value| (*value).to_owned()).collect()],
        query: None,
        byte_offset: Some(0),
        byte_count: Some(128),
        encoding: ReadOnlyEncoding::Utf8,
        limits: ReadOnlyLimits::default(),
        call_depth: 0,
    })
    .unwrap();
    ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw(format!("endpoint-files-call-{seq}")),
        correlation_id: CorrelationId::from_raw("endpoint-files-correlation"),
        action_id: runtime_action_id(&request.run_id, seq),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".into(),
            sha256: hash(&bytes),
            bytes,
        },
    }
}

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
#[test]
fn native_endpoint_file_preview_holds_exact_preimage_and_refuses_replay() {
    let mut f = Fixture::new();
    let challenge = f.challenge();
    let pending = f.boundary.pending.as_ref().unwrap();
    assert_eq!(pending.approval.targets.len(), 1);
    assert_eq!(
        pending.approval.targets[0]
            .path_components()
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>(),
        ["src", "lib.rs"]
    );
    assert_eq!(pending.approval.preimages.len(), 1);
    assert!(f.boundary.authority.authority().receipts().is_empty());
    assert!(f.evaluate().is_err());
    let response = f.response(&challenge, RuntimeApprovalDisposition::Allow);
    let allowed = f.resolve(&challenge, &response).unwrap();
    assert!(matches!(allowed, RuntimePermissionEvaluation::Allow { .. }));
    assert!(f.resolve(&challenge, &response).is_err());
    assert!(f.evaluate().is_err());
    assert!(f.boundary.authority.authority().receipts().is_empty());
    assert_eq!(
        fs::read(f.root.join("workspace/src/lib.rs")).unwrap(),
        b"pub fn answer() -> u8 { 42 }\n"
    );
}

#[test]
fn native_endpoint_file_refuses_foreign_request_definition_path_and_action() {
    let mut f = Fixture::new();
    let mut changed = f.request.clone();
    changed.task.objective.push_str(" changed");
    changed.work_packet.objective = changed.task.objective.clone();
    let changed = seal_runtime_endpoint_run_request(changed).unwrap();
    assert!(
        f.boundary
            .evaluate_file(
                RuntimeRunScope::endpoint(&changed).unwrap(),
                &f.operation,
                &f.definition,
                &f.call,
                1000
            )
            .is_err()
    );
    let mut definition = f.definition.clone();
    definition.display_name.push_str(" changed");
    assert!(
        f.boundary
            .evaluate_file(
                RuntimeRunScope::endpoint(&f.request).unwrap(),
                &f.operation,
                &definition,
                &f.call,
                1000
            )
            .is_err()
    );
    for (path, seq) in [
        (&["private", "secret"][..], 1),
        (&["absent"][..], 1),
        (&["src"][..], 1),
        (&["src", "lib.rs"][..], 5),
    ] {
        f.call = call(&f.request, &f.definition, path, seq);
        assert!(f.evaluate().is_err());
    }
    symlink("lib.rs", f.root.join("workspace/src/link.rs")).unwrap();
    f.call = call(&f.request, &f.definition, &["src", "link.rs"], 1);
    assert!(f.evaluate().is_err());
    f.call = call(&f.request, &f.definition, &["..", "outside"], 1);
    assert!(f.evaluate().is_err());
    assert!(f.boundary.spent.is_empty());
    assert!(f.boundary.pending.is_none());
    assert!(f.boundary.authority.authority().receipts().is_empty());
}

#[test]
fn native_endpoint_file_exact_response_denial_and_stale_preimage_fail_closed() {
    let mut f = Fixture::new();
    let challenge = f.challenge();
    let mut response = f.response(&challenge, RuntimeApprovalDisposition::Allow);
    response.challenge_sha256 = "f".repeat(64);
    assert!(f.resolve(&challenge, &response).is_err());
    assert!(f.boundary.pending.is_some());
    assert!(f.boundary.issued.is_none());
    let deny = f.response(&challenge, RuntimeApprovalDisposition::Deny);
    assert!(matches!(
        f.resolve(&challenge, &deny).unwrap(),
        RuntimePermissionEvaluation::Deny { .. }
    ));
    assert!(f.resolve(&challenge, &deny).is_err());
    assert!(f.evaluate().is_err());
    f.call = call(&f.request, &f.definition, &["src", "lib.rs"], 2);
    let challenge = f.challenge();
    fs::write(
        f.root.join("workspace/src/lib.rs"),
        b"concurrent replacement\n",
    )
    .unwrap();
    let allow = f.response(&challenge, RuntimeApprovalDisposition::Allow);
    assert!(f.resolve(&challenge, &allow).is_err());
    assert!(f.boundary.pending.is_none());
    assert!(f.boundary.issued.is_none());
    assert!(f.evaluate().is_err());
    assert!(f.boundary.authority.authority().receipts().is_empty());
}

#[test]
fn endpoint_file_grant_consumption_keeps_one_canonical_receipt() {
    let mut f = Fixture::new();
    let challenge = f.challenge();
    let response = f.response(&challenge, RuntimeApprovalDisposition::Allow);
    let allowed = f.resolve(&challenge, &response).unwrap();
    let scope = RuntimeRunScope::endpoint(&f.request).unwrap();
    let (issued, transaction, _held_input) = f
        .boundary
        .prepare_execution(scope, &allowed, &f.definition, &f.call)
        .unwrap();
    use agentmage_kernel_engine::authority_transaction::{
        EffectAuthorization, EffectDriver, EffectLaunch, EffectResult,
    };
    // Synthetic effect observation. No native worker/cancellation lifecycle.
    struct Observation(usize);
    impl EffectDriver for Observation {
        fn execute(&mut self, _: EffectAuthorization<'_>) -> EffectLaunch {
            self.0 += 1;
            EffectLaunch::completed(EffectResult::from_redacted_material(
                OperationOutcome::Cancelled,
                b"synthetic-cancellation",
                StateChange::NotChanged,
            ))
        }
    }
    let mut driver = Observation(0);
    let receipt = f
        .boundary
        .authority
        .authority_mut()
        .execute_effect(
            &f.boundary.registry,
            &issued.policy,
            transaction.clone(),
            &mut driver,
        )
        .unwrap();
    assert_eq!(driver.0, 1);
    let result = failed_read_projection(&receipt, &f.call, "runtime.tool.worker-failed");
    assert_eq!(result.result.outcome, OperationOutcome::Cancelled);
    assert_eq!(result.result.state_change, StateChange::NotChanged);
    assert!(result.result.output.is_none());
    assert_eq!(f.boundary.authority.authority().receipts().len(), 1);
    assert_eq!(
        f.boundary.authority.authority().receipts()[0].receipt_id,
        result.receipt_id
    );
    assert!(
        f.boundary
            .prepare_execution(scope, &allowed, &f.definition, &f.call)
            .is_err()
    );
    assert_eq!(f.boundary.authority.authority().receipts().len(), 1);
    assert!(f.boundary.issued.is_none());
    assert!(f.evaluate().is_err());
}

#[test]
fn native_endpoint_file_startup_refuses_substituted_policy_actor_and_catalog() {
    assert!(
        Fixture::modified(|input| input.actor_id = ActorId::from_raw("foreign-actor")).is_err()
    );
    assert!(
        Fixture::modified(|input| {
            input.registry = crate::runtime_tools::read_only_runtime_registry().unwrap();
        })
        .is_err()
    );
    assert!(
        Fixture::modified(|input| {
            input.bound.request.policy_sha256 = "f".repeat(64);
            input.bound.request =
                seal_runtime_endpoint_run_request(input.bound.request.clone()).unwrap();
        })
        .is_err()
    );
    assert!(
        Fixture::modified(|input| {
            input.bound.request.run_id = RuntimeRunId::from_raw("foreign-run");
            input.bound.request =
                seal_runtime_endpoint_run_request(input.bound.request.clone()).unwrap();
        })
        .is_err()
    );
    assert!(
        Fixture::modified(|input| input.bound.request.task.objective.push_str(" tampered"))
            .is_err()
    );
}

#[test]
fn native_endpoint_file_substituted_execution_and_expired_decision_never_dispatch() {
    let mut f = Fixture::new();
    let challenge = f.challenge();
    let response = f.response(&challenge, RuntimeApprovalDisposition::Allow);
    assert!(
        f.boundary
            .resolve_file(
                RuntimeRunScope::endpoint(&f.request).unwrap(),
                &challenge,
                &response,
                &f.definition,
                &f.call,
                challenge.expires_at_epoch_ms
            )
            .is_err()
    );
    assert!(f.boundary.issued.is_none());
    let allowed = f.resolve(&challenge, &response).unwrap();
    let mut foreign = allowed.clone();
    if let RuntimePermissionEvaluation::Allow {
        authority_sha256, ..
    } = &mut foreign
    {
        *authority_sha256 = "f".repeat(64);
    } else {
        panic!("expected allow")
    }
    assert!(
        f.boundary
            .prepare_execution(
                RuntimeRunScope::endpoint(&f.request).unwrap(),
                &foreign,
                &f.definition,
                &f.call,
            )
            .is_err()
    );
    assert!(f.boundary.issued.is_some());
    assert!(f.boundary.authority.authority().receipts().is_empty());
    fs::rename(
        f.root.join("workspace/src/lib.rs"),
        f.root.join("workspace/src/previous.rs"),
    )
    .unwrap();
    fs::write(f.root.join("workspace/src/lib.rs"), b"replacement inode\n").unwrap();
    assert!(
        f.boundary
            .prepare_execution(
                RuntimeRunScope::endpoint(&f.request).unwrap(),
                &allowed,
                &f.definition,
                &f.call,
            )
            .is_err()
    );
    assert!(f.boundary.issued.is_none());
    assert!(f.boundary.authority.authority().receipts().is_empty());
    assert!(f.evaluate().is_err());
}
