// SPDX-License-Identifier: Apache-2.0
//! Held-root preparation and provenance tests, without native worker/model claims.

use super::*;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::instruction_provenance::{
    GuidanceConstraint, GuidanceConstraintKind, InstructionDiscoveryInput, InstructionLocation,
    InstructionScope, InstructionSourceKind, InstructionTrustDisposition, build_instruction_ledger,
    record_instruction_discovery, record_instruction_read, record_instruction_trust_decision,
};
use agentmage_platform_linux::select_test_linux_workspace;
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

type Error = CodingEndpointStartupError;
#[path = "coding_endpoint_read_verifier_tests.rs"]
mod read_verifier_tests;
struct Fixture {
    root: PathBuf,
    workspace: Arc<LinuxAuthorizedWorkspace>,
    request: RuntimeEndpointRunRequest,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "agentmage-endpoint-startup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("sentinel"), b"unchanged synthetic source").unwrap();
        let workspace = Arc::new(
            select_test_linux_workspace(
                &root,
                WorkspaceId::from_raw("endpoint-startup-workspace"),
                WorkspaceAuthorizationId::from_raw("endpoint-startup-authorization"),
                AdapterInstanceId::from_raw("endpoint-startup-adapter"),
            )
            .unwrap(),
        );
        let request = request(&workspace, &workspace_file_read_runtime_registry().unwrap());
        Self {
            root,
            workspace,
            request,
        }
    }
    fn ledger(&self, constraints: Vec<GuidanceConstraint>) -> InstructionEvidenceLedger {
        let workspace_id = self.request.workspace_id.clone();
        let discovery = record_instruction_discovery(InstructionDiscoveryInput {
            discovery_id: "startup-instruction-discovery".into(),
            source_kind: InstructionSourceKind::RepositoryInstruction,
            location: InstructionLocation {
                workspace_path: Some(
                    WorkspacePath::new(workspace_id.clone(), ["AGENTS.md"]).unwrap(),
                ),
                source_identity_sha256: "c".repeat(64),
                revision_sha256: "d".repeat(64),
                line_start: Some(1),
                line_end: Some(1),
            },
            workspace_manifest_sha256: self.request.workspace_snapshot_sha256.clone(),
            freshness_sha256: "e".repeat(64),
        })
        .unwrap();
        let read = record_instruction_read(
            &discovery,
            &"e".repeat(64),
            b"Synthetic narrowing; never executable instructions.",
        )
        .unwrap();
        let decision = record_instruction_trust_decision(
            "startup-trust-decision".into(),
            &read,
            InstructionScope {
                workspace_id: workspace_id.clone(),
                path: WorkspaceScopePath::new(workspace_id, ["private"]).unwrap(),
            },
            100,
            InstructionTrustDisposition::TrustForNarrowing,
            constraints,
            "f".repeat(64),
        )
        .unwrap();
        build_instruction_ledger(
            self.request.workspace_snapshot_sha256.clone(),
            vec![discovery],
            vec![read.clone()],
            vec![decision],
            &BTreeMap::from([(read.record_sha256, "e".repeat(64))]),
        )
        .unwrap()
    }
    fn empty_ledger(&self) -> InstructionEvidenceLedger {
        build_instruction_ledger(
            self.request.workspace_snapshot_sha256.clone(),
            vec![],
            vec![],
            vec![],
            &BTreeMap::new(),
        )
        .unwrap()
    }
    fn prepare(&self, ledger: &InstructionEvidenceLedger) -> Result<CodingEndpointStartup, Error> {
        CodingEndpointStartup::prepare(
            self.request.clone(),
            self.workspace.clone(),
            ActorId::from_raw("endpoint-startup-actor"),
            ledger,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn constraint(kind: GuidanceConstraintKind, target: &str) -> GuidanceConstraint {
    GuidanceConstraint {
        kind,
        target: target.into(),
    }
}

#[test]
fn endpoint_startup_binds_explicit_empty_provenance_without_native_profile_or_effect() {
    let f = Fixture::new();
    let mut expected = f.request.clone();
    let ledger = f.empty_ledger();
    let prepared = f.prepare(&ledger).unwrap();
    assert!(Arc::ptr_eq(&prepared.workspace, &f.workspace));
    assert!(prepared.guidance().verify());
    assert_eq!(prepared.guidance().limits(), &f.request.limits);
    assert_eq!(prepared.request().visible_tools, f.request.visible_tools);
    assert!(prepared.bound.policy.excluded_targets().is_empty());
    assert!(prepared.bound.policy.binds_run(
        &ActorId::from_raw("endpoint-startup-actor"),
        &f.request.task.task_id,
        &f.request.run_id,
        f.request.limits.max_tool_calls
    ));
    assert_eq!(
        prepared.bound.policy.engine().policy_sha256(),
        prepared.request().policy_sha256
    );
    expected.task.constraints.push(format!(
        "Trusted repository narrowing policy SHA-256: {}",
        prepared.guidance().policy_sha256()
    ));
    expected.policy_id = prepared.request().policy_id.clone();
    expected.policy_sha256 = prepared.request().policy_sha256.clone();
    assert_eq!(
        seal_runtime_endpoint_run_request(expected).unwrap(),
        *prepared.request()
    );
    assert!(prepared.request().event_cursor.is_none());
    assert_eq!(
        fs::read(f.root.join("sentinel")).unwrap(),
        b"unchanged synthetic source"
    );
    assert_eq!(fs::read_dir(&f.root).unwrap().count(), 1);
}

#[test]
fn endpoint_startup_applies_original_budget_caps_before_policy_action_binding() {
    let f = Fixture::new();
    let ledger = f.ledger(vec![
        constraint(GuidanceConstraintKind::ReduceBudget, "max_tool_calls=2"),
        constraint(GuidanceConstraintKind::ReduceBudget, "max_model_calls=3"),
        constraint(GuidanceConstraintKind::ReduceBudget, "max_elapsed_ms=10000"),
    ]);
    let prepared = f.prepare(&ledger).unwrap();
    assert_eq!(prepared.request().limits.max_tool_calls, 2);
    assert_eq!(prepared.request().limits.max_model_calls, 3);
    assert_eq!(prepared.request().limits.max_elapsed_ms, 10000);
    assert_eq!(prepared.request().limits, *prepared.guidance().limits());
    assert!(prepared.bound.policy.binds_run(
        &ActorId::from_raw("endpoint-startup-actor"),
        &prepared.request().task.task_id,
        &prepared.request().run_id,
        2
    ));
    assert!(!prepared.bound.policy.binds_run(
        &ActorId::from_raw("endpoint-startup-actor"),
        &prepared.request().task.task_id,
        &prepared.request().run_id,
        4
    ));
}

#[test]
fn endpoint_startup_refuses_stop_disabled_tool_unavailable_validation_and_widening() {
    let f = Fixture::new();
    let tool = &f.request.visible_tools[0];
    let target = format!("{}@{}", tool.tool_id.as_str(), tool.tool_version);
    for (constraint, expected) in [
        (
            constraint(GuidanceConstraintKind::Stop, "stop"),
            Error::Guidance(CodingGuidanceError::StopRequested),
        ),
        (
            constraint(GuidanceConstraintKind::DisableTool, &target),
            Error::NoTools,
        ),
        (
            constraint(GuidanceConstraintKind::RequireVerification, "project-tests"),
            Error::Guidance(CodingGuidanceError::ConstraintDenied),
        ),
        (
            constraint(GuidanceConstraintKind::ReduceBudget, "max_tool_calls=5"),
            Error::Guidance(CodingGuidanceError::ConstraintDenied),
        ),
        (
            constraint(GuidanceConstraintKind::ReduceBudget, "max_tool_calls=0"),
            Error::Guidance(CodingGuidanceError::ConstraintDenied),
        ),
        (
            constraint(GuidanceConstraintKind::ReduceBudget, "unknown=1"),
            Error::Guidance(CodingGuidanceError::ConstraintDenied),
        ),
        (
            constraint(GuidanceConstraintKind::DisableTool, "absent@1"),
            Error::Guidance(CodingGuidanceError::ConstraintDenied),
        ),
    ] {
        assert_eq!(f.prepare(&f.ledger(vec![constraint])).err(), Some(expected));
    }
}

#[test]
fn endpoint_startup_refuses_invalid_stale_foreign_and_conflicting_instruction_provenance() {
    let f = Fixture::new();
    let mut corrupt = f.empty_ledger();
    corrupt.ledger_sha256 = "0".repeat(64);
    assert_eq!(f.prepare(&corrupt).err(), Some(Error::Instructions));
    let foreign =
        build_instruction_ledger("b".repeat(64), vec![], vec![], vec![], &BTreeMap::new()).unwrap();
    assert_eq!(f.prepare(&foreign).err(), Some(Error::Instructions));
    let ledger = f.ledger(vec![constraint(
        GuidanceConstraintKind::ExcludeScope,
        "private",
    )]);
    let stale = build_instruction_ledger(
        ledger.workspace_manifest_sha256.clone(),
        ledger.discoveries.clone(),
        ledger.reads.clone(),
        ledger.decisions.clone(),
        &BTreeMap::from([(ledger.reads[0].record_sha256.clone(), "a".repeat(64))]),
    )
    .unwrap();
    assert!(!stale.stale_read_records.is_empty());
    assert_eq!(f.prepare(&stale).err(), Some(Error::Instructions));
    let opposite = record_instruction_trust_decision(
        "opposite-decision".into(),
        &ledger.reads[0],
        ledger.decisions[0].scope.clone(),
        100,
        InstructionTrustDisposition::KeepUntrusted,
        vec![],
        "e".repeat(64),
    )
    .unwrap();
    let conflict = build_instruction_ledger(
        ledger.workspace_manifest_sha256.clone(),
        ledger.discoveries.clone(),
        ledger.reads.clone(),
        vec![ledger.decisions[0].clone(), opposite],
        &BTreeMap::from([(ledger.reads[0].record_sha256.clone(), "e".repeat(64))]),
    )
    .unwrap();
    assert!(!conflict.conflicts.is_empty());
    assert_eq!(f.prepare(&conflict).err(), Some(Error::Instructions));
}

#[test]
fn endpoint_startup_keeps_scoped_exclusions_out_of_sources_and_original_kernel_policy() {
    let f = Fixture::new();
    let prepared = f
        .prepare(&f.ledger(vec![constraint(
            GuidanceConstraintKind::ExcludeScope,
            "private",
        )]))
        .unwrap();
    let excluded = WorkspaceScopePath::new(f.request.workspace_id.clone(), ["private"]).unwrap();
    assert_eq!(prepared.guidance().excluded_scopes(), [excluded]);
    assert_eq!(prepared.bound.policy.excluded_targets().len(), 1);
    assert_eq!(
        prepared.bound.policy.excluded_targets()[0].path_components()[0].as_str(),
        "private"
    );
    let source = |path: &str| CodingSource {
        path: path.into(),
        content: "untrusted data".into(),
    };
    for path in [
        "private",
        "private/secret",
        "private/nested/source.rs",
        "../source",
        "src//lib.rs",
        "/absolute",
    ] {
        assert_eq!(prepared.check_sources(&[source(path)]), Err(Error::Source));
    }
    assert!(
        prepared
            .check_sources(&[source("private-neighbor/lib.rs"), source("src/lib.rs")])
            .is_ok()
    );
    assert!(prepared.check_sources(&[]).is_ok());
    assert_eq!(
        prepared.check_sources(&[source("src/lib.rs"), source("private/secret")]),
        Err(Error::Source)
    );
    assert_ne!(prepared.request().request_sha256, f.request.request_sha256);
}

#[test]
fn endpoint_startup_checks_original_selection_before_resealing_and_refuses_zero_calls() {
    let f = Fixture::new();
    let ledger = f.empty_ledger();
    let prepare = |request| {
        CodingEndpointStartup::prepare(
            request,
            f.workspace.clone(),
            ActorId::from_raw("endpoint-startup-actor"),
            &ledger,
        )
        .err()
    };
    let mut request = f.request.clone();
    request.request_sha256 = "0".repeat(64);
    assert_eq!(prepare(request), Some(Error::Binding));
    let mut request = f.request.clone();
    request.workspace_id = WorkspaceId::from_raw("foreign-workspace");
    assert_eq!(
        prepare(seal_runtime_endpoint_run_request(request).unwrap()),
        Some(Error::Binding)
    );
    let mut request = f.request.clone();
    request.visible_tools =
        runtime_tool_references(&crate::runtime_tools::read_only_runtime_registry().unwrap())
            .unwrap();
    request.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools).unwrap();
    assert_eq!(
        prepare(seal_runtime_endpoint_run_request(request).unwrap()),
        Some(Error::Binding)
    );
    let mut request = f.request.clone();
    request.limits.max_tool_calls = 0;
    assert_eq!(
        prepare(seal_runtime_endpoint_run_request(request).unwrap()),
        Some(Error::NoTools)
    );
    let mut request = f.request.clone();
    request.mode = RuntimeSessionMode::DurableReadOnly;
    assert_eq!(
        prepare(seal_runtime_endpoint_run_request(request).unwrap()),
        Some(Error::Binding)
    );
}

#[test]
fn endpoint_startup_original_compiler_and_confirmation_narrowing_remain_identical() {
    let f = Fixture::new();
    let tool = &f.request.visible_tools[0];
    let ledger = f.ledger(vec![constraint(
        GuidanceConstraintKind::RequireUserConfirmation,
        &format!("{}@{}", tool.tool_id.as_str(), tool.tool_version),
    )]);
    let prepared = f.prepare(&ledger).unwrap();
    assert!(
        prepared
            .guidance()
            .requires_confirmation(&tool.tool_id, &tool.tool_version)
    );
    let original = crate::coding_guidance::compile_coding_guidance(
        &f.request.workspace_id,
        &ledger,
        &effective_guidance(&ledger).unwrap(),
        &workspace_file_read_runtime_registry().unwrap(),
        &crate::coding_recipe::test_registry(&f.request.workspace_id),
        &f.request.limits,
    )
    .unwrap();
    assert_eq!(prepared.guidance(), &original);
    assert_eq!(prepared.request().visible_tools, f.request.visible_tools);
    assert_eq!(prepared.request().limits, f.request.limits);
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
