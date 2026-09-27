//! Read-only research provenance through the existing canonical owners.
//! No storage, execution, authority issuer or network interface is introduced.
//! Native identity must come from independently admitted trusted composition;
//! even a consistent canonical record cannot itself qualify a native producer.

use agentmage_kernel_contracts::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::authority_transaction::AuthorityTransactionCoordinator;
use crate::grants::GrantIssuer;
use crate::operational_store::OperationalStore;
use crate::research_fetch::{PublicGetDraft, PublicGetWorkerPacket};
use crate::research_journal::{ResearchBudgetContext, verify_retained_reservation};
use crate::research_result_binding::{PublicGetNativeIdentity, PublicGetResultBinding};
use crate::runtime_artifact::{
    RuntimeArtifactPayloadStore, RuntimeArtifactReadRequest, load_artifact_manifest,
    read_runtime_artifact,
};
use crate::runtime_journal::{current_cursor, load_run_events};
use crate::tooling::ToolRegistry;

const MAX_BUNDLE: u64 = 16 * 1024;
const MAX_CALL: u64 = 128 * 1024;
const MAX_RESULT: u64 = 64 * 1024;
const MAX_FRAME: u64 = 4 * 1024 * 1024 + 64 * 1024 + 4;
// A source read must not allocate an arbitrarily long run's journal. These are
// read-work limits, not event retention limits or permission to omit history.
const MAX_HISTORY_EVENTS: i64 = 16_384;
const MAX_HISTORY_BYTES: i64 = 8 * 1024 * 1024;

/// Closed output payload of a successful research tool. References are descriptive,
/// never evidence until the canonical consumer verifies all complete stored bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicGetArtifactOutput {
    /// Only version one of this internal payload is admitted.
    pub schema_version: u16,
    /// Complete original request, not a reconstructed replacement.
    pub packet: RuntimeArtifactRef,
    /// Exact redacted native material hashed into the authority transaction.
    pub material: RuntimeArtifactRef,
    /// Complete response frame, including all original source bytes.
    pub frame: RuntimeArtifactRef,
}

/// A Report artifact joins existing objects without introducing a second store.
/// Publish after the terminal commit; it cannot reference itself through ToolResult.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicGetArtifactBundle {
    /// Only version one of this internal payload is admitted.
    pub schema_version: u16,
    /// Exact transaction, looked up from the real coordinator, never supplied as data.
    pub transaction_id: AuthorityTransactionId,
    /// Original committed reservation revision, not necessarily today's budget head.
    pub reservation_sha256: String,
    /// Complete original canonical ToolCall.
    pub call: RuntimeArtifactRef,
    /// Exact output references also bound into the retained ToolResult.
    pub output: PublicGetArtifactOutput,
    /// Complete canonical ToolResult hashed into the actual ToolCompleted event.
    pub result: RuntimeArtifactRef,
}

/// A read restriction, not permission or evidence supplied by a model/provider.
pub struct PublicGetReadRequest<'a> {
    /// Exact current owning session/task/run/policy.
    pub context: &'a ResearchBudgetContext,
    /// Bundle to resolve through the canonical artifact owner.
    pub bundle: &'a RuntimeArtifactRef,
    /// Independently admitted native identities from trusted composition.
    pub expected_native: &'a PublicGetNativeIdentity,
    /// Current trusted read clock, distinct from the original dispatch clock.
    pub now_epoch_ms: u64,
}

/// Content-free refusal. Storage/integrity faults poison the canonical owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchRetrievalError {
    /// Complete event history exceeds this read's count or byte budget.
    Limit,
    /// Missing, non-successful or mismatched canonical authority/receipt.
    Authority,
    /// Full artifact unavailable, stale, foreign, released, corrupt or unpublished.
    Artifact,
    /// Noncanonical/unknown data, mismatched output, packet or native identity.
    Binding,
    /// Broken canonical event/accounting chain or storage access.
    Integrity,
}

