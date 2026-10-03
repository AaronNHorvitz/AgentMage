//! Model routes of the development coding host (Decisions 0128 and 0144).
//!
//! Before a run's model is composed, the development host's runtime factory
//! routes the run's model requests through the kernel router over the one
//! route of the selected source's exact profile. It routes in local-only
//! mode, or in hybrid mode when the run's workspace holds a live route grant
//! of the catalog host, carrying each grant with what its owner counted
//! against it (Decision 0144). The host declares the router's receipt
//! unchanged, in a type it can parse closed, and keeps a route history of one
//! model route entry per composition. The client keeps a receipt only when it
//! recomputes and describes either a selection of the run's own strict-local
//! route, or a remote route under a named grant and provider, which it shows
//! on a line of its own. The development host offers no remote route, so
//! nothing here can send a model request off the machine, and nothing here
//! grants authority.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CanonicalEndpointClass, ExactModelProfile, RuntimeRunRequest, to_canonical_json,
};
use agentmage_kernel_engine::action_history::{
    ActionAuthorization, ActionHistoryRecord, ActionKind, ActionOutcome, ActionRecordDraft,
};
use agentmage_kernel_engine::gateway_routing::{
    GatewayRouteReceipt, GatewayRoutingError, GatewayRoutingMode, GatewayRoutingRequest,
    GrantedHybridRoute, QualifiedGatewayRoute, RoutedDataClass, route_gateway,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_action_history::{
    RUN_ACTION_RETENTION_MS, RunActionChain, RunActionHistory, RunActionRecorder,
    verify_run_action_history,
};

/// Role every development route serves.
const DEVELOPMENT_ROUTE_ROLE: &str = "coding-development-proposer";
/// Platform policy identity of the development host.
const DEVELOPMENT_ROUTE_PLATFORM: &str = "linux-coding-development";
/// Identity of the development host's fixed routing policy.
const DEVELOPMENT_ROUTE_POLICY_ID: &str = "coding-development-local-only-v1";
/// Identity of the development host's hybrid routing policy (Decision 0144).
const DEVELOPMENT_HYBRID_POLICY_ID: &str = "coding-development-hybrid-v1";
/// Receipt schema the kernel router writes: the contract schema version.
const ROUTE_RECEIPT_SCHEMA_VERSION: u16 = agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The data classes every development run's model requests transmit: the
/// person's conversation, workspace excerpts and tool outputs.
pub const DEVELOPMENT_ROUTED_DATA: [RunRouteDataClass; 3] = [
    RunRouteDataClass::Conversation,
    RunRouteDataClass::WorkspaceExcerpts,
    RunRouteDataClass::ToolOutputs,
];

/// Routing mode, as the kernel router encodes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunRouteMode {
    /// No remote route is eligible.
    LocalOnly,
    /// A remote route is eligible only under its own grant.
    Hybrid,
}

/// Routed data class, as the kernel router encodes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunRouteDataClass {
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

impl RunRouteDataClass {
    pub(crate) const fn kernel(self) -> RoutedDataClass {
        match self {
            Self::Conversation => RoutedDataClass::Conversation,
            Self::WorkspaceExcerpts => RoutedDataClass::WorkspaceExcerpts,
            Self::ToolOutputs => RoutedDataClass::ToolOutputs,
            Self::RetrievedSources => RoutedDataClass::RetrievedSources,
            Self::Memory => RoutedDataClass::Memory,
        }
    }

    const fn of(value: RoutedDataClass) -> Self {
        match value {
            RoutedDataClass::Conversation => Self::Conversation,
            RoutedDataClass::WorkspaceExcerpts => Self::WorkspaceExcerpts,
            RoutedDataClass::ToolOutputs => Self::ToolOutputs,
            RoutedDataClass::RetrievedSources => Self::RetrievedSources,
            RoutedDataClass::Memory => Self::Memory,
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Conversation => "conversation",
            Self::WorkspaceExcerpts => "workspace_excerpts",
            Self::ToolOutputs => "tool_outputs",
            Self::RetrievedSources => "retrieved_sources",
            Self::Memory => "memory",
        }
    }
}

/// Audit of one considered route, as the kernel router wrote it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRouteAudit {
    /// Route identity.
    pub route_id: String,
    /// Exact candidate digest.
    pub candidate_sha256: String,
    /// Whether the route satisfied every deterministic gate.
    pub eligible: bool,
    /// Stable visible reason.
    pub reason_code: String,
}

/// The kernel router's receipt for one composition's model requests, with
/// the same fields and encoding, so its digest is the kernel's. Every member
/// is required and no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRouteReceipt {
    /// Receipt schema version.
    pub schema_version: u16,
    /// Request identity: the run identity.
    pub request_id: String,
    /// Routing policy digest.
    pub route_policy_sha256: String,
    /// Ordered audit of every considered route.
    pub considered_routes: Vec<RunRouteAudit>,
    /// Selected route, or none when blocked.
    #[serde(deserialize_with = "required_option")]
    pub selected_route_id: Option<String>,
    /// Whether the selected route is an explicit fallback.
    pub fallback_used: bool,
    /// Fallback policy digest, only when one was supplied.
    #[serde(deserialize_with = "required_option")]
    pub fallback_policy_sha256: Option<String>,
    /// Selected disclosure class, absent when blocked.
    #[serde(deserialize_with = "required_option")]
    pub disclosure_class: Option<CanonicalEndpointClass>,
    /// Mode the request was decided under.
    pub mode: RunRouteMode,
    /// Data classes the request transmits to the selected route.
    pub transmitted_data: Vec<RunRouteDataClass>,
    /// Grant digest when the selected route is remote.
    #[serde(deserialize_with = "required_option")]
    pub hybrid_grant_sha256: Option<String>,
    /// Provider named by that grant.
    #[serde(deserialize_with = "required_option")]
    pub provider_id: Option<String>,
    /// Stable terminal reason.
    pub reason_code: String,
    /// Digest of this receipt with this field zeroed.
    pub receipt_sha256: String,
}

/// An optional member that must still be present, as `null` when absent.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

