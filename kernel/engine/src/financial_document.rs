//! Source-preserving financial-document extraction, matching, and privacy contracts.
#![allow(missing_docs)]

use crate::financial_domain::{Money, UserDisposition};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancialDocumentKind {
    Receipt,
    Invoice,
    Reimbursement,
    Statement,
    TaxDocument,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancialFieldKind {
    Amount,
    Tax,
    Currency,
    Date,
    Merchant,
    AccountHint,
    DocumentType,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataCopyKind {
    Original,
    Derived,
    Index,
    Cache,
    LinkedRecord,
    Backup,
    Export,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivacyClassification {
    FinancialConfidential,
    TaxRestricted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRegion {
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractedFinancialField {
    pub field_id: String,
    pub kind: FinancialFieldKind,
    pub value_sha256: String,
    pub source_sha256: String,
    pub region: SourceRegion,
    pub parser_id: String,
    pub parser_version: u32,
    pub confidence_basis_points: u16,
    pub amount: Option<Money>,
    pub unresolved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialDocument {
    pub document_id: String,
    pub kind: FinancialDocumentKind,
    pub source_file_id: String,
    pub source_sha256: String,
    pub byte_length: u64,
    pub media_type: String,
    pub fields: Vec<ExtractedFinancialField>,
    pub classification: PrivacyClassification,
    pub retention_policy_id: String,
    pub original_preserved: bool,
    pub encrypted_at_rest: bool,
    pub hostile_content_observed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentMatchCandidate {
    pub candidate_id: String,
    pub document_id: String,
    pub transaction_id: String,
    pub evidence_field_ids: BTreeSet<String>,
    pub reason_codes: BTreeSet<String>,
    pub confidence_basis_points: u16,
    pub disposition: UserDisposition,
    pub records_remain_distinct: bool,
    pub reversible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateCandidate {
    pub candidate_id: String,
    pub left_document_id: String,
    pub right_document_id: String,
    pub exact_source_duplicate: bool,
    pub semantic_evidence: BTreeSet<String>,
    pub disposition: UserDisposition,
    pub versions_preserved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyCopyDisposition {
    pub document_id: String,
    pub copy_kind: DataCopyKind,
    pub classification: PrivacyClassification,
    pub policy_id: String,
    pub encrypted: bool,
    pub minimized: bool,
    pub redacted: bool,
    pub deleted: bool,
    pub residue_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialDocumentExport {
    pub document_id: String,
    pub destination_id: String,
    pub policy_id: String,
    pub approval_id: String,
    pub classification: PrivacyClassification,
    pub redacted: bool,
    pub external_effect: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancialDocumentError {
    Invalid,
    Ambiguous,
    UntrustedContent,
    PrivacyMismatch,
    UnsupportedEffect,
}

fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn validate_document(document: &FinancialDocument) -> Result<(), FinancialDocumentError> {
    if !identifier(&document.document_id)
        || !identifier(&document.source_file_id)
        || !digest(&document.source_sha256)
        || document.byte_length == 0
        || !identifier(&document.media_type)
        || document.fields.is_empty()
        || !identifier(&document.retention_policy_id)
        || !document.original_preserved
        || !document.encrypted_at_rest
    {
        return Err(FinancialDocumentError::Invalid);
    }
    if document.hostile_content_observed {
        return Err(FinancialDocumentError::UntrustedContent);
    }
    let mut identities = BTreeSet::new();
    for field in &document.fields {
        if !identifier(&field.field_id)
            || !identities.insert(field.field_id.clone())
            || !digest(&field.value_sha256)
            || field.source_sha256 != document.source_sha256
            || field.region.page == 0
            || field.region.width == 0
            || field.region.height == 0
            || !identifier(&field.parser_id)
            || field.parser_version == 0
            || field.confidence_basis_points > 10_000
            || (field.amount.is_some()
                && !matches!(
                    field.kind,
                    FinancialFieldKind::Amount | FinancialFieldKind::Tax
                ))
        {
            return Err(FinancialDocumentError::Invalid);
        }
        if field.confidence_basis_points < 10_000 && !field.unresolved {
            return Err(FinancialDocumentError::Ambiguous);
        }
    }
    Ok(())
}

pub fn validate_match(candidate: &DocumentMatchCandidate) -> Result<(), FinancialDocumentError> {
    if !identifier(&candidate.candidate_id)
        || !identifier(&candidate.document_id)
        || !identifier(&candidate.transaction_id)
        || candidate.evidence_field_ids.is_empty()
        || candidate.reason_codes.is_empty()
        || candidate.confidence_basis_points > 10_000
        || !candidate.records_remain_distinct
        || !candidate.reversible
    {
        return Err(FinancialDocumentError::Invalid);
    }
    if candidate.confidence_basis_points < 10_000
        && candidate.disposition == UserDisposition::Accepted
    {
        return Err(FinancialDocumentError::Ambiguous);
    }
    Ok(())
}

pub fn validate_duplicate(candidate: &DuplicateCandidate) -> Result<(), FinancialDocumentError> {
    if !identifier(&candidate.candidate_id)
        || !identifier(&candidate.left_document_id)
        || !identifier(&candidate.right_document_id)
        || candidate.left_document_id == candidate.right_document_id
        || (!candidate.exact_source_duplicate && candidate.semantic_evidence.is_empty())
        || !candidate.versions_preserved
    {
        return Err(FinancialDocumentError::Invalid);
    }
    Ok(())
}

pub fn reconcile_privacy_copies(
    copies: &[PrivacyCopyDisposition],
    deleting: bool,
) -> Result<(), FinancialDocumentError> {
    let expected = BTreeSet::from([
        DataCopyKind::Original,
        DataCopyKind::Derived,
        DataCopyKind::Index,
        DataCopyKind::Cache,
        DataCopyKind::LinkedRecord,
        DataCopyKind::Backup,
        DataCopyKind::Export,
    ]);
    let actual: BTreeSet<_> = copies.iter().map(|copy| copy.copy_kind).collect();
    let first = copies
        .first()
        .ok_or(FinancialDocumentError::PrivacyMismatch)?;
    if actual != expected
        || copies.iter().any(|copy| {
            copy.document_id != first.document_id
                || copy.classification != first.classification
                || copy.policy_id != first.policy_id
                || !copy.encrypted
                || !copy.minimized
                || (deleting && (!copy.deleted || copy.residue_count != 0))
        })
    {
        return Err(FinancialDocumentError::PrivacyMismatch);
    }
    Ok(())
}

pub fn validate_export(export: &FinancialDocumentExport) -> Result<(), FinancialDocumentError> {
    if !identifier(&export.document_id)
        || !identifier(&export.destination_id)
        || !identifier(&export.policy_id)
        || !identifier(&export.approval_id)
        || !export.redacted
        || export.external_effect
    {
        return Err(FinancialDocumentError::UnsupportedEffect);
    }
    Ok(())
}

pub fn reject_hostile_document(_bytes: &[u8]) -> Result<(), FinancialDocumentError> {
    Err(FinancialDocumentError::UntrustedContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> FinancialDocument {
        FinancialDocument {
            document_id: "document".into(),
            kind: FinancialDocumentKind::Receipt,
            source_file_id: "file".into(),
            source_sha256: "a".repeat(64),
            byte_length: 100,
            media_type: "application/pdf".into(),
            fields: vec![ExtractedFinancialField {
                field_id: "amount".into(),
                kind: FinancialFieldKind::Amount,
                value_sha256: "b".repeat(64),
                source_sha256: "a".repeat(64),
                region: SourceRegion {
                    page: 1,
                    x: 1,
                    y: 1,
                    width: 10,
                    height: 10,
                },
                parser_id: "bounded-pdf".into(),
                parser_version: 1,
                confidence_basis_points: 9_000,
                amount: None,
                unresolved: true,
            }],
            classification: PrivacyClassification::FinancialConfidential,
            retention_policy_id: "financial-v1".into(),
            original_preserved: true,
            encrypted_at_rest: true,
            hostile_content_observed: false,
        }
    }
    #[test]
    fn extraction_preserves_source_and_uncertainty() {
        assert_eq!(validate_document(&document()), Ok(()));
        let mut changed = document();
        changed.fields[0].source_sha256 = "c".repeat(64);
        assert_eq!(
            validate_document(&changed),
            Err(FinancialDocumentError::Invalid)
        );
        let mut guessed = document();
        guessed.fields[0].unresolved = false;
        assert_eq!(
            validate_document(&guessed),
            Err(FinancialDocumentError::Ambiguous)
        );
    }
    #[test]
    fn ambiguous_matches_remain_distinct() {
        let candidate = DocumentMatchCandidate {
            candidate_id: "candidate".into(),
            document_id: "document".into(),
            transaction_id: "transaction".into(),
            evidence_field_ids: BTreeSet::from(["amount".into()]),
            reason_codes: BTreeSet::from(["amount-and-date".into()]),
            confidence_basis_points: 8_000,
            disposition: UserDisposition::Unresolved,
            records_remain_distinct: true,
            reversible: true,
        };
        assert_eq!(validate_match(&candidate), Ok(()));
        let mut merged = candidate;
        merged.records_remain_distinct = false;
        assert_eq!(
            validate_match(&merged),
            Err(FinancialDocumentError::Invalid)
        );
    }
    #[test]
    fn deletion_reconciles_every_copy_without_residue() {
        let copies: Vec<_> = [
            DataCopyKind::Original,
            DataCopyKind::Derived,
            DataCopyKind::Index,
            DataCopyKind::Cache,
            DataCopyKind::LinkedRecord,
            DataCopyKind::Backup,
            DataCopyKind::Export,
        ]
        .into_iter()
        .map(|copy_kind| PrivacyCopyDisposition {
            document_id: "document".into(),
            copy_kind,
            classification: PrivacyClassification::FinancialConfidential,
            policy_id: "financial-v1".into(),
            encrypted: true,
            minimized: true,
            redacted: true,
            deleted: true,
            residue_count: 0,
        })
        .collect();
        assert_eq!(reconcile_privacy_copies(&copies, true), Ok(()));
        let mut residue = copies;
        residue[3].residue_count = 1;
        assert_eq!(
            reconcile_privacy_copies(&residue, true),
            Err(FinancialDocumentError::PrivacyMismatch)
        );
    }
    #[test]
    fn hostile_content_and_external_export_are_rejected() {
        assert_eq!(
            reject_hostile_document(b"macro"),
            Err(FinancialDocumentError::UntrustedContent)
        );
        let export = FinancialDocumentExport {
            document_id: "document".into(),
            destination_id: "folder".into(),
            policy_id: "financial-v1".into(),
            approval_id: "approval".into(),
            classification: PrivacyClassification::FinancialConfidential,
            redacted: true,
            external_effect: false,
        };
        assert_eq!(validate_export(&export), Ok(()));
        let mut effect = export;
        effect.external_effect = true;
        assert_eq!(
            validate_export(&effect),
            Err(FinancialDocumentError::UnsupportedEffect)
        );
    }
}
