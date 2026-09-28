// Reuse the canonical SQLCipher/run/artifact fixtures. Payload storage is fake;
// these tests do not establish native transport, TLS or model qualification.
use super::*;
use crate::public_research::{PublicSearchRequest, PublicSourceType};
use crate::research_budget::{
    ResearchBudgetError, ResearchDepth, ResearchLimits, ResearchNetworkMode, ResearchOperation,
};
use crate::research_fetch::{PreparedPublicGet, PublicGetDraft, PublicGetTarget};
use crate::research_journal::{ResearchBudgetContext, ResearchJournalError};
use crate::research_plan::{PreparedResearchPlan, ResearchPlanDraft};
use std::collections::BTreeSet;

fn freshness_fixture() -> (
    std::path::PathBuf,
    OperationalStore,
    FakePayloadStore,
    PreparedResearchPlan,
    PreparedPublicGet,
    crate::research_journal::DurableResearchReservation,
) {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, plan) = initialized(&path);
    let prepared = packet(&plan, "fresh-attempt-1", 101, 512);
    let reservation = runtime
        .reserve_research_request(
            &payloads,
            &context(),
            &prepared,
            ResearchOperation::Visit,
            101,
        )
        .unwrap();
    drop(runtime);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
    (directory, store, payloads, plan, prepared, reservation)
}

