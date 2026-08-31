//! Versioned gateway codec capabilities and deterministic no-silent-fallback routing.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, CanonicalEndpointClass};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_ROUTES: usize = 64;
const MAX_EVENTS: usize = 16_384;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Explicit gateway compatibility or routing refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayRoutingError {
    /// A version, identity, digest, bound, or collection is malformed.
    InvalidInput,
    /// The codec cannot preserve the supplied canonical semantic.
    UnsupportedSemantic,
    /// Stream order, identity, terminality, or usage is malformed.
    MalformedStream,
    /// No current exact qualified route satisfies the request.
    NoQualifiedRoute,
    /// A fallback was absent, stale, unordered, unauthorized, or control-inequivalent.
    FallbackDenied,
    /// Canonical hashing failed.
    SerializationFailed,
}

/// Exact capabilities of one protocol codec implementation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayCodecCapabilities {
    /// Codec identity.
    pub codec_id: String,
    /// Exact version.
    pub codec_version: String,
    /// Implementation/schema digest.
    pub codec_sha256: String,
    /// Message-part semantics supported exactly.
    pub message_parts: BTreeSet<GatewayMessagePartKind>,
    /// Ordered streaming is supported.
    pub streaming: bool,
    /// Structured output is preserved exactly.
    pub structured_output: bool,
    /// Tool proposals are returned as inert proposals.
    pub tool_proposals: bool,
    /// Usage accounting events are preserved.
    pub usage: bool,
    /// Cancellation is preserved.
    pub cancellation: bool,
    /// Current health can be observed.
    pub health: bool,
    /// Maximum admitted concurrent requests.
    pub max_concurrency: u16,
    /// Digest of the closed typed-failure map.
    pub failure_map_sha256: String,
}

/// Canonical message part kinds at the gateway boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayMessagePartKind {
    /// UTF-8 text.
    Text,
    /// Schema-bound JSON value.
    Structured,
    /// Inert tool-call proposal.
    ToolProposal,
}

/// One ordered canonical gateway event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayCanonicalEvent {
    /// Exact request identity.
    pub request_id: String,
    /// Zero-based contiguous sequence.
    pub sequence: u32,
    /// Closed event payload.
    pub kind: GatewayCanonicalEventKind,
}

/// Closed canonical event payload; unknown external semantics map to `Unsupported`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayCanonicalEventKind {
    /// One preserved response part.
    Part {
        /// Canonical part semantic.
        part: GatewayMessagePartKind,
        /// Digest of the exact inert payload.
        payload_sha256: String,
    },
    /// Cumulative usage accounting.
    Usage {
        /// Counted input tokens.
        input_tokens: u64,
        /// Counted output tokens.
        output_tokens: u64,
    },
    /// Cancellation terminal.
    Cancelled,
    /// Successful protocol terminal; this is not workflow completion.
    Completed,
    /// Typed protocol failure terminal.
    Failed {
        /// Stable typed failure code.
        failure_code: String,
    },
    /// Unknown or semantically unsupported external event.
    Unsupported {
        /// Exact unknown external event type.
        external_type: String,
    },
}

