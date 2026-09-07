//! Inert sandboxed-browser action planning and result verification.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_DOMAINS: usize = 32;
const MAX_SELECTOR_BYTES: usize = 1_024;
const MAX_RESULT_BYTES: u64 = 8 * 1_048_576;
const MAX_DOWNLOAD_BYTES: u64 = 64 * 1_048_576;

/// Closed visible browser action family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserInspectionAction {
    /// Navigate to one exact public HTTPS URL.
    Navigate,
    /// Inspect the current rendered document without mutation.
    Inspect,
    /// Find bounded visible text in the current document.
    Find,
    /// Click one exact visible target after a dedicated grant.
    Click,
    /// Capture one redacted visible viewport.
    Screenshot,
    /// Download one bounded response into quarantine.
    Download,
}

/// Visible browser-profile separation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserProfileClass {
    /// Ephemeral public profile with no credential state.
    PublicEphemeral,
    /// Explicit user-selected authenticated profile whose secrets stay brokered.
    AuthenticatedBrokered,
}

/// Exact action proposed to a separately owned sandboxed browser.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserInspectionRequest {
    /// Stable action identity.
    pub action_id: String,
    /// Exact protected browser session identity.
    pub session_id: String,
    /// Visible profile class.
    pub profile_class: BrowserProfileClass,
    /// Digest-only broker reference; required only for an authenticated profile.
    pub brokered_profile_sha256: Option<String>,
    /// Closed action kind.
    pub action: BrowserInspectionAction,
    /// Exact HTTPS URL for navigate or download.
    pub url: Option<String>,
    /// Sorted domain allowlist for the session.
    pub allowed_domains: Vec<String>,
    /// Visible selector or find text when required.
    pub selector_or_query: Option<String>,
    /// Digest of the exact single-use action grant.
    pub action_grant_sha256: String,
    /// Maximum returned inspection bytes.
    pub max_result_bytes: u64,
    /// Maximum download bytes, zero for non-download actions.
    pub max_download_bytes: u64,
    /// Whether screenshot and inspection output must be redacted.
    pub redaction_required: bool,
}

/// Content-free plan suitable for visible preview before execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserActionPreview {
    /// Exact action identity.
    pub action_id: String,
    /// Exact session identity.
    pub session_id: String,
    /// Closed action kind.
    pub action: BrowserInspectionAction,
    /// Destination hostname, never URL path/query content.
    pub destination_host: Option<String>,
    /// Digest of the selector/query when present.
    pub selector_or_query_sha256: Option<String>,
    /// Digest of the exact grant.
    pub action_grant_sha256: String,
    /// Whether the result requires redaction.
    pub redaction_required: bool,
    /// Whether any downloaded bytes must enter quarantine.
    pub quarantine_required: bool,
}

/// Untrusted outcome observation returned by the browser adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserResultObservation {
    /// Exact action identity.
    pub action_id: String,
    /// Exact session identity.
    pub session_id: String,
    /// Number of result bytes before redaction.
    pub result_bytes: u64,
    /// Number of downloaded bytes, zero for non-download actions.
    pub downloaded_bytes: u64,
    /// True only when output was redacted before retention or model delivery.
    pub redaction_applied: bool,
    /// True only when downloaded bytes entered quarantine.
    pub quarantined: bool,
    /// Digest of the content-minimized action trail.
    pub action_trail_sha256: String,
    /// True only after owned browser work and descendants terminated.
    pub terminated: bool,
}

/// Stable fail-closed planning or verification reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserInspectionError {
    /// Identity, profile, domain, URL, or action shape is invalid.
    InvalidRequest,
    /// Result bounds or required controls were not satisfied.
    ResultDenied,
}

