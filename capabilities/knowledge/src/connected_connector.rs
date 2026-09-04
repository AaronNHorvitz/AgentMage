//! Pure contracts for explicit temporary network grants and connector-cache observations.

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

const MAX_HOST_BYTES: usize = 253;
const MAX_PATH_BYTES: usize = 2_048;
const MAX_EXPECTED_DATA_BYTES: usize = 1_024;
const MAX_RESPONSE_BYTES: u64 = 64 * 1_024 * 1_024;
const MAX_DURATION_MILLISECONDS: u64 = 15 * 60 * 1_000;

/// Closed network method set for the first connected profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMethod {
    /// Read one resource representation.
    Get,
    /// Read resource metadata without a response body.
    Head,
}

/// Closed connected operation class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectedOperation {
    /// Bounded connector read.
    ConnectorRead,
}

/// Cache sensitivity label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheSensitivity {
    /// Public content.
    Public,
    /// Account-private content.
    Private,
    /// Restricted content requiring the tightest local handling.
    Restricted,
}

/// Exact temporary network grant request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporaryNetworkGrant {
    /// Stable grant identity.
    pub grant_id: String,
    /// Stable actor identity.
    pub actor_id: String,
    /// Stable session identity.
    pub session_id: String,
    /// Stable task identity.
    pub task_id: String,
    /// Stable connector identity.
    pub connector_id: String,
    /// Exact lowercase ASCII DNS host.
    pub destination_host: String,
    /// Closed method.
    pub method: NetworkMethod,
    /// Exact absolute destination path without query or fragment.
    pub destination_path: String,
    /// Hash of canonically ordered query parameters.
    pub query_sha256: String,
    /// Exact operation class.
    pub operation: ConnectedOperation,
    /// Stable human-visible purpose code.
    pub purpose: String,
    /// Stable scope identity.
    pub scope_id: String,
    /// Content-free expected-data description.
    pub expected_data: String,
    /// Maximum response bytes.
    pub max_response_bytes: u64,
    /// Grant issue time in Unix epoch milliseconds.
    pub issued_epoch_milliseconds: u64,
    /// Grant expiry time in Unix epoch milliseconds.
    pub expires_epoch_milliseconds: u64,
    /// Stable derived-credential reference; never credential material.
    pub credential_reference_id: String,
    /// SHA-256 of the approved credential class and scope.
    pub credential_scope_sha256: String,
    /// Stable target cache partition.
    pub cache_partition_id: String,
    /// Maximum cache retention in milliseconds.
    pub cache_retention_milliseconds: u64,
    /// True only for an immediate explicit user action.
    pub user_initiated: bool,
    /// False for startup polling or background refresh.
    pub background_operation: bool,
}

/// Visible approval preview for one exact grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkGrantPreview {
    /// Exact request.
    pub grant: TemporaryNetworkGrant,
    /// Canonical preview digest.
    pub preview_sha256: String,
    /// True because the destination is rendered before activation.
    pub destination_visible: bool,
    /// True because purpose/scope/data/limits/expiry/credential/cache are rendered.
    pub complete_disclosure: bool,
    /// False because preview has no network authority.
    pub network_effect_performed: bool,
}

/// Active temporary connected profile state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectedProfile {
    /// Exact approved preview digest.
    pub approved_preview_sha256: String,
    /// Exact grant identity.
    pub grant_id: String,
    /// Active connector identity.
    pub connector_id: String,
    /// Exact destination host.
    pub destination_host: String,
    /// Expiry time.
    pub expires_epoch_milliseconds: u64,
    /// True only between explicit activation and terminal reconciliation.
    pub active: bool,
    /// False while the temporary profile is active.
    pub strict_local_baseline_active: bool,
}