/// Validates exact field/event preservation or returns a visible non-success.
pub fn validate_gateway_event_stream(
    capabilities: &GatewayCodecCapabilities,
    events: &[GatewayCanonicalEvent],
) -> Result<(), GatewayRoutingError> {
    validate_capabilities(capabilities)?;
    if events.is_empty()
        || events.len() > MAX_EVENTS
        || (!capabilities.streaming && events.len() > 2)
    {
        return Err(GatewayRoutingError::MalformedStream);
    }
    let request_id = events[0].request_id.as_str();
    let mut terminal = false;
    for (index, event) in events.iter().enumerate() {
        if terminal
            || !valid_id(&event.request_id)
            || event.request_id != request_id
            || event.sequence as usize != index
        {
            return Err(GatewayRoutingError::MalformedStream);
        }
        match &event.kind {
            GatewayCanonicalEventKind::Part {
                part,
                payload_sha256,
            } => {
                if !valid_sha256(payload_sha256)
                    || !capabilities.message_parts.contains(part)
                    || (*part == GatewayMessagePartKind::Structured
                        && !capabilities.structured_output)
                    || (*part == GatewayMessagePartKind::ToolProposal
                        && !capabilities.tool_proposals)
                {
                    return Err(GatewayRoutingError::UnsupportedSemantic);
                }
            }
            GatewayCanonicalEventKind::Usage { .. } if !capabilities.usage => {
                return Err(GatewayRoutingError::UnsupportedSemantic);
            }
            GatewayCanonicalEventKind::Cancelled if !capabilities.cancellation => {
                return Err(GatewayRoutingError::UnsupportedSemantic);
            }
            GatewayCanonicalEventKind::Unsupported { .. } => {
                return Err(GatewayRoutingError::UnsupportedSemantic);
            }
            GatewayCanonicalEventKind::Cancelled
            | GatewayCanonicalEventKind::Completed
            | GatewayCanonicalEventKind::Failed { .. } => terminal = true,
            GatewayCanonicalEventKind::Usage { .. } => {}
        }
    }
    if terminal {
        Ok(())
    } else {
        Err(GatewayRoutingError::MalformedStream)
    }
}

/// Exact currently activated route considered by deterministic policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QualifiedGatewayRoute {
    /// Stable route identity.
    pub route_id: String,
    /// Whole disabled candidate tuple digest activated by a separate record.
    pub candidate_sha256: String,
    /// Deployment/disclosure class.
    pub endpoint_class: CanonicalEndpointClass,
    /// Exact role identity.
    pub role_id: String,
    /// Exact capability-set digest.
    pub capability_sha256: String,
    /// Exact platform policy identity.
    pub platform_id: String,
    /// Exact qualification digest.
    pub qualification_sha256: String,
    /// Exact invariant-control digest.
    pub invariant_controls_sha256: String,
    /// Maximum context admitted.
    pub max_context_tokens: u32,
    /// Maximum concurrent requests.
    pub max_concurrency: u16,
    /// Whether a separately authorized activation is current.
    pub enabled: bool,
    /// Current health observation.
    pub healthy: bool,
    /// Current resource fit.
    pub resources_available: bool,
    /// Current quota fit.
    pub quota_available: bool,
    /// Current cost-policy admission.
    pub cost_admitted: bool,
}

/// Explicit ordered fallback policy and destination authorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExplicitFallbackPolicy {
    /// Policy identity.
    pub policy_id: String,
    /// Policy digest.
    pub policy_sha256: String,
    /// Exact allowed route order, beginning with the prior route.
    pub ordered_route_ids: Vec<String>,
    /// Exact independently authorized destination route.
    pub authorized_destination_route_id: String,
    /// User-visible disclosure for the destination was accepted.
    pub destination_disclosure_accepted: bool,
    /// Destination was independently checked for equivalent invariant controls.
    pub equivalent_controls_required: bool,
}

/// Exact deterministic routing request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayRoutingRequest {
    /// Request identity.
    pub request_id: String,
    /// User-selected profile/policy identity.
    pub user_profile_id: String,
    /// Deterministic classification digest.
    pub classification_sha256: String,
    /// Required role.
    pub role_id: String,
    /// Required capability-set digest.
    pub capability_sha256: String,
    /// Current platform policy identity.
    pub platform_id: String,
    /// Maximum disclosure class.
    pub maximum_endpoint_class: CanonicalEndpointClass,
    /// Whether the requested disclosure boundary was accepted.
    pub disclosure_accepted: bool,
    /// Required context tokens.
    pub required_context_tokens: u32,
    /// Current concurrent request count.
    pub current_concurrency: u16,
    /// Route policy digest.
    pub route_policy_sha256: String,
    /// Prior failed route only for explicit fallback evaluation.
    pub prior_route_id: Option<String>,
    /// Explicit fallback policy; absent by default.
    pub fallback_policy: Option<ExplicitFallbackPolicy>,
}

