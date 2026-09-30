// Real encrypted operational stores in private temporary directories; no
// process, transport or model effects.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use agentmage_kernel_contracts::{StorageFilesystemClass, StrictLocalStorageObservation};

use super::*;
use crate::job_control::{JobControlAction, JobControlRefusal, JobPhase};
use crate::operational_store::{
    DurableAuthorityRuntime, OperationalStoreError, OperationalStoreKeyError,
    OperationalStoreKeyProvider,
};

const JOB: &str = "job-ledger-fixture";
const OWNER: &str = "owner-ledger-fixture";
const CLIENT: &str = "client-scope-a";
const OTHER_CLIENT: &str = "client-scope-b";
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
            "agentmage-job-ledger-{}-{sequence}",
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

    fn ledgers(&self) -> DurableJobLedgers {
        DurableJobLedgers::new(Arc::new(Mutex::new(self.open().expect("store opens"))))
    }

    /// Runs raw SQL as a key holder that bypasses the owner's rules.
    fn tamper(&self, sql: &str) {
        let store = self.open().expect("store opens for tampering");
        store.connection.execute_batch(sql).expect("tampering SQL");
    }

    fn entry_count(&self) -> i64 {
        let store = self.open().expect("store opens");
        store
            .connection
            .query_row("SELECT COUNT(*) FROM job_ledger_entries", [], |row| {
                row.get(0)
            })
            .expect("entry count")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn request(id: &str, action: JobControlAction, observed_revision: u64) -> JobControlRequest {
    JobControlRequest {
        schema_version: 1,
        job_id: JOB.to_owned(),
        request_id: id.to_owned(),
        action,
        observed_revision,
    }
}

/// A running job with an accepted suspension and a recorded stale refusal.
fn populated(fixture: &Fixture) {
    let ledgers = fixture.ledgers();
    ledgers.create(JOB, OWNER).unwrap();
    ledgers
        .observe_owner(JOB, OWNER, JobOwnerEvent::Started)
        .unwrap();
    ledgers
        .control(JOB, CLIENT, &request("s1", JobControlAction::Suspend, 1))
        .unwrap();
    ledgers
        .control(
            JOB,
            OTHER_CLIENT,
            &request("c1", JobControlAction::Cancel, 1),
        )
        .unwrap();
}

#[test]
fn a_job_ledger_persists_each_decision_and_replays_after_reopening() {
    let fixture = Fixture::new(41);
    let ledgers = fixture.ledgers();
    let queued = ledgers.create(JOB, OWNER).unwrap();
    assert_eq!((queued.phase, queued.revision), (JobPhase::Queued, 0));
    assert_eq!(
        ledgers.create(JOB, "another-owner"),
        Err(JobLedgerStoreError::Exists)
    );
    assert_eq!(
        ledgers.observation("unknown-job"),
        Err(JobLedgerStoreError::NotFound)
    );
    assert_eq!(
        ledgers.create("bad job", OWNER),
        Err(JobLedgerStoreError::Ledger(JobControlError::Invalid))
    );
    // Only the recorded owner advances the job.
    assert_eq!(
        ledgers.observe_owner(JOB, "another-owner", JobOwnerEvent::Started),
        Err(JobLedgerStoreError::NotOwner)
    );
    ledgers
        .observe_owner(JOB, OWNER, JobOwnerEvent::Started)
        .unwrap();
    let suspend = request("s1", JobControlAction::Suspend, 1);
    let applied = JobControlDecision::Applied {
        revision: 2,
        phase: JobPhase::Suspending,
    };
    assert_eq!(ledgers.control(JOB, CLIENT, &suspend), Ok(applied));
    drop(ledgers);
    assert_eq!(fixture.entry_count(), 3);

    // A client that reconnects to a restarted owner gets its original
    // decision, and the retry writes nothing.
    let ledgers = fixture.ledgers();
    assert_eq!(ledgers.control(JOB, CLIENT, &suspend), Ok(applied));
    // The same identity from another client is that client's own request.
    assert_eq!(
        ledgers.control(JOB, OTHER_CLIENT, &suspend),
        Ok(JobControlDecision::Refused {
            refusal: JobControlRefusal::StaleRevision,
            revision: 2,
            phase: JobPhase::Suspending,
        })
    );
    assert_eq!(
        ledgers.control(JOB, CLIENT, &request("s1", JobControlAction::Cancel, 2)),
        Err(JobLedgerStoreError::Ledger(
            JobControlError::RequestConflict
        ))
    );
    let suspended = ledgers
        .observe_owner(JOB, OWNER, JobOwnerEvent::SuspensionObserved)
        .unwrap();
    assert_eq!(
        (suspended.phase, suspended.revision),
        (JobPhase::Suspended, 3)
    );
    drop(ledgers);
    assert_eq!(fixture.entry_count(), 5);
    let ledgers = fixture.ledgers();
    assert_eq!(ledgers.observation(JOB), Ok(suspended));
    assert_eq!(ledgers.control(JOB, CLIENT, &suspend), Ok(applied));
    drop(ledgers);
    assert_eq!(fixture.entry_count(), 5);
}

#[test]
fn the_runtime_shares_one_store_with_its_job_ledgers() {
    let fixture = Fixture::new(42);
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 1)
            .unwrap();
    let ledgers = runtime.job_ledgers();
    ledgers.create(JOB, OWNER).unwrap();
    let clone = ledgers.clone();
    clone
        .control(JOB, CLIENT, &request("c1", JobControlAction::Cancel, 0))
        .unwrap();
    assert_eq!(ledgers.observation(JOB).unwrap().phase, JobPhase::Cancelled);
    drop((ledgers, clone, runtime));
    // Opening the runtime replays every ledger before it is usable.
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 2)
            .unwrap();
    let observation = runtime.job_ledgers().observation(JOB).unwrap();
    assert!(observation.cancellation_requested);
    assert_eq!(observation.phase, JobPhase::Cancelled);
}

