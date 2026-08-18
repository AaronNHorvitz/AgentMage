//! Encrypted persistence for verified answer ledgers and external receipt anchors.

use std::fmt;

use agentmage_kernel_contracts::Receipt;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::evidence_reconciliation::{
    AnswerClaimLedger, ReceiptIntegrityAnchor, ReceiptIntegrityKey, TamperEvidentReceiptLedger,
    seal_receipt_ledger, verify_answer_claim_ledger, verify_receipt_anchor,
};
use crate::operational_store::OperationalStore;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_SCOPE_BYTES: usize = 256;
const MAX_ANSWER_LEDGER_BYTES: usize = 4 * 1024 * 1024;
const MAX_RECEIPT_LEDGER_BYTES: usize = 64 * 1024 * 1024;

/// Stable failure from encrypted evidence persistence and verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceStoreError {
    /// A ledger, scope, timestamp, or anchor failed its closed contract.
    InvalidRecord,
    /// The supplied observation clock regressed behind the retained anchor.
    ClockRegression,
    /// Encrypted persistence could not commit atomically.
    Persistence,
    /// Retained bytes, hashes, chains, or keyed anchors failed verification.
    Integrity,
    /// The requested evidence record is absent.
    NotFound,
    /// The owning operational store is poisoned.
    Poisoned,
}

impl EvidenceStoreError {
    /// Returns a content-free stable diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidRecord => "evidence.store.record_invalid",
            Self::ClockRegression => "evidence.store.clock_regression",
            Self::Persistence => "evidence.store.persistence_failed",
            Self::Integrity => "evidence.store.integrity_failed",
            Self::NotFound => "evidence.store.not_found",
            Self::Poisoned => "evidence.store.poisoned",
        }
    }
}

impl fmt::Display for EvidenceStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for EvidenceStoreError {}

/// Content-free receipt-integrity checkpoint committed beside, not inside, its ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptIntegrityCheckpoint {
    /// Stable host-selected receipt scope.
    pub scope_id: String,
    /// Number of append-only receipts covered by the checkpoint.
    pub receipt_count: u64,
    /// Terminal receipt-chain digest covered by the keyed anchor.
    pub head_receipt_sha256: String,
    /// Hash-chain identity of this durable anchor record.
    pub checkpoint_sha256: String,
}