impl RunRouteReceipt {
    /// The kernel receipt, field for field.
    #[must_use]
    pub fn of(receipt: &GatewayRouteReceipt) -> Self {
        Self {
            schema_version: receipt.schema_version,
            request_id: receipt.request_id.clone(),
            route_policy_sha256: receipt.route_policy_sha256.clone(),
            considered_routes: receipt
                .considered_routes
                .iter()
                .map(|audit| RunRouteAudit {
                    route_id: audit.route_id.clone(),
                    candidate_sha256: audit.candidate_sha256.clone(),
                    eligible: audit.eligible,
                    reason_code: audit.reason_code.to_owned(),
                })
                .collect(),
            selected_route_id: receipt.selected_route_id.clone(),
            fallback_used: receipt.fallback_used,
            fallback_policy_sha256: receipt.fallback_policy_sha256.clone(),
            disclosure_class: receipt.disclosure_class,
            mode: match receipt.mode {
                GatewayRoutingMode::LocalOnly => RunRouteMode::LocalOnly,
                GatewayRoutingMode::Hybrid => RunRouteMode::Hybrid,
            },
            transmitted_data: receipt
                .transmitted_data
                .iter()
                .copied()
                .map(RunRouteDataClass::of)
                .collect(),
            hybrid_grant_sha256: receipt.hybrid_grant_sha256.clone(),
            provider_id: receipt.provider_id.clone(),
            reason_code: receipt.reason_code.to_owned(),
            receipt_sha256: receipt.receipt_sha256.clone(),
        }
    }

    /// The digest the receipt must carry: its encoding with the digest
    /// zeroed, as the kernel router computes it.
    #[must_use]
    pub fn computed_sha256(&self) -> Option<String> {
        let mut unsigned = self.clone();
        ZERO_SHA256.clone_into(&mut unsigned.receipt_sha256);
        serde_json::to_vec(&unsigned)
            .ok()
            .map(|bytes| sha256_hex(&bytes))
    }
}

/// What the routing owner declares about one composition's model route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRouteDeclaration {
    /// The router's receipt.
    pub receipt: RunRouteReceipt,
    /// The route history, or `None` when its entry could not be kept.
    pub history: Option<RunActionHistory>,
}

/// Content-free failure to route a development run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunRouteError {
    /// The run's profile or request could not form a route.
    Invalid,
    /// The router selected no route.
    NoRoute,
    /// The router refused the request or a fallback.
    Refused,
}

impl RunRouteError {
    /// Stable code for standard error.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::NoRoute => "no-qualified-route",
            Self::Refused => "refused",
        }
    }
}

/// Content-free reason a declared receipt or route history was dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunRouteVerificationError {
    /// The receipt does not recompute, name this run or describe a
    /// local-only, strict-local selection of the declared data.
    Receipt,
    /// The route history does not replay, holds another kind, or does not
    /// bind this run and its kept receipt.
    History,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn json_sha256(value: &serde_json::Value) -> Result<String, RunRouteError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| RunRouteError::Invalid)
}

/// The development route of one exact profile. `purpose` names the purpose
/// the profile was admitted for: `contract-test` for the scripted fixture and
/// `evaluation` for a development candidate. Its digests identify the profile
/// and its admission; they are not a qualification.
fn development_route(
    profile: &ExactModelProfile,
    purpose: &str,
) -> Result<QualifiedGatewayRoute, RunRouteError> {
    let candidate_sha256 = to_canonical_json(profile)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| RunRouteError::Invalid)?;
    let capability_sha256 = capability_sha256(profile)?;
    let qualification_sha256 = json_sha256(&serde_json::json!({
        "record_type": "agentmage-development-route-admission",
        "candidate_sha256": candidate_sha256,
        "capabilities": profile
            .capabilities
            .iter()
            .map(|capability| serde_json::json!({
                "role": capability.role,
                "state": capability.state,
                "result_sha256": capability.result_sha256,
            }))
            .collect::<Vec<_>>(),
        "purpose": purpose,
    }))?;
    Ok(QualifiedGatewayRoute {
        route_id: profile.profile_id.as_str().to_owned(),
        candidate_sha256,
        endpoint_class: CanonicalEndpointClass::StrictLocal,
        role_id: DEVELOPMENT_ROUTE_ROLE.to_owned(),
        capability_sha256,
        platform_id: DEVELOPMENT_ROUTE_PLATFORM.to_owned(),
        qualification_sha256,
        invariant_controls_sha256: route_policy_sha256()?,
        max_context_tokens: profile.context.max_context_tokens,
        max_concurrency: 1,
        // The development host observes no health, resource fit, quota or
        // cost before a run; a model that fails to start fails the run.
        enabled: true,
        healthy: true,
        resources_available: true,
        quota_available: true,
        cost_admitted: true,
    })
}

fn capability_sha256(profile: &ExactModelProfile) -> Result<String, RunRouteError> {
    json_sha256(&serde_json::json!({
        "record_type": "agentmage-development-route-capabilities",
        "capabilities": profile.capabilities,
    }))
}

/// Digest of the development host's fixed routing policy: local-only, no
/// hybrid grant, no fallback, nothing wider than strict-local.
fn route_policy_sha256() -> Result<String, RunRouteError> {
    json_sha256(&serde_json::json!({
        "record_type": "agentmage-development-route-policy",
        "policy_id": DEVELOPMENT_ROUTE_POLICY_ID,
        "mode": "local_only",
        "maximum_endpoint_class": "strict_local",
        "hybrid_grants": "none",
        "fallback": "none",
    }))
}

/// Digest of the development host's hybrid routing policy (Decision 0144): a
/// remote route only under its own live grant from the catalog host's route
/// grant owner, nothing wider than remote managed, no fallback.
fn hybrid_route_policy_sha256() -> Result<String, RunRouteError> {
    json_sha256(&serde_json::json!({
        "record_type": "agentmage-development-route-policy",
        "policy_id": DEVELOPMENT_HYBRID_POLICY_ID,
        "mode": "hybrid",
        "maximum_endpoint_class": "remote_managed",
        "hybrid_grants": "catalog-host-route-grant-owner",
        "fallback": "none",
    }))
}

