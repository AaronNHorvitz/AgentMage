// Real encrypted operational stores in private temporary directories; no
// process, transport or model effects.
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use agentmage_kernel_contracts::{StorageFilesystemClass, StrictLocalStorageObservation};

use super::*;
use crate::action_history::{
    ActionAuthorization, ActionHistoryExportRequest, ActionKind, ActionOutcome,
};
use crate::operational_store::{
    DurableAuthorityRuntime, OperationalStoreError, OperationalStoreKeyError,
    OperationalStoreKeyProvider,
};

const RUN: &str = "run-history-fixture";
const OWNER: &str = "owner-history-fixture";
const OTHER_OWNER: &str = "owner-history-other";
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
        root_identity_sha256: [7; 32],
        symlink_free: true,
    }
}

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
    key: [u8; 32],
}

impl Fixture {
    fn new(key: u8) -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "agentmage-run-history-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).expect("temporary directory");
        let path = directory.join("authority.db");
        Self {
            directory,
            path,
            key: [key; 32],
        }
    }

    fn open(&self) -> Result<OperationalStore, OperationalStoreError> {
        OperationalStore::open(&self.path, &observation(), &mut TestKey(self.key))
    }

    fn histories(&self) -> DurableRunActionHistories {
        DurableRunActionHistories::new(Arc::new(Mutex::new(self.open().expect("store opens"))))
    }

    /// Runs raw SQL as a key holder that bypasses the owner's rules.
    fn tamper(&self, sql: &str) {
        let store = self.open().expect("store opens for tampering");
        store.connection.execute_batch(sql).expect("tampering SQL");
    }

    fn record_count(&self) -> i64 {
        let store = self.open().expect("store opens");
        store
            .connection
            .query_row(
                "SELECT COUNT(*) FROM run_action_history_records",
                [],
                |row| row.get(0),
            )
            .expect("record count")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn digest(digit: char) -> String {
    digit.to_string().repeat(64)
}

fn draft(
    action_id: &str,
    recorded_at_epoch_ms: u64,
    retain_until_epoch_ms: u64,
) -> ActionRecordDraft {
    ActionRecordDraft {
        action_kind: ActionKind::FileWrite,
        action_id: action_id.to_owned(),
        authorization: ActionAuthorization::Grant {
            grant_id: format!("grant-{action_id}"),
            grant_sha256: digest('a'),
        },
        effect_sha256: digest('b'),
        outcome: ActionOutcome::Succeeded,
        reason_code: "coding.approved.succeeded".to_owned(),
        evidence_sha256s: vec![digest('c')],
        recorded_at_epoch_ms,
        retain_until_epoch_ms,
    }
}

/// An open effects chain with three kept entries.
fn populated(fixture: &Fixture) -> ActionHistoryHead {
    let histories = fixture.histories();
    histories
        .create(RUN, RunActionChainName::Effects, OWNER)
        .unwrap();
    let mut head = None;
    for (index, id) in ["operation-1", "operation-2", "operation-3"]
        .iter()
        .enumerate()
    {
        let at = 1_000 + index as u64;
        head = Some(
            histories
                .append(
                    RUN,
                    RunActionChainName::Effects,
                    OWNER,
                    &draft(id, at, at + 10_000),
                )
                .unwrap(),
        );
    }
    head.unwrap()
}

#[test]
fn a_chain_persists_each_entry_and_replays_after_reopening() {
    let fixture = Fixture::new(71);
    let head = populated(&fixture);
    assert_eq!(head.count, 3);
    assert_eq!(fixture.record_count(), 3);
    // A reopened store replays the chain to the same head, in the same
    // canonical records, still open and complete.
    let stored = fixture
        .histories()
        .history(RUN, RunActionChainName::Effects)
        .unwrap();
    assert_eq!(stored.head, head);
    assert_eq!(stored.records.len(), 3);
    assert!(stored.complete && !stored.closed);
    let replayed = ActionHistory::replay(stored.records.clone(), &stored.head).unwrap();
    assert_eq!(replayed.head(), head);
    // Every chain of a run is its own; another chain of the same run is
    // separate and starts empty.
    let histories = fixture.histories();
    assert_eq!(
        histories.history(RUN, RunActionChainName::Routes),
        Err(RunActionHistoryStoreError::NotFound)
    );
    histories
        .create(RUN, RunActionChainName::Routes, OWNER)
        .unwrap();
    let routes = histories.history(RUN, RunActionChainName::Routes).unwrap();
    assert_eq!(routes.head.count, 0);
    assert_eq!(routes.head.head_sha256, GENESIS_SHA256);
    // Closing ends the chain; a closed chain replays the same records.
    let closed = histories
        .close(RUN, RunActionChainName::Effects, OWNER)
        .unwrap();
    assert!(closed.closed && closed.complete);
    assert_eq!(closed.records, stored.records);
    drop(histories);
    let reopened = fixture
        .histories()
        .history(RUN, RunActionChainName::Effects)
        .unwrap();
    assert_eq!(reopened, closed);
}

#[test]
fn the_runtime_shares_one_store_with_its_run_histories() {
    let fixture = Fixture::new(72);
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 1)
            .unwrap();
    let histories = runtime.run_action_histories();
    histories
        .create(RUN, RunActionChainName::JobControl, OWNER)
        .unwrap();
    let clone = histories.clone();
    clone
        .append(
            RUN,
            RunActionChainName::JobControl,
            OWNER,
            &draft("cancel-1", 5, 10),
        )
        .unwrap();
    assert_eq!(
        histories
            .history(RUN, RunActionChainName::JobControl)
            .unwrap()
            .head
            .count,
        1
    );
    drop((histories, clone, runtime));
    // Opening the runtime replays every chain before it is usable.
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 2)
            .unwrap();
    let stored = runtime
        .run_action_histories()
        .history(RUN, RunActionChainName::JobControl)
        .unwrap();
    assert_eq!(stored.head.count, 1);
}

