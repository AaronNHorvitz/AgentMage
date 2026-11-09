// SPDX-License-Identifier: Apache-2.0
//! Synthetic creation provenance; real native grants, descriptors and atomic writes.
use super::*;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::runtime_coordinator::{
    runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    sync::atomic::{AtomicU64, Ordering},
};

const NOW: u64 = 1000;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "agentmage-endpoint-edit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for name in ["state", "original", "owned", "owned/worktree"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        for root_name in ["original", "owned/worktree"] {
            for (path, content) in [("one.py", "first\n"), ("two.py", "second\n")] {
                fs::write(root.join(root_name).join(path), content).unwrap();
            }
        }
        Self(root)
    }
    fn owner(&self) -> LinuxCodingRepository {
        let mut owner = LinuxCodingRepository::open(
            &self.0.join("state"),
            b"synthetic-endpoint-edit-control-secret",
            NOW,
        )
        .unwrap();
        // Public construction requires successful native Git creation. This
        // fixture deliberately claims no native Git/model/workflow success.
        let path = self.0.join("owned/worktree");
        owner.created = Some(CreatedWorktree {
            original_path: self.0.join("original"),
            original: held_root(&self.0.join("original")).unwrap(),
            held: held_root(&path).unwrap(),
            path,
            owned: held_root(&self.0.join("owned")).unwrap(),
            creation_receipt_sha256: hash(b"synthetic-creation"),
            git_creation: None,
            edits: BTreeMap::new(),
        });
        owner
    }
    fn prepared(&self) -> LinuxCodingRepository {
        let mut owner = self.owner();
        let paths = vec!["one.py".into(), "two.py".into()];
        let scope = owner.select_created_sources(&paths).unwrap();
        owner
            .prepare_edit(
                scope,
                &[
                    LinuxCodingReplacement {
                        path: "one.py",
                        before_sha256: &hash(b"first\n"),
                        content: "repaired first\n",
                    },
                    LinuxCodingReplacement {
                        path: "two.py",
                        before_sha256: &hash(b"second\n"),
                        content: "repaired second\n",
                    },
                ],
                "Apply the prepared fixture edit",
                NOW,
            )
            .unwrap();
        owner
    }
    fn assert_original(&self) {
        assert_eq!(
            fs::read(self.0.join("original/one.py")).unwrap(),
            b"first\n"
        );
        assert_eq!(
            fs::read(self.0.join("original/two.py")).unwrap(),
            b"second\n"
        );
    }
    fn assert_unwritten(&self) {
        self.assert_original();
        assert_eq!(
            fs::read(self.0.join("owned/worktree/one.py")).unwrap(),
            b"first\n"
        );
        assert_eq!(
            fs::read(self.0.join("owned/worktree/two.py")).unwrap(),
            b"second\n"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn request(
    workspace: &LinuxAuthorizedWorkspace,
    registry: &ToolRegistry,
) -> RuntimeEndpointRunRequest {
    let task_id = TaskId::from_raw("endpoint-policy-task");
    let session_id = SessionId::from_raw("endpoint-policy-session");
    let objective = "Apply the prepared fixture edit";
    let check = "Ground observations in current source";
    let catalog_id = ToolCatalogId::from_raw("endpoint-policy-catalog");
    let visible_tools = runtime_tool_references(registry).unwrap();
    seal_runtime_endpoint_run_request(RuntimeEndpointRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("endpoint-policy-run"),
        session_id: session_id.clone(),
        mode: RuntimeSessionMode::ControlledWrite,
        task: Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: task_id.clone(),
            session_id,
            objective: objective.into(),
            acceptance_criteria: vec![check.into()],
            constraints: vec!["Only the complete approved diff; no network".into()],
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
            mutable_files: vec!["one.py".into(), "two.py".into()],
            protected_files: vec!["private".into()],
            expected_output: "Grounded source observations".into(),
            acceptance_checks: vec![check.into()],
            required_evidence: vec![EvidenceKind::ToolOutput],
            required_capability_class: AuthorityClass::LocalWrite,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::ToolCallDepth,
                    limit: 1,
                },
                BudgetLimit {
                    resource: BudgetResource::InputBytes,
                    limit: 81920,
                },
                BudgetLimit {
                    resource: BudgetResource::OutputBytes,
                    limit: 4096,
                },
                BudgetLimit {
                    resource: BudgetResource::ElapsedMilliseconds,
                    limit: 60000,
                },
                BudgetLimit {
                    resource: BudgetResource::MemoryBytes,
                    limit: 16777216,
                },
                BudgetLimit {
                    resource: BudgetResource::DiskBytes,
                    limit: 4096,
                },
                BudgetLimit {
                    resource: BudgetResource::ProcessCount,
                    limit: 1,
                },
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
fn call(owner: &LinuxCodingRepository, request: &RuntimeEndpointRunRequest) -> ToolCall {
    let definition = EndpointEditTool::new().0;
    let bytes =
        serde_json::to_vec(&json!({"preview":owner.inspect_edit().unwrap()["pending"]})).unwrap();
    ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("endpoint-edit-call"),
        correlation_id: CorrelationId::from_raw("endpoint-edit-correlation"),
        action_id: runtime_action_id(&request.run_id, 1),
        tool_id: definition.tool_id,
        tool_version: definition.tool_version,
        arguments: ContractPayload {
            schema: definition.input_schema,
            media_type: "application/json".into(),
            sha256: hash(&bytes),
            bytes,
        },
    }
}
fn handoff(owner: LinuxCodingRepository) -> LinuxCodingEndpointEdit {
    let registry = linux_coding_endpoint_edit_registry().unwrap();
    let request = request(&owner.created.as_ref().unwrap().held, &registry);
    let call = call(&owner, &request);
    owner
        .into_endpoint_edit(
            request,
            RuntimeTurnId::from_raw("endpoint-edit-turn"),
            RuntimeOperationId::from_raw("endpoint-edit-operation"),
            call,
        )
        .ok()
        .unwrap()
}
fn response(edit: &LinuxCodingEndpointEdit, allow: bool) -> RuntimeApprovalResponse {
    let c = edit.challenge();
    RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: c.run_id.clone(),
        approval_id: c.approval_id.clone(),
        disposition: if allow {
            RuntimeApprovalDisposition::Allow
        } else {
            RuntimeApprovalDisposition::Deny
        },
        challenge_sha256: c.challenge_sha256.clone(),
        grant_id: allow.then(|| c.proposed_grant_id.clone()),
        selection: None,
    }
}
struct Control(std::result::Result<std::num::NonZeroU64, ModelOperationStop>);
impl ModelOperationControl for Control {
    fn remaining_ms(&self) -> std::result::Result<std::num::NonZeroU64, ModelOperationStop> {
        self.0
    }
}
fn running() -> Control {
    Control(Ok(std::num::NonZeroU64::new(5000).unwrap()))
}

#[test]
fn endpoint_native_edit_exact_approval_runs_original_multifile_transaction_once() {
    let f = Fixture::new();
    let owner = f.prepared();
    let native = owner.inspect_edit().unwrap()["pending"].clone();
    let mut edit = handoff(owner);
    f.assert_unwritten();
    assert_eq!(
        edit.challenge().presentation.arguments,
        edit.call().arguments
    );
    assert_eq!(edit.challenge().preview_sha256, native["preview_sha256"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&edit.call().arguments.bytes).unwrap()["preview"],
        native
    );
    let r = response(&edit, true);
    let result = edit.decide(&r, NOW, &running()).unwrap();
    assert_eq!(result["outcome"], "committed");
    assert_eq!(edit.inspect().unwrap()["latest"], result);
    assert!(edit.inspect().unwrap()["pending"].is_null());
    let receipts = result["write_result"]["receipts"].as_array().unwrap();
    assert!(!receipts.is_empty());
    assert!(
        receipts
            .iter()
            .all(|r| r["grant_id"] == edit.challenge().proposed_grant_id.as_str())
    );
    let terminal: Vec<_> = receipts
        .iter()
        .filter(|r| r["status"] == "verified")
        .collect();
    assert_eq!(terminal.len(), 2, "{receipts:?}");
    assert_eq!(
        fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
        b"repaired first\n"
    );
    assert_eq!(
        fs::read(f.0.join("owned/worktree/two.py")).unwrap(),
        b"repaired second\n"
    );
    f.assert_original();
    assert!(edit.decide(&r, NOW, &running()).is_err());
    assert_eq!(edit.inspect().unwrap()["latest"], result);
    let owner = edit.into_repository();
    assert_eq!(owner.inspect_edit().unwrap()["latest"], result);
    assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    assert_eq!(owner.created.as_ref().unwrap().edits.len(), 2);
    assert!(
        owner
            .select_created_sources(&["one.py".into(), "two.py".into()])
            .is_ok()
    );
}

#[test]
fn endpoint_native_edit_denial_and_stopped_controls_never_write_or_reopen() {
    for stop in [
        None,
        Some(ModelOperationStop::Cancelled),
        Some(ModelOperationStop::TimedOut),
        Some(ModelOperationStop::Unavailable),
        Some(ModelOperationStop::Invalid),
    ] {
        let f = Fixture::new();
        let mut edit = handoff(f.prepared());
        let r = response(&edit, stop.is_some());
        let control = stop.map_or_else(running, |s| Control(Err(s)));
        let result = edit.decide(&r, NOW, &control);
        if stop.is_none() {
            assert_eq!(result.unwrap()["outcome"], "denied");
        } else {
            assert_eq!(result.unwrap_err().code(), "code.endpoint-edit.stopped");
        }
        f.assert_unwritten();
        let allow = response(&edit, true);
        assert!(edit.decide(&allow, NOW, &running()).is_err());
        f.assert_unwritten();
    }
}

#[test]
fn endpoint_native_edit_rejects_altered_decisions_before_consuming_original() {
    for index in 0..8 {
        let f = Fixture::new();
        let mut edit = handoff(f.prepared());
        let original = response(&edit, true);
        let mut changed = original.clone();
        match index {
            0 => changed.schema_version += 1,
            1 => changed.run_id = RuntimeRunId::from_raw("other-run"),
            2 => changed.approval_id = ApprovalId::from_raw("other-approval"),
            3 => changed.challenge_sha256 = "f".repeat(64),
            4 => changed.grant_id = Some(GrantId::from_raw("other-grant")),
            5 => changed.grant_id = None,
            6 => changed.disposition = RuntimeApprovalDisposition::Narrow,
            _ => changed.disposition = RuntimeApprovalDisposition::Deny,
        }
        assert!(
            edit.decide(&changed, NOW, &running()).is_err(),
            "probe {index}"
        );
        f.assert_unwritten();
        assert!(!edit.spent);
        assert_eq!(
            edit.decide(&original, NOW, &running()).unwrap()["outcome"],
            "committed"
        );
        f.assert_original();
    }
}

#[test]
fn endpoint_native_edit_preserves_native_expiry_preimage_link_and_creation_refusals() {
    for case in [
        "expired",
        "backwards",
        "preimage",
        "symlink",
        "hardlink",
        "creation",
        "preview",
    ] {
        let f = Fixture::new();
        let mut edit = handoff(f.prepared());
        let r = response(&edit, true);
        let file = f.0.join("owned/worktree/one.py");
        let outside = f.0.join("outside");
        fs::write(&outside, b"human bytes").unwrap();
        match case {
            "preimage" => fs::write(&file, b"human bytes").unwrap(),
            "symlink" | "hardlink" => {
                fs::remove_file(&file).unwrap();
                if case == "symlink" {
                    symlink(&outside, &file).unwrap()
                } else {
                    fs::hard_link(&outside, &file).unwrap()
                }
            }
            "creation" => {
                edit.repository
                    .created
                    .as_mut()
                    .unwrap()
                    .creation_receipt_sha256 = hash(b"substitute")
            }
            "preview" => {
                edit.repository.edit.as_mut().unwrap().preview["preview_sha256"] =
                    json!(hash(b"substitute"))
            }
            _ => (),
        }
        let now = match case {
            "expired" => edit.challenge().expires_at_epoch_ms,
            "backwards" => NOW - 1,
            _ => NOW,
        };
        assert!(edit.decide(&r, now, &running()).is_err(), "{case}");
        if case != "expired" {
            assert!(edit.decide(&r, NOW, &running()).is_err(), "{case}");
        }
        f.assert_original();
        assert_eq!(fs::read(&outside).unwrap(), b"human bytes");
        assert_eq!(
            fs::read(f.0.join("owned/worktree/two.py")).unwrap(),
            b"second\n"
        );
        assert_eq!(
            fs::read(&file).unwrap(),
            if ["preimage", "symlink", "hardlink"].contains(&case) {
                &b"human bytes"[..]
            } else {
                &b"first\n"[..]
            }
        );
    }
}

#[test]
fn endpoint_native_edit_handoff_refuses_changed_scope_call_and_catalog_without_effects() {
    for index in 0..23 {
        let f = Fixture::new();
        let mut owner = f.prepared();
        let registry = linux_coding_endpoint_edit_registry().unwrap();
        let mut req = request(&owner.created.as_ref().unwrap().held, &registry);
        let mut c = call(&owner, &req);
        match index {
            0 => req.request_sha256 = "f".repeat(64),
            1 => req.workspace_id = WorkspaceId::from_raw("other-workspace"),
            2 => req.work_packet.mutable_files.pop().map(|_| ()).unwrap(),
            3 => req.work_packet.protected_files = vec!["one.py".into()],
            4 => req.mode = RuntimeSessionMode::DurableReadOnly,
            5 => c.action_id = runtime_action_id(&req.run_id, req.limits.max_tool_calls + 1),
            6 => c.arguments.bytes.extend_from_slice(b" "),
            7 => c.arguments.sha256 = "f".repeat(64),
            8 => c.arguments.media_type = "text/plain".into(),
            9 => c.tool_id = ToolId::from_raw("foreign.tool"),
            10 => c.arguments.schema.schema_sha256 = "f".repeat(64),
            11 => c.tool_version = "9.9.9".into(),
            12 => req.visible_tools[0].definition_sha256 = "f".repeat(64),
            13 => owner.created.as_mut().unwrap().creation_receipt_sha256 = hash(b"substitute"),
            14 => owner.edit = None,
            15 => {
                c.arguments.bytes=serde_json::to_vec(&json!({"preview": {"preview_sha256": owner.inspect_edit().unwrap()["pending"]["preview_sha256"]}})).unwrap();
                c.arguments.sha256 = hash(&c.arguments.bytes);
            }
            16 => {
                let bytes = c.arguments.bytes.clone();
                let original = String::from_utf8(bytes).unwrap();
                c.arguments.bytes = format!("{{\"preview\":{{}},{}", &original[1..]).into_bytes();
                c.arguments.sha256 = hash(&c.arguments.bytes);
            }
            17 => req.mode = RuntimeSessionMode::EphemeralReadOnly,
            18 => req.work_packet.protected_files = vec!["private/".into()],
            19 => req.work_packet.protected_files = vec!["**/one.py".into()],
            20 => req.work_packet.protected_files = vec!["../one.py".into()],
            21 => req.work_packet.protected_files = vec!["/one.py".into()],
            _ => req.work_packet.mutable_files.push("private//file".into()),
        }
        if index == 3 {
            // The original packet validator refuses exact mutable/protected
            // overlap before sealing. The handoff must also refuse these
            // unsealable bytes without spending the native pending edit.
            assert_eq!(
                seal_runtime_endpoint_run_request(req.clone())
                    .unwrap_err()
                    .code(),
                "runtime.coordinator.work_packet_denied"
            );
        }
        if [1, 2, 4, 17, 18, 19, 20, 21, 22].contains(&index) {
            req = seal_runtime_endpoint_run_request(req).unwrap();
        }
        if index == 12 {
            req.tool_catalog_sha256 =
                agentmage_kernel_engine::runtime_coordinator::runtime_tool_catalog_sha256(
                    &req.tool_catalog_id,
                    &req.visible_tools,
                )
                .unwrap();
            req = seal_runtime_endpoint_run_request(req).unwrap();
        }
        let (owner, _) = owner
            .into_endpoint_edit(
                req,
                RuntimeTurnId::from_raw("endpoint-edit-turn"),
                RuntimeOperationId::from_raw("endpoint-edit-operation"),
                c,
            )
            .err()
            .unwrap();
        assert!(
            !owner.inspect_edit().unwrap()["latest"].is_object(),
            "probe {index}"
        );
        f.assert_unwritten();
    }
}

#[test]
fn endpoint_native_edit_dependency_unwind_spends_decision_before_any_write() {
    struct PanickingControl;
    impl ModelOperationControl for PanickingControl {
        fn remaining_ms(&self) -> std::result::Result<std::num::NonZeroU64, ModelOperationStop> {
            panic!("synthetic control failure before effect")
        }
    }
    let f = Fixture::new();
    let mut edit = handoff(f.prepared());
    let r = response(&edit, true);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| edit.decide(
            &r,
            NOW,
            &PanickingControl
        )))
        .is_err()
    );
    f.assert_unwritten();
    assert!(edit.decide(&r, NOW, &running()).is_err());
    f.assert_unwritten();
}

