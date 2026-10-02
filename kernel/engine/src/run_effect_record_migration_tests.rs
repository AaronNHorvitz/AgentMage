// Real encrypted schema 24 fixtures upgraded to the run effect record schema
// (Decision 0143); no installed-upgrade or old-binary qualification.

fn version_twenty_four(path: &Path, key: &[u8; 32]) {
    version_twenty_three(path, key);
    let connection = open_connection(path, key).unwrap();
    let sql = crate::operational_store::MIGRATION_24_SCHEMA_SQL;
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute_batch(sql).unwrap();
    transaction
        .execute(
            "INSERT INTO schema_history(version, migration_sha256) VALUES (24, ?1)",
            [sha256_hex(sql.as_bytes())],
        )
        .unwrap();
    transaction
        .pragma_update(None, "user_version", 24_i64)
        .unwrap();
    transaction.commit().unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-24.json")).unwrap();
    assert_eq!(tables(&connection), fixture["tables"]);
    assert_eq!(history(&connection), fixture["migrations"]);
}

fn run_effect_record_tables(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name LIKE 'run_effect_record%'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn version_twenty_four_upgrades_to_run_effect_records_preserving_records_and_history() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [131; 32];
    version_twenty_four(&path, &key);
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
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-25.json")).unwrap();
    assert_eq!(tables(&store.connection), fixture["tables"]);
    assert_eq!(history(&store.connection), fixture["migrations"]);
    let migrated = history(&store.connection);
    assert_eq!(
        &migrated.as_array().unwrap()[..24],
        original_history.as_array().unwrap()
    );
    assert_eq!(legacy_record(&store.connection), original_record);
    assert_eq!(run_effect_record_tables(&store.connection), 3);
    let roots: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM run_effect_record_roots", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(roots, 0);
    drop(store);
    drop(OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_version_twenty_five_migration_rolls_back_and_stays_retryable() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [132; 32];
    version_twenty_four(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let before = history(&connection);
    let record = legacy_record(&connection);
    // A conflicting trigger name makes the run effect record migration fail
    // after its tables were created.
    connection
        .execute_batch(
            "CREATE TRIGGER run_effect_record_head_forward_only AFTER UPDATE ON schema_history
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
    assert_eq!(version, 24);
    assert_eq!(history(&connection), before);
    assert_eq!(legacy_record(&connection), record);
    // The tables created before the failure were rolled back with it.
    assert_eq!(run_effect_record_tables(&connection), 0);
    connection
        .execute_batch("DROP TRIGGER run_effect_record_head_forward_only;")
        .unwrap();
    drop(connection);
    let current = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    assert_eq!(history(&current.connection).as_array().unwrap().len(), 25);
    assert_eq!(run_effect_record_tables(&current.connection), 3);
    assert_eq!(legacy_record(&current.connection), record);
    drop(current);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn version_twenty_four_corrupt_history_cannot_add_run_effect_records() {
    for mutation in 0..3 {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let key = [133; 32];
        version_twenty_four(&path, &key);
        let connection = open_connection(&path, &key).unwrap();
        match mutation {
            0 => {
                connection
                    .execute(
                        "UPDATE schema_history SET migration_sha256 = ?1 WHERE version = 24",
                        ["0".repeat(64)],
                    )
                    .unwrap();
            }
            1 => {
                connection
                    .execute("DELETE FROM schema_history WHERE version = 23", [])
                    .unwrap();
            }
            _ => {
                connection
                    .execute("INSERT INTO schema_history VALUES (25, ?1)", ["0".repeat(64)])
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
        assert_eq!(version, 24);
        assert_eq!(history(&connection), before);
        assert_eq!(legacy_record(&connection), record);
        assert_eq!(run_effect_record_tables(&connection), 0);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }
}
