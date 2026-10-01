// Real encrypted schema 22 fixtures upgraded to the documentation pack schema
// (Decision 0130); no installed-upgrade or old-binary qualification.

fn version_twenty_two(path: &Path, key: &[u8; 32]) {
    version_twenty_one(path, key);
    let connection = open_connection(path, key).unwrap();
    let sql = crate::operational_store::MIGRATION_22_SCHEMA_SQL;
    let transaction = connection.unchecked_transaction().unwrap();
    transaction.execute_batch(sql).unwrap();
    transaction
        .execute(
            "INSERT INTO schema_history(version, migration_sha256) VALUES (22, ?1)",
            [sha256_hex(sql.as_bytes())],
        )
        .unwrap();
    transaction
        .pragma_update(None, "user_version", 22_i64)
        .unwrap();
    transaction.commit().unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-22.json")).unwrap();
    assert_eq!(tables(&connection), fixture["tables"]);
    assert_eq!(history(&connection), fixture["migrations"]);
}

fn doc_pack_tables(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name LIKE 'doc_pack_%'",
            [],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn version_twenty_two_upgrades_to_documentation_packs_preserving_records_and_history() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [111; 32];
    version_twenty_two(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let original_history = history(&connection);
    let original_record = legacy_record(&connection);
    drop(connection);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    let version: i64 = store
        .connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 23);
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-23.json")).unwrap();
    assert_eq!(tables(&store.connection), fixture["tables"]);
    assert_eq!(history(&store.connection), fixture["migrations"]);
    let migrated = history(&store.connection);
    assert_eq!(
        &migrated.as_array().unwrap()[..22],
        original_history.as_array().unwrap()
    );
    assert_eq!(legacy_record(&store.connection), original_record);
    let heads: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM doc_pack_catalog_heads", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(heads, 0);
    drop(store);
    drop(OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_version_twenty_three_migration_rolls_back_and_stays_retryable() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [112; 32];
    version_twenty_two(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    let before = history(&connection);
    let record = legacy_record(&connection);
    // A conflicting table makes the documentation pack migration fail part way.
    connection
        .execute_batch("CREATE TABLE doc_pack_files(incompatible INTEGER) STRICT;")
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
    assert_eq!(version, 22);
    assert_eq!(history(&connection), before);
    assert_eq!(legacy_record(&connection), record);
    // Only the conflicting table exists; no head, version or deletion table
    // was kept.
    assert_eq!(doc_pack_tables(&connection), 1);
    connection
        .execute_batch("DROP TABLE doc_pack_files;")
        .unwrap();
    drop(connection);
    let current = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    assert_eq!(history(&current.connection).as_array().unwrap().len(), 23);
    assert_eq!(doc_pack_tables(&current.connection), 4);
    assert_eq!(legacy_record(&current.connection), record);
    drop(current);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn version_twenty_two_corrupt_history_cannot_add_documentation_packs() {
    for mutation in 0..3 {
        let directory = temporary_directory();
        let path = directory.join("authority.db");
        let key = [113; 32];
        version_twenty_two(&path, &key);
        let connection = open_connection(&path, &key).unwrap();
        match mutation {
            0 => {
                connection
                    .execute(
                        "UPDATE schema_history SET migration_sha256 = ?1 WHERE version = 22",
                        ["0".repeat(64)],
                    )
                    .unwrap();
            }
            1 => {
                connection
                    .execute("DELETE FROM schema_history WHERE version = 21", [])
                    .unwrap();
            }
            _ => {
                connection
                    .execute("INSERT INTO schema_history VALUES (23, ?1)", ["0".repeat(64)])
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
        assert_eq!(version, 22);
        assert_eq!(history(&connection), before);
        assert_eq!(legacy_record(&connection), record);
        assert_eq!(doc_pack_tables(&connection), 0);
        drop(connection);
        fs::remove_dir_all(directory).unwrap();
    }
}
