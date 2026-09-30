//! Versioned gateway codec capabilities and deterministic no-silent-fallback routing.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, CanonicalEndpointClass};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_ROUTES: usize = 64;
const MAX_HYBRID_GRANTS: usize = 16;
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

/// Closed class of data a routed model request transmits (Decision 0124).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutedDataClass {
    /// The person's instructions and conversation turns.
    Conversation,
    /// Excerpts of workspace files.
    WorkspaceExcerpts,
    /// Outputs of tools and commands.
    ToolOutputs,
    /// Retrieved sources, documents and attachments.
    RetrievedSources,
    /// Loaded project memory.
    Memory,
}

/// Whether a request may leave the local network at all (Decision 0124).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayRoutingMode {
    /// No remote route is eligible, whatever grants exist.
    LocalOnly,
    /// A remote route is eligible only under its own separately granted route.
    Hybrid,
}

/// A separately granted remote route (Decision 0124). The person grants it
/// for one exact route and candidate, names the provider, the data classes it
/// may receive and a budget, and says whether it may replace a failed route.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HybridRouteGrant {
    /// Grant identity.
    pub grant_id: String,
    /// The exact granted route.
    pub route_id: String,
    /// The exact granted candidate tuple.
    pub candidate_sha256: String,
    /// The provider named to the person.
    pub provider_id: String,
    /// Every data class the route may receive.
    pub data_classes: BTreeSet<RoutedDataClass>,
    /// Most requests the grant admits in total.
    pub max_requests: u32,
    /// Most input tokens the grant admits in total.
    pub max_input_tokens: u64,
    /// Whether the route may serve as an explicit fallback for another route.
    pub fallback_allowed: bool,
    /// Exclusive expiry.
    pub expires_at_epoch_ms: u64,
    /// Digest of this grant with this field zeroed.
    pub grant_sha256: String,
}

/// One grant and what its owner counted against it so far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GrantedHybridRoute {
    /// The exact grant.
    pub grant: HybridRouteGrant,
    /// Requests already admitted under it.
    pub used_requests: u32,
    /// Input tokens already admitted under it.
    pub used_input_tokens: u64,
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
    /// Whether remote routes may be considered at all.
    pub mode: GatewayRoutingMode,
    /// Every data class this request transmits.
    pub transmitted_data: BTreeSet<RoutedDataClass>,
    /// Separately granted remote routes, at most one per route.
    pub hybrid_routes: Vec<GrantedHybridRoute>,
    /// Current time, for grant expiry.
    pub now_epoch_ms: u64,
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
    /// Routing mode the request was decided under.
    pub mode: GatewayRoutingMode,
    /// Data classes the request transmits to the selected route.
    pub transmitted_data: Vec<RoutedDataClass>,
    /// Grant digest when the selected route is remote.
    pub hybrid_grant_sha256: Option<String>,
    /// Provider named by that grant.
    pub provider_id: Option<String>,
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
    let grant = remote(selected.endpoint_class)
        .then(|| granted_route(request, selected))
        .flatten()
        .map(|granted| &granted.grant);
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
        mode: request.mode,
        transmitted_data: request.transmitted_data.iter().copied().collect(),
        hybrid_grant_sha256: grant.map(|grant| grant.grant_sha256.clone()),
        provider_id: grant.map(|grant| grant.provider_id.clone()),
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
    } else if remote(route.endpoint_class) {
        audit_hybrid_grant(request, route)
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

/// A remote route is eligible only in hybrid mode and only under its own
/// grant for this exact route and candidate, before the grant expires, for
/// data the grant covers, within its budget and, when replacing a failed
/// route, only if the grant allows that (Decision 0124).
fn audit_hybrid_grant(
    request: &GatewayRoutingRequest,
    route: &QualifiedGatewayRoute,
) -> (bool, &'static str) {
    if request.mode == GatewayRoutingMode::LocalOnly {
        return (false, "model-gateway.route.local-only");
    }
    let Some(granted) = granted_route(request, route) else {
        return (false, "model-gateway.route.grant-missing");
    };
    let grant = &granted.grant;
    if grant.candidate_sha256 != route.candidate_sha256 {
        (false, "model-gateway.route.grant-mismatch")
    } else if request.now_epoch_ms >= grant.expires_at_epoch_ms {
        (false, "model-gateway.route.grant-expired")
    } else if !request.transmitted_data.is_subset(&grant.data_classes) {
        (false, "model-gateway.route.grant-data-denied")
    } else if granted.used_requests >= grant.max_requests
        || granted
            .used_input_tokens
            .checked_add(u64::from(request.required_context_tokens))
            .is_none_or(|total| total > grant.max_input_tokens)
    {
        (false, "model-gateway.route.grant-budget-exhausted")
    } else if request.prior_route_id.is_some() && !grant.fallback_allowed {
        (false, "model-gateway.route.grant-fallback-denied")
    } else {
        (true, "model-gateway.route.eligible")
    }
}

fn granted_route<'a>(
    request: &'a GatewayRoutingRequest,
    route: &QualifiedGatewayRoute,
) -> Option<&'a GrantedHybridRoute> {
    request
        .hybrid_routes
        .iter()
        .find(|granted| granted.grant.route_id == route.route_id)
}