/// Visible audit for one considered route.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayRouteAudit {
    /// Route identity.
    pub route_id: String,
    /// Exact candidate digest.
    pub candidate_sha256: String,
    /// Whether the route satisfied every current deterministic gate.
    pub eligible: bool,
    /// Stable visible reason.
    pub reason_code: &'static str,
}

/// Complete deterministic route receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GatewayRouteReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Request identity.
    pub request_id: String,
    /// Route-policy digest.
    pub route_policy_sha256: String,
    /// Ordered audit of every candidate.
    pub considered_routes: Vec<GatewayRouteAudit>,
    /// Exact selected route, or none when blocked.
    pub selected_route_id: Option<String>,
    /// Whether the selected route is an explicit fallback.
    pub fallback_used: bool,
    /// Fallback policy digest only when used.
    pub fallback_policy_sha256: Option<String>,
    /// Selected disclosure class, absent when blocked.
    pub disclosure_class: Option<CanonicalEndpointClass>,
    /// Stable terminal reason.
    pub reason_code: &'static str,
    /// Digest of this receipt with this field zeroed.
    pub receipt_sha256: String,
}

/// Selects only one current exact route, with fallback disabled unless fully explicit.
pub fn route_gateway(
    request: &GatewayRoutingRequest,
    routes: &[QualifiedGatewayRoute],
) -> Result<GatewayRouteReceipt, GatewayRoutingError> {
    validate_request(request)?;
    if routes.is_empty() || routes.len() > MAX_ROUTES {
        return Err(GatewayRoutingError::NoQualifiedRoute);
    }
    let mut ordered = routes.to_vec();
    ordered.sort_by_key(|route| (class_rank(route.endpoint_class), route.route_id.clone()));
    if ordered
        .windows(2)
        .any(|pair| pair[0].route_id == pair[1].route_id)
    {
        return Err(GatewayRoutingError::InvalidInput);
    }
    let audits = ordered
        .iter()
        .map(|route| audit_route(request, route))
        .collect::<Result<Vec<_>, _>>()?;
    let eligible = ordered
        .iter()
        .zip(&audits)
        .filter(|(_, audit)| audit.eligible)
        .map(|(route, _)| route)
        .collect::<Vec<_>>();
    let selected = if let Some(prior) = &request.prior_route_id {
        select_fallback(request, prior, &ordered, &eligible)?
    } else {
        if request.fallback_policy.is_some() {
            return Err(GatewayRoutingError::FallbackDenied);
        }
        eligible
            .first()
            .copied()
            .ok_or(GatewayRoutingError::NoQualifiedRoute)?
    };
    let fallback_used = request.prior_route_id.is_some();
    let mut receipt = GatewayRouteReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        route_policy_sha256: request.route_policy_sha256.clone(),
        considered_routes: audits,
        selected_route_id: Some(selected.route_id.clone()),
        fallback_used,
        fallback_policy_sha256: request
            .fallback_policy
            .as_ref()
            .map(|policy| policy.policy_sha256.clone()),
        disclosure_class: Some(selected.endpoint_class),
        reason_code: if fallback_used {
            "model-gateway.route.explicit-fallback-selected"
        } else {
            "model-gateway.route.qualified-selected"
        },
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    receipt.receipt_sha256 = receipt_digest(&receipt)?;
    Ok(receipt)
}

