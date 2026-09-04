//! Bounded parameterized access to AgentMage-owned and synthetic SQLite records.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use rusqlite::{Connection, OpenFlags, TransactionBehavior, params, types::ValueRef};
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

const MAX_PARAMETER_BYTES: usize = 4_096;
const MAX_RESULT_ROWS: u32 = 10_000;
const MAX_RESULT_COLUMNS: u32 = 64;
const MAX_RESULT_BYTES: u64 = 16 * 1_024 * 1_024;

/// Closed database adapter family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseAdapterKind {
    /// The admitted local SQLite adapter.
    Sqlite,
    /// A caller-supplied synthetic fixture projected through SQLite.
    Fixture,
    /// Reserved and disabled for this release.
    PostgreSql,
}

/// Closed source admission class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseSourceKind {
    /// AgentMage-owned local state identified without caller path authority.
    AgentMageOwned,
    /// Synthetic test data created in memory.
    SyntheticFixture,
    /// Live, remote, credentialed, or otherwise external database; always refused.
    LiveExternal,
}

/// Content-addressed database source identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseSourceIdentity {
    /// Stable source identifier.
    pub source_id: String,
    /// Closed source class.
    pub source_kind: DatabaseSourceKind,
    /// Exact database content or fixture identity.
    pub source_sha256: String,
    /// Caller-observed freshness time in Unix epoch milliseconds.
    pub freshness_epoch_milliseconds: u64,
}

/// Separately granted database capabilities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseAccessGrant {
    /// Permit schema inventory only.
    pub inspect_schema: bool,
    /// Permit row-data templates only.
    pub read_rows: bool,
    /// Permit the closed synthetic-fixture construction transaction only.
    pub construct_fixture: bool,
}

/// Closed query template inventory. No raw SQL entry point exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseQueryTemplate {
    /// Inventory the admitted application table.
    SchemaInventory,
    /// Select evidence rows using one bound category parameter.
    EvidenceByCategory,
    /// Select evidence rows in stable identity order.
    EvidenceAll,
}

/// Bounded query request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseQueryRequest {
    /// Stable report scope authorized by the caller.
    pub report_scope_id: String,
    /// Exact source expected by the report.
    pub source_id: String,
    /// Closed query template.
    pub template: DatabaseQueryTemplate,
    /// Parameter present only for `evidence_by_category`.
    pub category_parameter: Option<String>,
    /// Columns whose text/blob values must be minimized.
    pub redacted_columns: Vec<String>,
    /// Query-specific ceilings.
    pub limits: DatabaseQueryLimits,
}

/// Query resource ceilings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseQueryLimits {
    /// Maximum returned rows.
    pub max_rows: u32,
    /// Maximum returned columns.
    pub max_columns: u32,
    /// Maximum encoded result bytes.
    pub max_bytes: u64,
}

/// One caller-supplied synthetic or already-admitted local fixture row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseFixtureRow {
    /// Stable row identity.
    pub row_id: String,
    /// Stable category used by the parameterized query template.
    pub category: String,
    /// Optional human-readable label.
    pub label: Option<String>,
    /// Exact fixed-point amount in millionths.
    pub amount_micros: i64,
    /// Optional value exercised by minimization policy.
    pub secret: Option<String>,
}

/// Type-preserving minimized SQLite value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum StructuredValue {
    /// SQLite NULL.
    Null,
    /// Exact signed integer.
    Integer(i64),
    /// Exact IEEE-754 bit pattern, avoiding decimal precision claims.
    RealBits(u64),
    /// UTF-8 text.
    Text(String),
    /// Content hash and byte count instead of raw binary data.
    Blob {
        /// Exact blob digest.
        sha256: String,
        /// Exact blob byte count.
        byte_count: u64,
    },
    /// Policy-minimized value retaining only its exact hash.
    Redacted {
        /// Exact digest of the minimized value.
        sha256: String,
    },
}

/// One deterministically ordered structured-evidence row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredEvidenceRow {
    /// Stable row identity.
    pub row_id: String,
    /// Ordered typed column values.
    pub columns: Vec<(String, StructuredValue)>,
    /// Stable normalization reason.
    pub reason_code: String,
    /// Integer uncertainty in parts per million.
    pub uncertainty_ppm: u32,
    /// Explicit reviewer disposition; projection never auto-accepts evidence.
    pub reviewer_decision: StructuredEvidenceReviewerDecision,
}

