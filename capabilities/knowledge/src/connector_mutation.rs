//! Pure per-connector governance, isolation, mutation, and recovery contracts.

use serde::{Deserialize, Serialize};

/// Separately governed connector families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorKind {
    /// Email account.
    Email,
    /// Messaging account.
    Messaging,
    /// Calendar account.
    Calendar,
    /// Document repository.
    DocumentRepository,
    /// Archive provider.
    Archive,
    /// Structured database.
    Database,
    /// Cloud resource provider.
    Cloud,
}

/// Closed per-connector operation classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorOperation {
    /// Read-only discovery before any draft or write.
    ReadOnlyDiscovery,
    /// Local draft generation with no remote effect.
    Draft,
    /// Send one email.
    SendEmail,
    /// Send one message.
    SendMessage,
    /// Change one calendar object.
    ChangeCalendar,
    /// Change one document object.
    UpdateDocument,
    /// Change one archive object.
    ChangeArchive,
    /// Write one bounded database object.
    WriteDatabase,
    /// Change one bounded cloud object.
    ChangeCloud,
}

impl ConnectorOperation {
    const fn writes(self) -> bool {
        !matches!(self, Self::ReadOnlyDiscovery | Self::Draft)
    }
}

/// Separate control bundle required for each connector/account pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorControlManifest {
    /// Connector family.
    pub connector: ConnectorKind,
    /// Stable connector instance identity.
    pub connector_id: String,
    /// Exact account identity digest.
    pub account_sha256: String,
    /// Threat model digest.
    pub threat_model_sha256: String,
    /// Capability manifest digest.
    pub capability_manifest_sha256: String,
    /// Data classification digest.
    pub data_classification_sha256: String,
    /// Narrow derived credential scope digest.
    pub credential_scope_sha256: String,
    /// Rate-limit policy digest.
    pub rate_limit_sha256: String,
    /// Publication rule digest.
    pub publication_rule_sha256: String,
    /// Recovery policy digest.
    pub recovery_sha256: String,
    /// Acceptance suite digest.
    pub acceptance_suite_sha256: String,
}

/// Exact remote state read immediately before a write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorRemoteRefresh {
    /// External object identity.
    pub external_identity_sha256: String,
    /// Exact remote-state digest.
    pub state_sha256: String,
    /// Exact permission digest.
    pub permissions_sha256: String,
    /// Observation time.
    pub observed_epoch_milliseconds: u64,
}

/// Exact isolated connector operation request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorMutationRequest {
    /// Stable attempt identity.
    pub request_id: String,
    /// Per-connector governance bundle.
    pub manifest: ConnectorControlManifest,
    /// Workspace partition digest.
    pub workspace_sha256: String,
    /// Task partition digest.
    pub task_sha256: String,
    /// Context partition digest.
    pub context_sha256: String,
    /// Cache partition digest.
    pub cache_sha256: String,
    /// Grant partition digest.
    pub grant_sha256: String,
    /// Short-lived derived credential reference digest, never credential material.
    pub derived_credential_sha256: String,
    /// Closed operation class.
    pub operation: ConnectorOperation,
    /// Exact recipient or object digest.
    pub destination_sha256: String,
    /// Exact payload digest.
    pub payload_sha256: String,
    /// Ordered attachment-set digest.
    pub attachments_sha256: String,
    /// Exact visibility digest.
    pub visibility_sha256: String,
    /// Exact expected side effect digest.
    pub expected_effect_sha256: String,
    /// Exact visible disclosure preview.
    pub disclosure_preview_sha256: String,
    /// Exact idempotency key.
    pub idempotency_key_sha256: String,
    /// Grant expiry.
    pub expires_epoch_milliseconds: u64,
    /// Current time.
    pub now_epoch_milliseconds: u64,
    /// Remote state approved by the user.
    pub approved_refresh: ConnectorRemoteRefresh,
    /// Remote state re-read before mutation.
    pub submission_refresh: ConnectorRemoteRefresh,
    /// Prior read-only discovery was completed for this exact partition.
    pub discovery_complete: bool,
    /// Local draft was completed for this exact write.
    pub draft_complete: bool,
    /// Current user reviewed the exact disclosure.
    pub user_reviewed: bool,
}

