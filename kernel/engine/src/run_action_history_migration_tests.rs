// Real encrypted schema 21 fixtures upgraded to the run action history schema
// (Decision 0129); no installed-upgrade or old-binary qualification.

fn version_twenty_one(path: &Path, key: &[u8; 32]) {
    version_twenty(path, key);
    let connection = open_connection(path, key).unwrap();
    let sql = crate::operational_store::MIGRATION_21_SCHEMA_SQL;
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute_batch(sql).unwrap();
    transaction
        .execute(
            "INSERT INTO schema_history(version, migration_sha256) VALUES (21, ?1)",
            [sha256_hex(sql.as_bytes())],
        )
        .unwrap();
    transaction
        .pragma_update(None, "user_version", 21_i64)
        .unwrap();
    transaction.commit().unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-21.json")).unwrap();
    assert_eq!(tables(&connection), fixture["tables"]);
    assert_eq!(history(&connection), fixture["migrations"]);
}

fn run_action_history_tables(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema
             WHERE type = 'table' AND name LIKE 'run_action_history_%'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn version_twenty_one_upgrades_to_run_action_histories_preserving_records_and_history() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [101; 32];
    version_twenty_one(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let original_history = history(&connection);
    let original_record = legacy_record(&connection);
    drop(connection);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    let version: i64 = store
        .connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    // The later documentation pack, owner state and run effect record
    // migrations add only their own tables (Decisions 0130, 0131 and 0143).
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-25.json")).unwrap();
    assert_eq!(tables(&store.connection), fixture["tables"]);
    assert_eq!(history(&store.connection), fixture["migrations"]);
    let migrated = history(&store.connection);
    assert_eq!(
        &migrated.as_array().unwrap()[..21],
        original_history.as_array().unwrap()
    );
    assert_eq!(legacy_record(&store.connection), original_record);
    let chains: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM run_action_history_roots", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(chains, 0);
    drop(store);
    drop(OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_version_twenty_two_migration_rolls_back_and_stays_retryable() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [102; 32];
    version_twenty_one(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let before = history(&connection);
    let record = legacy_record(&connection);
    // A conflicting table makes the run action history migration fail part way.
    connection
        .execute_batch("CREATE TABLE run_action_history_heads(incompatible INTEGER) STRICT;")
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
    assert_eq!(version, 21);
    assert_eq!(history(&connection), before);
    assert_eq!(legacy_record(&connection), record);
    // Only the conflicting table exists; no root or record table was kept.
    assert_eq!(run_action_history_tables(&connection), 1);
    connection
        .execute_batch("DROP TABLE run_action_history_heads;")
        .unwrap();
    drop(connection);
    let current = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    assert_eq!(history(&current.connection).as_array().unwrap().len(), 25);
    assert_eq!(run_action_history_tables(&current.connection), 3);
    assert_eq!(legacy_record(&current.connection), record);
    drop(current);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn version_twenty_one_corrupt_history_cannot_add_run_action_histories() {
    for mutation in 0..3 {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let key = [103; 32];
        version_twenty_one(&path, &key);
        let connection = open_connection(&path, &key).unwrap();
        match mutation {
            0 => {
                connection
                    .execute(
                        "UPDATE schema_history SET migration_sha256 = ?1 WHERE version = 21",
                        ["0".repeat(64)],
                    )
                    .unwrap();
            }
            1 => {
                connection
                    .execute("DELETE FROM schema_history WHERE version = 20", [])
                    .unwrap();
            }
            _ => {
                connection
                    .execute("INSERT INTO schema_history VALUES (22, ?1)", ["0".repeat(64)])
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
        assert_eq!(version, 21);
        assert_eq!(history(&connection), before);
        assert_eq!(legacy_record(&connection), record);
        assert_eq!(run_action_history_tables(&connection), 0);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }
}

mod doc_pack_migration {
    use super::*;
    include!("doc_pack_migration_tests.rs");
}
