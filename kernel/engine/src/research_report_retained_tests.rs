// Real canonical owners with synthetic effects/payload storage; no native admission.
use super::*;
use crate::research_report::{ResearchReportPublicationRequest, RetainedResearchReportReadRequest};
use crate::runtime_artifact::{RESEARCH_REPORT_DRAFT_MEDIA_TYPE, RuntimeArtifactReadRequest, RuntimeArtifactPageRequest};

fn draft_manifest(s: &Stored, draft: &ResearchReportDraft) -> RuntimeArtifactManifest {
    let bytes = serde_json::to_vec(draft).unwrap();
    let mut manifest = manifest_with_id("retained-report-1");
    manifest.kind = RuntimeArtifactKind::Report;
    manifest.media_type = RESEARCH_REPORT_DRAFT_MEDIA_TYPE.into();
    manifest.payload_sha256 = hash(&bytes);
    manifest.byte_size = bytes.len() as u64;
    manifest.session_id = s.context.session_id.clone();
    manifest.task_id = s.context.task_id.clone();
    manifest.producer_run_id = s.context.run_id.clone();
    manifest.producer_turn_id = s.runtime.runtime_events(&s.context.run_id).unwrap()
        .last().unwrap().turn_id.clone();
    manifest.producer_operation_id = None;
    manifest.receipt_id = None;
    manifest.policy_sha256 = s.context.policy_sha256.clone();
    manifest.created_at_epoch_ms = 200;
    manifest.preview = None;
    seal_runtime_artifact_manifest(manifest).unwrap()
}

fn retain(s: &mut Stored, draft: &ResearchReportDraft, manifest: &RuntimeArtifactManifest)
    -> Result<RuntimeArtifactPublication, DurableAuthorityError>
{
    s.runtime.publish_research_report_draft(&mut s.payloads, &s.registry,
        &ResearchReportPublicationRequest {
            read: ResearchReportReadRequest { context: &s.context, draft,
                expected_native: &s.native, now_epoch_ms: 200 }, manifest,
        })
}

fn draft_event_for(s: &Stored, publication: &RuntimeArtifactPublication) -> RuntimeEvent {
    let prior = s.runtime.runtime_events(&s.context.run_id).unwrap().pop().unwrap();
    let m = &publication.manifest;
    let mut event = draft_event(&prior, RuntimeEventKind::ArtifactCreated {
        artifact_id: publication.reference.artifact_id.clone(),
        manifest_sha256: publication.reference.manifest_sha256.clone(),
    }, 201);
    event.turn_id = m.producer_turn_id.clone();
    event.operation_id = None;
    event.retention = m.retention.clone();
    event.sensitivity = m.sensitivity;
    event.payload_reference = Some(runtime_payload_reference(m).unwrap());
    event
}

fn retain_with_event(s: &mut Stored) -> RuntimeArtifactPublication {
    let draft = draft(s);
    let manifest = draft_manifest(s, &draft);
    let publication = retain(s, &draft, &manifest).unwrap();
    let event = seal_runtime_event(draft_event_for(s, &publication)).unwrap();
    s.runtime.record_runtime_event(event).unwrap();
    publication
}

fn read(s: &mut Stored, reference: &RuntimeArtifactRef, now: u64)
    -> Result<CanonicalResearchReport, DurableAuthorityError>
{
    s.runtime.read_retained_research_report(&s.payloads, &s.registry,
        &RetainedResearchReportReadRequest { context: &s.context, reference,
            expected_native: &s.native, now_epoch_ms: now })
}

fn assert_no_generic_content(s: &Stored, reference: &RuntimeArtifactRef) {
    assert!(s.runtime.read_runtime_artifact(&s.payloads, &RuntimeArtifactReadRequest {
        session_id: s.context.session_id.clone(), task_id: s.context.task_id.clone(),
        policy_sha256: s.context.policy_sha256.clone(), reference: reference.clone(),
        now_epoch_ms: 202, maximum_bytes: 128 * 1024,
    }).is_err());
    assert!(s.runtime.read_runtime_artifact_page(&s.payloads, &RuntimeArtifactPageRequest {
        session_id: s.context.session_id.clone(), task_id: s.context.task_id.clone(),
        policy_sha256: s.context.policy_sha256.clone(), reference: reference.clone(),
        now_epoch_ms: 202, offset: 0, maximum_bytes: 64,
    }).is_err());
}

