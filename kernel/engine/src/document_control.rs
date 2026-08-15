//! Deterministic local document registers, quality findings, and exact action previews.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, DocumentActionApproval, DocumentActionKind, DocumentActionPreview,
    DocumentActionReview, DocumentAttachmentReviewState, DocumentControlFinding,
    DocumentControlFindingKind, DocumentLifecycleState, DocumentRegister, DocumentRegisterEntry,
    DocumentRegisterStatement, DocumentStatementClass, DocumentWorkflowKind,
    DocumentWorkflowReport, ExecutiveEvidenceState, ExecutiveSourceReference, MeetingFieldState,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 4_096;
const MAX_SHORT_TEXT_BYTES: usize = 512;
const MAX_TEXT_BYTES: usize = 256 * 1_024;
const MAX_ITEMS: usize = 4_096;

/// Stable fail-closed reason a document-control projection was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentControlError {
    /// An identity, path, date, hash, text, or collection is malformed.
    InvalidInput,
    /// Source, version, approval, or canonical ordering is inconsistent.
    IntegrityFailure,
    /// A supplied sealed register or preview does not match its digest.
    StaleRecord,
}

impl DocumentControlError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "document-control.input.invalid",
            Self::IntegrityFailure => "document-control.integrity.failed",
            Self::StaleRecord => "document-control.record.stale",
        }
    }
}

impl std::fmt::Display for DocumentControlError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DocumentControlError {}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a string cannot fail");
    }
    output
}

fn digest_record<T: Serialize>(value: &T) -> Result<String, DocumentControlError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| DocumentControlError::InvalidInput)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.is_ascii()
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.contains('\0')
        && !value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
        && value[5..7]
            .parse::<u8>()
            .is_ok_and(|month| (1..=12).contains(&month))
        && value[8..10]
            .parse::<u8>()
            .is_ok_and(|day| (1..=31).contains(&day))
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.starts_with('/')
        && !value.starts_with('~')
        && !value.contains('\0')
        && !value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
}

fn sorted_unique(values: &[String]) -> bool {
    values
        .windows(2)
        .all(|window| window[0].as_str() < window[1].as_str())
}

fn source_key(source: &ExecutiveSourceReference) -> (&str, &str, &str) {
    (
        source.source_id.as_str(),
        source.object_id.as_str(),
        source.fragment.as_deref().unwrap_or(""),
    )
}

fn validate_sources(
    sources: &[ExecutiveSourceReference],
) -> Result<BTreeSet<&str>, DocumentControlError> {
    if sources.is_empty()
        || sources.len() > MAX_ITEMS
        || !sources
            .windows(2)
            .all(|window| source_key(&window[0]) < source_key(&window[1]))
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    for source in sources {
        if !valid_identifier(&source.source_id)
            || !valid_identifier(&source.object_id)
            || !valid_sha256(&source.content_sha256)
            || source
                .observed_revision
                .as_deref()
                .is_some_and(|value| !valid_identifier(value))
            || source
                .fragment
                .as_deref()
                .is_some_and(|value| !valid_text(value, MAX_SHORT_TEXT_BYTES))
        {
            return Err(DocumentControlError::InvalidInput);
        }
    }
    Ok(sources
        .iter()
        .map(|source| source.source_id.as_str())
        .collect())
}

fn validate_optional_field(
    value: Option<&str>,
    state: MeetingFieldState,
    is_date: bool,
) -> Result<(), DocumentControlError> {
    if value.is_none() != (state == MeetingFieldState::Unknown) {
        return Err(DocumentControlError::IntegrityFailure);
    }
    if value.is_some_and(|value| {
        if is_date {
            !valid_date(value)
        } else {
            !valid_text(value, MAX_SHORT_TEXT_BYTES)
        }
    }) {
        return Err(DocumentControlError::InvalidInput);
    }
    Ok(())
}

