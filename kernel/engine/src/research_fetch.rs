//! Bounded public GET worker packets, not network grants or retrieval evidence.
//!
//! The existing owner must independently approve exact disclosure, persist a budget
//! reservation and consume an effect permit before dispatch. This module has no I/O.
//! The future worker still needs established URL/TLS parsing, complete DNS validation,
//! pinned connections, response validation and owned process cleanup.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::persistence::detect_secret_classes;
use crate::research_budget::{ResearchDepth, ResearchLimits, ResearchScope, public_dns_name};

const MAX_PACKET_BYTES: usize = 16 * 1024;

/// Exact public request data for disclosure. No arbitrary headers or method exist.
/// Query values are unencoded data; only the transport's URL library may encode them.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicGetTarget {
    /// Exact canonical ASCII DNS name; the transport independently resolves it.
    pub domain: String,
    /// Conservative unencoded absolute path, not a general URL parser.
    pub path: String,
    /// Ordered exact query fields. Duplicate names and detected secrets are refused.
    pub query: Vec<(String, String)>,
}

impl PublicGetTarget {
    /// Rechecks the closed disclosure syntax and secret ceiling without I/O.
    /// A transport must also parse URLs, validate DNS/peers/TLS and consume authority.
    pub fn validate(&self) -> Result<(), ResearchFetchError> {
        validate_target(self)
    }
}

/// Descriptive per-operation limits, not task accounting or an approval.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicGetDraft {
    /// Only schema 1 is accepted.
    pub schema_version: u16,
    /// Existing runtime operation identity used for no-replay accounting.
    pub operation_id: String,
    /// Exact proposed disclosure target.
    pub target: PublicGetTarget,
    /// Worst-case body bytes reserved before dispatch, including redirect bodies.
    pub maximum_response_bytes: u64,
    /// Same-origin hop ceiling, including the valid zero-redirect case.
    pub redirect_limit: u8,
    /// Downward-only attempt duration inside the original whole-task deadline.
    pub timeout_ms: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    schema_version: u16,
    task_id: String,
    policy_sha256: String,
    prepared_at_epoch_ms: u64,
    deadline_epoch_ms: u64,
    request: PublicGetDraft,
}

/// Closed content-free refusal before any network execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchFetchError {
    /// Unknown schema, invalid identity, unsupported target or malformed envelope.
    Invalid,
    /// Detected secret material must not enter a worker disclosure packet.
    Secret,
    /// Offline, undeclared destination or widened resource restriction.
    Scope,
    /// Expiry, clock rollback or duration outside the original task lifetime.
    Deadline,
}

impl ResearchFetchError {
    /// Stable diagnostic without target, query, path or payload content.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Invalid => "research.fetch.input-invalid",
            Self::Secret => "research.fetch.secret-denied",
            Self::Scope => "research.fetch.scope-denied",
            Self::Deadline => "research.fetch.deadline",
        }
    }
}

/// Validated immutable descriptive packet, deliberately not deserializable as authority.
pub struct PreparedPublicGet {
    packet: PublicGetWorkerPacket,
}

/// Syntactically valid consumer envelope, not proof of the parent's scope checks.
/// It cannot be converted into `PreparedPublicGet` or a consumed effect permit.
///
/// ```compile_fail
/// use agentmage_kernel_engine::research_fetch::{PreparedPublicGet, PublicGetWorkerPacket};
/// fn bypass(packet: PublicGetWorkerPacket) -> PreparedPublicGet { packet }
/// ```
pub struct PublicGetWorkerPacket {
    wire: WireRequest,
    bytes: Vec<u8>,
    sha256: String,
}