#[test]
fn fresh_dispatch_observation_preserves_spending_and_original_reservation_identity() {
    for now in [101, 102] {
        let (directory, mut store, payloads, _plan, prepared, reservation) = freshness_fixture();
        let original_sha256 = reservation.reservation_sha256().to_owned();
        let before = crate::research_journal::state(&store, &context()).unwrap();
        let fresh = crate::research_journal::consume_fresh_reservation(
            &mut store,
            &payloads,
            &context(),
            &prepared,
            reservation,
            now,
        )
        .unwrap();
        let after = crate::research_journal::state(&store, &context()).unwrap();
        assert_eq!(fresh.reservation_sha256, original_sha256);
        assert_eq!(fresh.request_sha256, prepared.packet().sha256());
        assert_eq!(fresh.checked_at_epoch_ms, now);
        assert_eq!(after.progress.started_epoch_ms, 100);
        assert_eq!(after.progress.last_epoch_ms, now);
        assert_eq!(after.progress.visits, before.progress.visits);
        assert_eq!(after.progress.visits, 1);
        assert_eq!(after.progress.queries, before.progress.queries);
        assert_eq!(after.progress.reserved_bytes, 512);
        assert_eq!(after.revision, before.revision + u16::from(now > 101));
        assert_eq!(after.head_sha256 == original_sha256, now == 101);
        // A newly verified observation cannot refund/replay the original attempt.
        assert_eq!(
            crate::research_journal::reserve(
                &mut store,
                &payloads,
                &context(),
                &prepared,
                ResearchOperation::Visit,
                now,
            )
            .err(),
            Some(ResearchJournalError::Budget(ResearchBudgetError::Binding))
        );
        drop(store);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn fresh_dispatch_rejects_each_context_or_packet_substitution_before_clock_mutation() {
    for mutation in 0..6 {
        let (directory, mut store, payloads, plan, prepared, reservation) = freshness_fixture();
        let before = crate::research_journal::state(&store, &context()).unwrap();
        let mut supplied_context = context();
        let mut supplied_packet = prepared;
        match mutation {
            0 => supplied_context.session_id = SessionId::from_raw("other-session"),
            1 => supplied_context.task_id = TaskId::from_raw("other-task"),
            2 => supplied_context.run_id = RuntimeRunId::from_raw("other-run"),
            3 => supplied_context.policy_sha256 = digest('c'),
            4 => supplied_packet = packet(&plan, "other-call", 101, 512),
            5 => supplied_packet = packet(&plan, "fresh-attempt-1", 101, 513),
            _ => unreachable!(),
        }
        let failure = crate::research_journal::consume_fresh_reservation(
            &mut store,
            &payloads,
            &supplied_context,
            &supplied_packet,
            reservation,
            102,
        )
        .err();
        assert_eq!(
            failure,
            Some(if mutation == 1 {
                ResearchJournalError::NotFound
            } else {
                ResearchJournalError::Binding
            }),
            "mutation {mutation}",
        );
        let after = crate::research_journal::state(&store, &context()).unwrap();
        assert_eq!(after.head_sha256, before.head_sha256);
        assert_eq!(after.progress, before.progress);
        drop(store);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn intervening_reservation_or_cancel_invalidates_old_dispatch_proof() {
    for cancel in [false, true] {
        let (directory, mut store, payloads, plan, prepared, reservation) = freshness_fixture();
        if cancel {
            crate::research_journal::cancel(&mut store, &context()).unwrap();
        } else {
            crate::research_journal::reserve(
                &mut store,
                &payloads,
                &context(),
                &packet(&plan, "fresh-attempt-2", 102, 1),
                ResearchOperation::Visit,
                102,
            )
            .unwrap();
        }
        let before = crate::research_journal::state(&store, &context()).unwrap();
        assert_eq!(
            crate::research_journal::consume_fresh_reservation(
                &mut store,
                &payloads,
                &context(),
                &prepared,
                reservation,
                103,
            )
            .err(),
            Some(ResearchJournalError::Binding)
        );
        let after = crate::research_journal::state(&store, &context()).unwrap();
        assert_eq!(after.head_sha256, before.head_sha256);
        assert_eq!(after.progress.cancelled, cancel);
        drop(store);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn fresh_dispatch_clock_rollback_or_budget_expiry_is_durable_and_terminal() {
    for now in [100, 60_100] {
        let (directory, mut store, payloads, plan, prepared, reservation) = freshness_fixture();
        assert_eq!(
            crate::research_journal::consume_fresh_reservation(
                &mut store,
                &payloads,
                &context(),
                &prepared,
                reservation,
                now,
            )
            .err(),
            Some(ResearchJournalError::Budget(ResearchBudgetError::Exhausted))
        );
        let state = crate::research_journal::state(&store, &context()).unwrap();
        assert!(state.progress.deadline_exhausted);
        assert_eq!(state.progress.reserved_bytes, 512);
        assert_eq!(state.revision, 2);
        drop(store);
        let path = directory.join("authority.db");
        let mut reopened = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
        assert_eq!(
            crate::research_journal::reserve(
                &mut reopened,
                &payloads,
                &context(),
                &packet(&plan, "new-after-clock-failure", 102, 1),
                ResearchOperation::Visit,
                102,
            )
            .err(),
            Some(ResearchJournalError::Budget(ResearchBudgetError::Exhausted))
        );
        drop(reopened);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn packet_expiry_and_disappeared_plan_do_not_return_fresh_material_or_refund() {
    for missing_payload in [false, true] {
        let (directory, mut store, mut payloads, _plan, prepared, reservation) =
            freshness_fixture();
        if missing_payload {
            payloads.objects.clear();
        }
        let now = if missing_payload {
            102
        } else {
            prepared.packet().deadline_epoch_ms()
        };
        assert_eq!(
            crate::research_journal::consume_fresh_reservation(
                &mut store,
                &payloads,
                &context(),
                &prepared,
                reservation,
                now,
            )
            .err(),
            Some(if missing_payload {
                ResearchJournalError::Plan
            } else {
                ResearchJournalError::Binding
            })
        );
        let state = crate::research_journal::state(&store, &context()).unwrap();
        assert_eq!(state.progress.visits, 1);
        assert_eq!(state.progress.reserved_bytes, 512);
        assert_eq!(state.progress.last_epoch_ms, now);
        drop(store);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn failed_freshness_append_is_atomic_and_cannot_return_dispatch_material() {
    let (directory, mut store, payloads, _plan, prepared, reservation) = freshness_fixture();
    let before = crate::research_journal::state(&store, &context()).unwrap();
    store
        .connection
        .execute_batch(
            "CREATE TRIGGER synthetic_freshness_failure BEFORE INSERT ON research_budget_revisions
         BEGIN SELECT RAISE(ABORT, 'synthetic.failure'); END;",
        )
        .unwrap();
    assert_eq!(
        crate::research_journal::consume_fresh_reservation(
            &mut store,
            &payloads,
            &context(),
            &prepared,
            reservation,
            102,
        )
        .err(),
        Some(ResearchJournalError::Storage)
    );
    let after = crate::research_journal::state(&store, &context()).unwrap();
    assert_eq!(after.head_sha256, before.head_sha256);
    assert_eq!(after.progress, before.progress);
    // Runtime poisoning and zero driver calls must ALSO be tested via the public
    // owner entry; this direct canonical-function fixture cannot establish either.
    drop(store);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn fresh_dispatch_preserves_cancel_capacity_and_refuses_a_full_journal() {
    for advance_clock in [false, true] {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let (mut runtime, payloads, plan) = initialized(&path);
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &search_packet(&plan, "quota-query-1", 101),
                ResearchOperation::Query("public Rust documentation"),
                101,
            )
            .unwrap();
        // Real conservative quota refusals advance the trusted high-water. Do not
        // forge counters/head hashes or disable journal-integrity triggers.
        for index in 2..126 {
            assert_eq!(
                runtime
                    .reserve_research_request(
                        &payloads,
                        &context(),
                        &search_packet(&plan, &format!("quota-refused-{index}"), 100 + index),
                        ResearchOperation::Query("public Rust documentation"),
                        100 + index,
                    )
                    .err(),
                Some(DurableAuthorityError::ResearchJournal(
                    ResearchJournalError::Budget(ResearchBudgetError::Exhausted)
                ))
            );
        }
        let prepared = packet(&plan, "fresh-at-capacity", 226, 1);
        let reservation = runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &prepared,
                ResearchOperation::Visit,
                226,
            )
            .unwrap();
        drop(runtime);
        let mut store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
        assert_eq!(
            crate::research_journal::state(&store, &context())
                .unwrap()
                .revision,
            126
        );
        let fresh = crate::research_journal::consume_fresh_reservation(
            &mut store,
            &payloads,
            &context(),
            &prepared,
            reservation,
            226 + u64::from(advance_clock),
        );
        if advance_clock {
            assert_eq!(fresh.err(), Some(ResearchJournalError::JournalExhausted));
            assert_eq!(
                crate::research_journal::cancel(&mut store, &context()),
                Err(ResearchJournalError::JournalExhausted)
            );
            assert!(
                !crate::research_journal::state(&store, &context())
                    .unwrap()
                    .progress
                    .cancelled
            );
        } else {
            assert!(fresh.is_ok());
            crate::research_journal::cancel(&mut store, &context()).unwrap();
            assert!(
                crate::research_journal::state(&store, &context())
                    .unwrap()
                    .progress
                    .cancelled
            );
        }
        assert_eq!(
            crate::research_journal::state(&store, &context())
                .unwrap()
                .revision,
            127
        );
        drop(store);
        fs::remove_dir_all(directory).unwrap();
    }
}

fn context() -> ResearchBudgetContext {
    ResearchBudgetContext {
        session_id: SessionId::from_raw("session-1"),
        task_id: TaskId::from_raw("task-1"),
        run_id: RuntimeRunId::from_raw("run-1"),
        policy_sha256: digest('b'),
    }
}

#[test]
fn historical_reservation_read_verifies_original_revision_without_refund_or_refresh() {
    let (directory, mut store, payloads, plan, prepared, reservation) = freshness_fixture();
    let original = reservation.reservation_sha256().to_owned();
    let later = packet(&plan, "history-later", 201, 512);
    crate::research_journal::reserve(
        &mut store,
        &payloads,
        &context(),
        &later,
        ResearchOperation::Visit,
        201,
    )
    .unwrap();
    crate::research_journal::cancel(&mut store, &context()).unwrap();
    let before = crate::research_journal::state(&store, &context()).unwrap();
    assert_ne!(before.head_sha256, original);
    crate::research_journal::verify_retained_reservation(
        &store,
        &payloads,
        &context(),
        prepared.packet(),
        &original,
        2000,
    )
    .unwrap();
    let after = crate::research_journal::state(&store, &context()).unwrap();
    assert_eq!(after.head_sha256, before.head_sha256);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.progress, before.progress);
    assert!(after.progress.cancelled);
    // A restricted/cancelled head cannot replace the original Reserved revision.
    assert!(
        crate::research_journal::verify_retained_reservation(
            &store,
            &payloads,
            &context(),
            prepared.packet(),
            &after.head_sha256,
            2000
        )
        .is_err()
    );
    assert!(
        crate::research_journal::consume_fresh_reservation(
            &mut store,
            &payloads,
            &context(),
            &prepared,
            reservation,
            2000
        )
        .is_err()
    );
    drop(store);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn historical_reservation_rechecks_full_plan_and_complete_history_not_only_named_hash() {
    for mutation in 0..5 {
        let (directory, store, mut payloads, _plan, prepared, reservation) = freshness_fixture();
        let mut expected = context();
        let plan = crate::research_journal::state(&store, &expected)
            .unwrap()
            .plan;
        match mutation {
            0 => {
                payloads.objects.remove(&plan.payload_sha256);
            }
            1 => {
                payloads
                    .objects
                    .get_mut(&plan.payload_sha256)
                    .unwrap()
                    .push(b'x');
            }
            2 => {
                store.connection.execute_batch("DROP TRIGGER research_budget_revision_update_forbidden; UPDATE research_budget_revisions SET record_json = CAST('{}' AS BLOB) WHERE revision = 0;").unwrap();
            }
            3 => expected.policy_sha256 = digest('d'),
            _ => {}
        }
        assert!(
            crate::research_journal::verify_retained_reservation(
                &store,
                &payloads,
                &expected,
                prepared.packet(),
                reservation.reservation_sha256(),
                if mutation == 4 { 100 } else { 2000 }
            )
            .is_err()
        );
        drop(store);
        fs::remove_dir_all(directory).unwrap();
    }
}

// Public-owner tests reuse the canonical SQLCipher/artifact fixtures. The recording
// driver is synthetic; packet/held-root native binding is tested independently.
mod dispatch_owner {
    mod start_observer {
        include!("research_start_observer_tests.rs");
    }

    mod retrieval {
        include!("research_retrieval_tests.rs");
    }
    use super::*;
    use crate::authority_transaction::AuthorityTransactionRequest;
    use crate::grants::{DerivedOperationGrantRequest, SessionReadGrantRequest};
    use crate::policy::{
        PolicyDocument, PolicyEngine, PolicyEvaluationContext, ScopeRules, ToolPolicyBinding,
    };
    use crate::research_dispatch::tests::RecordingResearchDriver;
    use crate::research_journal::DurableResearchReservation;
    use crate::test_target::{preimage, scope, target};
    use crate::tooling::{Tool, ToolRegistry};
    use agentmage_kernel_contracts::*;

    struct FixtureTool(ToolDefinition);
    impl Tool for FixtureTool {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }
    struct Fixture {
        directory: std::path::PathBuf,
        runtime: DurableAuthorityRuntime,
        payloads: FakePayloadStore,
        registry: ToolRegistry,
        policy: PolicyEngine,
        context: ResearchBudgetContext,
        plan: PreparedResearchPlan,
        prepared: PreparedPublicGet,
        reservation: DurableResearchReservation,
        request: AuthorityTransactionRequest,
        started: RuntimeEvent,
        grant_id: GrantId,
        transaction_id: AuthorityTransactionId,
    }
    fn rules<T: Ord>(one: T) -> ScopeRules<T> {
        ScopeRules {
            allowed: BTreeSet::from([one]),
            denied: BTreeSet::new(),
        }
    }
    fn next_event(prior: &RuntimeEvent, kind: RuntimeEventKind, now: u64) -> RuntimeEvent {
        let mut event = prior.clone();
        event.event_id = RuntimeEventId::from_raw(format!("research-owner-{}", prior.sequence + 1));
        event.sequence += 1;
        event.occurred_at_epoch_ms = now;
        event.previous_event_sha256 = prior.event_sha256.clone();
        event.causation_event_id = Some(prior.event_id.clone());
        event.payload_reference = None;
        event.turn_id = Some(RuntimeTurnId::from_raw("research-turn-1"));
        event.operation_id = if matches!(kind, RuntimeEventKind::TurnStarted) {
            None
        } else {
            Some(RuntimeOperationId::from_raw(
                "runtime-operation-distinct-from-call",
            ))
        };
        event.persistence = crate::runtime_event::runtime_event_persistence(&kind);
        event.kind = kind;
        seal_runtime_event(event).unwrap()
    }
    fn fixture() -> Fixture {
        fixture_with_argument_encoding(false)
    }

    fn fixture_with_argument_encoding(pretty: bool) -> Fixture {
        let directory = temporary_directory();
        let mut runtime = runtime_with_run(&directory.join("authority.db"));
        let mut payloads = FakePayloadStore::default();
        let plan = plan();
        let prepared = packet(&plan, "dispatch-call-1", 101, 512);
        let operation = OperationBinding::new(GrantOperation::NetworkAccess);
        let actor = ActorId::from_raw("actor-1");
        let action = ActionId::from_raw("action-1");
        let tool_id = ToolId::from_raw("research.fixture");
        let exact_target = target(&["fixture.txt"]);
        let mut context = context();
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules(actor.clone()),
            tasks: rules(context.task_id.clone()),
            actions: rules(action.clone()),
            tools: rules(ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".into(),
            }),
            operations: rules(operation),
            targets: rules(exact_target.clone()),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: rules("https:docs.example.com:443".into()),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .unwrap();
        context.policy_sha256 = policy.policy_sha256().into();
        let schema = SchemaReference {
            schema_id: SchemaId::from_raw("research-fixture"),
            schema_version: 1,
            schema_sha256: digest('1'),
        };
        let bytes = if pretty {
            serde_json::to_vec_pretty(prepared.packet().request()).unwrap()
        } else {
            serde_json::to_vec(prepared.packet().request()).unwrap()
        };
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("dispatch-call-1"),
            correlation_id: CorrelationId::from_raw("correlation-1"),
            action_id: action.clone(),
            tool_id: tool_id.clone(),
            tool_version: "1.0.0".into(),
            arguments: ContractPayload {
                schema: schema.clone(),
                media_type: "application/json".into(),
                sha256: super::super::super::sha256(&bytes),
                bytes,
            },
        };
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FixtureTool(ToolDefinition {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".into(),
                display_name: "Synthetic research".into(),
                description: "No native network".into(),
                input_schema: schema.clone(),
                output_schema: schema,
                risk_level: ToolRiskLevel::Moderate,
                declared_effects: vec![operation],
                required_grant: RequiredGrantTemplate {
                    operation,
                    target_scope: "exact-fixture".into(),
                    single_use: true,
                },
                timeout_ms: 1000,
            })))
            .unwrap();
        let parent = runtime
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("dispatch-parent-1"),
                actor_id: actor.clone(),
                session_id: context.session_id.clone(),
                task_id: context.task_id.clone(),
                targets: vec![scope(&[])],
                excluded_targets: vec![],
                sensitivity: DataSensitivity::Ephemeral,
                issued_at_epoch_ms: 1,
                expires_at_epoch_ms: 10_000,
                nonce: GrantNonce::from_raw("dispatch-parent-nonce"),
                maximum_derived_operations: 1,
                preview_sha256: digest('2'),
                policy_sha256: context.policy_sha256.clone(),
            })
            .unwrap();
        let approval = ApprovalId::from_raw("dispatch-approval-1");
        let grant = runtime
            .derive_operation(
                &parent.grant_id,
                DerivedOperationGrantRequest {
                    grant_id: GrantId::from_raw("dispatch-grant-1"),
                    approval_id: approval.clone(),
                    action_id: action.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    operation,
                    tool_id: tool_id.clone(),
                    tool_version: "1.0.0".into(),
                    targets: vec![exact_target.clone()],
                    argument_sha256: call.arguments.sha256.clone(),
                    preimages: vec![preimage(0, &exact_target)],
                    expected_side_effects: vec![GrantSideEffect {
                        operation,
                        target_indexes: vec![0],
                        details_sha256: prepared.packet().sha256().into(),
                    }],
                    rollback_description: "Synthetic observer only".into(),
                    issued_at_epoch_ms: 100,
                    expires_at_epoch_ms: 2000,
                    nonce: GrantNonce::from_raw("dispatch-child-nonce"),
                    preview_sha256: digest('3'),
                    policy_sha256: context.policy_sha256.clone(),
                },
            )
            .unwrap();
        let reference = publish_plan_under_policy(
            &mut runtime,
            &mut payloads,
            &serde_json::to_vec(plan.draft()).unwrap(),
            true,
            &context.policy_sha256,
        );
        runtime
            .open_research_budget(&payloads, &context, &reference, plan.scope(), 100)
            .unwrap();
        let reservation = runtime
            .reserve_research_request(
                &payloads,
                &context,
                &prepared,
                ResearchOperation::Visit,
                101,
            )
            .unwrap();
        let prior = runtime
            .runtime_events(&context.run_id)
            .unwrap()
            .pop()
            .unwrap();
        let turn = next_event(&prior, RuntimeEventKind::TurnStarted, 101);
        runtime.record_runtime_event(turn.clone()).unwrap();
        let requested = next_event(
            &turn,
            RuntimeEventKind::ToolRequested {
                tool_call_id: call.tool_call_id.clone(),
                arguments_sha256: call.arguments.sha256.clone(),
            },
            102,
        );
        let queued = runtime.record_runtime_event(requested.clone()).unwrap();
        assert_eq!(queued.queued_events, 2);
        let started = next_event(
            &requested,
            RuntimeEventKind::ToolStarted {
                tool_call_id: call.tool_call_id.clone(),
                authority_sha256: super::super::super::sha256(&to_canonical_json(&grant).unwrap()),
            },
            103,
        );
        let transaction_id = AuthorityTransactionId::from_raw("dispatch-transaction-1");
        let request = AuthorityTransactionRequest::new(
            transaction_id.clone(),
            OperationAttemptId::from_raw("dispatch-attempt-1"),
            approval,
            grant.grant_id.clone(),
            call.clone(),
            PolicyEvaluationContext {
                actor_id: actor,
                session_id: context.session_id.clone(),
                task_id: context.task_id.clone(),
                action_id: action,
                action_kind: ActionKind::DeterministicTool,
                tool_id,
                tool_version: "1.0.0".into(),
                targets: grant.targets.clone(),
                argument_sha256: call.arguments.sha256,
                preimages: grant.preimages.clone(),
                expected_side_effects: grant.expected_side_effects.clone(),
                preview_sha256: grant.preview_sha256.clone(),
                now_epoch_ms: 103,
                network_scope: Some("https:docs.example.com:443".into()),
                credential_scope: None,
                publication_scope: None,
            },
            103,
            "1970-01-01T00:00:00.103Z",
        )
        .unwrap();
        Fixture {
            directory,
            runtime,
            payloads,
            registry,
            policy,
            context,
            plan,
            prepared,
            reservation,
            request,
            started,
            grant_id: grant.grant_id,
            transaction_id,
        }
    }

    #[test]
    fn research_owner_dispatch_commits_exact_start_and_terminal_without_replay_on_reopen() {
        let mut fixture = fixture();
        let original_reservation = fixture.reservation.reservation_sha256().to_owned();
        let mut driver = RecordingResearchDriver {
            packet: fixture.prepared.packet(),
            now: 103,
            calls: 0,
            reservation: None,
        };
        let (receipt, pending) = fixture
            .runtime
            .begin_research_effect_with_runtime_event(
                &fixture.registry,
                &fixture.policy,
                fixture.request.clone(),
                &mut driver,
                fixture.started.clone(),
                &fixture.payloads,
                &fixture.context,
                &fixture.prepared,
                fixture.reservation,
            )
            .unwrap();
        assert_eq!(driver.calls, 1);
        assert_eq!(
            driver.reservation.as_deref(),
            Some(original_reservation.as_str())
        );
        assert_eq!(receipt.outcome, OperationOutcome::Succeeded);
        assert_eq!(
            fixture
                .runtime
                .current_grant(&fixture.grant_id)
                .unwrap()
                .status,
            GrantStatus::Consumed
        );
        let terminal = next_event(
            &fixture.started,
            RuntimeEventKind::ToolCompleted {
                tool_call_id: ToolCallId::from_raw("dispatch-call-1"),
                receipt_id: receipt.receipt_id.clone(),
                result_sha256: digest('6'),
            },
            104,
        );
        fixture
            .runtime
            .finish_effect_with_runtime_event(pending, terminal.clone())
            .unwrap();
        assert_eq!(
            fixture
                .runtime
                .runtime_events(&fixture.context.run_id)
                .unwrap()
                .last(),
            Some(&terminal)
        );
        let state = fixture
            .runtime
            .research_budget_state(&fixture.context)
            .unwrap();
        assert_eq!(state.progress.visits, 1);
        assert_eq!(state.progress.reserved_bytes, 512);
        assert_eq!(state.progress.last_epoch_ms, 103);
        drop(fixture.runtime);
        let mut reopened = DurableAuthorityRuntime::open(
            &fixture.directory.join("authority.db"),
            &observation(),
            &mut TestKey,
            105,
        )
        .unwrap();
        assert_eq!(reopened.receipts(), [receipt]);
        assert_eq!(
            reopened
                .current_transaction(&fixture.transaction_id)
                .unwrap()
                .state,
            AuthorityTransactionState::Terminal
        );
        assert!(
            reopened
                .reserve_research_request(
                    &fixture.payloads,
                    &fixture.context,
                    &fixture.prepared,
                    ResearchOperation::Visit,
                    105
                )
                .is_err()
        );
        assert_eq!(driver.calls, 1);
        drop(reopened);
        fs::remove_dir_all(fixture.directory).unwrap();
    }

    #[test]
    fn research_owner_refuses_stale_cancelled_missing_and_substituted_inputs_before_driver() {
        for mutation in 0..8 {
            let mut fixture = fixture();
            match mutation {
                0 => fixture
                    .runtime
                    .cancel_research_budget(&fixture.context)
                    .unwrap(),
                1 => {
                    fixture
                        .runtime
                        .reserve_research_request(
                            &fixture.payloads,
                            &fixture.context,
                            &packet(&fixture.plan, "newer-call", 102, 1),
                            ResearchOperation::Visit,
                            102,
                        )
                        .unwrap();
                }
                2 => fixture.payloads.objects.clear(),
                3 => {
                    fixture.started.correlation_id = CorrelationId::from_raw("foreign-correlation")
                }
                4 => {
                    fixture.started.operation_id =
                        Some(RuntimeOperationId::from_raw("unrequested-operation"))
                }
                5 => fixture.started.turn_id = Some(RuntimeTurnId::from_raw("unrequested-turn")),
                6 => fixture.context.policy_sha256 = digest('f'),
                7 => fixture.started.occurred_at_epoch_ms += 1,
                _ => unreachable!(),
            }
            let mut driver = RecordingResearchDriver {
                packet: fixture.prepared.packet(),
                now: 103,
                calls: 0,
                reservation: None,
            };
            assert!(
                matches!(
                    fixture.runtime.begin_research_effect_with_runtime_event(
                        &fixture.registry,
                        &fixture.policy,
                        fixture.request,
                        &mut driver,
                        fixture.started,
                        &fixture.payloads,
                        &fixture.context,
                        &fixture.prepared,
                        fixture.reservation,
                    ),
                    Err(DurableAuthorityError::ResearchJournal(_))
                ),
                "mutation {mutation}"
            );
            assert_eq!(driver.calls, 0);
            assert_eq!(
                fixture
                    .runtime
                    .current_grant(&fixture.grant_id)
                    .unwrap()
                    .status,
                GrantStatus::Issued
            );
            assert!(
                fixture
                    .runtime
                    .current_transaction(&fixture.transaction_id)
                    .is_none()
            );
            drop(fixture.runtime);
            fs::remove_dir_all(fixture.directory).unwrap();
        }
    }

    #[test]
    fn research_owner_run_cancellation_denies_before_budget_cancel_and_after_reopen() {
        for observed in [false, true] {
            for reopen in [false, true] {
                let mut fixture = fixture();
                fixture.runtime.flush_runtime_events().unwrap();
                let prior = fixture
                    .runtime
                    .runtime_events(&fixture.context.run_id)
                    .unwrap()
                    .pop()
                    .unwrap();
                let cancellation_id = CancellationId::from_raw("dispatch-cancellation-1");
                let mut cancelled = next_event(
                    &prior,
                    RuntimeEventKind::CancellationRequested {
                        cancellation_id: cancellation_id.clone(),
                    },
                    102,
                );
                cancelled.turn_id = None;
                cancelled.operation_id = None;
                cancelled = seal_runtime_event(cancelled).unwrap();
                fixture
                    .runtime
                    .record_runtime_event(cancelled.clone())
                    .unwrap();
                if observed {
                    cancelled = next_event(
                        &cancelled,
                        RuntimeEventKind::CancellationObserved { cancellation_id },
                        102,
                    );
                    cancelled.turn_id = None;
                    cancelled.operation_id = None;
                    cancelled = seal_runtime_event(cancelled).unwrap();
                    fixture
                        .runtime
                        .record_runtime_event(cancelled.clone())
                        .unwrap();
                }
                fixture.started = next_event(&cancelled, fixture.started.kind.clone(), 103);
                if reopen {
                    drop(fixture.runtime);
                    fixture.runtime = DurableAuthorityRuntime::open(
                        &fixture.directory.join("authority.db"),
                        &observation(),
                        &mut TestKey,
                        103,
                    )
                    .unwrap();
                }
                // The cancellation is in the existing event owner, not yet in the
                // separate conservative accounting projection. Neither can override it.
                assert!(
                    !fixture
                        .runtime
                        .research_budget_state(&fixture.context)
                        .unwrap()
                        .progress
                        .cancelled
                );
                let mut driver = RecordingResearchDriver {
                    packet: fixture.prepared.packet(),
                    now: 103,
                    calls: 0,
                    reservation: None,
                };
                assert_eq!(
                    fixture
                        .runtime
                        .begin_research_effect_with_runtime_event(
                            &fixture.registry,
                            &fixture.policy,
                            fixture.request,
                            &mut driver,
                            fixture.started,
                            &fixture.payloads,
                            &fixture.context,
                            &fixture.prepared,
                            fixture.reservation,
                        )
                        .err(),
                    Some(DurableAuthorityError::ResearchJournal(
                        ResearchJournalError::Budget(ResearchBudgetError::Cancelled)
                    ))
                );
                assert_eq!(driver.calls, 0);
                assert_eq!(
                    fixture
                        .runtime
                        .current_grant(&fixture.grant_id)
                        .unwrap()
                        .status,
                    GrantStatus::Issued
                );
                assert!(
                    fixture
                        .runtime
                        .current_transaction(&fixture.transaction_id)
                        .is_none()
                );
                drop(fixture.runtime);
                fs::remove_dir_all(fixture.directory).unwrap();
            }
        }
    }

    #[test]
    fn research_owner_failed_preflight_append_poisons_without_launch_or_grant_consumption() {
        let mut fixture = fixture();
        // Preserve the two buffered progress events before deliberately reopening
        // for fault injection. The positive dispatch test instead exercises the
        // owner's mandatory flush at entry.
        assert_eq!(fixture.runtime.flush_runtime_events().unwrap(), 2);
        drop(fixture.runtime);
        let path = fixture.directory.join("authority.db");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
        store.connection.execute_batch("CREATE TRIGGER synthetic_dispatch_failure BEFORE INSERT ON research_budget_revisions BEGIN SELECT RAISE(ABORT, 'synthetic.failure'); END;").unwrap();
        drop(store);
        fixture.runtime =
            DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 103).unwrap();
        let mut driver = RecordingResearchDriver {
            packet: fixture.prepared.packet(),
            now: 103,
            calls: 0,
            reservation: None,
        };
        assert_eq!(
            fixture
                .runtime
                .begin_research_effect_with_runtime_event(
                    &fixture.registry,
                    &fixture.policy,
                    fixture.request,
                    &mut driver,
                    fixture.started,
                    &fixture.payloads,
                    &fixture.context,
                    &fixture.prepared,
                    fixture.reservation,
                )
                .err(),
            Some(DurableAuthorityError::ResearchJournal(
                ResearchJournalError::Storage
            ))
        );
        assert_eq!(driver.calls, 0);
        assert_eq!(
            fixture
                .runtime
                .current_grant(&fixture.grant_id)
                .unwrap()
                .status,
            GrantStatus::Issued
        );
        assert_eq!(
            fixture.runtime.cancel_research_budget(&fixture.context),
            Err(DurableAuthorityError::Poisoned)
        );
        drop(fixture.runtime);
        fs::remove_dir_all(fixture.directory).unwrap();
    }
}

fn plan() -> PreparedResearchPlan {
    PreparedResearchPlan::prepare(ResearchPlanDraft {
        schema_version: 1,
        task_id: "task-1".into(),
        depth: ResearchDepth::Quick,
        network_mode: ResearchNetworkMode::Ask,
        limits: ResearchLimits::ceiling(ResearchDepth::Quick),
        destination_domains: BTreeSet::from([
            "docs.example.com".into(),
            "search.example.com".into(),
        ]),
        queries: vec![PublicSearchRequest {
            request_id: "query-1".into(),
            query: "public Rust documentation".into(),
            domains: vec!["docs.example.com".into()],
            recency_days: 30,
            source_types: vec![PublicSourceType::PrimaryDocumentation],
            max_results: 5,
            max_total_bytes: 1024,
        }],
    })
    .unwrap()
}

fn publish_plan(
    runtime: &mut DurableAuthorityRuntime,
    payloads: &mut FakePayloadStore,
    bytes: &[u8],
    emit: bool,
) -> agentmage_kernel_contracts::RuntimeArtifactRef {
    publish_plan_under_policy(runtime, payloads, bytes, emit, &context().policy_sha256)
}

fn publish_plan_under_policy(
    runtime: &mut DurableAuthorityRuntime,
    payloads: &mut FakePayloadStore,
    bytes: &[u8],
    emit: bool,
    policy_sha256: &str,
) -> agentmage_kernel_contracts::RuntimeArtifactRef {
    let mut candidate = manifest_with_id("research-plan-1");
    candidate.policy_sha256 = policy_sha256.to_owned();
    candidate.kind = RuntimeArtifactKind::Report;
    candidate.media_type = "application/json".into();
    candidate.payload_sha256 = super::super::sha256(bytes);
    candidate.byte_size = bytes.len() as u64;
    candidate.producer_turn_id = None;
    candidate.producer_operation_id = None;
    candidate.receipt_id = None;
    candidate.preview = None;
    candidate.created_at_epoch_ms = 2;
    let candidate = seal_runtime_artifact_manifest(candidate).unwrap();
    let publication = runtime
        .publish_runtime_artifact(payloads, candidate, &mut Cursor::new(bytes))
        .unwrap();
    if emit {
        publish_plan_event(runtime, &publication);
    }
    publication.reference
}

fn publish_plan_event(
    runtime: &mut DurableAuthorityRuntime,
    publication: &super::super::RuntimeArtifactPublication,
) {
    let events = runtime.runtime_events(&context().run_id).unwrap();
    let prior = events.last().unwrap();
    let mut event = prior.clone();
    event.event_id = RuntimeEventId::from_raw("event-research-plan-1");
    event.sequence += 1;
    event.occurred_at_epoch_ms = 3;
    event.causation_event_id = Some(prior.event_id.clone());
    event.previous_event_sha256 = prior.event_sha256.clone();
    event.event_sha256 = digest('0');
    event.payload_reference = Some(runtime_payload_reference(&publication.manifest).unwrap());
    event.kind = RuntimeEventKind::ArtifactCreated {
        artifact_id: publication.reference.artifact_id.clone(),
        manifest_sha256: publication.reference.manifest_sha256.clone(),
    };
    runtime
        .record_runtime_event(seal_runtime_event(event).unwrap())
        .unwrap();
}

fn packet(plan: &PreparedResearchPlan, id: &str, now: u64, maximum: u64) -> PreparedPublicGet {
    PreparedPublicGet::prepare(
        plan.scope(),
        PublicGetDraft {
            schema_version: 1,
            operation_id: id.into(),
            target: PublicGetTarget {
                domain: "docs.example.com".into(),
                path: "/guide".into(),
                query: vec![],
            },
            maximum_response_bytes: maximum,
            redirect_limit: 0,
            timeout_ms: 1000,
        },
        100,
        now,
    )
    .unwrap()
}

fn initialized(
    path: &std::path::Path,
) -> (
    DurableAuthorityRuntime,
    FakePayloadStore,
    PreparedResearchPlan,
) {
    let mut runtime = runtime_with_run(path);
    let mut payloads = FakePayloadStore::default();
    let plan = plan();
    let reference = publish_plan(
        &mut runtime,
        &mut payloads,
        &serde_json::to_vec(plan.draft()).unwrap(),
        true,
    );
    runtime
        .open_research_budget(&payloads, &context(), &reference, plan.scope(), 100)
        .unwrap();
    (runtime, payloads, plan)
}

fn search_packet(plan: &PreparedResearchPlan, id: &str, now: u64) -> PreparedPublicGet {
    PreparedPublicGet::prepare(
        plan.scope(),
        PublicGetDraft {
            schema_version: 1,
            operation_id: id.into(),
            target: PublicGetTarget {
                domain: "search.example.com".into(),
                path: "/search".into(),
                query: vec![("q".into(), "public Rust documentation".into())],
            },
            maximum_response_bytes: 1024,
            redirect_limit: 0,
            timeout_ms: 1000,
        },
        100,
        now,
    )
    .unwrap()
}

#[test]
fn original_deadline_cannot_restart_after_reopen() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, plan) = initialized(&path);
    let request = packet(&plan, "attempt-1", 101, 512);
    assert_eq!(
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Visit,
                60_100
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Exhausted)
        ))
    );
    drop(runtime);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 102).unwrap();
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Visit,
                102
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Exhausted)
        ))
    );
    drop(reopened);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn concurrent_owner_and_budget_reset_are_denied() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, plan) = initialized(&path);
    assert_eq!(
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 101).err(),
        Some(DurableAuthorityError::Store(
            crate::operational_store::OperationalStoreError::ConcurrentWriter
        ))
    );
    let artifact = RuntimeArtifactId::from_raw("research-plan-1");
    let reference = runtime
        .runtime_events(&context().run_id)
        .unwrap()
        .into_iter()
        .find_map(|event| {
            event
                .payload_reference
                .filter(|payload| payload.artifact_id == artifact)
        })
        .unwrap();
    // Full manifest reference comes from canonical publication; a caller cannot
    // select a new timestamp and silently reopen the same task's budget.
    let manifest = {
        drop(runtime);
        let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
        super::super::load_artifact_manifest(&store, &artifact).unwrap()
    };
    let full_reference = runtime_artifact_ref(&manifest).unwrap();
    assert_eq!(reference.sha256, full_reference.payload_sha256);
    runtime = DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 102).unwrap();
    assert_eq!(
        runtime.open_research_budget(&payloads, &context(), &full_reference, plan.scope(), 102),
        Err(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::AlreadyExists
        ))
    );
    drop(runtime);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn tampered_revision_and_missing_head_refuse_reopen() {
    for tamper in [
        "DROP TRIGGER research_budget_revision_update_forbidden; UPDATE research_budget_revisions SET record_json = CAST('{}' AS BLOB) WHERE revision = 0;",
        "DROP TRIGGER research_budget_head_delete_forbidden; DELETE FROM research_budget_heads;",
    ] {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let (runtime, _, _) = initialized(&path);
        drop(runtime);
        let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
        store.connection.execute_batch(tamper).unwrap();
        drop(store);
        assert_eq!(
            DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 101).err(),
            Some(DurableAuthorityError::Store(
                crate::operational_store::OperationalStoreError::IntegrityFailure
            ))
        );
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn bounded_revision_exhaustion_never_resets_accounting() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, plan) = initialized(&path);
    runtime
        .reserve_research_request(
            &payloads,
            &context(),
            &search_packet(&plan, "query-spent", 101),
            ResearchOperation::Query("public Rust documentation"),
            101,
        )
        .unwrap();
    for index in 2..128 {
        assert_eq!(
            runtime
                .reserve_research_request(
                    &payloads,
                    &context(),
                    &search_packet(&plan, &format!("query-refused-{index}"), 100 + index),
                    ResearchOperation::Query("public Rust documentation"),
                    100 + index
                )
                .err(),
            Some(DurableAuthorityError::ResearchJournal(
                ResearchJournalError::Budget(ResearchBudgetError::Exhausted)
            ))
        );
    }
    drop(runtime);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 228).unwrap();
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &packet(&plan, "visit-1", 228, 1),
                ResearchOperation::Visit,
                228
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::JournalExhausted
        ))
    );
    drop(reopened);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn full_plan_and_its_canonical_publication_event_are_required() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let mut runtime = runtime_with_run(&path);
    let mut payloads = FakePayloadStore::default();
    let plan = plan();
    let reference = publish_plan(
        &mut runtime,
        &mut payloads,
        &serde_json::to_vec(plan.draft()).unwrap(),
        false,
    );
    assert_eq!(
        runtime.open_research_budget(&payloads, &context(), &reference, plan.scope(), 100),
        Err(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Plan
        ))
    );
    drop(runtime);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn committed_attempt_survives_reopen_without_refund_or_replay() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, plan) = initialized(&path);
    let request = packet(&plan, "attempt-1", 101, 512);
    let reservation = runtime
        .reserve_research_request(
            &payloads,
            &context(),
            &request,
            ResearchOperation::Visit,
            101,
        )
        .unwrap();
    assert!(reservation.matches_packet(request.packet()));
    assert!(!reservation.matches_packet(packet(&plan, "attempt-2", 102, 512).packet()));
    assert_eq!(reservation.reservation_sha256().len(), 64);
    // No executor exists in this test. Dropping an unused or uncertain reservation
    // must not refund it, nor does it establish that an external request succeeded.
    drop(reservation);
    drop(runtime);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 102).unwrap();
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Visit,
                102
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Binding)
        ))
    );
    reopened
        .reserve_research_request(
            &payloads,
            &context(),
            &packet(&plan, "attempt-2", 102, 512),
            ResearchOperation::Visit,
            102,
        )
        .unwrap();
    drop(reopened);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_quota_clock_and_rollback_remain_terminal_after_reopen() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, plan) = initialized(&path);
    runtime
        .reserve_research_request(
            &payloads,
            &context(),
            &search_packet(&plan, "attempt-1", 101),
            ResearchOperation::Query("public Rust documentation"),
            101,
        )
        .unwrap();
    let request = search_packet(&plan, "attempt-2", 102);
    assert_eq!(
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Query("public Rust documentation"),
                200
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Exhausted)
        ))
    );
    drop(runtime);
    let request = packet(&plan, "visit-1", 102, 1);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 201).unwrap();
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Visit,
                199
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Exhausted)
        ))
    );
    drop(reopened);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 202).unwrap();
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Visit,
                201
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Exhausted)
        ))
    );
    drop(reopened);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cancellation_and_owner_mismatch_cannot_reopen_accounting() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, plan) = initialized(&path);
    let mut wrong = context();
    wrong.session_id = SessionId::from_raw("another-session");
    assert_eq!(
        runtime.cancel_research_budget(&wrong),
        Err(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Binding
        ))
    );
    runtime.cancel_research_budget(&context()).unwrap();
    drop(runtime);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 102).unwrap();
    let request = packet(&plan, "attempt-1", 102, 512);
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Visit,
                102
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Cancelled)
        ))
    );
    reopened.cancel_research_budget(&context()).unwrap();
    drop(reopened);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn disappeared_full_plan_cannot_yield_a_reservation_proof() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, mut payloads, plan) = initialized(&path);
    payloads.objects.clear();
    assert_eq!(
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &packet(&plan, "attempt-1", 101, 512),
                ResearchOperation::Visit,
                101
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Plan
        ))
    );
    drop(runtime);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 102).unwrap();
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &packet(&plan, "attempt-1", 102, 512),
                ResearchOperation::Visit,
                102
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Binding)
        ))
    );
    drop(reopened);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_reservation_commit_returns_no_proof_and_poisons_runtime() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (runtime, payloads, plan) = initialized(&path);
    drop(runtime);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
    store.connection.execute_batch("CREATE TRIGGER synthetic_research_commit_failure BEFORE INSERT ON research_budget_revisions BEGIN SELECT RAISE(ABORT, 'synthetic.failure'); END;").unwrap();
    drop(store);
    let mut reopened =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 101).unwrap();
    let request = packet(&plan, "attempt-1", 101, 512);
    assert_eq!(
        reopened
            .reserve_research_request(
                &payloads,
                &context(),
                &request,
                ResearchOperation::Visit,
                101
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Storage
        ))
    );
    assert_eq!(
        reopened.cancel_research_budget(&context()),
        Err(DurableAuthorityError::Poisoned)
    );
    drop(reopened);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
    let count: i64 = store
        .connection
        .query_row(
            "SELECT COUNT(*) FROM research_budget_revisions",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    drop(store);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn complete_plan_decode_and_exact_restrictions_cannot_be_substituted() {
    let prepared = plan();
    let original = serde_json::to_vec(prepared.draft()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let mut cases = Vec::new();
    for (field, replacement) in [
        ("schema_version", serde_json::json!(2)),
        ("approval", serde_json::json!(true)),
    ] {
        let mut changed = value.clone();
        changed[field] = replacement;
        cases.push((
            serde_json::to_vec(&changed).unwrap(),
            ResearchJournalError::Plan,
        ));
    }
    let duplicate =
        String::from_utf8(original.clone())
            .unwrap()
            .replacen('{', "{\"schema_version\":1,", 1);
    cases.push((duplicate.into_bytes(), ResearchJournalError::Plan));
    let mut changed = prepared.draft().clone();
    changed.limits.visits -= 1;
    let different = PreparedResearchPlan::prepare(changed).unwrap();
    cases.push((
        serde_json::to_vec(different.draft()).unwrap(),
        ResearchJournalError::Binding,
    ));
    for (bytes, expected) in cases {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut runtime = runtime_with_run(&path);
        let mut payloads = FakePayloadStore::default();
        let reference = publish_plan(&mut runtime, &mut payloads, &bytes, true);
        assert_eq!(
            runtime.open_research_budget(&payloads, &context(), &reference, prepared.scope(), 100),
            Err(DurableAuthorityError::ResearchJournal(expected))
        );
        drop(runtime);
        let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
        let count: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM research_budget_roots", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
        drop(store);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn corrupt_complete_plan_cannot_issue_proof_or_refund_spent_attempt() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, mut payloads, prepared) = initialized(&path);
    let bytes = payloads.objects.values_mut().next().unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    assert_eq!(
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &packet(&prepared, "attempt-1", 101, 512),
                ResearchOperation::Visit,
                101,
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Plan
        ))
    );
    // Replacing corrupt bytes with the original plan does not erase accounting.
    *payloads.objects.values_mut().next().unwrap() = serde_json::to_vec(prepared.draft()).unwrap();
    assert_eq!(
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &packet(&prepared, "attempt-1", 102, 512),
                ResearchOperation::Visit,
                102,
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Binding)
        ))
    );
    drop(runtime);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn released_plan_stays_released_after_restart_and_does_not_reset_budget() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (runtime, payloads, prepared) = initialized(&path);
    drop(runtime);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
    let manifest = super::super::load_artifact_manifest(
        &store,
        &RuntimeArtifactId::from_raw("research-plan-1"),
    )
    .unwrap();
    let reference = runtime_artifact_ref(&manifest).unwrap();
    drop(store);
    let mut runtime =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 101).unwrap();
    runtime
        .release_runtime_artifact(
            &context().session_id,
            &context().task_id,
            &context().policy_sha256,
            &reference,
            101,
        )
        .unwrap();
    drop(runtime);
    let mut runtime =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 102).unwrap();
    assert_eq!(
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &packet(&prepared, "attempt-1", 102, 512),
                ResearchOperation::Visit,
                102,
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Plan
        ))
    );
    assert_eq!(
        runtime.open_research_budget(&payloads, &context(), &reference, prepared.scope(), 102),
        Err(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::AlreadyExists
        ))
    );
    drop(runtime);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn encrypted_backup_and_fresh_restore_preserve_spent_operations_and_cancel() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let backup = directory.join("backup.db");
    let restored = directory.join("restored.db");
    let (mut runtime, payloads, prepared) = initialized(&path);
    runtime
        .reserve_research_request(
            &payloads,
            &context(),
            &packet(&prepared, "attempt-1", 101, 512),
            ResearchOperation::Visit,
            101,
        )
        .unwrap();
    runtime.cancel_research_budget(&context()).unwrap();
    runtime
        .backup(&backup, &observation(), &mut TestKey)
        .unwrap();
    drop(runtime);
    OperationalStore::restore_to_fresh_candidate(
        &backup,
        &observation(),
        &mut TestKey,
        &restored,
        &observation(),
        &mut TestKey,
    )
    .unwrap();
    let mut runtime =
        DurableAuthorityRuntime::open(&restored, &observation(), &mut TestKey, 102).unwrap();
    assert_eq!(
        runtime
            .reserve_research_request(
                &payloads,
                &context(),
                &packet(&prepared, "attempt-2", 102, 512),
                ResearchOperation::Visit,
                102,
            )
            .err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Budget(ResearchBudgetError::Cancelled)
        ))
    );
    drop(runtime);
    let store = OperationalStore::open(&restored, &observation(), &mut TestKey).unwrap();
    let revisions: i64 = store
        .connection
        .query_row(
            "SELECT COUNT(*) FROM research_budget_revisions",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(revisions, 3);
    drop(store);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn resume_projection_is_original_content_free_and_does_not_consume_revisions() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let (mut runtime, payloads, prepared) = initialized(&path);
    let original = runtime.research_budget_state(&context()).unwrap();
    assert_eq!(original.progress.started_epoch_ms, 100);
    assert_eq!(original.revision, 0);
    assert_eq!(original.remaining_revisions, 127);
    assert_eq!(original.scope, *prepared.scope());
    for _ in 0..256 {
        assert_eq!(
            runtime
                .research_budget_state(&context())
                .unwrap()
                .head_sha256,
            original.head_sha256
        );
    }
    let diagnostic = format!("{original:?}");
    for content in ["public Rust documentation", "task-1", "docs.example.com"] {
        assert!(!diagnostic.contains(content));
    }
    let mut wrong = context();
    wrong.run_id = RuntimeRunId::from_raw("another-run");
    assert_eq!(
        runtime.research_budget_state(&wrong).err(),
        Some(DurableAuthorityError::ResearchJournal(
            ResearchJournalError::Binding
        ))
    );
    runtime
        .reserve_research_request(
            &payloads,
            &context(),
            &packet(&prepared, "attempt-1", 101, 512),
            ResearchOperation::Visit,
            101,
        )
        .unwrap();
    drop(runtime);
    let mut runtime =
        DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 102).unwrap();
    let retained = runtime.research_budget_state(&context()).unwrap();
    assert_eq!(retained.progress.started_epoch_ms, 100);
    assert_eq!(retained.progress.last_epoch_ms, 101);
    assert_eq!(retained.progress.visits, 1);
    assert_eq!(retained.progress.reserved_bytes, 512);
    assert_eq!(retained.remaining_revisions, 126);
    assert_ne!(retained.head_sha256, original.head_sha256);
    drop(runtime);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn derived_export_includes_all_research_families_without_raw_plan_or_identity() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let export = directory.join("derivative.jsonl");
    let (runtime, _, _) = initialized(&path);
    drop(runtime);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey).unwrap();
    store.export_json_lines(&export, &observation()).unwrap();
    let text = fs::read_to_string(&export).unwrap();
    for content in [
        "public Rust documentation",
        "docs.example.com",
        "task-1",
        "research-plan-1",
    ] {
        assert!(!text.contains(content));
    }
    let records: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records[0]["executable"], false);
    assert_eq!(records[0]["startup_authority"], false);
    for family in [
        "research_budget_roots",
        "research_budget_revisions",
        "research_budget_heads",
    ] {
        assert_eq!(
            records
                .iter()
                .filter(|record| record["family"] == family)
                .count(),
            1
        );
    }
    // An integrity failure cannot be repackaged as a verified derivative.
    store.connection.execute_batch(
        "DROP TRIGGER research_budget_head_delete_forbidden; DELETE FROM research_budget_heads;",
    ).unwrap();
    assert_eq!(
        store
            .export_json_lines(&directory.join("invalid.jsonl"), &observation())
            .err(),
        Some(crate::operational_store::OperationalStoreError::IntegrityFailure)
    );
    assert!(!directory.join("invalid.jsonl").exists());
    drop(store);
    fs::remove_dir_all(directory).unwrap();
}