#[test]
fn retained_report_requires_publication_and_reconstructs_without_generic_content() {
    let mut s = stored("located-frame");
    let original = draft(&mut s);
    let manifest = draft_manifest(&s, &original);
    let publication = retain(&mut s, &original, &manifest).unwrap();
    assert!(read(&mut s, &publication.reference, 202).is_err());
    let event = seal_runtime_event(draft_event_for(&s, &publication)).unwrap();
    s.runtime.record_runtime_event(event).unwrap();
    let before = s.runtime.research_budget_state(&s.context).unwrap();
    let generation = s.runtime.generation().unwrap();
    let checked = read(&mut s, &publication.reference, 202).unwrap();
    assert!(checked.draft() == &original);
    assert_eq!(checked.disposition(), ResearchReportDisposition::SourceChecked);
    let wire: serde_json::Value = serde_json::from_slice(&checked.encode().unwrap()).unwrap();
    assert_eq!(wire["checked_at_epoch_ms"], 202);
    assert_eq!(wire["sources"][0]["retrieved_at_epoch_ms"], 104);
    assert_no_generic_content(&s, &publication.reference);
    assert_eq!(s.runtime.generation().unwrap(), generation);
    assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, before.head_sha256);
    assert_eq!(s.runtime.receipts().len(), 1);
    s.close();
}

#[test]
fn retained_report_refuses_alias_publication_in_both_orders() {
    for ordinary_first in [false, true] {
        let mut s = stored("");
        let original = draft(&mut s);
        let manifest = draft_manifest(&s, &original);
        let bytes = serde_json::to_vec(&original).unwrap();
        // Generic publication cannot mint the reserved type even with valid bytes.
        assert!(s.runtime.publish_runtime_artifact(&mut s.payloads, manifest.clone(),
            &mut Cursor::new(&bytes)).is_err());
        let mut alias = manifest.clone();
        alias.artifact_id = RuntimeArtifactId::from_raw("ordinary-alias");
        alias.media_type = "application/json".into();
        let alias = seal_runtime_artifact_manifest(alias).unwrap();
        if ordinary_first {
            s.runtime.publish_runtime_artifact(&mut s.payloads, alias, &mut Cursor::new(&bytes)).unwrap();
            assert!(retain(&mut s, &original, &manifest).is_err());
        } else {
            let publication = retain(&mut s, &original, &manifest).unwrap();
            s.runtime.release_runtime_artifact(&s.context.session_id, &s.context.task_id,
                &s.context.policy_sha256, &publication.reference, 201).unwrap();
            assert!(s.runtime.publish_runtime_artifact(&mut s.payloads, alias,
                &mut Cursor::new(&bytes)).is_err());
        }
        s.close();
    }
}

#[test]
fn retained_report_rechecks_every_source_and_its_own_lifecycle() {
    for mutation in 0..6 {
        let mut s = stored("");
        let publication = retain_with_event(&mut s);
        assert!(read(&mut s, &publication.reference, 202).is_ok());
        let target = if mutation < 3 { s.frame.clone() } else { publication.reference.clone() };
        match mutation % 3 {
            0 => { s.payloads.objects.remove(&target.payload_sha256); }
            1 => { s.payloads.objects.get_mut(&target.payload_sha256).unwrap().push(b'x'); }
            _ => { s.runtime.release_runtime_artifact(&s.context.session_id, &s.context.task_id,
                &s.context.policy_sha256, &target, 203).unwrap(); }
        }
        assert!(read(&mut s, &publication.reference, 204).is_err());
        if mutation % 3 == 1 {
            assert!(matches!(read(&mut s, &publication.reference, 205), Err(DurableAuthorityError::Poisoned)));
        }
        assert_no_generic_content(&s, &publication.reference);
        s.close();
    }
}

#[test]
fn retained_report_rejects_wrong_context_native_identity_and_clock() {
    for mutation in 0..6 {
        let mut s = stored("");
        let publication = retain_with_event(&mut s);
        match mutation {
            0 => s.native.worker_sha256 = digest('d'),
            1 => s.context.session_id = SessionId::from_raw("foreign-session"),
            2 => s.context.task_id = TaskId::from_raw("foreign-task"),
            3 => s.context.run_id = RuntimeRunId::from_raw("foreign-run"),
            4 => s.context.policy_sha256 = digest('d'),
            _ => {}
        }
        assert!(read(&mut s, &publication.reference, if mutation == 5 { 200 } else { 202 }).is_err());
        s.close();
    }
}