impl OperationalStore {
    /// Persists one verified complete answer ledger in the encrypted store.
    pub fn put_answer_claim_ledger(
        &mut self,
        ledger: &AnswerClaimLedger,
        created_at_epoch_ms: u64,
    ) -> Result<(), EvidenceStoreError> {
        if self.poisoned {
            return Err(EvidenceStoreError::Poisoned);
        }
        let record_json =
            serde_json::to_vec(ledger).map_err(|_| EvidenceStoreError::InvalidRecord)?;
        if !verify_answer_claim_ledger(ledger)
            || created_at_epoch_ms == 0
            || record_json.is_empty()
            || record_json.len() > MAX_ANSWER_LEDGER_BYTES
            || sha256(&record_json) == ZERO_SHA256
        {
            return Err(EvidenceStoreError::InvalidRecord);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EvidenceStoreError::Persistence)?;
        let retained = transaction
            .query_row(
                "SELECT task_id, record_json FROM answer_claim_ledgers WHERE ledger_sha256=?1",
                [&ledger.ledger_sha256],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| EvidenceStoreError::Integrity)?;
        if let Some((task_id, bytes)) = retained {
            if task_id != ledger.task_id.as_str() || bytes != record_json {
                return Err(EvidenceStoreError::Integrity);
            }
            return transaction
                .commit()
                .map_err(|_| EvidenceStoreError::Persistence);
        }
        transaction
            .execute(
                "INSERT INTO answer_claim_ledgers(
                     ledger_sha256, task_id, created_at_epoch_ms, record_json
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    ledger.ledger_sha256,
                    ledger.task_id.as_str(),
                    to_i64(created_at_epoch_ms)?,
                    record_json,
                ],
            )
            .map_err(|_| EvidenceStoreError::Persistence)?;
        transaction
            .commit()
            .map_err(|_| EvidenceStoreError::Persistence)
    }

    /// Reads and re-verifies one exact encrypted answer ledger by digest.
    pub fn answer_claim_ledger(
        &self,
        ledger_sha256: &str,
    ) -> Result<Option<AnswerClaimLedger>, EvidenceStoreError> {
        if self.poisoned {
            return Err(EvidenceStoreError::Poisoned);
        }
        if !is_sha256(ledger_sha256) {
            return Err(EvidenceStoreError::InvalidRecord);
        }
        let record = self
            .connection
            .query_row(
                "SELECT task_id, record_json FROM answer_claim_ledgers WHERE ledger_sha256=?1",
                [ledger_sha256],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| EvidenceStoreError::Integrity)?;
        record
            .map(|(task_id, bytes)| decode_answer_ledger(ledger_sha256, &task_id, &bytes))
            .transpose()
    }

    /// Atomically persists one append-only receipt ledger and its separately keyed anchor.
    pub fn checkpoint_receipt_integrity(
        &mut self,
        scope_id: &str,
        ledger: &TamperEvidentReceiptLedger,
        key: &ReceiptIntegrityKey,
        observed_at_epoch_ms: u64,
    ) -> Result<ReceiptIntegrityCheckpoint, EvidenceStoreError> {
        if self.poisoned {
            return Err(EvidenceStoreError::Poisoned);
        }
        if !valid_scope(scope_id) || observed_at_epoch_ms == 0 || !ledger.verify() {
            return Err(EvidenceStoreError::InvalidRecord);
        }
        let anchor =
            seal_receipt_ledger(ledger, key).map_err(|_| EvidenceStoreError::InvalidRecord)?;
        let ledger_json =
            serde_json::to_vec(ledger.receipts()).map_err(|_| EvidenceStoreError::InvalidRecord)?;
        if ledger_json.is_empty() || ledger_json.len() > MAX_RECEIPT_LEDGER_BYTES {
            return Err(EvidenceStoreError::InvalidRecord);
        }
        let ledger_sha256 = sha256(&ledger_json);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EvidenceStoreError::Persistence)?;
        let previous = transaction
            .query_row(
                "SELECT receipt_count, observed_at_epoch_ms, head_receipt_sha256,
                        anchor_hmac_sha256, anchor_sha256
                 FROM receipt_integrity_anchors
                 WHERE scope_id=?1 ORDER BY receipt_count DESC LIMIT 1",
                [scope_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| EvidenceStoreError::Integrity)?;
        if let Some((count, observed, head, hmac, checkpoint)) = &previous {
            let count = u64::try_from(*count).map_err(|_| EvidenceStoreError::Integrity)?;
            let observed = u64::try_from(*observed).map_err(|_| EvidenceStoreError::Integrity)?;
            if anchor.receipt_count < count {
                return Err(EvidenceStoreError::InvalidRecord);
            }
            if observed_at_epoch_ms < observed {
                return Err(EvidenceStoreError::ClockRegression);
            }
            if anchor.receipt_count == count {
                if anchor.head_receipt_sha256 != *head || anchor.anchor_hmac_sha256 != *hmac {
                    return Err(EvidenceStoreError::Integrity);
                }
                return Ok(ReceiptIntegrityCheckpoint {
                    scope_id: scope_id.to_owned(),
                    receipt_count: count,
                    head_receipt_sha256: head.clone(),
                    checkpoint_sha256: checkpoint.clone(),
                });
            }
        }
        let previous_sha256 = previous
            .as_ref()
            .map_or(ZERO_SHA256, |value| value.4.as_str());
        let checkpoint_sha256 =
            anchor_record_sha256(scope_id, &anchor, observed_at_epoch_ms, previous_sha256);
        transaction
            .execute(
                "INSERT INTO receipt_integrity_ledgers(
                     scope_id, receipt_count, ledger_sha256, ledger_json
                 ) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(scope_id) DO UPDATE SET
                     receipt_count=excluded.receipt_count,
                     ledger_sha256=excluded.ledger_sha256,
                     ledger_json=excluded.ledger_json",
                params![
                    scope_id,
                    to_i64(anchor.receipt_count)?,
                    ledger_sha256,
                    ledger_json,
                ],
            )
            .map_err(|_| EvidenceStoreError::Persistence)?;
        transaction
            .execute(
                "INSERT INTO receipt_integrity_anchors(
                     scope_id, receipt_count, head_receipt_sha256, anchor_hmac_sha256,
                     observed_at_epoch_ms, previous_anchor_sha256, anchor_sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    scope_id,
                    to_i64(anchor.receipt_count)?,
                    anchor.head_receipt_sha256,
                    anchor.anchor_hmac_sha256,
                    to_i64(observed_at_epoch_ms)?,
                    previous_sha256,
                    checkpoint_sha256,
                ],
            )
            .map_err(|_| EvidenceStoreError::Persistence)?;
        transaction
            .commit()
            .map_err(|_| EvidenceStoreError::Persistence)?;
        Ok(ReceiptIntegrityCheckpoint {
            scope_id: scope_id.to_owned(),
            receipt_count: anchor.receipt_count,
            head_receipt_sha256: anchor.head_receipt_sha256,
            checkpoint_sha256,
        })
    }

    /// Verifies every retained receipt checkpoint with one externally brokered key.
    pub fn verify_receipt_integrity(
        &self,
        scope_id: &str,
        key: &ReceiptIntegrityKey,
    ) -> Result<ReceiptIntegrityCheckpoint, EvidenceStoreError> {
        if self.poisoned {
            return Err(EvidenceStoreError::Poisoned);
        }
        if !valid_scope(scope_id) {
            return Err(EvidenceStoreError::InvalidRecord);
        }
        let (ledger, anchors) = load_receipt_scope(self, scope_id)?;
        let mut prefix = TamperEvidentReceiptLedger::default();
        let mut anchor_index = 0_usize;
        for receipt in ledger.receipts() {
            prefix
                .append(receipt.clone())
                .map_err(|_| EvidenceStoreError::Integrity)?;
            if anchors
                .get(anchor_index)
                .is_some_and(|anchor| anchor.anchor.receipt_count == prefix.receipts().len() as u64)
            {
                if !verify_receipt_anchor(&prefix, &anchors[anchor_index].anchor, key) {
                    return Err(EvidenceStoreError::Integrity);
                }
                anchor_index += 1;
            }
        }
        if anchor_index != anchors.len() {
            return Err(EvidenceStoreError::Integrity);
        }
        let latest = anchors.last().ok_or(EvidenceStoreError::NotFound)?;
        Ok(ReceiptIntegrityCheckpoint {
            scope_id: scope_id.to_owned(),
            receipt_count: latest.anchor.receipt_count,
            head_receipt_sha256: latest.anchor.head_receipt_sha256.clone(),
            checkpoint_sha256: latest.checkpoint_sha256.clone(),
        })
    }
}