/// Caller-supplied observation of one separately executed connector read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorResponseObservation {
    /// Grant identity used by the external executor.
    pub grant_id: String,
    /// Immutable source-object identity.
    pub object_id: String,
    /// Exact response body hash.
    pub content_sha256: String,
    /// Exact response body byte count.
    pub content_bytes: u64,
    /// Observed retrieval time.
    pub retrieved_epoch_milliseconds: u64,
    /// Optional HTTP status.
    pub status_code: Option<u16>,
    /// Optional rate-limit remaining count.
    pub rate_limit_remaining: Option<u64>,
    /// Optional retry-after duration.
    pub retry_after_milliseconds: Option<u64>,
    /// True when cancellation was observed.
    pub cancelled: bool,
    /// True when outcome cannot be reconciled.
    pub uncertain_result: bool,
    /// True only when the separately authorized executor observed a request.
    pub network_effect_observed: bool,
}

/// Sensitivity-labeled connector cache entry without policy authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorCacheEntry {
    /// Exact cache partition.
    pub cache_partition_id: String,
    /// Connector identity.
    pub connector_id: String,
    /// Source host.
    pub source_host: String,
    /// Immutable source object.
    pub object_id: String,
    /// Exact content hash.
    pub content_sha256: String,
    /// Exact content byte count.
    pub content_bytes: u64,
    /// Retrieval freshness.
    pub retrieved_epoch_milliseconds: u64,
    /// Expiry/retention boundary.
    pub expires_epoch_milliseconds: u64,
    /// Sensitivity label.
    pub sensitivity: CacheSensitivity,
    /// True only when the caller proves storage used the admitted encrypted cache.
    pub encrypted_at_rest_observed: bool,
    /// False because imported content cannot carry policy authority.
    pub policy_authority: bool,
    /// False because imported content cannot carry grant authority.
    pub grant_authority: bool,
    /// False because imported content cannot carry tool authority.
    pub tool_authority: bool,
    /// False because imported content cannot claim completion.
    pub completion_authority: bool,
    /// True because deletion at expiry remains mandatory.
    pub deletion_required: bool,
}

/// Closed terminal network disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkTerminalState {
    /// A bounded read observation was accepted.
    Completed,
    /// No request completed before cancellation.
    Cancelled,
    /// A rate limit requires a fresh later grant; no blind retry.
    RateLimited,
    /// Outcome is uncertain and cannot be retried blindly.
    Uncertain,
    /// The observed response failed.
    Failed,
}

/// Terminal operation receipt that restores the offline baseline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkOperationReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact grant identity.
    pub grant_id: String,
    /// Exact preview identity.
    pub approved_preview_sha256: String,
    /// Terminal state.
    pub terminal_state: NetworkTerminalState,
    /// Observed external request count, at most one.
    pub request_count: u8,
    /// True when a fresh grant is required before another request.
    pub fresh_grant_required: bool,
    /// True after terminal reconciliation.
    pub strict_local_baseline_restored: bool,
    /// True because derived credential material must be invalidated externally.
    pub credential_invalidation_required: bool,
    /// Stable receipt digest.
    pub receipt_sha256: String,
}

/// Connected-profile contract failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectedProfileError {
    /// An identity, host, path, hash, limit, or observation is invalid.
    InvalidInput,
    /// A grant is stale, background, mismatched, or not explicitly approved.
    NotAuthorized,
    /// A resource ceiling was exceeded.
    ResourceLimit,
    /// The connected profile has expired.
    Expired,
}

impl ConnectedProfileError {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "connector.input.invalid",
            Self::NotAuthorized => "connector.operation.not-authorized",
            Self::ResourceLimit => "connector.resource.limit",
            Self::Expired => "connector.grant.expired",
        }
    }
}

impl std::fmt::Display for ConnectedProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for ConnectedProfileError {}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
fn valid_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_HOST_BYTES
        && value.is_ascii()
        && value == value.to_ascii_lowercase()
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

