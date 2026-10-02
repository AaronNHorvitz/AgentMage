//! Research reports retained through the real coordinator and the real owners
//! (Decision 0142, AMR-03.1.4). An admitted run with a deep plan searches and
//! visits through the synthetic native worker and completes with a report
//! draft that cites its sources. The canonical report owner checks the draft
//! against the run's complete source bundles before it retains it, and every
//! read rebuilds the report through fresh checks. Nothing is sent, and the
//! model is the fixture model.

use super::*;
use crate::research_report::{
    CanonicalResearchReport, ResearchReportClaimDraft, ResearchReportDisposition,
    ResearchSourceSpan, RetainedResearchReportReadRequest,
};
use agentmage_kernel_contracts::RuntimePayloadReference;

/// The three queries the plan discloses.
const QUERIES: [&str; 3] = [
    "public Rust documentation",
    "public Rust release notes",
    "public Rust guide",
];
/// How long the plan allows from the budget's opening.
const ELAPSED_MS: u64 = 60_000;
/// The excerpt each draft quotes from every source body.
const EXCERPT: &str = "Public guide text.";
/// Searches come first in the run, then the two visits.
const FIRST_VISIT: usize = 3;

/// The deep plan the person confirmed: three disclosed queries through the
/// search endpoint and at most three visits.
fn deep_plan() -> Vec<u8> {
    let mut limits = ResearchLimits::ceiling(ResearchDepth::Deep);
    limits.queries = 3;
    limits.visits = 3;
    limits.elapsed_ms = ELAPSED_MS;
    let draft = ResearchPlanDraft {
        schema_version: 2,
        task_id: "task-0001".into(),
        depth: ResearchDepth::Deep,
        network_mode: ResearchNetworkMode::TaskAuthorized,
        limits,
        destination_domains: BTreeSet::from([
            DOMAIN.into(),
            SEARCH_DOMAIN.into(),
            UNLISTED_DOMAIN.into(),
        ]),
        queries: QUERIES
            .iter()
            .enumerate()
            .map(|(index, query)| PublicSearchRequest {
                request_id: format!("query-{}", index + 1),
                query: (*query).into(),
                domains: vec![DOMAIN.into()],
                recency_days: 30,
                source_types: vec![PublicSourceType::PrimaryDocumentation],
                max_results: 5,
                max_total_bytes: 1024,
            })
            .collect(),
        search_endpoint: Some(PublicSearchEndpoint {
            domain: SEARCH_DOMAIN.into(),
            path: "/search".into(),
            query_field: "q".into(),
            fixed_fields: vec![],
        }),
    };
    serde_json::to_vec(PreparedResearchPlan::prepare(draft).unwrap().draft()).unwrap()
}

/// The run's requests: one search for each disclosed query, then two visits.
fn deep_requests() -> Vec<PublicGetDraft> {
    let mut requests: Vec<_> = QUERIES
        .iter()
        .enumerate()
        .map(|(index, query)| {
            let mut search = draft_to(
                SEARCH_DOMAIN,
                &format!("public-search-{}", index + 1),
                "/search",
            );
            search.target.query = vec![("q".into(), (*query).into())];
            search
        })
        .collect();
    requests.push(draft("public-get-1", "/guide"));
    requests.push(draft("public-get-2", "/reference"));
    requests
}

/// Room in the run's limits and the task's budgets for six turns and the
/// five retrieved bundles. Each bundle retains six artifacts, and the run's
/// artifact allowance grows with its turns.
fn deep_limits(request: &mut RuntimeRunRequest) {
    let limits = &mut request.limits;
    limits.max_turns = 12;
    limits.max_model_calls = 8;
    limits.max_tool_calls = 8;
    limits.max_repeated_tool_calls = 8;
    limits.max_no_progress_turns = 8;
    limits.max_context_refreshes = 8;
    limits.max_events = 512;
    limits.max_output_bytes = MAX_RUNTIME_INLINE_OUTPUT_BYTES as u64;
    for budget in &mut request.work_packet.budgets {
        match budget.resource {
            BudgetResource::PlanSteps | BudgetResource::ModelCalls | BudgetResource::ToolCalls => {
                budget.limit = 8;
            }
            BudgetResource::InputBytes => budget.limit = 64 * 1024,
            _ => {}
        }
    }
}