/// Complete point-in-time canonical source. Has no public constructor, clone,
/// deserializer, dispatch method or claim of independent producer admission.
/// ```compile_fail
/// use agentmage_kernel_contracts::{ReceiptId, RuntimeArtifactRef};
/// use agentmage_kernel_engine::research_result_binding::PublicGetResultBinding;
/// use agentmage_kernel_engine::research_retrieval::CanonicalPublicGetSource;
/// fn forge(
///     bundle: RuntimeArtifactRef,
///     receipt_id: ReceiptId,
///     binding: PublicGetResultBinding,
/// ) -> CanonicalPublicGetSource {
///     CanonicalPublicGetSource { bundle, receipt_id, binding }
/// }
/// ```
/// ```compile_fail
/// use agentmage_kernel_engine::research_retrieval::CanonicalPublicGetSource;
/// fn duplicate(source: CanonicalPublicGetSource) { let _ = source.clone(); }
/// ```
/// ```compile_fail
/// use agentmage_kernel_engine::research_retrieval::CanonicalPublicGetSource;
/// let _: CanonicalPublicGetSource = serde_json::from_str("{}").unwrap();
/// ```
pub struct CanonicalPublicGetSource {
    bundle: RuntimeArtifactRef,
    receipt_id: ReceiptId,
    binding: PublicGetResultBinding,
}

impl std::fmt::Debug for CanonicalPublicGetSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanonicalPublicGetSource")
            .field(
                "body_sha256",
                &self.binding.response().observation().body_sha256,
            )
            .finish_non_exhaustive()
    }
}

impl CanonicalPublicGetSource {
    /// Full inert response verified at this read; later reads must recheck lifecycle.
    #[must_use]
    pub const fn response(&self) -> &crate::research_response::PublicGetResponse {
        self.binding.response()
    }

    /// Exact bundle identity to retain for subsequent drift-aware revalidation.
    #[must_use]
    pub const fn bundle(&self) -> &RuntimeArtifactRef {
        &self.bundle
    }

    /// Actual coordinator receipt, not provider-supplied citation metadata.
    #[must_use]
    pub const fn receipt_id(&self) -> &ReceiptId {
        &self.receipt_id
    }
}

