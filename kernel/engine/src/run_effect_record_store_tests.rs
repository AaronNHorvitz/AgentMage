// Real encrypted operational stores in private temporary directories; no
// process, transport, model or file effects.
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use agentmage_kernel_contracts::{
    RuntimeArtifactId, StorageFilesystemClass, StrictLocalStorageObservation,
};

use super::*;
use crate::operational_store::{
    DurableAuthorityRuntime, OperationalStoreError, OperationalStoreKeyError,
    OperationalStoreKeyProvider,
};

const SESSION: &str = "session-effects-fixture";
const TASK: &str = "task-effects-fixture";
const RUN: &str = "run-effects-fixture";
const OWNER: &str = "owner-effects-fixture";
const OTHER_OWNER: &str = "owner-effects-other";
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

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
    key: [u8; 32],
}

impl Fixture {
    fn new(key: u8) -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "agentmage-run-effects-{}-{sequence}",
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

    fn records(&self) -> DurableRunEffectRecords {
        DurableRunEffectRecords::new(Arc::new(Mutex::new(self.open().expect("store opens"))))
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
            .query_row("SELECT COUNT(*) FROM run_effect_records", [], |row| {
                row.get(0)
            })
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

fn write(operation_id: &str) -> RunEffectEntry {
    RunEffectEntry::Execution {
        operation_id: operation_id.to_owned(),
        kind: RunEffectKind::Write,
        path: None,
        outcome: Some(OperationOutcome::Succeeded),
        changed: true,
    }
}

fn create_entry(operation_id: &str, path: &[&str]) -> RunEffectEntry {
    RunEffectEntry::Execution {
        operation_id: operation_id.to_owned(),
        kind: RunEffectKind::Create,
        path: Some(
            path.iter()
                .map(|component| (*component).to_owned())
                .collect(),
        ),
        outcome: Some(OperationOutcome::Succeeded),
        changed: true,
    }
}

fn publication(artifact: &str) -> RunEffectEntry {
    RunEffectEntry::Publication {
        reference: RuntimeArtifactRef {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw(artifact),
            manifest_sha256: digest('a'),
            payload_sha256: digest('b'),
            byte_size: 64,
            media_type: "application/vnd.agentmage.coding-change-record+json".to_owned(),
        },
        policy_sha256: digest('c'),
    }
}

/// An open chain with a write, its change record and a command.
fn populated(fixture: &Fixture) -> (u64, String) {
    let records = fixture.records();
    records.create(SESSION, TASK, RUN, OWNER).unwrap();
    let mut head = None;
    for entry in [
        write("operation-1"),
        publication("artifact-change-1"),
        RunEffectEntry::Execution {
            operation_id: "operation-2".to_owned(),
            kind: RunEffectKind::Command,
            path: None,
            outcome: Some(OperationOutcome::Failed),
            changed: false,
        },
    ] {
        head = Some(records.append(RUN, OWNER, &entry).unwrap());
    }
    head.unwrap()
}

#[test]
fn a_chain_persists_each_entry_and_replays_after_reopening() {
    let fixture = Fixture::new(140);
    let (count, head_sha256) = populated(&fixture);
    assert_eq!(count, 3);
    // Each head digest chains the entry to the head before it.
    let stored = fixture.records().run(RUN).unwrap();
    let mut previous = GENESIS_SHA256.to_owned();
    for (index, entry) in stored.entries.iter().enumerate() {
        previous = run_effect_entry_sha256(RUN, index as u64 + 1, &previous, entry).unwrap();
    }
    assert_eq!(previous, head_sha256);
    assert_eq!(stored.head_sha256, head_sha256);
    assert_eq!(stored.entry_count, 3);
    assert_eq!(stored.entries[0], write("operation-1"));
    assert_eq!(stored.entries[1], publication("artifact-change-1"));
    assert_eq!(
        (stored.session_id.as_str(), stored.task_id.as_str()),
        (SESSION, TASK)
    );
    assert_eq!(stored.session_position, 1);
    assert!(stored.complete && !stored.closed);
    // A new handle over a reopened store replays the same chain.
    let reopened = fixture.records().run(RUN).unwrap();
    assert_eq!(reopened, stored);
    assert_eq!(fixture.record_count(), 3);
}

#[test]
fn the_runtime_shares_one_store_with_its_run_effect_records() {
    let fixture = Fixture::new(141);
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 1)
            .unwrap();
    let records = runtime.run_effect_records();
    records.create(SESSION, TASK, RUN, OWNER).unwrap();
    let clone = records.clone();
    clone
        .append(RUN, OWNER, &create_entry("create-1", &["src", "new.rs"]))
        .unwrap();
    assert_eq!(records.run(RUN).unwrap().entry_count, 1);
    drop((records, clone, runtime));
    // Opening the runtime replays every chain before it is usable.
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 2)
            .unwrap();
    let stored = runtime.run_effect_records().run(RUN).unwrap();
    assert_eq!(
        stored.entries,
        [create_entry("create-1", &["src", "new.rs"])]
    );
}

