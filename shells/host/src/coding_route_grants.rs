//! Hybrid route grants through the development catalog host (Decision 0144).
//!
//! The catalog host keeps one route grant catalog for its state root, as an
//! owner state in its operational store. A person grants a remote route to
//! the runs of one workspace for a bounded number of requests and input
//! tokens until an expiry, lists the grants of a workspace, and revokes one.
//! Each operation decodes and re-verifies the stored catalog, applies one
//! transition and commits the next state under the revision it read. A grant
//! is never deleted: a revoked or expired grant stays listed with what was
//! counted against it.
//!
//! The development host sources the live grants of a run's workspace and
//! their counters into the run's routing request (Decision 0144). It offers
//! no remote route, so no request leaves the machine; counting a request
//! against a grant is offered only for a later remote adapter. A grant states
//! what a person allowed; it performs no effect, and every refusal is closed,
//! content-free and changes nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_engine::gateway_routing::{
    GrantedHybridRoute, HybridRouteGrant, hybrid_route_grant_digest,
};
use agentmage_kernel_engine::owner_state_store::{
    DurableOwnerStates, OwnerStateName, OwnerStateStoreError,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_route::RunRouteDataClass;

/// Most workspaces the catalog holds.
pub const MAX_ROUTE_GRANT_SCOPES: usize = 64;
/// Most grants one workspace keeps, revoked and expired ones included.
pub const MAX_GRANTS_PER_SCOPE: usize = 64;
/// Most live grants one workspace holds: the router's own bound.
pub const MAX_LIVE_GRANTS_PER_SCOPE: usize = 16;
/// Most requests one grant admits.
pub const MAX_GRANT_REQUESTS: u32 = 100_000;
/// Most input tokens one grant admits.
pub const MAX_GRANT_INPUT_TOKENS: u64 = 1_000_000_000;
/// Longest validity a grant request may ask for, in hours.
pub const MAX_GRANT_VALIDITY_HOURS: u32 = 720;
/// Largest grant request file the CLI reads.
pub const MAX_ROUTE_GRANT_FILE_BYTES: u64 = 4 * 1024;
const HOUR_MS: u64 = 3_600_000;
/// Latest expiry the owner accepts after its clock: 30 days and one hour, so
/// a grant the CLI dated by its own clock is not refused for a small skew.
const MAX_EXPIRY_AFTER_MS: u64 = 721 * HOUR_MS;
const STATE_SCHEMA_VERSION: u16 = 1;
const MAX_STATE_BYTES: usize = 4 * 1024 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;

/// An optional member that must still be present, as `null` when absent.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// The closed grant request a person writes and the CLI reads. Every member
/// is required and no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteGrantFile {
    /// Request schema version, 1.
    pub schema_version: u16,
    /// The grant's identity.
    pub grant_id: String,
    /// The exact remote route.
    pub route_id: String,
    /// The exact candidate digest of that route.
    pub candidate_sha256: String,
    /// The provider named to the person.
    pub provider_id: String,
    /// Every data class the route may receive.
    pub data_classes: Vec<RunRouteDataClass>,
    /// Most requests the grant admits.
    pub max_requests: u32,
    /// Most input tokens the grant admits.
    pub max_input_tokens: u64,
    /// Whether the route may replace a failed route.
    pub fallback_allowed: bool,
    /// Hours from now until the grant expires, 1 to 720.
    pub valid_for_hours: u32,
}

/// One hybrid route grant: the kernel grant's members, with its digest.
/// Every member is required and no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteGrant {
    /// Grant identity.
    pub grant_id: String,
    /// The exact granted route.
    pub route_id: String,
    /// The exact granted candidate digest.
    pub candidate_sha256: String,
    /// The provider named to the person.
    pub provider_id: String,
    /// Every data class the route may receive, in order and once each.
    pub data_classes: Vec<RunRouteDataClass>,
    /// Most requests the grant admits.
    pub max_requests: u32,
    /// Most input tokens the grant admits.
    pub max_input_tokens: u64,
    /// Whether the route may replace a failed route.
    pub fallback_allowed: bool,
    /// Exclusive expiry, in Unix epoch milliseconds.
    pub expires_at_epoch_ms: u64,
    /// The kernel's digest of this grant.
    pub grant_sha256: String,
}

impl RouteGrant {
    /// The kernel grant, member for member.
    #[must_use]
    pub fn kernel(&self) -> HybridRouteGrant {
        HybridRouteGrant {
            grant_id: self.grant_id.clone(),
            route_id: self.route_id.clone(),
            candidate_sha256: self.candidate_sha256.clone(),
            provider_id: self.provider_id.clone(),
            data_classes: self
                .data_classes
                .iter()
                .map(|class| class.kernel())
                .collect(),
            max_requests: self.max_requests,
            max_input_tokens: self.max_input_tokens,
            fallback_allowed: self.fallback_allowed,
            expires_at_epoch_ms: self.expires_at_epoch_ms,
            grant_sha256: self.grant_sha256.clone(),
        }
    }

