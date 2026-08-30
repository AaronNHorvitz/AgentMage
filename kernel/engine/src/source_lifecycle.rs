//! Atomic lifecycle transactions for normalized source materializations.

use std::collections::BTreeMap;

use agentmage_kernel_contracts::{
    RuntimeArtifactIntegrityState, RuntimeArtifactLifecycleState, RuntimeArtifactManifest,
    RuntimeArtifactRef,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::operational_store::OperationalStore;
use crate::runtime_artifact::{
    RuntimeArtifactPayloadStore, RuntimeArtifactReconciliation, RuntimeArtifactStoreError,
    load_artifact_manifest, reconcile_runtime_artifacts, runtime_artifact_ref,
    transition_artifact_in_transaction, verify_all,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Closed source-lifecycle failure without paths or retained content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLifecycleError {
    /// An identity, reason, timestamp, revision, or relationship was malformed.
    Invalid,
    /// A required source or retention record does not exist.
    NotFound,
    /// Current canonical state differs from the caller's exact expectation.
    Conflict,
    /// An active user or legal hold blocks release, expiry, or deletion.
    Held,
    /// SQLCipher metadata could not be committed atomically.
    Storage,
    /// Canonical source lifecycle state is internally inconsistent.
    Integrity,
    /// The existing runtime artifact authority rejected the coupled transition.
    RuntimeArtifact(RuntimeArtifactStoreError),
}

impl SourceLifecycleError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Invalid => "source.lifecycle.invalid",
            Self::NotFound => "source.lifecycle.not_found",
            Self::Conflict => "source.lifecycle.conflict",
            Self::Held => "source.lifecycle.held",
            Self::Storage => "source.lifecycle.storage",
            Self::Integrity => "source.lifecycle.integrity",
            Self::RuntimeArtifact(error) => error.code(),
        }
    }
}

impl From<RuntimeArtifactStoreError> for SourceLifecycleError {
    fn from(error: RuntimeArtifactStoreError) -> Self {
        Self::RuntimeArtifact(error)
    }
}

/// Closed hold family that may retain a logical source reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceHoldKind {
    /// A user explicitly retained the source.
    User,
    /// A legal or compliance policy retained the source.
    Legal,
}

impl SourceHoldKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Legal => "legal",
        }
    }
}

/// Exact request to replace one current source and invalidate its dependency closure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRefreshRequest {
    /// Stable refresh identity.
    pub refresh_id: String,
    /// Current source being replaced.
    pub replaced_source_artifact_id: String,
    /// Newly captured replacement source.
    pub replacement_source_artifact_id: String,
    /// Expected lifecycle revision of the replaced source.
    pub expected_replaced_revision: u64,
    /// Stable content-free reason.
    pub reason_code: String,
    /// Trusted transition time.
    pub occurred_at_epoch_ms: u64,
}

/// Content-free receipt for one atomic refresh and transitive invalidation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRefreshReceipt {
    /// Stable refresh identity.
    pub refresh_id: String,
    /// Number of sources marked stale, including the replaced source.
    pub invalidated_source_count: u64,
    /// Digest binding the refresh and sorted invalidation closure.
    pub refresh_sha256: String,
}

/// Content-free receipt for a hold projection transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceHoldReceipt {
    /// Retention record governed by the hold.
    pub retention_id: String,
    /// Hold family.
    pub hold_kind: SourceHoldKind,
    /// Whether the hold is active after this transition.
    pub active: bool,
    /// Monotonic hold revision.
    pub revision: u64,
    /// Hash-chain head for the hold family.
    pub event_sha256: String,
}

/// Exact request to release one active source-retention hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceHoldReleaseRequest {
    /// Retention record governed by the hold.
    pub retention_id: String,
    /// Exact governing authority identity.
    pub authority_id: String,
    /// Expected current source-retention revision.
    pub expected_retention_revision: u64,
    /// Hold family being released.
    pub hold_kind: SourceHoldKind,
    /// Expected current hold revision.
    pub expected_hold_revision: u64,
    /// Stable content-free release reason.
    pub reason_code: String,
    /// Trusted transition time.
    pub occurred_at_epoch_ms: u64,
}

/// Content-free receipt for a release, expiry, or deletion transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRetentionReceipt {
    /// Retention record transitioned.
    pub retention_id: String,
    /// Logical source transitioned.
    pub source_artifact_id: String,
    /// New source-retention revision.
    pub revision: u64,
    /// New lifecycle state.
    pub lifecycle_state: &'static str,
    /// Physical content address retained for reconciliation.
    pub payload_sha256: String,
}

/// Atomic expiry result that keeps held due records visible.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceExpiryReport {
    /// Due unheld references released in the transaction.
    pub released: Vec<SourceRetentionReceipt>,
    /// Due references preserved because one or more holds are active.
    pub held_count: u64,
}

#[derive(Serialize)]
struct RetentionTransitionRecord<'a> {
    record_type: &'static str,
    retention_id: &'a str,
    source_artifact_id: &'a str,
    lifecycle_state: &'a str,
    reason_code: &'a str,
    revision: u64,
    occurred_at_epoch_ms: u64,
}

struct ActiveRetention {
    retention_id: String,
    source_artifact_id: String,
    authority_id: String,
    revision: u64,
    physical_artifact_id: String,
    payload_sha256: String,
    byte_size: u64,
}

struct HoldTransition<'a> {
    retention_id: &'a str,
    authority_id: &'a str,
    expected_retention_revision: u64,
    hold_kind: SourceHoldKind,
    activate: bool,
    expected_hold_revision: Option<u64>,
    reason_code: &'a str,
    occurred_at_epoch_ms: u64,
}

struct RetentionProjection<'a> {
    retention_id: &'a str,
    source_artifact_id: &'a str,
    expected_revision: u64,
    next_revision: u64,
    lifecycle_state: &'a str,
    reason_code: &'a str,
    occurred_at_epoch_ms: u64,
}

/// Adds one immutable dependency edge after rejecting cycles and stale endpoints.
pub fn register_source_dependency(
    store: &mut OperationalStore,
    upstream_source_artifact_id: &str,
    dependent_source_artifact_id: &str,
    recorded_at_epoch_ms: u64,
) -> Result<String, SourceLifecycleError> {
    if !valid_identity(upstream_source_artifact_id)
        || !valid_identity(dependent_source_artifact_id)
        || upstream_source_artifact_id == dependent_source_artifact_id
    {
        return Err(SourceLifecycleError::Invalid);
    }
    let dependency_sha256 = digest_fields(&[
        "agentmage.source-dependency.v1",
        upstream_source_artifact_id,
        dependent_source_artifact_id,
        &recorded_at_epoch_ms.to_string(),
    ]);
    let changed = store
        .connection
        .execute(
            "INSERT INTO source_dependencies(
                 upstream_source_artifact_id, dependent_source_artifact_id,
                 dependency_sha256, recorded_at_epoch_ms
             )
             SELECT ?1, ?2, ?3, ?4
             WHERE EXISTS (
                 SELECT 1 FROM source_materialization_states
                 WHERE source_artifact_id = ?1 AND lifecycle_state = 'current'
             ) AND EXISTS (
                 SELECT 1 FROM source_materialization_states
                 WHERE source_artifact_id = ?2 AND lifecycle_state = 'current'
             )",
            params![
                upstream_source_artifact_id,
                dependent_source_artifact_id,
                &dependency_sha256,
                sql_u64(recorded_at_epoch_ms)?,
            ],
        )
        .map_err(|_| SourceLifecycleError::Conflict)?;
    if changed != 1 {
        return Err(SourceLifecycleError::Conflict);
    }
    Ok(dependency_sha256)
}