#[test]
fn an_entry_and_its_head_commit_together_or_not_at_all() {
    for (trigger, table) in [
        ("reject_head", "UPDATE ON job_ledger_heads"),
        ("reject_entry", "INSERT ON job_ledger_entries"),
    ] {
        let fixture = Fixture::new(43);
        populated(&fixture);
        let before = fixture.ledgers().observation(JOB).unwrap();
        let entries = fixture.entry_count();
        fixture.tamper(&format!(
            "CREATE TRIGGER {trigger} BEFORE {table}
             BEGIN SELECT RAISE(ABORT, 'synthetic commit failure'); END;"
        ));
        let ledgers = fixture.ledgers();
        assert_eq!(
            ledgers.control(JOB, CLIENT, &request("r1", JobControlAction::Resume, 2)),
            Err(JobLedgerStoreError::Storage)
        );
        assert_eq!(
            ledgers.observe_owner(JOB, OWNER, JobOwnerEvent::SuspensionObserved),
            Err(JobLedgerStoreError::Storage)
        );
        // Neither the entry nor its head was kept, and the store stays usable.
        assert_eq!(ledgers.observation(JOB), Ok(before.clone()));
        drop(ledgers);
        assert_eq!(fixture.entry_count(), entries);
        fixture.tamper(&format!("DROP TRIGGER {trigger};"));
        let ledgers = fixture.ledgers();
        assert!(matches!(
            ledgers.control(JOB, CLIENT, &request("r1", JobControlAction::Resume, 2)),
            Ok(JobControlDecision::Applied {
                phase: JobPhase::Running,
                ..
            })
        ));
        drop(ledgers);
        assert_eq!(fixture.entry_count(), entries + 1);
    }
}