/// Produces a content-free action preview without opening a browser.
pub fn plan_browser_inspection(
    request: &BrowserInspectionRequest,
) -> Result<BrowserActionPreview, BrowserInspectionError> {
    if !valid_id(&request.action_id)
        || !valid_id(&request.session_id)
        || !valid_sha256(&request.action_grant_sha256)
        || request.allowed_domains.is_empty()
        || request.allowed_domains.len() > MAX_DOMAINS
        || !strictly_sorted(&request.allowed_domains)
        || request
            .allowed_domains
            .iter()
            .any(|domain| !valid_domain(domain))
        || request.max_result_bytes == 0
        || request.max_result_bytes > MAX_RESULT_BYTES
        || !profile_valid(request)
        || !action_shape_valid(request)
    {
        return Err(BrowserInspectionError::InvalidRequest);
    }
    let destination_host = request
        .url
        .as_deref()
        .map(|url| allowed_host(url, &request.allowed_domains))
        .transpose()?
        .map(str::to_owned);
    Ok(BrowserActionPreview {
        action_id: request.action_id.clone(),
        session_id: request.session_id.clone(),
        action: request.action,
        destination_host,
        selector_or_query_sha256: request
            .selector_or_query
            .as_deref()
            .map(|value| sha256(value.as_bytes())),
        action_grant_sha256: request.action_grant_sha256.clone(),
        redaction_required: request.redaction_required,
        quarantine_required: request.action == BrowserInspectionAction::Download,
    })
}

/// Verifies one terminal observation without retaining browser content or secrets.
pub fn verify_browser_result(
    request: &BrowserInspectionRequest,
    preview: &BrowserActionPreview,
    observation: &BrowserResultObservation,
) -> Result<(), BrowserInspectionError> {
    if plan_browser_inspection(request)? != *preview
        || observation.action_id != request.action_id
        || observation.session_id != request.session_id
        || observation.result_bytes > request.max_result_bytes
        || observation.downloaded_bytes > request.max_download_bytes
        || request.redaction_required != observation.redaction_applied
        || preview.quarantine_required != observation.quarantined
        || !valid_sha256(&observation.action_trail_sha256)
        || !observation.terminated
    {
        return Err(BrowserInspectionError::ResultDenied);
    }
    Ok(())
}

fn profile_valid(request: &BrowserInspectionRequest) -> bool {
    match request.profile_class {
        BrowserProfileClass::PublicEphemeral => request.brokered_profile_sha256.is_none(),
        BrowserProfileClass::AuthenticatedBrokered => request
            .brokered_profile_sha256
            .as_deref()
            .is_some_and(valid_sha256),
    }
}

fn action_shape_valid(request: &BrowserInspectionRequest) -> bool {
    let selector_valid = request
        .selector_or_query
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty() && value.len() <= MAX_SELECTOR_BYTES);
    match request.action {
        BrowserInspectionAction::Navigate => {
            request.url.is_some()
                && request.selector_or_query.is_none()
                && request.max_download_bytes == 0
        }
        BrowserInspectionAction::Inspect | BrowserInspectionAction::Screenshot => {
            request.url.is_none()
                && request.selector_or_query.is_none()
                && request.max_download_bytes == 0
                && request.redaction_required
        }
        BrowserInspectionAction::Find | BrowserInspectionAction::Click => {
            request.url.is_none() && selector_valid && request.max_download_bytes == 0
        }
        BrowserInspectionAction::Download => {
            request.url.is_some()
                && request.selector_or_query.is_none()
                && request.max_download_bytes > 0
                && request.max_download_bytes <= MAX_DOWNLOAD_BYTES
        }
    }
}

fn allowed_host<'a>(url: &'a str, domains: &[String]) -> Result<&'a str, BrowserInspectionError> {
    let rest = url
        .strip_prefix("https://")
        .ok_or(BrowserInspectionError::InvalidRequest)?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if host.is_empty()
        || url.contains('@')
        || url.bytes().any(|byte| byte.is_ascii_control())
        || !domains
            .iter()
            .any(|domain| host == domain || host.ends_with(&format!(".{domain}")))
    {
        return Err(BrowserInspectionError::InvalidRequest);
    }
    Ok(host)
}

