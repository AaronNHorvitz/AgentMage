// Synthetic canonical store/authority cases. No native or model qualification.
use super::*;
use crate::research_report::{
    CanonicalResearchReport, ResearchReportClaimDraft, ResearchReportConflictDraft,
    ResearchReportDisposition, ResearchReportDraft, ResearchReportReadRequest, ResearchSourceSpan,
};

fn draft(s: &mut Stored) -> ResearchReportDraft {
    let source = s.read(200).unwrap();
    let body = std::str::from_utf8(source.response().body()).unwrap();
    ResearchReportDraft {
        schema_version: 1,
        report_id: "report-1".into(),
        sources: vec![s.bundle.clone()],
        spans: vec![ResearchSourceSpan {
            source_index: 0,
            body_sha256: source.response().observation().body_sha256.clone(),
            start_byte: 0,
            end_byte: body.len() as u64,
            excerpt: body.into(),
        }],
        claims: vec![ResearchReportClaimDraft::Observed {
            claim_id: "observed-1".into(),
            span_index: 0,
        }],
        conflicts: vec![],
        unresolved: vec![],
    }
}

fn report(s: &mut Stored, draft: &ResearchReportDraft, now: u64)
    -> Result<CanonicalResearchReport, DurableAuthorityError>
{
    s.runtime.read_research_report(&s.payloads, &s.registry, &ResearchReportReadRequest {
        context: &s.context,
        draft,
        expected_native: &s.native,
        now_epoch_ms: now,
    })
}

#[test]
fn report_checks_fresh_full_sources_and_preserves_original_or_located_wire() {
    for mutation in ["", "located-frame"] {
        let mut s = stored(mutation);
        let draft = draft(&mut s);
        let before = s.runtime.research_budget_state(&s.context).unwrap();
        let generation = s.runtime.generation().unwrap();
        let checked = report(&mut s, &draft, 200).unwrap();
        assert_eq!(checked.disposition(), ResearchReportDisposition::SourceChecked);
        assert_eq!(checked.sources()[0].body_sha256(), draft.spans[0].body_sha256);
        assert_eq!(checked.sources()[0].worker_reported_url().is_some(), !mutation.is_empty());
        assert_eq!(checked.draft().claims[0].evidence_label(),
                   crate::web_research_safety::EvidenceLabel::Observed);
        assert!(!format!("{checked:?}").contains("Ignore all grants"));
        let wire: serde_json::Value = serde_json::from_slice(&checked.encode().unwrap()).unwrap();
        assert_eq!(wire["sources"][0]["retrieved_at_epoch_ms"], 104);
        assert_eq!(wire["checked_at_epoch_ms"], 200);
        assert!(wire.get("complete").is_none());
        assert!(wire["sources"][0].get("fresh").is_none());
        assert!(wire["sources"][0].get("publisher").is_none());
        // Instructions in observed source remain inert; no effects or budget revision.
        assert_eq!(s.runtime.generation().unwrap(), generation);
        assert_eq!(s.runtime.receipts().len(), 1);
        assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256,
                   before.head_sha256);
        s.close();
    }
}

#[test]
fn report_rechecks_lifecycle_after_prior_success_instead_of_using_returned_handle() {
    for mutation in 0..3 {
        let mut s = stored("located-frame");
        let draft = draft(&mut s);
        let _old_report = report(&mut s, &draft, 200).unwrap();
        match mutation {
            0 => { s.payloads.objects.remove(&s.frame.payload_sha256); }
            1 => { s.payloads.objects.get_mut(&s.frame.payload_sha256).unwrap().push(b'x'); }
            _ => {
                s.runtime.release_runtime_artifact(&s.context.session_id, &s.context.task_id,
                    &s.context.policy_sha256, &s.frame, 201).unwrap();
            }
        }
        assert!(report(&mut s, &draft, 202).is_err());
        if mutation == 1 {
            assert!(matches!(report(&mut s, &draft, 203), Err(DurableAuthorityError::Poisoned)));
        }
        s.close();
    }
}