/// Validates and digests a complete visible grant without performing network access.
pub fn prepare_network_grant(
    grant: TemporaryNetworkGrant,
) -> Result<NetworkGrantPreview, ConnectedProfileError> {
    if !valid_identifier(&grant.grant_id)
        || !valid_identifier(&grant.actor_id)
        || !valid_identifier(&grant.session_id)
        || !valid_identifier(&grant.task_id)
        || !valid_identifier(&grant.connector_id)
        || !valid_host(&grant.destination_host)
        || grant.destination_host.parse::<std::net::IpAddr>().is_ok()
        || !grant.destination_path.starts_with('/')
        || grant.destination_path.len() > MAX_PATH_BYTES
        || grant.destination_path.contains(['?', '#', '\\'])
        || grant.destination_path.split('/').any(|part| part == "..")
        || !valid_sha256(&grant.query_sha256)
        || !valid_identifier(&grant.purpose)
        || !valid_identifier(&grant.scope_id)
        || grant.expected_data.is_empty()
        || grant.expected_data.len() > MAX_EXPECTED_DATA_BYTES
        || grant.max_response_bytes == 0
        || grant.max_response_bytes > MAX_RESPONSE_BYTES
        || grant.issued_epoch_milliseconds == 0
        || grant.expires_epoch_milliseconds <= grant.issued_epoch_milliseconds
        || grant.expires_epoch_milliseconds - grant.issued_epoch_milliseconds
            > MAX_DURATION_MILLISECONDS
        || !valid_identifier(&grant.credential_reference_id)
        || !valid_sha256(&grant.credential_scope_sha256)
        || !valid_identifier(&grant.cache_partition_id)
        || grant.cache_retention_milliseconds == 0
    {
        return Err(ConnectedProfileError::InvalidInput);
    }
    if !grant.user_initiated || grant.background_operation {
        return Err(ConnectedProfileError::NotAuthorized);
    }
    let preview_sha256 =
        word_sha256(&serde_json::to_vec(&grant).map_err(|_| ConnectedProfileError::InvalidInput)?);
    Ok(NetworkGrantPreview {
        grant,
        preview_sha256,
        destination_visible: true,
        complete_disclosure: true,
        network_effect_performed: false,
    })
}

/// Activates one exact approved preview for a bounded time without executing a request.
pub fn activate_connected_profile(
    preview: &NetworkGrantPreview,
    approved_sha256: &str,
    now: u64,
) -> Result<ConnectedProfile, ConnectedProfileError> {
    if preview.preview_sha256 != approved_sha256
        || !preview.destination_visible
        || !preview.complete_disclosure
        || preview.network_effect_performed
    {
        return Err(ConnectedProfileError::NotAuthorized);
    }
    if now < preview.grant.issued_epoch_milliseconds
        || now >= preview.grant.expires_epoch_milliseconds
    {
        return Err(ConnectedProfileError::Expired);
    }
    Ok(ConnectedProfile {
        approved_preview_sha256: approved_sha256.to_owned(),
        grant_id: preview.grant.grant_id.clone(),
        connector_id: preview.grant.connector_id.clone(),
        destination_host: preview.grant.destination_host.clone(),
        expires_epoch_milliseconds: preview.grant.expires_epoch_milliseconds,
        active: true,
        strict_local_baseline_active: false,
    })
}