/// The routing request of one run in `mode`. In hybrid mode the grants are
/// the person's accepted disclosure (Decision 0144).
fn development_request(
    request: &RuntimeRunRequest,
    mode: GatewayRoutingMode,
    hybrid_routes: Vec<GrantedHybridRoute>,
    now_epoch_ms: u64,
) -> Result<GatewayRoutingRequest, RunRouteError> {
    let hybrid = mode == GatewayRoutingMode::Hybrid;
    let transmitted_data = DEVELOPMENT_ROUTED_DATA
        .iter()
        .map(|class| class.kernel())
        .collect::<BTreeSet<_>>();
    Ok(GatewayRoutingRequest {
        request_id: request.run_id.as_str().to_owned(),
        user_profile_id: if hybrid {
            DEVELOPMENT_HYBRID_POLICY_ID
        } else {
            DEVELOPMENT_ROUTE_POLICY_ID
        }
        .to_owned(),
        classification_sha256: json_sha256(&serde_json::json!({
            "record_type": "agentmage-development-route-classification",
            "transmitted_data": DEVELOPMENT_ROUTED_DATA,
        }))?,
        role_id: DEVELOPMENT_ROUTE_ROLE.to_owned(),
        capability_sha256: capability_sha256(&request.model_profile)?,
        platform_id: DEVELOPMENT_ROUTE_PLATFORM.to_owned(),
        maximum_endpoint_class: if hybrid {
            CanonicalEndpointClass::RemoteManaged
        } else {
            CanonicalEndpointClass::StrictLocal
        },
        disclosure_accepted: hybrid,
        required_context_tokens: request.context_budget.max_context_tokens,
        current_concurrency: 0,
        route_policy_sha256: if hybrid {
            hybrid_route_policy_sha256()?
        } else {
            route_policy_sha256()?
        },
        prior_route_id: None,
        fallback_policy: None,
        mode,
        transmitted_data,
        hybrid_routes,
        now_epoch_ms,
    })
}

fn route(
    request: &RuntimeRunRequest,
    routes: &[QualifiedGatewayRoute],
    mode: GatewayRoutingMode,
    hybrid_routes: Vec<GrantedHybridRoute>,
    now_epoch_ms: u64,
) -> Result<GatewayRouteReceipt, RunRouteError> {
    let routing = development_request(request, mode, hybrid_routes, now_epoch_ms)?;
    route_gateway(&routing, routes).map_err(|error| match error {
        GatewayRoutingError::NoQualifiedRoute => RunRouteError::NoRoute,
        GatewayRoutingError::FallbackDenied => RunRouteError::Refused,
        _ => RunRouteError::Invalid,
    })
}

/// Routes one run's model requests in local-only mode over the development
/// route of its exact profile, and keeps the route history entry. `purpose`
/// is the profile's admitted purpose.
pub fn route_development_run(
    request: &RuntimeRunRequest,
    purpose: &str,
    now_epoch_ms: u64,
) -> Result<RunRouteDeclaration, RunRouteError> {
    route_development_run_with_grants(request, purpose, now_epoch_ms, &[])
}

/// Routes one run's model requests over the development route of its exact
/// profile, in hybrid mode when `grants` holds a live grant of the run's
/// workspace and in local-only mode otherwise (Decision 0144). The
/// development host offers no remote route, so the run's own route is
/// selected either way.
pub fn route_development_run_with_grants(
    request: &RuntimeRunRequest,
    purpose: &str,
    now_epoch_ms: u64,
    grants: &[crate::coding_route_grants::GrantedRoute],
) -> Result<RunRouteDeclaration, RunRouteError> {
    route_development_offers(request, purpose, now_epoch_ms, grants, &[])
}

/// Routes over the run's profile route and `remote_routes`, which only host
/// tests offer.
fn route_development_offers(
    request: &RuntimeRunRequest,
    purpose: &str,
    now_epoch_ms: u64,
    grants: &[crate::coding_route_grants::GrantedRoute],
    remote_routes: &[QualifiedGatewayRoute],
) -> Result<RunRouteDeclaration, RunRouteError> {
    let mut routes = vec![development_route(&request.model_profile, purpose)?];
    routes.extend_from_slice(remote_routes);
    let mode = if grants.is_empty() {
        GatewayRoutingMode::LocalOnly
    } else {
        GatewayRoutingMode::Hybrid
    };
    let granted = grants
        .iter()
        .map(crate::coding_route_grants::GrantedRoute::kernel)
        .collect();
    let receipt = RunRouteReceipt::of(&route(request, &routes, mode, granted, now_epoch_ms)?);
    let mut history = RunActionRecorder::new();
    history.record(route_entry_draft(request, &receipt, now_epoch_ms));
    Ok(RunRouteDeclaration {
        receipt,
        history: history.declare(),
    })
}

/// Whether a receipt selected a route other than the run's own profile route,
/// which the development host has no adapter to send a request to.
#[must_use]
pub fn selects_remote_route(receipt: &RunRouteReceipt, request: &RuntimeRunRequest) -> bool {
    receipt.selected_route_id.as_deref() != Some(request.model_profile.profile_id.as_str())
}

/// Digest of the effect of one route selection: the run, the selected route
/// and candidate, the class, the mode and the data classes.
fn route_effect_sha256(receipt: &RunRouteReceipt) -> Option<String> {
    let selected = receipt.selected_route_id.as_deref()?;
    let candidate = receipt
        .considered_routes
        .iter()
        .find(|audit| audit.route_id == selected)?;
    json_sha256(&serde_json::json!({
        "record_type": "agentmage-model-route-effect",
        "run_id": receipt.request_id,
        "route_id": selected,
        "candidate_sha256": candidate.candidate_sha256,
        "disclosure_class": receipt.disclosure_class,
        "mode": receipt.mode,
        "transmitted_data": receipt.transmitted_data,
    }))
    .ok()
}

