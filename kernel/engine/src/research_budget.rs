//! Additional research restrictions, never network grants or transport attestations.
//!
//! The existing authority/effect owner must independently authorize each operation,
//! resolve and pin its connection, and persist reservations before dispatch. This module
//! performs no I/O and cannot convert provider/model content into permission.

use std::collections::BTreeSet;
use std::net::IpAddr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::persistence::detect_secret_classes;
use crate::research_fetch::{PublicGetTarget, PublicSearchEndpoint, ResearchFetchError};

/// Closed first-increment research modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchDepth {
    /// One query and at most five source visits.
    Quick,
    /// At most six queries and twenty visits.
    Deep,
}

/// Distinct network-disclosure choices, independent of model inference routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchNetworkMode {
    /// No outbound research, including search-provider requests.
    Offline,
    /// Every exact disclosure needs a fresh approval.
    Ask,
    /// A separately validated bounded task grant is required.
    TaskAuthorized,
}

/// Configurable downward-only resource ceilings accepted by Decision 0082.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchLimits {
    /// Search-query count, including failed and uncertain attempts.
    pub queries: u16,
    /// Page-visit count, including failures.
    pub visits: u16,
    /// Maximum exact destination domains, including the provider.
    pub domains: u16,
    /// Aggregate reserved response bytes, including redirect bodies.
    pub downloaded_bytes: u64,
    /// Maximum redirects per operation; zero disables redirects.
    pub redirects: u8,
    /// Whole task elapsed ceiling; resume must not restart the clock.
    pub elapsed_ms: u64,
    /// Fixed-point provider charge allowance; this increment requires zero.
    pub provider_charge_microunits: u64,
}

impl ResearchLimits {
    /// Returns the frozen ceiling, not a grant to consume it.
    #[must_use]
    pub const fn ceiling(depth: ResearchDepth) -> Self {
        match depth {
            ResearchDepth::Quick => Self {
                queries: 1,
                visits: 5,
                domains: 8,
                downloaded_bytes: 1_048_576,
                redirects: 3,
                elapsed_ms: 60_000,
                provider_charge_microunits: 0,
            },
            ResearchDepth::Deep => Self {
                queries: 6,
                visits: 20,
                domains: 16,
                downloaded_bytes: 8 * 1_048_576,
                redirects: 3,
                elapsed_ms: 300_000,
                provider_charge_microunits: 0,
            },
        }
    }

    fn validate(&self, depth: ResearchDepth) -> bool {
        let ceiling = Self::ceiling(depth);
        self.queries > 0
            && self.queries <= ceiling.queries
            && self.visits > 0
            && self.visits <= ceiling.visits
            && self.domains > 0
            && self.domains <= ceiling.domains
            && self.downloaded_bytes > 0
            && self.downloaded_bytes <= ceiling.downloaded_bytes
            && self.redirects <= ceiling.redirects
            && self.elapsed_ms > 0
            && self.elapsed_ms <= ceiling.elapsed_ms
            && self.provider_charge_microunits == 0
    }
}

/// Content-free refusal, suitable for the existing redacted event stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchBudgetError {
    /// Unsupported schema, identity or limits.
    Invalid,
    /// Offline is active.
    Offline,
    /// Query contains a detected secret; neither its value nor digest is returned.
    Secret,
    /// Destination or DNS answer is outside the exact public scope.
    Destination,
    /// Cancellation is terminal for this budget instance.
    Cancelled,
    /// Expired, rolled-back clock or exhausted resource ceiling.
    Exhausted,
    /// Replayed operation or changed task/policy binding.
    Binding,
}

/// Immutable restriction set. Not deserializable as an authority object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchScope {
    task_id: String,
    depth: ResearchDepth,
    network: ResearchNetworkMode,
    limits: ResearchLimits,
    domains: BTreeSet<String>,
    query_sha256: BTreeSet<String>,
    // Schema 2 only. Schema 1 scopes cannot bind queries to requests.
    search_endpoint: Option<PublicSearchEndpoint>,
    policy_sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResearchScopeSnapshot {
    schema_version: u16,
    task_id: String,
    depth: ResearchDepth,
    network: ResearchNetworkMode,
    limits: ResearchLimits,
    domains: Vec<String>,
    query_sha256: Vec<String>,
    // Absent in schema 1 so its canonical bytes stay unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    search_endpoint: Option<PublicSearchEndpoint>,
    policy_sha256: String,
}

impl ResearchScope {
    /// Canonical-owner metadata only. Full disclosed plan bytes remain in the
    /// existing artifact store; query digests cannot replace that full plan.
    pub(crate) fn snapshot(&self) -> Result<Vec<u8>, ResearchBudgetError> {
        serde_json::to_vec(&ResearchScopeSnapshot {
            schema_version: self.schema_version(),
            task_id: self.task_id.clone(),
            depth: self.depth,
            network: self.network,
            limits: self.limits.clone(),
            domains: self.domains.iter().cloned().collect(),
            query_sha256: self.query_sha256.iter().cloned().collect(),
            search_endpoint: self.search_endpoint.clone(),
            policy_sha256: self.policy_sha256.clone(),
        })
        .map_err(|_| ResearchBudgetError::Invalid)
    }