fn valid_domain(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn strictly_sorted(values: &[String]) -> bool {
    let set = values.iter().collect::<BTreeSet<_>>();
    set.len() == values.len() && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(action: BrowserInspectionAction) -> BrowserInspectionRequest {
        BrowserInspectionRequest {
            action_id: "browser-action-1".to_owned(),
            session_id: "browser-session-1".to_owned(),
            profile_class: BrowserProfileClass::PublicEphemeral,
            brokered_profile_sha256: None,
            action,
            url: matches!(
                action,
                BrowserInspectionAction::Navigate | BrowserInspectionAction::Download
            )
            .then(|| "https://docs.example.gov/record".to_owned()),
            allowed_domains: vec!["example.gov".to_owned()],
            selector_or_query: matches!(
                action,
                BrowserInspectionAction::Find | BrowserInspectionAction::Click
            )
            .then(|| "visible target".to_owned()),
            action_grant_sha256: "a".repeat(64),
            max_result_bytes: 4_096,
            max_download_bytes: if action == BrowserInspectionAction::Download {
                8_192
            } else {
                0
            },
            redaction_required: matches!(
                action,
                BrowserInspectionAction::Inspect | BrowserInspectionAction::Screenshot
            ),
        }
    }

    #[test]
    fn sprint_83_all_actions_produce_visible_content_free_previews() {
        for action in [
            BrowserInspectionAction::Navigate,
            BrowserInspectionAction::Inspect,
            BrowserInspectionAction::Find,
            BrowserInspectionAction::Click,
            BrowserInspectionAction::Screenshot,
            BrowserInspectionAction::Download,
        ] {
            let preview = plan_browser_inspection(&request(action)).expect("preview");
            assert_eq!(preview.action, action);
            assert_eq!(
                preview.quarantine_required,
                action == BrowserInspectionAction::Download
            );
        }
    }

    #[test]
    fn sprint_83_profiles_urls_domains_and_action_shapes_fail_closed() {
        let mut public_secret = request(BrowserInspectionAction::Navigate);
        public_secret.brokered_profile_sha256 = Some("b".repeat(64));
        assert_eq!(
            plan_browser_inspection(&public_secret),
            Err(BrowserInspectionError::InvalidRequest)
        );
        let mut off_domain = request(BrowserInspectionAction::Navigate);
        off_domain.url = Some("https://example.com/".to_owned());
        assert_eq!(
            plan_browser_inspection(&off_domain),
            Err(BrowserInspectionError::InvalidRequest)
        );
        let mut missing_grant = request(BrowserInspectionAction::Click);
        missing_grant.action_grant_sha256.clear();
        assert_eq!(
            plan_browser_inspection(&missing_grant),
            Err(BrowserInspectionError::InvalidRequest)
        );
    }

    #[test]
    fn sprint_83_authenticated_profile_retains_only_broker_digest() {
        let mut authenticated = request(BrowserInspectionAction::Inspect);
        authenticated.profile_class = BrowserProfileClass::AuthenticatedBrokered;
        authenticated.brokered_profile_sha256 = Some("b".repeat(64));
        assert!(plan_browser_inspection(&authenticated).is_ok());
    }

    #[test]
    fn sprint_83_result_requires_redaction_quarantine_termination_and_bounds() {
        for action in [
            BrowserInspectionAction::Screenshot,
            BrowserInspectionAction::Download,
        ] {
            let request = request(action);
            let preview = plan_browser_inspection(&request).expect("preview");
            let mut result = BrowserResultObservation {
                action_id: request.action_id.clone(),
                session_id: request.session_id.clone(),
                result_bytes: 100,
                downloaded_bytes: if action == BrowserInspectionAction::Download {
                    100
                } else {
                    0
                },
                redaction_applied: request.redaction_required,
                quarantined: preview.quarantine_required,
                action_trail_sha256: "c".repeat(64),
                terminated: true,
            };
            assert!(verify_browser_result(&request, &preview, &result).is_ok());
            result.terminated = false;
            assert_eq!(
                verify_browser_result(&request, &preview, &result),
                Err(BrowserInspectionError::ResultDenied)
            );
        }
    }
}