// Invoked only by DurableAuthorityRuntime after ensure_usable, while holding its
// existing shared store lock. Authority comes from its actual owners, not arguments
// accepted by a public free function.
pub(crate) fn read<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    coordinator: &AuthorityTransactionCoordinator,
    issuer: &GrantIssuer,
    registry: &ToolRegistry,
    request: &PublicGetReadRequest<'_>,
) -> Result<CanonicalPublicGetSource, ResearchRetrievalError> {
    use ResearchRetrievalError as E;
    verify_history_budget(store, &request.context.run_id)?;
    current_cursor(store, &request.context.run_id)
        .map_err(|_| E::Integrity)?
        .ok_or(E::Binding)?;
    let events = load_run_events(store, &request.context.run_id).map_err(|_| E::Integrity)?;
    // Reads do not advance any clock or spend budget, but cannot precede a later
    // event already committed in this same verified run. In particular, a stale
    // caller clock must not make expired source readable again.
    if events
        .last()
        .is_none_or(|event| event.occurred_at_epoch_ms > request.now_epoch_ms)
    {
        return Err(E::Binding);
    }
    let bundle_bytes = read_bytes(store, payloads, request, request.bundle, MAX_BUNDLE)?;
    let bundle: PublicGetArtifactBundle = decode_local(&bundle_bytes)?;
    if bundle.schema_version != 1 || bundle.output.schema_version != 1 {
        return Err(E::Binding);
    }
    let transaction = coordinator
        .current(&bundle.transaction_id)
        .ok_or(E::Authority)?;
    if transaction.state != AuthorityTransactionState::Terminal
        || transaction.outcome != Some(OperationOutcome::Succeeded)
        || transaction.uncertain_effect
        || transaction.session_id != request.context.session_id
        || transaction.task_id != request.context.task_id
        || transaction.operation != OperationBinding::new(GrantOperation::NetworkAccess)
    {
        return Err(E::Authority);
    }
    let receipt = coordinator
        .receipts()
        .iter()
        .find(|r| Some(&r.receipt_id) == transaction.receipt_id.as_ref())
        .ok_or(E::Authority)?;
    if receipt.authority_transaction_id != transaction.authority_transaction_id
        || receipt.operation_attempt_id != transaction.operation_attempt_id
        || receipt.approval_id != transaction.approval_id
        || receipt.grant_id != transaction.grant_id
        || receipt.session_id != transaction.session_id
        || receipt.task_id != transaction.task_id
        || receipt.action_id != transaction.action_id
        || receipt.tool_call_id.as_ref() != Some(&transaction.tool_call_id)
        || receipt.correlation_id != transaction.correlation_id
        || receipt.operation != transaction.operation
        || receipt.outcome != OperationOutcome::Succeeded
        || receipt.error.is_some()
        || Some(&receipt.receipt_sha256) != transaction.receipt_sha256.as_ref()
    {
        return Err(E::Authority);
    }
    let call_bytes = read_bytes(store, payloads, request, &bundle.call, MAX_CALL)?;
    let call: ToolCall = from_json(&call_bytes).map_err(|_| E::Binding)?;
    if to_canonical_json(&call).map_err(|_| E::Binding)? != call_bytes
        || call.tool_call_id != transaction.tool_call_id
        || call.action_id != transaction.action_id
        || call.correlation_id != transaction.correlation_id
    {
        return Err(E::Binding);
    }
    let definition = registry.validate_arguments(&call).map_err(|_| E::Binding)?;
    if definition.declared_effects != [transaction.operation]
        || definition.required_grant.operation != transaction.operation
        || !definition.required_grant.single_use
    {
        return Err(E::Binding);
    }
    let requested = unique_event(&events, |e| {
        matches!(&e.kind,
        RuntimeEventKind::ToolRequested { tool_call_id, arguments_sha256 }
        if tool_call_id == &call.tool_call_id && arguments_sha256 == &call.arguments.sha256)
    })?;
    let started = unique_event(&events, |e| {
        matches!(&e.kind,
        RuntimeEventKind::ToolStarted { tool_call_id, .. } if tool_call_id == &call.tool_call_id)
    })?;
    let completed = unique_event(&events, |e| {
        matches!(&e.kind,
        RuntimeEventKind::ToolCompleted { tool_call_id, receipt_id, .. }
        if tool_call_id == &call.tool_call_id && receipt_id == &receipt.receipt_id)
    })?;
    if requested.sequence >= started.sequence
        || started.sequence >= completed.sequence
        || started.operation_id.is_none()
        || started.turn_id.is_none()
        || completed.occurred_at_epoch_ms > request.now_epoch_ms
        || [requested, started, completed].iter().any(|event| {
            event.session_id != request.context.session_id
                || event.task_id != request.context.task_id
                || event.run_id != request.context.run_id
                || event.turn_id != started.turn_id
                || event.operation_id != started.operation_id
                || event.policy_id != started.policy_id
                || event.correlation_id != call.correlation_id
        })
    {
        return Err(E::Binding);
    }
    let packet_bytes = read_bytes(store, payloads, request, &bundle.output.packet, 16 * 1024)?;
    let packet =
        PublicGetWorkerPacket::decode_worker_packet(&packet_bytes, started.occurred_at_epoch_ms)
            .map_err(|_| E::Binding)?;
    let draft: PublicGetDraft =
        serde_json::from_slice(&call.arguments.bytes).map_err(|_| E::Binding)?;
    if &draft != packet.request()
        || draft.operation_id != call.tool_call_id.as_str()
        || call.arguments.media_type != "application/json"
        || call.arguments.bytes.len() > 16 * 1024
        || packet.task_id() != request.context.task_id.as_str()
        || definition.timeout_ms < draft.timeout_ms
    {
        return Err(E::Binding);
    }
    verify_grant(
        issuer,
        transaction,
        &call,
        &packet,
        started,
        &request.context.policy_sha256,
    )?;
    verify_retained_reservation(
        store,
        payloads,
        request.context,
        &packet,
        &bundle.reservation_sha256,
        request.now_epoch_ms,
    )
    .map_err(|e| {
        if e.poisons_runtime() {
            E::Integrity
        } else {
            E::Binding
        }
    })?;
    let material = read_bytes(store, payloads, request, &bundle.output.material, 4096)?;
    if transaction.result_sha256.as_deref() != Some(digest(&material).as_str()) {
        return Err(E::Authority);
    }
    let frame = read_bytes(store, payloads, request, &bundle.output.frame, MAX_FRAME)?;
    let binding = PublicGetResultBinding::decode_consistent(
        &packet,
        &bundle.reservation_sha256,
        &material,
        &frame,
    )
    .map_err(|_| E::Binding)?;
    if binding.native_identity() != request.expected_native
        || binding.parent_interval().started_epoch_ms < started.occurred_at_epoch_ms
        || binding.parent_interval().cleanup_verified_epoch_ms > completed.occurred_at_epoch_ms
    {
        return Err(E::Binding);
    }
    let result_bytes = read_bytes(store, payloads, request, &bundle.result, MAX_RESULT)?;
    let result: ToolResult = from_json(&result_bytes).map_err(|_| E::Binding)?;
    let output = result.output.as_ref().ok_or(E::Binding)?;
    if to_canonical_json(&result).map_err(|_| E::Binding)? != result_bytes
        || result.tool_call_id != call.tool_call_id
        || result.correlation_id != call.correlation_id
        || result.outcome != OperationOutcome::Succeeded
        || result.error.is_some()
        || !result.validation_issues.is_empty()
        || result.state_change != StateChange::Changed
        || result.elapsed_ms
            > completed
                .occurred_at_epoch_ms
                .saturating_sub(started.occurred_at_epoch_ms)
        || output.schema != definition.output_schema
        || output.media_type != "application/json"
        || output.bytes.len() as u64 > MAX_BUNDLE
        || digest(&output.bytes) != output.sha256
        || decode_local::<PublicGetArtifactOutput>(&output.bytes)? != bundle.output
        || !matches!(&completed.kind, RuntimeEventKind::ToolCompleted { result_sha256, .. }
            if result_sha256 == &digest(&result_bytes))
    {
        return Err(E::Binding);
    }
    let refs = [
        (&bundle.call, "application/json"),
        (&bundle.output.packet, "application/json"),
        (&bundle.output.material, "application/json"),
        (&bundle.output.frame, "application/octet-stream"),
        (&bundle.result, "application/json"),
        (request.bundle, "application/json"),
    ];
    let mut ids = std::collections::BTreeSet::new();
    for (reference, media) in refs {
        if reference.media_type != media || !ids.insert(&reference.artifact_id) {
            return Err(E::Artifact);
        }
        verify_publication(store, &events, request, reference, completed, receipt)?;
    }
    Ok(CanonicalPublicGetSource {
        bundle: request.bundle.clone(),
        receipt_id: receipt.receipt_id.clone(),
        binding,
    })
}