const fn remote(value: CanonicalEndpointClass) -> bool {
    matches!(
        value,
        CanonicalEndpointClass::RemotePrivate | CanonicalEndpointClass::RemoteManaged
    )
}

/// The digest a grant carries: its canonical encoding with the digest zeroed.
pub fn hybrid_route_grant_digest(grant: &HybridRouteGrant) -> Result<String, GatewayRoutingError> {
    let mut candidate = grant.clone();
    candidate.grant_sha256 = ZERO_SHA256.to_owned();
    serde_json::to_vec(&candidate)
        .map(|bytes| hex(&Sha256::digest(bytes)))
        .map_err(|_| GatewayRoutingError::SerializationFailed)
}

fn validate_hybrid_routes(value: &GatewayRoutingRequest) -> Result<(), GatewayRoutingError> {
    if value.transmitted_data.is_empty() || value.hybrid_routes.len() > MAX_HYBRID_GRANTS {
        return Err(GatewayRoutingError::InvalidInput);
    }
    let mut routes = BTreeSet::new();
    for granted in &value.hybrid_routes {
        let grant = &granted.grant;
        if !valid_id(&grant.grant_id)
            || !valid_id(&grant.route_id)
            || !valid_id(&grant.provider_id)
            || !valid_sha256(&grant.candidate_sha256)
            || grant.data_classes.is_empty()
            || grant.max_requests == 0
            || grant.max_input_tokens == 0
            || !routes.insert(grant.route_id.as_str())
            || hybrid_route_grant_digest(grant)? != grant.grant_sha256
        {
            return Err(GatewayRoutingError::InvalidInput);
        }
    }
    Ok(())
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
        validate_hybrid_routes(value)
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
            mode: GatewayRoutingMode::Hybrid,
            transmitted_data: BTreeSet::from([RoutedDataClass::Conversation]),
            hybrid_routes: Vec::new(),
            now_epoch_ms: 1_000,
        }
    }
    fn grant(route: &QualifiedGatewayRoute, fallback_allowed: bool) -> GrantedHybridRoute {
        let mut grant = HybridRouteGrant {
            grant_id: format!("grant-{}", route.route_id),
            route_id: route.route_id.clone(),
            candidate_sha256: route.candidate_sha256.clone(),
            provider_id: "provider-example".to_owned(),
            data_classes: BTreeSet::from([
                RoutedDataClass::Conversation,
                RoutedDataClass::WorkspaceExcerpts,
            ]),
            max_requests: 3,
            max_input_tokens: 10_000,
            fallback_allowed,
            expires_at_epoch_ms: 2_000,
            grant_sha256: String::new(),
        };
        grant.grant_sha256 = hybrid_route_grant_digest(&grant).unwrap();
        GrantedHybridRoute {
            grant,
            used_requests: 0,
            used_input_tokens: 0,
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
        // A remote destination also needs its own grant that allows fallback
        // (Decision 0124).
        assert_eq!(
            route_gateway(&value, &[prior.clone(), destination.clone()]),
            Err(GatewayRoutingError::FallbackDenied)
        );
        value.hybrid_routes = vec![grant(&destination, false)];
        assert_eq!(
            route_gateway(&value, &[prior.clone(), destination.clone()]),
            Err(GatewayRoutingError::FallbackDenied)
        );
        value.hybrid_routes = vec![grant(&destination, true)];
        let receipt = route_gateway(&value, &[prior, destination]).expect("explicit fallback");
        assert!(receipt.fallback_used);
        assert_eq!(receipt.selected_route_id.as_deref(), Some("route-remote"));
        assert_eq!(
            receipt.hybrid_grant_sha256,
            Some(value.hybrid_routes[0].grant.grant_sha256.clone())
        );
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
        value.hybrid_routes = vec![grant(&destination, true)];
        assert_eq!(
            route_gateway(&value, &[prior, destination]),
            Err(GatewayRoutingError::FallbackDenied)
        );
    }

    #[test]
    fn local_only_mode_routes_locally_and_never_to_a_granted_remote_route() {
        // Decision 0124: local-only works without cloud inference, whatever
        // grants exist.
        let local = route("route-local", CanonicalEndpointClass::StrictLocal);
        let remote_route = route("route-remote", CanonicalEndpointClass::RemoteManaged);
        let mut value = request();
        value.maximum_endpoint_class = CanonicalEndpointClass::RemoteManaged;
        value.disclosure_accepted = true;
        value.mode = GatewayRoutingMode::LocalOnly;
        value.hybrid_routes = vec![grant(&remote_route, true)];
        let receipt =
            route_gateway(&value, &[remote_route.clone(), local.clone()]).expect("the local route");
        assert_eq!(receipt.selected_route_id.as_deref(), Some("route-local"));
        assert_eq!(receipt.mode, GatewayRoutingMode::LocalOnly);
        assert_eq!(receipt.hybrid_grant_sha256, None);
        assert_eq!(receipt.provider_id, None);
        let audit = receipt
            .considered_routes
            .iter()
            .find(|audit| audit.route_id == "route-remote")
            .unwrap();
        assert_eq!(audit.reason_code, "model-gateway.route.local-only");
        // With the local route down nothing is substituted.
        let mut down = local;
        down.healthy = false;
        assert_eq!(
            route_gateway(&value, &[remote_route, down]),
            Err(GatewayRoutingError::NoQualifiedRoute)
        );
    }

    #[test]
    fn a_remote_route_needs_its_own_current_grant_for_the_data_sent_within_budget() {
        // Decision 0124.
        let remote_route = route("route-remote", CanonicalEndpointClass::RemotePrivate);
        let mut value = request();
        value.maximum_endpoint_class = CanonicalEndpointClass::RemotePrivate;
        value.disclosure_accepted = true;
        let reason = |value: &GatewayRoutingRequest| {
            match route_gateway(value, std::slice::from_ref(&remote_route)) {
                Ok(receipt) => receipt.considered_routes[0].reason_code,
                Err(GatewayRoutingError::NoQualifiedRoute) => {
                    // Recompute the audit to name the refusal.
                    audit_route(value, &remote_route).unwrap().reason_code
                }
                Err(error) => panic!("unexpected {error:?}"),
            }
        };
        assert_eq!(reason(&value), "model-gateway.route.grant-missing");
        value.hybrid_routes = vec![grant(&remote_route, false)];
        let receipt = route_gateway(&value, std::slice::from_ref(&remote_route)).unwrap();
        assert_eq!(receipt.selected_route_id.as_deref(), Some("route-remote"));
        assert_eq!(receipt.provider_id.as_deref(), Some("provider-example"));
        assert_eq!(
            receipt.hybrid_grant_sha256,
            Some(value.hybrid_routes[0].grant.grant_sha256.clone())
        );
        assert_eq!(receipt.transmitted_data, [RoutedDataClass::Conversation]);

        type Change<'a> = &'a dyn Fn(&mut GatewayRoutingRequest);
        let cases: [(Change<'_>, &str); 6] = [
            (
                &|value| {
                    let mut other = remote_route.clone();
                    other.candidate_sha256 = hash('e');
                    value.hybrid_routes = vec![grant(&other, false)];
                },
                "model-gateway.route.grant-mismatch",
            ),
            (
                &|value| value.now_epoch_ms = 2_000,
                "model-gateway.route.grant-expired",
            ),
            (
                &|value| {
                    value.transmitted_data.insert(RoutedDataClass::ToolOutputs);
                },
                "model-gateway.route.grant-data-denied",
            ),
            (
                &|value| value.hybrid_routes[0].used_requests = 3,
                "model-gateway.route.grant-budget-exhausted",
            ),
            (
                &|value| value.hybrid_routes[0].used_input_tokens = 6_000,
                "model-gateway.route.grant-budget-exhausted",
            ),
            (
                &|value| value.hybrid_routes[0].used_input_tokens = u64::MAX,
                "model-gateway.route.grant-budget-exhausted",
            ),
        ];
        for (change, expected) in cases {
            let mut changed = value.clone();
            change(&mut changed);
            assert_eq!(reason(&changed), expected);
        }
        // The last token the budget admits is admitted.
        let mut exact = value.clone();
        exact.hybrid_routes[0].used_input_tokens = 5_904;
        assert_eq!(reason(&exact), "model-gateway.route.eligible");
    }

    #[test]
    fn a_tampered_duplicate_or_empty_grant_is_refused() {
        let remote_route = route("route-remote", CanonicalEndpointClass::RemotePrivate);
        let mut value = request();
        value.maximum_endpoint_class = CanonicalEndpointClass::RemotePrivate;
        value.disclosure_accepted = true;
        let mut tampered = grant(&remote_route, false);
        tampered.grant.max_requests = 1_000;
        let mut empty = grant(&remote_route, false);
        empty.grant.data_classes.clear();
        empty.grant.grant_sha256 = hybrid_route_grant_digest(&empty.grant).unwrap();
        for hybrid_routes in [
            vec![tampered],
            vec![grant(&remote_route, false), grant(&remote_route, true)],
            vec![empty],
        ] {
            let mut changed = value.clone();
            changed.hybrid_routes = hybrid_routes;
            assert_eq!(
                route_gateway(&changed, std::slice::from_ref(&remote_route)),
                Err(GatewayRoutingError::InvalidInput)
            );
        }
        value.transmitted_data.clear();
        assert_eq!(
            route_gateway(&value, std::slice::from_ref(&remote_route)),
            Err(GatewayRoutingError::InvalidInput)
        );
    }
}