/// Content-free preview with no connector authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorMutationPreview {
    /// Stable attempt identity.
    pub request_id: String,
    /// Connector instance identity.
    pub connector_id: String,
    /// Account identity.
    pub account_sha256: String,
    /// Workspace partition.
    pub workspace_sha256: String,
    /// Task partition.
    pub task_sha256: String,
    /// Closed operation.
    pub operation: ConnectorOperation,
    /// Destination.
    pub destination_sha256: String,
    /// Payload.
    pub payload_sha256: String,
    /// Attachments.
    pub attachments_sha256: String,
    /// Visibility.
    pub visibility_sha256: String,
    /// Expected effect.
    pub expected_effect_sha256: String,
    /// Disclosure preview.
    pub disclosure_preview_sha256: String,
    /// Grant.
    pub grant_sha256: String,
    /// Idempotency key.
    pub idempotency_key_sha256: String,
    /// Current remote state.
    pub remote_state_sha256: String,
}

/// Closed connector terminal results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorMutationResult {
    /// Exact expected effect was observed once.
    VerifiedChanged,
    /// Fresh observation proves no effect.
    VerifiedUnchanged,
    /// A partial effect requires compensation and reconciliation.
    Partial,
    /// Outcome is unknown and cannot be retried.
    Unknown,
}

/// Complete connector request/response and recovery receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorMutationReceipt {
    /// Exact request identity.
    pub request_id: String,
    /// Exact external object identity.
    pub external_identity_sha256: String,
    /// Response classification.
    pub result: ConnectorMutationResult,
    /// Exact observed remote state.
    pub observed_state_sha256: String,
    /// Exact observed effect, if verified.
    pub observed_effect_sha256: Option<String>,
    /// Whether external state changed.
    pub external_state_changed: bool,
    /// Number of remote requests for this attempt.
    pub request_count: u8,
    /// Number of retries under this identity; always zero.
    pub retry_count: u8,
    /// Failure classification digest when applicable.
    pub failure_sha256: Option<String>,
    /// Rollback or compensation digest when applicable.
    pub compensation_sha256: Option<String>,
    /// Terminal work stopped.
    pub terminated: bool,
}

/// Stable connector admission and recovery failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectorMutationError {
    /// Identity, scope, stage, grant, expiry, or state mismatch.
    RequestDenied,
    /// Receipt contradicts the exact attempt.
    ReceiptDenied,
    /// Partial or unknown result prohibits retry.
    RetryBlocked,
}

/// Builds one inert preview after isolation and workflow-stage validation.
pub fn admit_connector_mutation(
    request: &ConnectorMutationRequest,
) -> Result<ConnectorMutationPreview, ConnectorMutationError> {
    let hashes = [
        &request.workspace_sha256,
        &request.task_sha256,
        &request.context_sha256,
        &request.cache_sha256,
        &request.grant_sha256,
        &request.derived_credential_sha256,
        &request.destination_sha256,
        &request.payload_sha256,
        &request.attachments_sha256,
        &request.visibility_sha256,
        &request.expected_effect_sha256,
        &request.disclosure_preview_sha256,
        &request.idempotency_key_sha256,
    ];
    if !valid_id(&request.request_id)
        || !valid_manifest(&request.manifest)
        || !hashes.into_iter().all(|value| valid_sha256(value))
        || !valid_refresh(&request.approved_refresh)
        || request.approved_refresh != request.submission_refresh
        || request.now_epoch_milliseconds == 0
        || request.now_epoch_milliseconds >= request.expires_epoch_milliseconds
        || (request.operation.writes()
            && (!request.discovery_complete || !request.draft_complete || !request.user_reviewed))
    {
        return Err(ConnectorMutationError::RequestDenied);
    }
    Ok(ConnectorMutationPreview {
        request_id: request.request_id.clone(),
        connector_id: request.manifest.connector_id.clone(),
        account_sha256: request.manifest.account_sha256.clone(),
        workspace_sha256: request.workspace_sha256.clone(),
        task_sha256: request.task_sha256.clone(),
        operation: request.operation,
        destination_sha256: request.destination_sha256.clone(),
        payload_sha256: request.payload_sha256.clone(),
        attachments_sha256: request.attachments_sha256.clone(),
        visibility_sha256: request.visibility_sha256.clone(),
        expected_effect_sha256: request.expected_effect_sha256.clone(),
        disclosure_preview_sha256: request.disclosure_preview_sha256.clone(),
        grant_sha256: request.grant_sha256.clone(),
        idempotency_key_sha256: request.idempotency_key_sha256.clone(),
        remote_state_sha256: request.submission_refresh.state_sha256.clone(),
    })
}

