//! Pure GitHub authentication admission and normalized read-observation contracts.
#![allow(missing_docs)]

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::connected_connector::{ConnectedProfile, NetworkMethod, TemporaryNetworkGrant};
use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

/// Approved GitHub host class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubHostKind {
    GithubCom,
    Enterprise,
}

/// Closed read-only provider transport set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubTransport {
    CommandLine,
    Rest,
    GraphQl,
}

/// Ordered authentication preference. Values are references, never secrets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubCredentialClass {
    GithubAppInstallation,
    SecureShellAgent,
    CredentialHelper,
    FineGrainedToken,
}

/// Closed normalized read operation set; no mutation operation is representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubReadOperation {
    RepositoryMetadata,
    RepositoryContent,
    CommitMetadata,
    ReferenceMetadata,
}

/// Stable response disposition independent of CLI/REST/GraphQL wire format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GithubReadState {
    Success,
    Partial,
    Empty,
    Malformed,
    RateLimited,
    Expired,
    Revoked,
    Unauthorized,
    Forbidden,
    Unavailable,
    Cancelled,
}

/// Exact provider authentication security domain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubAuthBinding {
    pub binding_id: String,
    pub host_kind: GithubHostKind,
    pub canonical_api_host: String,
    pub canonical_clone_host: String,
    pub transport_identity_sha256: String,
    pub enterprise_id: Option<String>,
    pub organization_id: String,
    pub account_id: String,
    pub app_id: Option<String>,
    pub installation_id: Option<String>,
    pub repository_ids: Vec<String>,
    pub operation: GithubReadOperation,
    pub permission_scopes: Vec<String>,
    pub single_sign_on_active: bool,
    pub expires_epoch_milliseconds: u64,
    pub credential_reference_id: String,
    pub credential_class: GithubCredentialClass,
    pub transport: GithubTransport,
}

/// Complete content-free authentication diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubAuthDiagnostic {
    pub binding_id: String,
    pub canonical_api_host: String,
    pub canonical_clone_host: String,
    pub account_or_app_id: String,
    pub installation_id: Option<String>,
    pub repository_ids: Vec<String>,
    pub effective_permission_scopes: Vec<String>,
    pub missing_permission_scopes: Vec<String>,
    pub expires_epoch_milliseconds: u64,
    pub single_sign_on_active: bool,
    pub enabled: bool,
    pub diagnostic_code: String,
    pub credential_reference_id: String,
}

/// Exact normalized read request admitted behind every provider transport.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubReadRequest {
    pub request_id: String,
    pub grant_id: String,
    pub binding_id: String,
    pub repository_id: String,
    pub actor_id: String,
    pub object_path: String,
    pub operation: GithubReadOperation,
    pub page_size: u16,
    pub page_cursor_sha256: Option<String>,
    pub entity_tag_sha256: Option<String>,
    pub cancel_requested: bool,
}

/// Caller-observed provider response. It cannot initiate network access.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubReadObservation {
    pub request_id: String,
    pub state: GithubReadState,
    pub immutable_object_id: Option<String>,
    pub content_sha256: Option<String>,
    pub item_count: u32,
    pub next_page_cursor_sha256: Option<String>,
    pub entity_tag_sha256: Option<String>,
    pub rate_limit_remaining: Option<u64>,
    pub retry_after_milliseconds: Option<u64>,
    pub observed_epoch_milliseconds: u64,
    pub provider_message_code: String,
}

/// Stable read receipt with no provider mutation authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubReadReceipt {
    pub schema_version: u16,
    pub host: String,
    pub repository_id: String,
    pub actor_id: String,
    pub object_path: String,
    pub immutable_object_id: Option<String>,
    pub request_id: String,
    pub operation: GithubReadOperation,
    pub result: GithubReadState,
    pub fresh_at_epoch_milliseconds: u64,
    pub next_page_cursor_sha256: Option<String>,
    pub cache_validator_sha256: Option<String>,
    pub rate_limit_remaining: Option<u64>,
    pub retry_after_milliseconds: Option<u64>,
    pub external_state_changed: bool,
    pub fresh_grant_required_for_retry: bool,
    pub receipt_sha256: String,
}