/// Atomically marks the replaced source and every transitive dependent stale.
pub fn refresh_source(
    store: &mut OperationalStore,
    request: &SourceRefreshRequest,
) -> Result<SourceRefreshReceipt, SourceLifecycleError> {
    if !valid_identity(&request.refresh_id)
        || !valid_identity(&request.replaced_source_artifact_id)
        || !valid_identity(&request.replacement_source_artifact_id)
        || request.replaced_source_artifact_id == request.replacement_source_artifact_id
        || !valid_reason_code(&request.reason_code)
    {
        return Err(SourceLifecycleError::Invalid);
    }
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| SourceLifecycleError::Storage)?;
    let replaced = transaction
        .query_row(
            "SELECT revision, lifecycle_state FROM source_materialization_states
             WHERE source_artifact_id = ?1",
            [&request.replaced_source_artifact_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?
        .ok_or(SourceLifecycleError::NotFound)?;
    if u64::try_from(replaced.0).ok() != Some(request.expected_replaced_revision)
        || replaced.1 != "current"
    {
        return Err(SourceLifecycleError::Conflict);
    }
    let replacement_state: Option<String> = transaction
        .query_row(
            "SELECT lifecycle_state FROM source_materialization_states
             WHERE source_artifact_id = ?1",
            [&request.replacement_source_artifact_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?;
    if replacement_state.as_deref() != Some("current") {
        return Err(SourceLifecycleError::Conflict);
    }
    let invalidated = transaction
        .prepare(
            "WITH RECURSIVE invalidated(source_artifact_id) AS (
                 SELECT ?1
                 UNION
                 SELECT d.dependent_source_artifact_id
                 FROM source_dependencies d
                 JOIN invalidated i
                   ON d.upstream_source_artifact_id = i.source_artifact_id
             )
             SELECT source_artifact_id FROM invalidated ORDER BY source_artifact_id",
        )
        .and_then(|mut statement| {
            statement
                .query_map([&request.replaced_source_artifact_id], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| SourceLifecycleError::Storage)?;
    if invalidated.is_empty()
        || invalidated
            .iter()
            .any(|source| source == &request.replacement_source_artifact_id)
    {
        return Err(SourceLifecycleError::Conflict);
    }
    for source_artifact_id in &invalidated {
        advance_source_state(
            &transaction,
            source_artifact_id,
            "stale",
            &request.reason_code,
            request.occurred_at_epoch_ms,
        )?;
    }
    let invalidated_count =
        u64::try_from(invalidated.len()).map_err(|_| SourceLifecycleError::Invalid)?;
    let mut refresh_fields = vec![
        "agentmage.source-refresh.v1".to_owned(),
        request.refresh_id.clone(),
        request.replaced_source_artifact_id.clone(),
        request.replacement_source_artifact_id.clone(),
        invalidated_count.to_string(),
        request.reason_code.clone(),
        request.occurred_at_epoch_ms.to_string(),
    ];
    refresh_fields.extend(invalidated.iter().cloned());
    let refresh_refs = refresh_fields
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let refresh_sha256 = digest_fields(&refresh_refs);
    transaction
        .execute(
            "INSERT INTO source_refreshes VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &request.refresh_id,
                &request.replaced_source_artifact_id,
                &request.replacement_source_artifact_id,
                sql_u64(invalidated_count)?,
                &request.reason_code,
                sql_u64(request.occurred_at_epoch_ms)?,
                &refresh_sha256,
            ],
        )
        .map_err(|_| SourceLifecycleError::Conflict)?;
    transaction
        .commit()
        .map_err(|_| SourceLifecycleError::Storage)?;
    Ok(SourceRefreshReceipt {
        refresh_id: request.refresh_id.clone(),
        invalidated_source_count: invalidated_count,
        refresh_sha256,
    })
}

/// Registers the trusted numeric expiry companion for one persisted retention.
pub fn register_source_retention_deadline(
    store: &mut OperationalStore,
    retention_id: &str,
    expires_at_epoch_ms: u64,
) -> Result<String, SourceLifecycleError> {
    if !valid_identity(retention_id) || expires_at_epoch_ms == 0 {
        return Err(SourceLifecycleError::Invalid);
    }
    let deadline_sha256 = digest_fields(&[
        "agentmage.source-retention-deadline.v1",
        retention_id,
        &expires_at_epoch_ms.to_string(),
    ]);
    store
        .connection
        .execute(
            "INSERT INTO source_retention_deadlines VALUES (?1, ?2, ?3)",
            params![
                retention_id,
                sql_u64(expires_at_epoch_ms)?,
                &deadline_sha256
            ],
        )
        .map_err(|_| SourceLifecycleError::Conflict)?;
    Ok(deadline_sha256)
}

/// Applies one user or legal hold at an exact active retention revision.
pub fn apply_source_hold(
    store: &mut OperationalStore,
    retention_id: &str,
    authority_id: &str,
    expected_retention_revision: u64,
    hold_kind: SourceHoldKind,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
) -> Result<SourceHoldReceipt, SourceLifecycleError> {
    transition_hold(
        store,
        HoldTransition {
            retention_id,
            authority_id,
            expected_retention_revision,
            hold_kind,
            activate: true,
            expected_hold_revision: None,
            reason_code,
            occurred_at_epoch_ms,
        },
    )
}

/// Releases one exact active hold without releasing the retained source itself.
pub fn release_source_hold(
    store: &mut OperationalStore,
    request: &SourceHoldReleaseRequest,
) -> Result<SourceHoldReceipt, SourceLifecycleError> {
    transition_hold(
        store,
        HoldTransition {
            retention_id: &request.retention_id,
            authority_id: &request.authority_id,
            expected_retention_revision: request.expected_retention_revision,
            hold_kind: request.hold_kind,
            activate: false,
            expected_hold_revision: Some(request.expected_hold_revision),
            reason_code: &request.reason_code,
            occurred_at_epoch_ms: request.occurred_at_epoch_ms,
        },
    )
}

/// Atomically releases one source retention and its exact runtime artifact reference.
pub fn release_source(
    store: &mut OperationalStore,
    retention_id: &str,
    authority_id: &str,
    expected_revision: u64,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
) -> Result<SourceRetentionReceipt, SourceLifecycleError> {
    validate_transition_request(retention_id, authority_id, reason_code)?;
    let retained = load_active_retention(store, retention_id)?;
    let manifest = load_artifact_manifest(
        store,
        &agentmage_kernel_contracts::RuntimeArtifactId::from_raw(
            retained.physical_artifact_id.clone(),
        ),
    )?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| SourceLifecycleError::Storage)?;
    let receipt = release_in_transaction(
        &transaction,
        &retained,
        &manifest,
        authority_id,
        expected_revision,
        reason_code,
        occurred_at_epoch_ms,
    )?;
    transaction
        .commit()
        .map_err(|_| SourceLifecycleError::Storage)?;
    Ok(receipt)
}

/// Atomically releases every due unheld source or retains the entire batch on failure.
pub fn expire_sources(
    store: &mut OperationalStore,
    now_epoch_ms: u64,
) -> Result<SourceExpiryReport, SourceLifecycleError> {
    let due = load_due_retentions(store, now_epoch_ms)?;
    let manifests = due
        .iter()
        .map(|retained| {
            load_artifact_manifest(
                store,
                &agentmage_kernel_contracts::RuntimeArtifactId::from_raw(
                    retained.physical_artifact_id.clone(),
                ),
            )
            .map(|manifest| (retained.retention_id.clone(), manifest))
            .map_err(SourceLifecycleError::from)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| SourceLifecycleError::Storage)?;
    let mut report = SourceExpiryReport::default();
    for retained in due {
        if active_hold_count(&transaction, &retained.retention_id)? != 0 {
            report.held_count = report
                .held_count
                .checked_add(1)
                .ok_or(SourceLifecycleError::Integrity)?;
            continue;
        }
        let manifest = manifests
            .get(&retained.retention_id)
            .ok_or(SourceLifecycleError::Integrity)?;
        report.released.push(release_in_transaction(
            &transaction,
            &retained,
            manifest,
            &retained.authority_id,
            retained.revision,
            "source.retention_expired",
            now_epoch_ms,
        )?);
    }
    transaction
        .commit()
        .map_err(|_| SourceLifecycleError::Storage)?;
    Ok(report)
}

/// Marks one released logical source deleted while preserving any shared live payload.
pub fn delete_source(
    store: &mut OperationalStore,
    retention_id: &str,
    authority_id: &str,
    expected_revision: u64,
    occurred_at_epoch_ms: u64,
) -> Result<SourceRetentionReceipt, SourceLifecycleError> {
    validate_transition_request(retention_id, authority_id, "source.deleted")?;
    let released = store
        .connection
        .query_row(
            "SELECT r.source_artifact_id, r.authority_id, r.revision,
                    p.physical_artifact_id, p.payload_sha256, p.byte_size
             FROM source_retentions r
             JOIN source_released_payloads p USING(retention_id)
             WHERE r.retention_id = ?1 AND r.retention_class = 'released'
               AND r.lifecycle_state = 'released' AND p.deleted_at_epoch_ms IS NULL",
            [retention_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?
        .ok_or(SourceLifecycleError::NotFound)?;
    if released.1 != authority_id || u64::try_from(released.2).ok() != Some(expected_revision) {
        return Err(SourceLifecycleError::Conflict);
    }
    let manifest = load_artifact_manifest(
        store,
        &agentmage_kernel_contracts::RuntimeArtifactId::from_raw(released.3.clone()),
    )?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| SourceLifecycleError::Storage)?;
    if active_hold_count(&transaction, retention_id)? != 0 {
        return Err(SourceLifecycleError::Held);
    }
    transition_artifact_in_transaction(
        &transaction,
        &manifest,
        RuntimeArtifactLifecycleState::Deleted,
        RuntimeArtifactIntegrityState::Deleted,
        "source.deleted",
        occurred_at_epoch_ms,
    )?;
    let next_revision = expected_revision
        .checked_add(1)
        .ok_or(SourceLifecycleError::Integrity)?;
    update_retention_projection(
        &transaction,
        &RetentionProjection {
            retention_id,
            source_artifact_id: &released.0,
            expected_revision,
            next_revision,
            lifecycle_state: "deleted",
            reason_code: "source.deleted",
            occurred_at_epoch_ms,
        },
    )?;
    let changed = transaction
        .execute(
            "UPDATE source_released_payloads SET deleted_at_epoch_ms = ?1
             WHERE retention_id = ?2 AND deleted_at_epoch_ms IS NULL",
            params![sql_u64(occurred_at_epoch_ms)?, retention_id],
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    if changed != 1 {
        return Err(SourceLifecycleError::Conflict);
    }
    advance_source_state(
        &transaction,
        &released.0,
        "deleted",
        "source.deleted",
        occurred_at_epoch_ms,
    )?;
    transaction
        .commit()
        .map_err(|_| SourceLifecycleError::Storage)?;
    Ok(SourceRetentionReceipt {
        retention_id: retention_id.to_owned(),
        source_artifact_id: released.0,
        revision: next_revision,
        lifecycle_state: "deleted",
        payload_sha256: released.4,
    })
}

/// Reconciles successful source deletions through the sole physical payload authority.
pub fn collect_source_garbage<S: RuntimeArtifactPayloadStore>(
    store: &mut OperationalStore,
    payloads: &mut S,
    now_epoch_ms: u64,
) -> Result<RuntimeArtifactReconciliation, SourceLifecycleError> {
    verify_source_lifecycle(store)?;
    let report = reconcile_runtime_artifacts(store, payloads, now_epoch_ms)?;
    verify_source_lifecycle(store)?;
    Ok(report)
}

/// Verifies source lifecycle projections and their canonical runtime-reference coupling.
pub fn verify_source_lifecycle(store: &OperationalStore) -> Result<(), SourceLifecycleError> {
    verify_all(store)?;
    let invalid: i64 = store
        .connection
        .query_row(
            "SELECT
                (SELECT COUNT(*) FROM source_manifests)
                - (SELECT COUNT(*) FROM source_materialization_states)
                + (SELECT COUNT(*)
                   FROM source_retentions r
                   WHERE r.retention_class = 'policy_persisted'
                     AND NOT EXISTS (
                         SELECT 1 FROM runtime_artifact_states s
                         WHERE s.artifact_id = r.physical_artifact_id
                           AND s.lifecycle_state = 'active'
                     ))
                + (SELECT COUNT(*)
                   FROM source_retention_holds h
                   JOIN source_retentions r USING(retention_id)
                   WHERE h.active = 1 AND r.lifecycle_state <> 'active')
                + (SELECT COUNT(*)
                   FROM source_released_payloads p
                   JOIN source_retentions r USING(retention_id)
                   WHERE (p.deleted_at_epoch_ms IS NULL AND r.retention_class <> 'released')
                      OR (p.deleted_at_epoch_ms IS NOT NULL AND r.retention_class <> 'deleted'))",
            [],
            |row| row.get(0),
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    if invalid != 0 {
        return Err(SourceLifecycleError::Integrity);
    }
    Ok(())
}

fn transition_hold(
    store: &mut OperationalStore,
    request: HoldTransition<'_>,
) -> Result<SourceHoldReceipt, SourceLifecycleError> {
    validate_transition_request(
        request.retention_id,
        request.authority_id,
        request.reason_code,
    )?;
    let transaction = store
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| SourceLifecycleError::Storage)?;
    let retention: Option<(String, i64)> = transaction
        .query_row(
            "SELECT authority_id, revision FROM source_retentions
             WHERE retention_id = ?1 AND retention_class = 'policy_persisted'
               AND lifecycle_state = 'active'",
            [request.retention_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?;
    let (retained_authority, retained_revision) =
        retention.ok_or(SourceLifecycleError::NotFound)?;
    if retained_authority != request.authority_id
        || u64::try_from(retained_revision).ok() != Some(request.expected_retention_revision)
    {
        return Err(SourceLifecycleError::Conflict);
    }
    let current = transaction
        .query_row(
            "SELECT active, revision, head_event_sha256 FROM source_retention_holds
             WHERE retention_id = ?1 AND hold_kind = ?2",
            params![request.retention_id, request.hold_kind.as_str()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?;
    let (revision, previous_event_sha256) = match (request.activate, current) {
        (true, None) => (1, ZERO_SHA256.to_owned()),
        (true, Some((0, revision, head))) => (
            u64::try_from(revision)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(SourceLifecycleError::Integrity)?,
            head,
        ),
        (false, Some((1, revision, head)))
            if u64::try_from(revision).ok() == request.expected_hold_revision =>
        {
            (
                u64::try_from(revision)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or(SourceLifecycleError::Integrity)?,
                head,
            )
        }
        _ => return Err(SourceLifecycleError::Conflict),
    };
    let action = if request.activate {
        "applied"
    } else {
        "released"
    };
    let event_sha256 = digest_fields(&[
        "agentmage.source-retention-hold-event.v1",
        request.retention_id,
        request.hold_kind.as_str(),
        &revision.to_string(),
        action,
        request.reason_code,
        &request.occurred_at_epoch_ms.to_string(),
        &previous_event_sha256,
    ]);
    transaction
        .execute(
            "INSERT INTO source_retention_holds VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(retention_id, hold_kind) DO UPDATE SET
                 active = excluded.active,
                 revision = excluded.revision,
                 head_event_sha256 = excluded.head_event_sha256,
                 updated_at_epoch_ms = excluded.updated_at_epoch_ms",
            params![
                request.retention_id,
                request.hold_kind.as_str(),
                i64::from(request.activate),
                sql_u64(revision)?,
                &event_sha256,
                sql_u64(request.occurred_at_epoch_ms)?,
            ],
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    transaction
        .execute(
            "INSERT INTO source_retention_hold_events VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request.retention_id,
                request.hold_kind.as_str(),
                sql_u64(revision)?,
                action,
                request.reason_code,
                sql_u64(request.occurred_at_epoch_ms)?,
                &previous_event_sha256,
                &event_sha256,
            ],
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    transaction
        .commit()
        .map_err(|_| SourceLifecycleError::Storage)?;
    Ok(SourceHoldReceipt {
        retention_id: request.retention_id.to_owned(),
        hold_kind: request.hold_kind,
        active: request.activate,
        revision,
        event_sha256,
    })
}

fn release_in_transaction(
    transaction: &Transaction<'_>,
    retained: &ActiveRetention,
    manifest: &RuntimeArtifactManifest,
    authority_id: &str,
    expected_revision: u64,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
) -> Result<SourceRetentionReceipt, SourceLifecycleError> {
    let current = transaction
        .query_row(
            "SELECT authority_id, revision, physical_artifact_id, payload_sha256, byte_size
             FROM source_retentions
             WHERE retention_id = ?1 AND retention_class = 'policy_persisted'
               AND lifecycle_state = 'active'",
            [&retained.retention_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?
        .ok_or(SourceLifecycleError::NotFound)?;
    if current.0 != authority_id
        || u64::try_from(current.1).ok() != Some(expected_revision)
        || current.2 != retained.physical_artifact_id
        || current.3 != retained.payload_sha256
        || u64::try_from(current.4).ok() != Some(retained.byte_size)
    {
        return Err(SourceLifecycleError::Conflict);
    }
    if active_hold_count(transaction, &retained.retention_id)? != 0 {
        return Err(SourceLifecycleError::Held);
    }
    let reference: RuntimeArtifactRef =
        runtime_artifact_ref(manifest).map_err(RuntimeArtifactStoreError::Contract)?;
    let checkpoint_references: i64 = transaction
        .query_row(
            "SELECT COUNT(*)
             FROM runtime_resume_artifacts r
             JOIN store_metadata m ON m.session_checkpoint_sha256 = r.checkpoint_sha256
             WHERE m.singleton = 1 AND r.artifact_id = ?1 AND r.manifest_sha256 = ?2",
            params![reference.artifact_id.as_str(), &reference.manifest_sha256],
            |row| row.get(0),
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    if checkpoint_references != 0 {
        return Err(SourceLifecycleError::Conflict);
    }
    transition_artifact_in_transaction(
        transaction,
        manifest,
        RuntimeArtifactLifecycleState::Released,
        RuntimeArtifactIntegrityState::Verified,
        reason_code,
        occurred_at_epoch_ms,
    )?;
    let release_sha256 = digest_fields(&[
        "agentmage.source-released-payload.v1",
        &retained.retention_id,
        &retained.source_artifact_id,
        &retained.physical_artifact_id,
        &retained.payload_sha256,
        &retained.byte_size.to_string(),
        &occurred_at_epoch_ms.to_string(),
    ]);
    transaction
        .execute(
            "INSERT INTO source_released_payloads VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
            params![
                &retained.retention_id,
                &retained.source_artifact_id,
                &retained.physical_artifact_id,
                &retained.payload_sha256,
                sql_u64(retained.byte_size)?,
                sql_u64(occurred_at_epoch_ms)?,
                &release_sha256,
            ],
        )
        .map_err(|_| SourceLifecycleError::Conflict)?;
    let next_revision = expected_revision
        .checked_add(1)
        .ok_or(SourceLifecycleError::Integrity)?;
    update_retention_projection(
        transaction,
        &RetentionProjection {
            retention_id: &retained.retention_id,
            source_artifact_id: &retained.source_artifact_id,
            expected_revision,
            next_revision,
            lifecycle_state: "released",
            reason_code,
            occurred_at_epoch_ms,
        },
    )?;
    advance_source_state(
        transaction,
        &retained.source_artifact_id,
        "released",
        reason_code,
        occurred_at_epoch_ms,
    )?;
    Ok(SourceRetentionReceipt {
        retention_id: retained.retention_id.clone(),
        source_artifact_id: retained.source_artifact_id.clone(),
        revision: next_revision,
        lifecycle_state: "released",
        payload_sha256: retained.payload_sha256.clone(),
    })
}

fn update_retention_projection(
    transaction: &Transaction<'_>,
    projection: &RetentionProjection<'_>,
) -> Result<(), SourceLifecycleError> {
    let record = RetentionTransitionRecord {
        record_type: "agentmage.source-retention-transition.v1",
        retention_id: projection.retention_id,
        source_artifact_id: projection.source_artifact_id,
        lifecycle_state: projection.lifecycle_state,
        reason_code: projection.reason_code,
        revision: projection.next_revision,
        occurred_at_epoch_ms: projection.occurred_at_epoch_ms,
    };
    let record_json = serde_json::to_vec(&record).map_err(|_| SourceLifecycleError::Invalid)?;
    let source_retention_sha256 = sha256(&record_json);
    let changed = transaction
        .execute(
            "UPDATE source_retentions
             SET retention_class = ?1, retention_policy_id = NULL,
                 retention_expires_at = NULL, physical_artifact_id = NULL,
                 payload_sha256 = NULL, byte_size = NULL,
                 encryption_state = 'not_persisted', lifecycle_state = ?1,
                 reason_code = ?2, revision = ?3, recorded_at = ?4,
                 source_retention_sha256 = ?5, record_json = ?6
             WHERE retention_id = ?7 AND revision = ?8",
            params![
                projection.lifecycle_state,
                projection.reason_code,
                sql_u64(projection.next_revision)?,
                projection.occurred_at_epoch_ms.to_string(),
                &source_retention_sha256,
                &record_json,
                projection.retention_id,
                sql_u64(projection.expected_revision)?,
            ],
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    if changed != 1 {
        return Err(SourceLifecycleError::Conflict);
    }
    let previous_event_sha256 = transaction
        .query_row(
            "SELECT event_sha256 FROM source_lifecycle_events
             WHERE retention_id = ?1 AND revision = ?2",
            params![
                projection.retention_id,
                sql_u64(projection.expected_revision)?
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?
        .unwrap_or_else(|| ZERO_SHA256.to_owned());
    let event_sha256 = digest_fields(&[
        "agentmage.source-retention-event.v1",
        projection.retention_id,
        &projection.next_revision.to_string(),
        projection.lifecycle_state,
        projection.reason_code,
        &projection.occurred_at_epoch_ms.to_string(),
        &previous_event_sha256,
        &source_retention_sha256,
    ]);
    transaction
        .execute(
            "INSERT INTO source_lifecycle_events VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                projection.retention_id,
                sql_u64(projection.next_revision)?,
                projection.lifecycle_state,
                projection.reason_code,
                projection.occurred_at_epoch_ms.to_string(),
                &previous_event_sha256,
                &source_retention_sha256,
                &event_sha256,
                &record_json,
            ],
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    Ok(())
}

fn advance_source_state(
    transaction: &Transaction<'_>,
    source_artifact_id: &str,
    lifecycle_state: &str,
    reason_code: &str,
    occurred_at_epoch_ms: u64,
) -> Result<(), SourceLifecycleError> {
    let current = transaction
        .query_row(
            "SELECT lifecycle_state, revision, head_event_sha256, updated_at_epoch_ms
             FROM source_materialization_states WHERE source_artifact_id = ?1",
            [source_artifact_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?
        .ok_or(SourceLifecycleError::NotFound)?;
    let legal = matches!(
        (current.0.as_str(), lifecycle_state),
        ("current", "stale")
            | ("current", "released")
            | ("stale", "released")
            | ("released", "deleted")
    );
    if current.0 == lifecycle_state && lifecycle_state == "stale" {
        return Ok(());
    }
    if !legal
        || i64::try_from(occurred_at_epoch_ms)
            .ok()
            .is_none_or(|time| time < current.3)
    {
        return Err(SourceLifecycleError::Conflict);
    }
    let revision = u64::try_from(current.1)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(SourceLifecycleError::Integrity)?;
    let event_sha256 = digest_fields(&[
        "agentmage.source-materialization-event.v1",
        source_artifact_id,
        &revision.to_string(),
        lifecycle_state,
        reason_code,
        &occurred_at_epoch_ms.to_string(),
        &current.2,
    ]);
    let changed = transaction
        .execute(
            "UPDATE source_materialization_states
             SET lifecycle_state = ?1, revision = ?2, head_event_sha256 = ?3,
                 updated_at_epoch_ms = ?4
             WHERE source_artifact_id = ?5 AND revision = ?6",
            params![
                lifecycle_state,
                sql_u64(revision)?,
                &event_sha256,
                sql_u64(occurred_at_epoch_ms)?,
                source_artifact_id,
                current.1,
            ],
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    if changed != 1 {
        return Err(SourceLifecycleError::Conflict);
    }
    transaction
        .execute(
            "INSERT INTO source_materialization_events VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source_artifact_id,
                sql_u64(revision)?,
                lifecycle_state,
                reason_code,
                sql_u64(occurred_at_epoch_ms)?,
                &current.2,
                &event_sha256,
            ],
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    Ok(())
}

fn load_active_retention(
    store: &OperationalStore,
    retention_id: &str,
) -> Result<ActiveRetention, SourceLifecycleError> {
    store
        .connection
        .query_row(
            "SELECT retention_id, source_artifact_id, authority_id, revision,
                    physical_artifact_id, payload_sha256, byte_size
             FROM source_retentions
             WHERE retention_id = ?1 AND retention_class = 'policy_persisted'
               AND lifecycle_state = 'active'",
            [retention_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|_| SourceLifecycleError::Storage)?
        .ok_or(SourceLifecycleError::NotFound)
        .and_then(|row| {
            Ok(ActiveRetention {
                retention_id: row.0,
                source_artifact_id: row.1,
                authority_id: row.2,
                revision: u64::try_from(row.3).map_err(|_| SourceLifecycleError::Integrity)?,
                physical_artifact_id: row.4,
                payload_sha256: row.5,
                byte_size: u64::try_from(row.6).map_err(|_| SourceLifecycleError::Integrity)?,
            })
        })
}

fn load_due_retentions(
    store: &OperationalStore,
    now_epoch_ms: u64,
) -> Result<Vec<ActiveRetention>, SourceLifecycleError> {
    store
        .connection
        .prepare(
            "SELECT r.retention_id, r.source_artifact_id, r.authority_id, r.revision,
                    r.physical_artifact_id, r.payload_sha256, r.byte_size
             FROM source_retentions r
             JOIN source_retention_deadlines d USING(retention_id)
             WHERE r.retention_class = 'policy_persisted'
               AND r.lifecycle_state = 'active' AND d.expires_at_epoch_ms <= ?1
             ORDER BY r.retention_id",
        )
        .and_then(|mut statement| {
            statement
                .query_map(
                    [sql_u64(now_epoch_ms).map_err(|_| rusqlite::Error::InvalidQuery)?],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, i64>(6)?,
                        ))
                    },
                )?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| SourceLifecycleError::Storage)?
        .into_iter()
        .map(|row| {
            Ok(ActiveRetention {
                retention_id: row.0,
                source_artifact_id: row.1,
                authority_id: row.2,
                revision: u64::try_from(row.3).map_err(|_| SourceLifecycleError::Integrity)?,
                physical_artifact_id: row.4,
                payload_sha256: row.5,
                byte_size: u64::try_from(row.6).map_err(|_| SourceLifecycleError::Integrity)?,
            })
        })
        .collect()
}

fn active_hold_count(
    transaction: &Transaction<'_>,
    retention_id: &str,
) -> Result<u64, SourceLifecycleError> {
    let count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM source_retention_holds
             WHERE retention_id = ?1 AND active = 1",
            [retention_id],
            |row| row.get(0),
        )
        .map_err(|_| SourceLifecycleError::Storage)?;
    u64::try_from(count).map_err(|_| SourceLifecycleError::Integrity)
}

fn validate_transition_request(
    retention_id: &str,
    authority_id: &str,
    reason_code: &str,
) -> Result<(), SourceLifecycleError> {
    if !valid_identity(retention_id)
        || !valid_identity(authority_id)
        || !valid_reason_code(reason_code)
    {
        return Err(SourceLifecycleError::Invalid);
    }
    Ok(())
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn sql_u64(value: u64) -> Result<i64, SourceLifecycleError> {
    i64::try_from(value).map_err(|_| SourceLifecycleError::Invalid)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_fields(fields: &[&str]) -> String {
    let mut digest = Sha256::new();
    for field in fields {
        digest.update((field.len() as u64).to_be_bytes());
        digest.update(field.as_bytes());
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{Cursor, Read};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ContextSensitivity, PolicyId, RuntimeArtifactId,
        RuntimeArtifactIntegrityState, RuntimeArtifactKind, RuntimeArtifactManifest,
        RuntimeArtifactPreview, RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeRunId,
        SessionId, StorageFilesystemClass, StrictLocalStorageObservation, TaskId,
    };
    use rusqlite::params;

    use super::{
        SourceHoldKind, SourceHoldReleaseRequest, SourceLifecycleError, SourceRefreshRequest,
        apply_source_hold, collect_source_garbage, delete_source, expire_sources, refresh_source,
        register_source_dependency, register_source_retention_deadline, release_source_hold,
        verify_source_lifecycle,
    };
    use crate::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use crate::runtime_artifact::{
        RuntimeArtifactPayloadError, RuntimeArtifactPayloadInventoryEntry,
        RuntimeArtifactPayloadInventoryIntegrity, RuntimeArtifactPayloadObservation,
        RuntimeArtifactPayloadPlacement, RuntimeArtifactPayloadStore, publish_runtime_artifact,
        seal_runtime_artifact_manifest,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestKey;

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&[57; 32]))
        }
    }

    #[derive(Default)]
    struct FakePayloadStore {
        objects: BTreeMap<String, Vec<u8>>,
    }

    impl RuntimeArtifactPayloadStore for FakePayloadStore {
        type Staged = Vec<u8>;

        fn stage(
            &mut self,
            _artifact_id: &RuntimeArtifactId,
            source: &mut dyn Read,
            maximum_bytes: u64,
        ) -> Result<(Self::Staged, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError>
        {
            let mut bytes = Vec::new();
            source
                .take(maximum_bytes + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
            if bytes.is_empty() || bytes.len() as u64 > maximum_bytes {
                return Err(RuntimeArtifactPayloadError::ResourceLimit);
            }
            Ok((
                bytes.clone(),
                RuntimeArtifactPayloadObservation {
                    payload_sha256: super::sha256(&bytes),
                    byte_size: bytes.len() as u64,
                },
            ))
        }

        fn discard_staged(
            &mut self,
            _staged: Self::Staged,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            Ok(())
        }

        fn place(
            &mut self,
            staged: Self::Staged,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<RuntimeArtifactPayloadPlacement, RuntimeArtifactPayloadError> {
            let deduplicated = match self.objects.get(&expected.payload_sha256) {
                Some(retained) if retained == &staged => true,
                Some(_) => return Err(RuntimeArtifactPayloadError::Conflict),
                None => {
                    self.objects.insert(expected.payload_sha256.clone(), staged);
                    false
                }
            };
            Ok(RuntimeArtifactPayloadPlacement {
                observation: expected.clone(),
                deduplicated,
            })
        }

        fn verify(
            &self,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            let bytes = self
                .objects
                .get(&expected.payload_sha256)
                .ok_or(RuntimeArtifactPayloadError::Missing)?;
            if bytes.len() as u64 != expected.byte_size
                || super::sha256(bytes) != expected.payload_sha256
            {
                return Err(RuntimeArtifactPayloadError::Corrupt);
            }
            Ok(())
        }

        fn read_complete(
            &self,
            expected: &RuntimeArtifactPayloadObservation,
            maximum_bytes: u64,
        ) -> Result<Vec<u8>, RuntimeArtifactPayloadError> {
            self.verify(expected)?;
            if expected.byte_size > maximum_bytes {
                return Err(RuntimeArtifactPayloadError::ResourceLimit);
            }
            Ok(self.objects[&expected.payload_sha256].clone())
        }

        fn read_range(
            &self,
            expected: &RuntimeArtifactPayloadObservation,
            offset: u64,
            maximum_bytes: u64,
        ) -> Result<Vec<u8>, RuntimeArtifactPayloadError> {
            self.verify(expected)?;
            let bytes = &self.objects[&expected.payload_sha256];
            let start =
                usize::try_from(offset).map_err(|_| RuntimeArtifactPayloadError::Invalid)?;
            let length =
                usize::try_from(maximum_bytes).map_err(|_| RuntimeArtifactPayloadError::Invalid)?;
            if start >= bytes.len() || length == 0 {
                return Err(RuntimeArtifactPayloadError::ResourceLimit);
            }
            Ok(bytes[start..start.saturating_add(length).min(bytes.len())].to_vec())
        }

        fn quarantine(
            &mut self,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            self.objects
                .remove(&expected.payload_sha256)
                .map(|_| ())
                .ok_or(RuntimeArtifactPayloadError::Missing)
        }

        fn delete(
            &mut self,
            expected: &RuntimeArtifactPayloadObservation,
        ) -> Result<(), RuntimeArtifactPayloadError> {
            self.objects
                .remove(&expected.payload_sha256)
                .map(|_| ())
                .ok_or(RuntimeArtifactPayloadError::Missing)
        }

        fn inventory(
            &self,
        ) -> Result<Vec<RuntimeArtifactPayloadInventoryEntry>, RuntimeArtifactPayloadError>
        {
            Ok(self
                .objects
                .iter()
                .map(
                    |(payload_sha256, bytes)| RuntimeArtifactPayloadInventoryEntry {
                        payload_sha256: payload_sha256.clone(),
                        byte_size: bytes.len() as u64,
                        integrity: RuntimeArtifactPayloadInventoryIntegrity::Verified,
                    },
                )
                .collect())
        }

        fn cleanup_staging(&mut self) -> Result<u64, RuntimeArtifactPayloadError> {
            Ok(0)
        }
    }

    fn temporary_directory() -> std::path::PathBuf {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agentmage-source-lifecycle-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary directory");
        path
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None,
            root_identity_sha256: [73; 32],
            symlink_free: true,
        }
    }

    fn digest(value: char) -> String {
        value.to_string().repeat(64)
    }

    fn insert_run(store: &OperationalStore, suffix: &str) {
        store
            .connection
            .execute(
                "INSERT INTO runtime_runs VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, 1, 0,
                    NULL, NULL, 1, 1
                 )",
                params![
                    format!("run-{suffix}"),
                    format!("session-{suffix}"),
                    format!("task-{suffix}"),
                    format!("correlation-{suffix}"),
                    format!("policy-{suffix}"),
                    digest('b'),
                    digest('c'),
                    format!("event-{suffix}"),
                    digest('d'),
                ],
            )
            .expect("runtime run");
    }

    fn publish(
        store: &mut OperationalStore,
        payloads: &mut FakePayloadStore,
        suffix: &str,
    ) -> RuntimeArtifactManifest {
        let bytes = format!("source payload {suffix}").into_bytes();
        let payload_sha256 = super::sha256(&bytes);
        let manifest = seal_runtime_artifact_manifest(RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw(format!("artifact-{suffix}")),
            kind: RuntimeArtifactKind::Report,
            payload_sha256: payload_sha256.clone(),
            byte_size: bytes.len() as u64,
            media_type: "text/plain".to_owned(),
            sensitivity: ContextSensitivity::Private,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            session_id: SessionId::from_raw(format!("session-{suffix}")),
            task_id: TaskId::from_raw(format!("task-{suffix}")),
            producer_run_id: RuntimeRunId::from_raw(format!("run-{suffix}")),
            producer_turn_id: None,
            producer_operation_id: None,
            receipt_id: None,
            policy_id: PolicyId::from_raw(format!("policy-{suffix}")),
            policy_sha256: digest('b'),
            created_at_epoch_ms: 1,
            integrity: RuntimeArtifactIntegrityState::Verified,
            preview: Some(RuntimeArtifactPreview {
                text: String::from_utf8(bytes.clone()).expect("text"),
                byte_size: u32::try_from(bytes.len()).expect("preview size"),
                truncated: false,
                sha256: payload_sha256,
            }),
            manifest_sha256: digest('0'),
        })
        .expect("manifest");
        publish_runtime_artifact(store, payloads, manifest.clone(), &mut Cursor::new(bytes))
            .expect("publication");
        manifest
    }

    fn insert_source(
        store: &mut OperationalStore,
        suffix: &str,
        manifest: &RuntimeArtifactManifest,
        shared_identity: bool,
        retained: bool,
    ) {
        let identity = if shared_identity { "root" } else { suffix };
        let request_id = format!("request-{identity}");
        let authority_id = format!("authority-{identity}");
        let reference_id = format!("reference-{identity}");
        let origin_id = format!("origin-{identity}");
        let record = b"{}".as_slice();
        store
            .connection
            .execute(
                "INSERT OR IGNORE INTO source_origins VALUES (?1, ?2, ?3, 'file', 'time', ?4, ?5)",
                params![
                    &origin_id,
                    &request_id,
                    &authority_id,
                    super::sha256(origin_id.as_bytes()),
                    record
                ],
            )
            .expect("origin");
        store
            .connection
            .execute(
                "INSERT OR IGNORE INTO source_references VALUES (
                    ?1, ?2, ?3, 'file_path', 'supported', ?4, 'time', ?5
                 )",
                params![
                    &reference_id,
                    &request_id,
                    &authority_id,
                    super::sha256(reference_id.as_bytes()),
                    record,
                ],
            )
            .expect("reference");
        let source_artifact_id = format!("source-{suffix}");
        let provenance_id = format!("provenance-{suffix}");
        let provenance_sha256 = super::sha256(provenance_id.as_bytes());
        let source_transaction = store.connection.transaction().expect("source transaction");
        source_transaction
            .execute(
                "INSERT INTO source_manifests VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'text/plain', 'internal', 'fresh',
                    'captured', ?8, ?9, ?10, 'time', ?11, ?12
                 )",
                params![
                    &source_artifact_id,
                    &request_id,
                    &authority_id,
                    &reference_id,
                    &origin_id,
                    &provenance_id,
                    &provenance_sha256,
                    manifest.artifact_id.as_str(),
                    &manifest.payload_sha256,
                    i64::try_from(manifest.byte_size).expect("size"),
                    super::sha256(source_artifact_id.as_bytes()),
                    record,
                ],
            )
            .expect("manifest");
        source_transaction
            .execute(
                "INSERT INTO source_provenance VALUES (
                    ?1, ?2, ?3, ?4, 'internal', 'fresh', 'time', 'time', ?5, ?6
                 )",
                params![
                    &provenance_id,
                    &source_artifact_id,
                    &reference_id,
                    &origin_id,
                    &provenance_sha256,
                    record,
                ],
            )
            .expect("provenance");
        source_transaction.commit().expect("source commit");
        store
            .connection
            .execute(
                "INSERT INTO source_extractions VALUES (
                    ?1, ?2, 'text', '1', ?3, ?4, 'text/plain', 'parsed', 0, 1, ?5, ?6
                 )",
                params![
                    format!("extraction-{suffix}"),
                    &source_artifact_id,
                    &manifest.payload_sha256,
                    super::sha256(format!("output-{suffix}").as_bytes()),
                    super::sha256(format!("extraction-{suffix}").as_bytes()),
                    record,
                ],
            )
            .expect("extraction");
        store
            .connection
            .execute(
                "INSERT INTO source_sections VALUES (
                    ?1, ?2, ?3, NULL, 0, 'document_root', 0, ?4,
                    NULL, NULL, 1, ?5, ?6, ?7
                 )",
                params![
                    format!("section-{suffix}"),
                    &source_artifact_id,
                    format!("extraction-{suffix}"),
                    i64::try_from(manifest.byte_size).expect("size"),
                    &manifest.payload_sha256,
                    super::sha256(format!("section-{suffix}").as_bytes()),
                    record,
                ],
            )
            .expect("section");
        store
            .connection
            .execute(
                "INSERT INTO source_cache_inputs VALUES (
                    ?1, ?2, ?3, ?4, 'text', '1', ?5, ?6, ?7, ?8, ?9, ?10
                 )",
                params![
                    format!("cache-{suffix}"),
                    &source_artifact_id,
                    format!("extraction-{suffix}"),
                    &manifest.payload_sha256,
                    digest('1'),
                    digest('2'),
                    digest('3'),
                    super::sha256(format!("cache-key-{suffix}").as_bytes()),
                    super::sha256(format!("cache-record-{suffix}").as_bytes()),
                    record,
                ],
            )
            .expect("cache input");
        store
            .connection
            .execute(
                "INSERT INTO source_lexical_indexes VALUES (
                    ?1, ?2, ?3, ?4, 'sqlite-fts5', '1', ?5, 1, 1, ?6, ?7
                 )",
                params![
                    format!("index-{suffix}"),
                    &source_artifact_id,
                    format!("extraction-{suffix}"),
                    format!("cache-{suffix}"),
                    super::sha256(format!("index-{suffix}").as_bytes()),
                    super::sha256(format!("index-record-{suffix}").as_bytes()),
                    record,
                ],
            )
            .expect("lexical index");
        if retained {
            store
                .connection
                .execute(
                    "INSERT INTO source_retentions VALUES (
                        ?1, ?2, ?3, ?4, 'session', ?5, 'policy_persisted', ?6,
                        'expiry', ?7, ?8, ?9, 'encrypted_at_rest', ?10,
                        'active', NULL, 1, 'time', ?11, ?12
                     )",
                    params![
                        format!("retention-{suffix}"),
                        &source_artifact_id,
                        &request_id,
                        &authority_id,
                        format!("session-{suffix}"),
                        format!("retention-policy-{suffix}"),
                        manifest.artifact_id.as_str(),
                        &manifest.payload_sha256,
                        i64::try_from(manifest.byte_size).expect("size"),
                        super::sha256(format!("protected-{suffix}").as_bytes()),
                        super::sha256(format!("retention-{suffix}").as_bytes()),
                        record,
                    ],
                )
                .expect("retention");
            store
                .connection
                .execute(
                    "INSERT INTO source_lifecycle_events VALUES (
                        ?1, 1, 'active', NULL, 'time', ?2, ?3, ?4, ?5
                     )",
                    params![
                        format!("retention-{suffix}"),
                        digest('0'),
                        super::sha256(format!("retention-{suffix}").as_bytes()),
                        super::sha256(format!("retention-event-{suffix}").as_bytes()),
                        record,
                    ],
                )
                .expect("initial lifecycle event");
        }
    }

    fn fixture() -> (std::path::PathBuf, OperationalStore, FakePayloadStore) {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let mut store = OperationalStore::open(&path, &observation(), &mut TestKey).expect("store");
        let mut payloads = FakePayloadStore::default();
        for suffix in ["root", "replacement", "dependent"] {
            insert_run(&store, suffix);
            let manifest = publish(&mut store, &mut payloads, suffix);
            insert_source(
                &mut store,
                suffix,
                &manifest,
                matches!(suffix, "root" | "replacement"),
                suffix == "root",
            );
        }
        (directory, store, payloads)
    }

    #[test]
    fn refresh_hold_expiry_delete_and_collection_are_atomic_and_reference_safe() {
        let (directory, mut store, mut payloads) = fixture();
        register_source_dependency(&mut store, "source-root", "source-dependent", 10)
            .expect("dependency");
        assert_eq!(
            register_source_dependency(&mut store, "source-dependent", "source-root", 11),
            Err(SourceLifecycleError::Conflict),
            "cycles fail closed"
        );
        let refresh = refresh_source(
            &mut store,
            &SourceRefreshRequest {
                refresh_id: "refresh-root".to_owned(),
                replaced_source_artifact_id: "source-root".to_owned(),
                replacement_source_artifact_id: "source-replacement".to_owned(),
                expected_replaced_revision: 0,
                reason_code: "source.content_changed".to_owned(),
                occurred_at_epoch_ms: 20,
            },
        )
        .expect("refresh");
        assert_eq!(refresh.invalidated_source_count, 2);
        let current_sections: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM current_source_sections", [], |row| {
                row.get(0)
            })
            .expect("current sections");
        assert_eq!(current_sections, 1, "only the replacement remains current");
        for view in [
            "current_source_extractions",
            "current_source_cache_inputs",
            "current_source_lexical_indexes",
        ] {
            let count: i64 = store
                .connection
                .query_row(&format!("SELECT COUNT(*) FROM {view}"), [], |row| {
                    row.get(0)
                })
                .expect("current derivative count");
            assert_eq!(count, 1, "{view} excludes the invalidated closure");
        }

        register_source_retention_deadline(&mut store, "retention-root", 30).expect("deadline");
        let hold = apply_source_hold(
            &mut store,
            "retention-root",
            "authority-root",
            1,
            SourceHoldKind::User,
            "source.user_hold",
            25,
        )
        .expect("hold");
        assert!(hold.active);
        let held = expire_sources(&mut store, 30).expect("held expiry");
        assert_eq!((held.released.len(), held.held_count), (0, 1));
        release_source_hold(
            &mut store,
            &SourceHoldReleaseRequest {
                retention_id: "retention-root".to_owned(),
                authority_id: "authority-root".to_owned(),
                expected_retention_revision: 1,
                hold_kind: SourceHoldKind::User,
                expected_hold_revision: 1,
                reason_code: "source.user_hold_released".to_owned(),
                occurred_at_epoch_ms: 31,
            },
        )
        .expect("release hold");
        let expired = expire_sources(&mut store, 32).expect("expiry");
        assert_eq!(expired.released.len(), 1);
        assert_eq!(expired.released[0].lifecycle_state, "released");
        let active_references: i64 = store
            .connection
            .query_row(
                "SELECT active_reference_count FROM runtime_payloads
                 WHERE payload_sha256 = ?1",
                [&expired.released[0].payload_sha256],
                |row| row.get(0),
            )
            .expect("reference count");
        assert_eq!(active_references, 0);
        let deleted = delete_source(&mut store, "retention-root", "authority-root", 2, 40)
            .expect("delete logical source");
        assert_eq!(deleted.lifecycle_state, "deleted");
        let collection =
            collect_source_garbage(&mut store, &mut payloads, 41).expect("physical collection");
        assert_eq!(collection.deleted_orphans, 1);
        assert!(!payloads.objects.contains_key(&deleted.payload_sha256));
        verify_source_lifecycle(&store).expect("verified lifecycle");
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn stale_revision_and_active_hold_roll_back_without_runtime_reference_drift() {
        let (directory, mut store, _payloads) = fixture();
        register_source_retention_deadline(&mut store, "retention-root", 10).expect("deadline");
        apply_source_hold(
            &mut store,
            "retention-root",
            "authority-root",
            1,
            SourceHoldKind::Legal,
            "source.legal_hold",
            5,
        )
        .expect("legal hold");
        assert_eq!(
            super::release_source(
                &mut store,
                "retention-root",
                "authority-root",
                1,
                "source.owner_released",
                6,
            ),
            Err(SourceLifecycleError::Held)
        );
        assert_eq!(
            super::release_source(
                &mut store,
                "retention-root",
                "authority-root",
                2,
                "source.owner_released",
                6,
            ),
            Err(SourceLifecycleError::Conflict)
        );
        let state: (String, i64, i64) = store
            .connection
            .query_row(
                "SELECT r.lifecycle_state, r.revision, p.active_reference_count
                 FROM source_retentions r
                 JOIN runtime_payloads p ON p.payload_sha256 = r.payload_sha256
                 WHERE r.retention_id = 'retention-root'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("unchanged state");
        assert_eq!(state, ("active".to_owned(), 1, 1));
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