/// The route history entry of one composition. The person's decision is the
/// exact run request, which the person started with this profile.
#[must_use]
pub fn route_entry_draft(
    request: &RuntimeRunRequest,
    receipt: &RunRouteReceipt,
    now_epoch_ms: u64,
) -> Option<ActionRecordDraft> {
    // A remote selection also names its grant (Decision 0144).
    let mut evidence = vec![
        receipt.receipt_sha256.clone(),
        receipt.route_policy_sha256.clone(),
    ];
    evidence.extend(receipt.hybrid_grant_sha256.clone());
    evidence.sort();
    evidence.dedup();
    Some(ActionRecordDraft {
        action_kind: ActionKind::ModelRoute,
        action_id: receipt.request_id.clone(),
        authorization: ActionAuthorization::PersonDecision {
            decision_sha256: request.request_sha256.clone(),
        },
        effect_sha256: route_effect_sha256(receipt)?,
        outcome: ActionOutcome::Succeeded,
        reason_code: receipt.reason_code.clone(),
        evidence_sha256s: evidence,
        recorded_at_epoch_ms: now_epoch_ms,
        retain_until_epoch_ms: now_epoch_ms.checked_add(RUN_ACTION_RETENTION_MS)?,
    })
}

/// Keeps a declared receipt only when it recomputes, names this run,
/// transmits exactly the declared data without any fallback, and describes
/// one of three selections (Decisions 0130 and 0144):
/// - in local-only mode under the development host's fixed policy, the run's
///   own profile route, by identity and canonical digest, of class
///   strict-local, with no grant or provider;
/// - in hybrid mode under the hybrid policy, that same local selection;
/// - in hybrid mode under the hybrid policy, one eligible remote route,
///   named with the grant and the provider it was selected under.
pub fn verify_run_route_receipt(
    receipt: &RunRouteReceipt,
    request: &RuntimeRunRequest,
) -> Result<(), RunRouteVerificationError> {
    let profile_id = request.model_profile.profile_id.as_str();
    let profile_sha256 = to_canonical_json(&request.model_profile)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| RunRouteVerificationError::Receipt)?;
    let policy_sha256 = match receipt.mode {
        RunRouteMode::LocalOnly => route_policy_sha256(),
        RunRouteMode::Hybrid => hybrid_route_policy_sha256(),
    }
    .map_err(|_| RunRouteVerificationError::Receipt)?;
    let selected = receipt.selected_route_id.as_deref();
    let selected_audits = receipt
        .considered_routes
        .iter()
        .filter(|audit| Some(audit.route_id.as_str()) == selected)
        .collect::<Vec<_>>();
    let local = selected == Some(profile_id)
        && receipt.disclosure_class == Some(CanonicalEndpointClass::StrictLocal)
        && matches!(
            selected_audits.as_slice(),
            [audit] if audit.eligible && audit.candidate_sha256 == profile_sha256
        )
        && receipt.hybrid_grant_sha256.is_none()
        && receipt.provider_id.is_none();
    let remote = receipt.mode == RunRouteMode::Hybrid
        && selected.is_some_and(|route| route != profile_id)
        && matches!(
            receipt.disclosure_class,
            Some(CanonicalEndpointClass::RemotePrivate | CanonicalEndpointClass::RemoteManaged)
        )
        && matches!(selected_audits.as_slice(), [audit] if audit.eligible)
        && receipt
            .hybrid_grant_sha256
            .as_deref()
            .is_some_and(lower_hex_sha256)
        && receipt
            .provider_id
            .as_deref()
            .is_some_and(crate::coding_route_grants::route_grant_identifier);
    let kept = receipt.schema_version == ROUTE_RECEIPT_SCHEMA_VERSION
        && receipt.request_id == request.run_id.as_str()
        && receipt.computed_sha256().as_deref() == Some(receipt.receipt_sha256.as_str())
        && receipt.route_policy_sha256 == policy_sha256
        && (local || remote)
        && !receipt.fallback_used
        && receipt.fallback_policy_sha256.is_none()
        && receipt.transmitted_data == DEVELOPMENT_ROUTED_DATA;
    if kept {
        Ok(())
    } else {
        Err(RunRouteVerificationError::Receipt)
    }
}

fn lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Keeps a declared route history only when it replays to its head, holds
/// model route entries only, and each entry is authorized by this run's
/// request and names the kept receipt as evidence.
pub fn verify_run_route_history(
    history: &RunActionHistory,
    request: &RuntimeRunRequest,
    receipt: Option<&RunRouteReceipt>,
) -> Result<(), RunRouteVerificationError> {
    let receipt = receipt.ok_or(RunRouteVerificationError::History)?;
    verify_run_action_history(history, RunActionChain::Routes)
        .map_err(|_| RunRouteVerificationError::History)?;
    let bound = !history.records.is_empty()
        && history.records.iter().all(|record| match record {
            ActionHistoryRecord::Kept(entry) => {
                entry.authorization
                    == ActionAuthorization::PersonDecision {
                        decision_sha256: request.request_sha256.clone(),
                    }
                    && entry.evidence_sha256s.contains(&receipt.receipt_sha256)
            }
            ActionHistoryRecord::Expired { .. } => false,
        });
    if bound {
        Ok(())
    } else {
        Err(RunRouteVerificationError::History)
    }
}

/// One line for a kept receipt, or an unavailable line. A remote selection
/// says so first, with its provider, grant and every data class it received,
/// so a remote route is never used silently (Decision 0144).
#[must_use]
pub fn render_run_route_receipt(receipt: Option<&RunRouteReceipt>) -> String {
    let Some(receipt) = receipt else {
        return "model route: unavailable; the host could not declare a verified route\n"
            .to_owned();
    };
    let data = receipt
        .transmitted_data
        .iter()
        .map(|class| class.name())
        .collect::<Vec<_>>()
        .join(", ");
    let short = |digest: &str| digest.get(..12).unwrap_or(digest).to_owned();
    let mode = match receipt.mode {
        RunRouteMode::LocalOnly => "local-only",
        RunRouteMode::Hybrid => "hybrid",
    };
    let selection = match (&receipt.hybrid_grant_sha256, &receipt.provider_id) {
        (Some(grant), Some(provider)) => format!(
            "REMOTE route {} of provider {provider} under grant {}; data {data} sent off this machine",
            receipt.selected_route_id.as_deref().unwrap_or("none"),
            short(grant)
        ),
        _ => format!(
            "selected {} (strict_local); data {data}",
            receipt.selected_route_id.as_deref().unwrap_or("none")
        ),
    };
    format!(
        "model route: {mode}; {selection}; {} considered; {}; receipt {}\n",
        receipt.considered_routes.len(),
        receipt.reason_code,
        short(&receipt.receipt_sha256)
    )
}

