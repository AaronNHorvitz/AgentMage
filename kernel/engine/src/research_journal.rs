//! Conservative research accounting in the existing encrypted operational store.
//!
//! Accounting is not network authority. Only a committed fresh reservation may be
//! passed to the separately admitted native driver, alongside an exact consumed grant.

use agentmage_kernel_contracts::{
    RuntimeArtifactKind, RuntimeArtifactRef, RuntimeEvent, RuntimeEventKind, RuntimeRunId,
    SessionId, TaskId,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::operational_store::OperationalStore;
use crate::research_budget::{
    ResearchBudget, ResearchBudgetError, ResearchBudgetProgress, ResearchScope,
};
use crate::research_fetch::{PreparedPublicGet, PublicGetWorkerPacket};
use crate::research_plan::PreparedResearchPlan;
use crate::runtime_artifact::{
    RuntimeArtifactPayloadStore, RuntimeArtifactReadRequest, load_artifact_manifest,
    read_runtime_artifact, verify_runtime_artifact_ref,
};
use crate::runtime_journal::{current_cursor, load_run_events};

const MAX_REVISIONS: u16 = 128;
const MAX_ROOT_BYTES: usize = 24 * 1024;
const MAX_REVISION_BYTES: usize = 16 * 1024;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Exact existing runtime context, not an authority token or a new run identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchBudgetContext {
    /// Existing owning session.
    pub session_id: SessionId,
    /// Existing owning task, shared across restart.
    pub task_id: TaskId,
    /// Original runtime run; a new run cannot silently reset a task budget.
    pub run_id: RuntimeRunId,
    /// Governing runtime policy, distinct from the additional research restriction hash.
    pub policy_sha256: String,
}

/// Content-free canonical research failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchJournalError {
    /// Exact context, plan, packet or original restriction binding differs.
    Binding,
    /// No canonical budget exists for this task.
    NotFound,
    /// This task already has an immutable budget root; it cannot be reopened fresh.
    AlreadyExists,
    /// The full plan is unavailable, released, corrupt or outside the owning run.
    Plan,
    /// The bounded accounting journal is full; no further attempts are admitted.
    JournalExhausted,
    /// Existing conservative resource/privacy restrictions refused the operation.
    Budget(ResearchBudgetError),
    /// Canonical storage failed; reopen is required before further effects.
    Storage,
    /// Canonical metadata, history or immutable bindings do not verify.
    Integrity,
}

impl ResearchJournalError {
    pub(crate) const fn poisons_runtime(self) -> bool {
        matches!(self, Self::Storage | Self::Integrity)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetRoot {
    schema_version: u16,
    session_id: SessionId,
    task_id: TaskId,
    origin_run_id: RuntimeRunId,
    policy_sha256: String,
    plan: RuntimeArtifactRef,
    scope_json: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RevisionKind {
    Opened,
    Reserved,
    Restricted,
    Cancelled,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BudgetRevision {
    schema_version: u16,
    revision: u16,
    root_sha256: String,
    previous_sha256: String,
    kind: RevisionKind,
    // Empty for non-reservation observations, never a raw request or query.
    operation_id: String,
    request_sha256: String,
    state_json: String,
}

struct RetainedBudget {
    root: BudgetRoot,
    root_sha256: String,
    scope: ResearchScope,
    budget: ResearchBudget,
    revision: u16,
    head_sha256: String,
}

/// Read-only resume projection of canonical accounting. This can be stale as soon
/// as it is returned; it is not proof of a live plan, admission or a fresh grant.
pub struct ResearchBudgetState {
    /// Original artifact identity; full payload/lifecycle is rechecked before dispatch.
    pub plan: RuntimeArtifactRef,
    /// Original task restrictions, never inferred from a new model response.
    pub scope: ResearchScope,
    /// Retained counters and original clock, not an observation of current wall time.
    pub progress: ResearchBudgetProgress,
    /// Last verified immutable accounting revision.
    pub revision: u16,
    /// Whole canonical head digest for drift-aware presentation.
    pub head_sha256: String,
    /// Remaining append capacity, not a resource or network authorization.
    pub remaining_revisions: u16,
}

impl std::fmt::Debug for ResearchBudgetState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResearchBudgetState")
            .field("revision", &self.revision)
            .field("head_sha256", &self.head_sha256)
            .finish_non_exhaustive()
    }
}

pub(crate) fn state(
    store: &OperationalStore,
    context: &ResearchBudgetContext,
) -> Result<ResearchBudgetState, ResearchJournalError> {
    let retained = load(store, &context.task_id)?;
    if !context_matches(&retained.root, context) {
        return Err(ResearchJournalError::Binding);
    }
    verify_plan_metadata(store, &retained.root)?;
    Ok(ResearchBudgetState {
        plan: retained.root.plan,
        scope: retained.scope,
        progress: retained.budget.progress(),
        revision: retained.revision,
        head_sha256: retained.head_sha256,
        remaining_revisions: MAX_REVISIONS - retained.revision - 1,
    })
}

/// Single-use evidence of a committed reservation, not a capability grant.
/// There is no public constructor, clone or deserializer. A native driver must
/// take ownership once and independently match its consumed effect authorization.
pub struct DurableResearchReservation {
    task_id: TaskId,
    operation_id: String,
    request_sha256: String,
    reservation_sha256: String,
}

impl std::fmt::Debug for DurableResearchReservation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableResearchReservation")
            .field("reservation_sha256", &self.reservation_sha256)
            .finish_non_exhaustive()
    }
}