#[test]
fn retained_report_cannot_lower_classification_or_extend_retention() {
    let mut s = stored("restricted-source");
    let original = draft(&mut s);
    let mut manifest = draft_manifest(&s, &original);
    assert!(retain(&mut s, &original, &manifest).is_err());
    manifest.sensitivity = ContextSensitivity::Restricted;
    let manifest = seal_runtime_artifact_manifest(manifest).unwrap();
    assert!(retain(&mut s, &original, &manifest).is_ok());
    s.close();

    let mut s = stored("expiring-bundle");
    let original = draft(&mut s);
    let mut manifest = draft_manifest(&s, &original);
    assert!(retain(&mut s, &original, &manifest).is_err()); // Session vs expiring source.
    manifest.retention.kind = RuntimeEventRetentionKind::UntilExpiration;
    manifest.retention.expires_at_epoch_ms = Some(301);
    let mut manifest = seal_runtime_artifact_manifest(manifest).unwrap();
    assert!(retain(&mut s, &original, &manifest).is_err());
    manifest.retention.expires_at_epoch_ms = Some(250);
    let manifest = seal_runtime_artifact_manifest(manifest).unwrap();
    let publication = retain(&mut s, &original, &manifest).unwrap();
    let event = seal_runtime_event(draft_event_for(&s, &publication)).unwrap();
    s.runtime.record_runtime_event(event).unwrap();
    assert!(read(&mut s, &publication.reference, 249).is_ok());
    assert!(read(&mut s, &publication.reference, 250).is_err());
    s.close();
}

#[test]
fn retained_report_refuses_duplicate_or_substituted_publication_events() {
    for mutation in 0..4 {
        let mut s = stored("");
        let original = draft(&mut s);
        let manifest = draft_manifest(&s, &original);
        let publication = retain(&mut s, &original, &manifest).unwrap();
        let mut event = draft_event_for(&s, &publication);
        match mutation {
            0 => {
                s.runtime.record_runtime_event(seal_runtime_event(event.clone()).unwrap()).unwrap();
                event = draft_event_for(&s, &publication);
            }
            1 => event.payload_reference.as_mut().unwrap().sha256 = digest('d'),
            2 => event.sensitivity = ContextSensitivity::Restricted,
            _ => event.kind = RuntimeEventKind::ArtifactCreated {
                artifact_id: publication.reference.artifact_id.clone(), manifest_sha256: digest('d'),
            },
        }
        s.runtime.record_runtime_event(seal_runtime_event(event).unwrap()).unwrap();
        assert!(read(&mut s, &publication.reference, 202).is_err());
        s.close();
    }
}

#[test]
fn retained_report_reopens_with_current_cancelled_state_without_replay() {
    let mut s = stored("");
    let publication = retain_with_event(&mut s);
    s.runtime.cancel_research_budget(&s.context).unwrap();
    let before = s.runtime.research_budget_state(&s.context).unwrap();
    let Stored { directory, runtime, payloads, registry, context, bundle, frame, native } = s;
    drop(runtime);
    let runtime = DurableAuthorityRuntime::open(&directory.join("authority.db"), &observation(), &mut TestKey, 203).unwrap();
    let mut s = Stored { directory, runtime, payloads, registry, context, bundle, frame, native };
    assert_eq!(read(&mut s, &publication.reference, 204).unwrap().disposition(), ResearchReportDisposition::Cancelled);
    assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, before.head_sha256);
    assert_eq!(s.runtime.receipts().len(), 1);
    assert_no_generic_content(&s, &publication.reference);
    s.close();
}

#[test]
fn retained_report_shape_cannot_load_a_cached_checked_projection() {
    let mut s = stored("");
    let original = draft(&mut s);
    let checked = report(&mut s, &original, 200).unwrap().encode().unwrap();
    let mut manifest = draft_manifest(&s, &original);
    manifest.payload_sha256 = hash(&checked);
    manifest.byte_size = checked.len() as u64;
    let manifest = seal_runtime_artifact_manifest(manifest).unwrap();
    // Adversarial fixture writes a self-consistent immutable object at the lower
    // payload owner. The public canonical publication entry would reject it.
    assert!(retain(&mut s, &original, &manifest).is_err());
    let publication = {
        let shared = s.runtime.engineering_store().unwrap().shared_store();
        let mut store = shared.lock().unwrap();
        crate::runtime_artifact::publish_research_draft(&mut store, &mut s.payloads, manifest, &checked).unwrap()
    };
    let event = seal_runtime_event(draft_event_for(&s, &publication)).unwrap();
    s.runtime.record_runtime_event(event).unwrap();
    assert!(read(&mut s, &publication.reference, 202).is_err());
    assert_no_generic_content(&s, &publication.reference);
    s.close();
}