fn audit_route(
    request: &GatewayRoutingRequest,
    route: &QualifiedGatewayRoute,
) -> Result<GatewayRouteAudit, GatewayRoutingError> {
    validate_route(route)?;
    let (eligible, reason) = if !route.enabled {
        (false, "model-gateway.route.disabled")
    } else if !route.healthy {
        (false, "model-gateway.route.unhealthy")
    } else if !route.resources_available {
        (false, "model-gateway.route.resource-unavailable")
    } else if !route.quota_available {
        (false, "model-gateway.route.quota-unavailable")
    } else if !route.cost_admitted {
        (false, "model-gateway.route.cost-denied")
    } else if route.role_id != request.role_id
        || route.capability_sha256 != request.capability_sha256
    {
        (false, "model-gateway.route.capability-mismatch")
    } else if route.platform_id != request.platform_id {
        (false, "model-gateway.route.platform-mismatch")
    } else if route.max_context_tokens < request.required_context_tokens
        || request.current_concurrency >= route.max_concurrency
    {
        (false, "model-gateway.route.limit-exceeded")
    } else if class_rank(route.endpoint_class) > class_rank(request.maximum_endpoint_class)
        || (route.endpoint_class != CanonicalEndpointClass::StrictLocal
            && !request.disclosure_accepted)
    {
        (false, "model-gateway.route.disclosure-denied")
    } else {
        (true, "model-gateway.route.eligible")
    };
    Ok(GatewayRouteAudit {
        route_id: route.route_id.clone(),
        candidate_sha256: route.candidate_sha256.clone(),
        eligible,
        reason_code: reason,
    })
}

fn select_fallback<'a>(
    request: &GatewayRoutingRequest,
    prior: &str,
    all_routes: &[QualifiedGatewayRoute],
    eligible: &[&'a QualifiedGatewayRoute],
) -> Result<&'a QualifiedGatewayRoute, GatewayRoutingError> {
    let policy = request
        .fallback_policy
        .as_ref()
        .ok_or(GatewayRoutingError::FallbackDenied)?;
    if !valid_id(&policy.policy_id)
        || !valid_sha256(&policy.policy_sha256)
        || policy.ordered_route_ids.len() < 2
        || policy.ordered_route_ids.first().map(String::as_str) != Some(prior)
        || !policy.destination_disclosure_accepted
        || !policy.equivalent_controls_required
    {
        return Err(GatewayRoutingError::FallbackDenied);
    }
    let destination_index = policy
        .ordered_route_ids
        .iter()
        .position(|item| item == &policy.authorized_destination_route_id)
        .ok_or(GatewayRoutingError::FallbackDenied)?;
    if destination_index == 0 {
        return Err(GatewayRoutingError::FallbackDenied);
    }
    let selected = eligible
        .iter()
        .copied()
        .find(|route| route.route_id == policy.authorized_destination_route_id)
        .ok_or(GatewayRoutingError::FallbackDenied)?;
    let prior_controls = all_routes
        .iter()
        .find(|route| route.route_id == prior)
        .map(|route| route.invariant_controls_sha256.as_str());
    if prior_controls.is_none_or(|digest| digest != selected.invariant_controls_sha256) {
        return Err(GatewayRoutingError::FallbackDenied);
    }
    Ok(selected)
}