    /// The digest the grant must carry, as the kernel computes it.
    #[must_use]
    pub fn computed_sha256(&self) -> Option<String> {
        hybrid_route_grant_digest(&self.kernel()).ok()
    }

    /// The grant a request file asks for, dated from `now_epoch_ms` and
    /// sealed with its digest, or `None` when the request is malformed.
    #[must_use]
    pub fn from_file(file: &RouteGrantFile, now_epoch_ms: u64) -> Option<Self> {
        if file.schema_version != 1
            || file.valid_for_hours == 0
            || file.valid_for_hours > MAX_GRANT_VALIDITY_HOURS
        {
            return None;
        }
        let mut grant = Self {
            grant_id: file.grant_id.clone(),
            route_id: file.route_id.clone(),
            candidate_sha256: file.candidate_sha256.clone(),
            provider_id: file.provider_id.clone(),
            data_classes: file
                .data_classes
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            max_requests: file.max_requests,
            max_input_tokens: file.max_input_tokens,
            fallback_allowed: file.fallback_allowed,
            expires_at_epoch_ms: now_epoch_ms
                .checked_add(u64::from(file.valid_for_hours) * HOUR_MS)?,
            grant_sha256: "0".repeat(64),
        };
        grant.grant_sha256 = grant.computed_sha256()?;
        (grant.data_classes.len() == file.data_classes.len() && grant.well_formed())
            .then_some(grant)
    }

    /// Identities, data classes, budgets and digest within their rules.
    fn well_formed(&self) -> bool {
        route_grant_identifier(&self.grant_id)
            && route_grant_identifier(&self.route_id)
            && route_grant_identifier(&self.provider_id)
            && lower_hex_sha256(&self.candidate_sha256)
            && !self.data_classes.is_empty()
            && self.data_classes.windows(2).all(|pair| pair[0] < pair[1])
            && (1..=MAX_GRANT_REQUESTS).contains(&self.max_requests)
            && (1..=MAX_GRANT_INPUT_TOKENS).contains(&self.max_input_tokens)
            && self.computed_sha256().as_deref() == Some(self.grant_sha256.as_str())
    }
}

/// One grant as the catalog keeps it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeptGrant {
    grant: RouteGrant,
    granted_at_epoch_ms: u64,
    decision_sha256: String,
    used_requests: u32,
    used_input_tokens: u64,
    #[serde(deserialize_with = "required_option")]
    revoked_at_epoch_ms: Option<u64>,
}

impl KeptGrant {
    fn live(&self, now_epoch_ms: u64) -> bool {
        self.revoked_at_epoch_ms.is_none() && now_epoch_ms < self.grant.expires_at_epoch_ms
    }

    fn state(&self, now_epoch_ms: u64) -> RouteGrantState {
        if self.revoked_at_epoch_ms.is_some() {
            RouteGrantState::Revoked
        } else if now_epoch_ms >= self.grant.expires_at_epoch_ms {
            RouteGrantState::Expired
        } else if self.used_requests >= self.grant.max_requests
            || self.used_input_tokens >= self.grant.max_input_tokens
        {
            RouteGrantState::Exhausted
        } else {
            RouteGrantState::Live
        }
    }

    fn valid(&self) -> bool {
        self.grant.well_formed()
            && lower_hex_sha256(&self.decision_sha256)
            && self.granted_at_epoch_ms < self.grant.expires_at_epoch_ms
            && self.grant.expires_at_epoch_ms - self.granted_at_epoch_ms <= MAX_EXPIRY_AFTER_MS
            && self.used_requests <= self.grant.max_requests
            && self.used_input_tokens <= self.grant.max_input_tokens
            && self
                .revoked_at_epoch_ms
                .is_none_or(|revoked| revoked >= self.granted_at_epoch_ms)
    }
}

/// The whole catalog, in its canonical encoding.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogState {
    schema_version: u16,
    scopes: BTreeMap<String, Vec<KeptGrant>>,
}

/// Listed state of one grant at the host's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteGrantState {
    /// Neither revoked nor expired, with budget left.
    Live,
    /// Neither revoked nor expired, but a counter reached its budget.
    Exhausted,
    /// Past its expiry.
    Expired,
    /// Revoked by the person.
    Revoked,
}

impl RouteGrantState {
    const fn name(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Exhausted => "exhausted",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }
}