/// One task-authorized deep run whose model completes with `report`.
fn deep_run(fault: Fault, report: ReportScript) -> OwnerCoordinator {
    compose_shaped(
        ResearchNetworkMode::TaskAuthorized,
        fault,
        8,
        PARENT_EXPIRY,
        deep_requests(),
        RunShape {
            plan: Some(deep_plan()),
            actions: Some(8),
            adjust: Some(deep_limits),
            report: Some(report),
        },
    )
}

/// A draft citing the run's bundles at `sources`, in that order: one exact
/// excerpt of each source body, one observation of each excerpt and one
/// labelled inference over all of them.
fn cited(
    bundles: &[RuntimeArtifactRef],
    sources: &[usize],
    unresolved: &[&str],
) -> ResearchReportDraft {
    let spans: Vec<_> = (0..sources.len())
        .map(|index| ResearchSourceSpan {
            source_index: u16::try_from(index).unwrap(),
            body_sha256: sha256(BODY),
            start_byte: 0,
            end_byte: EXCERPT.len() as u64,
            excerpt: EXCERPT.into(),
        })
        .collect();
    let mut claims: Vec<_> = (0..spans.len())
        .map(|index| ResearchReportClaimDraft::Observed {
            claim_id: format!("claim-{}", index + 1),
            span_index: u16::try_from(index).unwrap(),
        })
        .collect();
    claims.push(ResearchReportClaimDraft::ModelInference {
        claim_id: "inference-1".into(),
        text: "The search results and the pages quote the same public guide.".into(),
        span_indexes: (0..u16::try_from(spans.len()).unwrap()).collect(),
        limitation: "Neither the publisher nor the date of any source was checked.".into(),
    });
    ResearchReportDraft {
        schema_version: 1,
        report_id: "deep-report-1".into(),
        sources: sources
            .iter()
            .map(|&index| bundles[index].clone())
            .collect(),
        spans,
        claims,
        conflicts: vec![],
        unresolved: unresolved.iter().map(|&text| text.to_owned()).collect(),
    }
}

fn encoded(draft: &ResearchReportDraft) -> Vec<u8> {
    serde_json::to_vec(draft).unwrap()
}

/// The positive draft: one search result and both visited pages.
const CITED: [usize; 3] = [0, FIRST_VISIT, FIRST_VISIT + 1];

impl OwnerPort {
    /// Rebuilds a retained draft through the retained-report reader.
    fn read_report(
        &mut self,
        reference: &RuntimeArtifactRef,
        now: u64,
    ) -> Result<CanonicalResearchReport, DurableAuthorityError> {
        self.authority.read_retained_research_report(
            &self.payloads,
            &self.registry,
            &RetainedResearchReportReadRequest {
                context: &self.context,
                reference,
                expected_native: &self.native,
                now_epoch_ms: now,
            },
        )
    }

    /// Every artifact creation the canonical journal holds for the run.
    fn creation_events(&mut self) -> Vec<RuntimeArtifactId> {
        self.authority
            .runtime_events(&self.context.run_id)
            .unwrap()
            .into_iter()
            .filter_map(|event| match event.kind {
                RuntimeEventKind::ArtifactCreated { artifact_id, .. } => Some(artifact_id),
                _ => None,
            })
            .collect()
    }
}

/// The canonical journal created exactly the coordinator's artifacts, and
/// none of them is a report draft.
fn assert_no_draft_created(runtime: OwnerCoordinator) -> OwnerPort {
    assert_eq!(retained_report(&runtime), None);
    let created: Vec<_> = runtime
        .artifact_references
        .iter()
        .map(|reference| reference.artifact_id.clone())
        .collect();
    let mut port = runtime.tool_boundary;
    assert_eq!(port.creation_events(), created);
    port
}

