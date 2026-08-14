//! Exact-preview redacted evidence bundles derived from canonical conversation state.

use std::collections::BTreeSet;
use std::path::Path;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ConversationId, ConversationTurnId, ConversationTurnRole,
    DataSensitivity, MaterialClaimEvidenceStateKind, ReceiptId, StrictLocalStorageObservation,
    to_canonical_json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::conversation_library::{ConversationHistory, ConversationLibraryError};
use crate::operational_store::{OperationalStore, write_derived_export};
use crate::persistence::detect_secret_classes;
use crate::strict_local::{StrictLocalStorageDecision, evaluate_storage};

const MAX_BUNDLE_ID_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_EXCERPT_BYTES: usize = 64 * 1024;
const MAX_LIST_ITEMS: usize = 512;
const MAX_BUNDLE_BYTES: usize = 4 * 1024 * 1024;
const REDACTED: &str = "[REDACTED]";

/// Stable content-free evidence-bundle failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceBundleError {
    /// A draft, identifier, range, list, digest, preview, or approval is malformed.
    InvalidInput,
    /// A requested conversation or turn is unavailable.
    NotFound,
    /// Canonical conversation state or the exact preview changed before publication.
    StaleReview,
    /// Secret-like material was detected in content proposed for disclosure.
    SecretDetected,
    /// Hidden, restricted, unrelated, or otherwise unapproved content was selected.
    ProhibitedContent,
    /// The selected destination is not admitted strict-local storage.
    StorageRejected,
    /// An exact private local file could not be published atomically.
    PublicationFailed,
    /// A canonical record or source relationship failed verification.
    IntegrityFailure,
}

impl EvidenceBundleError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "evidence_bundle.input.invalid",
            Self::NotFound => "evidence_bundle.source.not_found",
            Self::StaleReview => "evidence_bundle.review.stale",
            Self::SecretDetected => "evidence_bundle.content.secret_detected",
            Self::ProhibitedContent => "evidence_bundle.content.prohibited",
            Self::StorageRejected => "evidence_bundle.storage.rejected",
            Self::PublicationFailed => "evidence_bundle.publication.failed",
            Self::IntegrityFailure => "evidence_bundle.integrity.failed",
        }
    }
}

impl std::fmt::Display for EvidenceBundleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for EvidenceBundleError {}

/// Closed disposition for one explicitly selected source range.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceExcerptDisposition {
    /// Include the exact canonical UTF-8 range after policy and secret checks.
    Include,
    /// Disclose only that the selected range was removed for named policy reasons.
    Redact,
}

/// One user-reviewed byte range from a canonical immutable turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceExcerptSelection {
    /// Exact immutable turn identity.
    pub turn_id: ConversationTurnId,
    /// Inclusive UTF-8 byte offset in canonical retained turn text.
    pub start_byte: u64,
    /// Exclusive UTF-8 byte offset in canonical retained turn text.
    pub end_byte: u64,
    /// Whether the exact range may be included or must be replaced.
    pub disposition: EvidenceExcerptDisposition,
    /// Stable policy codes explaining a redaction; empty only for included ranges.
    pub redaction_codes: Vec<String>,
    /// Explicit user decision that this exact range may be represented in the preview.
    pub user_approved: bool,
    /// Explicit reviewer decision that the range is related to the bundle purpose.
    pub related: bool,
}

/// One user-visible claim and its closed evidence state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvidenceBundleClaim {
    /// Stable bundle-local claim identity.
    pub claim_id: String,
    /// Exact bounded claim statement.
    pub statement: String,
    /// Closed evidence state; unknown and blocked remain visible.
    pub evidence_state: MaterialClaimEvidenceStateKind,
    /// Citation identities supporting this claim, all drawn from selected turns.
    pub citation_ids: Vec<String>,
}

/// One deterministic or explicitly identified method disclosed in the bundle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvidenceBundleMethod {
    /// Stable bundle-local method identity.
    pub method_id: String,
    /// Exact bounded method description.
    pub description: String,
    /// Digest of the method implementation, configuration, or result record.
    pub method_sha256: String,
}

/// Complete local request for one redacted portable evidence bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBundleDraft {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable evidence-bundle identity.
    pub bundle_id: String,
    /// Exact canonical conversation selected by the user.
    pub conversation_id: ConversationId,
    /// Digest of the complete immutable source history reviewed by the user.
    pub expected_source_history_sha256: String,
    /// Digest of the governing disclosure policy.
    pub policy_sha256: String,
    /// Digest of the governing redaction policy.
    pub redaction_policy_sha256: String,
    /// Exact model manifest used for any disclosed model-derived work.
    pub model_manifest_sha256: String,
    /// Trusted creation time as Unix epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// User-reviewed claim records.
    pub claims: Vec<EvidenceBundleClaim>,
    /// User-reviewed method records.
    pub methods: Vec<EvidenceBundleMethod>,
    /// Ordered authority, interpretation, and continuation constraints.
    pub constraints: Vec<String>,
    /// Exact canonical source ranges selected for inclusion or redaction.
    pub excerpts: Vec<EvidenceExcerptSelection>,
    /// Visible categories intentionally excluded from the portable artifact.
    pub exclusions: Vec<String>,
}