#[test]
fn a_session_lists_its_runs_in_order_and_only_its_own() {
    let fixture = Fixture::new(142);
    let records = fixture.records();
    for (session, run) in [
        (SESSION, "run-a"),
        ("session-other", "run-x"),
        (SESSION, "run-b"),
        (SESSION, "run-c"),
    ] {
        records.create(session, TASK, run, OWNER).unwrap();
        records.append(run, OWNER, &write("operation-1")).unwrap();
    }
    records.close("run-a", OWNER).unwrap();
    let session = records.session(SESSION).unwrap();
    assert_eq!(
        session
            .iter()
            .map(|run| (run.run_id.as_str(), run.session_position, run.closed))
            .collect::<Vec<_>>(),
        [("run-a", 1, true), ("run-b", 2, false), ("run-c", 3, false)]
    );
    let other = records.session("session-other").unwrap();
    assert_eq!(other.len(), 1);
    assert_eq!(other[0].session_position, 1);
    assert_eq!(records.session("session-empty").unwrap(), []);
    // A run belongs to one session; it cannot be created twice.
    assert_eq!(
        records.create("session-other", TASK, "run-b", OWNER),
        Err(RunEffectRecordStoreError::Exists)
    );
}

#[test]
fn only_the_owner_changes_an_open_chain_and_a_closed_chain_takes_nothing() {
    let fixture = Fixture::new(143);
    let head = populated(&fixture);
    let records = fixture.records();
    assert_eq!(
        records.append(RUN, OTHER_OWNER, &write("operation-9")),
        Err(RunEffectRecordStoreError::NotOwner)
    );
    assert_eq!(
        records.mark_incomplete(RUN, OTHER_OWNER),
        Err(RunEffectRecordStoreError::NotOwner)
    );
    assert_eq!(
        records.close(RUN, OTHER_OWNER),
        Err(RunEffectRecordStoreError::NotOwner)
    );
    assert_eq!(
        records.attach(RUN, OTHER_OWNER, false),
        Err(RunEffectRecordStoreError::NotOwner)
    );
    let closed = records.close(RUN, OWNER).unwrap();
    assert!(closed.closed && closed.complete);
    assert_eq!((closed.entry_count, closed.head_sha256.clone()), head);
    // Closing again writes nothing; nothing else reaches a closed chain.
    assert_eq!(records.close(RUN, OWNER).unwrap(), closed);
    assert_eq!(
        records.append(RUN, OWNER, &write("operation-9")),
        Err(RunEffectRecordStoreError::Closed)
    );
    assert_eq!(
        records.mark_incomplete(RUN, OWNER),
        Err(RunEffectRecordStoreError::Closed)
    );
    assert_eq!(
        records.attach(RUN, OWNER, true),
        Err(RunEffectRecordStoreError::Closed)
    );
    assert_eq!(records.run(RUN).unwrap(), closed);
    assert_eq!(
        records.run("run-missing"),
        Err(RunEffectRecordStoreError::NotFound)
    );
}

#[test]
fn an_attach_after_a_restart_marks_the_chain_incomplete_for_good() {
    let fixture = Fixture::new(144);
    populated(&fixture);
    let records = fixture.records();
    let attached = records.attach(RUN, OWNER, false).unwrap();
    assert!(attached.complete);
    let marked = records.attach(RUN, OWNER, true).unwrap();
    assert!(!marked.complete);
    assert_eq!(marked.entry_count, 3);
    // The flag only clears: attaching without marking keeps it cleared, and
    // the chain still takes entries and closes incomplete.
    assert!(!records.attach(RUN, OWNER, false).unwrap().complete);
    records.append(RUN, OWNER, &write("operation-3")).unwrap();
    records.mark_incomplete(RUN, OWNER).unwrap();
    let closed = records.close(RUN, OWNER).unwrap();
    assert!(!closed.complete && closed.closed);
    drop(records);
    assert!(!fixture.records().run(RUN).unwrap().complete);
}