/// Closed reviewer disposition for projected database evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredEvidenceReviewerDecision {
    /// Awaiting explicit review.
    Pending,
    /// Explicitly accepted outside the query adapter.
    Accepted,
    /// Explicitly rejected outside the query adapter.
    Rejected,
}

/// Deterministic query receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredDatabaseReceipt {
    /// Kernel contract version.
    pub schema_version: u16,
    /// Exact source identity.
    pub source: DatabaseSourceIdentity,
    /// Adapter family.
    pub adapter_kind: DatabaseAdapterKind,
    /// Report scope.
    pub report_scope_id: String,
    /// Query classification.
    pub template: DatabaseQueryTemplate,
    /// Exact schema identity.
    pub database_schema_sha256: String,
    /// Hash of ordered typed parameters.
    pub parameters_sha256: String,
    /// Hash of ordered minimized rows.
    pub rows_sha256: String,
    /// Returned row count.
    pub row_count: u32,
    /// Encoded returned bytes.
    pub returned_bytes: u64,
    /// Applied row ceiling.
    pub max_rows: u32,
    /// Applied encoded-byte ceiling.
    pub max_bytes: u64,
    /// True after SQLite query-only mode and statement classification are checked.
    pub read_only_verified: bool,
    /// True when no truncation occurred.
    pub complete: bool,
    /// Stable content-free limitation codes.
    pub limitations: Vec<String>,
    /// False because the query adapter opens no path and performs no file mutation.
    pub filesystem_effect_performed: bool,
    /// False because no network connector exists.
    pub network_effect_performed: bool,
    /// Hash of every preceding field.
    pub receipt_sha256: String,
}

/// Query result and receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseQueryResult {
    /// Ordered minimized rows.
    pub rows: Vec<StructuredEvidenceRow>,
    /// Exact receipt.
    pub receipt: StructuredDatabaseReceipt,
}

/// Closed database adapter failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseAdapterError {
    /// Identity, grant, request, parameter, or schema input is invalid.
    InvalidInput,
    /// The requested source, adapter, or operation is outside release scope.
    NotAuthorized,
    /// A compiled or request resource ceiling was exceeded.
    ResourceLimit,
    /// The caller cancelled before or during row projection.
    Cancelled,
    /// SQLite rejected the admitted fixed operation.
    StorageFailure,
}

impl DatabaseAdapterError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "database.input.invalid",
            Self::NotAuthorized => "database.operation.not-authorized",
            Self::ResourceLimit => "database.resource.limit",
            Self::Cancelled => "database.query.cancelled",
            Self::StorageFailure => "database.storage.failure",
        }
    }
}

impl std::fmt::Display for DatabaseAdapterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DatabaseAdapterError {}

/// One admitted in-memory view over AgentMage-owned or synthetic records.
pub struct LocalStructuredDatabase {
    connection: Connection,
    source: DatabaseSourceIdentity,
    adapter_kind: DatabaseAdapterKind,
    grant: DatabaseAccessGrant,
    schema_sha256: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_source(source: &DatabaseSourceIdentity) -> Result<(), DatabaseAdapterError> {
    if !valid_identifier(&source.source_id)
        || !valid_sha256(&source.source_sha256)
        || source.freshness_epoch_milliseconds == 0
        || source.source_kind == DatabaseSourceKind::LiveExternal
    {
        return Err(DatabaseAdapterError::NotAuthorized);
    }
    Ok(())
}

impl LocalStructuredDatabase {
    /// Constructs a migrated in-memory synthetic fixture through one transaction.
    pub fn from_synthetic_fixture(
        source: DatabaseSourceIdentity,
        grant: DatabaseAccessGrant,
        rows: &[DatabaseFixtureRow],
    ) -> Result<Self, DatabaseAdapterError> {
        if source.source_kind != DatabaseSourceKind::SyntheticFixture || !grant.construct_fixture {
            return Err(DatabaseAdapterError::NotAuthorized);
        }
        Self::build(source, DatabaseAdapterKind::Fixture, grant, rows)
    }

