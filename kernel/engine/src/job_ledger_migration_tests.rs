// Real encrypted schema 20 fixtures upgraded to the job ledger schema
// (Decision 0118); no installed-upgrade or old-binary qualification.
use super::*;

fn version_twenty(path: &Path, key: &[u8; 32]) {
    version_nineteen(path, key);
    let connection = open_connection(path, key).unwrap();
    let sql = crate::operational_store::MIGRATION_20_SCHEMA_SQL;
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute_batch(sql).unwrap();
    transaction
        .execute(
            "INSERT INTO schema_history(version, migration_sha256) VALUES (20, ?1)",
            [sha256_hex(sql.as_bytes())],
        )
        .unwrap();
    transaction
        .pragma_update(None, "user_version", 20_i64)
        .unwrap();
    transaction.commit().unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-20.json")).unwrap();
    assert_eq!(tables(&connection), fixture["tables"]);
    assert_eq!(history(&connection), fixture["migrations"]);
}

fn job_ledger_tables(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema
             WHERE type = 'table' AND name LIKE 'job_ledger_%'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn version_twenty_upgrades_to_job_ledgers_preserving_records_and_history() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [98; 32];
    version_twenty(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let original_history = history(&connection);
    let original_record = legacy_record(&connection);
    drop(connection);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    let version: i64 = store
        .connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 21);
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-21.json")).unwrap();
    assert_eq!(tables(&store.connection), fixture["tables"]);
    assert_eq!(history(&store.connection), fixture["migrations"]);
    let migrated = history(&store.connection);
    assert_eq!(
        &migrated.as_array().unwrap()[..20],
        original_history.as_array().unwrap()
    );
    assert_eq!(legacy_record(&store.connection), original_record);
    let ledgers: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM job_ledger_roots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(ledgers, 0);
    drop(store);
    drop(OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_version_twenty_one_migration_rolls_back_and_stays_retryable() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [99; 32];
    version_twenty(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let before = history(&connection);
    let record = legacy_record(&connection);
    // A conflicting table makes the job ledger migration fail part way.
    connection
        .execute_batch("CREATE TABLE job_ledger_heads(incompatible INTEGER) STRICT;")
        .unwrap();
    drop(connection);
    assert_eq!(
        OperationalStore::open(&path, &observation(), &mut TestKey(key)).err(),
        Some(OperationalStoreError::MigrationFailed)
    );
    let connection = open_connection(&path, &key).unwrap();
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 20);
    assert_eq!(history(&connection), before);
    assert_eq!(legacy_record(&connection), record);
    // Only the conflicting table exists; no root or entry table was kept.
    assert_eq!(job_ledger_tables(&connection), 1);
    connection
        .execute_batch("DROP TABLE job_ledger_heads;")
        .unwrap();
    drop(connection);
    let current = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    assert_eq!(history(&current.connection).as_array().unwrap().len(), 21);
    assert_eq!(job_ledger_tables(&current.connection), 3);
    assert_eq!(legacy_record(&current.connection), record);
    drop(current);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn version_twenty_corrupt_history_cannot_add_job_ledgers() {
    for mutation in 0..3 {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let key = [100; 32];
        version_twenty(&path, &key);
        let connection = open_connection(&path, &key).unwrap();
        match mutation {
            0 => {
                connection
                    .execute(
                        "UPDATE schema_history SET migration_sha256 = ?1 WHERE version = 20",
                        ["0".repeat(64)],
                    )
                    .unwrap();
            }
            1 => {
                connection
                    .execute("DELETE FROM schema_history WHERE version = 19", [])
                    .unwrap();
            }
            _ => {
                connection
                    .execute("INSERT INTO schema_history VALUES (21, ?1)", ["0".repeat(64)])
                    .unwrap();
            }
        }
        let before = history(&connection);
        let record = legacy_record(&connection);
        drop(connection);
        assert_eq!(
            OperationalStore::open(&path, &observation(), &mut TestKey(key)).err(),
            Some(OperationalStoreError::MigrationFailed)
        );
        let connection = open_connection(&path, &key).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 20);
        assert_eq!(history(&connection), before);
        assert_eq!(legacy_record(&connection), record);
        assert_eq!(job_ledger_tables(&connection), 0);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }
}
