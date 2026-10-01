// Real encrypted schema fixtures; no installed-upgrade or old-binary qualification.
use super::*;

fn version_nineteen(path: &Path, key: &[u8; 32]) {
    version_eighteen(path, key);
    let connection = open_connection(path, key).unwrap();
    let sql = crate::operational_store::MIGRATION_19_SCHEMA_SQL;
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute_batch(sql).unwrap();
    transaction.execute(
        "INSERT INTO schema_history(version, migration_sha256) VALUES (19, ?1)",
        [sha256_hex(sql.as_bytes())],
    ).unwrap();
    transaction.pragma_update(None, "user_version", 19_i64).unwrap();
    transaction.commit().unwrap();
    let fixture: Value = serde_json::from_str(include_str!("../fixtures/operational-store/schema-19.json")).unwrap();
    assert_eq!(tables(&connection), fixture["tables"]);
    let history = history(&connection);
    assert_eq!(history, fixture["migrations"]);
}

fn tables(connection: &rusqlite::Connection) -> Value {
    let names: Vec<String> = connection.prepare(
        "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    ).unwrap().query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap();
    serde_json::to_value(names).unwrap()
}

fn history(connection: &rusqlite::Connection) -> Value {
    let entries: Vec<(i64, String)> = connection.prepare(
        "SELECT version, migration_sha256 FROM schema_history ORDER BY version",
    ).unwrap().query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap()
        .collect::<Result<_, _>>().unwrap();
    Value::Array(entries.into_iter().map(|(version, sha256)|
        serde_json::json!({"version":version, "sha256":sha256})).collect())
}

fn legacy_record(connection: &rusqlite::Connection) -> (String, Vec<u8>) {
    connection.query_row(
        "SELECT record_sha256, record_json FROM sessions WHERE session_id = 'legacy-research-owner'",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap()
}

#[test]
fn version_nineteen_reader_epoch_preserves_tables_records_and_history() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [93; 32];
    version_nineteen(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let original_tables = tables(&connection);
    let original_history = history(&connection);
    let original_record = legacy_record(&connection);
    drop(connection);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    let version: i64 = store.connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    // The reader epoch adds no table; the later job ledger, run action
    // history and documentation pack migrations add only their own tables
    // (Decisions 0118 and 0129 to 0131).
    let mut expected_tables: Vec<String> = serde_json::from_value(original_tables).unwrap();
    expected_tables.extend(["job_ledger_entries", "job_ledger_heads", "job_ledger_roots"].map(str::to_owned));
    expected_tables.extend([
        "run_action_history_heads",
        "run_action_history_records",
        "run_action_history_roots",
    ].map(str::to_owned));
    expected_tables.extend([
        "doc_pack_catalog_heads",
        "doc_pack_deletions",
        "doc_pack_files",
        "doc_pack_versions",
    ].map(str::to_owned));
    expected_tables.push("owner_states".to_owned());
    expected_tables.sort();
    assert_eq!(tables(&store.connection), serde_json::to_value(expected_tables).unwrap());
    assert_eq!(legacy_record(&store.connection), original_record);
    let migrated = history(&store.connection);
    assert_eq!(&migrated.as_array().unwrap()[..19], original_history.as_array().unwrap());
    assert_eq!(migrated[19], serde_json::json!({"version":20,
        "sha256":sha256_hex(crate::operational_store::MIGRATION_20_SCHEMA_SQL.as_bytes())}));
    assert_eq!(migrated[20], serde_json::json!({"version":21,
        "sha256":sha256_hex(crate::operational_store::MIGRATION_21_SCHEMA_SQL.as_bytes())}));
    assert_eq!(migrated[21], serde_json::json!({"version":22,
        "sha256":sha256_hex(crate::operational_store::MIGRATION_22_SCHEMA_SQL.as_bytes())}));
    assert_eq!(migrated[22], serde_json::json!({"version":23,
        "sha256":sha256_hex(crate::operational_store::MIGRATION_23_SCHEMA_SQL.as_bytes())}));
    assert_eq!(migrated[23], serde_json::json!({"version":24,
        "sha256":sha256_hex(crate::operational_store::MIGRATION_24_SCHEMA_SQL.as_bytes())}));
    drop(store);
    drop(OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn version_nineteen_corrupt_history_cannot_advance_reader_epoch() {
    for mutation in 0..3 {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let key = [94; 32];
        version_nineteen(&path, &key);
        let connection = open_connection(&path, &key).unwrap();
        match mutation {
            0 => { connection.execute("UPDATE schema_history SET migration_sha256 = ?1 WHERE version = 19", ["0".repeat(64)]).unwrap(); }
            1 => { connection.execute("DELETE FROM schema_history WHERE version = 18", []).unwrap(); }
            _ => { connection.execute("INSERT INTO schema_history VALUES (20, ?1)", ["0".repeat(64)]).unwrap(); }
        }
        let before = history(&connection);
        let record = legacy_record(&connection);
        drop(connection);
        assert_eq!(OperationalStore::open(&path, &observation(), &mut TestKey(key)).err(),
            Some(OperationalStoreError::MigrationFailed));
        let connection = open_connection(&path, &key).unwrap();
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        assert_eq!(version, 19);
        assert_eq!(history(&connection), before);
        assert_eq!(legacy_record(&connection), record);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn failed_version_twenty_reader_epoch_keeps_version_nineteen_retryable() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [95; 32];
    version_nineteen(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let before = history(&connection);
    let record = legacy_record(&connection);
    connection.execute_batch("CREATE TRIGGER reject_reader_epoch BEFORE INSERT ON schema_history
        WHEN NEW.version = 20 BEGIN SELECT RAISE(ABORT, 'synthetic epoch failure'); END;").unwrap();
    drop(connection);
    assert_eq!(OperationalStore::open(&path, &observation(), &mut TestKey(key)).err(),
        Some(OperationalStoreError::MigrationFailed));
    let connection = open_connection(&path, &key).unwrap();
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    assert_eq!(version, 19);
    assert_eq!(history(&connection), before);
    assert_eq!(legacy_record(&connection), record);
    connection.execute_batch("DROP TRIGGER reject_reader_epoch;").unwrap();
    drop(connection);
    let current = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    assert_eq!(history(&current.connection).as_array().unwrap().len(), 24);
    assert_eq!(legacy_record(&current.connection), record);
    drop(current);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn version_nineteen_backup_refusal_preserves_source_without_creating_candidate() {
    let directory = temporary_directory();
    let backup = directory.join("legacy.backup.db");
    let candidate = directory.join("candidate.db");
    let key = [96; 32];
    version_nineteen(&backup, &key);
    let before = fs::read(&backup).unwrap();
    assert_eq!(
        OperationalStore::restore_to_fresh_candidate(
            &backup, &observation(), &mut TestKey(key),
            &candidate, &observation(), &mut TestKey([97; 32]),
        ).unwrap_err(),
        OperationalStoreError::RestoreFailure
    );
    assert_eq!(fs::read(&backup).unwrap(), before);
    let files: Vec<_> = fs::read_dir(&directory).unwrap()
        .map(|entry| entry.unwrap().path()).collect();
    assert_eq!(files, vec![backup]);
    fs::remove_dir_all(directory).unwrap();
}

mod job_ledger_migration {
    include!("job_ledger_migration_tests.rs");
}