/// Resolved, content-addressed excerpt disclosed by an evidence bundle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvidenceBundleExcerpt {
    /// Exact immutable source turn.
    pub turn_id: ConversationTurnId,
    /// Source turn role.
    pub role: ConversationTurnRole,
    /// Source turn sensitivity.
    pub sensitivity: DataSensitivity,
    /// Inclusive source UTF-8 byte offset.
    pub start_byte: u64,
    /// Exclusive source UTF-8 byte offset.
    pub end_byte: u64,
    /// Digest of the complete original source text.
    pub source_text_sha256: String,
    /// Exact included text or the fixed redaction marker.
    pub excerpt: String,
    /// Inclusion or redaction disposition.
    pub disposition: EvidenceExcerptDisposition,
    /// Visible stable redaction reasons.
    pub redaction_codes: Vec<String>,
}

/// Deterministic non-executable portable evidence bundle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PortableEvidenceBundle {
    /// Fixed record type.
    pub record_type: String,
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable evidence-bundle identity.
    pub bundle_id: String,
    /// Exact source conversation identity.
    pub conversation_id: ConversationId,
    /// Digest of complete immutable source history at review time.
    pub source_history_sha256: String,
    /// Governing disclosure-policy digest.
    pub policy_sha256: String,
    /// Governing redaction-policy digest.
    pub redaction_policy_sha256: String,
    /// Exact model-manifest digest.
    pub model_manifest_sha256: String,
    /// Trusted creation time.
    pub created_at_epoch_ms: u64,
    /// Reviewed claims and evidence states.
    pub claims: Vec<EvidenceBundleClaim>,
    /// Reviewed methods.
    pub methods: Vec<EvidenceBundleMethod>,
    /// Reviewed constraints.
    pub constraints: Vec<String>,
    /// Exact included or explicitly redacted excerpts.
    pub excerpts: Vec<EvidenceBundleExcerpt>,
    /// Sorted citation identities derived from selected canonical turns.
    pub citation_ids: Vec<String>,
    /// Sorted receipt identities derived from selected canonical turns.
    pub receipt_ids: Vec<ReceiptId>,
    /// Sorted source hashes derived from selected canonical turns.
    pub source_sha256: Vec<String>,
    /// Visible exclusion and omission statements.
    pub exclusions: Vec<String>,
    /// Fixed false marker: bundle contents cannot execute.
    pub executable: bool,
    /// Fixed false marker: the bundle is never startup or operational authority.
    pub startup_authority: bool,
    /// Fixed false marker: construction and local publication perform no external delivery.
    pub external_delivery_attempted: bool,
    /// Digest of every preceding bundle field.
    pub bundle_sha256: String,
}

/// Mandatory exact-byte disclosure preview before local evidence publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBundlePreview {
    /// Stable preview identity.
    pub preview_id: String,
    /// Complete portable record shown to the user.
    pub bundle: PortableEvidenceBundle,
    /// Exact canonical JSON bytes that would be published.
    pub exact_json: Vec<u8>,
    /// SHA-256 of exact published bytes.
    pub exact_json_sha256: String,
    /// Exact published byte count.
    pub exact_json_bytes: u64,
    /// Number of exact source excerpts included.
    pub included_excerpt_count: u64,
    /// Number of source ranges replaced by explicit redaction markers.
    pub redacted_excerpt_count: u64,
    /// Logical review expiry as Unix epoch milliseconds.
    pub expires_at_epoch_ms: u64,
    /// Digest binding every preview field.
    pub preview_sha256: String,
    /// Fixed false marker: preview construction publishes nothing.
    pub published: bool,
}

/// Explicit user approval bound to one exact evidence disclosure preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBundleApproval {
    /// Stable approval identity.
    pub approval_id: String,
    /// Exact preview digest approved by the user.
    pub approved_preview_sha256: String,
    /// Digest of explicit user decision evidence.
    pub decision_sha256: String,
    /// Trusted approval time.
    pub approved_at_epoch_ms: u64,
    /// Explicit confirmation marker; false approvals are inert.
    pub user_confirmed: bool,
}

/// Content-free receipt for one exact private local evidence-bundle publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBundlePublicationReceipt {
    /// Stable evidence-bundle identity.
    pub bundle_id: String,
    /// Exact approved preview digest.
    pub preview_sha256: String,
    /// SHA-256 of exact published bytes.
    pub published_sha256: String,
    /// Exact published byte count.
    pub published_bytes: u64,
    /// Digest of the approval identity.
    pub approval_id_sha256: String,
    /// Fixed true marker after atomic create-new publication.
    pub published: bool,
    /// Fixed false marker: no external delivery occurs.
    pub external_delivery_attempted: bool,
}

