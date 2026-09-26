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
    network: ResearchNetworkMode,
    limits: ResearchLimits,
    domains: BTreeSet<String>,
    query_sha256: BTreeSet<String>,
    policy_sha256: String,
}

impl ResearchScope {
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
        let encoded = serde_json::to_vec(&(
            1u16,
            &task_id,
            depth,
            network,
            &limits,
            &domains,
            &query_sha256,
        ))
        .map_err(|_| ResearchBudgetError::Invalid)?;
        Ok(Self {
            task_id,
            network,
            limits,
            domains,
            query_sha256,
            policy_sha256: digest(&encoded),
        })
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

impl ResearchBudget {
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

    /// Atomically reserves worst-case bytes before the existing owner dispatches.
    /// Failed/rejected/uncertain effects do not refund or erase this reservation.
    /// Trusted clock observations advance even on a quota refusal; observed expiry
    /// and clock rollback are terminal, not failures from which time can be reset.
    pub fn reserve(
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
        scope.network_requirement()?;
        if self.deadline_exhausted
            || now_epoch_ms < self.last_epoch_ms
            || now_epoch_ms.saturating_sub(self.started_epoch_ms) >= scope.limits.elapsed_ms
        {
            // Expiry or clock rollback is terminal. A subsequent older wall-clock
            // observation must not revive a budget after expiration was observed.
            self.deadline_exhausted = true;
            return Err(ResearchBudgetError::Exhausted);
        }
        self.last_epoch_ms = now_epoch_ms;
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
            "-----BEGIN PRIVATE KEY-----".into(),
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