#[cfg(all(test, feature = "source-artifacts", feature = "workflow-supervisor"))]
mod tests {
    use super::*;
    use agentmage_kernel_engine::gateway_routing::{HybridRouteGrant, hybrid_route_grant_digest};

    fn request() -> RuntimeRunRequest {
        crate::runtime_read_tests::completed_native_read_fixture().0
    }

    fn remote_route(request: &RuntimeRunRequest) -> QualifiedGatewayRoute {
        let mut remote = development_route(&request.model_profile, "evaluation").unwrap();
        remote.route_id = "remote-managed-route".to_owned();
        remote.candidate_sha256 = "d".repeat(64);
        remote.endpoint_class = CanonicalEndpointClass::RemoteManaged;
        remote
    }

    fn granted(route: &QualifiedGatewayRoute) -> GrantedHybridRoute {
        let mut grant = HybridRouteGrant {
            grant_id: "grant-remote".to_owned(),
            route_id: route.route_id.clone(),
            candidate_sha256: route.candidate_sha256.clone(),
            provider_id: "provider-remote".to_owned(),
            data_classes: DEVELOPMENT_ROUTED_DATA
                .iter()
                .map(|class| class.kernel())
                .collect(),
            max_requests: 10,
            max_input_tokens: 10_000_000,
            fallback_allowed: false,
            expires_at_epoch_ms: u64::MAX,
            grant_sha256: ZERO_SHA256.to_owned(),
        };
        grant.grant_sha256 = hybrid_route_grant_digest(&grant).unwrap();
        GrantedHybridRoute {
            grant,
            used_requests: 0,
            used_input_tokens: 0,
        }
    }

    #[test]
    fn a_run_is_routed_locally_and_its_receipt_keeps_the_kernel_digest() {
        let request = request();
        let route = development_route(&request.model_profile, "contract-test").unwrap();
        let kernel = super::route(
            &request,
            &[route],
            GatewayRoutingMode::LocalOnly,
            Vec::new(),
            5_000,
        )
        .unwrap();
        let receipt = RunRouteReceipt::of(&kernel);
        // The host type encodes exactly as the kernel receipt, so the kernel
        // digest recomputes over it.
        assert_eq!(
            serde_json::to_value(&receipt).unwrap(),
            serde_json::to_value(&kernel).unwrap()
        );
        assert_eq!(
            receipt.computed_sha256().as_deref(),
            Some(kernel.receipt_sha256.as_str())
        );
        assert_eq!(receipt.mode, RunRouteMode::LocalOnly);
        assert_eq!(
            receipt.selected_route_id.as_deref(),
            Some(request.model_profile.profile_id.as_str())
        );
        assert_eq!(
            receipt.disclosure_class,
            Some(CanonicalEndpointClass::StrictLocal)
        );
        assert_eq!(receipt.transmitted_data, DEVELOPMENT_ROUTED_DATA);
        assert_eq!(
            receipt.reason_code,
            "model-gateway.route.qualified-selected"
        );
        assert_eq!(verify_run_route_receipt(&receipt, &request), Ok(()));

        let declared = route_development_run(&request, "contract-test", 5_000).unwrap();
        assert_eq!(declared.receipt, receipt);
        let history = declared.history.expect("route history");
        assert_eq!(
            verify_run_route_history(&history, &request, Some(&receipt)),
            Ok(())
        );
        let [ActionHistoryRecord::Kept(entry)] = history.records.as_slice() else {
            panic!("one kept entry");
        };
        assert_eq!(entry.action_kind, ActionKind::ModelRoute);
        assert_eq!(entry.action_id, request.run_id.as_str());
        assert_eq!(entry.outcome, ActionOutcome::Succeeded);
        assert_eq!(entry.reason_code, receipt.reason_code);
        assert_eq!(entry.recorded_at_epoch_ms, 5_000);
        assert!(entry.evidence_sha256s.contains(&receipt.receipt_sha256));
    }

    #[test]
    fn local_only_mode_keeps_every_remote_route_unavailable_even_with_a_grant() {
        // Decision 0128: the development host routes in local-only mode, so a
        // remote route is refused whatever grant is offered, and a remote
        // route alone leaves nothing selected.
        let request = request();
        let local = development_route(&request.model_profile, "evaluation").unwrap();
        let remote = remote_route(&request);
        let grant = granted(&remote);
        let receipt = RunRouteReceipt::of(
            &super::route(
                &request,
                &[remote.clone(), local.clone()],
                GatewayRoutingMode::LocalOnly,
                vec![grant.clone()],
                5_000,
            )
            .unwrap(),
        );
        assert_eq!(
            receipt.selected_route_id.as_deref(),
            Some(local.route_id.as_str())
        );
        let refused = receipt
            .considered_routes
            .iter()
            .find(|audit| audit.route_id == remote.route_id)
            .unwrap();
        assert!(!refused.eligible);
        assert_eq!(refused.reason_code, "model-gateway.route.disclosure-denied");
        assert_eq!(receipt.hybrid_grant_sha256, None);
        assert_eq!(receipt.provider_id, None);
        assert_eq!(verify_run_route_receipt(&receipt, &request), Ok(()));
        // The request itself allows nothing wider than strict-local, accepts
        // no disclosure, and carries no fallback.
        let routing =
            development_request(&request, GatewayRoutingMode::LocalOnly, Vec::new(), 5_000)
                .unwrap();
        assert_eq!(routing.mode, GatewayRoutingMode::LocalOnly);
        assert_eq!(
            routing.maximum_endpoint_class,
            CanonicalEndpointClass::StrictLocal
        );
        assert!(!routing.disclosure_accepted);
        assert!(routing.fallback_policy.is_none() && routing.prior_route_id.is_none());
        assert!(routing.hybrid_routes.is_empty());
        // The same remote route, admitted by class and disclosure, is still
        // refused by the local-only mode itself.
        let mut routing = development_request(
            &request,
            GatewayRoutingMode::LocalOnly,
            vec![grant.clone()],
            5_000,
        )
        .unwrap();
        routing.maximum_endpoint_class = CanonicalEndpointClass::RemoteManaged;
        routing.disclosure_accepted = true;
        let receipt =
            RunRouteReceipt::of(&route_gateway(&routing, &[remote.clone(), local]).unwrap());
        let refused = receipt
            .considered_routes
            .iter()
            .find(|audit| audit.route_id == remote.route_id)
            .unwrap();
        assert_eq!(
            (refused.eligible, refused.reason_code.as_str()),
            (false, "model-gateway.route.local-only")
        );
        // Only a remote route: nothing is selected, so composition refuses.
        assert_eq!(
            super::route(
                &request,
                &[remote],
                GatewayRoutingMode::LocalOnly,
                vec![grant],
                5_000
            ),
            Err(RunRouteError::NoRoute)
        );
    }