struct ResolvedEvidenceExcerpts {
    excerpts: Vec<EvidenceBundleExcerpt>,
    citation_ids: Vec<String>,
    receipt_ids: Vec<ReceiptId>,
    source_sha256: Vec<String>,
}

impl OperationalStore {
    /// Builds the exact no-write disclosure preview for one current evidence draft.
    pub fn preview_evidence_bundle(
        &self,
        draft: &EvidenceBundleDraft,
        preview_id: String,
        expires_at_epoch_ms: u64,
    ) -> Result<EvidenceBundlePreview, EvidenceBundleError> {
        validate_draft(draft)?;
        if !valid_identifier(&preview_id) || expires_at_epoch_ms <= draft.created_at_epoch_ms {
            return Err(EvidenceBundleError::InvalidInput);
        }
        let history = self
            .conversation_history(&draft.conversation_id)
            .map_err(map_conversation_error)?;
        let source_history_sha256 = history_digest(&history)?;
        if source_history_sha256 != draft.expected_source_history_sha256 {
            return Err(EvidenceBundleError::StaleReview);
        }
        let resolved = resolve_excerpts(&history, &draft.excerpts)?;
        validate_claim_citations(&draft.claims, &resolved.citation_ids)?;
        let mut bundle = PortableEvidenceBundle {
            record_type: "agentmage.portable_evidence_bundle.v1".to_owned(),
            schema_version: CONTRACT_SCHEMA_VERSION,
            bundle_id: draft.bundle_id.clone(),
            conversation_id: draft.conversation_id.clone(),
            source_history_sha256,
            policy_sha256: draft.policy_sha256.clone(),
            redaction_policy_sha256: draft.redaction_policy_sha256.clone(),
            model_manifest_sha256: draft.model_manifest_sha256.clone(),
            created_at_epoch_ms: draft.created_at_epoch_ms,
            claims: draft.claims.clone(),
            methods: draft.methods.clone(),
            constraints: draft.constraints.clone(),
            excerpts: resolved.excerpts,
            citation_ids: resolved.citation_ids,
            receipt_ids: resolved.receipt_ids,
            source_sha256: resolved.source_sha256,
            exclusions: draft.exclusions.clone(),
            executable: false,
            startup_authority: false,
            external_delivery_attempted: false,
            bundle_sha256: String::new(),
        };
        bundle.bundle_sha256 = portable_bundle_digest(&bundle)?;
        let exact_json =
            serde_json::to_vec(&bundle).map_err(|_| EvidenceBundleError::InvalidInput)?;
        if exact_json.is_empty() || exact_json.len() > MAX_BUNDLE_BYTES {
            return Err(EvidenceBundleError::InvalidInput);
        }
        if !detect_secret_classes("portable_evidence_bundle", &exact_json).is_empty() {
            return Err(EvidenceBundleError::SecretDetected);
        }
        let exact_json_sha256 = sha256(&exact_json);
        let exact_json_bytes = exact_json.len() as u64;
        let included_excerpt_count = bundle
            .excerpts
            .iter()
            .filter(|entry| entry.disposition == EvidenceExcerptDisposition::Include)
            .count() as u64;
        let redacted_excerpt_count = bundle.excerpts.len() as u64 - included_excerpt_count;
        let preview_sha256 = digest(&(
            "agentmage-evidence-bundle-preview-v1",
            &preview_id,
            &bundle.bundle_sha256,
            &exact_json_sha256,
            exact_json.len() as u64,
            included_excerpt_count,
            redacted_excerpt_count,
            expires_at_epoch_ms,
        ))?;
        Ok(EvidenceBundlePreview {
            preview_id,
            bundle,
            exact_json,
            exact_json_sha256,
            exact_json_bytes,
            included_excerpt_count,
            redacted_excerpt_count,
            expires_at_epoch_ms,
            preview_sha256,
            published: false,
        })
    }

    /// Revalidates canonical state and atomically publishes exactly the approved bytes.
    pub fn publish_evidence_bundle(
        &self,
        destination: &Path,
        observation: &StrictLocalStorageObservation,
        draft: &EvidenceBundleDraft,
        preview: &EvidenceBundlePreview,
        approval: &EvidenceBundleApproval,
        now_epoch_ms: u64,
    ) -> Result<EvidenceBundlePublicationReceipt, EvidenceBundleError> {
        if evaluate_storage(observation) != StrictLocalStorageDecision::Eligible {
            return Err(EvidenceBundleError::StorageRejected);
        }
        if now_epoch_ms == 0
            || now_epoch_ms >= preview.expires_at_epoch_ms
            || !valid_identifier(&approval.approval_id)
            || approval.approved_preview_sha256 != preview.preview_sha256
            || !valid_sha256(&approval.decision_sha256)
            || approval.approved_at_epoch_ms == 0
            || approval.approved_at_epoch_ms > now_epoch_ms
            || !approval.user_confirmed
        {
            return Err(EvidenceBundleError::StaleReview);
        }
        let fresh = self.preview_evidence_bundle(
            draft,
            preview.preview_id.clone(),
            preview.expires_at_epoch_ms,
        )?;
        if &fresh != preview {
            return Err(EvidenceBundleError::StaleReview);
        }
        write_derived_export(destination, &preview.exact_json)
            .map_err(|_| EvidenceBundleError::PublicationFailed)?;
        Ok(EvidenceBundlePublicationReceipt {
            bundle_id: preview.bundle.bundle_id.clone(),
            preview_sha256: preview.preview_sha256.clone(),
            published_sha256: preview.exact_json_sha256.clone(),
            published_bytes: preview.exact_json_bytes,
            approval_id_sha256: sha256(approval.approval_id.as_bytes()),
            published: true,
            external_delivery_attempted: false,
        })
    }
}