fn validate_statement(
    statement: &DocumentRegisterStatement,
    available: &BTreeSet<&str>,
) -> Result<(), DocumentControlError> {
    if !valid_identifier(&statement.statement_id)
        || !valid_text(&statement.text, MAX_TEXT_BYTES)
        || statement.source_ids.is_empty()
        || !sorted_unique(&statement.source_ids)
        || statement
            .source_ids
            .iter()
            .any(|source_id| !available.contains(source_id.as_str()))
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    let confirmed_required = matches!(
        statement.class,
        DocumentStatementClass::VerbatimSource
            | DocumentStatementClass::ObservedFact
            | DocumentStatementClass::UserApprovedFinalLanguage
    );
    if confirmed_required && statement.evidence_state != ExecutiveEvidenceState::Confirmed
        || statement.class == DocumentStatementClass::InferredSummary
            && statement.evidence_state == ExecutiveEvidenceState::Confirmed
        || statement.class == DocumentStatementClass::UnresolvedConflict
            && statement.evidence_state != ExecutiveEvidenceState::Disputed
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    validate_optional_field(statement.owner.as_deref(), statement.owner_state, false)?;
    validate_optional_field(
        statement.due_date.as_deref(),
        statement.due_date_state,
        true,
    )
}

fn validate_entry(entry: &DocumentRegisterEntry) -> Result<(), DocumentControlError> {
    let available = validate_sources(&entry.sources)?;
    if !valid_identifier(&entry.record_id)
        || !valid_text(&entry.title, MAX_SHORT_TEXT_BYTES)
        || !valid_text(&entry.version, MAX_SHORT_TEXT_BYTES)
        || !valid_sha256(&entry.content_sha256)
        || !valid_relative_path(&entry.source_path)
        || entry
            .record_category
            .as_deref()
            .is_some_and(|value| !valid_text(value, MAX_SHORT_TEXT_BYTES))
        || entry
            .retention_schedule_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value))
        || entry
            .approval_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value))
        || entry
            .supersedes_record_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value) || value == entry.record_id)
        || entry
            .superseded_by_record_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value) || value == entry.record_id)
        || entry.attachments.len() > MAX_ITEMS
        || entry.commitments.len() > MAX_ITEMS
        || entry.deadlines.len() > MAX_ITEMS
        || entry.statements.len() > MAX_ITEMS
    {
        return Err(DocumentControlError::InvalidInput);
    }
    if matches!(
        entry.lifecycle_state,
        DocumentLifecycleState::Approved | DocumentLifecycleState::Final
    ) != entry.approval_id.is_some()
        || (entry.lifecycle_state == DocumentLifecycleState::Superseded)
            != entry.superseded_by_record_id.is_some()
        || entry.lifecycle_state == DocumentLifecycleState::Final
            && !entry
                .statements
                .iter()
                .any(|item| item.class == DocumentStatementClass::UserApprovedFinalLanguage)
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    if !entry
        .attachments
        .windows(2)
        .all(|window| window[0].attachment_id < window[1].attachment_id)
        || !entry
            .commitments
            .windows(2)
            .all(|window| window[0].statement_id < window[1].statement_id)
        || !entry
            .deadlines
            .windows(2)
            .all(|window| window[0].statement_id < window[1].statement_id)
        || !entry
            .statements
            .windows(2)
            .all(|window| window[0].statement_id < window[1].statement_id)
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    for attachment in &entry.attachments {
        if !valid_identifier(&attachment.attachment_id)
            || !valid_text(&attachment.filename, MAX_SHORT_TEXT_BYTES)
            || !valid_relative_path(&attachment.source_path)
            || !valid_sha256(&attachment.content_sha256)
            || attachment
                .approval_id
                .as_deref()
                .is_some_and(|value| !valid_identifier(value))
            || (attachment.review_state == DocumentAttachmentReviewState::Approved)
                != attachment.approval_id.is_some()
        {
            return Err(DocumentControlError::IntegrityFailure);
        }
    }
    for statement in entry
        .commitments
        .iter()
        .chain(&entry.deadlines)
        .chain(&entry.statements)
    {
        validate_statement(statement, &available)?;
    }
    Ok(())
}