/// One grant as the person sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteGrantView {
    /// The workspace whose runs the grant covers.
    pub workspace_id: String,
    /// The grant.
    pub grant: RouteGrant,
    /// Its state at the host's clock.
    pub state: RouteGrantState,
    /// Requests counted against it.
    pub used_requests: u32,
    /// Input tokens counted against it.
    pub used_input_tokens: u64,
    /// When it was granted, in Unix epoch milliseconds.
    pub granted_at_epoch_ms: u64,
    /// When it was revoked, if it was.
    #[serde(deserialize_with = "required_option")]
    pub revoked_at_epoch_ms: Option<u64>,
}

/// The receipt of one transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteGrantReceipt {
    /// Catalog revision after the transition.
    pub catalog_revision: u64,
    /// Digest of the catalog after the transition.
    pub catalog_sha256: String,
    /// Digest of the person's decision: the exact request.
    pub decision_sha256: String,
}

/// One route grant request of a catalog client. Every member is required and
/// no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RouteGrantRequest {
    /// The person grants one remote route to the runs of one workspace.
    Grant {
        /// The workspace.
        workspace_id: String,
        /// The sealed grant.
        grant: RouteGrant,
    },
    /// Every grant, or every grant of one workspace.
    List {
        /// One workspace, or every workspace.
        #[serde(deserialize_with = "required_option")]
        workspace_id: Option<String>,
    },
    /// Revokes one grant of a workspace.
    Revoke {
        /// The workspace.
        workspace_id: String,
        /// The grant.
        grant_id: String,
    },
}

/// Content-free reason a route grant request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteGrantRefusal {
    /// A value is malformed or out of bounds, or the digest does not recompute.
    InvalidInput,
    /// The expiry is not after the host's clock, or too far after it.
    ExpiryOutOfRange,
    /// The workspace already holds a grant with this identity.
    Duplicate,
    /// The workspace already holds a live grant for this route.
    RouteAlreadyGranted,
    /// No such grant in the workspace.
    NotFound,
    /// The grant is already revoked.
    AlreadyRevoked,
    /// The grant is not live, or cannot admit the request.
    NotAdmitted,
    /// The catalog would exceed a bound.
    ResourceLimit,
    /// The host has no clock reading, or it is earlier than the catalog's.
    ClockUnavailable,
    /// The store could not be opened, read or written.
    StoreUnavailable,
    /// The catalog changed underneath this operation.
    StoreConflict,
    /// The stored catalog failed its checks.
    StoreIntegrity,
}

impl RouteGrantRefusal {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "route-grant.invalid-input",
            Self::ExpiryOutOfRange => "route-grant.expiry-out-of-range",
            Self::Duplicate => "route-grant.duplicate",
            Self::RouteAlreadyGranted => "route-grant.route-already-granted",
            Self::NotFound => "route-grant.not-found",
            Self::AlreadyRevoked => "route-grant.already-revoked",
            Self::NotAdmitted => "route-grant.not-admitted",
            Self::ResourceLimit => "route-grant.resource-limit",
            Self::ClockUnavailable => "route-grant.clock-unavailable",
            Self::StoreUnavailable => "route-grant.store-unavailable",
            Self::StoreConflict => "route-grant.store-conflict",
            Self::StoreIntegrity => "route-grant.store-integrity",
        }
    }

    const fn of_store(error: OwnerStateStoreError) -> Self {
        match error {
            OwnerStateStoreError::Stale => Self::StoreConflict,
            OwnerStateStoreError::Integrity => Self::StoreIntegrity,
            OwnerStateStoreError::ResourceLimit => Self::ResourceLimit,
            OwnerStateStoreError::InvalidInput
            | OwnerStateStoreError::Storage
            | OwnerStateStoreError::Unavailable => Self::StoreUnavailable,
        }
    }
}

/// One route grant answer of the catalog host. Every member is required and
/// no other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RouteGrantAnswer {
    /// The grant was kept.
    Granted {
        /// The kept grant.
        grant: RouteGrantView,
        /// The transition.
        receipt: RouteGrantReceipt,
    },
    /// The requested grants.
    Listed {
        /// The grants, by workspace and then in the order they were granted.
        grants: Vec<RouteGrantView>,
        /// Catalog revision.
        catalog_revision: u64,
    },
    /// The grant was revoked.
    Revoked {
        /// The revoked grant.
        grant: RouteGrantView,
        /// The transition.
        receipt: RouteGrantReceipt,
    },
    /// The request was refused and changed nothing.
    Refused {
        /// Why.
        refusal: RouteGrantRefusal,
    },
}

/// One live grant of a workspace and what its owner counted against it, as a
/// routing request carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantedRoute {
    /// The grant.
    pub grant: RouteGrant,
    /// Requests counted against it.
    pub used_requests: u32,
    /// Input tokens counted against it.
    pub used_input_tokens: u64,
}