#[test]
fn only_the_owner_changes_an_open_chain_and_a_closed_chain_takes_nothing() {
    let fixture = Fixture::new(73);
    let head = populated(&fixture);
    let histories = fixture.histories();
    let chain = RunActionChainName::Effects;
    let next = draft("operation-4", 2_000, 20_000);
    assert_eq!(
        histories.create(RUN, chain, OWNER),
        Err(RunActionHistoryStoreError::Exists)
    );
    for refused in [
        histories.append(RUN, chain, OTHER_OWNER, &next).map(|_| ()),
        histories.mark_incomplete(RUN, chain, OTHER_OWNER),
        histories.close(RUN, chain, OTHER_OWNER).map(|_| ()),
        histories.attach(RUN, chain, OTHER_OWNER, false).map(|_| ()),
    ] {
        assert_eq!(refused, Err(RunActionHistoryStoreError::NotOwner));
    }
    for run in ["", "run\0id", &"r".repeat(129)] {
        assert_eq!(
            histories.history(run, chain),
            Err(RunActionHistoryStoreError::InvalidInput)
        );
        assert_eq!(
            histories.create(run, chain, OWNER),
            Err(RunActionHistoryStoreError::InvalidInput)
        );
    }
    assert_eq!(
        histories.create("run-other", chain, ""),
        Err(RunActionHistoryStoreError::InvalidInput)
    );
    assert_eq!(
        histories.append("run-missing", chain, OWNER, &next),
        Err(RunActionHistoryStoreError::NotFound)
    );
    // A draft the history refuses (time backwards) writes nothing.
    assert_eq!(
        histories.append(RUN, chain, OWNER, &draft("operation-0", 10, 1_000)),
        Err(RunActionHistoryStoreError::History(
            ActionHistoryError::InvalidInput
        ))
    );
    assert_eq!(histories.history(RUN, chain).unwrap().head, head);
    histories.close(RUN, chain, OWNER).unwrap();
    // Closing twice writes nothing; a closed chain takes no entry or mark.
    assert_eq!(histories.close(RUN, chain, OWNER).unwrap().head, head);
    for refused in [
        histories.append(RUN, chain, OWNER, &next).map(|_| ()),
        histories.mark_incomplete(RUN, chain, OWNER),
        histories.attach(RUN, chain, OWNER, true).map(|_| ()),
    ] {
        assert_eq!(refused, Err(RunActionHistoryStoreError::Closed));
    }
    let stored = histories.history(RUN, chain).unwrap();
    assert!(stored.closed && stored.complete);
    assert_eq!(stored.head, head);
    drop(histories);
    assert_eq!(fixture.record_count(), 3);
}

