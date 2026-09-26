// Actual encrypted legacy-schema migration fixtures; no transport or model effects.
use super::*;

fn version_eighteen(path: &Path, key: &[u8; 32]) {
    create_version_two_store(path, key);
    let connection = open_connection(path, key).unwrap();
    for (index, sql) in [
        super::super::MIGRATION_3_SCHEMA_SQL,
        super::super::MIGRATION_4_SCHEMA_SQL,
        super::super::MIGRATION_5_SCHEMA_SQL,
        super::super::MIGRATION_6_SCHEMA_SQL,
        super::super::MIGRATION_7_SCHEMA_SQL,
        super::super::MIGRATION_8_SCHEMA_SQL,
        super::super::MIGRATION_9_SCHEMA_SQL,
        super::super::MIGRATION_10_SCHEMA_SQL,
        super::super::MIGRATION_11_SCHEMA_SQL,
        super::super::MIGRATION_12_SCHEMA_SQL,
        super::super::MIGRATION_13_SCHEMA_SQL,
        super::super::MIGRATION_14_SCHEMA_SQL,
        super::super::MIGRATION_15_SCHEMA_SQL,
        super::super::MIGRATION_16_SCHEMA_SQL,
        super::super::MIGRATION_17_SCHEMA_SQL,
        super::super::MIGRATION_18_SCHEMA_SQL,
    ]
    .iter()
    .enumerate()
    {
        // The empty v2 fixture has no retention rows needing v3 data conversion.
        let transaction = connection.unchecked_transaction().unwrap();
        transaction.execute_batch(sql).unwrap();
        let version = i64::try_from(index).unwrap() + 3;
        transaction
            .execute(
                "INSERT INTO schema_history(version, migration_sha256) VALUES (?1, ?2)",
                params![version, sha256_hex(sql.as_bytes())],
            )
            .unwrap();
        transaction
            .pragma_update(None, "user_version", version)
            .unwrap();
        transaction.commit().unwrap();
    }
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/operational-store/schema-18.json")).unwrap();
    let tables: Vec<String> = connection.prepare(
        "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    ).unwrap().query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(serde_json::to_value(tables).unwrap(), fixture["tables"]);
    connection.execute(
        "INSERT INTO sessions VALUES ('legacy-research-owner', 'profile-1', 'active', 1, 1, ?1, X'7B7D')",
        [sha256_hex(b"{}")],
    ).unwrap();
}

#[test]
fn version_eighteen_upgrades_without_losing_existing_records() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [91; 32];
    version_eighteen(&path, &key);
    let store = OperationalStore::open(&path, &observation(), &mut TestKey(key)).unwrap();
    let version: i64 = store
        .connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 19);
    let record: Vec<u8> = store
        .connection
        .query_row(
            "SELECT record_json FROM sessions WHERE session_id = 'legacy-research-owner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(record, b"{}");
    let count: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM research_budget_roots", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 0);
    drop(store);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn failed_version_nineteen_migration_rolls_back_tables_history_and_version() {
    let directory = temporary_directory();
    let path = directory.join("authority.db");
    let key = [92; 32];
    version_eighteen(&path, &key);
    let connection = open_connection(&path, &key).unwrap();
    connection
        .execute_batch("CREATE TABLE research_budget_heads(incompatible INTEGER) STRICT;")
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
    let history: i64 = connection
        .query_row("SELECT COUNT(*) FROM schema_history", [], |row| row.get(0))
        .unwrap();
    let created: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_schema WHERE name IN ('research_budget_roots', 'research_budget_revisions')", [], |row| row.get(0),
    ).unwrap();
    let records: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sessions WHERE session_id = 'legacy-research-owner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((version, history, created, records), (18, 18, 0, 1));
    drop(connection);
    fs::remove_dir_all(directory).unwrap();
}
