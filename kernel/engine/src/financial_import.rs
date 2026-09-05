//! Immutable CSV, OFX, and QFX import and reconciliation contracts.
#![allow(missing_docs)]

use crate::financial_domain::{Currency, FinancialError, Money, UserDisposition};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportFormat {
    Csv,
    Ofx,
    Qfx,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportState {
    Detected,
    Confirmed,
    Parsed,
    Committed,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportError {
    Invalid,
    Ambiguous,
    Unsupported,
    Oversized,
    Malformed,
    Cancelled,
    Conflict,
    Duplicate,
    Arithmetic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportProfile {
    pub profile_id: String,
    pub format: ImportFormat,
    pub encoding: String,
    pub account_id: String,
    pub period_start: String,
    pub period_end: String,
    pub field_mapping: BTreeMap<String, String>,
    pub locale: String,
    pub sign_rule: String,
    pub currency: Currency,
    pub time_zone: String,
    pub parser_version: u32,
    pub user_confirmed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportEnvelope {
    pub source_id: String,
    pub source_sha256: String,
    pub byte_length: u64,
    pub format_version: String,
    pub extension_fields: BTreeSet<String>,
    pub entity_count: u32,
    pub cancelled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchKind {
    ExactDuplicate,
    PendingToPosted,
    Transfer,
    Split,
    Correction,
    Supersession,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportedRecord {
    pub record_id: String,
    pub source_id: String,
    pub source_row: u64,
    pub account_id: String,
    pub provider_id: String,
    pub posted_date: String,
    pub pending: bool,
    pub amount_canonical: String,
    pub description_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchCandidate {
    pub left_record_id: String,
    pub right_record_id: String,
    pub kind: MatchKind,
    pub evidence: BTreeSet<String>,
    pub confidence_basis_points: u16,
    pub disposition: UserDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatementReconciliation {
    pub statement_id: String,
    pub account_id: String,
    pub opening: Money,
    pub closing: Money,
    pub included_record_ids: BTreeSet<String>,
    pub pending_exclusions: BTreeSet<String>,
    pub adjustment_ids: BTreeSet<String>,
    pub calculated_closing: Money,
    pub tolerance_minor_units: i128,
    pub disposition: UserDisposition,
    pub conflict: Option<String>,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn detect_csv_profile(candidates: &[ImportProfile]) -> Result<ImportProfile, ImportError> {
    if candidates.len() != 1 {
        return Err(ImportError::Ambiguous);
    }
    let profile = candidates[0].clone();
    if profile.format != ImportFormat::Csv || !profile.user_confirmed {
        return Err(ImportError::Ambiguous);
    }
    validate_profile(&profile)?;
    Ok(profile)
}

pub fn validate_profile(profile: &ImportProfile) -> Result<(), ImportError> {
    if !id(&profile.profile_id)
        || !id(&profile.encoding)
        || !id(&profile.account_id)
        || !id(&profile.period_start)
        || !id(&profile.period_end)
        || !id(&profile.locale)
        || !id(&profile.sign_rule)
        || !id(&profile.time_zone)
        || profile.parser_version == 0
        || profile.field_mapping.is_empty()
    {
        return Err(ImportError::Invalid);
    }
    Ok(())
}

pub fn admit_envelope(
    envelope: &ImportEnvelope,
    format: ImportFormat,
    max_bytes: u64,
    max_entities: u32,
) -> Result<(), ImportError> {
    if envelope.cancelled {
        return Err(ImportError::Cancelled);
    }
    if !id(&envelope.source_id) || !digest(&envelope.source_sha256) {
        return Err(ImportError::Invalid);
    }
    if envelope.byte_length > max_bytes || envelope.entity_count > max_entities {
        return Err(ImportError::Oversized);
    }
    if !envelope.extension_fields.is_empty() {
        return Err(ImportError::Unsupported);
    }
    match (format, envelope.format_version.as_str()) {
        (ImportFormat::Csv, "1")
        | (ImportFormat::Ofx, "1.6")
        | (ImportFormat::Ofx, "2.2")
        | (ImportFormat::Qfx, "1.6") => Ok(()),
        _ => Err(ImportError::Unsupported),
    }
}

pub fn canonical_records(records: &[ImportedRecord]) -> Result<Vec<ImportedRecord>, ImportError> {
    let mut identities = BTreeSet::new();
    for record in records {
        if !id(&record.record_id)
            || !id(&record.source_id)
            || !id(&record.account_id)
            || !id(&record.provider_id)
            || !id(&record.posted_date)
            || !digest(&record.description_sha256)
            || !identities.insert(record.record_id.clone())
        {
            return Err(ImportError::Duplicate);
        }
    }
    let mut output = records.to_vec();
    output.sort();
    Ok(output)
}

pub fn reconcile(statement: &StatementReconciliation) -> Result<bool, ImportError> {
    if !id(&statement.statement_id)
        || !id(&statement.account_id)
        || statement.tolerance_minor_units < 0
    {
        return Err(ImportError::Invalid);
    }
    if statement.opening.currency() != statement.closing.currency()
        || statement.opening.currency() != statement.calculated_closing.currency()
        || statement.opening.scale() != statement.closing.scale()
        || statement.opening.scale() != statement.calculated_closing.scale()
    {
        return Err(ImportError::Arithmetic);
    }
    let difference = statement
        .closing
        .minor_units()
        .checked_sub(statement.calculated_closing.minor_units())
        .ok_or(ImportError::Arithmetic)?
        .abs();
    if difference > statement.tolerance_minor_units
        || statement.disposition == UserDisposition::Unresolved
    {
        if statement.conflict.is_none() {
            return Err(ImportError::Conflict);
        }
        return Ok(false);
    }
    if statement.conflict.is_some() {
        return Err(ImportError::Conflict);
    }
    Ok(true)
}

impl From<FinancialError> for ImportError {
    fn from(_: FinancialError) -> Self {
        Self::Arithmetic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> ImportProfile {
        ImportProfile {
            profile_id: "csv-us".into(),
            format: ImportFormat::Csv,
            encoding: "utf-8".into(),
            account_id: "account".into(),
            period_start: "2026-01-01".into(),
            period_end: "2026-01-31".into(),
            field_mapping: BTreeMap::from([("amount".into(), "Amount".into())]),
            locale: "en-US".into(),
            sign_rule: "signed".into(),
            currency: Currency::new("USD").unwrap(),
            time_zone: "UTC".into(),
            parser_version: 1,
            user_confirmed: true,
        }
    }
    #[test]
    fn ambiguous_csv_requires_confirmation() {
        let mut second = profile();
        second.profile_id = "other".into();
        assert_eq!(
            detect_csv_profile(&[profile(), second]),
            Err(ImportError::Ambiguous)
        )
    }
    #[test]
    fn ofx_qfx_are_bounded_and_extensions_fail_closed() {
        let e = ImportEnvelope {
            source_id: "file".into(),
            source_sha256: "a".repeat(64),
            byte_length: 10,
            format_version: "1.6".into(),
            extension_fields: BTreeSet::new(),
            entity_count: 1,
            cancelled: false,
        };
        assert_eq!(admit_envelope(&e, ImportFormat::Ofx, 100, 10), Ok(()));
        let mut hostile = e.clone();
        hostile.extension_fields.insert("SCRIPT".into());
        assert_eq!(
            admit_envelope(&hostile, ImportFormat::Qfx, 100, 10),
            Err(ImportError::Unsupported)
        )
    }
    #[test]
    fn repeated_and_reordered_records_are_byte_stable() {
        let record = ImportedRecord {
            record_id: "txn".into(),
            source_id: "file".into(),
            source_row: 1,
            account_id: "a".into(),
            provider_id: "p".into(),
            posted_date: "2026-01-01".into(),
            pending: false,
            amount_canonical: "USD:2:100".into(),
            description_sha256: "b".repeat(64),
        };
        assert_eq!(
            canonical_records(&[record.clone()]),
            canonical_records(&[record])
        )
    }
    #[test]
    fn unresolved_reconciliation_never_guesses() {
        let usd = Currency::new("USD").unwrap();
        let statement = StatementReconciliation {
            statement_id: "s".into(),
            account_id: "a".into(),
            opening: Money::new(0, usd.clone(), 2).unwrap(),
            closing: Money::new(100, usd.clone(), 2).unwrap(),
            included_record_ids: BTreeSet::new(),
            pending_exclusions: BTreeSet::new(),
            adjustment_ids: BTreeSet::new(),
            calculated_closing: Money::new(99, usd, 2).unwrap(),
            tolerance_minor_units: 0,
            disposition: UserDisposition::Unresolved,
            conflict: Some("missing record".into()),
        };
        assert_eq!(reconcile(&statement), Ok(false))
    }
}