/// Provider admission failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GithubProviderError {
    InvalidInput,
    SecurityDomainMismatch,
    InsufficientPermission,
    Expired,
    Revoked,
}

impl GithubProviderError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "github.input.invalid",
            Self::SecurityDomainMismatch => "github.security-domain.mismatch",
            Self::InsufficientPermission => "github.permission.insufficient",
            Self::Expired => "github.authentication.expired",
            Self::Revoked => "github.authentication.revoked",
        }
    }
}
impl std::fmt::Display for GithubProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for GithubProviderError {}

fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn valid_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value == value.to_ascii_lowercase()
        && !value.contains(['/', ':', '@', '?', '#', '\\'])
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value.split('.').all(|p| {
            !p.is_empty()
                && !p.starts_with('-')
                && !p.ends_with('-')
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
fn sorted_unique(values: &[String]) -> bool {
    !values.is_empty()
        && values.windows(2).all(|w| w[0] < w[1])
        && values.iter().all(|v| valid_identifier(v))
}
fn required_scope(operation: GithubReadOperation) -> &'static str {
    match operation {
        GithubReadOperation::RepositoryMetadata => "metadata:read",
        GithubReadOperation::RepositoryContent => "contents:read",
        GithubReadOperation::CommitMetadata | GithubReadOperation::ReferenceMetadata => {
            "contents:read"
        }
    }
}

/// Admit an exact auth binding and emit its secret-free diagnostic.
pub fn admit_github_authentication(
    binding: &GithubAuthBinding,
    now: u64,
    revoked: bool,
) -> Result<GithubAuthDiagnostic, GithubProviderError> {
    if revoked {
        return Err(GithubProviderError::Revoked);
    }
    if !valid_identifier(&binding.binding_id)
        || !valid_host(&binding.canonical_api_host)
        || !valid_host(&binding.canonical_clone_host)
        || !valid_sha(&binding.transport_identity_sha256)
        || !valid_identifier(&binding.organization_id)
        || !valid_identifier(&binding.account_id)
        || binding
            .enterprise_id
            .as_ref()
            .is_some_and(|v| !valid_identifier(v))
        || binding
            .app_id
            .as_ref()
            .is_some_and(|v| !valid_identifier(v))
        || binding
            .installation_id
            .as_ref()
            .is_some_and(|v| !valid_identifier(v))
        || !sorted_unique(&binding.repository_ids)
        || !sorted_unique(&binding.permission_scopes)
        || !valid_identifier(&binding.credential_reference_id)
        || binding.expires_epoch_milliseconds == 0
    {
        return Err(GithubProviderError::InvalidInput);
    }
    let github_com = binding.host_kind == GithubHostKind::GithubCom;
    if (github_com
        && (binding.canonical_api_host != "api.github.com"
            || binding.canonical_clone_host != "github.com"
            || binding.enterprise_id.is_some()))
        || (!github_com
            && (binding.enterprise_id.is_none()
                || binding.canonical_api_host != binding.canonical_clone_host))
        || (binding.credential_class == GithubCredentialClass::GithubAppInstallation
            && (binding.app_id.is_none() || binding.installation_id.is_none()))
    {
        return Err(GithubProviderError::SecurityDomainMismatch);
    }
    if now >= binding.expires_epoch_milliseconds {
        return Err(GithubProviderError::Expired);
    }
    let needed = required_scope(binding.operation);
    if !binding.single_sign_on_active
        || binding
            .permission_scopes
            .iter()
            .any(|s| s.ends_with(":write"))
        || !binding.permission_scopes.iter().any(|s| s == needed)
    {
        return Err(GithubProviderError::InsufficientPermission);
    }
    Ok(GithubAuthDiagnostic {
        binding_id: binding.binding_id.clone(),
        canonical_api_host: binding.canonical_api_host.clone(),
        canonical_clone_host: binding.canonical_clone_host.clone(),
        account_or_app_id: binding
            .app_id
            .clone()
            .unwrap_or_else(|| binding.account_id.clone()),
        installation_id: binding.installation_id.clone(),
        repository_ids: binding.repository_ids.clone(),
        effective_permission_scopes: binding.permission_scopes.clone(),
        missing_permission_scopes: vec![],
        expires_epoch_milliseconds: binding.expires_epoch_milliseconds,
        single_sign_on_active: true,
        enabled: true,
        diagnostic_code: "github.authentication.ready".into(),
        credential_reference_id: binding.credential_reference_id.clone(),
    })
}

/// Verify a normalized read against both the active connector grant and auth binding.
pub fn authorize_github_read(
    binding: &GithubAuthBinding,
    grant: &TemporaryNetworkGrant,
    profile: &ConnectedProfile,
    request: &GithubReadRequest,
    now: u64,
) -> Result<(), GithubProviderError> {
    if !profile.active
        || now >= profile.expires_epoch_milliseconds
        || now >= binding.expires_epoch_milliseconds
    {
        return Err(GithubProviderError::Expired);
    }
    if !valid_identifier(&request.request_id)
        || !valid_identifier(&request.actor_id)
        || request.object_path.is_empty()
        || request.object_path.len() > 2048
        || request.object_path.contains(['?', '#', '\\'])
        || request.object_path.split('/').any(|p| p == "..")
        || request.page_size == 0
        || request.page_size > 100
        || request
            .page_cursor_sha256
            .as_ref()
            .is_some_and(|v| !valid_sha(v))
        || request
            .entity_tag_sha256
            .as_ref()
            .is_some_and(|v| !valid_sha(v))
    {
        return Err(GithubProviderError::InvalidInput);
    }
    if request.cancel_requested {
        return Err(GithubProviderError::SecurityDomainMismatch);
    }
    if grant.grant_id != request.grant_id
        || profile.grant_id != request.grant_id
        || grant.actor_id != request.actor_id
        || request.binding_id != binding.binding_id
        || request.operation != binding.operation
        || !binding.repository_ids.contains(&request.repository_id)
        || grant.destination_host != binding.canonical_api_host
        || profile.destination_host != binding.canonical_api_host
        || grant.method != NetworkMethod::Get
        || grant.credential_reference_id != binding.credential_reference_id
    {
        return Err(GithubProviderError::SecurityDomainMismatch);
    }
    Ok(())
}

/// Normalize one caller observation into a content-free, immutable read receipt.
pub fn record_github_read(
    binding: &GithubAuthBinding,
    request: &GithubReadRequest,
    observation: &GithubReadObservation,
) -> Result<GithubReadReceipt, GithubProviderError> {
    if observation.request_id != request.request_id
        || observation.observed_epoch_milliseconds == 0
        || !valid_identifier(&observation.provider_message_code)
        || observation
            .immutable_object_id
            .as_ref()
            .is_some_and(|v| !valid_identifier(v))
        || observation
            .content_sha256
            .as_ref()
            .is_some_and(|v| !valid_sha(v))
        || observation
            .next_page_cursor_sha256
            .as_ref()
            .is_some_and(|v| !valid_sha(v))
        || observation
            .entity_tag_sha256
            .as_ref()
            .is_some_and(|v| !valid_sha(v))
        || observation
            .retry_after_milliseconds
            .is_some_and(|v| v > 86_400_000)
    {
        return Err(GithubProviderError::InvalidInput);
    }
    let has_content = matches!(
        observation.state,
        GithubReadState::Success | GithubReadState::Partial
    );
    if has_content
        != (observation.immutable_object_id.is_some() && observation.content_sha256.is_some())
        || (observation.state == GithubReadState::Empty && observation.item_count != 0)
        || (observation.state == GithubReadState::RateLimited
            && observation.retry_after_milliseconds.is_none())
    {
        return Err(GithubProviderError::InvalidInput);
    }
    let mut receipt = GithubReadReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        host: binding.canonical_api_host.clone(),
        repository_id: request.repository_id.clone(),
        actor_id: request.actor_id.clone(),
        object_path: request.object_path.clone(),
        immutable_object_id: observation.immutable_object_id.clone(),
        request_id: request.request_id.clone(),
        operation: request.operation,
        result: observation.state,
        fresh_at_epoch_milliseconds: observation.observed_epoch_milliseconds,
        next_page_cursor_sha256: observation.next_page_cursor_sha256.clone(),
        cache_validator_sha256: observation.entity_tag_sha256.clone(),
        rate_limit_remaining: observation.rate_limit_remaining,
        retry_after_milliseconds: observation.retry_after_milliseconds,
        external_state_changed: false,
        fresh_grant_required_for_retry: !matches!(
            observation.state,
            GithubReadState::Success | GithubReadState::Partial | GithubReadState::Empty
        ),
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 =
        word_sha256(&serde_json::to_vec(&receipt).map_err(|_| GithubProviderError::InvalidInput)?);
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConnectedOperation, activate_connected_profile, prepare_network_grant};
    fn binding() -> GithubAuthBinding {
        GithubAuthBinding {
            binding_id: "binding-1".into(),
            host_kind: GithubHostKind::GithubCom,
            canonical_api_host: "api.github.com".into(),
            canonical_clone_host: "github.com".into(),
            transport_identity_sha256: "a".repeat(64),
            enterprise_id: None,
            organization_id: "org-1".into(),
            account_id: "account-1".into(),
            app_id: Some("app-1".into()),
            installation_id: Some("install-1".into()),
            repository_ids: vec!["repo-1".into()],
            operation: GithubReadOperation::RepositoryContent,
            permission_scopes: vec!["contents:read".into(), "metadata:read".into()],
            single_sign_on_active: true,
            expires_epoch_milliseconds: 900,
            credential_reference_id: "credential-ref-1".into(),
            credential_class: GithubCredentialClass::GithubAppInstallation,
            transport: GithubTransport::Rest,
        }
    }
    fn grant() -> TemporaryNetworkGrant {
        TemporaryNetworkGrant {
            grant_id: "grant-1".into(),
            actor_id: "actor-1".into(),
            session_id: "session-1".into(),
            task_id: "task-1".into(),
            connector_id: "github".into(),
            destination_host: "api.github.com".into(),
            method: NetworkMethod::Get,
            destination_path: "/repos/org/repo/contents".into(),
            query_sha256: "b".repeat(64),
            operation: ConnectedOperation::ConnectorRead,
            purpose: "github-read".into(),
            scope_id: "workspace-1".into(),
            expected_data: "repository-content".into(),
            max_response_bytes: 1024,
            issued_epoch_milliseconds: 100,
            expires_epoch_milliseconds: 800,
            credential_reference_id: "credential-ref-1".into(),
            credential_scope_sha256: "c".repeat(64),
            cache_partition_id: "account-workspace".into(),
            cache_retention_milliseconds: 100,
            user_initiated: true,
            background_operation: false,
        }
    }
    fn request() -> GithubReadRequest {
        GithubReadRequest {
            request_id: "request-1".into(),
            grant_id: "grant-1".into(),
            binding_id: "binding-1".into(),
            repository_id: "repo-1".into(),
            actor_id: "actor-1".into(),
            object_path: "src/lib.rs".into(),
            operation: GithubReadOperation::RepositoryContent,
            page_size: 50,
            page_cursor_sha256: None,
            entity_tag_sha256: None,
            cancel_requested: false,
        }
    }
    #[test]
    fn exact_app_binding_and_read_receipt_pass() {
        let b = binding();
        let diagnostic = admit_github_authentication(&b, 200, false).unwrap();
        assert!(diagnostic.enabled);
        let p = prepare_network_grant(grant()).unwrap();
        let active = activate_connected_profile(&p, &p.preview_sha256, 200).unwrap();
        authorize_github_read(&b, &p.grant, &active, &request(), 200).unwrap();
        let o = GithubReadObservation {
            request_id: "request-1".into(),
            state: GithubReadState::Success,
            immutable_object_id: Some("blob-1".into()),
            content_sha256: Some("d".repeat(64)),
            item_count: 1,
            next_page_cursor_sha256: Some("e".repeat(64)),
            entity_tag_sha256: Some("f".repeat(64)),
            rate_limit_remaining: Some(10),
            retry_after_milliseconds: None,
            observed_epoch_milliseconds: 300,
            provider_message_code: "ok".into(),
        };
        let receipt = record_github_read(&b, &request(), &o).unwrap();
        assert!(!receipt.external_state_changed);
        assert_eq!(receipt.host, "api.github.com");
    }
    #[test]
    fn wrong_host_account_repository_and_credential_fail_closed() {
        let b = binding();
        let p = prepare_network_grant(grant()).unwrap();
        let active = activate_connected_profile(&p, &p.preview_sha256, 200).unwrap();
        for mutate in 0..4 {
            let mut r = request();
            let mut g = p.grant.clone();
            match mutate {
                0 => g.destination_host = "uploads.github.com".into(),
                1 => r.binding_id = "other-account".into(),
                2 => r.repository_id = "repo-2".into(),
                _ => g.credential_reference_id = "other-ref".into(),
            };
            assert_eq!(
                authorize_github_read(&b, &g, &active, &r, 200),
                Err(GithubProviderError::SecurityDomainMismatch)
            );
        }
    }
    #[test]
    fn excessive_missing_expired_revoked_and_enterprise_aliases_fail() {
        let mut b = binding();
        b.permission_scopes.push("issues:write".into());
        b.permission_scopes.sort();
        assert_eq!(
            admit_github_authentication(&b, 200, false),
            Err(GithubProviderError::InsufficientPermission)
        );
        let mut b = binding();
        b.single_sign_on_active = false;
        assert!(admit_github_authentication(&b, 200, false).is_err());
        assert_eq!(
            admit_github_authentication(&binding(), 900, false),
            Err(GithubProviderError::Expired)
        );
        assert_eq!(
            admit_github_authentication(&binding(), 200, true),
            Err(GithubProviderError::Revoked)
        );
        let mut b = binding();
        b.host_kind = GithubHostKind::Enterprise;
        b.enterprise_id = Some("enterprise-1".into());
        assert_eq!(
            admit_github_authentication(&b, 200, false),
            Err(GithubProviderError::SecurityDomainMismatch)
        );
    }
    #[test]
    fn all_typed_response_states_are_stable_and_validated() {
        let b = binding();
        for state in [
            GithubReadState::Success,
            GithubReadState::Partial,
            GithubReadState::Empty,
            GithubReadState::Malformed,
            GithubReadState::RateLimited,
            GithubReadState::Expired,
            GithubReadState::Revoked,
            GithubReadState::Unauthorized,
            GithubReadState::Forbidden,
            GithubReadState::Unavailable,
            GithubReadState::Cancelled,
        ] {
            let content = matches!(state, GithubReadState::Success | GithubReadState::Partial);
            let o = GithubReadObservation {
                request_id: "request-1".into(),
                state,
                immutable_object_id: content.then(|| "object-1".into()),
                content_sha256: content.then(|| "d".repeat(64)),
                item_count: u32::from(content),
                next_page_cursor_sha256: None,
                entity_tag_sha256: None,
                rate_limit_remaining: Some(0),
                retry_after_milliseconds: (state == GithubReadState::RateLimited).then_some(100),
                observed_epoch_milliseconds: 300,
                provider_message_code: "observed".into(),
            };
            let r = record_github_read(&b, &request(), &o).unwrap();
            assert_eq!(r.result, state);
            assert!(!r.external_state_changed);
        }
    }
}