impl DurableResearchReservation {
    /// Checks the exact packet content, not permission or successful execution.
    #[must_use]
    pub fn matches_packet(&self, packet: &PublicGetWorkerPacket) -> bool {
        self.task_id.as_str() == packet.task_id()
            && self.operation_id == packet.request().operation_id
            && self.request_sha256 == packet.sha256()
    }

    /// Immutable canonical accounting revision identity, never a retrieval receipt.
    #[must_use]
    pub fn reservation_sha256(&self) -> &str {
        &self.reservation_sha256
    }
}

fn context_matches(root: &BudgetRoot, context: &ResearchBudgetContext) -> bool {
    root.task_id == context.task_id
        && root.session_id == context.session_id
        && root.origin_run_id == context.run_id
        && root.policy_sha256 == context.policy_sha256
}

fn verify_plan_metadata(
    store: &OperationalStore,
    root: &BudgetRoot,
) -> Result<(), ResearchJournalError> {
    let manifest = load_artifact_manifest(store, &root.plan.artifact_id)
        .map_err(|_| ResearchJournalError::Plan)?;
    verify_runtime_artifact_ref(&root.plan, &manifest).map_err(|_| ResearchJournalError::Plan)?;
    if manifest.kind != RuntimeArtifactKind::Report
        || manifest.media_type != "application/json"
        || manifest.byte_size > 64 * 1024
        || manifest.session_id != root.session_id
        || manifest.task_id != root.task_id
        || manifest.producer_run_id != root.origin_run_id
        || manifest.policy_sha256 != root.policy_sha256
        || manifest.receipt_id.is_some()
        || manifest.producer_operation_id.is_some()
    {
        return Err(ResearchJournalError::Plan);
    }
    Ok(())
}

fn verify_live_plan<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    root: &BudgetRoot,
    now_epoch_ms: u64,
) -> Result<(), ResearchJournalError> {
    let terminal: bool = store.connection.query_row(
        "SELECT terminal FROM runtime_runs WHERE run_id = ?1 AND session_id = ?2 AND task_id = ?3",
        params![root.origin_run_id.as_str(), root.session_id.as_str(), root.task_id.as_str()],
        |row| row.get(0),
    ).map_err(|_| ResearchJournalError::Plan)?;
    if terminal {
        return Err(ResearchJournalError::Plan);
    }
    verify_retained_plan(store, payloads, root, now_epoch_ms)
}