impl GrantedRoute {
    /// The kernel's granted route.
    #[must_use]
    pub fn kernel(&self) -> GrantedHybridRoute {
        GrantedHybridRoute {
            grant: self.grant.kernel(),
            used_requests: self.used_requests,
            used_input_tokens: self.used_input_tokens,
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// The first twelve characters of a digest, for display.
fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

fn lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether a value is a plain identity: ASCII letters, digits, hyphens,
/// underscores, dots and colons, beginning with a letter or digit, at most
/// 128 bytes.
#[must_use]
pub fn route_grant_identifier(value: &str) -> bool {
    value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

/// The digest of the person's decision: the SHA-256 of the exact request in
/// its wire form, which names the workspace and the sealed grant.
#[must_use]
pub fn route_grant_decision_sha256(request: &RouteGrantRequest) -> Option<String> {
    serde_json::to_vec(request)
        .ok()
        .map(|bytes| sha256_hex(&bytes))
}

/// Whether an answer acknowledges exactly the request that was sent: the
/// matching kind, the decision digest of that request, and only the
/// workspace and grant it named.
#[must_use]
pub fn route_grant_answer_acknowledges(
    request: &RouteGrantRequest,
    answer: &RouteGrantAnswer,
) -> bool {
    let decision = route_grant_decision_sha256(request);
    let decided = |receipt: &RouteGrantReceipt| Some(&receipt.decision_sha256) == decision.as_ref();
    match (request, answer) {
        (_, RouteGrantAnswer::Refused { .. }) => true,
        (
            RouteGrantRequest::Grant {
                workspace_id,
                grant,
            },
            RouteGrantAnswer::Granted {
                grant: view,
                receipt,
            },
        ) => {
            decided(receipt)
                && view.workspace_id == *workspace_id
                && view.grant == *grant
                && view.state == RouteGrantState::Live
                && view.used_requests == 0
                && view.used_input_tokens == 0
                && view.revoked_at_epoch_ms.is_none()
        }
        (RouteGrantRequest::List { workspace_id }, RouteGrantAnswer::Listed { grants, .. }) => {
            grants.iter().all(|view| {
                workspace_id
                    .as_ref()
                    .is_none_or(|workspace| view.workspace_id == *workspace)
            })
        }
        (
            RouteGrantRequest::Revoke {
                workspace_id,
                grant_id,
            },
            RouteGrantAnswer::Revoked {
                grant: view,
                receipt,
            },
        ) => {
            decided(receipt)
                && view.workspace_id == *workspace_id
                && view.grant.grant_id == *grant_id
                && view.state == RouteGrantState::Revoked
                && view.revoked_at_epoch_ms.is_some()
        }
        _ => false,
    }
}

fn view(workspace_id: &str, kept: &KeptGrant, now_epoch_ms: u64) -> RouteGrantView {
    RouteGrantView {
        workspace_id: workspace_id.to_owned(),
        grant: kept.grant.clone(),
        state: kept.state(now_epoch_ms),
        used_requests: kept.used_requests,
        used_input_tokens: kept.used_input_tokens,
        granted_at_epoch_ms: kept.granted_at_epoch_ms,
        revoked_at_epoch_ms: kept.revoked_at_epoch_ms,
    }
}

fn encode(state: &CatalogState) -> Result<Vec<u8>, RouteGrantRefusal> {
    serde_json::to_vec(state)
        .ok()
        .filter(|bytes| bytes.len() <= MAX_STATE_BYTES)
        .ok_or(RouteGrantRefusal::ResourceLimit)
}

/// Decodes a stored catalog exactly and checks every grant and bound.
fn decode(bytes: &[u8]) -> Result<CatalogState, RouteGrantRefusal> {
    let state: CatalogState =
        serde_json::from_slice(bytes).map_err(|_| RouteGrantRefusal::StoreIntegrity)?;
    let valid = state.schema_version == STATE_SCHEMA_VERSION
        && state.scopes.len() <= MAX_ROUTE_GRANT_SCOPES
        && state.scopes.iter().all(|(workspace_id, grants)| {
            let mut identities = BTreeSet::new();
            route_grant_identifier(workspace_id)
                && !grants.is_empty()
                && grants.len() <= MAX_GRANTS_PER_SCOPE
                && grants
                    .iter()
                    .all(|kept| kept.valid() && identities.insert(kept.grant.grant_id.as_str()))
                && grants
                    .windows(2)
                    .all(|pair| pair[0].granted_at_epoch_ms <= pair[1].granted_at_epoch_ms)
        })
        && serde_json::to_vec(&state).is_ok_and(|encoded| encoded == bytes);
    if valid {
        Ok(state)
    } else {
        Err(RouteGrantRefusal::StoreIntegrity)
    }
}

/// The stored catalog and the revision it was read at.
fn load(states: &DurableOwnerStates) -> Result<(CatalogState, u64), RouteGrantRefusal> {
    let stored = states
        .load(OwnerStateName::RouteGrantCatalog)
        .map_err(RouteGrantRefusal::of_store)?;
    let state = if stored.revision == 0 {
        CatalogState {
            schema_version: STATE_SCHEMA_VERSION,
            scopes: BTreeMap::new(),
        }
    } else {
        decode(&stored.state)?
    };
    Ok((state, stored.revision))
}

/// Commits the next catalog under the revision it was read at.
fn commit(
    states: &DurableOwnerStates,
    state: &CatalogState,
    read_revision: u64,
    decision_sha256: String,
) -> Result<RouteGrantReceipt, RouteGrantRefusal> {
    let bytes = encode(state)?;
    let catalog_revision = states
        .commit(OwnerStateName::RouteGrantCatalog, read_revision, &bytes)
        .map_err(RouteGrantRefusal::of_store)?;
    Ok(RouteGrantReceipt {
        catalog_revision,
        catalog_sha256: sha256_hex(&bytes),
        decision_sha256,
    })
}

/// Whether the host's clock is not earlier than any grant the catalog kept,
/// so a grant that expired cannot appear live again.
fn clock_current(state: &CatalogState, now_epoch_ms: u64) -> bool {
    state.scopes.values().flatten().all(|kept| {
        kept.granted_at_epoch_ms <= now_epoch_ms
            && kept
                .revoked_at_epoch_ms
                .is_none_or(|revoked| revoked <= now_epoch_ms)
    })
}

/// Answers one route grant request. `open` opens the store for the
/// operation; `now_epoch_ms` is the host's clock, if it has one.
pub fn answer_route_grant(
    request: &RouteGrantRequest,
    open: &mut dyn FnMut() -> Result<DurableOwnerStates, RouteGrantRefusal>,
    now_epoch_ms: Option<u64>,
) -> RouteGrantAnswer {
    match decide(request, open, now_epoch_ms) {
        Ok(answer) => answer,
        Err(refusal) => RouteGrantAnswer::Refused { refusal },
    }
}

fn decide(
    request: &RouteGrantRequest,
    open: &mut dyn FnMut() -> Result<DurableOwnerStates, RouteGrantRefusal>,
    now_epoch_ms: Option<u64>,
) -> Result<RouteGrantAnswer, RouteGrantRefusal> {
    validate(request)?;
    let now = now_epoch_ms.ok_or(RouteGrantRefusal::ClockUnavailable)?;
    let decision_sha256 =
        route_grant_decision_sha256(request).ok_or(RouteGrantRefusal::InvalidInput)?;
    let states = open()?;
    let (mut state, revision) = load(&states)?;
    if !clock_current(&state, now) {
        return Err(RouteGrantRefusal::ClockUnavailable);
    }
    match request {
        RouteGrantRequest::List { workspace_id } => Ok(RouteGrantAnswer::Listed {
            grants: state
                .scopes
                .iter()
                .filter(|(scope, _)| {
                    workspace_id
                        .as_ref()
                        .is_none_or(|workspace| *scope == workspace)
                })
                .flat_map(|(scope, grants)| grants.iter().map(|kept| view(scope, kept, now)))
                .collect(),
            catalog_revision: revision,
        }),
        RouteGrantRequest::Grant {
            workspace_id,
            grant,
        } => {
            if grant.expires_at_epoch_ms <= now
                || grant.expires_at_epoch_ms - now > MAX_EXPIRY_AFTER_MS
            {
                return Err(RouteGrantRefusal::ExpiryOutOfRange);
            }
            if !state.scopes.contains_key(workspace_id)
                && state.scopes.len() >= MAX_ROUTE_GRANT_SCOPES
            {
                return Err(RouteGrantRefusal::ResourceLimit);
            }
            let grants = state.scopes.entry(workspace_id.clone()).or_default();
            if grants
                .iter()
                .any(|kept| kept.grant.grant_id == grant.grant_id)
            {
                return Err(RouteGrantRefusal::Duplicate);
            }
            if grants
                .iter()
                .any(|kept| kept.live(now) && kept.grant.route_id == grant.route_id)
            {
                return Err(RouteGrantRefusal::RouteAlreadyGranted);
            }
            if grants.len() >= MAX_GRANTS_PER_SCOPE
                || grants.iter().filter(|kept| kept.live(now)).count() >= MAX_LIVE_GRANTS_PER_SCOPE
            {
                return Err(RouteGrantRefusal::ResourceLimit);
            }
            let kept = KeptGrant {
                grant: grant.clone(),
                granted_at_epoch_ms: now,
                decision_sha256: decision_sha256.clone(),
                used_requests: 0,
                used_input_tokens: 0,
                revoked_at_epoch_ms: None,
            };
            if !kept.valid() {
                return Err(RouteGrantRefusal::InvalidInput);
            }
            let listed = view(workspace_id, &kept, now);
            grants.push(kept);
            let receipt = commit(&states, &state, revision, decision_sha256)?;
            Ok(RouteGrantAnswer::Granted {
                grant: listed,
                receipt,
            })
        }
        RouteGrantRequest::Revoke {
            workspace_id,
            grant_id,
        } => {
            let kept = state
                .scopes
                .get_mut(workspace_id)
                .and_then(|grants| {
                    grants
                        .iter_mut()
                        .find(|kept| kept.grant.grant_id == *grant_id)
                })
                .ok_or(RouteGrantRefusal::NotFound)?;
            if kept.revoked_at_epoch_ms.is_some() {
                return Err(RouteGrantRefusal::AlreadyRevoked);
            }
            kept.revoked_at_epoch_ms = Some(now);
            let listed = view(workspace_id, kept, now);
            let receipt = commit(&states, &state, revision, decision_sha256)?;
            Ok(RouteGrantAnswer::Revoked {
                grant: listed,
                receipt,
            })
        }
    }
}

fn validate(request: &RouteGrantRequest) -> Result<(), RouteGrantRefusal> {
    let valid = match request {
        RouteGrantRequest::Grant {
            workspace_id,
            grant,
        } => route_grant_identifier(workspace_id) && grant.well_formed(),
        RouteGrantRequest::List { workspace_id } => {
            workspace_id.as_deref().is_none_or(route_grant_identifier)
        }
        RouteGrantRequest::Revoke {
            workspace_id,
            grant_id,
        } => route_grant_identifier(workspace_id) && route_grant_identifier(grant_id),
    };
    if valid {
        Ok(())
    } else {
        Err(RouteGrantRefusal::InvalidInput)
    }
}

/// The live grants of one workspace with their counters, for a run's routing
/// request: neither revoked nor expired at `now_epoch_ms`, at most one per
/// route and at most the router's bound. A catalog whose live grants break
/// either rule, or whose clock went backwards, is refused.
pub fn live_route_grants(
    states: &DurableOwnerStates,
    workspace_id: &str,
    now_epoch_ms: u64,
) -> Result<Vec<GrantedRoute>, RouteGrantRefusal> {
    if !route_grant_identifier(workspace_id) {
        return Err(RouteGrantRefusal::InvalidInput);
    }
    let (state, _) = load(states)?;
    if !clock_current(&state, now_epoch_ms) {
        return Err(RouteGrantRefusal::ClockUnavailable);
    }
    let live = state
        .scopes
        .get(workspace_id)
        .into_iter()
        .flatten()
        .filter(|kept| kept.live(now_epoch_ms))
        .map(|kept| GrantedRoute {
            grant: kept.grant.clone(),
            used_requests: kept.used_requests,
            used_input_tokens: kept.used_input_tokens,
        })
        .collect::<Vec<_>>();
    let routes = live
        .iter()
        .map(|granted| granted.grant.route_id.as_str())
        .collect::<BTreeSet<_>>();
    if routes.len() != live.len() || live.len() > MAX_LIVE_GRANTS_PER_SCOPE {
        return Err(RouteGrantRefusal::StoreIntegrity);
    }
    Ok(live)
}

/// Counts one request of `input_tokens` against the live grant of a
/// workspace with the digest `grant_sha256`, under the revision the owner
/// read, and returns the grant's counters after it. A grant that is revoked,
/// expired or cannot admit the request is refused and nothing is counted.
/// Offered for a later remote adapter (AMR-05.9.7.2); the development host
/// calls it nowhere, because it sends no request to a remote route.
pub fn count_route_grant_request(
    states: &DurableOwnerStates,
    workspace_id: &str,
    grant_sha256: &str,
    input_tokens: u64,
    now_epoch_ms: u64,
) -> Result<(u32, u64), RouteGrantRefusal> {
    if !route_grant_identifier(workspace_id) || !lower_hex_sha256(grant_sha256) || input_tokens == 0
    {
        return Err(RouteGrantRefusal::InvalidInput);
    }
    let (mut state, revision) = load(states)?;
    if !clock_current(&state, now_epoch_ms) {
        return Err(RouteGrantRefusal::ClockUnavailable);
    }
    let kept = state
        .scopes
        .get_mut(workspace_id)
        .and_then(|grants| {
            grants
                .iter_mut()
                .find(|kept| kept.grant.grant_sha256 == grant_sha256)
        })
        .ok_or(RouteGrantRefusal::NotFound)?;
    let requests = kept.used_requests.checked_add(1);
    let tokens = kept.used_input_tokens.checked_add(input_tokens);
    let (Some(requests), Some(tokens)) = (requests, tokens) else {
        return Err(RouteGrantRefusal::NotAdmitted);
    };
    if !kept.live(now_epoch_ms)
        || requests > kept.grant.max_requests
        || tokens > kept.grant.max_input_tokens
    {
        return Err(RouteGrantRefusal::NotAdmitted);
    }
    kept.used_requests = requests;
    kept.used_input_tokens = tokens;
    let decision = sha256_hex(
        format!("agentmage-route-grant-count:{workspace_id}:{grant_sha256}:{requests}:{tokens}")
            .as_bytes(),
    );
    commit(states, &state, revision, decision)?;
    Ok((requests, tokens))
}

/// The UTC timestamp of a Unix epoch time in milliseconds, or the number.
fn timestamp(epoch_ms: u64) -> String {
    crate::coding_memory::memory_timestamp(epoch_ms).unwrap_or_else(|| epoch_ms.to_string())
}

fn data_names(grant: &RouteGrant) -> String {
    grant
        .data_classes
        .iter()
        .map(|class| class.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// What the person is asked to confirm before a grant is sent: what the route
/// may receive, from which workspace's runs, for how much and until when,
/// and that this development host offers no remote route yet.
#[must_use]
pub fn render_route_grant_preview(workspace_id: &str, grant: &RouteGrant) -> String {
    format!(
        "route grant {grant_id} would let remote route {route} of provider {provider} (candidate {candidate}) \
receive {data} from runs in workspace {workspace_id}\n\
- at most {requests} requests and {tokens} input tokens, until {expiry}\n\
- {fallback}\n\
- grant digest {digest}\n\
This development host offers no remote route yet: no run sends anything off this machine until a host offers this route.\n",
        grant_id = grant.grant_id,
        route = grant.route_id,
        provider = grant.provider_id,
        candidate = short(&grant.candidate_sha256),
        data = data_names(grant),
        requests = grant.max_requests,
        tokens = grant.max_input_tokens,
        expiry = timestamp(grant.expires_at_epoch_ms),
        fallback = if grant.fallback_allowed {
            "it may replace a failed route"
        } else {
            "it may not replace a failed route"
        },
        digest = short(&grant.grant_sha256),
    )
}

fn render_view(view: &RouteGrantView) -> String {
    format!(
        "- {workspace} {grant_id} {state}: route {route} of provider {provider}; data {data}; {used_requests} of {max_requests} requests and {used_tokens} of {max_tokens} input tokens used; granted {granted}; expires {expiry}{revoked}; digest {digest}\n",
        workspace = view.workspace_id,
        grant_id = view.grant.grant_id,
        state = view.state.name(),
        route = view.grant.route_id,
        provider = view.grant.provider_id,
        data = data_names(&view.grant),
        used_requests = view.used_requests,
        max_requests = view.grant.max_requests,
        used_tokens = view.used_input_tokens,
        max_tokens = view.grant.max_input_tokens,
        granted = timestamp(view.granted_at_epoch_ms),
        expiry = timestamp(view.grant.expires_at_epoch_ms),
        revoked = view
            .revoked_at_epoch_ms
            .map(|revoked| format!("; revoked {}", timestamp(revoked)))
            .unwrap_or_default(),
        digest = short(&view.grant.grant_sha256),
    )
}

/// The answer for a person on standard output, or one JSON row.
#[must_use]
pub fn render_route_grant_answer(answer: &RouteGrantAnswer, json: bool) -> String {
    if json {
        let row = match answer {
            RouteGrantAnswer::Granted { grant, receipt } => serde_json::json!({
                "type": "route_grant", "result": "granted", "grant": grant, "receipt": receipt,
            }),
            RouteGrantAnswer::Listed {
                grants,
                catalog_revision,
            } => serde_json::json!({
                "type": "route_grant", "result": "listed", "grants": grants,
                "catalog_revision": catalog_revision,
            }),
            RouteGrantAnswer::Revoked { grant, receipt } => serde_json::json!({
                "type": "route_grant", "result": "revoked", "grant": grant, "receipt": receipt,
            }),
            RouteGrantAnswer::Refused { refusal } => serde_json::json!({
                "type": "route_grant", "result": "refused", "refusal": refusal.code(),
            }),
        };
        return format!("{row}\n");
    }
    match answer {
        RouteGrantAnswer::Granted { grant, receipt } => format!(
            "route grant kept at catalog revision {}\n{}",
            receipt.catalog_revision,
            render_view(grant)
        ),
        RouteGrantAnswer::Listed {
            grants,
            catalog_revision,
        } => {
            let mut text = format!(
                "route grants at catalog revision {catalog_revision}: {}\n",
                grants.len()
            );
            for grant in grants {
                text.push_str(&render_view(grant));
            }
            text
        }
        RouteGrantAnswer::Revoked { grant, receipt } => format!(
            "route grant revoked at catalog revision {}\n{}",
            receipt.catalog_revision,
            render_view(grant)
        ),
        RouteGrantAnswer::Refused { refusal } => {
            format!("route grant refused: {}\n", refusal.code())
        }
    }
}

/// A route grant operation of the development CLI (Decision 0144).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteGrantCommand {
    /// Grants the route a request file names to a workspace's runs.
    Grant {
        /// The workspace.
        workspace_id: String,
        /// Absolute path of the grant request file.
        file: std::path::PathBuf,
    },
    /// Lists every grant, or every grant of one workspace.
    List {
        /// One workspace, or every workspace.
        workspace_id: Option<String>,
    },
    /// Revokes one grant of a workspace.
    Revoke {
        /// The workspace.
        workspace_id: String,
        /// The grant.
        grant_id: String,
    },
}

/// The request a command sends, the grant dated from the client's clock at
/// `now_epoch_ms`. A grant request file that cannot be read, is not exactly
/// one closed request, or asks for a malformed grant is refused before any
/// host is launched.
#[cfg(target_os = "linux")]
pub fn route_grant_request(
    command: &RouteGrantCommand,
    now_epoch_ms: u64,
) -> Result<RouteGrantRequest, RouteGrantRefusal> {
    Ok(match command {
        RouteGrantCommand::Grant { workspace_id, file } => {
            let bytes =
                crate::coding_extensions::read_bounded_file(file, MAX_ROUTE_GRANT_FILE_BYTES)
                    .ok_or(RouteGrantRefusal::InvalidInput)?;
            grant_request_of(workspace_id, &bytes, now_epoch_ms)?
        }
        RouteGrantCommand::List { workspace_id } => RouteGrantRequest::List {
            workspace_id: workspace_id.clone(),
        },
        RouteGrantCommand::Revoke {
            workspace_id,
            grant_id,
        } => RouteGrantRequest::Revoke {
            workspace_id: workspace_id.clone(),
            grant_id: grant_id.clone(),
        },
    })
}

/// The grant request of one request file's bytes: exactly one closed
/// request within the file bound, sealed from `now_epoch_ms`.
fn grant_request_of(
    workspace_id: &str,
    bytes: &[u8],
    now_epoch_ms: u64,
) -> Result<RouteGrantRequest, RouteGrantRefusal> {
    if bytes.len() as u64 > MAX_ROUTE_GRANT_FILE_BYTES {
        return Err(RouteGrantRefusal::InvalidInput);
    }
    let request: RouteGrantFile =
        serde_json::from_slice(bytes).map_err(|_| RouteGrantRefusal::InvalidInput)?;
    Ok(RouteGrantRequest::Grant {
        workspace_id: workspace_id.to_owned(),
        grant: RouteGrant::from_file(&request, now_epoch_ms)
            .ok_or(RouteGrantRefusal::InvalidInput)?,
    })
}

/// The exit class of a refusal.
#[must_use]
pub const fn route_grant_refusal_exit(
    refusal: RouteGrantRefusal,
) -> crate::headless::ClientExitCode {
    use crate::headless::ClientExitCode;
    match refusal {
        RouteGrantRefusal::InvalidInput => ClientExitCode::InvalidInput,
        RouteGrantRefusal::ExpiryOutOfRange
        | RouteGrantRefusal::Duplicate
        | RouteGrantRefusal::RouteAlreadyGranted
        | RouteGrantRefusal::NotFound
        | RouteGrantRefusal::AlreadyRevoked
        | RouteGrantRefusal::NotAdmitted => ClientExitCode::PolicyDenied,
        RouteGrantRefusal::ResourceLimit => ClientExitCode::ResourceBound,
        RouteGrantRefusal::ClockUnavailable
        | RouteGrantRefusal::StoreUnavailable
        | RouteGrantRefusal::StoreConflict
        | RouteGrantRefusal::StoreIntegrity => ClientExitCode::ServiceUnavailable,
    }
}

/// A refusal for a person on standard error, or one JSON row.
#[must_use]
pub fn render_route_grant_refusal(refusal: RouteGrantRefusal, json: bool) -> String {
    if json {
        format!(
            "{}\n",
            serde_json::json!({"type": "route_grant", "result": "refused", "refusal": refusal.code()})
        )
    } else {
        format!("route grant refused: {}\n", refusal.code())
    }
}

#[cfg(all(
    test,
    target_os = "linux",
    feature = "source-artifacts",
    feature = "workflow-supervisor"
))]
#[path = "coding_route_grants_tests.rs"]
mod tests;