    #[test]
    fn a_receipt_or_route_history_that_does_not_bind_this_local_run_is_dropped() {
        let request = request();
        let declared = route_development_run(&request, "contract-test", 5_000).unwrap();
        let receipt = declared.receipt.clone();
        let history = declared.history.clone().unwrap();
        let resealed = |mut value: RunRouteReceipt| {
            value.receipt_sha256 = value.computed_sha256().unwrap();
            value
        };
        let mut other_run = request.clone();
        other_run.run_id = agentmage_kernel_contracts::RuntimeRunId::from_raw("run-other");
        assert_eq!(
            verify_run_route_receipt(&receipt, &other_run),
            Err(RunRouteVerificationError::Receipt)
        );
        let mut tampered = receipt.clone();
        tampered.reason_code = "model-gateway.route.explicit-fallback-selected".to_owned();
        let changes: Vec<RunRouteReceipt> = vec![
            // A changed field without a recomputed digest.
            tampered,
            resealed(RunRouteReceipt {
                mode: RunRouteMode::Hybrid,
                ..receipt.clone()
            }),
            resealed(RunRouteReceipt {
                disclosure_class: Some(CanonicalEndpointClass::RemoteManaged),
                ..receipt.clone()
            }),
            resealed(RunRouteReceipt {
                selected_route_id: None,
                ..receipt.clone()
            }),
            resealed(RunRouteReceipt {
                hybrid_grant_sha256: Some("e".repeat(64)),
                ..receipt.clone()
            }),
            resealed(RunRouteReceipt {
                provider_id: Some("provider-remote".to_owned()),
                ..receipt.clone()
            }),
            resealed(RunRouteReceipt {
                fallback_used: true,
                ..receipt.clone()
            }),
            resealed(RunRouteReceipt {
                transmitted_data: vec![RunRouteDataClass::Conversation],
                ..receipt.clone()
            }),
            resealed(RunRouteReceipt {
                schema_version: ROUTE_RECEIPT_SCHEMA_VERSION + 1,
                ..receipt.clone()
            }),
            // Another local route than the person's profile, consistently
            // renamed in its audit.
            resealed(RunRouteReceipt {
                selected_route_id: Some("another-local-profile".to_owned()),
                considered_routes: receipt
                    .considered_routes
                    .iter()
                    .map(|audit| RunRouteAudit {
                        route_id: "another-local-profile".to_owned(),
                        ..audit.clone()
                    })
                    .collect(),
                ..receipt.clone()
            }),
            // The profile's identity with another candidate digest.
            resealed(RunRouteReceipt {
                considered_routes: receipt
                    .considered_routes
                    .iter()
                    .map(|audit| RunRouteAudit {
                        candidate_sha256: "c".repeat(64),
                        ..audit.clone()
                    })
                    .collect(),
                ..receipt.clone()
            }),
            // Another routing policy.
            resealed(RunRouteReceipt {
                route_policy_sha256: "b".repeat(64),
                ..receipt.clone()
            }),
        ];
        for changed in changes {
            assert_eq!(
                verify_run_route_receipt(&changed, &request),
                Err(RunRouteVerificationError::Receipt),
                "{changed:?}"
            );
        }
        // A missing or extra member is refused when the receipt is parsed.
        let mut value = serde_json::to_value(&receipt).unwrap();
        value.as_object_mut().unwrap().remove("provider_id");
        assert!(serde_json::from_value::<RunRouteReceipt>(value).is_err());
        let mut value = serde_json::to_value(&receipt).unwrap();
        value["extra"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RunRouteReceipt>(value).is_err());
        let mut value = serde_json::to_value(&receipt).unwrap();
        value["considered_routes"][0]["extra"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RunRouteReceipt>(value).is_err());

        // A history needs the kept receipt, this run's request and a model
        // route entry naming that receipt.
        assert_eq!(
            verify_run_route_history(&history, &request, None),
            Err(RunRouteVerificationError::History)
        );
        let mut other_request = request.clone();
        other_request.request_sha256 = "f".repeat(64);
        assert_eq!(
            verify_run_route_history(&history, &other_request, Some(&receipt)),
            Err(RunRouteVerificationError::History)
        );
        let other_receipt = RunRouteReceipt {
            receipt_sha256: "a".repeat(64),
            ..receipt.clone()
        };
        assert_eq!(
            verify_run_route_history(&history, &request, Some(&other_receipt)),
            Err(RunRouteVerificationError::History)
        );
        let mut foreign = RunActionRecorder::new();
        let mut draft = route_entry_draft(&request, &receipt, 5_000).unwrap();
        draft.action_kind = ActionKind::ToolCall;
        foreign.record(Some(draft));
        assert_eq!(
            verify_run_route_history(&foreign.declare().unwrap(), &request, Some(&receipt)),
            Err(RunRouteVerificationError::History)
        );
        assert_eq!(
            verify_run_route_history(
                &RunActionRecorder::new().declare().unwrap(),
                &request,
                Some(&receipt)
            ),
            Err(RunRouteVerificationError::History)
        );
    }