fn validate_register(register: &DocumentRegister) -> Result<(), DocumentControlError> {
    if register.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&register.register_id)
        || register.entries.len() > MAX_ITEMS
        || !register.proposal_only
        || register.external_effects_performed
        || register.records_disposition_performed
        || !register
            .entries
            .windows(2)
            .all(|window| window[0].record_id < window[1].record_id)
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    for entry in &register.entries {
        validate_entry(entry)?;
    }
    let entries = register
        .entries
        .iter()
        .map(|entry| (entry.record_id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    for entry in &register.entries {
        if let Some(prior) = &entry.supersedes_record_id
            && entries.get(prior.as_str()).is_none_or(|candidate| {
                candidate.superseded_by_record_id.as_deref() != Some(&entry.record_id)
            })
        {
            return Err(DocumentControlError::IntegrityFailure);
        }
    }
    Ok(())
}

/// Validates and seals one local document and correspondence register.
pub fn seal_document_register(
    mut register: DocumentRegister,
) -> Result<DocumentRegister, DocumentControlError> {
    register.register_sha256.clear();
    validate_register(&register)?;
    register.register_sha256 = digest_record(&register)?;
    Ok(register)
}

/// Verifies an exact sealed document and correspondence register.
pub fn verify_document_register(register: &DocumentRegister) -> Result<(), DocumentControlError> {
    let supplied = register.register_sha256.clone();
    let mut candidate = register.clone();
    candidate.register_sha256.clear();
    validate_register(&candidate)?;
    if !valid_sha256(&supplied) || digest_record(&candidate)? != supplied {
        return Err(DocumentControlError::StaleRecord);
    }
    Ok(())
}

fn validate_preview(preview: &DocumentActionPreview) -> Result<(), DocumentControlError> {
    if preview.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&preview.preview_id)
        || !valid_identifier(&preview.record_id)
        || !valid_relative_path(&preview.destination_path)
        || preview
            .source_path
            .as_deref()
            .is_some_and(|value| !valid_relative_path(value))
        || preview
            .source_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || !valid_sha256(&preview.proposed_sha256)
        || !valid_sha256(&preview.metadata_sha256)
        || !valid_text(&preview.content_preview, MAX_TEXT_BYTES)
        || !valid_text(&preview.metadata_preview, MAX_TEXT_BYTES)
        || preview
            .record_category
            .as_deref()
            .is_some_and(|value| !valid_text(value, MAX_SHORT_TEXT_BYTES))
        || preview
            .retention_schedule_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value))
        || !preview.approval_required
        || preview.approval_granted
        || preview.effect_performed
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    let existing_required = !matches!(preview.action, DocumentActionKind::SaveDraft);
    if existing_required != preview.source_path.is_some()
        || existing_required != preview.source_sha256.is_some()
        || preview.action == DocumentActionKind::File
            && (preview.record_category.is_none() || preview.retention_schedule_id.is_none())
    {
        return Err(DocumentControlError::IntegrityFailure);
    }
    Ok(())
}

/// Validates and seals an exact local action preview without performing the action.
pub fn seal_document_action_preview(
    mut preview: DocumentActionPreview,
) -> Result<DocumentActionPreview, DocumentControlError> {
    preview.preview_sha256.clear();
    validate_preview(&preview)?;
    preview.preview_sha256 = digest_record(&preview)?;
    Ok(preview)
}

/// Verifies an exact sealed local action preview.
pub fn verify_document_action_preview(
    preview: &DocumentActionPreview,
) -> Result<(), DocumentControlError> {
    let supplied = preview.preview_sha256.clone();
    let mut candidate = preview.clone();
    candidate.preview_sha256.clear();
    validate_preview(&candidate)?;
    if !valid_sha256(&supplied) || digest_record(&candidate)? != supplied {
        return Err(DocumentControlError::StaleRecord);
    }
    Ok(())
}