/// Computes the current complete immutable history digest used to construct a draft.
pub fn conversation_evidence_history_sha256(
    history: &ConversationHistory,
) -> Result<String, EvidenceBundleError> {
    history_digest(history)
}

fn resolve_excerpts(
    history: &ConversationHistory,
    selections: &[EvidenceExcerptSelection],
) -> Result<ResolvedEvidenceExcerpts, EvidenceBundleError> {
    let mut previous: Option<(&str, u64, u64)> = None;
    let mut excerpts = Vec::with_capacity(selections.len());
    let mut citation_ids = BTreeSet::new();
    let mut receipt_ids = BTreeSet::new();
    let mut source_sha256 = BTreeSet::new();
    for selection in selections {
        let current = (
            selection.turn_id.as_str(),
            selection.start_byte,
            selection.end_byte,
        );
        if previous.is_some_and(|value| value >= current) {
            return Err(EvidenceBundleError::InvalidInput);
        }
        previous = Some(current);
        let turn = history
            .turns
            .iter()
            .find(|turn| turn.turn_id == selection.turn_id)
            .ok_or(EvidenceBundleError::NotFound)?;
        if !selection.user_approved
            || !selection.related
            || turn.role == ConversationTurnRole::System
        {
            return Err(EvidenceBundleError::ProhibitedContent);
        }
        let text = turn
            .text
            .as_deref()
            .ok_or(EvidenceBundleError::ProhibitedContent)?;
        let start =
            usize::try_from(selection.start_byte).map_err(|_| EvidenceBundleError::InvalidInput)?;
        let end =
            usize::try_from(selection.end_byte).map_err(|_| EvidenceBundleError::InvalidInput)?;
        if start >= end
            || end > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
            || end - start > MAX_EXCERPT_BYTES
        {
            return Err(EvidenceBundleError::InvalidInput);
        }
        let source = &text[start..end];
        let excerpt = match selection.disposition {
            EvidenceExcerptDisposition::Include => {
                if turn.sensitivity == DataSensitivity::Restricted
                    || !selection.redaction_codes.is_empty()
                {
                    return Err(EvidenceBundleError::ProhibitedContent);
                }
                if !detect_secret_classes("evidence_excerpt", source.as_bytes()).is_empty() {
                    return Err(EvidenceBundleError::SecretDetected);
                }
                source.to_owned()
            }
            EvidenceExcerptDisposition::Redact => {
                if selection.redaction_codes.is_empty()
                    || selection
                        .redaction_codes
                        .iter()
                        .any(|code| !valid_code(code))
                {
                    return Err(EvidenceBundleError::InvalidInput);
                }
                REDACTED.to_owned()
            }
        };
        citation_ids.extend(turn.citation_ids.iter().cloned());
        receipt_ids.extend(turn.receipt_ids.iter().cloned());
        source_sha256.extend(turn.source_sha256.iter().cloned());
        excerpts.push(EvidenceBundleExcerpt {
            turn_id: turn.turn_id.clone(),
            role: turn.role,
            sensitivity: turn.sensitivity,
            start_byte: selection.start_byte,
            end_byte: selection.end_byte,
            source_text_sha256: turn.text_sha256.clone(),
            excerpt,
            disposition: selection.disposition,
            redaction_codes: selection.redaction_codes.clone(),
        });
    }
    Ok(ResolvedEvidenceExcerpts {
        excerpts,
        citation_ids: citation_ids.into_iter().collect(),
        receipt_ids: receipt_ids.into_iter().collect(),
        source_sha256: source_sha256.into_iter().collect(),
    })
}