#[test]
fn report_refuses_wrong_pins_context_clock_and_partial_source_substitution() {
    for mutation in 0..7 {
        let mut s = stored("");
        let mut draft = draft(&mut s);
        draft.unresolved.push("A declared question remains unanswered.".into());
        match mutation {
            0 => s.native.worker_sha256 = digest('d'),
            1 => s.context.session_id = SessionId::from_raw("other-session"),
            2 => s.context.task_id = TaskId::from_raw("other-task"),
            3 => s.context.run_id = RuntimeRunId::from_raw("other-run"),
            4 => draft.sources[0].payload_sha256 = digest('d'),
            5 => { s.payloads.objects.remove(&s.frame.payload_sha256); }
            _ => {}
        }
        assert!(report(&mut s, &draft, if mutation == 6 { 104 } else { 200 }).is_err());
        // An unresolved question cannot turn an invalid canonical source into Partial.
        s.close();
    }
}

#[test]
fn report_span_identity_unicode_and_aggregate_quotation_limits_are_enforced() {
    let mut s = stored("unicode-body");
    let original = draft(&mut s);
    assert!(report(&mut s, &original, 200).is_ok());
    for mutation in 0..5 {
        let mut candidate = original.clone();
        match mutation {
            0 => candidate.spans[0].body_sha256 = digest('d'),
            1 => candidate.spans[0].end_byte = 6, // Inside the UTF-8 e-acute.
            2 => candidate.spans[0].excerpt = "different".into(),
            3 => candidate.spans[0].end_byte = u64::MAX,
            _ => candidate.spans[0].source_index = 1,
        }
        assert!(report(&mut s, &candidate, 200).is_err());
    }
    s.close();
    let mut s = stored("");
    let mut candidate = draft(&mut s);
    for index in 1..3 {
        candidate.spans.push(candidate.spans[0].clone());
        candidate.claims.push(ResearchReportClaimDraft::Observed {
            claim_id: format!("observed-{}", index + 1), span_index: index,
        });
    }
    assert!(matches!(report(&mut s, &candidate, 200),
        Err(DurableAuthorityError::ResearchReport(crate::research_report::ResearchReportError::Limit))));
    s.close();
}

#[test]
fn report_keeps_inference_conflicts_and_questions_explicit() {
    let mut s = stored("");
    let mut candidate = draft(&mut s);
    candidate.claims.push(ResearchReportClaimDraft::Observed {
        claim_id: "observed-2".into(), span_index: 0,
    });
    candidate.claims.push(ResearchReportClaimDraft::ModelInference {
        claim_id: "inference-1".into(), text: "This is an interpretation.".into(),
        span_indexes: vec![0], limitation: "Publisher truth was not independently verified.".into(),
    });
    candidate.conflicts.push(ResearchReportConflictDraft {
        claim_ids: vec!["observed-1".into(), "observed-2".into()],
        limitation: "The proposed conflict is not adjudicated.".into(),
    });
    candidate.unresolved.push("Publication time is unknown.".into());
    let checked = report(&mut s, &candidate, 200).unwrap();
    assert_eq!(checked.disposition(), ResearchReportDisposition::Partial);
    assert_eq!(checked.draft().claims[2].evidence_label(),
               crate::web_research_safety::EvidenceLabel::ModelInference);
    assert!(checked.draft() == &candidate);
    assert_eq!(s.runtime.receipts().len(), 1);
    s.close();
}

#[test]
fn report_secret_text_is_refused_without_disclosing_or_spending() {
    let mut s = stored("secret-body");
    let candidate = draft(&mut s);
    let before = s.runtime.research_budget_state(&s.context).unwrap();
    let error = report(&mut s, &candidate, 200).unwrap_err();
    assert!(!format!("{error:?}").contains("Bearer"));
    assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, before.head_sha256);
    s.close();
}