#[test]
fn endpoint_native_edit_stop_after_native_inspection_prevents_dispatch() {
    struct StopOnSecond(std::sync::atomic::AtomicUsize);
    impl ModelOperationControl for StopOnSecond {
        fn remaining_ms(&self) -> std::result::Result<std::num::NonZeroU64, ModelOperationStop> {
            if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(std::num::NonZeroU64::new(5000).unwrap())
            } else {
                Err(ModelOperationStop::Cancelled)
            }
        }
    }
    let f = Fixture::new();
    let mut edit = handoff(f.prepared());
    let r = response(&edit, true);
    let control = StopOnSecond(std::sync::atomic::AtomicUsize::new(0));
    assert_eq!(
        edit.decide(&r, NOW, &control).unwrap_err().code(),
        "code.endpoint-edit.stopped"
    );
    assert_eq!(control.0.load(Ordering::SeqCst), 2);
    f.assert_unwritten();
    assert!(edit.decide(&r, NOW, &running()).is_err());
    f.assert_unwritten();
}

#[test]
fn endpoint_native_edit_returning_live_owner_discards_unexecuted_preview() {
    let f = Fixture::new();
    let edit = handoff(f.prepared());
    let digest = edit.challenge().preview_sha256.clone();
    let mut owner = edit.into_repository();
    assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    assert!(owner.decide_edit(&digest, true, NOW).is_err());
    assert!(owner.created.as_ref().unwrap().edits.is_empty());
    f.assert_unwritten();
}

#[path = "coding_repository_endpoint_boundary_tests.rs"]
mod boundary_tests;
