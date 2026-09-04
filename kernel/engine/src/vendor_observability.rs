//! Exact vendor telemetry identities, bounded queries, and observe-only authority.
#![allow(missing_docs)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservabilityProvider {
    Datadog,
    Prometheus,
    Grafana,
    Loki,
    Elastic,
    Splunk,
    Sentry,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderIdentity {
    pub provider: ObservabilityProvider,
    pub version: String,
    pub endpoint_sha256: String,
    pub account_sha256: String,
    pub tenant_sha256: String,
    pub datasource_sha256: String,
    pub project_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryPreview {
    pub identity: ProviderIdentity,
    pub data_scope_sha256: String,
    pub query_sha256: String,
    pub start_unix_ms: u64,
    pub end_unix_ms: u64,
    pub max_results: u32,
    pub max_bytes: u64,
    pub max_cardinality: u32,
    pub max_cost_units: u32,
    pub retention_sha256: String,
    pub expected_result_sha256: String,
    pub credential_sha256: String,
    pub cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryDiagnostic {
    pub provider: ObservabilityProvider,
    pub tenant_sha256: String,
    pub source_sha256: String,
    pub page_count: u32,
    pub result_count: u32,
    pub byte_count: u64,
    pub cost_units: u32,
    pub rate_limit_remaining: u32,
    pub quota_remaining: u32,
    pub freshness_ms: u64,
    pub partial: bool,
    pub cancelled: bool,
    pub secret_value_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedTelemetry {
    pub service_sha256: String,
    pub environment_sha256: String,
    pub release_sha256: String,
    pub deployment_sha256: String,
    pub time_window_sha256: String,
    pub common_fields_sha256: String,
    pub namespaced_extensions_sha256: String,
    pub inferred_field_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserveOperation {
    Read,
    EditMonitor,
    PublishDashboard,
    ChangeIncident,
    Notify,
    Remediate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservabilityError {
    InvalidIdentity,
    InvalidQuery,
    InvalidDiagnostic,
    InvalidNormalization,
    AuthorityEscalation,
}

pub fn validate_preview(preview: &QueryPreview) -> Result<(), ObservabilityError> {
    let identity = &preview.identity;
    if !valid_id(&identity.version)
        || [
            &identity.endpoint_sha256,
            &identity.account_sha256,
            &identity.tenant_sha256,
            &identity.datasource_sha256,
            &identity.project_sha256,
            &preview.data_scope_sha256,
            &preview.query_sha256,
            &preview.retention_sha256,
            &preview.expected_result_sha256,
            &preview.credential_sha256,
        ]
        .into_iter()
        .any(|value| !valid_sha(value))
        || preview.start_unix_ms >= preview.end_unix_ms
        || preview.max_results == 0
        || preview.max_results > 10_000
        || preview.max_bytes == 0
        || preview.max_bytes > 16 * 1024 * 1024
        || preview.max_cardinality == 0
        || preview.max_cardinality > 10_000
        || preview.max_cost_units == 0
        || preview.max_cost_units > 100_000
    {
        return Err(ObservabilityError::InvalidQuery);
    }
    Ok(())
}

pub fn validate_diagnostic(
    preview: &QueryPreview,
    diagnostic: &QueryDiagnostic,
) -> Result<(), ObservabilityError> {
    validate_preview(preview)?;
    if diagnostic.provider != preview.identity.provider
        || diagnostic.tenant_sha256 != preview.identity.tenant_sha256
        || !valid_sha(&diagnostic.source_sha256)
        || diagnostic.page_count == 0
        || diagnostic.result_count > preview.max_results
        || diagnostic.byte_count > preview.max_bytes
        || diagnostic.cost_units > preview.max_cost_units
        || diagnostic.secret_value_count != 0
        || diagnostic.cancelled != preview.cancelled
    {
        return Err(ObservabilityError::InvalidDiagnostic);
    }
    Ok(())
}

pub fn validate_normalization(value: &NormalizedTelemetry) -> Result<(), ObservabilityError> {
    if [
        &value.service_sha256,
        &value.environment_sha256,
        &value.release_sha256,
        &value.deployment_sha256,
        &value.time_window_sha256,
        &value.common_fields_sha256,
        &value.namespaced_extensions_sha256,
    ]
    .into_iter()
    .any(|field| !valid_sha(field))
        || value.inferred_field_count != 0
    {
        return Err(ObservabilityError::InvalidNormalization);
    }
    Ok(())
}

pub fn authorize_observe(operation: ObserveOperation) -> Result<(), ObservabilityError> {
    if operation != ObserveOperation::Read {
        return Err(ObservabilityError::AuthorityEscalation);
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview() -> QueryPreview {
        let hash = "a".repeat(64);
        QueryPreview {
            identity: ProviderIdentity {
                provider: ObservabilityProvider::Datadog,
                version: "1.0.0".into(),
                endpoint_sha256: hash.clone(),
                account_sha256: hash.clone(),
                tenant_sha256: hash.clone(),
                datasource_sha256: hash.clone(),
                project_sha256: hash.clone(),
            },
            data_scope_sha256: hash.clone(),
            query_sha256: hash.clone(),
            start_unix_ms: 1_000,
            end_unix_ms: 2_000,
            max_results: 100,
            max_bytes: 4096,
            max_cardinality: 50,
            max_cost_units: 100,
            retention_sha256: hash.clone(),
            expected_result_sha256: hash.clone(),
            credential_sha256: hash,
            cancelled: false,
        }
    }

    #[test]
    fn exact_bounded_preview_is_valid() {
        assert_eq!(validate_preview(&preview()), Ok(()));
    }

    #[test]
    fn cross_tenant_and_cost_overrun_fail_closed() {
        let preview = preview();
        let mut diagnostic = QueryDiagnostic {
            provider: ObservabilityProvider::Datadog,
            tenant_sha256: "b".repeat(64),
            source_sha256: "c".repeat(64),
            page_count: 1,
            result_count: 1,
            byte_count: 1,
            cost_units: 1,
            rate_limit_remaining: 1,
            quota_remaining: 1,
            freshness_ms: 1,
            partial: false,
            cancelled: false,
            secret_value_count: 0,
        };
        assert_eq!(
            validate_diagnostic(&preview, &diagnostic),
            Err(ObservabilityError::InvalidDiagnostic)
        );
        diagnostic.tenant_sha256 = preview.identity.tenant_sha256.clone();
        diagnostic.cost_units = preview.max_cost_units + 1;
        assert_eq!(
            validate_diagnostic(&preview, &diagnostic),
            Err(ObservabilityError::InvalidDiagnostic)
        );
    }

    #[test]
    fn vendor_extensions_cannot_be_guessed() {
        let hash = "d".repeat(64);
        let mut normalized = NormalizedTelemetry {
            service_sha256: hash.clone(),
            environment_sha256: hash.clone(),
            release_sha256: hash.clone(),
            deployment_sha256: hash.clone(),
            time_window_sha256: hash.clone(),
            common_fields_sha256: hash.clone(),
            namespaced_extensions_sha256: hash,
            inferred_field_count: 0,
        };
        assert_eq!(validate_normalization(&normalized), Ok(()));
        normalized.inferred_field_count = 1;
        assert_eq!(
            validate_normalization(&normalized),
            Err(ObservabilityError::InvalidNormalization)
        );
    }

    #[test]
    fn observe_authority_refuses_every_write() {
        assert_eq!(authorize_observe(ObserveOperation::Read), Ok(()));
        for operation in [
            ObserveOperation::EditMonitor,
            ObserveOperation::PublishDashboard,
            ObserveOperation::ChangeIncident,
            ObserveOperation::Notify,
            ObserveOperation::Remediate,
        ] {
            assert_eq!(
                authorize_observe(operation),
                Err(ObservabilityError::AuthorityEscalation)
            );
        }
    }
}
