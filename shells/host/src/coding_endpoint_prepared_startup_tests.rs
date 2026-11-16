// SPDX-License-Identifier: MIT OR Apache-2.0
//! Inert controlled startup and native-owner refusal, without execution claims.
use super::*;
use crate::coding_endpoint_prepared_startup::{
    CodingEndpointPreparedKind as Kind, CodingEndpointPreparedStartup as Startup,
};
use agentmage_platform_linux::{
    LinuxCodingRepository, linux_coding_endpoint_command_registry,
    linux_coding_endpoint_edit_registry,
};
use std::os::unix::fs::PermissionsExt;

#[path = "coding_endpoint_model_edit_tests.rs"]
mod model_edit_tests;

fn controlled(f: &Fixture, kind: Kind) -> RuntimeEndpointRunRequest {
    let mut r = f.request.clone();
    r.mode = RuntimeSessionMode::ControlledWrite;
    r.work_packet.budgets.extend(
        [
            (BudgetResource::ToolCallDepth, 1),
            (BudgetResource::InputBytes, 81920),
            (BudgetResource::OutputBytes, 4096),
            (BudgetResource::ElapsedMilliseconds, 60000),
            (BudgetResource::MemoryBytes, 16777216),
            (BudgetResource::DiskBytes, 4096),
            (BudgetResource::ProcessCount, 1),
        ]
        .map(|(resource, limit)| BudgetLimit { resource, limit }),
    );
    r.work_packet.mutable_files = vec!["src/lib.rs".into(), "private/secret.rs".into()];
    r.work_packet.protected_files = vec!["tests".into()];
    r.work_packet.required_capability_class = match kind {
        Kind::Edit => AuthorityClass::LocalWrite,
        Kind::Command | Kind::EditAndTest => AuthorityClass::Execute,
    };
    let registry = match kind {
        Kind::Edit => linux_coding_endpoint_edit_registry(),
        Kind::Command => linux_coding_endpoint_command_registry(),
        Kind::EditAndTest => agentmage_platform_linux::linux_coding_endpoint_coding_registry(),
    }
    .unwrap();
    r.visible_tools = runtime_tool_references(&registry).unwrap();
    r.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&r.tool_catalog_id, &r.visible_tools).unwrap();
    seal_runtime_endpoint_run_request(r).unwrap()
}

#[test]
fn prepared_endpoint_startup_keeps_original_catalog_and_policy_metadata_without_effects() {
    let f = Fixture::new();
    for kind in [Kind::Edit, Kind::Command] {
        let r = controlled(&f, kind);
        let startup = Startup::prepare(r.clone(), kind, &f.empty_ledger()).unwrap();
        assert!(startup.guidance().verify());
        let mut expected = r.clone();
        expected.task.constraints.push(format!(
            "Trusted repository narrowing policy SHA-256: {}",
            startup.guidance().policy_sha256()
        ));
        assert_eq!(
            *startup.request(),
            seal_runtime_endpoint_run_request(expected).unwrap()
        );
        assert_eq!(startup.request().policy_id, r.policy_id);
        assert_eq!(startup.request().policy_sha256, r.policy_sha256);
        assert_eq!(startup.request().visible_tools, r.visible_tools);
        assert_eq!(startup.request().limits, *startup.guidance().limits());
        assert_ne!(startup.request().request_sha256, r.request_sha256);
    }
    assert_eq!(fs::read_dir(&f.root).unwrap().count(), 1);
    assert_eq!(
        fs::read(f.root.join("sentinel")).unwrap(),
        b"unchanged synthetic source"
    );
}

#[test]
fn prepared_endpoint_startup_narrows_before_binding_native_effects() {
    let f = Fixture::new();
    let ledger = f.ledger(vec![
        constraint(GuidanceConstraintKind::ReduceBudget, "max_tool_calls=2"),
        constraint(GuidanceConstraintKind::ReduceBudget, "max_elapsed_ms=10000"),
    ]);
    for kind in [Kind::Edit, Kind::Command] {
        let startup = Startup::prepare(controlled(&f, kind), kind, &ledger).unwrap();
        assert_eq!(startup.request().limits.max_tool_calls, 2);
        assert_eq!(startup.request().limits.max_elapsed_ms, 10000);
        agentmage_kernel_engine::runtime_coordinator::verify_runtime_endpoint_run_request(
            startup.request(),
        )
        .unwrap();
    }
}

