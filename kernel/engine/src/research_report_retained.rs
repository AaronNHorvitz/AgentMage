//! Retained drafts are untrusted inputs, never cached canonical projections.
use agentmage_kernel_contracts::{
    ContextSensitivity, RuntimeArtifactManifest, RuntimeArtifactRef, RuntimeEventKind,
    RuntimeEventRetentionKind,
};

use super::{
    CanonicalResearchReport, ResearchReportDraft, ResearchReportError as E,
    ResearchReportReadRequest,
};
use crate::authority_transaction::AuthorityTransactionCoordinator;
use crate::grants::GrantIssuer;
use crate::operational_store::OperationalStore;
use crate::research_journal::ResearchBudgetContext;
use crate::research_result_binding::PublicGetNativeIdentity;
use crate::research_retrieval::{ResearchRetrievalError, verify_history_budget};
use crate::runtime_artifact::{
    RESEARCH_REPORT_DRAFT_MEDIA_TYPE, RuntimeArtifactPayloadStore, RuntimeArtifactPublication,
    RuntimeArtifactReadRequest, load_artifact_manifest, publish_research_draft,
    read_research_draft, runtime_payload_reference, verify_runtime_artifact_manifest,
};
use crate::runtime_journal::load_run_events;
use crate::tooling::ToolRegistry;

/// Proposed publication, still subject to fresh sources and canonical metadata.
pub struct ResearchReportPublicationRequest<'a> {
    /// Fresh source checks, owning context and current trusted clock.
    pub read: ResearchReportReadRequest<'a>,
    /// Exact closed draft byte identity; no checked snapshot or preview is accepted.
    pub manifest: &'a RuntimeArtifactManifest,
}

/// A retained reference is only an input to reconstruction, never evidence itself.
pub struct RetainedResearchReportReadRequest<'a> {
    /// Original owning session, task, run and policy; no cross-run widening.
    pub context: &'a ResearchBudgetContext,
    /// Exact retained draft identity from the canonical artifact owner.
    pub reference: &'a RuntimeArtifactRef,
    /// Independently admitted producer identity, never loaded from the draft.
    pub expected_native: &'a PublicGetNativeIdentity,
    /// Trusted current clock; original retrieval and deadline clocks are unchanged.
    pub now_epoch_ms: u64,
}

// Both entry points execute under DurableAuthorityRuntime's existing store lock.
pub(crate) fn publish_draft<S: RuntimeArtifactPayloadStore>(
    store: &mut OperationalStore,
    payloads: &mut S,
    coordinator: &AuthorityTransactionCoordinator,
    issuer: &GrantIssuer,
    registry: &ToolRegistry,
    request: &ResearchReportPublicationRequest<'_>,
) -> Result<RuntimeArtifactPublication, E> {
    verify_runtime_artifact_manifest(request.manifest)
        .map_err(|error| E::Artifact(error.into()))?;
    let checked = super::read(
        store,
        payloads,
        coordinator,
        issuer,
        registry,
        &request.read,
    )?;
    verify_manifest(store, request.manifest, &checked)?;
    if request.manifest.created_at_epoch_ms != request.read.now_epoch_ms {
        return Err(E::Binding);
    }
    // The source reader has already verified and bounded this full run's history.
    let events = load_run_events(store, &request.read.context.run_id)
        .map_err(|_| E::Source(ResearchRetrievalError::Integrity))?;
    if events
        .last()
        .is_none_or(|event| matches!(event.kind, RuntimeEventKind::RunTerminal { .. }))
    {
        return Err(E::Binding);
    }
    let bytes = serde_json::to_vec(request.read.draft).map_err(|_| E::Binding)?;
    // The ordinary payload owner verifies exact size/digest before placement. Its
    // special entry refuses an existing ordinary-media alias of these bytes.
    publish_research_draft(store, payloads, request.manifest.clone(), &bytes).map_err(E::Artifact)
}

