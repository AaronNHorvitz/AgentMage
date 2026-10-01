// Real encrypted operational stores in private temporary directories; no
// process, transport or model effects.
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use agentmage_kernel_contracts::{StorageFilesystemClass, StrictLocalStorageObservation};

use super::*;
use crate::operational_store::{
    DurableAuthorityRuntime, OperationalStoreError, OperationalStoreKeyError,
    OperationalStoreKeyProvider,
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
            "agentmage-doc-packs-{}-{sequence}",
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

    fn catalog(&self) -> DurableDocPackCatalog {
        DurableDocPackCatalog::new(Arc::new(Mutex::new(self.open().expect("store opens"))))
    }

    /// Runs raw SQL as a key holder that bypasses the owner's rules.
    fn tamper(&self, sql: &str) {
        let store = self.open().expect("store opens for tampering");
        store.connection.execute_batch(sql).expect("tampering SQL");
    }

    fn count(&self, table: &str) -> i64 {
        let store = self.open().expect("store opens");
        store
            .connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("row count")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn key(pack_id: &str, major: u32) -> DocPackVersionKey {
    DocPackVersionKey {
        pack_id: pack_id.to_owned(),
        major,
        minor: 0,
        patch: 0,
    }
}

fn file(path: &str, text: &str) -> StoredDocPackFile {
    StoredDocPackFile {
        path: path.to_owned(),
        sha256: sha256_hex(text.as_bytes()),
        content: text.as_bytes().to_vec(),
    }
}

/// A version whose manifest bytes are opaque to the store.
fn version(pack_id: &str, major: u32, superseded_on: Option<&str>) -> StoredDocPackVersion {
    let manifest_json = format!("{{\"pack\":\"{pack_id}\",\"major\":{major}}}").into_bytes();
    StoredDocPackVersion {
        key: key(pack_id, major),
        manifest_sha256: sha256_hex(&manifest_json),
        manifest_json,
        superseded_on: superseded_on.map(str::to_owned),
        files: vec![
            file("guide/cache.md", &format!("# Cache {major}\n")),
            file("notes.txt", "Offline notes.\n"),
        ],
    }
}

fn deletion(
    removed: &StoredDocPackVersion,
    deleted_on: &str,
    reason: StoredDocPackDeletionReason,
) -> StoredDocPackDeletion {
    StoredDocPackDeletion {
        key: removed.key.clone(),
        manifest_sha256: removed.manifest_sha256.clone(),
        deleted_on: deleted_on.to_owned(),
        reason,
    }
}

fn first() -> DocPackCatalogContents {
    DocPackCatalogContents {
        versions: vec![version("guide", 1, None)],
        deletions: Vec::new(),
        last_changed_on: Some("2026-01-02".to_owned()),
    }
}

/// A refresh supersedes version 1, and a second pack is imported.
fn second() -> DocPackCatalogContents {
    DocPackCatalogContents {
        versions: vec![
            version("guide", 1, Some("2026-02-01")),
            version("guide", 2, None),
            version("linker", 1, None),
        ],
        deletions: Vec::new(),
        last_changed_on: Some("2026-02-01".to_owned()),
    }
}

/// Retention deletes the superseded version.
fn third() -> DocPackCatalogContents {
    let mut contents = second();
    let removed = contents.versions.remove(0);
    contents.deletions.push(deletion(
        &removed,
        "2026-03-03",
        StoredDocPackDeletionReason::Retention,
    ));
    contents.last_changed_on = Some("2026-03-03".to_owned());
    contents
}

/// The second pack's version is superseded too, so a kept version of each
/// state remains.
fn fourth() -> DocPackCatalogContents {
    let mut contents = third();
    contents.versions[1].superseded_on = Some("2026-03-04".to_owned());
    contents.last_changed_on = Some("2026-03-04".to_owned());
    contents
}

/// Commits the four changes and returns the handle at revision 4.
fn populated(fixture: &Fixture) -> DurableDocPackCatalog {
    let catalog = fixture.catalog();
    catalog.commit(0, &first()).unwrap();
    catalog.commit(1, &second()).unwrap();
    catalog.commit(2, &third()).unwrap();
    catalog.commit(3, &fourth()).unwrap();
    catalog
}

#[test]
fn an_empty_catalog_loads_at_revision_zero_until_its_first_commit() {
    let fixture = Fixture::new(41);
    let catalog = fixture.catalog();
    let heads = |catalog: &DurableDocPackCatalog| {
        catalog
            .with_store(|store| {
                store
                    .connection
                    .query_row("SELECT COUNT(*) FROM doc_pack_catalog_heads", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(|_| DocPackStoreError::Storage)
            })
            .unwrap()
    };
    let empty = catalog.load().unwrap();
    assert_eq!(empty.revision, 0);
    assert_eq!(empty.contents, DocPackCatalogContents::default());
    assert_eq!(empty.state_sha256.len(), 64);
    assert_eq!(heads(&catalog), 0);
    // Committing the empty state again writes nothing.
    assert_eq!(
        catalog
            .commit(0, &DocPackCatalogContents::default())
            .unwrap(),
        empty
    );
    assert_eq!(heads(&catalog), 0);
    let stored = catalog.commit(0, &first()).unwrap();
    assert_eq!(stored.revision, 1);
    assert_eq!(stored.contents, first());
    assert_ne!(stored.state_sha256, empty.state_sha256);
    assert_eq!(heads(&catalog), 1);
}

#[test]
fn each_change_persists_and_reloads_after_reopening() {
    let fixture = Fixture::new(42);
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 1)
            .unwrap();
    let catalog = runtime.doc_pack_catalog();
    let clone = catalog.clone();
    assert_eq!(catalog.commit(0, &first()).unwrap().revision, 1);
    assert_eq!(clone.commit(1, &second()).unwrap().revision, 2);
    let stored = catalog.commit(2, &third()).unwrap();
    assert_eq!(stored.revision, 3);
    assert_eq!(stored.contents, third());
    // Only the dated head moves when nothing else changes.
    let mut later = third();
    later.last_changed_on = Some("2026-03-04".to_owned());
    let moved = catalog.commit(3, &later).unwrap();
    assert_eq!(moved.revision, 4);
    assert_eq!(moved.contents, later);
    drop((catalog, clone, runtime));
    // The deleted version's files went with it; its record stays.
    assert_eq!(fixture.count("doc_pack_versions"), 2);
    assert_eq!(fixture.count("doc_pack_files"), 4);
    assert_eq!(fixture.count("doc_pack_deletions"), 1);
    // Opening the runtime checks the catalog before it is usable.
    let runtime =
        DurableAuthorityRuntime::open(&fixture.path, &observation(), &mut TestKey(fixture.key), 2)
            .unwrap();
    let reloaded = runtime.doc_pack_catalog().load().unwrap();
    assert_eq!(reloaded, moved);
    // A version deleted earlier may be added again; removing it again needs
    // a later deletion of its own.
    let mut readded = later;
    readded.versions.insert(0, version("guide", 1, None));
    readded.last_changed_on = Some("2026-03-05".to_owned());
    assert_eq!(
        runtime
            .doc_pack_catalog()
            .commit(4, &readded)
            .unwrap()
            .revision,
        5
    );
    drop(runtime);
    let store = fixture.open().unwrap();
    assert!(
        store
            .connection
            .execute_batch("DELETE FROM doc_pack_files WHERE pack_id = 'guide' AND major = 1")
            .is_err()
    );
}

#[test]
fn a_version_goes_only_with_a_later_deletion_naming_its_manifest() {
    let fixture = Fixture::new(50);
    drop(populated(&fixture));
    let mut store = fixture.open().unwrap();
    let linker = version("linker", 1, Some("2026-03-04"));
    let insert = |digest: &str| {
        format!(
            "INSERT INTO doc_pack_deletions(sequence, pack_id, major, minor, patch,
                 manifest_sha256, deleted_on, reason)
             VALUES (2, 'linker', 1, 0, 0, '{digest}', '2026-03-05', 'person')"
        )
    };
    for (digest, allowed) in [
        ("c".repeat(64), false),
        (linker.manifest_sha256.clone(), true),
    ] {
        let transaction = store.connection.transaction().unwrap();
        transaction.execute_batch(&insert(&digest)).unwrap();
        let files =
            transaction.execute_batch("DELETE FROM doc_pack_files WHERE pack_id = 'linker'");
        let version =
            transaction.execute_batch("DELETE FROM doc_pack_versions WHERE pack_id = 'linker'");
        assert_eq!(
            (files.is_ok(), version.is_ok()),
            (allowed, allowed),
            "{digest}"
        );
        // Rolled back: the owner's commit is the only path that keeps it.
        drop(transaction);
    }
    // A version cannot go while its files remain, even with its deletion.
    let transaction = store.connection.transaction().unwrap();
    transaction
        .execute_batch(&insert(&linker.manifest_sha256))
        .unwrap();
    assert!(
        transaction
            .execute_batch("DELETE FROM doc_pack_versions WHERE pack_id = 'linker'")
            .is_err()
    );
    drop(transaction);
    drop(store);
    assert_eq!(fixture.catalog().load().unwrap().contents, fourth());
}

#[test]
fn a_commit_that_names_another_revision_writes_nothing() {
    let fixture = Fixture::new(43);
    let catalog = fixture.catalog();
    catalog.commit(0, &first()).unwrap();
    for read in [0, 2, 7] {
        assert_eq!(
            catalog.commit(read, &second()),
            Err(DocPackStoreError::Stale),
            "{read}"
        );
    }
    let stored = catalog.load().unwrap();
    assert_eq!((stored.revision, stored.contents), (1, first()));
}

#[test]
fn only_changes_the_catalogs_own_operations_make_are_committed() {
    type Change = Box<dyn Fn(&mut DocPackCatalogContents)>;
    let fixture = Fixture::new(44);
    let catalog = fixture.catalog();
    catalog.commit(0, &first()).unwrap();
    catalog.commit(1, &second()).unwrap();
    let base = third();
    let cases: Vec<(&str, Change)> = vec![
        (
            "a kept manifest changes",
            Box::new(|next| {
                next.versions[0].manifest_json = b"{\"changed\":true}".to_vec();
            }),
        ),
        (
            "a kept manifest digest changes",
            Box::new(|next| next.versions[0].manifest_sha256 = "a".repeat(64)),
        ),
        (
            "a kept file changes",
            Box::new(|next| next.versions[0].files[1] = file("notes.txt", "Changed.\n")),
        ),
        (
            "a kept version gains a file",
            Box::new(|next| next.versions[0].files.push(file("zz.md", "More.\n"))),
        ),
        (
            "a superseded version becomes current",
            Box::new(|next| {
                next.versions.insert(0, version("guide", 1, None));
                next.deletions.clear();
            }),
        ),
        (
            "a supersession day changes",
            Box::new(|next| {
                next.versions
                    .insert(0, version("guide", 1, Some("2026-02-02")));
                next.deletions.clear();
            }),
        ),
        (
            "a version is removed without its deletion",
            Box::new(|next| next.deletions.clear()),
        ),
        (
            "a deletion names another manifest",
            Box::new(|next| next.deletions[0].manifest_sha256 = "b".repeat(64)),
        ),
        (
            "a deletion names a version that stays",
            Box::new(|next| {
                let kept = next.versions[0].clone();
                next.deletions.push(deletion(
                    &kept,
                    "2026-03-03",
                    StoredDocPackDeletionReason::Person,
                ));
            }),
        ),
        (
            "a deletion is dated after the change",
            Box::new(|next| next.deletions[0].deleted_on = "2026-03-04".to_owned()),
        ),
        (
            "a deletion is dated before the last change",
            Box::new(|next| next.deletions[0].deleted_on = "2026-01-31".to_owned()),
        ),
        (
            "the day goes backwards",
            Box::new(|next| next.last_changed_on = Some("2026-01-31".to_owned())),
        ),
        (
            "a version is added already superseded",
            Box::new(|next| next.versions.push(version("tools", 1, Some("2026-03-03")))),
        ),
        (
            "a current version is superseded after the change",
            Box::new(|next| next.versions[0].superseded_on = Some("2026-03-04".to_owned())),
        ),
        (
            "a current version is superseded before the last change",
            Box::new(|next| next.versions[0].superseded_on = Some("2026-01-31".to_owned())),
        ),
    ];
    for (name, change) in cases {
        let mut next = base.clone();
        change(&mut next);
        assert_eq!(
            catalog.commit(2, &next),
            Err(DocPackStoreError::InvalidChange),
            "{name}"
        );
        let stored = catalog.load().unwrap();
        assert_eq!((stored.revision, stored.contents), (2, second()), "{name}");
    }
    // The deletion history is append-only.
    catalog.commit(2, &base).unwrap();
    let mut fourth = base.clone();
    let removed = fourth.versions.remove(0);
    fourth.deletions.push(deletion(
        &removed,
        "2026-03-05",
        StoredDocPackDeletionReason::Person,
    ));
    fourth.last_changed_on = Some("2026-03-05".to_owned());
    let edited: Vec<(&str, Change)> = vec![
        (
            "an earlier deletion changes",
            Box::new(|next| next.deletions[0].reason = StoredDocPackDeletionReason::Person),
        ),
        (
            "an earlier deletion is dropped",
            Box::new(|next| {
                next.deletions.remove(0);
            }),
        ),
        (
            "deletions are reordered",
            Box::new(|next| next.deletions.swap(0, 1)),
        ),
        (
            "the day is cleared",
            Box::new(|next| {
                next.last_changed_on = None;
                next.versions.clear();
                next.deletions.clear();
            }),
        ),
    ];
    for (name, change) in edited {
        let mut next = fourth.clone();
        change(&mut next);
        assert!(
            matches!(
                catalog.commit(3, &next),
                Err(DocPackStoreError::InvalidChange | DocPackStoreError::InvalidInput)
            ),
            "{name}"
        );
        assert_eq!(catalog.load().unwrap().revision, 3, "{name}");
    }
    assert_eq!(catalog.commit(3, &fourth).unwrap().contents, fourth);
}

#[test]
fn malformed_or_oversized_states_are_refused_before_the_store() {
    type Change = Box<dyn Fn(&mut DocPackCatalogContents)>;
    let fixture = Fixture::new(45);
    let catalog = fixture.catalog();
    let invalid: Vec<(&str, Change)> = vec![
        (
            "versions out of order",
            Box::new(|next| next.versions.swap(0, 1)),
        ),
        (
            "a duplicate version",
            Box::new(|next| {
                let copy = next.versions[1].clone();
                next.versions.insert(1, copy);
            }),
        ),
        (
            "an uppercase pack",
            Box::new(|next| next.versions[2].key.pack_id = "Linker".to_owned()),
        ),
        (
            "an empty pack",
            Box::new(|next| next.versions[2].key.pack_id = String::new()),
        ),
        (
            "a malformed manifest digest",
            Box::new(|next| next.versions[0].manifest_sha256 = "x".repeat(64)),
        ),
        (
            "an empty manifest",
            Box::new(|next| next.versions[0].manifest_json.clear()),
        ),
        (
            "a malformed supersession day",
            Box::new(|next| next.versions[0].superseded_on = Some("2026-2-01".to_owned())),
        ),
        (
            "a version without files",
            Box::new(|next| next.versions[0].files.clear()),
        ),
        (
            "a file whose bytes differ from its digest",
            Box::new(|next| next.versions[0].files[0].content = b"other".to_vec()),
        ),
        (
            "an empty file",
            Box::new(|next| {
                next.versions[0].files[0].content.clear();
                next.versions[0].files[0].sha256 = sha256_hex(b"");
            }),
        ),
        (
            "files out of order",
            Box::new(|next| next.versions[0].files.swap(0, 1)),
        ),
        (
            "a traversing path",
            Box::new(|next| next.versions[0].files[0].path = "../escape.md".to_owned()),
        ),
        (
            "an absolute path",
            Box::new(|next| next.versions[0].files[0].path = "/guide.md".to_owned()),
        ),
        (
            "a malformed last change",
            Box::new(|next| next.last_changed_on = Some("2026-02-1".to_owned())),
        ),
        (
            "kept versions without a last change",
            Box::new(|next| next.last_changed_on = None),
        ),
        (
            "a malformed deletion",
            Box::new(|next| {
                let removed = version("old", 1, None);
                next.deletions.push(StoredDocPackDeletion {
                    deleted_on: "yesterday".to_owned(),
                    ..deletion(&removed, "2026-02-01", StoredDocPackDeletionReason::Person)
                });
            }),
        ),
    ];
    for (name, change) in invalid {
        let mut next = second();
        change(&mut next);
        assert_eq!(
            catalog.commit(0, &next),
            Err(DocPackStoreError::InvalidInput),
            "{name}"
        );
    }
    let mut large = first();
    large.versions[0].manifest_json = vec![b' '; MAX_DOC_PACK_MANIFEST_BYTES + 1];
    assert_eq!(
        catalog.commit(0, &large),
        Err(DocPackStoreError::ResourceLimit)
    );
    let mut large = first();
    let bytes = vec![b'a'; MAX_DOC_PACK_FILE_BYTES + 1];
    large.versions[0].files[0] = StoredDocPackFile {
        path: "guide/cache.md".to_owned(),
        sha256: sha256_hex(&bytes),
        content: bytes,
    };
    assert_eq!(
        catalog.commit(0, &large),
        Err(DocPackStoreError::ResourceLimit)
    );
    let mut many = first();
    many.versions = (0..=MAX_DOC_PACK_STORED_VERSIONS as u32)
        .map(|major| version("guide", major, None))
        .collect();
    assert_eq!(
        catalog.commit(0, &many),
        Err(DocPackStoreError::ResourceLimit)
    );
    assert_eq!(catalog.load().unwrap().revision, 0);
}

#[test]
fn a_change_commits_together_or_not_at_all() {
    for (trigger, table) in [
        ("reject_head", "UPDATE ON doc_pack_catalog_heads"),
        ("reject_file", "INSERT ON doc_pack_files"),
        ("reject_deletion", "INSERT ON doc_pack_deletions"),
    ] {
        let fixture = Fixture::new(46);
        let catalog = fixture.catalog();
        catalog.commit(0, &first()).unwrap();
        catalog.commit(1, &second()).unwrap();
        drop(catalog);
        fixture.tamper(&format!(
            "CREATE TRIGGER {trigger} BEFORE {table}
             BEGIN SELECT RAISE(ABORT, 'synthetic commit failure'); END;"
        ));
        let catalog = fixture.catalog();
        let mut next = third();
        next.versions.push(version("tools", 1, None));
        assert_eq!(
            catalog.commit(2, &next),
            Err(DocPackStoreError::Storage),
            "{trigger}"
        );
        // Nothing was kept, and the store stays usable.
        let stored = catalog.load().unwrap();
        assert_eq!((stored.revision, stored.contents), (2, second()));
        drop(catalog);
        assert_eq!(fixture.count("doc_pack_deletions"), 0);
        assert_eq!(fixture.count("doc_pack_files"), 6);
        fixture.tamper(&format!("DROP TRIGGER {trigger};"));
        assert_eq!(fixture.catalog().commit(2, &next).unwrap().contents, next);
    }
}

#[test]
fn retained_rows_follow_the_catalog_transitions() {
    let fixture = Fixture::new(47);
    drop(populated(&fixture));
    let store = fixture.open().unwrap();
    for sql in [
        // A kept version may only become superseded, once.
        "UPDATE doc_pack_versions SET manifest_json = X'7B7D' WHERE pack_id = 'guide'",
        "UPDATE doc_pack_versions SET manifest_sha256 = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' WHERE pack_id = 'guide'",
        "UPDATE doc_pack_versions SET major = 9 WHERE pack_id = 'guide'",
        "UPDATE doc_pack_versions SET deletions_before = 1 WHERE pack_id = 'guide'",
        "UPDATE doc_pack_versions SET superseded_on = NULL WHERE pack_id = 'linker'",
        "UPDATE doc_pack_versions SET superseded_on = '2026-03-05' WHERE pack_id = 'linker'",
        // Versions and files go only with a later deletion naming them.
        "DELETE FROM doc_pack_files WHERE pack_id = 'linker'",
        "DELETE FROM doc_pack_versions WHERE pack_id = 'linker'",
        // A file never changes.
        "UPDATE doc_pack_files SET content = X'78' WHERE path = 'notes.txt'",
        "UPDATE doc_pack_files SET path = 'other.txt' WHERE path = 'notes.txt'",
        // A version is inserted only after every deletion so far.
        "INSERT INTO doc_pack_versions(pack_id, major, minor, patch, manifest_sha256, manifest_json, superseded_on, deletions_before)
             VALUES ('tools', 1, 0, 0, 'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd', X'7B7D', NULL, 0)",
        // Deletions are append-only, in sequence.
        "UPDATE doc_pack_deletions SET reason = 'person'",
        "DELETE FROM doc_pack_deletions",
        "INSERT INTO doc_pack_deletions(sequence, pack_id, major, minor, patch, manifest_sha256, deleted_on, reason)
             VALUES (3, 'guide', 9, 0, 0, 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee', '2026-03-03', 'person')",
        // The head only moves forward.
        "DELETE FROM doc_pack_catalog_heads",
        "UPDATE doc_pack_catalog_heads SET revision = revision + 2",
        "UPDATE doc_pack_catalog_heads SET revision = revision - 1",
        "UPDATE doc_pack_catalog_heads SET catalog_id = 'other', revision = revision + 1",
        "UPDATE doc_pack_catalog_heads SET revision = revision + 1, last_changed_on = '2026-03-03'",
        "UPDATE doc_pack_catalog_heads SET revision = revision + 1, last_changed_on = NULL",
        "INSERT INTO doc_pack_catalog_heads(catalog_id, revision, last_changed_on, state_sha256)
             VALUES ('other', 1, NULL, 'ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff')",
        // REPLACE removes the conflicting row, which the delete triggers
        // refuse because the store turns recursive triggers on.
        "REPLACE INTO doc_pack_catalog_heads(catalog_id, revision, last_changed_on, state_sha256)
             SELECT catalog_id, 1, NULL, state_sha256 FROM doc_pack_catalog_heads",
        "REPLACE INTO doc_pack_deletions(sequence, pack_id, major, minor, patch, manifest_sha256, deleted_on, reason)
             SELECT sequence, pack_id, major, minor, patch, manifest_sha256, deleted_on, 'person'
             FROM doc_pack_deletions",
        "REPLACE INTO doc_pack_files(pack_id, major, minor, patch, path, sha256, content)
             SELECT pack_id, major, minor, patch, path, sha256, X'78' FROM doc_pack_files
             WHERE path = 'notes.txt'",
    ] {
        assert!(store.connection.execute_batch(sql).is_err(), "{sql}");
    }
    drop(store);
    assert_eq!(fixture.catalog().load().unwrap().contents, fourth());
}

#[test]
fn a_tampered_catalog_is_refused_poisons_the_store_and_blocks_reopening() {
    // Each case bypasses the owner's triggers or foreign keys, as only a key
    // holder outside the owner could. Metadata changes fail every open; a
    // changed file body fails the load that reads it.
    let cases: [(&str, &str, bool); 10] = [
        (
            "changed manifest",
            "DROP TRIGGER doc_pack_version_supersede_only;
             UPDATE doc_pack_versions SET manifest_json = X'7B7D' WHERE pack_id = 'guide';",
            true,
        ),
        (
            "un-superseded version",
            "DROP TRIGGER doc_pack_version_supersede_only;
             UPDATE doc_pack_versions SET superseded_on = NULL;",
            true,
        ),
        (
            "removed version",
            "PRAGMA foreign_keys = OFF;
             DROP TRIGGER doc_pack_version_delete_requires_record;
             DROP TRIGGER doc_pack_file_delete_requires_record;
             DELETE FROM doc_pack_files WHERE pack_id = 'linker';
             DELETE FROM doc_pack_versions WHERE pack_id = 'linker';",
            true,
        ),
        (
            "removed file",
            "DROP TRIGGER doc_pack_file_delete_requires_record;
             DELETE FROM doc_pack_files WHERE path = 'notes.txt' AND pack_id = 'linker';",
            true,
        ),
        (
            "changed file digest",
            "DROP TRIGGER doc_pack_file_update_forbidden;
             UPDATE doc_pack_files SET sha256 = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
             WHERE path = 'notes.txt' AND pack_id = 'linker';",
            true,
        ),
        (
            "edited deletion",
            "DROP TRIGGER doc_pack_deletion_update_forbidden;
             UPDATE doc_pack_deletions SET reason = 'person';",
            true,
        ),
        (
            "removed deletion",
            "DROP TRIGGER doc_pack_deletion_delete_forbidden;
             DELETE FROM doc_pack_deletions;",
            true,
        ),
        (
            "rolled-back head",
            "DROP TRIGGER doc_pack_head_forward_only;
             UPDATE doc_pack_catalog_heads SET revision = 2;",
            true,
        ),
        (
            "missing head",
            "DROP TRIGGER doc_pack_head_delete_forbidden;
             DELETE FROM doc_pack_catalog_heads;",
            true,
        ),
        (
            "changed file body",
            "DROP TRIGGER doc_pack_file_update_forbidden;
             UPDATE doc_pack_files SET content = CAST('Offline notez.' || char(10) AS BLOB)
             WHERE path = 'notes.txt' AND pack_id = 'linker';",
            false,
        ),
    ];
    for (name, sql, refused_at_open) in cases {
        let fixture = Fixture::new(48);
        drop(populated(&fixture));
        fixture.tamper(sql);
        if refused_at_open {
            assert_eq!(
                fixture.open().err(),
                Some(OperationalStoreError::IntegrityFailure),
                "{name}"
            );
        } else {
            assert_eq!(
                fixture.catalog().load(),
                Err(DocPackStoreError::Integrity),
                "{name}"
            );
        }
        // A handle over a store opened before the change refuses it too, and
        // the store stays poisoned.
        let fixture = Fixture::new(49);
        let catalog = populated(&fixture);
        catalog
            .with_store(|store| {
                store
                    .connection
                    .execute_batch(sql)
                    .map_err(|_| DocPackStoreError::Storage)
            })
            .unwrap();
        assert_eq!(catalog.load(), Err(DocPackStoreError::Integrity), "{name}");
        assert_eq!(
            catalog.load(),
            Err(DocPackStoreError::Unavailable),
            "{name}"
        );
        assert_eq!(
            catalog.commit(4, &fourth()),
            Err(DocPackStoreError::Unavailable),
            "{name}"
        );
    }
}

#[test]
fn every_catalog_row_has_a_content_free_export_family_and_a_tampered_catalog_is_not_exported() {
    let fixture = Fixture::new(51);
    drop(populated(&fixture));
    let store = fixture.open().unwrap();
    let destination = fixture.directory.join("export.jsonl");
    store
        .export_json_lines(&destination, &observation())
        .expect("derived export");
    let exported = std::fs::read_to_string(&destination).unwrap();
    for family in [
        "doc_pack_catalog_heads",
        "doc_pack_versions",
        "doc_pack_deletions",
    ] {
        assert!(exported.contains(&format!("\"{family}\"")), "{family}");
    }
    // Identities and digests only: no file path, text or manifest.
    for content in ["Offline notes", "guide/cache.md", "notes.txt", "\"pack\""] {
        assert!(!exported.contains(content), "{content}");
    }
    // A catalog changed underneath the open store is not exported.
    store
        .connection
        .execute_batch(
            "DROP TRIGGER doc_pack_deletion_update_forbidden;
             UPDATE doc_pack_deletions SET reason = 'person';",
        )
        .unwrap();
    assert_eq!(
        store
            .export_json_lines(&fixture.directory.join("second.jsonl"), &observation())
            .err(),
        Some(OperationalStoreError::IntegrityFailure)
    );
}