#[test]
fn malformed_identities_and_entries_are_refused_before_anything_is_written() {
    let fixture = Fixture::new(145);
    populated(&fixture);
    let records = fixture.records();
    let long = "r".repeat(129);
    for (session, task, run, owner) in [
        ("", TASK, "run-new", OWNER),
        (SESSION, "", "run-new", OWNER),
        (SESSION, TASK, "", OWNER),
        (SESSION, TASK, "run-new", ""),
        (long.as_str(), TASK, "run-new", OWNER),
        (SESSION, TASK, "run\0new", OWNER),
    ] {
        assert_eq!(
            records.create(session, task, run, owner),
            Err(RunEffectRecordStoreError::InvalidInput)
        );
    }
    let path = |components: &[&str]| -> Option<Vec<String>> {
        Some(components.iter().map(|value| (*value).to_owned()).collect())
    };
    let invalid = [
        // A creation names its path, and nothing else does.
        RunEffectEntry::Execution {
            operation_id: "create-1".to_owned(),
            kind: RunEffectKind::Create,
            path: None,
            outcome: Some(OperationOutcome::Succeeded),
            changed: true,
        },
        RunEffectEntry::Execution {
            operation_id: "write-1".to_owned(),
            kind: RunEffectKind::Write,
            path: path(&["src", "lib.rs"]),
            outcome: Some(OperationOutcome::Succeeded),
            changed: true,
        },
        create_entry("create-2", &[]),
        create_entry("create-3", &["src", ".."]),
        create_entry("create-4", &["src", "a/b"]),
        create_entry("create-5", &["."]),
        create_entry("create-6", &[""]),
        // An unknown outcome cannot claim a change.
        RunEffectEntry::Execution {
            operation_id: "write-2".to_owned(),
            kind: RunEffectKind::Write,
            path: None,
            outcome: None,
            changed: true,
        },
        write(""),
        RunEffectEntry::Publication {
            reference: RuntimeArtifactRef {
                manifest_sha256: "A".repeat(64),
                ..match publication("artifact-1") {
                    RunEffectEntry::Publication { reference, .. } => reference,
                    RunEffectEntry::Execution { .. } => unreachable!(),
                }
            },
            policy_sha256: digest('c'),
        },
        // A publication names the exact policy revision it was published under.
        RunEffectEntry::Publication {
            reference: match publication("artifact-1") {
                RunEffectEntry::Publication { reference, .. } => reference,
                RunEffectEntry::Execution { .. } => unreachable!(),
            },
            policy_sha256: "c".repeat(63),
        },
    ];
    for entry in &invalid {
        assert_eq!(
            records.append(RUN, OWNER, entry),
            Err(RunEffectRecordStoreError::InvalidInput),
            "{entry:?}"
        );
    }
    // An entry over the record bound is refused too.
    let oversized = create_entry("create-7", &["d".repeat(255).as_str(); 17]);
    assert_eq!(
        records.append(RUN, OWNER, &oversized),
        Err(RunEffectRecordStoreError::InvalidInput)
    );
    drop(records);
    assert_eq!(fixture.record_count(), 3);
    assert_eq!(fixture.records().run(RUN).unwrap().entry_count, 3);
}

#[test]
fn a_chain_and_a_session_are_bounded_and_full_writes_nothing() {
    let fixture = Fixture::new(146);
    let records = fixture.records();
    records.create(SESSION, TASK, RUN, OWNER).unwrap();
    for index in 0..MAX_RUN_EFFECT_POSITIONS {
        records
            .append(RUN, OWNER, &write(&format!("operation-{index}")))
            .unwrap();
    }
    assert_eq!(
        records.append(RUN, OWNER, &write("operation-over")),
        Err(RunEffectRecordStoreError::Full)
    );
    assert_eq!(
        records.run(RUN).unwrap().entry_count,
        MAX_RUN_EFFECT_POSITIONS as u64
    );
    // One more run's positions exceed what one session read loads.
    records.create(SESSION, TASK, "run-second", OWNER).unwrap();
    for index in 0..=(MAX_SESSION_POSITIONS - MAX_RUN_EFFECT_POSITIONS) {
        if index < MAX_RUN_EFFECT_POSITIONS {
            records
                .append("run-second", OWNER, &write(&format!("operation-{index}")))
                .unwrap();
        }
    }
    assert_eq!(
        records.session(SESSION).unwrap().len(),
        2,
        "two full runs fit one read"
    );
    records.create(SESSION, TASK, "run-third", OWNER).unwrap();
    records
        .append("run-third", OWNER, &write("operation-over"))
        .unwrap();
    assert_eq!(
        records.session(SESSION),
        Err(RunEffectRecordStoreError::Full)
    );
    // A session holds at most its runs.
    let other = Fixture::new(147);
    let records = other.records();
    for index in 0..MAX_SESSION_RUNS {
        records
            .create(SESSION, TASK, &format!("run-{index}"), OWNER)
            .unwrap();
    }
    assert_eq!(
        records.create(SESSION, TASK, "run-over", OWNER),
        Err(RunEffectRecordStoreError::Full)
    );
    assert_eq!(records.session(SESSION).unwrap().len(), MAX_SESSION_RUNS);
    records
        .create("session-next", TASK, "run-over", OWNER)
        .unwrap();
}