struct StoredAnchor {
    anchor: ReceiptIntegrityAnchor,
    checkpoint_sha256: String,
}

fn load_receipt_scope(
    store: &OperationalStore,
    scope_id: &str,
) -> Result<(TamperEvidentReceiptLedger, Vec<StoredAnchor>), EvidenceStoreError> {
    let retained = store
        .connection
        .query_row(
            "SELECT receipt_count, ledger_sha256, ledger_json
             FROM receipt_integrity_ledgers WHERE scope_id=?1",
            [scope_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| EvidenceStoreError::Integrity)?
        .ok_or(EvidenceStoreError::NotFound)?;
    let receipts: Vec<Receipt> =
        serde_json::from_slice(&retained.2).map_err(|_| EvidenceStoreError::Integrity)?;
    if sha256(&retained.2) != retained.1
        || u64::try_from(retained.0).ok() != Some(receipts.len() as u64)
    {
        return Err(EvidenceStoreError::Integrity);
    }
    let mut ledger = TamperEvidentReceiptLedger::default();
    for receipt in receipts {
        ledger
            .append(receipt)
            .map_err(|_| EvidenceStoreError::Integrity)?;
    }
    let mut statement = store
        .connection
        .prepare(
            "SELECT receipt_count, head_receipt_sha256, anchor_hmac_sha256,
                    observed_at_epoch_ms, previous_anchor_sha256, anchor_sha256
             FROM receipt_integrity_anchors WHERE scope_id=?1 ORDER BY receipt_count",
        )
        .map_err(|_| EvidenceStoreError::Integrity)?;
    let rows = statement
        .query_map([scope_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|_| EvidenceStoreError::Integrity)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| EvidenceStoreError::Integrity)?;
    let mut anchors = Vec::with_capacity(rows.len());
    let mut previous_count = 0_u64;
    let mut previous_observed = 0_u64;
    let mut previous_sha256 = ZERO_SHA256.to_owned();
    for (count, head, hmac, observed, previous, retained_sha256) in rows {
        let count = u64::try_from(count).map_err(|_| EvidenceStoreError::Integrity)?;
        let observed = u64::try_from(observed).map_err(|_| EvidenceStoreError::Integrity)?;
        let anchor = ReceiptIntegrityAnchor {
            receipt_count: count,
            head_receipt_sha256: head,
            anchor_hmac_sha256: hmac,
        };
        if count <= previous_count
            || count > ledger.receipts().len() as u64
            || observed < previous_observed
            || previous != previous_sha256
            || ledger.receipts()[count as usize - 1].receipt_sha256 != anchor.head_receipt_sha256
            || retained_sha256 != anchor_record_sha256(scope_id, &anchor, observed, &previous)
        {
            return Err(EvidenceStoreError::Integrity);
        }
        previous_count = count;
        previous_observed = observed;
        previous_sha256.clone_from(&retained_sha256);
        anchors.push(StoredAnchor {
            anchor,
            checkpoint_sha256: retained_sha256,
        });
    }
    if previous_count != ledger.receipts().len() as u64 {
        return Err(EvidenceStoreError::Integrity);
    }
    Ok((ledger, anchors))
}

pub(crate) fn verify_all(
    store: &OperationalStore,
) -> Result<(), crate::operational_store::OperationalStoreError> {
    let mut statement = store
        .connection
        .prepare(
            "SELECT ledger_sha256, task_id, created_at_epoch_ms, record_json
             FROM answer_claim_ledgers ORDER BY ledger_sha256",
        )
        .map_err(|_| crate::operational_store::OperationalStoreError::IntegrityFailure)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })
        .map_err(|_| crate::operational_store::OperationalStoreError::IntegrityFailure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| crate::operational_store::OperationalStoreError::IntegrityFailure)?;
    for (digest, task_id, created, bytes) in rows {
        if created <= 0 || decode_answer_ledger(&digest, &task_id, &bytes).is_err() {
            return Err(crate::operational_store::OperationalStoreError::IntegrityFailure);
        }
    }
    let scopes = store
        .connection
        .prepare("SELECT scope_id FROM receipt_integrity_ledgers ORDER BY scope_id")
        .and_then(|mut query| {
            query
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| crate::operational_store::OperationalStoreError::IntegrityFailure)?;
    for scope in scopes {
        if !valid_scope(&scope) || load_receipt_scope(store, &scope).is_err() {
            return Err(crate::operational_store::OperationalStoreError::IntegrityFailure);
        }
    }
    Ok(())
}