fn validate_capabilities(value: &GatewayCodecCapabilities) -> Result<(), GatewayRoutingError> {
    if valid_id(&value.codec_id)
        && valid_id(&value.codec_version)
        && valid_sha256(&value.codec_sha256)
        && valid_sha256(&value.failure_map_sha256)
        && !value.message_parts.is_empty()
        && value.max_concurrency > 0
    {
        Ok(())
    } else {
        Err(GatewayRoutingError::InvalidInput)
    }
}
fn validate_request(value: &GatewayRoutingRequest) -> Result<(), GatewayRoutingError> {
    if valid_id(&value.request_id)
        && valid_id(&value.user_profile_id)
        && valid_id(&value.role_id)
        && valid_id(&value.platform_id)
        && valid_sha256(&value.classification_sha256)
        && valid_sha256(&value.capability_sha256)
        && valid_sha256(&value.route_policy_sha256)
        && value.required_context_tokens > 0
    {
        Ok(())
    } else {
        Err(GatewayRoutingError::InvalidInput)
    }
}
fn validate_route(value: &QualifiedGatewayRoute) -> Result<(), GatewayRoutingError> {
    if [
        value.candidate_sha256.as_str(),
        value.capability_sha256.as_str(),
        value.qualification_sha256.as_str(),
        value.invariant_controls_sha256.as_str(),
    ]
    .into_iter()
    .all(valid_sha256)
        && [
            value.route_id.as_str(),
            value.role_id.as_str(),
            value.platform_id.as_str(),
        ]
        .into_iter()
        .all(valid_id)
        && value.max_context_tokens > 0
        && value.max_concurrency > 0
    {
        Ok(())
    } else {
        Err(GatewayRoutingError::InvalidInput)
    }
}
fn class_rank(value: CanonicalEndpointClass) -> u8 {
    match value {
        CanonicalEndpointClass::StrictLocal => 0,
        CanonicalEndpointClass::LocalNetworkPrivate => 1,
        CanonicalEndpointClass::RemotePrivate => 2,
        CanonicalEndpointClass::RemoteManaged => 3,
    }
}
fn receipt_digest(value: &GatewayRouteReceipt) -> Result<String, GatewayRoutingError> {
    let mut candidate = value.clone();
    candidate.receipt_sha256 = ZERO_SHA256.to_owned();
    serde_json::to_vec(&candidate)
        .map(|bytes| hex(&Sha256::digest(bytes)))
        .map_err(|_| GatewayRoutingError::SerializationFailed)
}
fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.contains('\0')
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }
    fn capabilities() -> GatewayCodecCapabilities {
        GatewayCodecCapabilities {
            codec_id: "codec-1".to_owned(),
            codec_version: "1".to_owned(),
            codec_sha256: hash('1'),
            message_parts: [
                GatewayMessagePartKind::Text,
                GatewayMessagePartKind::Structured,
                GatewayMessagePartKind::ToolProposal,
            ]
            .into_iter()
            .collect(),
            streaming: true,
            structured_output: true,
            tool_proposals: true,
            usage: true,
            cancellation: true,
            health: true,
            max_concurrency: 2,
            failure_map_sha256: hash('2'),
        }
    }
    fn event(sequence: u32, kind: GatewayCanonicalEventKind) -> GatewayCanonicalEvent {
        GatewayCanonicalEvent {
            request_id: "request-1".to_owned(),
            sequence,
            kind,
        }
    }
    fn route(id: &str, class: CanonicalEndpointClass) -> QualifiedGatewayRoute {
        QualifiedGatewayRoute {
            route_id: id.to_owned(),
            candidate_sha256: hash('3'),
            endpoint_class: class,
            role_id: "role-coding".to_owned(),
            capability_sha256: hash('4'),
            platform_id: "fedora-x86-64".to_owned(),
            qualification_sha256: hash('5'),
            invariant_controls_sha256: hash('6'),
            max_context_tokens: 8192,
            max_concurrency: 2,
            enabled: true,
            healthy: true,
            resources_available: true,
            quota_available: true,
            cost_admitted: true,
        }
    }
    fn request() -> GatewayRoutingRequest {
        GatewayRoutingRequest {
            request_id: "request-1".to_owned(),
            user_profile_id: "user-profile-1".to_owned(),
            classification_sha256: hash('7'),
            role_id: "role-coding".to_owned(),
            capability_sha256: hash('4'),
            platform_id: "fedora-x86-64".to_owned(),
            maximum_endpoint_class: CanonicalEndpointClass::StrictLocal,
            disclosure_accepted: false,
            required_context_tokens: 4096,
            current_concurrency: 0,
            route_policy_sha256: hash('8'),
            prior_route_id: None,
            fallback_policy: None,
        }
    }

    #[test]
    fn story_13_6_codec_preserves_supported_ordered_semantics() {
        let events = vec![
            event(
                0,
                GatewayCanonicalEventKind::Part {
                    part: GatewayMessagePartKind::Text,
                    payload_sha256: hash('9'),
                },
            ),
            event(
                1,
                GatewayCanonicalEventKind::Usage {
                    input_tokens: 10,
                    output_tokens: 2,
                },
            ),
            event(2, GatewayCanonicalEventKind::Completed),
        ];
        assert_eq!(
            validate_gateway_event_stream(&capabilities(), &events),
            Ok(())
        );
    }
    #[test]
    fn story_13_6_unknown_malformed_and_unsupported_streams_fail_visibly() {
        let unknown = vec![event(
            0,
            GatewayCanonicalEventKind::Unsupported {
                external_type: "provider.new-event".to_owned(),
            },
        )];
        assert_eq!(
            validate_gateway_event_stream(&capabilities(), &unknown),
            Err(GatewayRoutingError::UnsupportedSemantic)
        );
        let malformed = vec![event(1, GatewayCanonicalEventKind::Completed)];
        assert_eq!(
            validate_gateway_event_stream(&capabilities(), &malformed),
            Err(GatewayRoutingError::MalformedStream)
        );
        let mut no_cancel = capabilities();
        no_cancel.cancellation = false;
        assert_eq!(
            validate_gateway_event_stream(
                &no_cancel,
                &[event(0, GatewayCanonicalEventKind::Cancelled)]
            ),
            Err(GatewayRoutingError::UnsupportedSemantic)
        );
    }
    #[test]
    fn story_13_6_route_selection_audits_health_quota_limits_and_disclosure() {
        let good = route("route-b", CanonicalEndpointClass::StrictLocal);
        let mut unhealthy = route("route-a", CanonicalEndpointClass::StrictLocal);
        unhealthy.healthy = false;
        let mut quota = route("route-c", CanonicalEndpointClass::StrictLocal);
        quota.quota_available = false;
        let receipt =
            route_gateway(&request(), &[quota, good, unhealthy]).expect("one qualified route");
        assert_eq!(receipt.selected_route_id.as_deref(), Some("route-b"));
        assert_eq!(receipt.considered_routes.len(), 3);
        assert!(!receipt.fallback_used);
    }
    #[test]
    fn story_13_6_fallback_is_denied_by_default_and_requires_exact_equivalent_policy() {
        let prior = route("route-local", CanonicalEndpointClass::RemotePrivate);
        let destination = route("route-remote", CanonicalEndpointClass::RemotePrivate);
        let mut value = request();
        value.maximum_endpoint_class = CanonicalEndpointClass::RemotePrivate;
        value.disclosure_accepted = true;
        value.prior_route_id = Some(prior.route_id.clone());
        assert_eq!(
            route_gateway(&value, &[prior.clone(), destination.clone()]),
            Err(GatewayRoutingError::FallbackDenied)
        );
        value.fallback_policy = Some(ExplicitFallbackPolicy {
            policy_id: "fallback-1".to_owned(),
            policy_sha256: hash('9'),
            ordered_route_ids: vec![prior.route_id.clone(), destination.route_id.clone()],
            authorized_destination_route_id: destination.route_id.clone(),
            destination_disclosure_accepted: true,
            equivalent_controls_required: true,
        });
        let receipt = route_gateway(&value, &[prior, destination]).expect("explicit fallback");
        assert!(receipt.fallback_used);
        assert_eq!(receipt.selected_route_id.as_deref(), Some("route-remote"));
    }
    #[test]
    fn story_13_6_health_quota_and_control_drift_cannot_silently_fallback() {
        let prior = route("route-local", CanonicalEndpointClass::RemotePrivate);
        let mut destination = route("route-remote", CanonicalEndpointClass::RemotePrivate);
        destination.invariant_controls_sha256 = hash('a');
        let mut value = request();
        value.maximum_endpoint_class = CanonicalEndpointClass::RemotePrivate;
        value.disclosure_accepted = true;
        value.prior_route_id = Some(prior.route_id.clone());
        value.fallback_policy = Some(ExplicitFallbackPolicy {
            policy_id: "fallback-1".to_owned(),
            policy_sha256: hash('9'),
            ordered_route_ids: vec![prior.route_id.clone(), destination.route_id.clone()],
            authorized_destination_route_id: destination.route_id.clone(),
            destination_disclosure_accepted: true,
            equivalent_controls_required: true,
        });
        assert_eq!(
            route_gateway(&value, &[prior, destination]),
            Err(GatewayRoutingError::FallbackDenied)
        );
    }
}