fn validate_draft(draft: &EvidenceBundleDraft) -> Result<(), EvidenceBundleError> {
    if draft.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&draft.bundle_id)
        || !valid_identifier(draft.conversation_id.as_str())
        || !valid_sha256(&draft.expected_source_history_sha256)
        || !valid_sha256(&draft.policy_sha256)
        || !valid_sha256(&draft.redaction_policy_sha256)
        || !valid_sha256(&draft.model_manifest_sha256)
        || draft.created_at_epoch_ms == 0
        || draft.claims.is_empty()
        || draft.claims.len() > MAX_LIST_ITEMS
        || draft.methods.is_empty()
        || draft.methods.len() > MAX_LIST_ITEMS
        || draft.constraints.is_empty()
        || draft.constraints.len() > MAX_LIST_ITEMS
        || draft.excerpts.is_empty()
        || draft.excerpts.len() > MAX_LIST_ITEMS
        || draft.exclusions.len() > MAX_LIST_ITEMS
    {
        return Err(EvidenceBundleError::InvalidInput);
    }
    validate_claims(&draft.claims)?;
    validate_methods(&draft.methods)?;
    validate_safe_text_list("constraint", &draft.constraints)?;
    validate_safe_text_list("exclusion", &draft.exclusions)?;
    Ok(())
}

fn validate_claims(claims: &[EvidenceBundleClaim]) -> Result<(), EvidenceBundleError> {
    let mut identities = BTreeSet::new();
    for claim in claims {
        if !valid_identifier(&claim.claim_id)
            || !valid_text(&claim.statement)
            || !identities.insert(claim.claim_id.as_str())
            || claim.citation_ids.len() > MAX_LIST_ITEMS
            || claim.citation_ids.windows(2).any(|pair| pair[0] >= pair[1])
            || claim
                .citation_ids
                .iter()
                .any(|value| !valid_identifier(value))
        {
            return Err(EvidenceBundleError::InvalidInput);
        }
        if !detect_secret_classes("claim", claim.statement.as_bytes()).is_empty() {
            return Err(EvidenceBundleError::SecretDetected);
        }
    }
    Ok(())
}

fn validate_methods(methods: &[EvidenceBundleMethod]) -> Result<(), EvidenceBundleError> {
    let mut identities = BTreeSet::new();
    for method in methods {
        if !valid_identifier(&method.method_id)
            || !valid_text(&method.description)
            || !valid_sha256(&method.method_sha256)
            || !identities.insert(method.method_id.as_str())
        {
            return Err(EvidenceBundleError::InvalidInput);
        }
        if !detect_secret_classes("method", method.description.as_bytes()).is_empty() {
            return Err(EvidenceBundleError::SecretDetected);
        }
    }
    Ok(())
}

fn validate_safe_text_list(name: &str, values: &[String]) -> Result<(), EvidenceBundleError> {
    if values.iter().any(|value| !valid_text(value)) {
        return Err(EvidenceBundleError::InvalidInput);
    }
    if values
        .iter()
        .any(|value| !detect_secret_classes(name, value.as_bytes()).is_empty())
    {
        return Err(EvidenceBundleError::SecretDetected);
    }
    Ok(())
}

fn validate_claim_citations(
    claims: &[EvidenceBundleClaim],
    selected: &[String],
) -> Result<(), EvidenceBundleError> {
    let selected = selected.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if claims
        .iter()
        .flat_map(|claim| &claim.citation_ids)
        .any(|citation| !selected.contains(citation.as_str()))
    {
        return Err(EvidenceBundleError::ProhibitedContent);
    }
    Ok(())
}

fn history_digest(history: &ConversationHistory) -> Result<String, EvidenceBundleError> {
    if !history.read_only {
        return Err(EvidenceBundleError::IntegrityFailure);
    }
    let mut digest = Sha256::new();
    digest.update(b"agentmage-evidence-bundle-history-v1\n");
    digest.update(
        to_canonical_json(&history.conversation)
            .map_err(|_| EvidenceBundleError::IntegrityFailure)?,
    );
    for turn in &history.turns {
        digest.update(to_canonical_json(turn).map_err(|_| EvidenceBundleError::IntegrityFailure)?);
    }
    for compaction in &history.compactions {
        digest.update(
            to_canonical_json(compaction).map_err(|_| EvidenceBundleError::IntegrityFailure)?,
        );
    }
    Ok(hex_digest(&digest.finalize()))
}

fn portable_bundle_digest(bundle: &PortableEvidenceBundle) -> Result<String, EvidenceBundleError> {
    #[derive(Serialize)]
    struct Unsigned<'a> {
        digest_domain: &'static str,
        record_type: &'a str,
        schema_version: u16,
        bundle_id: &'a str,
        conversation_id: &'a ConversationId,
        source_history_sha256: &'a str,
        policy_sha256: &'a str,
        redaction_policy_sha256: &'a str,
        model_manifest_sha256: &'a str,
        created_at_epoch_ms: u64,
        claims: &'a [EvidenceBundleClaim],
        methods: &'a [EvidenceBundleMethod],
        constraints: &'a [String],
        excerpts: &'a [EvidenceBundleExcerpt],
        citation_ids: &'a [String],
        receipt_ids: &'a [ReceiptId],
        source_sha256: &'a [String],
        exclusions: &'a [String],
        executable: bool,
        startup_authority: bool,
        external_delivery_attempted: bool,
    }
    digest(&Unsigned {
        digest_domain: "agentmage-portable-evidence-bundle-v1",
        record_type: &bundle.record_type,
        schema_version: bundle.schema_version,
        bundle_id: &bundle.bundle_id,
        conversation_id: &bundle.conversation_id,
        source_history_sha256: &bundle.source_history_sha256,
        policy_sha256: &bundle.policy_sha256,
        redaction_policy_sha256: &bundle.redaction_policy_sha256,
        model_manifest_sha256: &bundle.model_manifest_sha256,
        created_at_epoch_ms: bundle.created_at_epoch_ms,
        claims: &bundle.claims,
        methods: &bundle.methods,
        constraints: &bundle.constraints,
        excerpts: &bundle.excerpts,
        citation_ids: &bundle.citation_ids,
        receipt_ids: &bundle.receipt_ids,
        source_sha256: &bundle.source_sha256,
        exclusions: &bundle.exclusions,
        executable: bundle.executable,
        startup_authority: bundle.startup_authority,
        external_delivery_attempted: bundle.external_delivery_attempted,
    })
}