// Historical consumption is read-only. A completed run may still have retained
// source bytes, but cannot use this path to obtain a fresh dispatch proof.
fn verify_retained_plan<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    root: &BudgetRoot,
    now_epoch_ms: u64,
) -> Result<(), ResearchJournalError> {
    verify_plan_metadata(store, root)?;
    current_cursor(store, &root.origin_run_id)
        .map_err(|_| ResearchJournalError::Integrity)?
        .ok_or(ResearchJournalError::Plan)?;
    let events =
        load_run_events(store, &root.origin_run_id).map_err(|_| ResearchJournalError::Integrity)?;
    if !events.iter().any(|event| matches!(&event.kind,
        RuntimeEventKind::ArtifactCreated { artifact_id, manifest_sha256 }
            if artifact_id == &root.plan.artifact_id && manifest_sha256 == &root.plan.manifest_sha256)
        && event.payload_reference.as_ref().is_some_and(|payload|
            payload.artifact_id == root.plan.artifact_id && payload.sha256 == root.plan.payload_sha256
                && payload.byte_size == root.plan.byte_size && payload.media_type == root.plan.media_type))
    {
        return Err(ResearchJournalError::Plan);
    }
    let bytes = read_runtime_artifact(
        store,
        payloads,
        &RuntimeArtifactReadRequest {
            session_id: root.session_id.clone(),
            task_id: root.task_id.clone(),
            policy_sha256: root.policy_sha256.clone(),
            reference: root.plan.clone(),
            now_epoch_ms,
            maximum_bytes: 64 * 1024,
        },
    )
    .map_err(|_| ResearchJournalError::Plan)?;
    let plan = PreparedResearchPlan::decode(&bytes).map_err(|_| ResearchJournalError::Plan)?;
    if plan.plan_sha256() != root.plan.payload_sha256
        || plan
            .scope()
            .snapshot()
            .map_err(|_| ResearchJournalError::Plan)?
            != root.scope_json.as_bytes()
    {
        return Err(ResearchJournalError::Binding);
    }
    Ok(())
}

/// Verifies historical accounting without updating clocks, refunding budget or
/// yielding dispatch material. The caller separately verifies actual completion.
pub(crate) fn verify_retained_reservation<S: RuntimeArtifactPayloadStore>(
    store: &OperationalStore,
    payloads: &S,
    context: &ResearchBudgetContext,
    packet: &PublicGetWorkerPacket,
    reservation_sha256: &str,
    now_epoch_ms: u64,
) -> Result<(), ResearchJournalError> {
    // Verify the entire bounded chain, not just a caller-named revision or head.
    let retained = load(store, &context.task_id)?;
    if !context_matches(&retained.root, context)
        || context.task_id.as_str() != packet.task_id()
        || !valid_digest(reservation_sha256)
        || now_epoch_ms < retained.budget.progress().last_epoch_ms
    {
        return Err(ResearchJournalError::Binding);
    }
    let bytes = store.connection.query_row(
        "SELECT record_json FROM research_budget_revisions WHERE task_id = ?1 AND record_sha256 = ?2",
        params![context.task_id.as_str(), reservation_sha256],
        |row| row.get::<_, Vec<u8>>(0),
    ).optional().map_err(|_| ResearchJournalError::Storage)?
        .ok_or(ResearchJournalError::Binding)?;
    let revision: BudgetRevision = decode(&bytes, MAX_REVISION_BYTES)?;
    if revision.kind != RevisionKind::Reserved
        || revision.operation_id != packet.request().operation_id
        || revision.request_sha256 != packet.sha256()
        || digest(&bytes) != reservation_sha256
    {
        return Err(ResearchJournalError::Binding);
    }
    let prepared = PreparedPublicGet::prepare(
        &retained.scope,
        packet.request().clone(),
        retained.budget.started_epoch_ms(),
        packet.prepared_at_epoch_ms(),
    )
    .map_err(|_| ResearchJournalError::Binding)?;
    if prepared.packet().bytes() != packet.bytes() {
        return Err(ResearchJournalError::Binding);
    }
    verify_retained_plan(store, payloads, &retained.root, now_epoch_ms)
}

