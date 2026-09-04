//! Bounded telemetry identity, correlation, aggregation, and authority refusal.
#![allow(missing_docs)]
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalKind {
    Trace,
    Span,
    Metric,
    Log,
    Event,
    Error,
    Monitor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipKind {
    ExactAttribute,
    TemporalInference,
    StructuralInference,
    DeterministicCausation,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryQuery {
    pub provider_sha256: String,
    pub service_sha256: String,
    pub environment_sha256: String,
    pub start_unix_ms: u64,
    pub end_unix_ms: u64,
    pub max_results: u32,
    pub max_bytes: u64,
    pub max_cardinality: u32,
    pub sampling_sha256: String,
    pub clock_skew_ms: u32,
    pub retention_sha256: String,
    pub classification_sha256: String,
    pub cancelled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelemetryObservation {
    pub observation_id: String,
    pub kind: SignalKind,
    pub resource_sha256: String,
    pub service_sha256: String,
    pub environment_sha256: String,
    pub provider_sha256: String,
    pub release_sha256: String,
    pub deployment_sha256: String,
    pub artifact_digest: String,
    pub incident_sha256: Option<String>,
    pub timestamp_unix_ms: u64,
    pub content_untrusted: bool,
    pub secret_scan_passed: bool,
    pub persisted: bool,
    pub model_delivered: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correlation {
    pub left_id: String,
    pub right_id: String,
    pub relationship: RelationshipKind,
    pub evidence_sha256: Option<String>,
    pub source_citation_sha256: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StatisticalSummary {
    pub method: String,
    pub sample_count: u64,
    pub missing_count: u64,
    pub sampling_rate: f64,
    pub uncertainty: f64,
    pub unit: String,
    pub source_citations: BTreeMap<String, String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryError {
    InvalidQuery,
    InvalidObservation,
    UnsupportedCausation,
    AuthorityEscalation,
    InvalidSummary,
}
pub fn validate_query(q: &TelemetryQuery) -> Result<(), TelemetryError> {
    if [
        &q.provider_sha256,
        &q.service_sha256,
        &q.environment_sha256,
        &q.sampling_sha256,
        &q.retention_sha256,
        &q.classification_sha256,
    ]
    .into_iter()
    .any(|v| !valid_sha(v))
        || q.start_unix_ms >= q.end_unix_ms
        || q.max_results == 0
        || q.max_results > 10_000
        || q.max_bytes == 0
        || q.max_bytes > 16 * 1024 * 1024
        || q.max_cardinality == 0
        || q.max_cardinality > 10_000
        || q.clock_skew_ms > 300_000
    {
        return Err(TelemetryError::InvalidQuery);
    }
    Ok(())
}
pub fn validate_observation(
    o: &TelemetryObservation,
    q: &TelemetryQuery,
) -> Result<(), TelemetryError> {
    validate_query(q)?;
    if !valid_id(&o.observation_id)
        || [
            &o.resource_sha256,
            &o.service_sha256,
            &o.environment_sha256,
            &o.provider_sha256,
            &o.release_sha256,
            &o.deployment_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
        || !valid_digest(&o.artifact_digest)
        || o.incident_sha256.as_ref().is_some_and(|v| !valid_sha(v))
        || o.timestamp_unix_ms < q.start_unix_ms.saturating_sub(q.clock_skew_ms.into())
        || o.timestamp_unix_ms > q.end_unix_ms + u64::from(q.clock_skew_ms)
        || !o.content_untrusted
        || !o.secret_scan_passed
    {
        return Err(TelemetryError::InvalidObservation);
    }
    Ok(())
}
pub fn validate_correlation(c: &Correlation) -> Result<(), TelemetryError> {
    if !valid_id(&c.left_id) || !valid_id(&c.right_id) || !valid_sha(&c.source_citation_sha256) {
        return Err(TelemetryError::InvalidObservation);
    }
    if c.relationship == RelationshipKind::DeterministicCausation
        && c.evidence_sha256.as_ref().is_none_or(|v| !valid_sha(v))
    {
        return Err(TelemetryError::UnsupportedCausation);
    }
    Ok(())
}
pub fn validate_summary(s: &StatisticalSummary) -> Result<(), TelemetryError> {
    if !valid_id(&s.method)
        || s.sample_count == 0
        || s.missing_count > s.sample_count
        || !s.sampling_rate.is_finite()
        || !(0.0..=1.0).contains(&s.sampling_rate)
        || !s.uncertainty.is_finite()
        || s.uncertainty < 0.0
        || !valid_id(&s.unit)
        || s.source_citations.is_empty()
        || s.source_citations
            .iter()
            .any(|(k, v)| !valid_id(k) || !valid_sha(v))
    {
        return Err(TelemetryError::InvalidSummary);
    }
    Ok(())
}
pub fn operation_authorized_by_telemetry(_: &TelemetryObservation) -> Result<(), TelemetryError> {
    Err(TelemetryError::AuthorityEscalation)
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 512
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn valid_digest(v: &str) -> bool {
    v.starts_with("sha256:") && valid_sha(&v[7..])
}
#[cfg(test)]
mod tests {
    use super::*;
    fn query() -> TelemetryQuery {
        let h = "a".repeat(64);
        TelemetryQuery {
            provider_sha256: h.clone(),
            service_sha256: h.clone(),
            environment_sha256: h.clone(),
            start_unix_ms: 1000,
            end_unix_ms: 2000,
            max_results: 100,
            max_bytes: 4096,
            max_cardinality: 50,
            sampling_sha256: h.clone(),
            clock_skew_ms: 10,
            retention_sha256: h.clone(),
            classification_sha256: h,
            cancelled: false,
        }
    }
    fn observation() -> TelemetryObservation {
        let h = "b".repeat(64);
        TelemetryObservation {
            observation_id: "obs-1".into(),
            kind: SignalKind::Metric,
            resource_sha256: h.clone(),
            service_sha256: h.clone(),
            environment_sha256: h.clone(),
            provider_sha256: h.clone(),
            release_sha256: h.clone(),
            deployment_sha256: h.clone(),
            artifact_digest: format!("sha256:{h}"),
            incident_sha256: Some(h),
            timestamp_unix_ms: 1500,
            content_untrusted: true,
            secret_scan_passed: true,
            persisted: false,
            model_delivered: false,
        }
    }
    #[test]
    fn bounded_query_and_identity_are_valid() {
        let q = query();
        assert_eq!(validate_query(&q), Ok(()));
        assert_eq!(validate_observation(&observation(), &q), Ok(()));
    }
    #[test]
    fn cardinality_and_secret_fail_closed() {
        let mut q = query();
        q.max_cardinality = 10001;
        assert_eq!(validate_query(&q), Err(TelemetryError::InvalidQuery));
        let mut o = observation();
        o.secret_scan_passed = false;
        assert_eq!(
            validate_observation(&o, &query()),
            Err(TelemetryError::InvalidObservation)
        );
    }
    #[test]
    fn inference_is_not_causation() {
        let h = "c".repeat(64);
        let mut c = Correlation {
            left_id: "deployment-1".into(),
            right_id: "error-1".into(),
            relationship: RelationshipKind::TemporalInference,
            evidence_sha256: None,
            source_citation_sha256: h,
        };
        assert_eq!(validate_correlation(&c), Ok(()));
        c.relationship = RelationshipKind::DeterministicCausation;
        assert_eq!(
            validate_correlation(&c),
            Err(TelemetryError::UnsupportedCausation)
        );
    }
    #[test]
    fn telemetry_never_grants_operations() {
        assert_eq!(
            operation_authorized_by_telemetry(&observation()),
            Err(TelemetryError::AuthorityEscalation)
        );
    }
}