fn verify_history_budget(
    store: &OperationalStore,
    run_id: &RuntimeRunId,
) -> Result<(), ResearchRetrievalError> {
    // Count at most one row beyond the cap, without materializing JSON into Rust.
    // The existing owner holds its store lock throughout this check and the read.
    let (count, bytes): (i64, i64) = store
        .connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(length(record_json)), 0) FROM (
            SELECT record_json FROM runtime_events WHERE run_id = ?1 LIMIT ?2
        )",
            rusqlite::params![run_id.as_str(), MAX_HISTORY_EVENTS + 1],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| ResearchRetrievalError::Integrity)?;
    if !(0..=MAX_HISTORY_EVENTS).contains(&count) || !(0..=MAX_HISTORY_BYTES).contains(&bytes) {
        return Err(ResearchRetrievalError::Limit);
    }
    Ok(())
}

fn verify_grant(
    issuer: &GrantIssuer,
    transaction: &AuthorityTransactionRecord,
    call: &ToolCall,
    packet: &PublicGetWorkerPacket,
    started: &RuntimeEvent,
    policy: &str,
) -> Result<(), ResearchRetrievalError> {
    use ResearchRetrievalError as E;
    let history = issuer.history(&transaction.grant_id).ok_or(E::Authority)?;
    let issued = history
        .iter()
        .find(|g| g.status == GrantStatus::Issued)
        .ok_or(E::Authority)?;
    let consumed = history
        .iter()
        .find(|g| g.status == GrantStatus::Consumed)
        .ok_or(E::Authority)?;
    if transaction.consumed_grant_sha256.as_deref()
        != Some(digest(&to_canonical_json(consumed).map_err(|_| E::Binding)?).as_str())
        || !matches!(&started.kind, RuntimeEventKind::ToolStarted { authority_sha256, .. }
            if authority_sha256 == &digest(&to_canonical_json(issued).map_err(|_| E::Binding)?))
        || issued.operation != transaction.operation
        || issued.session_id != transaction.session_id
        || issued.task_id != transaction.task_id
        || issued.approval_id.as_ref() != Some(&transaction.approval_id)
        || issued.action_id.as_ref() != Some(&transaction.action_id)
        || issued.tool_id.as_ref() != Some(&call.tool_id)
        || issued.tool_version.as_ref() != Some(&call.tool_version)
        || issued.argument_sha256 != call.arguments.sha256
        || issued.policy_sha256 != policy
        || issued.use_limit != 1
        || issued.use_count != 0
        || consumed.use_count != 1
        || issued.issued_at_epoch_ms > started.occurred_at_epoch_ms
        || issued.expires_at_epoch_ms < packet.deadline_epoch_ms()
        || issued.targets.len() != 1
        || issued.expected_side_effects.len() != 1
        || issued.expected_side_effects[0].operation != transaction.operation
        || issued.expected_side_effects[0].target_indexes != [0]
        || issued.expected_side_effects[0].details_sha256 != packet.sha256()
    {
        return Err(E::Authority);
    }
    Ok(())
}