pub(crate) fn open_budget<S: RuntimeArtifactPayloadStore>(
    store: &mut OperationalStore,
    payloads: &S,
    context: &ResearchBudgetContext,
    plan: &RuntimeArtifactRef,
    scope: &ResearchScope,
    now_epoch_ms: u64,
) -> Result<(), ResearchJournalError> {
    if scope.task_id() != context.task_id.as_str() || !valid_digest(&context.policy_sha256) {
        return Err(ResearchJournalError::Binding);
    }
    let exists: bool = store
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM research_budget_roots WHERE task_id = ?1)",
            [context.task_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| ResearchJournalError::Storage)?;
    if exists {
        return Err(ResearchJournalError::AlreadyExists);
    }
    let root = BudgetRoot {
        schema_version: 1,
        session_id: context.session_id.clone(),
        task_id: context.task_id.clone(),
        origin_run_id: context.run_id.clone(),
        policy_sha256: context.policy_sha256.clone(),
        plan: plan.clone(),
        scope_json: String::from_utf8(scope.snapshot().map_err(ResearchJournalError::Budget)?)
            .map_err(|_| ResearchJournalError::Integrity)?,
    };
    verify_live_plan(store, payloads, &root, now_epoch_ms)?;
    let budget = ResearchBudget::new(scope, now_epoch_ms).map_err(ResearchJournalError::Budget)?;
    let root_bytes = encode(&root, MAX_ROOT_BYTES)?;
    let root_sha256 = digest(&root_bytes);
    let revision = BudgetRevision {
        schema_version: 1,
        revision: 0,
        root_sha256: root_sha256.clone(),
        previous_sha256: ZERO_SHA256.into(),
        kind: RevisionKind::Opened,
        operation_id: String::new(),
        request_sha256: String::new(),
        state_json: String::from_utf8(encode(&budget, 8192)?)
            .map_err(|_| ResearchJournalError::Integrity)?,
    };
    let bytes = encode(&revision, MAX_REVISION_BYTES)?;
    let hash = digest(&bytes);
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| ResearchJournalError::Storage)?;
    transaction.execute(
        "INSERT INTO research_budget_roots(task_id, session_id, origin_run_id, plan_artifact_id,
            plan_manifest_sha256, root_sha256, record_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![context.task_id.as_str(), context.session_id.as_str(), context.run_id.as_str(),
            plan.artifact_id.as_str(), &plan.manifest_sha256, &root_sha256, root_bytes],
    ).map_err(|_| ResearchJournalError::Storage)?;
    transaction.execute(
        "INSERT INTO research_budget_revisions(task_id, revision, previous_sha256, record_sha256, record_json)
         VALUES (?1, 0, ?2, ?3, ?4)", params![context.task_id.as_str(), ZERO_SHA256, &hash, bytes],
    ).map_err(|_| ResearchJournalError::Storage)?;
    transaction.execute("INSERT INTO research_budget_heads(task_id, revision, record_sha256) VALUES (?1, 0, ?2)",
        params![context.task_id.as_str(), &hash]).map_err(|_| ResearchJournalError::Storage)?;
    transaction
        .commit()
        .map_err(|_| ResearchJournalError::Storage)?;
    load(store, &context.task_id).map(|_| ())
}

