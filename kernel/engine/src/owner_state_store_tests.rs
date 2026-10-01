// Real encrypted operational stores in private temporary directories; no
// process, transport or model effects.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use agentmage_kernel_contracts::{StorageFilesystemClass, StrictLocalStorageObservation};

use super::*;
use crate::operational_store::{
    DurableAuthorityRuntime, OperationalStoreError, OperationalStoreKeyError,
    OperationalStoreKeyProvider,
};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);
const OWNER: OwnerStateName = OwnerStateName::MemoryCatalog;

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
            "agentmage-owner-states-{}-{sequence}",
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

    fn states(&self) -> DurableOwnerStates {
        DurableOwnerStates::new(Arc::new(Mutex::new(self.open().expect("store opens"))))
    }

    /// Runs raw SQL as a key holder that bypasses the owner's rules.
    fn tamper(&self, sql: &str) {
        let store = self.open().expect("store opens for tampering");
        store.connection.execute_batch(sql).expect("tampering SQL");
    }

    fn directory(&self) -> &Path {
        &self.directory
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

/// Commits two states and returns the handle at revision 2.
fn populated(fixture: &Fixture) -> DurableOwnerStates {
    let states = fixture.states();
    assert_eq!(states.commit(OWNER, 0, b"first state").unwrap(), 1);
    assert_eq!(states.commit(OWNER, 1, b"second state").unwrap(), 2);
    states
}

#[test]
fn each_state_persists_and_reloads_after_reopening() {
    let fixture = Fixture::new(61);
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 1)
            .unwrap();
    let states = runtime.owner_states();
    assert_eq!(
        states.load(OWNER).unwrap(),
        StoredOwnerState {
            revision: 0,
            state: Vec::new()
        }
    );
    let clone = states.clone();
    assert_eq!(states.commit(OWNER, 0, b"first state").unwrap(), 1);
    assert_eq!(clone.commit(OWNER, 1, b"second state").unwrap(), 2);
    drop((states, clone, runtime));
    // Opening the runtime checks every state before it is usable.
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 2)
            .unwrap();
    assert_eq!(
        runtime.owner_states().load(OWNER).unwrap(),
        StoredOwnerState {
            revision: 2,
            state: b"second state".to_vec()
        }
    );
}

#[test]
fn a_commit_that_names_another_revision_or_no_state_writes_nothing() {
    let fixture = Fixture::new(62);
    let states = populated(&fixture);
    for read in [0, 1, 3, u64::MAX] {
        assert_eq!(
            states.commit(OWNER, read, b"stale state"),
            Err(OwnerStateStoreError::Stale),
            "{read}"
        );
    }
    assert_eq!(
        states.commit(OWNER, 2, b""),
        Err(OwnerStateStoreError::InvalidInput)
    );
    assert_eq!(
        states.commit(OWNER, 2, &vec![b'x'; MAX_OWNER_STATE_BYTES + 1]),
        Err(OwnerStateStoreError::ResourceLimit)
    );
    assert_eq!(
        states.load(OWNER).unwrap(),
        StoredOwnerState {
            revision: 2,
            state: b"second state".to_vec()
        }
    );
}

#[test]
fn a_state_and_its_revision_commit_together_or_not_at_all() {
    let fixture = Fixture::new(63);
    drop(populated(&fixture));
    fixture.tamper(
        "CREATE TRIGGER reject_state BEFORE UPDATE ON owner_states
         BEGIN SELECT RAISE(ABORT, 'synthetic commit failure'); END;",
    );
    let states = fixture.states();
    assert_eq!(
        states.commit(OWNER, 2, b"third state"),
        Err(OwnerStateStoreError::Storage)
    );
    assert_eq!(states.load(OWNER).unwrap().revision, 2);
    drop(states);
    fixture.tamper("DROP TRIGGER reject_state;");
    assert_eq!(
        fixture.states().commit(OWNER, 2, b"third state").unwrap(),
        3
    );
}