/// Validates one caller-supplied response observation and constructs a powerless cache record.
pub fn record_connector_response(
    preview: &NetworkGrantPreview,
    profile: &ConnectedProfile,
    observation: &ConnectorResponseObservation,
    sensitivity: CacheSensitivity,
    encrypted_at_rest_observed: bool,
) -> Result<Option<ConnectorCacheEntry>, ConnectedProfileError> {
    if !profile.active
        || profile.grant_id != preview.grant.grant_id
        || observation.grant_id != profile.grant_id
        || !valid_identifier(&observation.object_id)
        || !valid_sha256(&observation.content_sha256)
        || observation.content_bytes > preview.grant.max_response_bytes
        || observation.retrieved_epoch_milliseconds >= preview.grant.expires_epoch_milliseconds
        || observation
            .status_code
            .is_some_and(|value| !(100..=599).contains(&value))
        || observation
            .retry_after_milliseconds
            .is_some_and(|value| value > MAX_DURATION_MILLISECONDS)
    {
        return Err(ConnectedProfileError::InvalidInput);
    }
    if observation.content_bytes > 0 && !observation.network_effect_observed {
        return Err(ConnectedProfileError::InvalidInput);
    }
    if observation.cancelled || observation.uncertain_result || observation.status_code != Some(200)
    {
        return Ok(None);
    }
    if !encrypted_at_rest_observed {
        return Err(ConnectedProfileError::NotAuthorized);
    }
    Ok(Some(ConnectorCacheEntry {
        cache_partition_id: preview.grant.cache_partition_id.clone(),
        connector_id: preview.grant.connector_id.clone(),
        source_host: preview.grant.destination_host.clone(),
        object_id: observation.object_id.clone(),
        content_sha256: observation.content_sha256.clone(),
        content_bytes: observation.content_bytes,
        retrieved_epoch_milliseconds: observation.retrieved_epoch_milliseconds,
        expires_epoch_milliseconds: observation
            .retrieved_epoch_milliseconds
            .saturating_add(preview.grant.cache_retention_milliseconds)
            .min(preview.grant.expires_epoch_milliseconds),
        sensitivity,
        encrypted_at_rest_observed: true,
        policy_authority: false,
        grant_authority: false,
        tool_authority: false,
        completion_authority: false,
        deletion_required: true,
    }))
}