/// Reviews exact destination, content, metadata, category, and retention confirmations.
pub fn review_document_action(
    preview: &DocumentActionPreview,
    approval: Option<&DocumentActionApproval>,
) -> Result<DocumentActionReview, DocumentControlError> {
    verify_document_action_preview(preview)?;
    let mut missing = Vec::new();
    let approval_id = if let Some(approval) = approval {
        if !valid_identifier(&approval.approval_id)
            || approval.preview_sha256 != preview.preview_sha256
        {
            return Err(DocumentControlError::IntegrityFailure);
        }
        if !approval.destination_confirmed {
            missing.push("approval.destination.missing".to_owned());
        }
        if !approval.content_confirmed {
            missing.push("approval.content.missing".to_owned());
        }
        if !approval.metadata_confirmed {
            missing.push("approval.metadata.missing".to_owned());
        }
        if preview.action == DocumentActionKind::File && !approval.record_category_confirmed {
            missing.push("approval.record-category.missing".to_owned());
        }
        if preview.action == DocumentActionKind::File && !approval.retention_confirmed {
            missing.push("approval.retention.missing".to_owned());
        }
        if !approval.approved {
            missing.push("approval.decision.denied".to_owned());
        }
        Some(approval.approval_id.clone())
    } else {
        missing.push("approval.record.missing".to_owned());
        None
    };
    missing.sort();
    Ok(DocumentActionReview {
        schema_version: CONTRACT_SCHEMA_VERSION,
        preview_sha256: preview.preview_sha256.clone(),
        approval_id,
        approval_complete: missing.is_empty(),
        missing_confirmation_codes: missing,
        execution_authority_created: false,
        effect_performed: false,
    })
}

/// Produces deterministic quality, duplicate, version, deadline, and filing-review findings.
pub fn review_document_register(
    register: &DocumentRegister,
) -> Result<Vec<DocumentControlFinding>, DocumentControlError> {
    verify_document_register(register)?;
    let mut findings = Vec::new();
    let mut digests: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for entry in &register.entries {
        digests
            .entry(&entry.content_sha256)
            .or_default()
            .push(&entry.record_id);
        let first_source = entry.sources[0].source_id.clone();
        let mut add = |kind, field: &str, reason: &str| {
            findings.push(DocumentControlFinding {
                finding_id: format!("finding:{}:{field}", entry.record_id),
                kind,
                record_id: entry.record_id.clone(),
                field_code: field.to_owned(),
                reason_code: reason.to_owned(),
                source_ids: vec![first_source.clone()],
            });
        };
        if entry.record_category.is_none() {
            add(
                DocumentControlFindingKind::FilingReviewRequired,
                "record_category",
                "record.category.unconfirmed",
            );
        }
        if entry.retention_schedule_id.is_none() {
            add(
                DocumentControlFindingKind::FilingReviewRequired,
                "retention_schedule",
                "record.retention.unconfirmed",
            );
        }
        if entry
            .attachments
            .iter()
            .any(|item| item.review_state != DocumentAttachmentReviewState::Approved)
        {
            add(
                DocumentControlFindingKind::AttachmentIssue,
                "attachments",
                "record.attachment.unapproved",
            );
        }
        if entry.deadlines.iter().any(|item| {
            item.due_date_state == MeetingFieldState::Unknown
                || item.evidence_state == ExecutiveEvidenceState::Disputed
        }) {
            add(
                DocumentControlFindingKind::DeadlineIssue,
                "deadlines",
                "record.deadline.unresolved",
            );
        }
    }
    for records in digests.values().filter(|records| records.len() > 1) {
        for record_id in records {
            let entry = register
                .entries
                .iter()
                .find(|entry| entry.record_id == *record_id)
                .expect("digest records came from entries");
            findings.push(DocumentControlFinding {
                finding_id: format!("finding:{record_id}:duplicate"),
                kind: DocumentControlFindingKind::DuplicateContent,
                record_id: (*record_id).to_owned(),
                field_code: "content_sha256".to_owned(),
                reason_code: "record.content.duplicate".to_owned(),
                source_ids: vec![entry.sources[0].source_id.clone()],
            });
        }
    }
    findings.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    Ok(findings)
}