#[test]
fn a_record_and_its_head_commit_together_or_not_at_all() {
    for (trigger, table) in [
        ("reject_head", "UPDATE ON run_effect_record_heads"),
        ("reject_record", "INSERT ON run_effect_records"),
    ] {
        let fixture = Fixture::new(148);
        let head = populated(&fixture);
        fixture.tamper(&format!(
            "CREATE TRIGGER {trigger} BEFORE {table}
             BEGIN SELECT RAISE(ABORT, 'synthetic commit failure'); END;"
        ));
        let records = fixture.records();
        assert_eq!(
            records.append(RUN, OWNER, &write("operation-4")),
            Err(RunEffectRecordStoreError::Storage)
        );
        // Neither the record nor its head was kept, and the store stays usable.
        let stored = records.run(RUN).unwrap();
        assert_eq!((stored.entry_count, stored.head_sha256), head);
        drop(records);
        assert_eq!(fixture.record_count(), 3);
        fixture.tamper(&format!("DROP TRIGGER {trigger};"));
        assert_eq!(
            fixture
                .records()
                .append(RUN, OWNER, &write("operation-4"))
                .unwrap()
                .0,
            4
        );
        assert_eq!(fixture.record_count(), 4);
    }
}

#[test]
fn retained_rows_are_append_only_and_heads_only_move_forward() {
    let fixture = Fixture::new(149);
    populated(&fixture);
    let store = fixture.open().unwrap();
    let earlier: String = store
        .connection
        .query_row(
            "SELECT entry_sha256 FROM run_effect_records WHERE sequence = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    for sql in [
        "UPDATE run_effect_record_roots SET owner_id = 'another-owner'",
        "UPDATE run_effect_record_roots SET session_position = 2",
        "DELETE FROM run_effect_record_roots",
        "UPDATE run_effect_records SET record_json = X'7B7D' WHERE sequence = 1",
        "UPDATE run_effect_records SET sequence = 9 WHERE sequence = 3",
        "DELETE FROM run_effect_records WHERE sequence = 3",
        "DELETE FROM run_effect_record_heads",
        "UPDATE run_effect_record_heads SET entry_count = 2",
        "UPDATE run_effect_record_heads SET entry_count = 5",
        "UPDATE run_effect_record_heads SET head_sha256 = 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff'",
        "UPDATE run_effect_record_heads SET closed = 1, complete = 1, entry_count = 4",
        // A head advances only to a record it names.
        "UPDATE run_effect_record_heads SET entry_count = 4",
        // REPLACE removes the conflicting row, which the delete triggers
        // refuse because the store turns recursive triggers on.
        "INSERT OR REPLACE INTO run_effect_record_roots(run_id, session_id, task_id, owner_id, session_position)
             SELECT run_id, session_id, task_id, 'another-owner', session_position FROM run_effect_record_roots",
        "INSERT OR REPLACE INTO run_effect_records(run_id, sequence, entry_sha256, record_json)
             SELECT run_id, sequence, entry_sha256, X'7B7D' FROM run_effect_records WHERE sequence = 1",
        "REPLACE INTO run_effect_record_heads(run_id, entry_count, head_sha256, complete, closed)
             SELECT run_id, 0, '0000000000000000000000000000000000000000000000000000000000000000', 1, 0
             FROM run_effect_record_heads",
    ] {
        assert!(store.connection.execute_batch(sql).is_err(), "{sql}");
    }
    assert!(
        store
            .connection
            .execute(
                "UPDATE run_effect_record_heads SET entry_count = 1, head_sha256 = ?1",
                [&earlier],
            )
            .is_err()
    );
    // Even past an existing record, a head cannot advance to a digest that
    // record does not have.
    let transaction = store.connection.unchecked_transaction().unwrap();
    transaction
        .execute_batch(
            "INSERT INTO run_effect_records(run_id, sequence, entry_sha256, record_json)
             SELECT run_id, 4, entry_sha256, record_json FROM run_effect_records
             WHERE sequence = 3",
        )
        .unwrap();
    assert!(
        transaction
            .execute_batch(
                "UPDATE run_effect_record_heads SET entry_count = 4,
                 head_sha256 = '1111111111111111111111111111111111111111111111111111111111111111'",
            )
            .is_err()
    );
    transaction.rollback().unwrap();
    // A cleared complete flag cannot be set again, and a closed head cannot
    // reopen.
    store
        .connection
        .execute_batch("UPDATE run_effect_record_heads SET complete = 0")
        .unwrap();
    for sql in [
        "UPDATE run_effect_record_heads SET complete = 1",
        "UPDATE run_effect_record_heads SET closed = 1; UPDATE run_effect_record_heads SET closed = 0",
    ] {
        assert!(store.connection.execute_batch(sql).is_err(), "{sql}");
    }
    drop(store);
    let stored = fixture.records().run(RUN).unwrap();
    assert_eq!(stored.entry_count, 3);
}