#[test]
fn prepared_endpoint_startup_excludes_edits_and_sources_and_refuses_unconfineable_commands() {
    let f = Fixture::new();
    let ledger = f.ledger(vec![constraint(
        GuidanceConstraintKind::ExcludeScope,
        "private",
    )]);
    let startup = Startup::prepare(controlled(&f, Kind::Edit), Kind::Edit, &ledger).unwrap();
    assert_eq!(startup.request().work_packet.mutable_files, ["src/lib.rs"]);
    assert_eq!(startup.request().work_packet.protected_files, ["tests"]);
    let source = |p: &str| CodingSource {
        path: p.into(),
        content: "untrusted source".into(),
    };
    for p in [
        "private",
        "private/a.rs",
        "private/deep/a.rs",
        "../outside",
        "/absolute",
        "src//lib.rs",
    ] {
        assert_eq!(startup.check_sources(&[source(p)]), Err(Error::Source));
    }
    assert!(
        startup
            .check_sources(&[source("private-neighbor/a.rs"), source("src/lib.rs")])
            .is_ok()
    );
    assert_eq!(
        startup.check_sources(&[source("src/lib.rs"), source("private/a.rs")]),
        Err(Error::Source)
    );
    assert!(startup.check_sources(&[]).is_ok());
    assert_eq!(
        Startup::prepare(controlled(&f, Kind::Command), Kind::Command, &ledger).err(),
        Some(Error::Source)
    );
    let mut r = controlled(&f, Kind::Edit);
    r.work_packet.mutable_files = vec!["private/secret.rs".into()];
    assert_eq!(
        Startup::prepare(
            seal_runtime_endpoint_run_request(r).unwrap(),
            Kind::Edit,
            &ledger
        )
        .err(),
        Some(Error::NoTools)
    );
}

#[test]
fn prepared_endpoint_startup_refuses_stop_disabled_verification_and_widening_for_both_tools() {
    let f = Fixture::new();
    for kind in [Kind::Edit, Kind::Command] {
        let r = controlled(&f, kind);
        let tool = r.visible_tools[0].clone();
        for (c, e) in [
            (
                constraint(GuidanceConstraintKind::Stop, "stop"),
                Error::Guidance(CodingGuidanceError::StopRequested),
            ),
            (
                constraint(
                    GuidanceConstraintKind::DisableTool,
                    &format!("{}@{}", tool.tool_id.as_str(), tool.tool_version),
                ),
                Error::NoTools,
            ),
            (
                constraint(
                    GuidanceConstraintKind::RequireVerification,
                    "unavailable-check",
                ),
                Error::Guidance(CodingGuidanceError::ConstraintDenied),
            ),
            (
                constraint(GuidanceConstraintKind::ReduceBudget, "max_tool_calls=1000"),
                Error::Guidance(CodingGuidanceError::ConstraintDenied),
            ),
        ] {
            assert_eq!(
                Startup::prepare(r.clone(), kind, &f.ledger(vec![c])).err(),
                Some(e)
            );
        }
        let ledger = f.ledger(vec![constraint(
            GuidanceConstraintKind::RequireUserConfirmation,
            &format!("{}@{}", tool.tool_id.as_str(), tool.tool_version),
        )]);
        let startup = Startup::prepare(r, kind, &ledger).unwrap();
        assert!(
            startup
                .guidance()
                .requires_confirmation(&tool.tool_id, &tool.tool_version)
        );
    }
}

#[test]
fn prepared_endpoint_startup_refuses_original_seal_catalog_mode_and_malformed_paths() {
    let f = Fixture::new();
    let prepare = |r, k| Startup::prepare(r, k, &f.empty_ledger()).err();
    for kind in [Kind::Edit, Kind::Command] {
        let original = controlled(&f, kind);
        let mut r = original.clone();
        r.request_sha256 = "0".repeat(64);
        assert_eq!(prepare(r, kind), Some(Error::Binding));
        assert_eq!(prepare(f.request.clone(), kind), Some(Error::Binding));
        let other = if kind == Kind::Edit {
            Kind::Command
        } else {
            Kind::Edit
        };
        assert_eq!(prepare(original.clone(), other), Some(Error::Binding));
        for path in ["../outside", "src//lib.rs", "/absolute"] {
            for protected in [false, true] {
                let mut r = original.clone();
                if protected {
                    r.work_packet.protected_files.push(path.into());
                } else {
                    r.work_packet.mutable_files.push(path.into());
                }
                assert_eq!(
                    prepare(seal_runtime_endpoint_run_request(r).unwrap(), kind),
                    Some(Error::Binding)
                );
            }
        }
        let mut r = original;
        r.limits.max_tool_calls = 0;
        assert_eq!(
            prepare(seal_runtime_endpoint_run_request(r).unwrap(), kind),
            Some(Error::NoTools)
        );
    }
}