/// Reconciles a terminal observation, requires fresh authority for retry, and restores strict-local.
pub fn finish_connected_operation(
    preview: &NetworkGrantPreview,
    profile: ConnectedProfile,
    observation: &ConnectorResponseObservation,
) -> Result<NetworkOperationReceipt, ConnectedProfileError> {
    if profile.grant_id != preview.grant.grant_id || observation.grant_id != profile.grant_id {
        return Err(ConnectedProfileError::InvalidInput);
    }
    let terminal_state = if observation.uncertain_result {
        NetworkTerminalState::Uncertain
    } else if observation.cancelled {
        NetworkTerminalState::Cancelled
    } else if observation.status_code == Some(429) || observation.retry_after_milliseconds.is_some()
    {
        NetworkTerminalState::RateLimited
    } else if observation.status_code == Some(200) {
        NetworkTerminalState::Completed
    } else {
        NetworkTerminalState::Failed
    };
    let request_count = u8::from(observation.network_effect_observed);
    let mut receipt = NetworkOperationReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        grant_id: profile.grant_id,
        approved_preview_sha256: profile.approved_preview_sha256,
        terminal_state,
        request_count,
        fresh_grant_required: true,
        strict_local_baseline_restored: true,
        credential_invalidation_required: true,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = word_sha256(
        &serde_json::to_vec(&receipt).map_err(|_| ConnectedProfileError::InvalidInput)?,
    );
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grant() -> TemporaryNetworkGrant {
        TemporaryNetworkGrant {
            grant_id: "grant-1".into(),
            actor_id: "actor-1".into(),
            session_id: "session-1".into(),
            task_id: "task-1".into(),
            connector_id: "connector-1".into(),
            destination_host: "api.example.com".into(),
            method: NetworkMethod::Get,
            destination_path: "/v1/items".into(),
            query_sha256: "a".repeat(64),
            operation: ConnectedOperation::ConnectorRead,
            purpose: "user-requested-read".into(),
            scope_id: "workspace-1".into(),
            expected_data: "bounded-item-metadata".into(),
            max_response_bytes: 1024,
            issued_epoch_milliseconds: 100,
            expires_epoch_milliseconds: 1000,
            credential_reference_id: "credential-ref-1".into(),
            credential_scope_sha256: "b".repeat(64),
            cache_partition_id: "account-1-workspace-1".into(),
            cache_retention_milliseconds: 500,
            user_initiated: true,
            background_operation: false,
        }
    }
    fn observation(status: u16) -> ConnectorResponseObservation {
        ConnectorResponseObservation {
            grant_id: "grant-1".into(),
            object_id: "object-1".into(),
            content_sha256: "c".repeat(64),
            content_bytes: 10,
            retrieved_epoch_milliseconds: 300,
            status_code: Some(status),
            rate_limit_remaining: Some(10),
            retry_after_milliseconds: None,
            cancelled: false,
            uncertain_result: false,
            network_effect_observed: true,
        }
    }

    #[test]
    fn exact_preview_activation_cache_and_offline_receipt() {
        let preview = prepare_network_grant(grant()).expect("preview");
        let profile =
            activate_connected_profile(&preview, &preview.preview_sha256, 200).expect("active");
        let cached = record_connector_response(
            &preview,
            &profile,
            &observation(200),
            CacheSensitivity::Private,
            true,
        )
        .expect("record")
        .expect("cache");
        assert!(
            cached.encrypted_at_rest_observed
                && !cached.policy_authority
                && cached.deletion_required
        );
        let receipt =
            finish_connected_operation(&preview, profile, &observation(200)).expect("finish");
        assert!(receipt.strict_local_baseline_restored && receipt.fresh_grant_required);
        assert_eq!(receipt.request_count, 1);
    }
    #[test]
    fn background_stale_mutated_and_local_destination_grants_fail() {
        let mut changed = grant();
        changed.background_operation = true;
        assert_eq!(
            prepare_network_grant(changed),
            Err(ConnectedProfileError::NotAuthorized)
        );
        let mut changed = grant();
        changed.destination_host = "127.0.0.1".into();
        assert!(prepare_network_grant(changed).is_err());
        let preview = prepare_network_grant(grant()).expect("preview");
        assert!(activate_connected_profile(&preview, &"d".repeat(64), 200).is_err());
        assert_eq!(
            activate_connected_profile(&preview, &preview.preview_sha256, 1000),
            Err(ConnectedProfileError::Expired)
        );
    }
    #[test]
    fn cancellation_rate_limit_and_uncertain_results_never_cache_or_blind_retry() {
        let preview = prepare_network_grant(grant()).expect("preview");
        let profile =
            activate_connected_profile(&preview, &preview.preview_sha256, 200).expect("active");
        let mut cancelled = observation(200);
        cancelled.cancelled = true;
        assert_eq!(
            record_connector_response(
                &preview,
                &profile,
                &cancelled,
                CacheSensitivity::Public,
                true
            )
            .expect("record"),
            None
        );
        let mut limited = observation(429);
        limited.retry_after_milliseconds = Some(500);
        let receipt =
            finish_connected_operation(&preview, profile.clone(), &limited).expect("finish");
        assert_eq!(receipt.terminal_state, NetworkTerminalState::RateLimited);
        assert!(receipt.fresh_grant_required);
        let mut uncertain = observation(200);
        uncertain.uncertain_result = true;
        let receipt = finish_connected_operation(&preview, profile, &uncertain).expect("finish");
        assert_eq!(receipt.terminal_state, NetworkTerminalState::Uncertain);
    }
    #[test]
    fn oversized_unencrypted_and_cross_grant_observations_fail() {
        let preview = prepare_network_grant(grant()).expect("preview");
        let profile =
            activate_connected_profile(&preview, &preview.preview_sha256, 200).expect("active");
        let mut oversized = observation(200);
        oversized.content_bytes = 1025;
        assert_eq!(
            record_connector_response(
                &preview,
                &profile,
                &oversized,
                CacheSensitivity::Restricted,
                true
            ),
            Err(ConnectedProfileError::InvalidInput)
        );
        assert_eq!(
            record_connector_response(
                &preview,
                &profile,
                &observation(200),
                CacheSensitivity::Private,
                false
            ),
            Err(ConnectedProfileError::NotAuthorized)
        );
        let mut wrong = observation(200);
        wrong.grant_id = "grant-2".into();
        assert!(finish_connected_operation(&preview, profile, &wrong).is_err());
    }
}