impl fmt::Debug for PublicGetWorkerPacket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublicGetWorkerPacket")
            .field("packet_sha256", &self.sha256)
            .field("packet_bytes", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl PreparedPublicGet {
    /// Prepares a disclosure packet without reserving budget, approving it or dispatching.
    /// Times come from the existing trusted task owner, never from retrieved content.
    pub fn prepare(
        scope: &ResearchScope,
        request: PublicGetDraft,
        task_started_epoch_ms: u64,
        now_epoch_ms: u64,
    ) -> Result<Self, ResearchFetchError> {
        validate_draft(&request)?;
        scope
            .check_host(&request.target.domain)
            .map_err(|_| ResearchFetchError::Scope)?;
        if request.maximum_response_bytes > scope.limits().downloaded_bytes
            || request.redirect_limit > scope.limits().redirects
        {
            return Err(ResearchFetchError::Scope);
        }
        let task_deadline = task_started_epoch_ms
            .checked_add(scope.limits().elapsed_ms)
            .ok_or(ResearchFetchError::Deadline)?;
        let deadline = now_epoch_ms
            .checked_add(request.timeout_ms)
            .ok_or(ResearchFetchError::Deadline)?;
        if task_started_epoch_ms == 0
            || now_epoch_ms < task_started_epoch_ms
            || deadline > task_deadline
        {
            return Err(ResearchFetchError::Deadline);
        }
        let wire = WireRequest {
            schema_version: 1,
            task_id: scope.task_id().to_owned(),
            policy_sha256: scope.policy_sha256().to_owned(),
            prepared_at_epoch_ms: now_epoch_ms,
            deadline_epoch_ms: deadline,
            request,
        };
        let bytes = serde_json::to_vec(&wire).map_err(|_| ResearchFetchError::Invalid)?;
        Ok(Self {
            packet: PublicGetWorkerPacket::decode_worker_packet(&bytes, now_epoch_ms)?,
        })
    }

    /// Immutable packet for exact disclosure and effect-detail binding before dispatch.
    #[must_use]
    pub const fn packet(&self) -> &PublicGetWorkerPacket {
        &self.packet
    }
}

impl PublicGetWorkerPacket {
    /// Validates the entire bounded worker envelope, including duplicate/unknown fields.
    /// This proves only packet syntax/restrictions, not grant consumption or model admission.
    /// It is stateless: the existing task budget and authority owner must enforce terminal
    /// expiry, cancellation and no replay; decoding cannot restore those owners' state.
    pub fn decode_worker_packet(
        bytes: &[u8],
        now_epoch_ms: u64,
    ) -> Result<Self, ResearchFetchError> {
        if bytes.is_empty() || bytes.len() > MAX_PACKET_BYTES {
            return Err(ResearchFetchError::Invalid);
        }
        let wire: WireRequest =
            serde_json::from_slice(bytes).map_err(|_| ResearchFetchError::Invalid)?;
        validate_draft(&wire.request)?;
        if wire.schema_version != 1
            || !valid_id(&wire.task_id)
            || wire.policy_sha256.len() != 64
            || !wire
                .policy_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || wire.policy_sha256.bytes().all(|b| b == b'0')
        {
            return Err(ResearchFetchError::Invalid);
        }
        if wire.prepared_at_epoch_ms == 0
            || now_epoch_ms < wire.prepared_at_epoch_ms
            || now_epoch_ms >= wire.deadline_epoch_ms
            || wire
                .prepared_at_epoch_ms
                .checked_add(wire.request.timeout_ms)
                != Some(wire.deadline_epoch_ms)
        {
            return Err(ResearchFetchError::Deadline);
        }
        let sha256 = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self {
            wire,
            bytes: bytes.to_vec(),
            sha256,
        })
    }

    /// Exact bytes to bind in the approved effect details before worker dispatch.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Complete packet identity; not an authorization token.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Task identity which must match the independently consumed effect permit.
    #[must_use]
    pub fn task_id(&self) -> &str {
        &self.wire.task_id
    }

    /// Request data remains inert and immutable after preparation.
    #[must_use]
    pub const fn request(&self) -> &PublicGetDraft {
        &self.wire.request
    }

    /// Exclusive deadline which redirects and restart cannot extend.
    #[must_use]
    pub const fn deadline_epoch_ms(&self) -> u64 {
        self.wire.deadline_epoch_ms
    }

    /// Earliest possible attempt start, established during trusted preparation.
    #[must_use]
    pub const fn prepared_at_epoch_ms(&self) -> u64 {
        self.wire.prepared_at_epoch_ms
    }
}

