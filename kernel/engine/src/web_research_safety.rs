//! Bounded public-web retrieval, citations, quarantine, and disclosure refusal.
#![allow(missing_docs)]
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceState {
    Current,
    Stale,
    Conflicting,
    Malicious,
    Redirected,
    Archived,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceLabel {
    Observed,
    DeterministicDerivation,
    ModelInference,
    Conflict,
    Stale,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchRequest {
    pub query_digest: String,
    pub provider_id: String,
    pub recency_ms: u64,
    pub allowed_domains: BTreeSet<String>,
    pub schemes: BTreeSet<String>,
    pub dns_policy_id: String,
    pub proxy_id: Option<String>,
    pub certificate_policy_id: String,
    pub redirect_limit: u8,
    pub item_limit: u32,
    pub byte_limit: u64,
    pub media_types: BTreeSet<String>,
    pub scripts_allowed: bool,
    pub archives_allowed: bool,
    pub downloads_allowed: bool,
    pub cache_policy_id: String,
    pub timeout_ms: u64,
    pub cancellation_id: String,
    pub authenticated_browser_state_count: u32,
    pub workspace_handle_count: u32,
    pub connector_credential_count: u32,
    pub external_effect_tool_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Citation {
    pub query_digest: String,
    pub provider_id: String,
    pub direct_url: String,
    pub redirect_urls: Vec<String>,
    pub title_digest: String,
    pub retrieved_unix_ms: u64,
    pub publication_unix_ms: Option<u64>,
    pub event_unix_ms: Option<u64>,
    pub excerpt_sha256: String,
    pub source_class: String,
    pub cache_state: String,
    pub state: SourceState,
    pub primary: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundedClaim {
    pub claim_digest: String,
    pub citation_digests: BTreeSet<String>,
    pub label: EvidenceLabel,
    pub freshness_checked_unix_ms: u64,
    pub uncertainty_code: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisclosurePreview {
    pub destination: String,
    pub selected_field_digests: BTreeSet<String>,
    pub classification_id: String,
    pub approved: bool,
    pub approval_id: Option<String>,
    pub undeclared_egress_count: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadQuarantine {
    pub download_sha256: String,
    pub media_type: String,
    pub executable: bool,
    pub opened: bool,
    pub installed: bool,
    pub artifact_admission_id: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebSafetyError {
    Invalid,
    Authority,
    Uncited,
    Disclosure,
    Quarantine,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 4096
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_request(v: &ResearchRequest) -> Result<(), WebSafetyError> {
    if !digest(&v.query_digest)
        || !id(&v.provider_id)
        || v.recency_ms == 0
        || v.allowed_domains.is_empty()
        || v.schemes != BTreeSet::from(["https".into()])
        || !id(&v.dns_policy_id)
        || !id(&v.certificate_policy_id)
        || v.redirect_limit == 0
        || v.item_limit == 0
        || v.byte_limit == 0
        || v.media_types.is_empty()
        || v.scripts_allowed
        || v.archives_allowed
        || v.downloads_allowed
        || !id(&v.cache_policy_id)
        || v.timeout_ms == 0
        || !id(&v.cancellation_id)
    {
        return Err(WebSafetyError::Invalid);
    }
    if v.authenticated_browser_state_count != 0
        || v.workspace_handle_count != 0
        || v.connector_credential_count != 0
        || v.external_effect_tool_count != 0
    {
        return Err(WebSafetyError::Authority);
    }
    Ok(())
}
pub fn validate_citation(v: &Citation) -> Result<(), WebSafetyError> {
    if !digest(&v.query_digest)
        || !id(&v.provider_id)
        || !v.direct_url.starts_with("https://")
        || !digest(&v.title_digest)
        || v.retrieved_unix_ms == 0
        || !digest(&v.excerpt_sha256)
        || !id(&v.source_class)
        || !id(&v.cache_state)
    {
        return Err(WebSafetyError::Invalid);
    }
    Ok(())
}
pub fn validate_claim(v: &GroundedClaim) -> Result<(), WebSafetyError> {
    if !digest(&v.claim_digest)
        || v.citation_digests.is_empty()
        || v.citation_digests.iter().any(|x| !digest(x))
        || v.freshness_checked_unix_ms == 0
    {
        return Err(WebSafetyError::Uncited);
    }
    if matches!(
        v.label,
        EvidenceLabel::Conflict
            | EvidenceLabel::Stale
            | EvidenceLabel::Unknown
            | EvidenceLabel::ModelInference
    ) && v.uncertainty_code.as_deref().is_none_or(|x| !id(x))
    {
        return Err(WebSafetyError::Uncited);
    }
    Ok(())
}
pub fn admit_disclosure(v: &DisclosurePreview) -> Result<(), WebSafetyError> {
    if !id(&v.destination)
        || v.selected_field_digests.is_empty()
        || v.selected_field_digests.iter().any(|x| !digest(x))
        || !id(&v.classification_id)
        || !v.approved
        || v.approval_id.as_deref().is_none_or(|x| !id(x))
        || v.undeclared_egress_count != 0
    {
        return Err(WebSafetyError::Disclosure);
    }
    Ok(())
}
pub fn validate_quarantine(v: &DownloadQuarantine) -> Result<(), WebSafetyError> {
    if !digest(&v.download_sha256)
        || !id(&v.media_type)
        || v.executable
        || v.opened
        || v.installed
        || v.artifact_admission_id.is_some()
    {
        Err(WebSafetyError::Quarantine)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_worker_has_no_private_authority() {
        let v = ResearchRequest {
            query_digest: "a".repeat(64),
            provider_id: "search".into(),
            recency_ms: 1,
            allowed_domains: BTreeSet::from(["example.invalid".into()]),
            schemes: BTreeSet::from(["https".into()]),
            dns_policy_id: "dns".into(),
            proxy_id: None,
            certificate_policy_id: "cert".into(),
            redirect_limit: 3,
            item_limit: 10,
            byte_limit: 1000,
            media_types: BTreeSet::from(["text/html".into()]),
            scripts_allowed: false,
            archives_allowed: false,
            downloads_allowed: false,
            cache_policy_id: "cache".into(),
            timeout_ms: 1000,
            cancellation_id: "cancel".into(),
            authenticated_browser_state_count: 0,
            workspace_handle_count: 0,
            connector_credential_count: 0,
            external_effect_tool_count: 0,
        };
        assert_eq!(validate_request(&v), Ok(()))
    }
    #[test]
    fn downloads_stay_inert_until_separate_admission() {
        let v = DownloadQuarantine {
            download_sha256: "b".repeat(64),
            media_type: "application/octet-stream".into(),
            executable: false,
            opened: false,
            installed: false,
            artifact_admission_id: None,
        };
        assert_eq!(validate_quarantine(&v), Ok(()))
    }
    #[test]
    fn inference_requires_visible_uncertainty() {
        let v = GroundedClaim {
            claim_digest: "a".repeat(64),
            citation_digests: BTreeSet::from(["b".repeat(64)]),
            label: EvidenceLabel::ModelInference,
            freshness_checked_unix_ms: 1,
            uncertainty_code: None,
        };
        assert_eq!(validate_claim(&v), Err(WebSafetyError::Uncited))
    }
}