/// The retained draft among the coordinator's artifacts.
fn retained_report(runtime: &OwnerCoordinator) -> Option<RuntimeArtifactRef> {
    runtime
        .artifact_references
        .iter()
        .find(|reference| reference.media_type == RESEARCH_REPORT_DRAFT_MEDIA_TYPE)
        .cloned()
}

/// The person's cancellation of the run's own task.
fn cancellation_of(runtime: &OwnerCoordinator) -> CancellationSignal {
    CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("research-cancellation-1"),
        correlation_id: runtime.correlation_id.clone(),
        task_id: runtime.request.task.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    }
}

/// The run spent exactly its five requests, each dispatched once.
fn assert_deep_requests_spent(port: &mut OwnerPort, now: u64) {
    let state = port.authority.research_budget_state(&port.context).unwrap();
    assert_eq!(state.progress.queries, 3);
    assert_eq!(state.progress.visits, 2);
    assert_eq!(state.progress.reserved_bytes, 5 * MAXIMUM_RESPONSE_BYTES);
    assert_eq!(port.attempts.len(), 5);
    assert_eq!(port.authority.receipts().len(), 5);
    for index in 0..5 {
        assert_known_effect_without_replay(port, index, now);
    }
}

#[test]
fn a_deep_run_retains_its_report_through_the_owner_and_reads_it_back() {
    let mut runtime = deep_run(
        Fault::None,
        Box::new(|bundles| encoded(&cited(bundles, &CITED, &[]))),
    );
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("the deep run completes");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert!(outcome.unresolved_codes.is_empty());
    let report = retained_report(&runtime).unwrap();
    assert_eq!(
        outcome.output,
        Some(RuntimeOutput::Artifact {
            reference: RuntimePayloadReference {
                artifact_id: report.artifact_id.clone(),
                sha256: report.payload_sha256.clone(),
                byte_size: report.byte_size,
                media_type: report.media_type.clone(),
            },
        })
    );
    let coordinator_events = runtime.events().to_vec();
    let mut port = runtime.tool_boundary;
    assert_eq!(port.report_publications, 1);
    assert_eq!(port.report_refusal, None);
    // The canonical journal holds exactly the coordinator's stream, and the
    // draft's creation precedes the terminal.
    assert_eq!(
        port.authority.runtime_events(&port.context.run_id).unwrap(),
        coordinator_events
    );
    let created = coordinator_events
        .iter()
        .position(|event| {
            matches!(&event.kind, RuntimeEventKind::ArtifactCreated { artifact_id, .. }
                if artifact_id == &report.artifact_id)
        })
        .unwrap();
    assert!(created < coordinator_events.len() - 1);
    assert_deep_requests_spent(&mut port, READ_AT);
    // The retained draft reads back through fresh checks of every source.
    let bundles = port.bundles.borrow().clone();
    let expected = cited(&bundles, &CITED, &[]);
    let checked = port.read_report(&report, READ_AT).unwrap();
    assert_eq!(
        checked.disposition(),
        ResearchReportDisposition::SourceChecked
    );
    assert!(checked.draft() == &expected);
    assert_eq!(checked.sources().len(), 3);
    for source in checked.sources() {
        assert_eq!(source.body_sha256(), sha256(BODY));
    }
    // A read cannot precede the run's last committed event.
    assert!(port.read_report(&report, 9_000).is_err());
    // At the plan's elapsed limit the same draft reads back as expired.
    let started = port
        .authority
        .research_budget_state(&port.context)
        .unwrap()
        .progress
        .started_epoch_ms;
    assert_eq!(
        port.read_report(&report, started + ELAPSED_MS)
            .unwrap()
            .disposition(),
        ResearchReportDisposition::Expired
    );
    // Reopening the store reads the same report without replay.
    port = port.reopen(READ_AT + 1);
    let reread = port.read_report(&report, READ_AT + 2).unwrap();
    assert_eq!(
        reread.disposition(),
        ResearchReportDisposition::SourceChecked
    );
    assert!(reread.draft() == &expected);
    assert_deep_requests_spent(&mut port, READ_AT + 3);
    assert_eq!(port.report_publications, 1);
    port.close();
}