fn digest<T: Serialize>(value: &T) -> Result<String, EvidenceBundleError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| EvidenceBundleError::InvalidInput)
}

fn sha256(value: &[u8]) -> String {
    hex_digest(&Sha256::digest(value))
}

fn hex_digest(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_BUNDLE_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_BUNDLE_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(|character| character == '\0')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn map_conversation_error(error: ConversationLibraryError) -> EvidenceBundleError {
    match error {
        ConversationLibraryError::NotFound => EvidenceBundleError::NotFound,
        ConversationLibraryError::IntegrityFailure => EvidenceBundleError::IntegrityFailure,
        _ => EvidenceBundleError::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        CloudSynchronizationMarker, ConversationRecord, ConversationRetention,
        ConversationRetentionKind, ConversationStatus, ConversationTurn, DataSensitivity,
        ModelProfileId, StorageFilesystemClass, WorkspaceId,
    };

    use super::*;
    use crate::operational_store::{OperationalStoreKeyError, OperationalStoreKeyProvider};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);
    const CREATED: u64 = 1_800_100_000_000;
    const SAFE_TEXT: &str = "The local verification completed successfully.";
    const SECRET_TEXT: &str = "authorization: bearer synthetic-private-value";
    const SYSTEM_TEXT: &str = "hidden system instruction must never leave";
    const RESTRICTED_TEXT: &str = "restricted private source text";

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None::<CloudSynchronizationMarker>,
            root_identity_sha256: [41; 32],
            symlink_free: true,
        }
    }

    fn fixture() -> (std::path::PathBuf, OperationalStore, ConversationRecord) {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "agentmage-evidence-bundle-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("temporary directory");
        let mut store = OperationalStore::open(
            &directory.join("canonical.db"),
            &observation(),
            &mut TestKey([61; 32]),
        )
        .expect("encrypted store");
        let conversation = ConversationRecord {
            schema_version: CONTRACT_SCHEMA_VERSION,
            conversation_id: ConversationId::from_raw("conversation-bundle-alpha"),
            title: "Evidence bundle fixture".to_owned(),
            sensitivity: DataSensitivity::Operational,
            created_at_epoch_ms: CREATED,
            updated_at_epoch_ms: CREATED,
            local_date: "2027-01-16".to_owned(),
            local_timezone: "America/Chicago".to_owned(),
            workspace_id: WorkspaceId::from_raw("workspace-bundle-alpha"),
            project_id: Some("project-bundle-alpha".to_owned()),
            model_profile_id: ModelProfileId::from_raw("model-local-alpha"),
            status: ConversationStatus::Active,
            parent_conversation_id: None,
            branch_from_turn_id: None,
            current_turn_id: None,
            tags: vec!["evidence".to_owned()],
            retention: ConversationRetention {
                kind: ConversationRetentionKind::Session,
                expires_at_epoch_ms: Some(CREATED + 86_400_000),
                policy_sha256: "a".repeat(64),
            },
            pinned: false,
            persistence_enabled: true,
        };
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        for (ordinal, role, sensitivity, text) in [
            (
                1,
                ConversationTurnRole::User,
                DataSensitivity::Operational,
                SAFE_TEXT,
            ),
            (
                2,
                ConversationTurnRole::Tool,
                DataSensitivity::Operational,
                SECRET_TEXT,
            ),
            (
                3,
                ConversationTurnRole::System,
                DataSensitivity::Operational,
                SYSTEM_TEXT,
            ),
            (
                4,
                ConversationTurnRole::User,
                DataSensitivity::Restricted,
                RESTRICTED_TEXT,
            ),
        ] {
            store
                .append_conversation_turn(&turn(ordinal, role, sensitivity, text))
                .expect("turn appends");
        }
        (directory, store, conversation)
    }

    fn turn(
        ordinal: u64,
        role: ConversationTurnRole,
        sensitivity: DataSensitivity,
        text: &str,
    ) -> ConversationTurn {
        ConversationTurn {
            schema_version: CONTRACT_SCHEMA_VERSION,
            turn_id: ConversationTurnId::from_raw(format!("turn-bundle-{ordinal}")),
            conversation_id: ConversationId::from_raw("conversation-bundle-alpha"),
            ordinal,
            role,
            sensitivity,
            created_at_epoch_ms: CREATED + ordinal,
            local_date: "2027-01-16".to_owned(),
            text: Some(text.to_owned()),
            text_sha256: sha256(text.as_bytes()),
            attachments: Vec::new(),
            grant_ids: Vec::new(),
            receipt_ids: vec![ReceiptId::from_raw(format!("receipt-bundle-{ordinal}"))],
            checkpoint_id: None,
            citation_ids: vec![format!("citation-bundle-{ordinal}")],
            source_sha256: vec![format!("{ordinal:x}").repeat(64)],
        }
    }

    fn draft(store: &OperationalStore, conversation: &ConversationRecord) -> EvidenceBundleDraft {
        let history = store
            .conversation_history(&conversation.conversation_id)
            .expect("history");
        EvidenceBundleDraft {
            schema_version: CONTRACT_SCHEMA_VERSION,
            bundle_id: "evidence-bundle-alpha".to_owned(),
            conversation_id: conversation.conversation_id.clone(),
            expected_source_history_sha256: conversation_evidence_history_sha256(&history)
                .expect("history digest"),
            policy_sha256: "b".repeat(64),
            redaction_policy_sha256: "c".repeat(64),
            model_manifest_sha256: "d".repeat(64),
            created_at_epoch_ms: CREATED + 10,
            claims: vec![EvidenceBundleClaim {
                claim_id: "claim-alpha".to_owned(),
                statement: "The selected local verification completed.".to_owned(),
                evidence_state: MaterialClaimEvidenceStateKind::Observed,
                citation_ids: vec!["citation-bundle-1".to_owned()],
            }],
            methods: vec![EvidenceBundleMethod {
                method_id: "method-alpha".to_owned(),
                description: "Deterministic local verification".to_owned(),
                method_sha256: "e".repeat(64),
            }],
            constraints: vec!["Treat this bundle as non-authoritative evidence.".to_owned()],
            excerpts: vec![
                EvidenceExcerptSelection {
                    turn_id: ConversationTurnId::from_raw("turn-bundle-1"),
                    start_byte: 0,
                    end_byte: SAFE_TEXT.len() as u64,
                    disposition: EvidenceExcerptDisposition::Include,
                    redaction_codes: Vec::new(),
                    user_approved: true,
                    related: true,
                },
                EvidenceExcerptSelection {
                    turn_id: ConversationTurnId::from_raw("turn-bundle-2"),
                    start_byte: 0,
                    end_byte: SECRET_TEXT.len() as u64,
                    disposition: EvidenceExcerptDisposition::Redact,
                    redaction_codes: vec!["credential.removed".to_owned()],
                    user_approved: true,
                    related: true,
                },
            ],
            exclusions: vec![
                "Unselected conversation text".to_owned(),
                "Hidden instructions and credentials".to_owned(),
            ],
        }
    }

    fn approval(preview: &EvidenceBundlePreview) -> EvidenceBundleApproval {
        EvidenceBundleApproval {
            approval_id: "approval-bundle-alpha".to_owned(),
            approved_preview_sha256: preview.preview_sha256.clone(),
            decision_sha256: "f".repeat(64),
            approved_at_epoch_ms: CREATED + 11,
            user_confirmed: true,
        }
    }

    #[test]
    fn exact_preview_publishes_only_selected_and_redacted_content() {
        let (directory, store, conversation) = fixture();
        let destination = directory.join("evidence-bundle.json");
        let draft = draft(&store, &conversation);
        let preview = store
            .preview_evidence_bundle(&draft, "preview-bundle-alpha".to_owned(), CREATED + 100)
            .expect("preview builds");
        assert_eq!(preview.included_excerpt_count, 1);
        assert_eq!(preview.redacted_excerpt_count, 1);
        let visible = String::from_utf8(preview.exact_json.clone()).expect("UTF-8 bundle");
        assert!(visible.contains(SAFE_TEXT));
        assert!(visible.contains(REDACTED));
        for prohibited in [SECRET_TEXT, SYSTEM_TEXT, RESTRICTED_TEXT] {
            assert!(!visible.contains(prohibited));
        }
        assert_eq!(
            preview.bundle.receipt_ids,
            vec![
                ReceiptId::from_raw("receipt-bundle-1"),
                ReceiptId::from_raw("receipt-bundle-2"),
            ]
        );
        assert_eq!(
            preview.bundle.citation_ids,
            vec![
                "citation-bundle-1".to_owned(),
                "citation-bundle-2".to_owned(),
            ]
        );
        let receipt = store
            .publish_evidence_bundle(
                &destination,
                &observation(),
                &draft,
                &preview,
                &approval(&preview),
                CREATED + 12,
            )
            .expect("bundle publishes");
        assert!(receipt.published);
        assert!(!receipt.external_delivery_attempted);
        assert_eq!(
            fs::read(&destination).expect("published bytes"),
            preview.exact_json
        );
        assert_eq!(
            fs::symlink_metadata(&destination)
                .expect("private metadata")
                .mode()
                & 0o077,
            0
        );
        assert!(
            store
                .publish_evidence_bundle(
                    &destination,
                    &observation(),
                    &draft,
                    &preview,
                    &approval(&preview),
                    CREATED + 12,
                )
                .is_err()
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn secret_hidden_and_restricted_inclusions_fail_closed() {
        let (directory, store, conversation) = fixture();
        let base = draft(&store, &conversation);
        let cases = [
            (
                "turn-bundle-2",
                SECRET_TEXT.len(),
                EvidenceBundleError::SecretDetected,
            ),
            (
                "turn-bundle-3",
                SYSTEM_TEXT.len(),
                EvidenceBundleError::ProhibitedContent,
            ),
            (
                "turn-bundle-4",
                RESTRICTED_TEXT.len(),
                EvidenceBundleError::ProhibitedContent,
            ),
        ];
        for (turn_id, text_len, expected) in cases {
            let mut candidate = base.clone();
            candidate.excerpts = vec![EvidenceExcerptSelection {
                turn_id: ConversationTurnId::from_raw(turn_id),
                start_byte: 0,
                end_byte: text_len as u64,
                disposition: EvidenceExcerptDisposition::Include,
                redaction_codes: Vec::new(),
                user_approved: true,
                related: true,
            }];
            candidate.claims[0].citation_ids.clear();
            assert_eq!(
                store
                    .preview_evidence_bundle(
                        &candidate,
                        format!("preview-{turn_id}"),
                        CREATED + 100,
                    )
                    .expect_err("prohibited disclosure must fail"),
                expected
            );
        }
        for mutate in [
            |selection: &mut EvidenceExcerptSelection| selection.user_approved = false,
            |selection: &mut EvidenceExcerptSelection| selection.related = false,
        ] {
            let mut candidate = base.clone();
            mutate(&mut candidate.excerpts[0]);
            assert_eq!(
                store
                    .preview_evidence_bundle(
                        &candidate,
                        "preview-unapproved".to_owned(),
                        CREATED + 100,
                    )
                    .expect_err("unapproved or unrelated content must fail"),
                EvidenceBundleError::ProhibitedContent
            );
        }
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn stale_history_preview_and_unselected_citations_cannot_publish() {
        let (directory, mut store, conversation) = fixture();
        let destination = directory.join("stale.json");
        let mut candidate = draft(&store, &conversation);
        candidate.claims[0]
            .citation_ids
            .push("citation-not-selected".to_owned());
        assert_eq!(
            store
                .preview_evidence_bundle(
                    &candidate,
                    "preview-invalid-citation".to_owned(),
                    CREATED + 100,
                )
                .expect_err("unselected citation must fail"),
            EvidenceBundleError::ProhibitedContent
        );

        let current = draft(&store, &conversation);
        let preview = store
            .preview_evidence_bundle(&current, "preview-before-change".to_owned(), CREATED + 100)
            .expect("preview builds");
        store
            .append_conversation_turn(&turn(
                5,
                ConversationTurnRole::Assistant,
                DataSensitivity::Operational,
                "Later canonical state",
            ))
            .expect("history changes");
        assert_eq!(
            store
                .publish_evidence_bundle(
                    &destination,
                    &observation(),
                    &current,
                    &preview,
                    &approval(&preview),
                    CREATED + 12,
                )
                .expect_err("stale source must fail"),
            EvidenceBundleError::StaleReview
        );
        assert!(!destination.exists());
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn expired_unconfirmed_and_nonlocal_publication_attempts_have_no_side_effect() {
        let (directory, store, conversation) = fixture();
        let destination = directory.join("denied.json");
        let draft = draft(&store, &conversation);
        let preview = store
            .preview_evidence_bundle(&draft, "preview-denied".to_owned(), CREATED + 100)
            .expect("preview builds");
        let mut denied = approval(&preview);
        denied.user_confirmed = false;
        assert_eq!(
            store
                .publish_evidence_bundle(
                    &destination,
                    &observation(),
                    &draft,
                    &preview,
                    &denied,
                    CREATED + 12,
                )
                .expect_err("confirmation required"),
            EvidenceBundleError::StaleReview
        );
        assert_eq!(
            store
                .publish_evidence_bundle(
                    &destination,
                    &observation(),
                    &draft,
                    &preview,
                    &approval(&preview),
                    CREATED + 100,
                )
                .expect_err("expired preview must fail"),
            EvidenceBundleError::StaleReview
        );
        let remote = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Remote,
            ..observation()
        };
        assert_eq!(
            store
                .publish_evidence_bundle(
                    &destination,
                    &remote,
                    &draft,
                    &preview,
                    &approval(&preview),
                    CREATED + 12,
                )
                .expect_err("remote destination must fail"),
            EvidenceBundleError::StorageRejected
        );
        assert!(!destination.exists());
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