    #[test]
    fn the_route_is_rendered_or_said_to_be_unavailable() {
        let request = request();
        let declared = route_development_run(&request, "contract-test", 5_000).unwrap();
        let line = render_run_route_receipt(Some(&declared.receipt));
        assert!(line.starts_with("model route: local-only; selected "));
        assert!(line.contains(
            "(strict_local); data conversation, workspace_excerpts, tool_outputs; 1 considered; model-gateway.route.qualified-selected; receipt "
        ));
        assert!(line.ends_with('\n') && line.lines().count() == 1);
        assert_eq!(
            render_run_route_receipt(None),
            "model route: unavailable; the host could not declare a verified route\n"
        );
    }
    /// A live grant of the remote fixture route, from the catalog host's own
    /// request type (Decision 0144).
    fn route_grant(
        remote: &QualifiedGatewayRoute,
        data_classes: &[RunRouteDataClass],
        fallback_allowed: bool,
    ) -> crate::coding_route_grants::GrantedRoute {
        let file = crate::coding_route_grants::RouteGrantFile {
            schema_version: 1,
            grant_id: "grant-remote".to_owned(),
            route_id: remote.route_id.clone(),
            candidate_sha256: remote.candidate_sha256.clone(),
            provider_id: "provider-remote".to_owned(),
            data_classes: data_classes.to_vec(),
            max_requests: 2,
            max_input_tokens: 10_000_000,
            fallback_allowed,
            valid_for_hours: 1,
        };
        crate::coding_route_grants::GrantedRoute {
            grant: crate::coding_route_grants::RouteGrant::from_file(&file, 5_000).unwrap(),
            used_requests: 0,
            used_input_tokens: 0,
        }
    }

    /// The run with a context budget its local route cannot serve, and a
    /// remote fixture route that can.
    fn beyond_local(request: &RuntimeRunRequest) -> (RuntimeRunRequest, QualifiedGatewayRoute) {
        let mut wide = request.clone();
        wide.context_budget.max_context_tokens =
            request.model_profile.context.max_context_tokens + 1;
        let mut remote = remote_route(request);
        remote.max_context_tokens = wide.context_budget.max_context_tokens;
        (wide, remote)
    }

    #[test]
    fn a_workspace_with_live_grants_routes_in_hybrid_mode_and_still_selects_its_local_route() {
        // Decision 0144: a live grant puts the run in hybrid mode under the
        // hybrid policy; the host offers no remote route, so the run's own
        // route is selected and the receipt names no grant or provider.
        let request = request();
        let remote = remote_route(&request);
        let grant = route_grant(&remote, &DEVELOPMENT_ROUTED_DATA, false);
        let declared = route_development_run_with_grants(
            &request,
            "contract-test",
            5_000,
            std::slice::from_ref(&grant),
        )
        .unwrap();
        let receipt = &declared.receipt;
        assert_eq!(receipt.mode, RunRouteMode::Hybrid);
        assert_eq!(
            receipt.route_policy_sha256,
            hybrid_route_policy_sha256().unwrap()
        );
        assert_eq!(
            receipt.selected_route_id.as_deref(),
            Some(request.model_profile.profile_id.as_str())
        );
        assert_eq!(
            (&receipt.hybrid_grant_sha256, &receipt.provider_id),
            (&None, &None)
        );
        assert!(!selects_remote_route(receipt, &request));
        assert_eq!(verify_run_route_receipt(receipt, &request), Ok(()));
        let history = declared.history.as_ref().unwrap();
        assert_eq!(
            verify_run_route_history(history, &request, Some(receipt)),
            Ok(())
        );
        assert!(
            render_run_route_receipt(Some(receipt)).starts_with("model route: hybrid; selected ")
        );
        // Without a grant the same run is routed local-only, as before.
        let local = route_development_run_with_grants(&request, "contract-test", 5_000, &[])
            .unwrap()
            .receipt;
        assert_eq!(
            local,
            route_development_run(&request, "contract-test", 5_000)
                .unwrap()
                .receipt
        );
        assert_eq!(local.mode, RunRouteMode::LocalOnly);
    }

    #[test]
    fn a_granted_remote_route_is_selected_only_within_its_grant_and_is_never_silent() {
        // Decision 0144: host tests offer a remote fixture route; it is
        // selected only when the local route cannot serve the run, and only
        // under a live grant that covers the data and has budget left.
        let request = request();
        let (wide, remote) = beyond_local(&request);
        let grant = route_grant(&remote, &DEVELOPMENT_ROUTED_DATA, false);
        let offer = |grants: &[crate::coding_route_grants::GrantedRoute], now| {
            route_development_offers(
                &wide,
                "contract-test",
                now,
                grants,
                std::slice::from_ref(&remote),
            )
        };
        let declared = offer(std::slice::from_ref(&grant), 5_000).unwrap();
        let receipt = &declared.receipt;
        assert_eq!(
            receipt.selected_route_id.as_deref(),
            Some(remote.route_id.as_str())
        );
        assert_eq!(
            receipt.hybrid_grant_sha256.as_deref(),
            Some(grant.grant.grant_sha256.as_str())
        );
        assert_eq!(receipt.provider_id.as_deref(), Some("provider-remote"));
        assert_eq!(
            receipt.disclosure_class,
            Some(CanonicalEndpointClass::RemoteManaged)
        );
        assert!(selects_remote_route(receipt, &wide));
        assert_eq!(verify_run_route_receipt(receipt, &wide), Ok(()));
        let history = declared.history.as_ref().unwrap();
        assert_eq!(
            verify_run_route_history(history, &wide, Some(receipt)),
            Ok(())
        );
        let ActionHistoryRecord::Kept(entry) = &history.records[0] else {
            panic!("the route entry is kept");
        };
        assert!(entry.evidence_sha256s.contains(&grant.grant.grant_sha256));
        let line = render_run_route_receipt(Some(receipt));
        assert!(line.starts_with(&format!(
            "model route: hybrid; REMOTE route {} of provider provider-remote under grant {}; data conversation, workspace_excerpts, tool_outputs sent off this machine; ",
            remote.route_id,
            &grant.grant.grant_sha256[..12]
        )));

        // Expiry, exhausted counters, data the grant does not cover, and a
        // revoked grant (which the owner no longer offers) leave nothing.
        let exhausted = crate::coding_route_grants::GrantedRoute {
            used_requests: 2,
            ..grant.clone()
        };
        let tokens_spent = crate::coding_route_grants::GrantedRoute {
            used_input_tokens: 10_000_000,
            ..grant.clone()
        };
        let narrow = route_grant(&remote, &[RunRouteDataClass::Conversation], false);
        for (grants, now) in [
            (vec![grant.clone()], grant.grant.expires_at_epoch_ms),
            (vec![exhausted], 5_000),
            (vec![tokens_spent], 5_000),
            (vec![narrow], 5_000),
            (Vec::new(), 5_000),
        ] {
            assert_eq!(offer(&grants, now), Err(RunRouteError::NoRoute));
        }
        // In local-only mode the granted route itself is refused, so a run
        // its local route cannot serve gets no route at all.
        assert_eq!(
            super::route(
                &wide,
                &[
                    development_route(&wide.model_profile, "contract-test").unwrap(),
                    remote.clone(),
                ],
                GatewayRoutingMode::LocalOnly,
                vec![grant.kernel()],
                5_000,
            ),
            Err(RunRouteError::NoRoute)
        );
    }