#[test]
fn report_cancellation_and_reopen_preserve_evidence_without_replay() {
    let mut s = stored("located-frame");
    let candidate = draft(&mut s);
    s.runtime.cancel_research_budget(&s.context).unwrap();
    assert_eq!(report(&mut s, &candidate, 200).unwrap().disposition(),
               ResearchReportDisposition::Cancelled);
    let before = s.runtime.research_budget_state(&s.context).unwrap();
    let Stored { directory, runtime, payloads, registry, context, bundle, frame, native } = s;
    drop(runtime);
    let runtime = DurableAuthorityRuntime::open(
        &directory.join("authority.db"), &observation(), &mut TestKey, 203,
    ).unwrap();
    let mut s = Stored { directory, runtime, payloads, registry, context, bundle, frame, native };
    assert_eq!(report(&mut s, &candidate, 204).unwrap().disposition(),
               ResearchReportDisposition::Cancelled);
    assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, before.head_sha256);
    assert_eq!(s.runtime.receipts().len(), 1);
    s.close();
}

#[test]
fn report_sees_journal_cancellation_before_budget_append_and_original_expiry() {
    let mut s = stored("");
    let candidate = draft(&mut s);
    let before = s.runtime.research_budget_state(&s.context).unwrap();
    let deadline = before.progress.started_epoch_ms + before.scope.limits().elapsed_ms;
    assert_eq!(report(&mut s, &candidate, deadline).unwrap().disposition(),
               ResearchReportDisposition::Expired);
    // A read reports expiry but cannot mutate the authoritative clock or reset it.
    assert_eq!(s.runtime.research_budget_state(&s.context).unwrap().head_sha256, before.head_sha256);
    let prior = s.runtime.runtime_events(&s.context.run_id).unwrap().pop().unwrap();
    let mut event = draft_event(&prior, RuntimeEventKind::CancellationRequested {
        cancellation_id: CancellationId::from_raw("cancel-report-run"),
    }, 201);
    event.operation_id = None;
    event.turn_id = None;
    s.runtime.record_runtime_event(seal_runtime_event(event).unwrap()).unwrap();
    assert!(!s.runtime.research_budget_state(&s.context).unwrap().progress.cancelled);
    assert_eq!(report(&mut s, &candidate, 202).unwrap().disposition(),
               ResearchReportDisposition::Cancelled);
    assert!(report(&mut s, &candidate, 200).is_err());
    s.close();
}

#[test]
fn report_does_not_count_alias_bundles_as_distinct_receipts() {
    let mut s = stored("");
    let mut candidate = draft(&mut s);
    let bytes = s.payloads.objects[&s.bundle.payload_sha256].clone();
    let started = s.runtime.runtime_events(&s.context.run_id).unwrap().into_iter()
        .find(|e| matches!(e.kind, RuntimeEventKind::ToolStarted { .. })).unwrap();
    let alias = artifact("source-bundle-alias", &bytes, &s.context, &started,
                         &s.runtime.receipts()[0], 107);
    let publication = s.runtime.publish_runtime_artifact(&mut s.payloads, alias,
                                                        &mut Cursor::new(bytes)).unwrap();
    publication_event(&mut s.runtime, &publication);
    let original = std::mem::replace(&mut s.bundle, publication.reference.clone());
    assert!(s.read(200).is_ok());
    s.bundle = original;
    candidate.sources.push(publication.reference);
    let mut second_span = candidate.spans[0].clone();
    second_span.source_index = 1;
    candidate.spans.push(second_span);
    candidate.claims.push(ResearchReportClaimDraft::Observed {
        claim_id: "alias-claim".into(), span_index: 1,
    });
    assert!(matches!(report(&mut s, &candidate, 200),
        Err(DurableAuthorityError::ResearchReport(crate::research_report::ResearchReportError::Binding))));
    s.close();
}