#[test]
fn a_tampered_chain_is_refused_poisons_the_store_and_blocks_reopening() {
    // Each case bypasses the owner's triggers or foreign keys, as only a key
    // holder outside the owner could.
    let cases: [(&str, &str); 8] = [
        (
            "rolled-back head",
            "DROP TRIGGER run_effect_record_head_forward_only;
             UPDATE run_effect_record_heads SET entry_count = 2,
                 head_sha256 = (SELECT entry_sha256 FROM run_effect_records WHERE sequence = 2);",
        ),
        (
            "changed outcome",
            "DROP TRIGGER run_effect_record_update_forbidden;
             UPDATE run_effect_records
                 SET record_json = CAST(replace(CAST(record_json AS TEXT), '\"failed\"', '\"succeeded\"') AS BLOB)
                 WHERE sequence = 3;",
        ),
        (
            "non-canonical encoding",
            "DROP TRIGGER run_effect_record_update_forbidden;
             UPDATE run_effect_records
                 SET record_json = CAST(CAST(record_json AS TEXT) || ' ' AS BLOB)
                 WHERE sequence = 1;",
        ),
        (
            "entry outside the closed shape",
            "DROP TRIGGER run_effect_record_update_forbidden;
             UPDATE run_effect_records
                 SET record_json = CAST(replace(CAST(record_json AS TEXT), '\"kind\":\"write\"', '\"kind\":\"create\"') AS BLOB)
                 WHERE sequence = 1;",
        ),
        (
            "moved session position",
            "DROP TRIGGER run_effect_record_root_update_forbidden;
             UPDATE run_effect_record_roots SET session_position = 2;",
        ),
        (
            "missing head",
            "DROP TRIGGER run_effect_record_head_delete_forbidden;
             DELETE FROM run_effect_record_heads;",
        ),
        (
            "missing record",
            "PRAGMA foreign_keys = OFF;
             DROP TRIGGER run_effect_record_delete_forbidden;
             DELETE FROM run_effect_records WHERE sequence = 2;",
        ),
        (
            "record outside every chain",
            "PRAGMA foreign_keys = OFF;
             INSERT INTO run_effect_records(run_id, sequence, entry_sha256, record_json)
                 SELECT 'run-orphan', sequence, entry_sha256, record_json
                 FROM run_effect_records WHERE sequence = 1;",
        ),
    ];
    for (name, sql) in cases {
        let fixture = Fixture::new(150);
        populated(&fixture);
        fixture.tamper(sql);
        if name == "moved session position" {
            // The chain itself still replays; only the session read refuses
            // the gap before the run's position.
            let records = fixture.records();
            assert_eq!(
                records.session(SESSION),
                Err(RunEffectRecordStoreError::Integrity),
                "{name}"
            );
            assert_eq!(
                records.run(RUN),
                Err(RunEffectRecordStoreError::Unavailable),
                "{name}"
            );
            continue;
        }
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
        let fixture = Fixture::new(151);
        populated(&fixture);
        let records = fixture.records();
        records
            .with_store(|store| {
                store
                    .connection
                    .execute_batch(sql)
                    .map_err(|_| RunEffectRecordStoreError::Storage)
            })
            .unwrap();
        assert_eq!(
            records.run(RUN),
            Err(RunEffectRecordStoreError::Integrity),
            "{name}"
        );
        assert_eq!(
            records.run(RUN),
            Err(RunEffectRecordStoreError::Unavailable),
            "{name}"
        );
        assert_eq!(
            records.append(RUN, OWNER, &write("operation-9")),
            Err(RunEffectRecordStoreError::Unavailable),
            "{name}"
        );
    }
}