#[test]
fn a_chain_is_bounded_and_full_writes_nothing() {
    let fixture = Fixture::new(74);
    let histories = fixture.histories();
    histories
        .create(RUN, RunActionChainName::Routes, OWNER)
        .unwrap();
    for index in 0..MAX_RUN_ACTION_HISTORY_POSITIONS as u64 {
        histories
            .append(
                RUN,
                RunActionChainName::Routes,
                OWNER,
                &draft(&format!("route-{index}"), index, index + 1),
            )
            .unwrap();
    }
    assert_eq!(
        histories.append(
            RUN,
            RunActionChainName::Routes,
            OWNER,
            &draft("route-overflow", 600, 601)
        ),
        Err(RunActionHistoryStoreError::Full)
    );
    drop(histories);
    assert_eq!(
        fixture.record_count(),
        MAX_RUN_ACTION_HISTORY_POSITIONS as i64
    );
    let stored = fixture
        .histories()
        .history(RUN, RunActionChainName::Routes)
        .unwrap();
    assert_eq!(stored.head.count, MAX_RUN_ACTION_HISTORY_POSITIONS as u64);
}

#[test]
fn an_attach_after_a_restart_marks_the_chain_incomplete_for_good() {
    let fixture = Fixture::new(75);
    populated(&fixture);
    let histories = fixture.histories();
    let chain = RunActionChainName::Effects;
    // Attaching in the same host keeps the chain complete.
    let kept = histories.attach(RUN, chain, OWNER, false).unwrap();
    assert!(kept.complete && !kept.closed);
    // Attaching after a restart marks it incomplete, and appends continue.
    let attached = histories.attach(RUN, chain, OWNER, true).unwrap();
    assert!(!attached.complete);
    assert_eq!(attached.head, kept.head);
    histories
        .append(RUN, chain, OWNER, &draft("operation-4", 2_000, 20_000))
        .unwrap();
    histories.mark_incomplete(RUN, chain, OWNER).unwrap();
    let closed = histories.close(RUN, chain, OWNER).unwrap();
    assert!(!closed.complete && closed.closed);
    assert_eq!(closed.head.count, 4);
    drop(histories);
    // A key holder cannot make it complete again, reopen it, or append to
    // it once closed.
    let store = fixture.open().unwrap();
    for sql in [
        "UPDATE run_action_history_heads SET complete = 1",
        "UPDATE run_action_history_heads SET closed = 0",
    ] {
        assert!(store.connection.execute_batch(sql).is_err(), "{sql}");
    }
    drop(store);
    let stored = fixture.histories().history(RUN, chain).unwrap();
    assert!(!stored.complete && stored.closed);
    // A separately marked open chain is incomplete too.
    let histories = fixture.histories();
    histories
        .create(RUN, RunActionChainName::JobControl, OWNER)
        .unwrap();
    histories
        .mark_incomplete(RUN, RunActionChainName::JobControl, OWNER)
        .unwrap();
    assert!(
        !histories
            .history(RUN, RunActionChainName::JobControl)
            .unwrap()
            .complete
    );
}

#[test]
fn a_record_and_its_head_commit_together_or_not_at_all() {
    for (trigger, table) in [
        ("reject_head", "UPDATE ON run_action_history_heads"),
        ("reject_record", "INSERT ON run_action_history_records"),
    ] {
        let fixture = Fixture::new(76);
        let head = populated(&fixture);
        fixture.tamper(&format!(
            "CREATE TRIGGER {trigger} BEFORE {table}
             BEGIN SELECT RAISE(ABORT, 'synthetic commit failure'); END;"
        ));
        let histories = fixture.histories();
        assert_eq!(
            histories.append(
                RUN,
                RunActionChainName::Effects,
                OWNER,
                &draft("operation-4", 2_000, 20_000)
            ),
            Err(RunActionHistoryStoreError::Storage)
        );
        // Neither the record nor its head was kept, and the store stays usable.
        assert_eq!(
            histories
                .history(RUN, RunActionChainName::Effects)
                .unwrap()
                .head,
            head
        );
        drop(histories);
        assert_eq!(fixture.record_count(), 3);
        fixture.tamper(&format!("DROP TRIGGER {trigger};"));
        let histories = fixture.histories();
        assert_eq!(
            histories
                .append(
                    RUN,
                    RunActionChainName::Effects,
                    OWNER,
                    &draft("operation-4", 2_000, 20_000)
                )
                .unwrap()
                .count,
            4
        );
        drop(histories);
        assert_eq!(fixture.record_count(), 4);
    }
}