    #[test]
    fn a_remote_fallback_is_refused_unless_its_grant_allows_one_and_the_client_keeps_none() {
        // Decision 0144: the development host never falls back, but the
        // router it uses refuses a fallback to a remote route whose grant
        // does not allow one, and the client drops any fallback receipt.
        let request = request();
        let local = development_route(&request.model_profile, "contract-test").unwrap();
        let remote = remote_route(&request);
        for fallback_allowed in [false, true] {
            let grant = route_grant(&remote, &DEVELOPMENT_ROUTED_DATA, fallback_allowed);
            let mut routing = development_request(
                &request,
                GatewayRoutingMode::Hybrid,
                vec![grant.kernel()],
                5_000,
            )
            .unwrap();
            routing.prior_route_id = Some(local.route_id.clone());
            routing.fallback_policy = Some(
                agentmage_kernel_engine::gateway_routing::ExplicitFallbackPolicy {
                    policy_id: "fallback-fixture".to_owned(),
                    policy_sha256: "f".repeat(64),
                    ordered_route_ids: vec![local.route_id.clone(), remote.route_id.clone()],
                    authorized_destination_route_id: remote.route_id.clone(),
                    destination_disclosure_accepted: true,
                    equivalent_controls_required: true,
                },
            );
            let routed = route_gateway(&routing, &[local.clone(), remote.clone()]);
            if fallback_allowed {
                let receipt = RunRouteReceipt::of(&routed.unwrap());
                assert!(receipt.fallback_used);
                assert_eq!(
                    verify_run_route_receipt(&receipt, &request),
                    Err(RunRouteVerificationError::Receipt)
                );
            } else {
                assert_eq!(routed, Err(GatewayRoutingError::FallbackDenied));
            }
        }
    }

    #[test]
    fn a_hybrid_receipt_is_kept_only_for_a_local_or_named_remote_selection() {
        let request = request();
        let (wide, remote) = beyond_local(&request);
        let grant = route_grant(&remote, &DEVELOPMENT_ROUTED_DATA, false);
        let remote_receipt = route_development_offers(
            &wide,
            "contract-test",
            5_000,
            std::slice::from_ref(&grant),
            std::slice::from_ref(&remote),
        )
        .unwrap()
        .receipt;
        let local_receipt = route_development_run_with_grants(
            &request,
            "contract-test",
            5_000,
            std::slice::from_ref(&grant),
        )
        .unwrap()
        .receipt;
        let resealed = |mut value: RunRouteReceipt| {
            value.receipt_sha256 = value.computed_sha256().unwrap();
            value
        };
        for changed in [
            // A remote selection without its grant, its provider or a remote
            // class, under the local-only policy, or in local-only mode.
            resealed(RunRouteReceipt {
                hybrid_grant_sha256: None,
                ..remote_receipt.clone()
            }),
            resealed(RunRouteReceipt {
                provider_id: None,
                ..remote_receipt.clone()
            }),
            resealed(RunRouteReceipt {
                hybrid_grant_sha256: Some("G".repeat(64)),
                ..remote_receipt.clone()
            }),
            resealed(RunRouteReceipt {
                disclosure_class: Some(CanonicalEndpointClass::StrictLocal),
                ..remote_receipt.clone()
            }),
            resealed(RunRouteReceipt {
                route_policy_sha256: route_policy_sha256().unwrap(),
                ..remote_receipt.clone()
            }),
            resealed(RunRouteReceipt {
                mode: RunRouteMode::LocalOnly,
                route_policy_sha256: route_policy_sha256().unwrap(),
                ..remote_receipt.clone()
            }),
            // The local selection naming a grant, or under the other policy.
            resealed(RunRouteReceipt {
                hybrid_grant_sha256: Some("e".repeat(64)),
                provider_id: Some("provider-remote".to_owned()),
                ..local_receipt.clone()
            }),
            resealed(RunRouteReceipt {
                route_policy_sha256: route_policy_sha256().unwrap(),
                ..local_receipt.clone()
            }),
        ] {
            let request = if changed.selected_route_id == remote_receipt.selected_route_id {
                &wide
            } else {
                &request
            };
            assert_eq!(
                verify_run_route_receipt(&changed, request),
                Err(RunRouteVerificationError::Receipt),
                "{changed:?}"
            );
        }
        assert_eq!(verify_run_route_receipt(&remote_receipt, &wide), Ok(()));
        assert_eq!(verify_run_route_receipt(&local_receipt, &request), Ok(()));
    }
}