#[test]
fn retained_states_only_move_forward() {
    let fixture = Fixture::new(64);
    drop(populated(&fixture));
    let store = fixture.open().unwrap();
    for sql in [
        "UPDATE owner_states SET revision = revision + 2",
        "UPDATE owner_states SET revision = revision - 1",
        "UPDATE owner_states SET state = X'78'",
        "UPDATE owner_states SET owner_id = 'other-owner', revision = revision + 1",
        "DELETE FROM owner_states",
        "INSERT INTO owner_states(owner_id, revision, state, state_sha256)
             VALUES ('other-owner', 2, X'78', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')",
        // REPLACE removes the conflicting row, which the delete trigger
        // refuses because the store turns recursive triggers on.
        "REPLACE INTO owner_states(owner_id, revision, state, state_sha256)
             SELECT owner_id, 1, state, state_sha256 FROM owner_states",
    ] {
        assert!(store.connection.execute_batch(sql).is_err(), "{sql}");
    }
    drop(store);
    assert_eq!(fixture.states().load(OWNER).unwrap().revision, 2);
}

#[test]
fn a_tampered_state_is_refused_poisons_the_store_and_blocks_reopening() {
    // Each case bypasses the owner's triggers, as only a key holder outside
    // the owner could.
    let cases = [
        (
            "changed state",
            "DROP TRIGGER owner_state_forward_only;
             UPDATE owner_states SET state = CAST('changed state' AS BLOB);",
        ),
        (
            "changed digest",
            "DROP TRIGGER owner_state_forward_only;
             UPDATE owner_states SET state_sha256 = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';",
        ),
        (
            "unknown owner",
            "INSERT INTO owner_states(owner_id, revision, state, state_sha256)
                 SELECT 'unknown-owner', 1, state, state_sha256 FROM owner_states;",
        ),
    ];
    for (name, sql) in cases {
        let fixture = Fixture::new(65);
        drop(populated(&fixture));
        fixture.tamper(sql);
        assert_eq!(
            fixture.open().err(),
            Some(OperationalStoreError::IntegrityFailure),
            "{name}"
        );
        if name == "unknown owner" {
            // The known owner's own state still loads; only the opening refuses.
            continue;
        }
        // A handle over a store opened before the change refuses it too, and
        // the store stays poisoned.
        let fixture = Fixture::new(66);
        let states = populated(&fixture);
        states
            .with_store(|store| {
                store
                    .connection
                    .execute_batch(sql)
                    .map_err(|_| OwnerStateStoreError::Storage)
            })
            .unwrap();
        assert_eq!(
            states.load(OWNER),
            Err(OwnerStateStoreError::Integrity),
            "{name}"
        );
        assert_eq!(
            states.load(OWNER),
            Err(OwnerStateStoreError::Unavailable),
            "{name}"
        );
        assert_eq!(
            states.commit(OWNER, 2, b"third state"),
            Err(OwnerStateStoreError::Unavailable),
            "{name}"
        );
    }
}

#[test]
fn every_state_has_a_content_free_export_family_and_a_tampered_state_is_not_exported() {
    let fixture = Fixture::new(67);
    drop(populated(&fixture));
    let store = fixture.open().unwrap();
    let destination = fixture.directory().join("export.jsonl");
    store
        .export_json_lines(&destination, &observation())
        .expect("derived export");
    let exported = std::fs::read_to_string(&destination).unwrap();
    assert!(exported.contains("\"owner_states\""));
    assert!(!exported.contains("second state"));
    store
        .connection
        .execute_batch(
            "DROP TRIGGER owner_state_forward_only;
             UPDATE owner_states SET state = CAST('changed state' AS BLOB);",
        )
        .unwrap();
    assert_eq!(
        store
            .export_json_lines(&fixture.directory().join("second.jsonl"), &observation())
            .err(),
        Some(OperationalStoreError::IntegrityFailure)
    );
}