#[test]
fn an_append_that_names_another_head_writes_nothing() {
    // The head update names the head the owner read, so an append prepared
    // against any other head commits no record.
    let fixture = Fixture::new(77);
    let head = populated(&fixture);
    let histories = fixture.histories();
    let result = histories.with_store(|store| {
        let loaded = load(store, RUN, RunActionChainName::Effects)?;
        let mut history = loaded.history;
        let entry = history
            .append(&draft("operation-4", 2_000, 20_000))
            .map_err(RunActionHistoryStoreError::History)?;
        let other = ActionHistoryHead {
            count: head.count,
            head_sha256: "f".repeat(64),
        };
        let refused = commit_append(
            store,
            RUN,
            RunActionChainName::Effects,
            &other,
            &history.head(),
            &ActionHistoryRecord::Kept(entry),
        );
        // Nothing was committed, so the handle stays usable.
        assert_eq!(
            load(store, RUN, RunActionChainName::Effects)?.stored.head,
            head
        );
        refused
    });
    assert_eq!(result, Err(RunActionHistoryStoreError::Integrity));
    drop(histories);
    assert_eq!(fixture.record_count(), 3);
    assert_eq!(
        fixture
            .histories()
            .history(RUN, RunActionChainName::Effects)
            .unwrap()
            .head,
        head
    );
}

#[test]
fn retained_rows_are_append_only_and_heads_only_move_forward() {
    let fixture = Fixture::new(78);
    populated(&fixture);
    let store = fixture.open().unwrap();
    let earlier: String = store
        .connection
        .query_row(
            "SELECT entry_sha256 FROM run_action_history_records WHERE sequence = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    for sql in [
        "UPDATE run_action_history_roots SET owner_id = 'another-owner'",
        "DELETE FROM run_action_history_roots",
        // A kept record may change only into its expired place.
        "UPDATE run_action_history_records SET record_json = X'7B7D' WHERE sequence = 1",
        "UPDATE run_action_history_records SET sequence = 9 WHERE sequence = 3",
        "DELETE FROM run_action_history_records WHERE sequence = 3",
        "DELETE FROM run_action_history_heads",
        "UPDATE run_action_history_heads SET entry_count = 2",
        "UPDATE run_action_history_heads SET entry_count = 5",
        "UPDATE run_action_history_heads SET head_sha256 = 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff'",
        // REPLACE removes the conflicting row, which the delete triggers
        // refuse because the store turns recursive triggers on.
        "INSERT OR REPLACE INTO run_action_history_roots(run_id, chain, owner_id)
             SELECT run_id, chain, 'another-owner' FROM run_action_history_roots",
        "INSERT OR REPLACE INTO run_action_history_records(run_id, chain, sequence, entry_sha256, expired, record_json)
             SELECT run_id, chain, sequence, entry_sha256, 0, X'7B7D'
             FROM run_action_history_records WHERE sequence = 1",
        "REPLACE INTO run_action_history_heads(run_id, chain, entry_count, head_sha256, complete, closed)
             SELECT run_id, chain, 0, '0000000000000000000000000000000000000000000000000000000000000000', 1, 0
             FROM run_action_history_heads",
    ] {
        assert!(store.connection.execute_batch(sql).is_err(), "{sql}");
    }
    assert!(
        store
            .connection
            .execute(
                "UPDATE run_action_history_heads SET entry_count = 1, head_sha256 = ?1",
                [&earlier],
            )
            .is_err()
    );
    drop(store);
    assert_eq!(
        fixture
            .histories()
            .history(RUN, RunActionChainName::Effects)
            .unwrap()
            .head
            .count,
        3
    );
}