/// Reconciles one exact connector observation and blocks uncertain retry.
pub fn reconcile_connector_mutation(
    request: &ConnectorMutationRequest,
    preview: &ConnectorMutationPreview,
    receipt: &ConnectorMutationReceipt,
) -> Result<ConnectorMutationResult, ConnectorMutationError> {
    if admit_connector_mutation(request)? != *preview
        || receipt.request_id != request.request_id
        || receipt.external_identity_sha256 != request.submission_refresh.external_identity_sha256
        || !valid_sha256(&receipt.observed_state_sha256)
        || receipt.request_count > 1
        || receipt.retry_count != 0
        || !receipt.terminated
        || [
            receipt.observed_effect_sha256.as_deref(),
            receipt.failure_sha256.as_deref(),
            receipt.compensation_sha256.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|value| !valid_sha256(value))
    {
        return Err(ConnectorMutationError::ReceiptDenied);
    }
    match receipt.result {
        ConnectorMutationResult::VerifiedChanged
            if receipt.external_state_changed
                && receipt.request_count == 1
                && receipt.observed_effect_sha256.as_deref()
                    == Some(request.expected_effect_sha256.as_str()) =>
        {
            Ok(receipt.result)
        }
        ConnectorMutationResult::VerifiedUnchanged
            if !receipt.external_state_changed && receipt.observed_effect_sha256.is_none() =>
        {
            Ok(receipt.result)
        }
        ConnectorMutationResult::Partial | ConnectorMutationResult::Unknown => {
            Err(ConnectorMutationError::RetryBlocked)
        }
        _ => Err(ConnectorMutationError::ReceiptDenied),
    }
}