    /// Revalidates stored restrictions and recomputes their exact content binding.
    /// This is not a user approval, grant or native-worker admission.
    pub(crate) fn restore(bytes: &[u8]) -> Result<Self, ResearchBudgetError> {
        if bytes.is_empty() || bytes.len() > 16 * 1024 {
            return Err(ResearchBudgetError::Invalid);
        }
        let wire: ResearchScopeSnapshot =
            serde_json::from_slice(bytes).map_err(|_| ResearchBudgetError::Invalid)?;
        if wire.schema_version != 1 + u16::from(wire.search_endpoint.is_some())
            || wire.domains.windows(2).any(|pair| pair[0] >= pair[1])
            || wire.query_sha256.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ResearchBudgetError::Invalid);
        }
        let mut restored = Self::from_query_hashes(
            wire.task_id,
            wire.depth,
            wire.network,
            wire.limits,
            wire.domains.into_iter().collect(),
            wire.query_sha256.into_iter().collect(),
        )?;
        if let Some(endpoint) = wire.search_endpoint {
            restored = restored.with_search_endpoint(endpoint)?;
        }
        if restored.policy_sha256 != wire.policy_sha256 {
            return Err(ResearchBudgetError::Binding);
        }
        Ok(restored)
    }

    /// Exact owning task, not an authority token.
    #[must_use]
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Frozen downward-only ceilings for preparation and disclosure previews.
    #[must_use]
    pub const fn limits(&self) -> &ResearchLimits {
        &self.limits
    }

    /// Checks only the declared host restriction; no DNS or peer proof is implied.
    pub fn check_host(&self, host: &str) -> Result<(), ResearchBudgetError> {
        self.network_requirement()?;
        if self.domains.contains(host) {
            Ok(())
        } else {
            Err(ResearchBudgetError::Destination)
        }
    }

    /// Binds a task to exact proposed query disclosures after secret refusal.
    /// Preparation is not evidence of user consent: the existing authority owner
    /// must separately approve these exact disclosures, never retrieved instructions.
    pub fn new(
        task_id: String,
        depth: ResearchDepth,
        network: ResearchNetworkMode,
        limits: ResearchLimits,
        domains: BTreeSet<String>,
        disclosed_queries: &[String],
    ) -> Result<Self, ResearchBudgetError> {
        if !valid_id(&task_id)
            || !limits.validate(depth)
            || domains.is_empty()
            || domains.len() > usize::from(limits.domains)
            || domains.iter().any(|domain| !public_dns_name(domain))
            || disclosed_queries.is_empty()
            || disclosed_queries.len() > usize::from(limits.queries)
        {
            return Err(ResearchBudgetError::Invalid);
        }
        let mut query_sha256 = BTreeSet::new();
        for query in disclosed_queries {
            query_sha256.insert(public_query_sha256(query)?);
        }
        if query_sha256.len() != disclosed_queries.len() {
            return Err(ResearchBudgetError::Invalid);
        }
        Self::from_query_hashes(task_id, depth, network, limits, domains, query_sha256)
    }

    fn from_query_hashes(
        task_id: String,
        depth: ResearchDepth,
        network: ResearchNetworkMode,
        limits: ResearchLimits,
        domains: BTreeSet<String>,
        query_sha256: BTreeSet<String>,
    ) -> Result<Self, ResearchBudgetError> {
        if !valid_id(&task_id)
            || !limits.validate(depth)
            || domains.is_empty()
            || domains.len() > usize::from(limits.domains)
            || domains.iter().any(|domain| !public_dns_name(domain))
            || query_sha256.is_empty()
            || query_sha256.len() > usize::from(limits.queries)
            || query_sha256.iter().any(|value| {
                value.len() != 64
                    || value
                        .bytes()
                        .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
            })
        {
            return Err(ResearchBudgetError::Invalid);
        }
        let mut scope = Self {
            task_id,
            depth,
            network,
            limits,
            domains,
            query_sha256,
            search_endpoint: None,
            policy_sha256: String::new(),
        };
        scope.policy_sha256 = scope.policy_digest()?;
        Ok(scope)
    }

    const fn schema_version(&self) -> u16 {
        if self.search_endpoint.is_some() { 2 } else { 1 }
    }

    // Schema 1 keeps its original preimage; schema 2 also binds the endpoint.
    fn policy_digest(&self) -> Result<String, ResearchBudgetError> {
        let (task, depth, network) = (&self.task_id, self.depth, self.network);
        let (limits, domains, queries) = (&self.limits, &self.domains, &self.query_sha256);
        let encoded = match &self.search_endpoint {
            None => serde_json::to_vec(&(1u16, task, depth, network, limits, domains, queries)),
            Some(endpoint) => serde_json::to_vec(&(
                2u16, task, depth, network, limits, domains, queries, endpoint,
            )),
        }
        .map_err(|_| ResearchBudgetError::Invalid)?;
        Ok(digest(&encoded))
    }

    /// Binds the exact disclosed search endpoint (Decision 0106), producing a
    /// schema 2 scope. The endpoint's domain must already be a declared destination.
    pub fn with_search_endpoint(
        mut self,
        endpoint: PublicSearchEndpoint,
    ) -> Result<Self, ResearchBudgetError> {
        if self.search_endpoint.is_some() {
            return Err(ResearchBudgetError::Invalid);
        }
        endpoint.validate().map_err(|error| match error {
            ResearchFetchError::Secret => ResearchBudgetError::Secret,
            _ => ResearchBudgetError::Invalid,
        })?;
        if !self.domains.contains(&endpoint.domain) {
            return Err(ResearchBudgetError::Destination);
        }
        self.search_endpoint = Some(endpoint);
        self.policy_sha256 = self.policy_digest()?;
        Ok(self)
    }

    /// Derives query versus visit from the exact prepared target, never from a
    /// caller label. Only the disclosed endpoint shape carries a search query.
    pub(crate) fn classify<'a>(
        &self,
        target: &'a PublicGetTarget,
    ) -> Result<ResearchOperation<'a>, ResearchBudgetError> {
        // A schema 1 scope cannot tell a provider search from a page visit.
        let endpoint = self
            .search_endpoint
            .as_ref()
            .ok_or(ResearchBudgetError::Invalid)?;
        if target.domain != endpoint.domain {
            return Ok(ResearchOperation::Visit);
        }
        if target.path != endpoint.path || target.query.len() != endpoint.fixed_fields.len() + 1 {
            return Err(ResearchBudgetError::Destination);
        }
        let mut query = None;
        for (name, value) in &target.query {
            if name == &endpoint.query_field {
                query = Some(value.as_str());
            } else if !endpoint
                .fixed_fields
                .iter()
                .any(|(fixed, expected)| fixed == name && expected == value)
            {
                return Err(ResearchBudgetError::Destination);
            }
        }
        query
            .map(ResearchOperation::Query)
            .ok_or(ResearchBudgetError::Destination)
    }

    /// Returns a content binding, never an authorization token.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Exposes the distinct approval requirement to the existing effect owner.
    pub fn network_requirement(&self) -> Result<ResearchNetworkMode, ResearchBudgetError> {
        if self.network == ResearchNetworkMode::Offline {
            Err(ResearchBudgetError::Offline)
        } else {
            Ok(self.network)
        }
    }

    /// Checks the complete DNS answer set before the transport pins a connection.
    /// This is not proof of the actual peer, TLS certificate or redirect destination.
    pub fn check_destination(
        &self,
        host: &str,
        port: u16,
        addresses: &[IpAddr],
        uses_proxy: bool,
    ) -> Result<(), ResearchBudgetError> {
        self.check_host(host)?;
        if uses_proxy
            || port != 443
            || addresses.is_empty()
            || addresses.len() > 16
            || addresses.iter().any(|address| !public_address(*address))
        {
            return Err(ResearchBudgetError::Destination);
        }
        Ok(())
    }

    /// Rechecks one redirect hop against the same exact scope and frozen ceiling.
    /// The transport must also bound response bytes and pin the new connection.
    pub fn check_redirect(
        &self,
        hop: u8,
        host: &str,
        port: u16,
        addresses: &[IpAddr],
        uses_proxy: bool,
    ) -> Result<(), ResearchBudgetError> {
        self.network_requirement()?;
        if hop == 0 || hop > self.limits.redirects {
            return Err(ResearchBudgetError::Exhausted);
        }
        self.check_destination(host, port, addresses, uses_proxy)
    }
}