fn read_bytes<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    request: &PublicGetReadRequest<'_>,
    reference: &RuntimeArtifactRef,
    maximum_bytes: u64,
) -> Result<Vec<u8>, ResearchRetrievalError> {
    read_runtime_artifact(
        store,
        payloads,
        &RuntimeArtifactReadRequest {
            session_id: request.context.session_id.clone(),
            task_id: request.context.task_id.clone(),
            policy_sha256: request.context.policy_sha256.clone(),
            reference: reference.clone(),
            now_epoch_ms: request.now_epoch_ms,
            maximum_bytes,
        },
    )
    .map_err(artifact_error)
}

fn artifact_error(
    error: crate::runtime_artifact::RuntimeArtifactStoreError,
) -> ResearchRetrievalError {
    if error.poisons_runtime() {
        ResearchRetrievalError::Integrity
    } else {
        ResearchRetrievalError::Artifact
    }
}

fn verify_publication(
    store: &OperationalStore,
    events: &[RuntimeEvent],
    request: &PublicGetReadRequest<'_>,
    reference: &RuntimeArtifactRef,
    completed: &RuntimeEvent,
    receipt: &Receipt,
) -> Result<(), ResearchRetrievalError> {
    use ResearchRetrievalError as E;
    let manifest = load_artifact_manifest(store, &reference.artifact_id).map_err(artifact_error)?;
    if manifest.kind != RuntimeArtifactKind::Report
        || manifest.session_id != request.context.session_id
        || manifest.task_id != request.context.task_id
        || manifest.producer_run_id != request.context.run_id
        || manifest.producer_turn_id != completed.turn_id
        || manifest.producer_operation_id != completed.operation_id
        || manifest.receipt_id.as_ref() != Some(&receipt.receipt_id)
        || manifest.policy_sha256 != request.context.policy_sha256
        || manifest.policy_id != completed.policy_id
        || manifest.created_at_epoch_ms < completed.occurred_at_epoch_ms
        || manifest.created_at_epoch_ms > request.now_epoch_ms
    {
        return Err(E::Artifact);
    }
    let event = unique_event(events, |e| {
        matches!(&e.kind, RuntimeEventKind::ArtifactCreated {
        artifact_id, manifest_sha256 } if artifact_id == &reference.artifact_id && manifest_sha256 == &reference.manifest_sha256)
    })?;
    if event.sequence <= completed.sequence
        || event.turn_id != completed.turn_id
        || event.operation_id != completed.operation_id
        || event.correlation_id != completed.correlation_id
        || event.policy_id != completed.policy_id
        || event.occurred_at_epoch_ms < manifest.created_at_epoch_ms
        || event.occurred_at_epoch_ms > request.now_epoch_ms
        || !event.payload_reference.as_ref().is_some_and(|p| {
            p.artifact_id == reference.artifact_id
                && p.sha256 == reference.payload_sha256
                && p.byte_size == reference.byte_size
                && p.media_type == reference.media_type
        })
    {
        return Err(E::Artifact);
    }
    Ok(())
}

fn unique_event(
    events: &[RuntimeEvent],
    predicate: impl Fn(&RuntimeEvent) -> bool,
) -> Result<&RuntimeEvent, ResearchRetrievalError> {
    let mut matches = events.iter().filter(|event| predicate(event));
    let event = matches.next().ok_or(ResearchRetrievalError::Binding)?;
    if matches.next().is_some() {
        return Err(ResearchRetrievalError::Binding);
    }
    Ok(event)
}

fn decode_local<T: for<'de> Deserialize<'de> + Serialize>(
    bytes: &[u8],
) -> Result<T, ResearchRetrievalError> {
    let value: T = serde_json::from_slice(bytes).map_err(|_| ResearchRetrievalError::Binding)?;
    if serde_json::to_vec(&value).map_err(|_| ResearchRetrievalError::Binding)? != bytes {
        return Err(ResearchRetrievalError::Binding);
    }
    Ok(value)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