/// Derives query versus visit from the prepared target and the plan's exact
/// disclosed search endpoint (Decision 0106), never from a caller label.
pub(crate) fn reserve<S: RuntimeArtifactPayloadStore>(
    store: &mut OperationalStore,
    payloads: &S,
    context: &ResearchBudgetContext,
    prepared: &PreparedPublicGet,
    now_epoch_ms: u64,
) -> Result<DurableResearchReservation, ResearchJournalError> {
    let mut retained = load(store, &context.task_id)?;
    let packet = prepared.packet();
    if !context_matches(&retained.root, context) || packet.task_id() != context.task_id.as_str() {
        return Err(ResearchJournalError::Binding);
    }
    if retained.revision >= MAX_REVISIONS - 1 {
        return Err(ResearchJournalError::JournalExhausted);
    }
    let before = encode(&retained.budget, 8192)?;
    let result = match retained.scope.classify(&packet.request().target) {
        Ok(operation) => retained.budget.reserve(
            &retained.scope,
            &packet.request().operation_id,
            operation,
            packet.request().maximum_response_bytes,
            now_epoch_ms,
        ),
        // A refused shape spends nothing, but cancellation, offline mode and an
        // observed expiry or rollback keep their priority and are still retained.
        Err(error) => retained
            .budget
            .observe_clock(&retained.scope, now_epoch_ms)
            .and(Err(error)),
    };
    if let Err(error) = result {
        if encode(&retained.budget, 8192)? != before {
            append(store, &retained, RevisionKind::Restricted, "", "")?;
        }
        return Err(ResearchJournalError::Budget(error));
    }
    // Successful accounting is spent even when subsequent plan/packet checks
    // fail. Persist BEFORE any proof can escape, never refund an uncertain attempt.
    let hash = append(
        store,
        &retained,
        RevisionKind::Reserved,
        &packet.request().operation_id,
        packet.sha256(),
    )?;
    verify_live_plan(store, payloads, &retained.root, now_epoch_ms)?;
    let independently_prepared = PreparedPublicGet::prepare(
        &retained.scope,
        packet.request().clone(),
        retained.budget.started_epoch_ms(),
        packet.prepared_at_epoch_ms(),
    )
    .map_err(|_| ResearchJournalError::Binding)?;
    if independently_prepared.packet().sha256() != packet.sha256()
        || now_epoch_ms < packet.prepared_at_epoch_ms()
        || now_epoch_ms >= packet.deadline_epoch_ms()
    {
        return Err(ResearchJournalError::Binding);
    }
    Ok(DurableResearchReservation {
        task_id: context.task_id.clone(),
        operation_id: packet.request().operation_id.clone(),
        request_sha256: packet.sha256().to_owned(),
        reservation_sha256: hash,
    })
}

pub(crate) fn cancel(
    store: &mut OperationalStore,
    context: &ResearchBudgetContext,
) -> Result<(), ResearchJournalError> {
    let mut retained = load(store, &context.task_id)?;
    if !context_matches(&retained.root, context) {
        return Err(ResearchJournalError::Binding);
    }
    if retained.budget.is_cancelled() {
        return Ok(());
    }
    retained.budget.cancel();
    append(store, &retained, RevisionKind::Cancelled, "", "").map(|_| ())
}

pub(crate) fn verify_all(store: &OperationalStore) -> Result<(), ResearchJournalError> {
    let mut statement = store
        .connection
        .prepare("SELECT task_id FROM research_budget_roots ORDER BY task_id")
        .map_err(|_| ResearchJournalError::Storage)?;
    let tasks = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|_| ResearchJournalError::Storage)?;
    for task in tasks {
        let retained = load(
            store,
            &TaskId::from_raw(task.map_err(|_| ResearchJournalError::Storage)?),
        )?;
        verify_plan_metadata(store, &retained.root).map_err(|_| ResearchJournalError::Integrity)?;
    }
    Ok(())
}