/// An exact candidate operation, still requiring independent kernel authorization.
/// The canonical owner derives it with `ResearchScope::classify`, never from a caller.
pub enum ResearchOperation<'a> {
    /// Exact previously disclosed query.
    Query(&'a str),
    /// One source fetch; destination is checked separately for every hop.
    Visit,
}

/// Single-owner reservation accounting; serialize through the canonical owner only.
/// There is intentionally no blind deserialize/resume path or refund on uncertain I/O.
#[derive(Debug, Serialize)]
pub struct ResearchBudget {
    schema_version: u16,
    task_id: String,
    policy_sha256: String,
    started_epoch_ms: u64,
    last_epoch_ms: u64,
    queries: u16,
    visits: u16,
    reserved_bytes: u64,
    operation_ids: BTreeSet<String>,
    cancelled: bool,
    deadline_exhausted: bool,
}

/// Descriptive counters from canonical accounting, never execution authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchBudgetProgress {
    /// Original task start; resume cannot replace it.
    pub started_epoch_ms: u64,
    /// Highest nonterminal trusted clock observation already retained.
    pub last_epoch_ms: u64,
    /// Queries already reserved, including failed or uncertain attempts.
    pub queries: u16,
    /// Source visits already reserved, including failed or uncertain attempts.
    pub visits: u16,
    /// Worst-case bytes already reserved, never refunded on failure.
    pub reserved_bytes: u64,
    /// Sticky task cancellation, independent of an external effect outcome.
    pub cancelled: bool,
    /// Sticky observed deadline expiry or clock rollback.
    pub deadline_exhausted: bool,
}

// Kept separate from the live restriction object. Only the canonical owner may
// recover a previously committed snapshot; decoding bytes never issues authority.
// Vec preserves duplicate/order errors that BTreeSet deserialization would erase.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResearchBudgetSnapshot {
    schema_version: u16,
    task_id: String,
    policy_sha256: String,
    started_epoch_ms: u64,
    last_epoch_ms: u64,
    queries: u16,
    visits: u16,
    reserved_bytes: u64,
    operation_ids: Vec<String>,
    cancelled: bool,
    deadline_exhausted: bool,
}

impl ResearchBudget {
    pub(crate) const fn progress(&self) -> ResearchBudgetProgress {
        ResearchBudgetProgress {
            started_epoch_ms: self.started_epoch_ms,
            last_epoch_ms: self.last_epoch_ms,
            queries: self.queries,
            visits: self.visits,
            reserved_bytes: self.reserved_bytes,
            cancelled: self.cancelled,
            deadline_exhausted: self.deadline_exhausted,
        }
    }

    pub(crate) fn reservation_count(&self) -> usize {
        self.operation_ids.len()
    }

    pub(crate) fn contains_operation(&self, operation_id: &str) -> bool {
        self.operation_ids.contains(operation_id)
    }

    pub(crate) const fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Original trusted start used to reconstruct the same per-operation deadline.
    pub(crate) const fn started_epoch_ms(&self) -> u64 {
        self.started_epoch_ms
    }