#[test]
fn retained_report_terminal_reconstruction_never_extends_original_deadline() {
    let mut s = stored("");
    let publication = retain_with_event(&mut s);
    let prior = s.runtime.runtime_events(&s.context.run_id).unwrap().pop().unwrap();
    let mut turn = draft_event(&prior, RuntimeEventKind::TurnCompleted {
        outcome_sha256: digest('5'),
    }, 202);
    turn.operation_id = None;
    let turn = seal_runtime_event(turn).unwrap();
    s.runtime.record_runtime_event(turn.clone()).unwrap();
    let mut terminal = draft_event(&turn, RuntimeEventKind::RunTerminal {
        state: AgentStateKind::Success, outcome_sha256: digest('6'),
    }, 203);
    terminal.operation_id = None;
    terminal.turn_id = None;
    s.runtime.record_runtime_event(seal_runtime_event(terminal).unwrap()).unwrap();
    let before = s.runtime.research_budget_state(&s.context).unwrap();
    let deadline = before.progress.started_epoch_ms + before.scope.limits().elapsed_ms;
    let checked = read(&mut s, &publication.reference, deadline).unwrap();
    assert_eq!(checked.disposition(), ResearchReportDisposition::Expired);
    assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, before.head_sha256);
    let mut m = publication.manifest.clone();
    m.artifact_id = RuntimeArtifactId::from_raw("after-terminal-report");
    m.created_at_epoch_ms = deadline;
    let m = seal_runtime_artifact_manifest(m).unwrap();
    assert!(s.runtime.publish_research_report_draft(&mut s.payloads, &s.registry,
        &ResearchReportPublicationRequest { manifest: &m, read: ResearchReportReadRequest {
            context: &s.context, draft: checked.draft(), expected_native: &s.native,
            now_epoch_ms: deadline,
        }}).is_err());
    assert!(s.runtime.runtime_artifact_state(&runtime_artifact_ref(&m)).is_err());
    assert_eq!(s.runtime.receipts().len(), 1);
    s.close();
}

#[test]
fn retained_report_manifest_cannot_carry_a_preview_effect_or_public_classification() {
    let mut s = stored("");
    let original = draft(&mut s);
    let base = draft_manifest(&s, &original);
    for mutation in 0..6 {
        let mut m = base.clone();
        match mutation {
            0 => m.preview = Some(RuntimeArtifactPreview {
                text: "x".into(), byte_size: 1, truncated: true, sha256: hash(b"x"),
            }),
            1 => m.sensitivity = ContextSensitivity::Public,
            2 => m.producer_operation_id = Some(RuntimeOperationId::from_raw("operation-1")),
            3 => m.retention.kind = RuntimeEventRetentionKind::UserHold,
            4 => m.kind = RuntimeArtifactKind::GeneratedFile,
            _ => m.byte_size = 128 * 1024 + 1,
        }
        assert!(seal_runtime_artifact_manifest(m).is_err());
    }
    assert_eq!(s.runtime.receipts().len(), 1);
    s.close();
}

#[test]
fn retained_report_publication_checks_sources_before_creating_any_artifact() {
    for mutation in 0..3 {
        let mut s = stored("");
        let mut original = draft(&mut s);
        match mutation {
            0 => { s.runtime.release_runtime_artifact(&s.context.session_id, &s.context.task_id,
                &s.context.policy_sha256, &s.frame, 199).unwrap(); }
            1 => original.spans[0].excerpt = "Substituted source text.".into(),
            _ => s.native.worker_sha256 = digest('d'),
        }
        let manifest = draft_manifest(&s, &original);
        let before = s.runtime.runtime_artifact_row_counts().unwrap();
        let accounting = s.runtime.research_budget_state(&s.context).unwrap();
        assert!(retain(&mut s, &original, &manifest).is_err());
        assert_eq!(s.runtime.runtime_artifact_row_counts().unwrap(), before);
        assert!(!s.payloads.objects.contains_key(&manifest.payload_sha256));
        assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, accounting.head_sha256);
        assert_eq!(s.runtime.receipts().len(), 1);
        s.close();
    }
}

#[test]
fn retained_report_store_refuses_version_nineteen_reader_without_changing_bytes() {
    let mut s = stored("located-frame");
    let publication = retain_with_event(&mut s);
    assert!(read(&mut s, &publication.reference, 202).is_ok());
    let before_budget = s.runtime.research_budget_state(&s.context).unwrap();
    let Stored { directory, runtime, payloads, registry, context, bundle, frame, native } = s;
    drop(runtime);
    let path = directory.join("authority.db");
    let encrypted = fs::read(&path).unwrap();
    // This invokes the real early-open path with the preceding reader ceiling;
    // it is a component probe, not execution of an installed older binary.
    assert_eq!(crate::operational_store::probe_version_nineteen_reader(&path, &[31; 32]),
        Err(crate::operational_store::OperationalStoreError::MigrationFailed));
    assert_eq!(fs::read(&path).unwrap(), encrypted);
    let runtime = DurableAuthorityRuntime::open(&path, &observation(), &mut TestKey, 203).unwrap();
    let mut s = Stored { directory, runtime, payloads, registry, context, bundle, frame, native };
    assert!(read(&mut s, &publication.reference, 204).is_ok());
    assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, before_budget.head_sha256);
    assert_eq!(s.runtime.receipts().len(), 1);
    assert_no_generic_content(&s, &publication.reference);
    s.close();
}