fn valid_manifest(value: &ConnectorControlManifest) -> bool {
    valid_id(&value.connector_id)
        && [
            &value.account_sha256,
            &value.threat_model_sha256,
            &value.capability_manifest_sha256,
            &value.data_classification_sha256,
            &value.credential_scope_sha256,
            &value.rate_limit_sha256,
            &value.publication_rule_sha256,
            &value.recovery_sha256,
            &value.acceptance_suite_sha256,
        ]
        .into_iter()
        .all(|value| valid_sha256(value))
}
fn valid_refresh(value: &ConnectorRemoteRefresh) -> bool {
    value.observed_epoch_milliseconds > 0
        && [
            &value.external_identity_sha256,
            &value.state_sha256,
            &value.permissions_sha256,
        ]
        .into_iter()
        .all(|value| valid_sha256(value))
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

#[cfg(test)]
mod tests {
    use super::*;
    fn hash(c: char) -> String {
        c.to_string().repeat(64)
    }
    fn manifest(connector: ConnectorKind) -> ConnectorControlManifest {
        ConnectorControlManifest {
            connector,
            connector_id: "connector-1".into(),
            account_sha256: hash('a'),
            threat_model_sha256: hash('b'),
            capability_manifest_sha256: hash('c'),
            data_classification_sha256: hash('d'),
            credential_scope_sha256: hash('e'),
            rate_limit_sha256: hash('f'),
            publication_rule_sha256: hash('1'),
            recovery_sha256: hash('2'),
            acceptance_suite_sha256: hash('3'),
        }
    }
    fn refresh() -> ConnectorRemoteRefresh {
        ConnectorRemoteRefresh {
            external_identity_sha256: hash('4'),
            state_sha256: hash('5'),
            permissions_sha256: hash('6'),
            observed_epoch_milliseconds: 1,
        }
    }
    fn request(
        connector: ConnectorKind,
        operation: ConnectorOperation,
    ) -> ConnectorMutationRequest {
        ConnectorMutationRequest {
            request_id: "request-1".into(),
            manifest: manifest(connector),
            workspace_sha256: hash('7'),
            task_sha256: hash('8'),
            context_sha256: hash('9'),
            cache_sha256: hash('a'),
            grant_sha256: hash('b'),
            derived_credential_sha256: hash('c'),
            operation,
            destination_sha256: hash('d'),
            payload_sha256: hash('e'),
            attachments_sha256: hash('f'),
            visibility_sha256: hash('1'),
            expected_effect_sha256: hash('2'),
            disclosure_preview_sha256: hash('3'),
            idempotency_key_sha256: hash('4'),
            expires_epoch_milliseconds: 2,
            now_epoch_milliseconds: 1,
            approved_refresh: refresh(),
            submission_refresh: refresh(),
            discovery_complete: true,
            draft_complete: true,
            user_reviewed: true,
        }
    }
    #[test]
    fn sprint_87_each_connector_has_an_isolated_manifest() {
        for connector in [
            ConnectorKind::Email,
            ConnectorKind::Messaging,
            ConnectorKind::Calendar,
            ConnectorKind::DocumentRepository,
            ConnectorKind::Archive,
            ConnectorKind::Database,
            ConnectorKind::Cloud,
        ] {
            assert!(
                admit_connector_mutation(&request(
                    connector,
                    ConnectorOperation::ReadOnlyDiscovery
                ))
                .is_ok()
            );
        }
    }
    #[test]
    fn sprint_87_cross_partition_or_remote_drift_is_denied() {
        let mut value = request(ConnectorKind::Email, ConnectorOperation::Draft);
        value.submission_refresh.state_sha256 = hash('0');
        assert_eq!(
            admit_connector_mutation(&value),
            Err(ConnectorMutationError::RequestDenied)
        );
    }
    #[test]
    fn sprint_87_write_requires_discovery_draft_and_review() {
        let mut value = request(ConnectorKind::Messaging, ConnectorOperation::SendMessage);
        value.discovery_complete = false;
        assert_eq!(
            admit_connector_mutation(&value),
            Err(ConnectorMutationError::RequestDenied)
        );
    }
    #[test]
    fn sprint_88_each_write_class_has_a_distinct_preview() {
        for operation in [
            ConnectorOperation::SendEmail,
            ConnectorOperation::SendMessage,
            ConnectorOperation::ChangeCalendar,
            ConnectorOperation::UpdateDocument,
            ConnectorOperation::ChangeArchive,
            ConnectorOperation::WriteDatabase,
            ConnectorOperation::ChangeCloud,
        ] {
            assert_eq!(
                admit_connector_mutation(&request(ConnectorKind::Cloud, operation))
                    .expect("preview")
                    .operation,
                operation
            );
        }
    }
    #[test]
    fn sprint_88_exact_changed_receipt_is_admitted() {
        let value = request(ConnectorKind::Database, ConnectorOperation::WriteDatabase);
        let preview = admit_connector_mutation(&value).expect("preview");
        let receipt = ConnectorMutationReceipt {
            request_id: value.request_id.clone(),
            external_identity_sha256: value.submission_refresh.external_identity_sha256.clone(),
            result: ConnectorMutationResult::VerifiedChanged,
            observed_state_sha256: hash('5'),
            observed_effect_sha256: Some(value.expected_effect_sha256.clone()),
            external_state_changed: true,
            request_count: 1,
            retry_count: 0,
            failure_sha256: None,
            compensation_sha256: None,
            terminated: true,
        };
        assert_eq!(
            reconcile_connector_mutation(&value, &preview, &receipt),
            Ok(ConnectorMutationResult::VerifiedChanged)
        );
    }
    #[test]
    fn sprint_88_partial_and_unknown_results_block_retry() {
        let value = request(ConnectorKind::Archive, ConnectorOperation::ChangeArchive);
        let preview = admit_connector_mutation(&value).expect("preview");
        for result in [
            ConnectorMutationResult::Partial,
            ConnectorMutationResult::Unknown,
        ] {
            let receipt = ConnectorMutationReceipt {
                request_id: value.request_id.clone(),
                external_identity_sha256: value.submission_refresh.external_identity_sha256.clone(),
                result,
                observed_state_sha256: hash('5'),
                observed_effect_sha256: None,
                external_state_changed: false,
                request_count: 1,
                retry_count: 0,
                failure_sha256: Some(hash('6')),
                compensation_sha256: Some(hash('7')),
                terminated: true,
            };
            assert_eq!(
                reconcile_connector_mutation(&value, &preview, &receipt),
                Err(ConnectorMutationError::RetryBlocked)
            );
        }
    }
}