#[test]
fn prepared_endpoint_startup_refuses_stale_invalid_and_foreign_instruction_provenance() {
    let f = Fixture::new();
    for kind in [Kind::Edit, Kind::Command] {
        let r = controlled(&f, kind);
        let mut stale = f.empty_ledger();
        stale.workspace_manifest_sha256 = "1".repeat(64);
        assert_eq!(
            Startup::prepare(r.clone(), kind, &stale).err(),
            Some(Error::Instructions)
        );
        let mut invalid = f.empty_ledger();
        invalid.ledger_sha256 = "0".repeat(64);
        assert_eq!(
            Startup::prepare(r.clone(), kind, &invalid).err(),
            Some(Error::Instructions)
        );
        let ledger = f.ledger(vec![constraint(
            GuidanceConstraintKind::ReduceBudget,
            "max_tool_calls=2",
        )]);
        let mut r = r;
        r.workspace_id = WorkspaceId::from_raw("foreign-workspace");
        assert_eq!(
            Startup::prepare(seal_runtime_endpoint_run_request(r).unwrap(), kind, &ledger).err(),
            Some(Error::Guidance(CodingGuidanceError::SourceDenied))
        );
    }
}

#[test]
fn prepared_endpoint_startup_native_binding_refuses_unowned_repository_and_returns_it_intact() {
    let f = Fixture::new();
    let state = f.root.join("state");
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    for kind in [Kind::Edit, Kind::Command] {
        let startup = Startup::prepare(controlled(&f, kind), kind, &f.empty_ledger()).unwrap();
        let repository =
            LinuxCodingRepository::open(&state, b"synthetic-prepared-startup-state-secret", 1000)
                .unwrap();
        let before = repository.inspect().unwrap();
        let registry = match kind {
            Kind::Edit => linux_coding_endpoint_edit_registry(),
            Kind::Command => linux_coding_endpoint_command_registry(),
            Kind::EditAndTest => agentmage_platform_linux::linux_coding_endpoint_coding_registry(),
        }
        .unwrap();
        let def = registry.list_tools()[0];
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("prepared-startup-call"),
            correlation_id: CorrelationId::from_raw("prepared-startup-correlation"),
            action_id: agentmage_kernel_engine::runtime_loop::runtime_action_id(
                &startup.request().run_id,
                1,
            ),
            tool_id: def.tool_id.clone(),
            tool_version: def.tool_version.clone(),
            arguments: ContractPayload {
                schema: def.input_schema.clone(),
                media_type: "application/json".into(),
                bytes: b"{}".to_vec(),
                sha256: "0".repeat(64),
            },
        };
        let (repository, error) = startup
            .bind(
                repository,
                RuntimeTurnId::from_raw("prepared-turn"),
                RuntimeOperationId::from_raw("prepared-operation"),
                call,
            )
            .err()
            .unwrap();
        assert_eq!(error, Error::Native);
        assert_eq!(repository.inspect().unwrap(), before);
        assert!(repository.inspect_edit().unwrap()["pending"].is_null());
        assert!(repository.inspect_command().unwrap()["pending"].is_null());
    }
    assert_eq!(
        fs::read(f.root.join("sentinel")).unwrap(),
        b"unchanged synthetic source"
    );
}

#[test]
fn combined_endpoint_startup_refuses_read_exclusions_and_either_disabled_original_tool() {
    let f = Fixture::new();
    let kind = Kind::EditAndTest;
    let startup = Startup::prepare(controlled(&f, kind), kind, &f.empty_ledger()).unwrap();
    assert_eq!(startup.request().visible_tools.len(), 2);
    let excluded = f.ledger(vec![constraint(
        GuidanceConstraintKind::ExcludeScope,
        "private",
    )]);
    assert_eq!(
        Startup::prepare(controlled(&f, kind), kind, &excluded).err(),
        Some(Error::Source)
    );
    for tool in [
        "agentmage.code.apply-prepared-edit",
        "agentmage.code.run-prepared-command",
    ] {
        let ledger = f.ledger(vec![constraint(
            GuidanceConstraintKind::DisableTool,
            &format!("{tool}@1.0.0"),
        )]);
        assert_eq!(
            Startup::prepare(controlled(&f, kind), kind, &ledger).err(),
            Some(Error::NoTools)
        );
    }
    let ledger = f.ledger(vec![constraint(
        GuidanceConstraintKind::ReduceBudget,
        "max_model_calls=1",
    )]);
    let startup = Startup::prepare(controlled(&f, kind), kind, &ledger).unwrap();
    assert_eq!(startup.request().limits.max_model_calls, 1);
}