pub(crate) fn consume_fresh_reservation<S: RuntimeArtifactPayloadStore>(
    store: &mut OperationalStore,
    payloads: &S,
    context: &ResearchBudgetContext,
    prepared: &PreparedPublicGet,
    reservation: DurableResearchReservation,
    now_epoch_ms: u64,
) -> Result<crate::research_dispatch::FreshResearchDispatch, ResearchJournalError> {
    let mut retained = load(store, &context.task_id)?;
    let packet = prepared.packet();
    if !context_matches(&retained.root, context)
        || !reservation.matches_packet(packet)
        || retained.head_sha256 != reservation.reservation_sha256
    {
        return Err(ResearchJournalError::Binding);
    }
    if retained.revision >= MAX_REVISIONS - 1 {
        // Even an earlier valid reservation cannot execute after a full journal
        // prevented recording cancellation. Never call that cancellation persisted.
        return Err(ResearchJournalError::JournalExhausted);
    }
    let before = encode(&retained.budget, 8192)?;
    let clock = retained.budget.observe_clock(&retained.scope, now_epoch_ms);
    let clock_changed = before != encode(&retained.budget, 8192)?;
    if clock_changed {
        // Clock rollback/expiry is retained even when refusing; no second operation
        // is spent here, no budget is refunded, no original deadline is reset.
        append(store, &retained, RevisionKind::Restricted, "", "")?;
    }
    clock.map_err(ResearchJournalError::Budget)?;
    if clock_changed && retained.revision + 1 >= MAX_REVISIONS - 1 {
        // Preserve at least one append for cancellation after successful preflight.
        // The observation is committed but no dispatch proof escapes at capacity.
        return Err(ResearchJournalError::JournalExhausted);
    }
    verify_live_plan(store, payloads, &retained.root, now_epoch_ms)?;
    let reconstructed = PreparedPublicGet::prepare(
        &retained.scope,
        packet.request().clone(),
        retained.budget.started_epoch_ms(),
        packet.prepared_at_epoch_ms(),
    )
    .map_err(|_| ResearchJournalError::Binding)?;
    if reconstructed.packet().sha256() != packet.sha256()
        || now_epoch_ms < packet.prepared_at_epoch_ms()
        || now_epoch_ms >= packet.deadline_epoch_ms()
    {
        return Err(ResearchJournalError::Binding);
    }
    Ok(crate::research_dispatch::FreshResearchDispatch {
        task_id: context.task_id.as_str().to_owned(),
        operation_id: packet.request().operation_id.clone(),
        request_sha256: packet.sha256().to_owned(),
        reservation_sha256: reservation.reservation_sha256,
        checked_at_epoch_ms: now_epoch_ms,
    })
}

pub(crate) fn verify_requested_operation(
    store: &OperationalStore,
    context: &ResearchBudgetContext,
    started: &RuntimeEvent,
    call: &agentmage_kernel_contracts::ToolCall,
) -> Result<(), ResearchJournalError> {
    // load_run_events decodes rows but does not itself verify sequence hashes.
    // Check the complete canonical chain BEFORE interpreting requested mappings.
    current_cursor(store, &context.run_id)
        .map_err(|_| ResearchJournalError::Integrity)?
        .ok_or(ResearchJournalError::Binding)?;
    let events =
        load_run_events(store, &context.run_id).map_err(|_| ResearchJournalError::Integrity)?;
    let operation = started
        .operation_id
        .as_ref()
        .ok_or(ResearchJournalError::Binding)?;
    let mut matched = false;
    for event in &events {
        // Run-level cancellation has no tool operation identity. Check it before
        // filtering by operation: the separate budget cancellation append may not
        // have happened yet, but the already durable request must prevent launch.
        if matches!(
            event.kind,
            RuntimeEventKind::CancellationRequested { .. }
                | RuntimeEventKind::CancellationObserved { .. }
        ) {
            return Err(ResearchJournalError::Budget(ResearchBudgetError::Cancelled));
        }
        if event.operation_id.as_ref() != Some(operation) {
            continue;
        }
        match &event.kind {
            RuntimeEventKind::ToolRequested {
                tool_call_id,
                arguments_sha256,
            } => {
                if matched
                    || tool_call_id != &call.tool_call_id
                    || arguments_sha256 != &call.arguments.sha256
                    || event.turn_id != started.turn_id
                    || event.correlation_id != call.correlation_id
                {
                    return Err(ResearchJournalError::Binding);
                }
                matched = true;
            }
            RuntimeEventKind::ToolStarted { .. }
            | RuntimeEventKind::ToolCompleted { .. }
            | RuntimeEventKind::ToolFailed { .. }
            | RuntimeEventKind::ToolRejected { .. } => {
                // A spent runtime operation cannot be repurposed even if someone
                // retained another accounting proof or supplied a fresh transaction.
                return Err(ResearchJournalError::Binding);
            }
            _ => {}
        }
    }
    if !matched {
        return Err(ResearchJournalError::Binding);
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value != ZERO_SHA256
}

fn encode<T: Serialize>(value: &T, maximum: usize) -> Result<Vec<u8>, ResearchJournalError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ResearchJournalError::Integrity)?;
    if bytes.is_empty() || bytes.len() > maximum {
        return Err(ResearchJournalError::Integrity);
    }
    Ok(bytes)
}