    /// Recovers restrictions only from bytes verified by the canonical owner.
    /// The owner must additionally verify the original plan, complete revision
    /// chain and current head. This never grants permission or resets a clock.
    pub(crate) fn restore(
        scope: &ResearchScope,
        bytes: &[u8],
    ) -> Result<Self, ResearchBudgetError> {
        if bytes.is_empty() || bytes.len() > 8 * 1024 {
            return Err(ResearchBudgetError::Invalid);
        }
        let snapshot: ResearchBudgetSnapshot =
            serde_json::from_slice(bytes).map_err(|_| ResearchBudgetError::Invalid)?;
        let deadline = snapshot
            .started_epoch_ms
            .checked_add(scope.limits.elapsed_ms)
            .ok_or(ResearchBudgetError::Invalid)?;
        let count = usize::from(snapshot.queries) + usize::from(snapshot.visits);
        if snapshot.schema_version != 1
            || snapshot.task_id != scope.task_id
            || snapshot.policy_sha256 != scope.policy_sha256
            || snapshot.started_epoch_ms == 0
            || snapshot.last_epoch_ms < snapshot.started_epoch_ms
            || snapshot.last_epoch_ms >= deadline
            || snapshot.queries > scope.limits.queries
            || snapshot.visits > scope.limits.visits
            || snapshot.reserved_bytes > scope.limits.downloaded_bytes
            || snapshot.reserved_bytes < count as u64
            || (count == 0 && snapshot.reserved_bytes != 0)
            || (scope.network == ResearchNetworkMode::Offline && count != 0)
            || snapshot.operation_ids.len() != count
            || snapshot.operation_ids.iter().any(|id| !valid_id(id))
            || snapshot
                .operation_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(ResearchBudgetError::Invalid);
        }
        Ok(Self {
            schema_version: snapshot.schema_version,
            task_id: snapshot.task_id,
            policy_sha256: snapshot.policy_sha256,
            started_epoch_ms: snapshot.started_epoch_ms,
            last_epoch_ms: snapshot.last_epoch_ms,
            queries: snapshot.queries,
            visits: snapshot.visits,
            reserved_bytes: snapshot.reserved_bytes,
            operation_ids: snapshot.operation_ids.into_iter().collect(),
            cancelled: snapshot.cancelled,
            deadline_exhausted: snapshot.deadline_exhausted,
        })
    }

    /// Verifies an append-only accounting transition, not an execution result.
    /// Both states must first pass `restore` against the same original scope.
    pub(crate) fn check_successor(&self, next: &Self) -> Result<(), ResearchBudgetError> {
        let count = self.operation_ids.len();
        let next_count = next.operation_ids.len();
        if self.task_id != next.task_id
            || self.policy_sha256 != next.policy_sha256
            || self.started_epoch_ms != next.started_epoch_ms
            || self.last_epoch_ms > next.last_epoch_ms
            || self.queries > next.queries
            || self.visits > next.visits
            || self.reserved_bytes > next.reserved_bytes
            || !self.operation_ids.is_subset(&next.operation_ids)
            || (self.cancelled && !next.cancelled)
            || (self.deadline_exhausted && !next.deadline_exhausted)
            || next_count > count + 1
            || ((self.cancelled || self.deadline_exhausted) && next_count != count)
            || (next_count == count && self.reserved_bytes != next.reserved_bytes)
            || (next_count > count && self.reserved_bytes == next.reserved_bytes)
        {
            return Err(ResearchBudgetError::Binding);
        }
        Ok(())
    }

    /// Opens fresh task accounting without dispatching anything.
    pub fn new(scope: &ResearchScope, started_epoch_ms: u64) -> Result<Self, ResearchBudgetError> {
        if started_epoch_ms == 0
            || started_epoch_ms
                .checked_add(scope.limits.elapsed_ms)
                .is_none()
        {
            return Err(ResearchBudgetError::Invalid);
        }
        Ok(Self {
            schema_version: 1,
            task_id: scope.task_id.clone(),
            policy_sha256: scope.policy_sha256.clone(),
            started_epoch_ms,
            last_epoch_ms: started_epoch_ms,
            queries: 0,
            visits: 0,
            reserved_bytes: 0,
            operation_ids: BTreeSet::new(),
            cancelled: false,
            deadline_exhausted: false,
        })
    }

    // A trusted clock observation may restrict an already spent attempt without
    // reserving a second operation, refunding bytes or resetting its original start.
    pub(crate) fn observe_clock(
        &mut self,
        scope: &ResearchScope,
        now_epoch_ms: u64,
    ) -> Result<(), ResearchBudgetError> {
        if self.cancelled {
            return Err(ResearchBudgetError::Cancelled);
        }
        if self.task_id != scope.task_id || self.policy_sha256 != scope.policy_sha256 {
            return Err(ResearchBudgetError::Binding);
        }
        scope.network_requirement()?;
        if self.deadline_exhausted
            || now_epoch_ms < self.last_epoch_ms
            || now_epoch_ms.saturating_sub(self.started_epoch_ms) >= scope.limits.elapsed_ms
        {
            self.deadline_exhausted = true;
            return Err(ResearchBudgetError::Exhausted);
        }
        self.last_epoch_ms = now_epoch_ms;
        Ok(())
    }

    /// Atomically reserves worst-case bytes before the existing owner dispatches.
    /// Failed/rejected/uncertain effects do not refund or erase this reservation.
    /// Trusted clock observations advance even on a quota refusal; observed expiry
    /// and clock rollback are terminal, not failures from which time can be reset.
    pub(crate) fn reserve(
        &mut self,
        scope: &ResearchScope,
        operation_id: &str,
        operation: ResearchOperation<'_>,
        max_response_bytes: u64,
        now_epoch_ms: u64,
    ) -> Result<(), ResearchBudgetError> {
        if self.cancelled {
            return Err(ResearchBudgetError::Cancelled);
        }
        if self.task_id != scope.task_id
            || self.policy_sha256 != scope.policy_sha256
            || !valid_id(operation_id)
            || self.operation_ids.contains(operation_id)
        {
            return Err(ResearchBudgetError::Binding);
        }
        self.observe_clock(scope, now_epoch_ms)?;
        let (queries, visits) = match operation {
            ResearchOperation::Query(query) => {
                if !scope.query_sha256.contains(&public_query_sha256(query)?) {
                    return Err(ResearchBudgetError::Binding);
                }
                (self.queries.checked_add(1), Some(self.visits))
            }
            ResearchOperation::Visit => (Some(self.queries), self.visits.checked_add(1)),
        };
        let next = queries
            .zip(visits)
            .zip(self.reserved_bytes.checked_add(max_response_bytes))
            .filter(|((queries, visits), bytes)| {
                max_response_bytes > 0
                    && *queries <= scope.limits.queries
                    && *visits <= scope.limits.visits
                    && *bytes <= scope.limits.downloaded_bytes
            })
            .ok_or(ResearchBudgetError::Exhausted)?;
        ((self.queries, self.visits), self.reserved_bytes) = next;
        self.operation_ids.insert(operation_id.to_owned());
        Ok(())
    }

    /// Records independent cancellation without refunding an in-flight operation.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

/// Content-free identity of an explicitly disclosed non-secret public query.
pub fn public_query_sha256(query: &str) -> Result<String, ResearchBudgetError> {
    if query.trim().is_empty() || query.len() > 2048 || query.chars().any(char::is_control) {
        return Err(ResearchBudgetError::Invalid);
    }
    if !detect_secret_classes("public_query", query.as_bytes()).is_empty() {
        return Err(ResearchBudgetError::Secret);
    }
    Ok(digest(query.as_bytes()))
}