#[test]
fn a_report_with_unresolved_questions_reads_back_as_partial() {
    let mut runtime = deep_run(
        Fault::None,
        Box::new(|bundles| {
            encoded(&cited(
                bundles,
                &CITED,
                &["Which release first documented this guide?"],
            ))
        }),
    );
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("the deep run completes");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    let report = retained_report(&runtime).unwrap();
    let mut port = runtime.tool_boundary;
    assert_eq!(
        port.read_report(&report, READ_AT).unwrap().disposition(),
        ResearchReportDisposition::Partial
    );
    port.close();
}

#[test]
fn a_cancellation_after_the_report_keeps_it_and_reads_back_as_cancelled() {
    let mut runtime = deep_run(
        Fault::CancelAfterReport,
        Box::new(|bundles| encoded(&cited(bundles, &CITED, &[]))),
    );
    let probe = RaisedCancellation {
        raised: Arc::clone(&runtime.tool_boundary.cancellation),
        signal: cancellation_of(&runtime),
    };
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, Some(&probe)).unwrap()
    else {
        panic!("the cancelled run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Cancelled);
    assert_eq!(outcome.unresolved_codes, ["runtime.cancelled"]);
    assert_eq!(outcome.output, None);
    let report = retained_report(&runtime).unwrap();
    let mut port = runtime.tool_boundary;
    assert_eq!(port.report_publications, 1);
    assert_eq!(port.budget_cancellations, 1);
    assert!(
        port.authority
            .research_budget_state(&port.context)
            .unwrap()
            .progress
            .cancelled
    );
    assert_eq!(
        port.read_report(&report, READ_AT).unwrap().disposition(),
        ResearchReportDisposition::Cancelled
    );
    port = port.reopen(READ_AT + 1);
    assert_eq!(
        port.read_report(&report, READ_AT + 2)
            .unwrap()
            .disposition(),
        ResearchReportDisposition::Cancelled
    );
    assert_deep_requests_spent(&mut port, READ_AT + 3);
    port.close();
}

/// One change that makes the owner refuse an otherwise valid draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// The run's plan, a report of the run but not a source bundle.
    PlanAsSource,
    /// A bundle reference whose manifest digest was changed.
    ForgedSource,
    /// The quoted range holds other text than the excerpt.
    AbsentExcerpt,
    /// The excerpt names another body.
    OtherBody,
    /// Four sources under a plan that allows three visits.
    OverLimit,
}

/// The plan's reference. Every run composed this way publishes the same plan
/// at its start under the same clock, so a first run that only starts shows
/// it.
fn plan_reference() -> RuntimeArtifactRef {
    let mut first = deep_run(Fault::None, Box::new(|_| Vec::new()));
    first.start().unwrap();
    let plan = first
        .tool_boundary
        .authority
        .research_budget_state(&first.tool_boundary.context)
        .unwrap()
        .plan;
    first.tool_boundary.close();
    plan
}