#[test]
fn a_tampered_chain_is_refused_poisons_the_store_and_blocks_reopening() {
    // Each case bypasses the owner's triggers or foreign keys, as only a key
    // holder outside the owner could.
    let cases: [(&str, &str); 7] = [
        (
            "rolled-back head",
            "DROP TRIGGER run_action_history_head_forward_only;
             UPDATE run_action_history_heads SET entry_count = 2,
                 head_sha256 = (SELECT entry_sha256 FROM run_action_history_records WHERE sequence = 2);",
        ),
        (
            "changed outcome",
            "DROP TRIGGER run_action_history_record_expire_only;
             UPDATE run_action_history_records
                 SET record_json = CAST(replace(CAST(record_json AS TEXT), '\"succeeded\"', '\"failed\"') AS BLOB)
                 WHERE sequence = 2;",
        ),
        (
            "non-canonical encoding",
            "DROP TRIGGER run_action_history_record_expire_only;
             UPDATE run_action_history_records
                 SET record_json = CAST(CAST(record_json AS TEXT) || ' ' AS BLOB)
                 WHERE sequence = 1;",
        ),
        (
            "expired flag without its place",
            "DROP TRIGGER run_action_history_record_expire_only;
             UPDATE run_action_history_records SET expired = 1 WHERE sequence = 1;",
        ),
        (
            "missing head",
            "DROP TRIGGER run_action_history_head_delete_forbidden;
             DELETE FROM run_action_history_heads;",
        ),
        (
            "missing record",
            "PRAGMA foreign_keys = OFF;
             DROP TRIGGER run_action_history_record_delete_forbidden;
             DELETE FROM run_action_history_records WHERE sequence = 2;",
        ),
        (
            "record outside every chain",
            "PRAGMA foreign_keys = OFF;
             INSERT INTO run_action_history_records(run_id, chain, sequence, entry_sha256, expired, record_json)
                 SELECT 'run-orphan', chain, sequence, entry_sha256, expired, record_json
                 FROM run_action_history_records WHERE sequence = 1;",
        ),
    ];
    for (name, sql) in cases {
        let fixture = Fixture::new(79);
        populated(&fixture);
        fixture.tamper(sql);
        assert_eq!(
            fixture.open().err(),
            Some(OperationalStoreError::IntegrityFailure),
            "{name}"
        );
        if name == "record outside every chain" {
            // The run's own chain still replays; only the opening refuses.
            continue;
        }
        // A handle over a store opened before the change refuses it too, and
        // the store stays poisoned.
        let fixture = Fixture::new(81);
        populated(&fixture);
        let histories = fixture.histories();
        histories
            .with_store(|store| {
                store
                    .connection
                    .execute_batch(sql)
                    .map_err(|_| RunActionHistoryStoreError::Storage)
            })
            .unwrap();
        assert_eq!(
            histories.history(RUN, RunActionChainName::Effects),
            Err(RunActionHistoryStoreError::Integrity),
            "{name}"
        );
        assert_eq!(
            histories.history(RUN, RunActionChainName::Effects),
            Err(RunActionHistoryStoreError::Unavailable),
            "{name}"
        );
        assert_eq!(
            histories.append(
                RUN,
                RunActionChainName::Effects,
                OWNER,
                &draft("operation-9", 9_000, 90_000)
            ),
            Err(RunActionHistoryStoreError::Unavailable),
            "{name}"
        );
    }
}

#[test]
fn retention_keeps_only_the_places_of_a_closed_chain_past_its_deadline() {
    let fixture = Fixture::new(80);
    let histories = fixture.histories();
    let chain = RunActionChainName::Effects;
    histories.create(RUN, chain, OWNER).unwrap();
    histories
        .append(RUN, chain, OWNER, &draft("operation-1", 1_000, 2_000))
        .unwrap();
    histories
        .append(RUN, chain, OWNER, &draft("operation-2", 1_100, 5_000))
        .unwrap();
    let head = histories
        .append(RUN, chain, OWNER, &draft("operation-3", 1_200, 2_500))
        .unwrap();
    // An open chain is never expired.
    assert_eq!(
        histories.apply_retention(RUN, chain, 3_000),
        Err(RunActionHistoryStoreError::NotClosed)
    );
    histories.close(RUN, chain, OWNER).unwrap();
    assert_eq!(histories.apply_retention(RUN, chain, 1_999), Ok(0));
    assert_eq!(histories.apply_retention(RUN, chain, 3_000), Ok(2));
    // Applying again changes nothing.
    assert_eq!(histories.apply_retention(RUN, chain, 3_000), Ok(0));
    drop(histories);
    let stored = fixture.histories().history(RUN, chain).unwrap();
    assert_eq!(stored.head, head);
    let states = stored
        .records
        .iter()
        .map(|record| matches!(record, ActionHistoryRecord::Expired { .. }))
        .collect::<Vec<_>>();
    assert_eq!(states, [true, false, true]);
    // The places still replay to the head, and an export counts them as
    // places without their content.
    let replayed = ActionHistory::replay(stored.records, &stored.head).unwrap();
    let export = replayed
        .export(ActionHistoryExportRequest {
            from_sequence: 1,
            to_sequence: 3,
        })
        .unwrap();
    assert_eq!((export.kept_count, export.expired_count), (1, 2));
    assert!(
        !String::from_utf8(export.bytes)
            .unwrap()
            .contains("operation-1")
    );
    // An expired place cannot be made kept again.
    let store = fixture.open().unwrap();
    assert!(
        store
            .connection
            .execute_batch("UPDATE run_action_history_records SET expired = 0 WHERE sequence = 1")
            .is_err()
    );
}
