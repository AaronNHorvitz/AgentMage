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

fn context() -> ResearchBudgetContext {
    ResearchBudgetContext {
        session_id: SessionId::from_raw("session-1"),
        task_id: TaskId::from_raw("task-1"),
        run_id: RuntimeRunId::from_raw("run-1"),
        policy_sha256: digest('b'),
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
    let mut candidate = manifest_with_id("research-plan-1");
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