#[test]
fn the_owner_refuses_a_draft_its_sources_do_not_support_and_retains_nothing() {
    let plan = plan_reference();
    for refusal in [
        Refusal::PlanAsSource,
        Refusal::ForgedSource,
        Refusal::AbsentExcerpt,
        Refusal::OtherBody,
        Refusal::OverLimit,
    ] {
        let plan = plan.clone();
        let drafted = Rc::new(RefCell::new(Vec::new()));
        let written = Rc::clone(&drafted);
        let mut runtime = deep_run(
            Fault::None,
            Box::new(move |bundles| {
                let mut draft = cited(bundles, &CITED, &[]);
                match refusal {
                    Refusal::PlanAsSource => draft.sources[0] = plan.clone(),
                    Refusal::ForgedSource => {
                        draft.sources[1].manifest_sha256 = sha256(b"forged manifest");
                    }
                    Refusal::AbsentExcerpt => draft.spans[1].excerpt = "Public guide test.".into(),
                    Refusal::OtherBody => draft.spans[2].body_sha256 = sha256(b"another body"),
                    Refusal::OverLimit => draft = cited(bundles, &[0, 1, FIRST_VISIT, 4], &[]),
                }
                let bytes = encoded(&draft);
                written.borrow_mut().clone_from(&bytes);
                bytes
            }),
        );
        let RuntimeCoordinatorStep::Complete { outcome } =
            runtime.run_until_boundary(None, None).unwrap()
        else {
            panic!("the run ends");
        };
        assert_eq!(outcome.state, AgentStateKind::Failed, "{refusal:?}");
        assert_eq!(
            outcome.unresolved_codes,
            ["runtime.research.report_refused"],
            "{refusal:?}"
        );
        assert_eq!(outcome.output, None, "{refusal:?}");
        let mut port = assert_no_draft_created(runtime);
        assert_eq!(port.report_publications, 1, "{refusal:?}");
        let error = port.report_refusal.unwrap();
        assert!(
            match refusal {
                Refusal::PlanAsSource | Refusal::ForgedSource => {
                    matches!(error, ResearchReportError::Source(_))
                }
                Refusal::AbsentExcerpt | Refusal::OtherBody => {
                    error == ResearchReportError::Binding
                }
                Refusal::OverLimit => error == ResearchReportError::Limit,
            },
            "{refusal:?}: {error:?}"
        );
        // Nothing was retained: no payload of the draft and no creation event.
        assert!(
            !port
                .payloads
                .objects
                .contains_key(&sha256(&drafted.borrow())),
            "{refusal:?}"
        );
        assert_deep_requests_spent(&mut port, READ_AT);
        port.close();
    }
}

#[test]
fn a_draft_that_is_not_exact_never_reaches_the_owner() {
    for case in ["duplicate-source", "not-canonical"] {
        let mut runtime = deep_run(
            Fault::None,
            Box::new(move |bundles| {
                let mut draft = cited(bundles, &CITED, &[]);
                if case == "duplicate-source" {
                    draft.sources[1] = draft.sources[0].clone();
                    encoded(&draft)
                } else {
                    serde_json::to_vec_pretty(&draft).unwrap()
                }
            }),
        );
        let RuntimeCoordinatorStep::Complete { outcome } =
            runtime.run_until_boundary(None, None).unwrap()
        else {
            panic!("the run ends");
        };
        assert_eq!(outcome.state, AgentStateKind::Failed, "{case}");
        assert_eq!(outcome.unresolved_codes, ["runtime.proposal.invalid"]);
        let port = assert_no_draft_created(runtime);
        assert_eq!(port.report_publications, 0, "{case}");
        port.close();
    }
}

#[test]
fn source_drift_after_the_report_is_refused_on_each_read() {
    for drift in ["frame-bytes", "material-missing", "native-identity"] {
        let mut runtime = deep_run(
            Fault::None,
            Box::new(|bundles| encoded(&cited(bundles, &CITED, &[]))),
        );
        runtime.run_until_boundary(None, None).unwrap();
        let report = retained_report(&runtime).unwrap();
        let mut port = runtime.tool_boundary;
        assert!(port.read_report(&report, READ_AT).is_ok());
        let visit = port.bundles.borrow()[FIRST_VISIT].clone();
        let retained = port
            .read(&visit, READ_AT)
            .unwrap()
            .retained_artifacts()
            .to_vec();
        match drift {
            // Retained artifacts are, in order: call, packet, material, frame,
            // result and bundle.
            "frame-bytes" => port
                .payloads
                .objects
                .get_mut(&retained[3].payload_sha256)
                .unwrap()
                .push(b'x'),
            "material-missing" => {
                port.payloads.objects.remove(&retained[2].payload_sha256);
            }
            _ => port.native.worker_sha256 = sha256(b"another worker"),
        }
        assert!(
            port.read_report(&report, READ_AT + 1).is_err(),
            "accepted {drift}"
        );
        port = port.reopen(READ_AT + 2);
        assert!(
            port.read_report(&report, READ_AT + 3).is_err(),
            "accepted {drift} after reopening"
        );
        port.close();
    }
}