fn decode<T: for<'de> Deserialize<'de> + Serialize>(
    bytes: &[u8],
    maximum: usize,
) -> Result<T, ResearchJournalError> {
    if bytes.is_empty() || bytes.len() > maximum {
        return Err(ResearchJournalError::Integrity);
    }
    let value = serde_json::from_slice(bytes).map_err(|_| ResearchJournalError::Integrity)?;
    if encode(&value, maximum)? != bytes {
        return Err(ResearchJournalError::Integrity);
    }
    Ok(value)
}

fn load(
    store: &OperationalStore,
    task_id: &TaskId,
) -> Result<RetainedBudget, ResearchJournalError> {
    let root_row = store
        .connection
        .query_row(
            "SELECT session_id, origin_run_id, plan_artifact_id, plan_manifest_sha256,
                root_sha256, record_json FROM research_budget_roots WHERE task_id = ?1",
            [task_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Vec<u8>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| ResearchJournalError::Storage)?
        .ok_or(ResearchJournalError::NotFound)?;
    let root: BudgetRoot = decode(&root_row.5, MAX_ROOT_BYTES)?;
    if root.schema_version != 1
        || root.task_id != *task_id
        || root.session_id.as_str() != root_row.0
        || root.origin_run_id.as_str() != root_row.1
        || root.plan.artifact_id.as_str() != root_row.2
        || root.plan.manifest_sha256 != root_row.3
        || digest(&root_row.5) != root_row.4
        || !valid_digest(&root.policy_sha256)
    {
        return Err(ResearchJournalError::Integrity);
    }
    let scope = ResearchScope::restore(root.scope_json.as_bytes())
        .map_err(|_| ResearchJournalError::Integrity)?;
    if scope.task_id() != task_id.as_str() {
        return Err(ResearchJournalError::Integrity);
    }
    let head = store
        .connection
        .query_row(
            "SELECT revision, record_sha256 FROM research_budget_heads WHERE task_id = ?1",
            [task_id.as_str()],
            |row| Ok((row.get::<_, u16>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|_| ResearchJournalError::Integrity)?;
    let rows = store
        .connection
        .prepare(
            "SELECT revision, previous_sha256, record_sha256, record_json
         FROM research_budget_revisions WHERE task_id = ?1 ORDER BY revision LIMIT 129",
        )
        .and_then(|mut statement| {
            statement
                .query_map([task_id.as_str()], |row| {
                    Ok((
                        row.get::<_, u16>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| ResearchJournalError::Storage)?;
    if rows.is_empty() || rows.len() > MAX_REVISIONS as usize {
        return Err(ResearchJournalError::Integrity);
    }
    let mut previous: Option<ResearchBudget> = None;
    let mut previous_sha256 = ZERO_SHA256.to_owned();
    for (index, row) in rows.iter().enumerate() {
        let revision: BudgetRevision = decode(&row.3, MAX_REVISION_BYTES)?;
        if row.0 != index as u16
            || row.1 != previous_sha256
            || row.2 != digest(&row.3)
            || revision.schema_version != 1
            || revision.revision != row.0
            || revision.root_sha256 != root_row.4
            || revision.previous_sha256 != previous_sha256
        {
            return Err(ResearchJournalError::Integrity);
        }
        let budget = ResearchBudget::restore(&scope, revision.state_json.as_bytes())
            .map_err(|_| ResearchJournalError::Integrity)?;
        if let Some(before) = previous.as_ref() {
            before
                .check_successor(&budget)
                .map_err(|_| ResearchJournalError::Integrity)?;
            match revision.kind {
                RevisionKind::Opened => return Err(ResearchJournalError::Integrity),
                RevisionKind::Reserved => {
                    if budget.reservation_count() != before.reservation_count() + 1
                        || before.contains_operation(&revision.operation_id)
                        || !budget.contains_operation(&revision.operation_id)
                        || !valid_digest(&revision.request_sha256)
                    {
                        return Err(ResearchJournalError::Integrity);
                    }
                }
                RevisionKind::Restricted | RevisionKind::Cancelled => {
                    if budget.reservation_count() != before.reservation_count()
                        || !revision.operation_id.is_empty()
                        || !revision.request_sha256.is_empty()
                        || (revision.kind == RevisionKind::Cancelled && !budget.is_cancelled())
                    {
                        return Err(ResearchJournalError::Integrity);
                    }
                }
            }
        } else {
            let initial = ResearchBudget::new(&scope, budget.started_epoch_ms())
                .map_err(|_| ResearchJournalError::Integrity)?;
            if revision.kind != RevisionKind::Opened
                || !revision.operation_id.is_empty()
                || !revision.request_sha256.is_empty()
                || encode(&initial, 8192)? != revision.state_json.as_bytes()
            {
                return Err(ResearchJournalError::Integrity);
            }
        }
        previous = Some(budget);
        previous_sha256 = row.2.clone();
    }
    if head.0 != rows.len() as u16 - 1 || head.1 != previous_sha256 {
        return Err(ResearchJournalError::Integrity);
    }
    Ok(RetainedBudget {
        root,
        root_sha256: root_row.4,
        scope,
        budget: previous.ok_or(ResearchJournalError::Integrity)?,
        revision: head.0,
        head_sha256: head.1,
    })
}

fn append(
    store: &mut OperationalStore,
    retained: &RetainedBudget,
    kind: RevisionKind,
    operation_id: &str,
    request_sha256: &str,
) -> Result<String, ResearchJournalError> {
    let revision = retained
        .revision
        .checked_add(1)
        .filter(|value| *value < MAX_REVISIONS)
        .ok_or(ResearchJournalError::JournalExhausted)?;
    let state_json = String::from_utf8(encode(&retained.budget, 8192)?)
        .map_err(|_| ResearchJournalError::Integrity)?;
    let record = BudgetRevision {
        schema_version: 1,
        revision,
        root_sha256: retained.root_sha256.clone(),
        previous_sha256: retained.head_sha256.clone(),
        kind,
        operation_id: operation_id.to_owned(),
        request_sha256: request_sha256.to_owned(),
        state_json,
    };
    let bytes = encode(&record, MAX_REVISION_BYTES)?;
    let hash = digest(&bytes);
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| ResearchJournalError::Storage)?;
    transaction.execute(
        "INSERT INTO research_budget_revisions(task_id, revision, previous_sha256, record_sha256, record_json)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![retained.root.task_id.as_str(), revision, &retained.head_sha256, &hash, bytes],
    ).map_err(|_| ResearchJournalError::Storage)?;
    let changed = transaction
        .execute(
            "UPDATE research_budget_heads SET revision = ?1, record_sha256 = ?2
         WHERE task_id = ?3 AND revision = ?4 AND record_sha256 = ?5",
            params![
                revision,
                &hash,
                retained.root.task_id.as_str(),
                retained.revision,
                &retained.head_sha256
            ],
        )
        .map_err(|_| ResearchJournalError::Storage)?;
    if changed != 1 {
        return Err(ResearchJournalError::Integrity);
    }
    transaction
        .commit()
        .map_err(|_| ResearchJournalError::Storage)?;
    let verified = load(store, &retained.root.task_id)?;
    if verified.head_sha256 != hash {
        return Err(ResearchJournalError::Integrity);
    }
    Ok(hash)
}