/// Builds a local-only workflow report over an exact sealed register and preview set.
pub fn build_document_workflow_report(
    report_id: String,
    kind: DocumentWorkflowKind,
    register: &DocumentRegister,
    previews: &[DocumentActionPreview],
    rendered_preview: String,
) -> Result<DocumentWorkflowReport, DocumentControlError> {
    verify_document_register(register)?;
    if !valid_identifier(&report_id)
        || !valid_text(&rendered_preview, MAX_TEXT_BYTES)
        || previews.len() > MAX_ITEMS
        || !previews
            .windows(2)
            .all(|window| window[0].preview_id < window[1].preview_id)
    {
        return Err(DocumentControlError::InvalidInput);
    }
    for preview in previews {
        verify_document_action_preview(preview)?;
        if !register
            .entries
            .iter()
            .any(|entry| entry.record_id == preview.record_id)
        {
            return Err(DocumentControlError::IntegrityFailure);
        }
    }
    Ok(DocumentWorkflowReport {
        schema_version: CONTRACT_SCHEMA_VERSION,
        report_id,
        kind,
        register_sha256: register.register_sha256.clone(),
        findings: review_document_register(register)?,
        preview_sha256: previews
            .iter()
            .map(|preview| preview.preview_sha256.clone())
            .collect(),
        rendered_preview,
        local_preview_only: true,
        communication_effect_performed: false,
        calendar_effect_performed: false,
        filesystem_effect_performed: false,
        records_disposition_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{DocumentRegisterKind, ExecutiveSourceStore};

    use super::*;

    fn source() -> ExecutiveSourceReference {
        ExecutiveSourceReference {
            source_id: "source-1".to_owned(),
            object_id: "document-1".to_owned(),
            fragment: Some("Approved text".to_owned()),
            content_sha256: "a".repeat(64),
            observed_revision: Some("revision-1".to_owned()),
            store: ExecutiveSourceStore::PlainFolder,
        }
    }

    fn statement(id: &str, class: DocumentStatementClass) -> DocumentRegisterStatement {
        let evidence_state = match class {
            DocumentStatementClass::InferredSummary => ExecutiveEvidenceState::Inferred,
            DocumentStatementClass::UnresolvedConflict => ExecutiveEvidenceState::Disputed,
            _ => ExecutiveEvidenceState::Confirmed,
        };
        DocumentRegisterStatement {
            statement_id: id.to_owned(),
            class,
            text: "Exact source-backed statement.".to_owned(),
            evidence_state,
            owner: None,
            owner_state: MeetingFieldState::Unknown,
            due_date: None,
            due_date_state: MeetingFieldState::Unknown,
            source_ids: vec!["source-1".to_owned()],
        }
    }

    fn entry(id: &str, hash: char, state: DocumentLifecycleState) -> DocumentRegisterEntry {
        DocumentRegisterEntry {
            record_id: id.to_owned(),
            kind: DocumentRegisterKind::Document,
            title: "Controlled Record".to_owned(),
            record_category: None,
            version: "1.0".to_owned(),
            lifecycle_state: state,
            approval_id: matches!(
                state,
                DocumentLifecycleState::Approved | DocumentLifecycleState::Final
            )
            .then(|| "approval-1".to_owned()),
            attachments: vec![],
            commitments: vec![],
            deadlines: vec![statement(
                "deadline-1",
                DocumentStatementClass::ObservedFact,
            )],
            statements: if state == DocumentLifecycleState::Final {
                vec![statement(
                    "statement-1",
                    DocumentStatementClass::UserApprovedFinalLanguage,
                )]
            } else {
                vec![statement(
                    "statement-1",
                    DocumentStatementClass::ObservedFact,
                )]
            },
            content_sha256: hash.to_string().repeat(64),
            source_path: format!("records/{id}.md"),
            retention_schedule_id: None,
            supersedes_record_id: None,
            superseded_by_record_id: None,
            sources: vec![source()],
        }
    }

    fn register(entries: Vec<DocumentRegisterEntry>) -> DocumentRegister {
        DocumentRegister {
            schema_version: CONTRACT_SCHEMA_VERSION,
            register_id: "register-1".to_owned(),
            entries,
            register_sha256: String::new(),
            proposal_only: true,
            external_effects_performed: false,
            records_disposition_performed: false,
        }
    }

    fn preview(action: DocumentActionKind) -> DocumentActionPreview {
        let existing = !matches!(action, DocumentActionKind::SaveDraft);
        DocumentActionPreview {
            schema_version: CONTRACT_SCHEMA_VERSION,
            preview_id: "preview-1".to_owned(),
            action,
            record_id: "record-1".to_owned(),
            source_path: existing.then(|| "records/record-1.md".to_owned()),
            destination_path: "filed/record-1.md".to_owned(),
            source_sha256: existing.then(|| "a".repeat(64)),
            proposed_sha256: "b".repeat(64),
            metadata_sha256: "c".repeat(64),
            content_preview: "Exact proposed content.".to_owned(),
            metadata_preview: "category: correspondence".to_owned(),
            record_category: (action == DocumentActionKind::File)
                .then(|| "Correspondence".to_owned()),
            retention_schedule_id: (action == DocumentActionKind::File)
                .then(|| "schedule-1".to_owned()),
            preview_sha256: String::new(),
            approval_required: true,
            approval_granted: false,
            effect_performed: false,
        }
    }

    #[test]
    fn register_preserves_versions_sources_unknowns_and_zero_effects() {
        let sealed = seal_document_register(register(vec![entry(
            "record-1",
            'a',
            DocumentLifecycleState::Draft,
        )]))
        .expect("sealed register");
        verify_document_register(&sealed).expect("verified register");
        assert!(sealed.proposal_only);
        assert!(!sealed.external_effects_performed);
        assert!(!sealed.records_disposition_performed);
        assert_eq!(
            sealed.entries[0].deadlines[0].due_date_state,
            MeetingFieldState::Unknown
        );
    }

    #[test]
    fn final_copy_requires_exact_approval_and_approved_language() {
        let mut candidate = entry("record-1", 'a', DocumentLifecycleState::Final);
        candidate.approval_id = None;
        assert!(seal_document_register(register(vec![candidate])).is_err());
        let sealed = seal_document_register(register(vec![entry(
            "record-1",
            'a',
            DocumentLifecycleState::Final,
        )]));
        assert!(sealed.is_ok());
    }

    #[test]
    fn statement_classes_cannot_promote_inference_or_resolve_conflict() {
        let mut candidate = entry("record-1", 'a', DocumentLifecycleState::Draft);
        candidate.statements[0].class = DocumentStatementClass::InferredSummary;
        candidate.statements[0].evidence_state = ExecutiveEvidenceState::Confirmed;
        assert!(seal_document_register(register(vec![candidate])).is_err());
    }

    #[test]
    fn every_file_action_requires_exact_preview_and_separate_approval() {
        for action in [
            DocumentActionKind::SaveDraft,
            DocumentActionKind::Rename,
            DocumentActionKind::Move,
            DocumentActionKind::File,
        ] {
            let sealed = seal_document_action_preview(preview(action)).expect("preview");
            verify_document_action_preview(&sealed).expect("verified preview");
            let absent = review_document_action(&sealed, None).expect("missing approval");
            assert!(!absent.approval_complete);
            assert!(!absent.execution_authority_created);
            assert!(!absent.effect_performed);
            let approval = DocumentActionApproval {
                approval_id: "approval-1".to_owned(),
                preview_sha256: sealed.preview_sha256.clone(),
                destination_confirmed: true,
                content_confirmed: true,
                metadata_confirmed: true,
                record_category_confirmed: action == DocumentActionKind::File,
                retention_confirmed: action == DocumentActionKind::File,
                approved: true,
            };
            let reviewed = review_document_action(&sealed, Some(&approval)).expect("review");
            assert!(reviewed.approval_complete);
            assert!(!reviewed.execution_authority_created);
            assert!(!reviewed.effect_performed);
        }
    }

    #[test]
    fn filing_preview_requires_category_and_retention_confirmation() {
        let mut candidate = preview(DocumentActionKind::File);
        candidate.retention_schedule_id = None;
        assert!(seal_document_action_preview(candidate).is_err());
        let sealed =
            seal_document_action_preview(preview(DocumentActionKind::File)).expect("preview");
        let incomplete = DocumentActionApproval {
            approval_id: "approval-1".to_owned(),
            preview_sha256: sealed.preview_sha256.clone(),
            destination_confirmed: true,
            content_confirmed: true,
            metadata_confirmed: true,
            record_category_confirmed: false,
            retention_confirmed: false,
            approved: true,
        };
        let reviewed = review_document_action(&sealed, Some(&incomplete)).expect("review");
        assert!(!reviewed.approval_complete);
        assert_eq!(reviewed.missing_confirmation_codes.len(), 2);
    }

    #[test]
    fn quality_review_detects_duplicate_filing_and_deadline_gaps() {
        let sealed = seal_document_register(register(vec![
            entry("record-1", 'a', DocumentLifecycleState::Draft),
            entry("record-2", 'a', DocumentLifecycleState::Draft),
        ]))
        .expect("register");
        let findings = review_document_register(&sealed).expect("review");
        assert_eq!(
            findings
                .iter()
                .filter(|item| item.kind == DocumentControlFindingKind::DuplicateContent)
                .count(),
            2
        );
        assert!(
            findings
                .iter()
                .any(|item| item.field_code == "record_category")
        );
        assert!(
            findings
                .iter()
                .any(|item| item.field_code == "retention_schedule")
        );
        assert!(findings.iter().any(|item| item.field_code == "deadlines"));
    }

    #[test]
    fn all_workflow_reports_are_local_only_and_effect_free() {
        let sealed_register = seal_document_register(register(vec![entry(
            "record-1",
            'a',
            DocumentLifecycleState::Draft,
        )]))
        .expect("register");
        let sealed_preview =
            seal_document_action_preview(preview(DocumentActionKind::SaveDraft)).expect("preview");
        for (index, kind) in [
            DocumentWorkflowKind::Naming,
            DocumentWorkflowKind::Duplicate,
            DocumentWorkflowKind::Superseded,
            DocumentWorkflowKind::FinalCopy,
            DocumentWorkflowKind::Quality,
            DocumentWorkflowKind::Deadline,
            DocumentWorkflowKind::RoutingSlip,
            DocumentWorkflowKind::MailMergePreview,
            DocumentWorkflowKind::CalendarFileDraft,
            DocumentWorkflowKind::FilingSuggestion,
        ]
        .into_iter()
        .enumerate()
        {
            let report = build_document_workflow_report(
                format!("report-{index}"),
                kind,
                &sealed_register,
                std::slice::from_ref(&sealed_preview),
                "Exact local-only rendered preview.".to_owned(),
            )
            .expect("report");
            assert!(report.local_preview_only);
            assert!(!report.communication_effect_performed);
            assert!(!report.calendar_effect_performed);
            assert!(!report.filesystem_effect_performed);
            assert!(!report.records_disposition_performed);
        }
    }

    #[test]
    fn stale_register_or_preview_is_rejected() {
        let mut sealed = seal_document_register(register(vec![entry(
            "record-1",
            'a',
            DocumentLifecycleState::Draft,
        )]))
        .expect("register");
        sealed.entries[0].title.push_str(" changed");
        assert_eq!(
            verify_document_register(&sealed).expect_err("stale"),
            DocumentControlError::StaleRecord
        );
    }
}