#[test]
fn retained_rows_are_append_only_and_heads_only_advance() {
    let fixture = Fixture::new(44);
    populated(&fixture);
    let store = fixture.open().unwrap();
    let head: (i64, String) = store
        .connection
        .query_row(
            "SELECT entry_count, head_sha256 FROM job_ledger_heads WHERE job_id = ?1",
            [JOB],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let earlier: String = store
        .connection
        .query_row(
            "SELECT entry_sha256 FROM job_ledger_entries WHERE job_id = ?1 AND sequence = 1",
            [JOB],
            |row| row.get(0),
        )
        .unwrap();
    for sql in [
        "UPDATE job_ledger_roots SET owner_id = 'another-owner'",
        "DELETE FROM job_ledger_roots",
        "UPDATE job_ledger_entries SET entry_json = X'7B7D' WHERE sequence = 1",
        "DELETE FROM job_ledger_entries WHERE sequence = 3",
        "DELETE FROM job_ledger_heads",
    ] {
        assert!(store.connection.execute_batch(sql).is_err(), "{sql}");
    }
    assert!(
        store
            .connection
            .execute(
                "UPDATE job_ledger_heads SET entry_count = 2, last_sequence = 1, head_sha256 = ?1",
                [&earlier],
            )
            .is_err()
    );
    drop(store);
    let observation = fixture.ledgers().observation(JOB).unwrap();
    assert_eq!(observation.head_sha256, head.1);
    assert_eq!(fixture.entry_count(), head.0);
}

#[test]
fn a_tampered_ledger_is_refused_poisons_the_store_and_blocks_reopening() {
    // Each case bypasses the owner's triggers or foreign keys, as only a key
    // holder outside the owner could.
    let cases: [(&str, &str); 7] = [
        (
            "rolled-back head",
            "DROP TRIGGER job_ledger_head_advance_only;
             UPDATE job_ledger_heads SET entry_count = 3, last_sequence = 2,
                 head_sha256 = (SELECT entry_sha256 FROM job_ledger_entries WHERE sequence = 2);",
        ),
        (
            "changed decision",
            "DROP TRIGGER job_ledger_entry_update_forbidden;
             UPDATE job_ledger_entries
                 SET entry_json = CAST(replace(CAST(entry_json AS TEXT), 'stale-revision', 'not-allowed') AS BLOB)
                 WHERE sequence = 3;",
        ),
        (
            "non-canonical encoding",
            "DROP TRIGGER job_ledger_entry_update_forbidden;
             UPDATE job_ledger_entries
                 SET entry_json = CAST(CAST(entry_json AS TEXT) || ' ' AS BLOB)
                 WHERE sequence = 1;",
        ),
        (
            "moved client scope",
            "DROP TRIGGER job_ledger_entry_update_forbidden;
             UPDATE job_ledger_entries
                 SET entry_json = CAST(replace(CAST(entry_json AS TEXT), 'client-scope-a', 'client-scope-b') AS BLOB)
                 WHERE sequence = 2;",
        ),
        (
            "changed owner",
            "DROP TRIGGER job_ledger_root_update_forbidden;
             UPDATE job_ledger_roots SET owner_id = 'another-owner';",
        ),
        (
            "missing head",
            "DROP TRIGGER job_ledger_head_delete_forbidden;
             DELETE FROM job_ledger_heads;",
        ),
        (
            "missing entry",
            "PRAGMA foreign_keys = OFF;
             DROP TRIGGER job_ledger_entry_delete_forbidden;
             DELETE FROM job_ledger_entries WHERE sequence = 1;",
        ),
    ];
    for (name, sql) in cases {
        let fixture = Fixture::new(45);
        populated(&fixture);
        fixture.tamper(sql);
        let store =
            OperationalStore::open(&fixture.path, &observation(), &mut TestKey(fixture.key));
        assert_eq!(
            store.err(),
            Some(OperationalStoreError::IntegrityFailure),
            "{name}"
        );
        // A handle over a store opened before the change refuses it too, and
        // the store stays poisoned.
        let fixture = Fixture::new(46);
        populated(&fixture);
        let ledgers = fixture.ledgers();
        ledgers
            .with_store(|store| {
                store
                    .connection
                    .execute_batch(sql)
                    .map_err(|_| JobLedgerStoreError::Storage)
            })
            .unwrap();
        assert_eq!(
            ledgers.observation(JOB),
            Err(JobLedgerStoreError::Integrity),
            "{name}"
        );
        assert_eq!(
            ledgers.observation(JOB),
            Err(JobLedgerStoreError::Unavailable),
            "{name}"
        );
        assert_eq!(
            ledgers.control(JOB, CLIENT, &request("c9", JobControlAction::Cancel, 2)),
            Err(JobLedgerStoreError::Unavailable),
            "{name}"
        );
    }
}

#[test]
fn every_ledger_row_has_a_content_free_export_family() {
    let fixture = Fixture::new(47);
    populated(&fixture);
    let store = fixture.open().unwrap();
    let destination: &Path = &fixture.directory.join("export.jsonl");
    store
        .export_json_lines(destination, &observation())
        .expect("derived export");
    let exported = std::fs::read_to_string(destination).unwrap();
    for family in ["job_ledger_roots", "job_ledger_entries", "job_ledger_heads"] {
        assert!(exported.contains(&format!("\"{family}\"")), "{family}");
    }
    // Identities and digests only: no request identity, scope or decision.
    for content in ["client-scope-a", "stale-revision", "\"s1\"", OWNER] {
        assert!(!exported.contains(content), "{content}");
    }
}