fn validate_draft(request: &PublicGetDraft) -> Result<(), ResearchFetchError> {
    let ceiling = ResearchLimits::ceiling(ResearchDepth::Deep);
    if request.schema_version != 1
        || !valid_id(&request.operation_id)
        || request.maximum_response_bytes == 0
        || request.maximum_response_bytes > ceiling.downloaded_bytes
        || request.redirect_limit > ceiling.redirects
        || request.timeout_ms == 0
        || request.timeout_ms > ceiling.elapsed_ms
    {
        return Err(ResearchFetchError::Invalid);
    }
    validate_target(&request.target)
}

pub(crate) fn validate_target(target: &PublicGetTarget) -> Result<(), ResearchFetchError> {
    if !public_dns_name(&target.domain)
        || !target.path.starts_with('/')
        || target.path.starts_with("//")
        || target.path.len() > 4096
        || target
            .path
            .split('/')
            .any(|part| matches!(part, "." | ".."))
        || !target
            .path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-._~!$&'()*+,;=:@".contains(&b))
        || target.query.len() > 16
    {
        return Err(ResearchFetchError::Invalid);
    }
    if !detect_secret_classes("public_path", target.path.as_bytes()).is_empty() {
        return Err(ResearchFetchError::Secret);
    }
    let mut names = BTreeSet::new();
    for (name, value) in &target.query {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || !names.insert(name)
            || value.len() > 2048
            || value.chars().any(char::is_control)
        {
            return Err(ResearchFetchError::Invalid);
        }
        if !detect_secret_classes(name, value.as_bytes()).is_empty() {
            return Err(ResearchFetchError::Secret);
        }
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    value.len() <= 128
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_budget::{ResearchDepth, ResearchLimits, ResearchNetworkMode};

    fn scope(mode: ResearchNetworkMode) -> ResearchScope {
        ResearchScope::new(
            "task-1".into(),
            ResearchDepth::Quick,
            mode,
            ResearchLimits::ceiling(ResearchDepth::Quick),
            BTreeSet::from(["docs.example.com".into()]),
            &["public Rust docs".into()],
        )
        .unwrap()
    }
    fn draft() -> PublicGetDraft {
        PublicGetDraft {
            schema_version: 1,
            operation_id: "op-1".into(),
            target: PublicGetTarget {
                domain: "docs.example.com".into(),
                path: "/search".into(),
                query: vec![("q".into(), "public Rust docs".into())],
            },
            maximum_response_bytes: 4096,
            redirect_limit: 0,
            timeout_ms: 1000,
        }
    }
    #[test]
    fn producer_consumer_preserve_complete_packet_and_original_deadline() {
        let prepared =
            PreparedPublicGet::prepare(&scope(ResearchNetworkMode::Ask), draft(), 100, 200)
                .unwrap();
        let prepared = prepared.packet();
        let worker = PublicGetWorkerPacket::decode_worker_packet(prepared.bytes(), 300).unwrap();
        assert_eq!(worker.bytes(), prepared.bytes());
        assert_eq!(worker.sha256(), prepared.sha256());
        assert_eq!(worker.task_id(), "task-1");
        assert_eq!(worker.deadline_epoch_ms(), 1200);
        assert!(!format!("{worker:?}").contains("public Rust docs"));
        for now in [199, 1200, 1201] {
            assert_eq!(
                PublicGetWorkerPacket::decode_worker_packet(prepared.bytes(), now).err(),
                Some(ResearchFetchError::Deadline)
            );
        }
    }
    #[test]
    fn offline_scope_and_downward_limits_are_independent_of_preparation() {
        assert_eq!(
            PreparedPublicGet::prepare(&scope(ResearchNetworkMode::Offline), draft(), 100, 200)
                .err(),
            Some(ResearchFetchError::Scope)
        );
        let allowed = scope(ResearchNetworkMode::Ask);
        let mut changed = draft();
        changed.target.domain = "other.example.com".into();
        assert_eq!(
            PreparedPublicGet::prepare(&allowed, changed, 100, 200).err(),
            Some(ResearchFetchError::Scope)
        );
        let mut changed = draft();
        changed.maximum_response_bytes = 1_048_577;
        assert_eq!(
            PreparedPublicGet::prepare(&allowed, changed, 100, 200).err(),
            Some(ResearchFetchError::Scope)
        );
        for (started, now) in [(0, 100), (100, 99), (100, 59_101), (u64::MAX, u64::MAX)] {
            assert_eq!(
                PreparedPublicGet::prepare(&allowed, draft(), started, now).err(),
                Some(ResearchFetchError::Deadline)
            );
        }
    }
    #[test]
    fn unknown_fields_duplicate_keys_and_unsafe_targets_are_refused() {
        let allowed = scope(ResearchNetworkMode::Ask);
        let prepared = PreparedPublicGet::prepare(&allowed, draft(), 100, 200).unwrap();
        let prepared = prepared.packet();
        let mut value: serde_json::Value = serde_json::from_slice(prepared.bytes()).unwrap();
        value["headers"] = serde_json::json!({"Authorization": "forbidden"});
        assert_eq!(
            PublicGetWorkerPacket::decode_worker_packet(&serde_json::to_vec(&value).unwrap(), 200)
                .err(),
            Some(ResearchFetchError::Invalid)
        );
        let duplicate = String::from_utf8(prepared.bytes().to_vec())
            .unwrap()
            .replacen('{', "{\"schema_version\":1,", 1);
        assert_eq!(
            PublicGetWorkerPacket::decode_worker_packet(duplicate.as_bytes(), 200).err(),
            Some(ResearchFetchError::Invalid)
        );
        for path in [
            "//other.example.com",
            "/a/../private",
            "/a?secret=x",
            "/a#fragment",
            "/%2f",
            "/back\\slash",
            "/raw space",
        ] {
            let mut changed = draft();
            changed.target.path = path.into();
            assert_eq!(
                PreparedPublicGet::prepare(&allowed, changed, 100, 200).err(),
                Some(ResearchFetchError::Invalid)
            );
        }
    }
    #[test]
    fn secret_canaries_duplicate_query_fields_and_packet_pressure_fail_closed() {
        let allowed = scope(ResearchNetworkMode::Ask);
        let mut changed = draft();
        changed.target.query[0].1 = format!("Bearer {}", "x".repeat(32));
        assert_eq!(
            PreparedPublicGet::prepare(&allowed, changed, 100, 200).err(),
            Some(ResearchFetchError::Secret)
        );
        let mut changed = draft();
        changed.target.query.push(changed.target.query[0].clone());
        assert_eq!(
            PreparedPublicGet::prepare(&allowed, changed, 100, 200).err(),
            Some(ResearchFetchError::Invalid)
        );
        let mut changed = draft();
        changed.target.query = (0..16)
            .map(|i| (format!("q{i}"), "public ".repeat(290)))
            .collect();
        assert_eq!(
            PreparedPublicGet::prepare(&allowed, changed, 100, 200).err(),
            Some(ResearchFetchError::Invalid)
        );
        assert_eq!(
            PublicGetWorkerPacket::decode_worker_packet(&vec![b' '; MAX_PACKET_BYTES + 1], 200)
                .err(),
            Some(ResearchFetchError::Invalid)
        );
    }
}