pub(crate) fn read_retained<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    coordinator: &AuthorityTransactionCoordinator,
    issuer: &GrantIssuer,
    registry: &ToolRegistry,
    request: &RetainedResearchReportReadRequest<'_>,
) -> Result<CanonicalResearchReport, E> {
    // Bound the journal before loading any full history, including malformed drafts.
    verify_history_budget(store, &request.context.run_id).map_err(E::Source)?;
    let bytes = read_research_draft(
        store,
        payloads,
        &RuntimeArtifactReadRequest {
            session_id: request.context.session_id.clone(),
            task_id: request.context.task_id.clone(),
            policy_sha256: request.context.policy_sha256.clone(),
            reference: request.reference.clone(),
            now_epoch_ms: request.now_epoch_ms,
            maximum_bytes: 128 * 1024,
        },
    )
    .map_err(E::Artifact)?;
    let draft = ResearchReportDraft::decode(&bytes).map_err(E::Shape)?;
    if serde_json::to_vec(&draft).map_err(|_| E::Binding)? != bytes {
        return Err(E::Binding);
    }
    let checked = super::read(
        store,
        payloads,
        coordinator,
        issuer,
        registry,
        &ResearchReportReadRequest {
            context: request.context,
            draft: &draft,
            expected_native: request.expected_native,
            now_epoch_ms: request.now_epoch_ms,
        },
    )?;
    let manifest =
        load_artifact_manifest(store, &request.reference.artifact_id).map_err(E::Artifact)?;
    verify_manifest(store, &manifest, &checked)?;
    let events = load_run_events(store, &request.context.run_id)
        .map_err(|_| E::Source(ResearchRetrievalError::Integrity))?;
    let mut publications = events.iter().filter(|event| {
        matches!(&event.kind,
        RuntimeEventKind::ArtifactCreated { artifact_id, .. }
            if artifact_id == &request.reference.artifact_id)
    });
    let event = publications.next().ok_or(E::Binding)?;
    if publications.next().is_some()
        || !matches!(&event.kind, RuntimeEventKind::ArtifactCreated { manifest_sha256, .. }
            if manifest_sha256 == &request.reference.manifest_sha256)
        || event.session_id != manifest.session_id
        || event.task_id != manifest.task_id
        || event.run_id != manifest.producer_run_id
        || event.turn_id != manifest.producer_turn_id
        || event.operation_id != manifest.producer_operation_id
        || event.policy_id != manifest.policy_id
        || event.sensitivity != manifest.sensitivity
        || event.retention != manifest.retention
        || event.occurred_at_epoch_ms < manifest.created_at_epoch_ms
        || event.occurred_at_epoch_ms > request.now_epoch_ms
        || event.payload_reference.as_ref()
            != Some(
                &runtime_payload_reference(&manifest).map_err(|error| E::Artifact(error.into()))?,
            )
    {
        return Err(E::Binding);
    }
    for source in &checked.sources {
        for reference in &source.retained_artifacts {
            if events.iter().any(|upstream| {
                matches!(&upstream.kind,
                RuntimeEventKind::ArtifactCreated { artifact_id, .. }
                    if artifact_id == &reference.artifact_id)
                    && upstream.sequence >= event.sequence
            }) {
                return Err(E::Binding);
            }
        }
    }
    Ok(checked)
}

fn verify_manifest(
    store: &OperationalStore,
    manifest: &RuntimeArtifactManifest,
    checked: &CanonicalResearchReport,
) -> Result<(), E> {
    if manifest.media_type != RESEARCH_REPORT_DRAFT_MEDIA_TYPE
        || manifest.session_id != checked.session_id
        || manifest.task_id != checked.task_id
        || manifest.producer_run_id != checked.run_id
        || manifest.policy_sha256 != checked.policy_sha256
        || manifest.created_at_epoch_ms > checked.checked_at_epoch_ms
    {
        return Err(E::Binding);
    }
    // Immutable source references were freshly resolved by this same locked read.
    // No cached draft metadata chooses classification or lengthens retention.
    for source in &checked.sources {
        for reference in &source.retained_artifacts {
            let upstream =
                load_artifact_manifest(store, &reference.artifact_id).map_err(E::Artifact)?;
            if sensitivity_rank(manifest.sensitivity) < sensitivity_rank(upstream.sensitivity)
                || manifest.created_at_epoch_ms < upstream.created_at_epoch_ms
                || manifest.retention.kind != upstream.retention.kind
                || match upstream.retention.kind {
                    RuntimeEventRetentionKind::Session => false,
                    RuntimeEventRetentionKind::UntilExpiration => match (
                        manifest.retention.expires_at_epoch_ms,
                        upstream.retention.expires_at_epoch_ms,
                    ) {
                        (Some(draft), Some(source)) => draft > source,
                        _ => true,
                    },
                    RuntimeEventRetentionKind::Ephemeral | RuntimeEventRetentionKind::UserHold => {
                        true
                    }
                }
                || manifest.policy_id != upstream.policy_id
            {
                return Err(E::Binding);
            }
        }
    }
    Ok(())
}

const fn sensitivity_rank(value: ContextSensitivity) -> u8 {
    match value {
        ContextSensitivity::Public => 0,
        ContextSensitivity::Internal => 1,
        ContextSensitivity::Private => 2,
        ContextSensitivity::Restricted => 3,
    }
}