    /// Constructs an authority-free test view for already admitted AgentMage-owned rows.
    pub fn from_agentmage_owned_rows(
        source: DatabaseSourceIdentity,
        grant: DatabaseAccessGrant,
        rows: &[DatabaseFixtureRow],
    ) -> Result<Self, DatabaseAdapterError> {
        if source.source_kind != DatabaseSourceKind::AgentMageOwned || !grant.construct_fixture {
            return Err(DatabaseAdapterError::NotAuthorized);
        }
        Self::build(source, DatabaseAdapterKind::Sqlite, grant, rows)
    }

    /// Refuses future or live adapter families without opening a connection.
    pub fn refuse_unavailable_adapter(
        adapter: DatabaseAdapterKind,
        source_kind: DatabaseSourceKind,
    ) -> Result<(), DatabaseAdapterError> {
        if adapter == DatabaseAdapterKind::PostgreSql
            || source_kind == DatabaseSourceKind::LiveExternal
        {
            return Err(DatabaseAdapterError::NotAuthorized);
        }
        Ok(())
    }

    fn build(
        source: DatabaseSourceIdentity,
        adapter_kind: DatabaseAdapterKind,
        grant: DatabaseAccessGrant,
        rows: &[DatabaseFixtureRow],
    ) -> Result<Self, DatabaseAdapterError> {
        validate_source(&source)?;
        let mut connection = Connection::open_with_flags(
            ":memory:",
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        connection
            .execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON;")
            .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        transaction
            .execute_batch(
                "CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY) STRICT;
             INSERT INTO schema_migrations(version) VALUES(1);
             CREATE TABLE evidence_rows(
               row_id TEXT PRIMARY KEY, category TEXT NOT NULL, label TEXT,
               amount_micros INTEGER NOT NULL, secret TEXT
             ) STRICT;
             CREATE INDEX evidence_rows_category ON evidence_rows(category, row_id);",
            )
            .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        for row in rows {
            if !valid_identifier(&row.row_id) || !valid_identifier(&row.category) {
                return Err(DatabaseAdapterError::InvalidInput);
            }
            transaction.execute(
                "INSERT INTO evidence_rows(row_id, category, label, amount_micros, secret) VALUES(?1, ?2, ?3, ?4, ?5)",
                params![row.row_id, row.category, row.label, row.amount_micros, row.secret],
            ).map_err(|_| DatabaseAdapterError::StorageFailure)?;
        }
        transaction
            .commit()
            .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        connection
            .execute_batch("PRAGMA query_only=ON;")
            .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        let schema_sha256 =
            word_sha256(b"schema-v1:evidence_rows(row_id,category,label,amount_micros,secret)");
        Ok(Self {
            connection,
            source,
            adapter_kind,
            grant,
            schema_sha256,
        })
    }

    /// Executes one fixed, parameterized, read-only query and projects typed evidence.
    pub fn query(
        &self,
        request: &DatabaseQueryRequest,
        cancelled: &AtomicBool,
    ) -> Result<DatabaseQueryResult, DatabaseAdapterError> {
        self.validate_request(request)?;
        if cancelled.load(Ordering::Acquire) {
            return Err(DatabaseAdapterError::Cancelled);
        }
        let (sql, parameter) = match request.template {
            DatabaseQueryTemplate::SchemaInventory => (
                "SELECT 'evidence_rows' AS row_id, 'table' AS object_kind, 5 AS column_count ORDER BY row_id LIMIT ?1",
                None,
            ),
            DatabaseQueryTemplate::EvidenceByCategory => (
                "SELECT row_id, category, label, amount_micros, secret FROM evidence_rows WHERE category=?1 ORDER BY row_id LIMIT ?2",
                request.category_parameter.as_deref(),
            ),
            DatabaseQueryTemplate::EvidenceAll => (
                "SELECT row_id, category, label, amount_micros, secret FROM evidence_rows ORDER BY row_id LIMIT ?1",
                None,
            ),
        };
        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        if !statement.readonly() {
            return Err(DatabaseAdapterError::NotAuthorized);
        }
        let column_names: Vec<String> = statement
            .column_names()
            .iter()
            .map(|value| (*value).to_owned())
            .collect();
        if column_names.len() > request.limits.max_columns as usize {
            return Err(DatabaseAdapterError::ResourceLimit);
        }
        let bound_limit = i64::from(request.limits.max_rows) + 1;
        let mut query = match parameter {
            Some(value) => statement.query(params![value, bound_limit]),
            None => statement.query(params![bound_limit]),
        }
        .map_err(|_| DatabaseAdapterError::StorageFailure)?;
        let redacted: BTreeSet<&str> = request
            .redacted_columns
            .iter()
            .map(String::as_str)
            .collect();
        let mut rows = Vec::new();
        let mut returned_bytes = 0_u64;
        let mut complete = true;
        while let Some(row) = query
            .next()
            .map_err(|_| DatabaseAdapterError::StorageFailure)?
        {
            if cancelled.load(Ordering::Acquire) {
                return Err(DatabaseAdapterError::Cancelled);
            }
            if rows.len() == request.limits.max_rows as usize {
                complete = false;
                break;
            }
            let mut columns = Vec::with_capacity(column_names.len());
            for (index, name) in column_names.iter().enumerate() {
                let value = project_value(
                    row.get_ref(index)
                        .map_err(|_| DatabaseAdapterError::StorageFailure)?,
                    redacted.contains(name.as_str()),
                )?;
                returned_bytes = returned_bytes.saturating_add(
                    u64::try_from(
                        name.len()
                            + serde_json::to_vec(&value)
                                .map_err(|_| DatabaseAdapterError::InvalidInput)?
                                .len(),
                    )
                    .unwrap_or(u64::MAX),
                );
                if returned_bytes > request.limits.max_bytes {
                    return Err(DatabaseAdapterError::ResourceLimit);
                }
                columns.push((name.clone(), value));
            }
            let row_id = match &columns[0].1 {
                StructuredValue::Text(value) if valid_identifier(value) => value.clone(),
                _ => return Err(DatabaseAdapterError::InvalidInput),
            };
            rows.push(StructuredEvidenceRow {
                row_id,
                columns,
                reason_code: "database.row.observed".to_owned(),
                uncertainty_ppm: 0,
                reviewer_decision: StructuredEvidenceReviewerDecision::Pending,
            });
        }
        let parameters_sha256 = word_sha256(
            serde_json::to_vec(&request.category_parameter)
                .map_err(|_| DatabaseAdapterError::InvalidInput)?
                .as_slice(),
        );
        let rows_bytes =
            serde_json::to_vec(&rows).map_err(|_| DatabaseAdapterError::InvalidInput)?;
        let mut limitations = Vec::new();
        if !complete {
            limitations.push("database.result.truncated".to_owned());
        }
        let mut receipt = StructuredDatabaseReceipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source: self.source.clone(),
            adapter_kind: self.adapter_kind,
            report_scope_id: request.report_scope_id.clone(),
            template: request.template,
            database_schema_sha256: self.schema_sha256.clone(),
            parameters_sha256,
            rows_sha256: word_sha256(&rows_bytes),
            row_count: u32::try_from(rows.len()).unwrap_or(u32::MAX),
            returned_bytes,
            max_rows: request.limits.max_rows,
            max_bytes: request.limits.max_bytes,
            read_only_verified: true,
            complete,
            limitations,
            filesystem_effect_performed: false,
            network_effect_performed: false,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_digest(&receipt)?;
        Ok(DatabaseQueryResult { rows, receipt })
    }

    fn validate_request(&self, request: &DatabaseQueryRequest) -> Result<(), DatabaseAdapterError> {
        if !valid_identifier(&request.report_scope_id)
            || request.source_id != self.source.source_id
            || request.limits.max_rows == 0
            || request.limits.max_rows > MAX_RESULT_ROWS
            || request.limits.max_columns == 0
            || request.limits.max_columns > MAX_RESULT_COLUMNS
            || request.limits.max_bytes == 0
            || request.limits.max_bytes > MAX_RESULT_BYTES
            || request
                .redacted_columns
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || request
                .redacted_columns
                .iter()
                .any(|value| !valid_identifier(value))
        {
            return Err(DatabaseAdapterError::InvalidInput);
        }
        match request.template {
            DatabaseQueryTemplate::SchemaInventory
                if !self.grant.inspect_schema || request.category_parameter.is_some() =>
            {
                Err(DatabaseAdapterError::NotAuthorized)
            }
            DatabaseQueryTemplate::EvidenceByCategory if !self.grant.read_rows => {
                Err(DatabaseAdapterError::NotAuthorized)
            }
            DatabaseQueryTemplate::EvidenceByCategory
                if request.category_parameter.as_deref().is_none_or(|value| {
                    !valid_identifier(value) || value.len() > MAX_PARAMETER_BYTES
                }) =>
            {
                Err(DatabaseAdapterError::InvalidInput)
            }
            DatabaseQueryTemplate::EvidenceAll
                if !self.grant.read_rows || request.category_parameter.is_some() =>
            {
                Err(DatabaseAdapterError::NotAuthorized)
            }
            _ => Ok(()),
        }
    }
}

fn project_value(
    value: ValueRef<'_>,
    redact: bool,
) -> Result<StructuredValue, DatabaseAdapterError> {
    let encoded = match value {
        ValueRef::Null => Vec::new(),
        ValueRef::Integer(value) => value.to_be_bytes().to_vec(),
        ValueRef::Real(value) => value.to_bits().to_be_bytes().to_vec(),
        ValueRef::Text(value) | ValueRef::Blob(value) => value.to_vec(),
    };
    if redact && !matches!(value, ValueRef::Null) {
        return Ok(StructuredValue::Redacted {
            sha256: word_sha256(&encoded),
        });
    }
    Ok(match value {
        ValueRef::Null => StructuredValue::Null,
        ValueRef::Integer(value) => StructuredValue::Integer(value),
        ValueRef::Real(value) => StructuredValue::RealBits(value.to_bits()),
        ValueRef::Text(value) => StructuredValue::Text(
            std::str::from_utf8(value)
                .map_err(|_| DatabaseAdapterError::InvalidInput)?
                .to_owned(),
        ),
        ValueRef::Blob(value) => StructuredValue::Blob {
            sha256: word_sha256(value),
            byte_count: u64::try_from(value.len()).unwrap_or(u64::MAX),
        },
    })
}

fn receipt_digest(receipt: &StructuredDatabaseReceipt) -> Result<String, DatabaseAdapterError> {
    let mut copy = receipt.clone();
    copy.receipt_sha256.clear();
    serde_json::to_vec(&copy)
        .map(|bytes| word_sha256(&bytes))
        .map_err(|_| DatabaseAdapterError::InvalidInput)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(kind: DatabaseSourceKind) -> DatabaseSourceIdentity {
        DatabaseSourceIdentity {
            source_id: "fixture-1".to_owned(),
            source_kind: kind,
            source_sha256: "a".repeat(64),
            freshness_epoch_milliseconds: 1,
        }
    }
    fn grant(schema: bool, rows: bool) -> DatabaseAccessGrant {
        DatabaseAccessGrant {
            inspect_schema: schema,
            read_rows: rows,
            construct_fixture: true,
        }
    }
    fn limits(rows: u32) -> DatabaseQueryLimits {
        DatabaseQueryLimits {
            max_rows: rows,
            max_columns: 8,
            max_bytes: 16_384,
        }
    }
    fn fixture(grant: DatabaseAccessGrant) -> LocalStructuredDatabase {
        LocalStructuredDatabase::from_synthetic_fixture(
            source(DatabaseSourceKind::SyntheticFixture),
            grant,
            &[
                DatabaseFixtureRow {
                    row_id: "row-1".into(),
                    category: "finance".into(),
                    label: Some("invoice".into()),
                    amount_micros: 125_000,
                    secret: Some("secret-1".into()),
                },
                DatabaseFixtureRow {
                    row_id: "row-2".into(),
                    category: "legal".into(),
                    label: None,
                    amount_micros: -1,
                    secret: None,
                },
            ],
        )
        .expect("fixture")
    }

    #[test]
    fn parameterized_rows_preserve_types_redaction_provenance_and_receipt() {
        let database = fixture(grant(false, true));
        let result = database
            .query(
                &DatabaseQueryRequest {
                    report_scope_id: "report-1".into(),
                    source_id: "fixture-1".into(),
                    template: DatabaseQueryTemplate::EvidenceByCategory,
                    category_parameter: Some("finance".into()),
                    redacted_columns: vec!["secret".into()],
                    limits: limits(10),
                },
                &AtomicBool::new(false),
            )
            .expect("query");
        assert_eq!(result.rows.len(), 1);
        assert!(matches!(
            result.rows[0].columns[3].1,
            StructuredValue::Integer(125_000)
        ));
        assert!(matches!(
            result.rows[0].columns[4].1,
            StructuredValue::Redacted { .. }
        ));
        assert!(result.receipt.read_only_verified && result.receipt.complete);
        assert_eq!(result.receipt.source.source_sha256, "a".repeat(64));
    }

    #[test]
    fn parameter_injection_is_data_and_schema_grant_does_not_read_rows() {
        let database = fixture(grant(true, false));
        let schema = database
            .query(
                &DatabaseQueryRequest {
                    report_scope_id: "report-1".into(),
                    source_id: "fixture-1".into(),
                    template: DatabaseQueryTemplate::SchemaInventory,
                    category_parameter: None,
                    redacted_columns: vec![],
                    limits: limits(10),
                },
                &AtomicBool::new(false),
            )
            .expect("schema");
        assert_eq!(schema.rows[0].row_id, "evidence_rows");
        let request = DatabaseQueryRequest {
            report_scope_id: "report-1".into(),
            source_id: "fixture-1".into(),
            template: DatabaseQueryTemplate::EvidenceByCategory,
            category_parameter: Some("finance-or-1-equals-1".into()),
            redacted_columns: vec![],
            limits: limits(10),
        };
        assert_eq!(
            database
                .query(&request, &AtomicBool::new(false))
                .unwrap_err(),
            DatabaseAdapterError::NotAuthorized
        );
    }

    #[test]
    fn row_limits_truncate_and_byte_limits_fail_closed() {
        let database = fixture(grant(false, true));
        let mut request = DatabaseQueryRequest {
            report_scope_id: "report-1".into(),
            source_id: "fixture-1".into(),
            template: DatabaseQueryTemplate::EvidenceAll,
            category_parameter: None,
            redacted_columns: vec![],
            limits: limits(1),
        };
        let result = database
            .query(&request, &AtomicBool::new(false))
            .expect("bounded");
        assert!(!result.receipt.complete);
        assert_eq!(result.receipt.limitations, ["database.result.truncated"]);
        request.limits.max_bytes = 1;
        assert_eq!(
            database
                .query(&request, &AtomicBool::new(false))
                .unwrap_err(),
            DatabaseAdapterError::ResourceLimit
        );
    }

    #[test]
    fn cancellation_live_sources_future_adapters_and_wrong_scope_fail_closed() {
        let database = fixture(grant(false, true));
        let request = DatabaseQueryRequest {
            report_scope_id: "report-1".into(),
            source_id: "fixture-1".into(),
            template: DatabaseQueryTemplate::EvidenceAll,
            category_parameter: None,
            redacted_columns: vec![],
            limits: limits(10),
        };
        assert_eq!(
            database
                .query(&request, &AtomicBool::new(true))
                .unwrap_err(),
            DatabaseAdapterError::Cancelled
        );
        assert_eq!(
            LocalStructuredDatabase::refuse_unavailable_adapter(
                DatabaseAdapterKind::PostgreSql,
                DatabaseSourceKind::SyntheticFixture
            ),
            Err(DatabaseAdapterError::NotAuthorized)
        );
        assert_eq!(
            LocalStructuredDatabase::refuse_unavailable_adapter(
                DatabaseAdapterKind::Sqlite,
                DatabaseSourceKind::LiveExternal
            ),
            Err(DatabaseAdapterError::NotAuthorized)
        );
        let mut wrong = request;
        wrong.source_id = "other-source".into();
        assert_eq!(
            database.query(&wrong, &AtomicBool::new(false)).unwrap_err(),
            DatabaseAdapterError::InvalidInput
        );
    }
}