fn decode_answer_ledger(
    digest: &str,
    task_id: &str,
    bytes: &[u8],
) -> Result<AnswerClaimLedger, EvidenceStoreError> {
    let ledger: AnswerClaimLedger =
        serde_json::from_slice(bytes).map_err(|_| EvidenceStoreError::Integrity)?;
    if ledger.ledger_sha256 != digest
        || ledger.task_id.as_str() != task_id
        || !verify_answer_claim_ledger(&ledger)
    {
        return Err(EvidenceStoreError::Integrity);
    }
    Ok(ledger)
}

fn anchor_record_sha256(
    scope_id: &str,
    anchor: &ReceiptIntegrityAnchor,
    observed_at_epoch_ms: u64,
    previous_anchor_sha256: &str,
) -> String {
    digest(&(
        scope_id,
        anchor.receipt_count,
        &anchor.head_receipt_sha256,
        &anchor.anchor_hmac_sha256,
        observed_at_epoch_ms,
        previous_anchor_sha256,
    ))
}

fn digest(value: &impl Serialize) -> String {
    serde_json::to_vec(value).map_or_else(|_| ZERO_SHA256.to_owned(), |bytes| sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn valid_scope(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SCOPE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn to_i64(value: u64) -> Result<i64, EvidenceStoreError> {
    i64::try_from(value).map_err(|_| EvidenceStoreError::InvalidRecord)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        ActionId, ApprovalId, AuthorityTransactionId, CONTRACT_SCHEMA_VERSION, CorrelationId,
        GrantId, GrantOperation, MaterialClaim, MaterialClaimKind, OperationAttemptId,
        OperationBinding, OperationOutcome, Receipt, ReceiptId, SessionId, StorageFilesystemClass,
        StrictLocalStorageObservation, TaskId, UnknownBlockedReason,
    };

    use super::{EvidenceStoreError, ReceiptIntegrityKey, TamperEvidentReceiptLedger, sha256};
    use crate::evidence_reconciliation::build_answer_claim_ledger;
    use crate::evidence_state::{DeterministicMethodRegistry, EvidenceStateAssigner};
    use crate::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None,
            root_identity_sha256: [9; 32],
            symlink_free: true,
        }
    }

    fn directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agentmage-evidence-store-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("temporary directory creates");
        path
    }

    fn answer_ledger() -> crate::evidence_reconciliation::AnswerClaimLedger {
        let task_id = TaskId::from_raw("task-evidence-store-1");
        let mut assigner =
            EvidenceStateAssigner::new(task_id.clone(), DeterministicMethodRegistry::empty())
                .expect("assigner creates");
        assigner
            .assign_unknown_blocked(
                "assignment-unavailable",
                MaterialClaim {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    claim_id: "claim-unavailable".to_owned(),
                    task_id,
                    kind: MaterialClaimKind::Read,
                    statement: "Source was unavailable.".to_owned(),
                    subject_id: "source-unavailable".to_owned(),
                    expected_revision: "revision-1".to_owned(),
                    prerequisite_claim_ids: Vec::new(),
                },
                UnknownBlockedReason::Unavailable,
                "source.unavailable".to_owned(),
                Vec::new(),
            )
            .expect("unknown assignment records");
        build_answer_claim_ledger(
            assigner.finalize(),
            vec!["claim-unavailable".to_owned()],
            BTreeMap::from([(
                "assignment-unavailable".to_owned(),
                (Vec::new(), vec!["source.unavailable".to_owned()]),
            )]),
        )
        .expect("ledger builds")
    }

    fn receipt(sequence: u64, previous: &str) -> Receipt {
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let mut receipt = Receipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            receipt_id: ReceiptId::from_raw(format!("receipt-store-{sequence}")),
            sequence,
            correlation_id: CorrelationId::from_raw("correlation-store-1"),
            authority_transaction_id: AuthorityTransactionId::from_raw(format!(
                "transaction-store-{sequence}"
            )),
            operation_attempt_id: OperationAttemptId::from_raw(format!("attempt-store-{sequence}")),
            approval_id: ApprovalId::from_raw(format!("approval-store-{sequence}")),
            grant_id: GrantId::from_raw(format!("grant-store-{sequence}")),
            session_id: SessionId::from_raw("session-store-1"),
            task_id: TaskId::from_raw("task-store-1"),
            action_id: ActionId::from_raw(format!("action-store-{sequence}")),
            tool_call_id: None,
            operation,
            outcome: OperationOutcome::Succeeded,
            operation_sha256: sha256(
                &serde_json::to_vec(&operation).expect("operation serializes"),
            ),
            evidence: Vec::new(),
            error: None,
            previous_receipt_sha256: previous.to_owned(),
            receipt_sha256: super::ZERO_SHA256.to_owned(),
            occurred_at: format!("1970-01-01T00:00:0{sequence}Z"),
        };
        let mut unsigned = receipt.clone();
        unsigned.receipt_sha256 = super::ZERO_SHA256.to_owned();
        receipt.receipt_sha256 = sha256(
            &agentmage_kernel_contracts::to_canonical_json(&unsigned).expect("receipt serializes"),
        );
        receipt
    }

    #[test]
    fn encrypted_ledgers_and_external_anchors_survive_verified_restart() {
        let directory = directory();
        let path = directory.join("authority.db");
        let mut store = OperationalStore::open(&path, &observation(), &mut TestKey([31; 32]))
            .expect("store opens");
        let answer = answer_ledger();
        store
            .put_answer_claim_ledger(&answer, 1_000)
            .expect("answer persists");
        assert_eq!(
            store
                .answer_claim_ledger(&answer.ledger_sha256)
                .expect("answer reads"),
            Some(answer.clone())
        );

        let anchor_key = ReceiptIntegrityKey::new([41; 32]).expect("anchor key admits");
        let mut receipts = TamperEvidentReceiptLedger::default();
        let first = receipt(1, super::ZERO_SHA256);
        receipts
            .append(first.clone())
            .expect("first receipt appends");
        let first_checkpoint = store
            .checkpoint_receipt_integrity("authority.receipts", &receipts, &anchor_key, 2_000)
            .expect("first checkpoint persists");
        let second = receipt(2, &first.receipt_sha256);
        receipts.append(second).expect("second receipt appends");
        let second_checkpoint = store
            .checkpoint_receipt_integrity("authority.receipts", &receipts, &anchor_key, 3_000)
            .expect("second checkpoint persists");
        assert_ne!(
            first_checkpoint.checkpoint_sha256,
            second_checkpoint.checkpoint_sha256
        );
        assert_eq!(
            store
                .checkpoint_receipt_integrity("authority.receipts", &receipts, &anchor_key, 2_999,)
                .expect_err("clock regression rejects"),
            EvidenceStoreError::ClockRegression
        );
        drop(store);

        let reopened = OperationalStore::open(&path, &observation(), &mut TestKey([31; 32]))
            .expect("verified restart opens");
        assert_eq!(
            reopened
                .answer_claim_ledger(&answer.ledger_sha256)
                .expect("answer reopens"),
            Some(answer)
        );
        assert_eq!(
            reopened
                .verify_receipt_integrity("authority.receipts", &anchor_key)
                .expect("anchors verify"),
            second_checkpoint
        );
        assert_eq!(
            reopened
                .verify_receipt_integrity(
                    "authority.receipts",
                    &ReceiptIntegrityKey::new([42; 32]).expect("wrong key admits"),
                )
                .expect_err("wrong key rejects"),
            EvidenceStoreError::Integrity
        );
        drop(reopened);
        fs::remove_dir_all(directory).expect("temporary directory removes");
    }

    #[test]
    fn retained_answer_or_anchor_tampering_blocks_reopen() {
        let directory = directory();
        let path = directory.join("authority.db");
        let mut store = OperationalStore::open(&path, &observation(), &mut TestKey([51; 32]))
            .expect("store opens");
        let answer = answer_ledger();
        store
            .put_answer_claim_ledger(&answer, 1_000)
            .expect("answer persists");
        store
            .connection
            .execute(
                "UPDATE answer_claim_ledgers SET record_json=X'7b7d' WHERE ledger_sha256=?1",
                [&answer.ledger_sha256],
            )
            .expect("tamper injects");
        drop(store);
        assert!(OperationalStore::open(&path, &observation(), &mut TestKey([51; 32])).is_err());
        fs::remove_dir_all(directory).expect("temporary directory removes");
    }
}
