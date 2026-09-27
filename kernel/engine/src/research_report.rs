//! Point-in-time source checking through the existing canonical authority owner.
//! Reports do not grant effects, qualify producers, establish publisher truth or
//! complete workflows. Reuse requires a new canonical read, including lifecycle.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ReceiptId, RuntimeArtifactRef, RuntimeEventKind, RuntimeRunId, SessionId, TaskId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::authority_transaction::AuthorityTransactionCoordinator;
use crate::grants::GrantIssuer;
use crate::operational_store::OperationalStore;
use crate::research_fetch::PublicGetTarget;
use crate::research_journal::{self, ResearchBudgetContext, ResearchJournalError};
use crate::research_result_binding::PublicGetNativeIdentity;
use crate::research_retrieval::{self, PublicGetReadRequest, ResearchRetrievalError};
use crate::runtime_artifact::RuntimeArtifactPayloadStore;
use crate::tooling::ToolRegistry;

#[path = "research_report_retained.rs"]
mod retained;
#[path = "research_report_shape.rs"]
mod shape;
#[path = "research_report_span.rs"]
mod span;

pub use retained::{ResearchReportPublicationRequest, RetainedResearchReportReadRequest};
pub(crate) use retained::{publish_draft, read_retained};

pub use shape::{
    ResearchReportClaimDraft, ResearchReportConflictDraft, ResearchReportDraft,
    ResearchReportShapeError,
};
pub use span::ResearchSourceSpan;
use span::{QuoteBudget, SpanError, excerpt_sha256};

const MAX_REPORT_BYTES: usize = 256 * 1024;

/// Exact current read restrictions; these arguments cannot create authority.
pub struct ResearchReportReadRequest<'a> {
    /// Existing canonical context shared by every source.
    pub context: &'a ResearchBudgetContext,
    /// Untrusted proposed claims, never previously checked source handles.
    pub draft: &'a ResearchReportDraft,
    /// Producer pins independently admitted by trusted composition.
    pub expected_native: &'a PublicGetNativeIdentity,
    /// Trusted read time; original retrieval clocks remain unchanged.
    pub now_epoch_ms: u64,
}

/// Report disposition only; none of these values establishes workflow completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchReportDisposition {
    /// All declared excerpts checked, without an asserted completeness of research.
    SourceChecked,
    /// Sources checked but questions or proposed conflicts remain unresolved.
    Partial,
    /// Existing accounting records cancellation; retained evidence remains readable.
    Cancelled,
    /// Original research deadline elapsed or accounting already recorded expiry.
    Expired,
}

/// Read-only source projection; publication time and publisher independence are unknown.
#[derive(Serialize)]
pub struct ResearchReportSource {
    bundle: RuntimeArtifactRef,
    receipt_id: ReceiptId,
    target: PublicGetTarget,
    worker_reported_url: Option<String>,
    body_sha256: String,
    retrieved_at_epoch_ms: u64,
    #[serde(skip)]
    retained_artifacts: Vec<RuntimeArtifactRef>,
}

impl ResearchReportSource {
    /// Exact source bytes identity, not a provider snippet or report hash.
    #[must_use]
    pub fn body_sha256(&self) -> &str {
        &self.body_sha256
    }

    /// URL spelling from the native worker, or None for original version-one frames.
    #[must_use]
    pub fn worker_reported_url(&self) -> Option<&str> {
        self.worker_reported_url.as_deref()
    }
}

/// Checked only at its recorded read time; no deserializer or public constructor.
/// Serializing this value cannot mint an artifact receipt or preserve future freshness.
/// ```compile_fail
/// use agentmage_kernel_engine::research_report::CanonicalResearchReport;
/// let _: CanonicalResearchReport = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use agentmage_kernel_engine::research_report::CanonicalResearchReport;
/// fn duplicate(report: &CanonicalResearchReport) -> CanonicalResearchReport {
///     report.clone()
/// }
/// ```
#[derive(Serialize)]
pub struct CanonicalResearchReport {
    schema_version: u16,
    session_id: SessionId,
    task_id: TaskId,
    run_id: RuntimeRunId,
    policy_sha256: String,
    checked_at_epoch_ms: u64,
    accounting_head_sha256: String,
    disposition: ResearchReportDisposition,
    draft: ResearchReportDraft,
    sources: Vec<ResearchReportSource>,
    excerpt_sha256: Vec<String>,
}

impl std::fmt::Debug for CanonicalResearchReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalResearchReport")
            .field("source_count", &self.sources.len())
            .field("disposition", &self.disposition)
            .finish_non_exhaustive()
    }
}

impl CanonicalResearchReport {
    /// A report state, never runtime success or model qualification.
    #[must_use]
    pub const fn disposition(&self) -> ResearchReportDisposition {
        self.disposition
    }

    /// Checked source table with exact canonical identities and original clocks.
    #[must_use]
    pub fn sources(&self) -> &[ResearchReportSource] {
        &self.sources
    }

    /// Preserves exact quotation spans, inference limitations and unresolved conflicts.
    #[must_use]
    pub const fn draft(&self) -> &ResearchReportDraft {
        &self.draft
    }

    /// Bounded inert display JSON; raw bodies stay private. This is not a retained
    /// draft or reusable evidence. Persist drafts through the dedicated canonical
    /// path so future use rechecks original source bundles and their lifecycle.
    pub fn encode(&self) -> Result<Vec<u8>, ResearchReportError> {
        let bytes = serde_json::to_vec(self).map_err(|_| ResearchReportError::Binding)?;
        if bytes.len() > MAX_REPORT_BYTES {
            return Err(ResearchReportError::Limit);
        }
        Ok(bytes)
    }
}