/// Conservative ASCII DNS-name syntax, not evidence that resolution is public.
#[must_use]
pub fn public_dns_name(host: &str) -> bool {
    host.len() <= 253
        && host.contains('.')
        && host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'.'))
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
        && host.rsplit('.').next().is_some_and(|label| {
            label.len() >= 2
                && label.bytes().all(|b| b.is_ascii_lowercase())
                && !matches!(
                    label,
                    "localhost"
                        | "local"
                        | "internal"
                        | "invalid"
                        | "test"
                        | "onion"
                        | "arpa"
                        | "alt"
                        | "home"
                        | "lan"
                        | "example"
                )
        })
}

/// Conservative public-unicast classification; uncertain/special ranges fail closed.
#[must_use]
pub fn public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let [a, b, c, _] = address.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && (b == 0 || b == 168 || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(address) => {
            let words = address.segments();
            (0x2000..=0x3fff).contains(&words[0])
                && !(words[0] == 0x2001 && (words[1] < 0x200 || words[1] == 0xdb8))
                && words[0] != 0x2002
                && !(words[0] == 0x3fff && words[1] < 0x1000)
        }
    }
}

fn valid_id(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_scope_retains_exact_restrictions_without_raw_disclosures() {
        for mode in [
            ResearchNetworkMode::Offline,
            ResearchNetworkMode::Ask,
            ResearchNetworkMode::TaskAuthorized,
        ] {
            let original = scope(mode);
            let bytes = original.snapshot().unwrap();
            assert!(
                !std::str::from_utf8(&bytes)
                    .unwrap()
                    .contains("public Rust documentation")
            );
            assert_eq!(ResearchScope::restore(&bytes).unwrap(), original);
            let original_wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            for (key, value) in [
                ("schema_version", serde_json::json!(2)),
                ("task_id", serde_json::json!("another-task")),
                ("depth", serde_json::json!("deep")),
                ("domains", serde_json::json!(["other.example.com"])),
                (
                    "domains",
                    serde_json::json!(["docs.example.com", "docs.example.com"]),
                ),
                ("query_sha256", serde_json::json!(["A".repeat(64)])),
                ("query_sha256", serde_json::json!(["1".repeat(64)])),
                (
                    "query_sha256",
                    serde_json::json!(["1".repeat(64), "1".repeat(64)]),
                ),
                ("policy_sha256", serde_json::json!("f".repeat(64))),
                ("approval", serde_json::json!(true)),
            ] {
                let mut changed = original_wire.clone();
                changed[key] = value;
                assert!(
                    ResearchScope::restore(&serde_json::to_vec(&changed).unwrap()).is_err(),
                    "{key}"
                );
            }
            let mut changed = original_wire;
            changed["limits"]["visits"] = serde_json::json!(4);
            assert!(ResearchScope::restore(&serde_json::to_vec(&changed).unwrap()).is_err());
            let duplicate =
                std::str::from_utf8(&bytes)
                    .unwrap()
                    .replacen('{', "{\"schema_version\":1,", 1);
            assert!(ResearchScope::restore(duplicate.as_bytes()).is_err());
        }
        assert!(ResearchScope::restore(&vec![b' '; 16385]).is_err());
    }

    fn restore(scope: &ResearchScope, budget: &ResearchBudget) -> ResearchBudget {
        ResearchBudget::restore(scope, &serde_json::to_vec(budget).unwrap()).unwrap()
    }

    #[test]
    fn recovery_preserves_failed_quota_clock_and_consumed_attempts() {
        let scope = scope(ResearchNetworkMode::Ask);
        let mut original = ResearchBudget::new(&scope, 100).unwrap();
        original
            .reserve(&scope, "op-1", ResearchOperation::Visit, 100, 101)
            .unwrap();
        assert_eq!(
            original.reserve(&scope, "op-2", ResearchOperation::Visit, u64::MAX, 200),
            Err(ResearchBudgetError::Exhausted)
        );
        let mut recovered = restore(&scope, &original);
        assert_eq!(recovered.last_epoch_ms, 200);
        assert_eq!(recovered.reserved_bytes, 100);
        assert_eq!(
            recovered.reserve(&scope, "op-1", ResearchOperation::Visit, 1, 200),
            Err(ResearchBudgetError::Binding)
        );
        assert_eq!(
            recovered.reserve(&scope, "op-2", ResearchOperation::Visit, 1, 199),
            Err(ResearchBudgetError::Exhausted)
        );
        assert!(recovered.deadline_exhausted);
    }

    #[test]
    fn recovered_cancellation_and_expiry_never_reopen() {
        let scope = scope(ResearchNetworkMode::Ask);
        for cancel in [false, true] {
            let mut budget = ResearchBudget::new(&scope, 100).unwrap();
            budget
                .reserve(&scope, "op-1", ResearchOperation::Visit, 100, 101)
                .unwrap();
            if cancel {
                budget.cancel();
            } else {
                assert_eq!(
                    budget.reserve(&scope, "op-2", ResearchOperation::Visit, 1, 60_100),
                    Err(ResearchBudgetError::Exhausted)
                );
            }
            let mut recovered = restore(&scope, &budget);
            assert_eq!(
                recovered.reserve(&scope, "op-2", ResearchOperation::Visit, 1, 102),
                Err(if cancel {
                    ResearchBudgetError::Cancelled
                } else {
                    ResearchBudgetError::Exhausted
                })
            );
            assert_eq!(recovered.reserved_bytes, 100);
        }
        let budget = ResearchBudget::new(&scope, 100).unwrap();
        let mut recovered = restore(&scope, &budget);
        assert_eq!(
            recovered.reserve(&scope, "op-1", ResearchOperation::Visit, 1, 60_100),
            Err(ResearchBudgetError::Exhausted)
        );
    }

    #[test]
    fn snapshot_schema_binding_counters_and_complete_identity_set_are_closed() {
        let scope = scope(ResearchNetworkMode::Ask);
        let mut budget = ResearchBudget::new(&scope, 100).unwrap();
        budget
            .reserve(&scope, "op-1", ResearchOperation::Visit, 10, 101)
            .unwrap();
        let original = serde_json::to_value(&budget).unwrap();
        for (key, value) in [
            ("schema_version", serde_json::json!(2)),
            ("task_id", serde_json::json!("another-task")),
            ("policy_sha256", serde_json::json!("f".repeat(64))),
            ("started_epoch_ms", serde_json::json!(0)),
            ("started_epoch_ms", serde_json::json!(u64::MAX)),
            ("last_epoch_ms", serde_json::json!(99)),
            ("last_epoch_ms", serde_json::json!(60_100)),
            ("queries", serde_json::json!(2)),
            ("visits", serde_json::json!(6)),
            ("reserved_bytes", serde_json::json!(0)),
            ("reserved_bytes", serde_json::json!(u64::MAX)),
            ("operation_ids", serde_json::json!([])),
            ("operation_ids", serde_json::json!(["op-1", "op-1"])),
            ("operation_ids", serde_json::json!(["../bad-id"])),
            ("approve", serde_json::json!(true)),
        ] {
            let mut value_copy = original.clone();
            value_copy[key] = value;
            assert!(
                ResearchBudget::restore(&scope, &serde_json::to_vec(&value_copy).unwrap()).is_err(),
                "{key}"
            );
        }
        for key in original.as_object().unwrap().keys() {
            let mut value = original.clone();
            value.as_object_mut().unwrap().remove(key);
            assert!(
                ResearchBudget::restore(&scope, &serde_json::to_vec(&value).unwrap()).is_err(),
                "{key}"
            );
        }
        let bytes = serde_json::to_string(&budget).unwrap();
        let duplicated = bytes.replacen('{', "{\"schema_version\":1,", 1);
        for bytes in [vec![], vec![b' '; 8193], duplicated.into_bytes()] {
            assert!(ResearchBudget::restore(&scope, &bytes).is_err());
        }
        budget
            .reserve(&scope, "op-2", ResearchOperation::Visit, 10, 102)
            .unwrap();
        let mut unsorted = serde_json::to_value(&budget).unwrap();
        unsorted["operation_ids"] = serde_json::json!(["op-2", "op-1"]);
        assert!(ResearchBudget::restore(&scope, &serde_json::to_vec(&unsorted).unwrap()).is_err());
    }

    #[test]
    fn snapshot_rejects_reserved_bytes_without_an_operation_and_offline_consumption() {
        let offline = scope(ResearchNetworkMode::Offline);
        let mut value = serde_json::to_value(ResearchBudget::new(&offline, 100).unwrap()).unwrap();
        value["reserved_bytes"] = serde_json::json!(1);
        assert!(ResearchBudget::restore(&offline, &serde_json::to_vec(&value).unwrap()).is_err());
        value["visits"] = serde_json::json!(1);
        value["operation_ids"] = serde_json::json!(["op-1"]);
        assert!(ResearchBudget::restore(&offline, &serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn successor_refuses_refunds_reset_replaced_ids_and_cleared_terminal_flags() {
        let scope = scope(ResearchNetworkMode::Ask);
        let initial = ResearchBudget::new(&scope, 100).unwrap();
        let mut reserved = restore(&scope, &initial);
        reserved
            .reserve(&scope, "op-1", ResearchOperation::Visit, 100, 101)
            .unwrap();
        assert!(initial.check_successor(&reserved).is_ok());
        assert!(reserved.check_successor(&initial).is_err());
        let mut later = restore(&scope, &reserved);
        later
            .reserve(&scope, "op-2", ResearchOperation::Visit, 100, 102)
            .unwrap();
        assert!(reserved.check_successor(&later).is_ok());
        assert!(initial.check_successor(&later).is_err());
        for (key, value) in [
            ("started_epoch_ms", serde_json::json!(101)),
            ("last_epoch_ms", serde_json::json!(100)),
            ("reserved_bytes", serde_json::json!(99)),
            ("reserved_bytes", serde_json::json!(101)),
            ("operation_ids", serde_json::json!(["op-changed"])),
        ] {
            let mut changed = serde_json::to_value(&reserved).unwrap();
            changed[key] = value;
            let recovered =
                ResearchBudget::restore(&scope, &serde_json::to_vec(&changed).unwrap()).unwrap();
            assert!(reserved.check_successor(&recovered).is_err(), "{key}");
        }
        let mut terminal = restore(&scope, &reserved);
        terminal.cancel();
        assert!(reserved.check_successor(&terminal).is_ok());
        assert!(terminal.check_successor(&reserved).is_err());
        assert!(terminal.check_successor(&later).is_err());
        terminal.cancelled = false;
        terminal.deadline_exhausted = true;
        assert!(terminal.check_successor(&reserved).is_err());
        later.deadline_exhausted = true;
        assert!(terminal.check_successor(&later).is_err());
    }

    fn scope(mode: ResearchNetworkMode) -> ResearchScope {
        ResearchScope::new(
            "task-1".into(),
            ResearchDepth::Quick,
            mode,
            ResearchLimits::ceiling(ResearchDepth::Quick),
            BTreeSet::from(["docs.example.com".into()]),
            &["public Rust documentation".into()],
        )
        .unwrap()
    }
    fn endpoint() -> PublicSearchEndpoint {
        PublicSearchEndpoint {
            domain: "search.example.com".into(),
            path: "/search".into(),
            query_field: "q".into(),
            fixed_fields: vec![("format".into(), "json".into())],
        }
    }
    fn endpoint_scope() -> ResearchScope {
        ResearchScope::new(
            "task-1".into(),
            ResearchDepth::Quick,
            ResearchNetworkMode::Ask,
            ResearchLimits::ceiling(ResearchDepth::Quick),
            BTreeSet::from(["docs.example.com".into(), "search.example.com".into()]),
            &["public Rust documentation".into()],
        )
        .unwrap()
        .with_search_endpoint(endpoint())
        .unwrap()
    }
    fn target(domain: &str, path: &str, query: &[(&str, &str)]) -> PublicGetTarget {
        PublicGetTarget {
            domain: domain.into(),
            path: path.into(),
            query: query
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }
    #[test]
    fn schema_two_scope_snapshot_binds_the_exact_search_endpoint() {
        let scope = endpoint_scope();
        let bytes = scope.snapshot().unwrap();
        let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(wire["schema_version"], 2);
        assert_eq!(wire["search_endpoint"]["path"], "/search");
        assert_eq!(ResearchScope::restore(&bytes).unwrap(), scope);
        assert!(!format!("{scope:?}").contains("/search"));
        for (key, value) in [
            ("schema_version", serde_json::json!(1)),
            ("search_endpoint", serde_json::Value::Null),
            ("domains", serde_json::json!(["docs.example.com"])),
        ] {
            let mut changed = wire.clone();
            changed[key] = value;
            assert!(
                ResearchScope::restore(&serde_json::to_vec(&changed).unwrap()).is_err(),
                "{key}"
            );
        }
        let mut changed = wire.clone();
        changed["search_endpoint"]["query_field"] = serde_json::json!("query");
        assert_eq!(
            ResearchScope::restore(&serde_json::to_vec(&changed).unwrap()),
            Err(ResearchBudgetError::Binding)
        );
        let mut changed = wire;
        changed.as_object_mut().unwrap().remove("search_endpoint");
        assert!(ResearchScope::restore(&serde_json::to_vec(&changed).unwrap()).is_err());
        assert_eq!(
            endpoint_scope().with_search_endpoint(endpoint()),
            Err(ResearchBudgetError::Invalid)
        );
        assert_eq!(
            docs_only_scope().with_search_endpoint(endpoint()),
            Err(ResearchBudgetError::Destination)
        );
        let v1 = docs_only_scope();
        let v1_wire: serde_json::Value = serde_json::from_slice(&v1.snapshot().unwrap()).unwrap();
        assert_eq!(v1_wire["schema_version"], 1);
        assert!(v1_wire.get("search_endpoint").is_none());
    }
    fn docs_only_scope() -> ResearchScope {
        scope(ResearchNetworkMode::Ask)
    }
    #[test]
    fn classification_derives_a_query_only_from_the_exact_endpoint_shape() {
        let scope = endpoint_scope();
        let query = [("format", "json"), ("q", "public Rust documentation")];
        assert!(matches!(
            scope.classify(&target("search.example.com", "/search", &query)),
            Ok(ResearchOperation::Query("public Rust documentation"))
        ));
        // Field order is not part of the disclosed shape; names are unique.
        assert!(matches!(
            scope.classify(&target(
                "search.example.com",
                "/search",
                &[query[1], query[0]]
            )),
            Ok(ResearchOperation::Query(_))
        ));
        assert!(matches!(
            scope.classify(&target("docs.example.com", "/search", &query)),
            Ok(ResearchOperation::Visit)
        ));
        for refused in [
            target("search.example.com", "/results", &query),
            target("search.example.com", "/search", &query[1..]),
            target(
                "search.example.com",
                "/search",
                &[("format", "html"), query[1]],
            ),
            target(
                "search.example.com",
                "/search",
                &[query[0], query[1], ("page", "2")],
            ),
            target("search.example.com", "/search", &[query[0], ("page", "2")]),
            target("search.example.com", "/", &[]),
        ] {
            assert!(matches!(
                scope.classify(&refused),
                Err(ResearchBudgetError::Destination)
            ));
        }
        // The derived query must still be one of the disclosed digests.
        let mut budget = ResearchBudget::new(&scope, 100).unwrap();
        let undisclosed = target("search.example.com", "/search", &[query[0], ("q", "other")]);
        let operation = scope.classify(&undisclosed).unwrap();
        assert_eq!(
            budget.reserve(&scope, "op-1", operation, 1, 101),
            Err(ResearchBudgetError::Binding)
        );
        let legacy = docs_only_scope();
        assert!(matches!(
            legacy.classify(&target("docs.example.com", "/guide", &[])),
            Err(ResearchBudgetError::Invalid)
        ));
    }
    #[test]
    fn offline_and_changed_disclosure_never_consume_budget() {
        let offline = scope(ResearchNetworkMode::Offline);
        let mut budget = ResearchBudget::new(&offline, 100).unwrap();
        assert_eq!(
            budget.reserve(&offline, "op-1", ResearchOperation::Visit, 1, 100),
            Err(ResearchBudgetError::Offline)
        );
        assert!(budget.operation_ids.is_empty());
        let allowed = scope(ResearchNetworkMode::Ask);
        let mut budget = ResearchBudget::new(&allowed, 100).unwrap();
        assert_eq!(
            budget.reserve(
                &allowed,
                "op-1",
                ResearchOperation::Query("undisclosed input"),
                1,
                100
            ),
            Err(ResearchBudgetError::Binding)
        );
        assert!(budget.operation_ids.is_empty());
    }
    #[test]
    fn secret_canaries_are_refused_before_hash_or_disclosure() {
        for query in [
            format!("lookup Bearer {}", "q".repeat(32)),
            format!("lookup ghp_{}", "a".repeat(32)),
            "https://user:password@example.com".into(),
            concat!("-----BEGIN ", "PRIVATE KEY-----").into(),
        ] {
            assert_eq!(
                public_query_sha256(&query),
                Err(ResearchBudgetError::Secret)
            );
        }
    }
    #[test]
    fn exact_domain_all_dns_answers_proxy_and_port_are_enforced() {
        let scope = scope(ResearchNetworkMode::TaskAuthorized);
        let public = "93.184.216.34".parse().unwrap();
        assert!(
            scope
                .check_destination("docs.example.com", 443, &[public], false)
                .is_ok()
        );
        for address in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "203.0.113.1",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "2002:7f00:1::1",
            "3fff::1",
        ] {
            assert_eq!(
                scope.check_destination(
                    "docs.example.com",
                    443,
                    &[public, address.parse().unwrap()],
                    false
                ),
                Err(ResearchBudgetError::Destination),
                "{address}"
            );
        }
        assert!(
            scope
                .check_destination("evil.docs.example.com", 443, &[public], false)
                .is_err()
        );
        assert!(
            scope
                .check_destination("docs.example.com", 443, &[], false)
                .is_err()
        );
        assert!(
            scope
                .check_destination("docs.example.com", 443, &[public; 17], false)
                .is_err()
        );
        assert!(
            scope
                .check_destination("docs.example.com", 80, &[public], false)
                .is_err()
        );
        assert!(
            scope
                .check_destination("docs.example.com", 443, &[public], true)
                .is_err()
        );
        assert!(public_address("2606:4700:4700::1111".parse().unwrap()));
        assert!(
            scope
                .check_redirect(3, "docs.example.com", 443, &[public], false)
                .is_ok()
        );
        assert_eq!(
            scope.check_redirect(4, "docs.example.com", 443, &[public], false),
            Err(ResearchBudgetError::Exhausted)
        );
        assert!(
            scope
                .check_redirect(1, "other.example.com", 443, &[public], false)
                .is_err()
        );
    }
    #[test]
    fn atomic_reservations_failures_replay_and_cancellation() {
        let scope = scope(ResearchNetworkMode::Ask);
        let mut budget = ResearchBudget::new(&scope, 100).unwrap();
        budget
            .reserve(
                &scope,
                "op-1",
                ResearchOperation::Query("public Rust documentation"),
                10,
                101,
            )
            .unwrap();
        let before = serde_json::to_value(&budget).unwrap();
        assert_eq!(
            budget.reserve(&scope, "op-1", ResearchOperation::Visit, 10, 102),
            Err(ResearchBudgetError::Binding)
        );
        assert_eq!(
            budget.reserve(
                &scope,
                "op-2",
                ResearchOperation::Query("public Rust documentation"),
                10,
                102
            ),
            Err(ResearchBudgetError::Exhausted)
        );
        assert_eq!(
            budget.reserve(&scope, "op-2", ResearchOperation::Visit, u64::MAX, 102),
            Err(ResearchBudgetError::Exhausted)
        );
        let mut after = serde_json::to_value(&budget).unwrap();
        assert_eq!(after["last_epoch_ms"], 102);
        after["last_epoch_ms"] = before["last_epoch_ms"].clone();
        assert_eq!(before, after);
        budget.cancel();
        assert_eq!(
            budget.reserve(&scope, "op-2", ResearchOperation::Visit, 1, 103),
            Err(ResearchBudgetError::Cancelled)
        );
        assert_eq!(budget.reserved_bytes, 10);
    }
    #[test]
    fn time_and_policy_drift_are_not_fresh_budgets() {
        let scope = scope(ResearchNetworkMode::Ask);
        let mut budget = ResearchBudget::new(&scope, 100).unwrap();
        assert!(
            budget
                .reserve(&scope, "op-1", ResearchOperation::Visit, 1, 99)
                .is_err()
        );
        assert!(
            budget
                .reserve(&scope, "op-1", ResearchOperation::Visit, 1, 60_100)
                .is_err()
        );
        let mut changed = scope.clone();
        changed.limits.visits -= 1;
        changed.policy_sha256 = "changed".into();
        assert_eq!(
            budget.reserve(&changed, "op-1", ResearchOperation::Visit, 1, 101),
            Err(ResearchBudgetError::Binding)
        );
    }

    #[test]
    fn observed_expiry_and_clock_rollback_cannot_be_revived() {
        let scope = scope(ResearchNetworkMode::Ask);
        for rejected_time in [99, 60_100] {
            let mut budget = ResearchBudget::new(&scope, 100).unwrap();
            assert_eq!(
                budget.reserve(&scope, "op-1", ResearchOperation::Visit, 1, rejected_time),
                Err(ResearchBudgetError::Exhausted)
            );
            assert_eq!(
                budget.reserve(&scope, "op-2", ResearchOperation::Visit, 1, 101),
                Err(ResearchBudgetError::Exhausted)
            );
            assert!(budget.operation_ids.is_empty());
            assert_eq!(budget.reserved_bytes, 0);
        }
        let mut budget = ResearchBudget::new(&scope, 100).unwrap();
        assert_eq!(
            budget.reserve(&scope, "op-1", ResearchOperation::Visit, u64::MAX, 200),
            Err(ResearchBudgetError::Exhausted)
        );
        assert_eq!(
            budget.reserve(&scope, "op-2", ResearchOperation::Visit, 1, 150),
            Err(ResearchBudgetError::Exhausted)
        );
        assert_eq!(
            budget.reserve(&scope, "op-3", ResearchOperation::Visit, 1, 201),
            Err(ResearchBudgetError::Exhausted)
        );
    }

    #[test]
    fn exact_byte_and_visit_ceilings_are_inclusive_without_refunds() {
        let scope = scope(ResearchNetworkMode::Ask);
        let mut bytes = ResearchBudget::new(&scope, 100).unwrap();
        bytes
            .reserve(
                &scope,
                "op-1",
                ResearchOperation::Visit,
                scope.limits.downloaded_bytes,
                101,
            )
            .unwrap();
        assert_eq!(
            bytes.reserve(&scope, "op-2", ResearchOperation::Visit, 1, 102),
            Err(ResearchBudgetError::Exhausted)
        );
        let mut visits = ResearchBudget::new(&scope, 100).unwrap();
        for index in 0..scope.limits.visits {
            visits
                .reserve(
                    &scope,
                    &format!("op-{index}"),
                    ResearchOperation::Visit,
                    1,
                    101,
                )
                .unwrap();
        }
        assert_eq!(
            visits.reserve(&scope, "op-excess", ResearchOperation::Visit, 1, 102),
            Err(ResearchBudgetError::Exhausted)
        );
        assert_eq!(visits.operation_ids.len(), usize::from(scope.limits.visits));
    }
    #[test]
    fn ceilings_invalid_hosts_and_charge_are_closed() {
        let mut limits = ResearchLimits::ceiling(ResearchDepth::Quick);
        limits.provider_charge_microunits = 1;
        assert!(!limits.validate(ResearchDepth::Quick));
        assert!(!ResearchLimits::ceiling(ResearchDepth::Deep).validate(ResearchDepth::Quick));
        for host in [
            "localhost",
            "127.0.0.1",
            "foo.local",
            "foo..com",
            "-foo.com",
            "foo.com.",
            "foo:443",
            "foo.com@evil.com",
            "foo%2ecom",
            "EXAMPLE.com",
        ] {
            assert!(!public_dns_name(host), "{host}");
        }
    }
}
