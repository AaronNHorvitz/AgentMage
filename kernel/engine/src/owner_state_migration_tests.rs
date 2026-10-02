// Real encrypted schema 23 fixtures upgraded to the owner state schema
// (Decision 0131); no installed-upgrade or old-binary qualification.

fn version_twenty_three(path: &Path, key: &[u8; 32]) {
    version_twenty_two(path, key);
    let connection = open_connection(path, key).unwrap();
    let sql = crate::operational_store::MIGRATION_23_SCHEMA_SQL;
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute_batch(sql).unwrap();
    transaction
        .execute(
            "INSERT INTO schema_history(version, migration_sha256) VALUES (23, ?1)",
            [sha256_hex(sql.as_bytes())],
        )
        .unwrap();
    transaction
        .pragma_update(None, "user_version", 23_i64)
        .unwrap();
    transaction.commit().unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-23.json")).unwrap();
    assert_eq!(tables(&connection), fixture["tables"]);
    assert_eq!(history(&connection), fixture["migrations"]);
}

fn owner_state_tables(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name = 'owner_states'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn version_twenty_three_upgrades_to_owner_states_preserving_records_and_history() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [121; 32];
    version_twenty_three(&path, &key);
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
    // The later run effect record migration adds only its own tables
    // (Decision 0143).
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-25.json")).unwrap();
    assert_eq!(tables(&store.connection), fixture["tables"]);
    assert_eq!(history(&store.connection), fixture["migrations"]);
    let migrated = history(&store.connection);
    assert_eq!(
        &migrated.as_array().unwrap()[..23],
        original_history.as_array().unwrap()
    );
    assert_eq!(legacy_record(&store.connection), original_record);
    let states: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM owner_states", [], |row| row.get(0))
        .unwrap();
    assert_eq!(states, 0);
    drop(store);
    drop(OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_version_twenty_four_migration_rolls_back_and_stays_retryable() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [122; 32];
    version_twenty_three(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let before = history(&connection);
    let record = legacy_record(&connection);
    // A conflicting trigger name makes the owner state migration fail after
    // its table was created.
    connection
        .execute_batch(
            "CREATE TRIGGER owner_state_delete_forbidden AFTER UPDATE ON schema_history
             BEGIN SELECT 1; END;",
        )
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
    assert_eq!(version, 23);
    assert_eq!(history(&connection), before);
    assert_eq!(legacy_record(&connection), record);
    // The table created before the failure was rolled back with it.
    assert_eq!(owner_state_tables(&connection), 0);
    connection
        .execute_batch("DROP TRIGGER owner_state_delete_forbidden;")
        .unwrap();
    drop(connection);
    let current = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    assert_eq!(history(&current.connection).as_array().unwrap().len(), 25);
    assert_eq!(owner_state_tables(&current.connection), 1);
    assert_eq!(legacy_record(&current.connection), record);
    drop(current);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn version_twenty_three_corrupt_history_cannot_add_owner_states() {
    for mutation in 0..3 {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let key = [123; 32];
        version_twenty_three(&path, &key);
        let connection = open_connection(&path, &key).unwrap();
        match mutation {
            0 => {
                connection
                    .execute(
                        "UPDATE schema_history SET migration_sha256 = ?1 WHERE version = 23",
                        ["0".repeat(64)],
                    )
                    .unwrap();
            }
            1 => {
                connection
                    .execute("DELETE FROM schema_history WHERE version = 22", [])
                    .unwrap();
            }
            _ => {
                connection
                    .execute("INSERT INTO schema_history VALUES (24, ?1)", ["0".repeat(64)])
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
        assert_eq!(version, 23);
        assert_eq!(history(&connection), before);
        assert_eq!(legacy_record(&connection), record);
        assert_eq!(owner_state_tables(&connection), 0);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }
}

mod run_effect_record_migration {
    use super::*;
    include!("run_effect_record_migration_tests.rs");
}