/// Content-free failure; an invalid source cannot be downgraded to a partial report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchReportError {
    /// Malformed or unsafe untrusted shape.
    Shape(ResearchReportShapeError),
    /// Failed canonical source verification.
    Source(ResearchRetrievalError),
    /// Failed original canonical accounting verification.
    Accounting(ResearchJournalError),
    /// Retained draft publication or lifecycle refused by the existing artifact owner.
    Artifact(crate::runtime_artifact::RuntimeArtifactStoreError),
    /// Excerpt identity or exact UTF-8 range differs, or a receipt is duplicated.
    Binding,
    /// Source, report or aggregate quotation ceiling exceeded.
    Limit,
    /// Detected secret in proposed report text.
    Secret,
}

impl ResearchReportError {
    pub(crate) const fn poisons_runtime(self) -> bool {
        matches!(self, Self::Source(ResearchRetrievalError::Integrity))
            || matches!(self, Self::Accounting(error) if error.poisons_runtime())
            || matches!(self, Self::Artifact(error) if error.poisons_runtime())
    }
}

// Called while holding the existing owner's one canonical mutex. Sources cannot
// change canonical lifecycle between validation and assembling this projection.
pub(crate) fn read<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    coordinator: &AuthorityTransactionCoordinator,
    issuer: &GrantIssuer,
    registry: &ToolRegistry,
    request: &ResearchReportReadRequest<'_>,
) -> Result<CanonicalResearchReport, ResearchReportError> {
    use ResearchReportError as E;
    let draft = request.draft;
    draft.validate().map_err(E::Shape)?;
    let accounting = research_journal::state(store, request.context).map_err(E::Accounting)?;
    if draft.sources.len() > usize::from(accounting.scope.limits().visits) {
        return Err(E::Limit);
    }
    let mut sources = Vec::with_capacity(draft.sources.len());
    let mut receipts = BTreeSet::new();
    let mut budget = QuoteBudget::default();
    let mut total_bytes = 0_u64;
    for (index, bundle) in draft.sources.iter().enumerate() {
        let source = research_retrieval::read(
            store,
            payloads,
            coordinator,
            issuer,
            registry,
            &PublicGetReadRequest {
                context: request.context,
                bundle,
                expected_native: request.expected_native,
                now_epoch_ms: request.now_epoch_ms,
            },
        )
        .map_err(E::Source)?;
        if !receipts.insert(source.receipt_id().clone()) {
            return Err(E::Binding);
        }
        let response = source.response();
        let observed = response.observation();
        let target = &observed.hops.last().ok_or(E::Binding)?.target;
        total_bytes = total_bytes
            .checked_add(response.body().len() as u64)
            .ok_or(E::Limit)?;
        if total_bytes > accounting.scope.limits().downloaded_bytes {
            return Err(E::Limit);
        }
        let location_bytes = serde_json::to_vec(target).map_err(|_| E::Binding)?;
        let location_digest: String = Sha256::digest(location_bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        for excerpt in draft
            .spans
            .iter()
            .filter(|s| usize::from(s.source_index) == index)
        {
            budget
                .check(
                    excerpt,
                    response.body(),
                    &observed.body_sha256,
                    &location_digest,
                )
                .map_err(|error| match error {
                    SpanError::Invalid | SpanError::Binding => E::Binding,
                    SpanError::Limit => E::Limit,
                    SpanError::Secret => E::Secret,
                })?;
        }
        sources.push(ResearchReportSource {
            bundle: source.bundle().clone(),
            receipt_id: source.receipt_id().clone(),
            target: target.clone(),
            worker_reported_url: response
                .worker_reported_urls()
                .and_then(|u| u.last())
                .cloned(),
            body_sha256: observed.body_sha256.clone(),
            retrieved_at_epoch_ms: observed.completed_epoch_ms,
            retained_artifacts: source.retained_artifacts().to_vec(),
        });
    }
    let deadline = accounting
        .progress
        .started_epoch_ms
        .checked_add(accounting.scope.limits().elapsed_ms)
        .ok_or(E::Binding)?;
    // At least one complete canonical read above has verified and bounded this
    // run's history under the same lock. Cancellation can precede its separate
    // budget append, so do not rely only on the accounting flag.
    let events = crate::runtime_journal::load_run_events(store, &request.context.run_id)
        .map_err(|_| E::Source(ResearchRetrievalError::Integrity))?;
    let cancellation_recorded = events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::CancellationRequested { .. }
                | RuntimeEventKind::CancellationObserved { .. }
        )
    });
    let disposition = if accounting.progress.cancelled || cancellation_recorded {
        ResearchReportDisposition::Cancelled
    } else if accounting.progress.deadline_exhausted || request.now_epoch_ms >= deadline {
        ResearchReportDisposition::Expired
    } else if !draft.unresolved.is_empty() || !draft.conflicts.is_empty() {
        ResearchReportDisposition::Partial
    } else {
        ResearchReportDisposition::SourceChecked
    };
    let report = CanonicalResearchReport {
        schema_version: 1,
        session_id: request.context.session_id.clone(),
        task_id: request.context.task_id.clone(),
        run_id: request.context.run_id.clone(),
        policy_sha256: request.context.policy_sha256.clone(),
        checked_at_epoch_ms: request.now_epoch_ms,
        accounting_head_sha256: accounting.head_sha256,
        disposition,
        draft: draft.clone(),
        sources,
        excerpt_sha256: draft.spans.iter().map(excerpt_sha256).collect(),
    };
    report.encode()?;
    Ok(report)
}
