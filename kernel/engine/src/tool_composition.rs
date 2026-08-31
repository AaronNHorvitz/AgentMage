//! Deterministic native-tool preflight, attempt, launch, and verification composition.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::{CanonicalEffectClass, GrantOperation, ToolCall, ToolId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::tooling::{ToolAttemptGuard, ToolRegistry};

const VERSION: &str = "1.0.0";
const MAX_AGE_MS: u64 = 60_000;
const MAX_EVIDENCE_BYTES: u64 = 1024 * 1024;
const MAX_IDENTITIES: usize = 256;

/// Closed deterministic preflight families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightProbeKind {
    /// Workspace and held-object facts.
    Workspace,
    /// Repository and worktree facts.
    Git,
    /// Executable identity and digest.
    Executable,
    /// Local service state.
    Service,
    /// Current deterministic network policy.
    NetworkPolicy,
    /// Platform and architecture.
    Platform,
    /// Storage capacity and policy.
    Storage,
    /// Brokered credential-reference presence, never credential values.
    CredentialReference,
    /// CPU, memory, task, and time capacity.
    Resource,
}

impl PreflightProbeKind {
    /// Every registered probe in stable order.
    pub const ALL: [Self; 9] = [
        Self::Workspace,
        Self::Git,
        Self::Executable,
        Self::Service,
        Self::NetworkPolicy,
        Self::Platform,
        Self::Storage,
        Self::CredentialReference,
        Self::Resource,
    ];

    /// Exact identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Workspace => "preflight.workspace",
            Self::Git => "preflight.git",
            Self::Executable => "preflight.executable",
            Self::Service => "preflight.service",
            Self::NetworkPolicy => "preflight.network-policy",
            Self::Platform => "preflight.platform",
            Self::Storage => "preflight.storage",
            Self::CredentialReference => "preflight.credential-reference",
            Self::Resource => "preflight.resource",
        }
    }
}

/// Exact immutable probe definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreflightProbeDefinition {
    /// Exact identity.
    pub probe_id: String,
    /// Exact version.
    pub probe_version: String,
    /// Maximum trusted age.
    pub freshness_ms: u64,
    /// Maximum evidence size.
    pub max_evidence_bytes: u64,
    /// Ambiguity is always denied.
    pub ambiguity_denied: bool,
}

/// Closed registry of supported preflight probes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreflightRegistry {
    definitions: BTreeMap<String, PreflightProbeDefinition>,
}

impl PreflightRegistry {
    /// Builds all nine exact definitions.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            definitions: PreflightProbeKind::ALL
                .into_iter()
                .map(|kind| {
                    (
                        kind.id().to_owned(),
                        PreflightProbeDefinition {
                            probe_id: kind.id().to_owned(),
                            probe_version: VERSION.to_owned(),
                            freshness_ms: MAX_AGE_MS,
                            max_evidence_bytes: MAX_EVIDENCE_BYTES,
                            ambiguity_denied: true,
                        },
                    )
                })
                .collect(),
        }
    }

    /// Stable definitions.
    #[must_use]
    pub fn definitions(&self) -> Vec<&PreflightProbeDefinition> {
        self.definitions.values().collect()
    }

    /// Resolves only an exact identity and version.
    #[must_use]
    pub fn get(&self, id: &str, version: &str) -> Option<&PreflightProbeDefinition> {
        (version == VERSION)
            .then(|| self.definitions.get(id))
            .flatten()
    }
}

impl Default for PreflightRegistry {
    fn default() -> Self {
        Self::standard()
    }
}

/// Terminal preflight state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightOutcome {
    /// Exact current facts are available.
    Passed,
    /// Required facts are missing.
    Missing,
    /// Facts have multiple possible meanings.
    Ambiguous,
    /// Probe exceeded a declared limit.
    OutOfBudget,
    /// Facts disagree internally.
    Inconsistent,
    /// Probe failed.
    Failed,
}

/// Content-free exact preflight observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreflightObservation {
    /// Probe identity.
    pub probe_id: String,
    /// Probe version.
    pub probe_version: String,
    /// Digest of exact call, targets, and preimages.
    pub subject_sha256: String,
    /// Evidence digest.
    pub evidence_sha256: String,
    /// Evidence byte size.
    pub evidence_bytes: u64,
    /// Trusted observation time.
    pub observed_at_epoch_ms: u64,
    /// Exclusive expiry.
    pub valid_until_epoch_ms: u64,
    /// Terminal state.
    pub outcome: PreflightOutcome,
    /// Stable failure code, absent only on pass.
    pub reason_code: Option<String>,
}

/// Approval rule bound before dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolApprovalRule {
    /// Current deterministic policy may admit a read.
    PolicyBoundRead,
    /// Exact current user approval is required.
    ExplicitUserApproval,
}

/// Retry rule bound before dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRetryPolicy {
    /// A new read may follow exact reconciliation.
    FreshReadAfterReconciliation,
    /// A new approval is required after reconciliation.
    NewApprovalAfterReconciliation,
    /// No automatic retry.
    Never,
}

/// Terminal diagnostic policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDiagnosticPolicy {
    /// Disclose only content-free codes and artifact references.
    ContentFreeWithArtifactReferences,
}

/// Complete exact policy mapping for one runtime-visible tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCompositionPolicy {
    /// Tool identity.
    pub tool_id: ToolId,
    /// Tool version.
    pub tool_version: String,
    /// Trusted effect class.
    pub effect_class: CanonicalEffectClass,
    /// Required probe identities in stable order.
    pub required_preflight_ids: Vec<String>,
    /// Approval rule.
    pub approval_rule: ToolApprovalRule,
    /// Deterministic verifier identity.
    pub verifier_id: String,
    /// Retry rule.
    pub retry_policy: ToolRetryPolicy,
    /// Diagnostic policy.
    pub diagnostic_policy: ToolDiagnosticPolicy,
    /// Digest binding the complete policy.
    pub policy_sha256: String,
}

/// Closed mapping for every tool in one common registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCompositionRegistry {
    policies: BTreeMap<(ToolId, String), ToolCompositionPolicy>,
}

impl ToolCompositionRegistry {
    /// Derives one conservative policy for every exact registered tool.
    pub fn for_tools(tools: &ToolRegistry) -> Result<Self, ToolCompositionError> {
        let mut policies = BTreeMap::new();
        for definition in tools.list_tools() {
            let effect_class = tools
                .get_effect_class(&definition.tool_id, &definition.tool_version)
                .ok_or(ToolCompositionError::RegistryMismatch)?;
            let approval_rule = if effect_class == CanonicalEffectClass::ReadOnly {
                ToolApprovalRule::PolicyBoundRead
            } else {
                ToolApprovalRule::ExplicitUserApproval
            };
            let retry_policy = match effect_class {
                CanonicalEffectClass::ReadOnly => ToolRetryPolicy::FreshReadAfterReconciliation,
                CanonicalEffectClass::IdempotentWrite | CanonicalEffectClass::Conditional => {
                    ToolRetryPolicy::NewApprovalAfterReconciliation
                }
                _ => ToolRetryPolicy::Never,
            };
            let mut policy = ToolCompositionPolicy {
                tool_id: definition.tool_id.clone(),
                tool_version: definition.tool_version.clone(),
                effect_class,
                required_preflight_ids: required_preflights(
                    definition.required_grant.operation.operation(),
                    definition.tool_id.as_str(),
                ),
                approval_rule,
                verifier_id: format!("verifier.{}", definition.tool_id.as_str()),
                retry_policy,
                diagnostic_policy: ToolDiagnosticPolicy::ContentFreeWithArtifactReferences,
                policy_sha256: "0".repeat(64),
            };
            policy.policy_sha256 = policy_digest(&policy);
            if policies
                .insert(
                    (policy.tool_id.clone(), policy.tool_version.clone()),
                    policy,
                )
                .is_some()
            {
                return Err(ToolCompositionError::RegistryMismatch);
            }
        }
        Ok(Self { policies })
    }

    /// Resolves one exact mapping.
    #[must_use]
    pub fn get(&self, id: &ToolId, version: &str) -> Option<&ToolCompositionPolicy> {
        self.policies.get(&(id.clone(), version.to_owned()))
    }

    /// Stable complete mappings.
    #[must_use]
    pub fn policies(&self) -> Vec<&ToolCompositionPolicy> {
        self.policies.values().collect()
    }
}

/// Current authority facts checked without consuming authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolAuthorityState {
    /// Current policy admits the exact call.
    pub policy_allowed: bool,
    /// Exact approval is current when required.
    pub approval_current: bool,
    /// Exact grant is current and unconsumed.
    pub grant_current: bool,
}

/// Fully prepared attempt before grant consumption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedToolAttempt {
    call: ToolCall,
    /// Operation-attempt identity.
    pub attempt_id: String,
    /// Composition-policy digest.
    pub policy_sha256: String,
    /// Complete preflight digest.
    pub preflight_sha256: String,
    /// Canonical targets.
    pub targets: Vec<String>,
    /// Canonical preimage digests.
    pub preimage_sha256s: Vec<String>,
    /// Expected effects.
    pub expected_effects: Vec<String>,
    /// Deterministic postconditions.
    pub postconditions: Vec<String>,
    /// Repeated-call guard sequence.
    pub attempt_sequence: u64,
    /// Digest binding every prepared field.
    pub prepared_sha256: String,
}

impl PreparedToolAttempt {
    /// Exact validated call.
    #[must_use]
    pub const fn call(&self) -> &ToolCall {
        &self.call
    }
}

/// Trusted just-in-time facts rechecked immediately before grant consumption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDispatchRevalidation {
    /// Current composition-policy digest.
    pub policy_sha256: String,
    /// Current complete preflight digest.
    pub preflight_sha256: String,
    /// Current policy admission.
    pub policy_allowed: bool,
    /// Current approval admission.
    pub approval_current: bool,
    /// Current exact grant state.
    pub grant_current: bool,
}

impl ToolDispatchRevalidation {
    /// Builds a current revalidation fixture from one still-current prepared attempt.
    #[must_use]
    pub fn current(attempt: &PreparedToolAttempt) -> Self {
        Self {
            policy_sha256: attempt.policy_sha256.clone(),
            preflight_sha256: attempt.preflight_sha256.clone(),
            policy_allowed: true,
            approval_current: true,
            grant_current: true,
        }
    }
}

/// Closed worker disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolWorkerDisposition {
    /// Worker reported success.
    Succeeded,
    /// Worker proved no change was needed.
    NoOp,
    /// Worker reported a partial effect.
    Partial,
    /// Worker was denied.
    Denied,
    /// Worker was cancelled.
    Cancelled,
    /// Worker failed.
    Failed,
    /// Effect truth is uncertain.
    Uncertain,
    /// Launch was blocked after consumption.
    Blocked,
}

/// Cleanup result for owned resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCleanupState {
    /// All owned resources are absent.
    Complete,
    /// Exact residue remains.
    Partial,
    /// Cleanup truth is unknown.
    Unknown,
}

/// Bounded raw worker report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolWorkerReport {
    /// Worker disposition.
    pub disposition: ToolWorkerDisposition,
    /// Terminal receipt identity.
    pub receipt_id: Option<String>,
    /// Complete result digest.
    pub result_sha256: String,
    /// Produced content-addressed artifacts.
    pub artifact_ids: Vec<String>,
    /// Changed-state identities.
    pub changed_state_ids: Vec<String>,
    /// Cleanup result.
    pub cleanup: ToolCleanupState,
}

/// Deterministic verification state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolVerificationState {
    /// Every postcondition passed.
    Passed,
    /// A postcondition failed.
    Failed,
    /// Current truth cannot be established.
    Uncertain,
    /// Verification could not run.
    Blocked,
}

/// Single-use grant-consumption port.
pub trait ToolGrantConsumptionPort {
    /// Atomically consumes the exact grant immediately before launch.
    fn consume_exact(
        &mut self,
        attempt: &PreparedToolAttempt,
    ) -> Result<String, ToolGrantConsumptionError>;
}

/// Content-free failure from just-in-time grant consumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolGrantConsumptionError {
    /// Grant is absent, stale, mismatched, consumed, or otherwise unavailable.
    Denied,
}

/// Single-worker launch port.
pub trait ToolWorkerPort {
    /// Launches at most one worker with the consumed-grant digest.
    fn launch(
        &mut self,
        attempt: &PreparedToolAttempt,
        consumed_grant_sha256: &str,
    ) -> ToolWorkerReport;
}

/// Deterministic verifier port.
pub trait ToolVerificationPort {
    /// Evaluates exact postconditions without model interpretation.
    fn verify(
        &mut self,
        attempt: &PreparedToolAttempt,
        report: &ToolWorkerReport,
    ) -> ToolVerificationState;
}

/// Process-local launch ledger preventing reuse of an admitted prepared attempt.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolExecutionLedger {
    committed_attempt_ids: BTreeSet<String>,
    terminals: BTreeMap<String, ToolTerminalAttemptRecord>,
}

impl ToolExecutionLedger {
    /// Restores verified committed identities and terminal records after restart.
    pub fn restore(
        committed_attempt_ids: Vec<String>,
        terminals: Vec<ToolTerminalAttemptRecord>,
    ) -> Result<Self, ToolCompositionError> {
        if !valid_sorted_ids(&committed_attempt_ids, true) {
            return Err(ToolCompositionError::InvalidCall);
        }
        let committed_attempt_ids = committed_attempt_ids.into_iter().collect::<BTreeSet<_>>();
        let mut terminal_map = BTreeMap::new();
        for terminal in terminals {
            if !terminal.verify()
                || !committed_attempt_ids.contains(&terminal.attempt_id)
                || terminal_map
                    .insert(terminal.attempt_id.clone(), terminal)
                    .is_some()
            {
                return Err(ToolCompositionError::InvalidCall);
            }
        }
        Ok(Self {
            committed_attempt_ids,
            terminals: terminal_map,
        })
    }

    /// Number of unique launch commitments.
    #[must_use]
    pub fn committed_len(&self) -> usize {
        self.committed_attempt_ids.len()
    }

    /// Returns one retained terminal record.
    #[must_use]
    pub fn terminal(&self, attempt_id: &str) -> Option<&ToolTerminalAttemptRecord> {
        self.terminals.get(attempt_id)
    }

    /// Reconciles one committed but nonterminal crash as explicit uncertainty without relaunch.
    pub fn reconcile_uncertain(
        &mut self,
        attempt_id: &str,
        prepared_sha256: &str,
        consumed_grant_sha256: &str,
        cleanup: ToolCleanupState,
    ) -> Result<&ToolTerminalAttemptRecord, ToolCompositionError> {
        if !self.committed_attempt_ids.contains(attempt_id)
            || self.terminals.contains_key(attempt_id)
            || !valid_sha256(prepared_sha256)
            || !valid_sha256(consumed_grant_sha256)
        {
            return Err(ToolCompositionError::RepeatedAttempt);
        }
        let mut terminal = ToolTerminalAttemptRecord {
            attempt_id: attempt_id.to_owned(),
            prepared_sha256: prepared_sha256.to_owned(),
            consumed_grant_sha256: consumed_grant_sha256.to_owned(),
            disposition: ToolWorkerDisposition::Uncertain,
            receipt_id: format!(
                "terminal-receipt:{}",
                &sha256_hex(attempt_id.as_bytes())[..16]
            ),
            worker_receipt_id: None,
            result_sha256: sha256_hex(b"tool-composition-recovered-uncertain"),
            artifact_ids: Vec::new(),
            changed_state_ids: Vec::new(),
            verification: ToolVerificationState::Uncertain,
            cleanup,
            completion_verified: false,
            terminal_sha256: "0".repeat(64),
        };
        terminal.terminal_sha256 = terminal_digest(&terminal);
        self.terminals.insert(attempt_id.to_owned(), terminal);
        Ok(self.terminals.get(attempt_id).expect("inserted terminal"))
    }
}

/// One reconciled terminal record for one launched attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolTerminalAttemptRecord {
    /// Attempt identity.
    pub attempt_id: String,
    /// Prepared-attempt digest.
    pub prepared_sha256: String,
    /// Consumed-grant digest.
    pub consumed_grant_sha256: String,
    /// Worker disposition.
    pub disposition: ToolWorkerDisposition,
    /// Composition-owned terminal receipt identity, present for every launch commitment.
    pub receipt_id: String,
    /// Worker-owned receipt identity when the worker produced one.
    pub worker_receipt_id: Option<String>,
    /// Worker result digest.
    pub result_sha256: String,
    /// Produced artifacts.
    pub artifact_ids: Vec<String>,
    /// Changed states.
    pub changed_state_ids: Vec<String>,
    /// Verification state.
    pub verification: ToolVerificationState,
    /// Cleanup state.
    pub cleanup: ToolCleanupState,
    /// True only for receipted, verified, cleaned success or no-op.
    pub completion_verified: bool,
    /// Digest binding every preceding field.
    pub terminal_sha256: String,
}

impl ToolTerminalAttemptRecord {
    /// Seals trusted adapter measurements into the same terminal contract used by composition.
    #[allow(clippy::too_many_arguments)]
    pub fn seal_observed(
        attempt_id: impl Into<String>,
        prepared_sha256: impl Into<String>,
        consumed_grant_sha256: impl Into<String>,
        disposition: ToolWorkerDisposition,
        worker_receipt_id: Option<String>,
        result_sha256: impl Into<String>,
        artifact_ids: Vec<String>,
        changed_state_ids: Vec<String>,
        verification: ToolVerificationState,
        cleanup: ToolCleanupState,
    ) -> Result<Self, ToolCompositionError> {
        let attempt_id = attempt_id.into();
        let prepared_sha256 = prepared_sha256.into();
        let consumed_grant_sha256 = consumed_grant_sha256.into();
        let result_sha256 = result_sha256.into();
        if !valid_id(&attempt_id)
            || !valid_sha256(&prepared_sha256)
            || !valid_sha256(&consumed_grant_sha256)
            || !valid_sha256(&result_sha256)
            || worker_receipt_id
                .as_deref()
                .is_some_and(|value| !valid_id(value))
            || !valid_sorted_ids(&artifact_ids, true)
            || !valid_sorted_ids(&changed_state_ids, true)
        {
            return Err(ToolCompositionError::InvalidCall);
        }
        let completion_verified = matches!(
            disposition,
            ToolWorkerDisposition::Succeeded | ToolWorkerDisposition::NoOp
        ) && worker_receipt_id.is_some()
            && verification == ToolVerificationState::Passed
            && cleanup == ToolCleanupState::Complete;
        let mut terminal = Self {
            receipt_id: format!(
                "terminal-receipt:{}",
                &sha256_hex(attempt_id.as_bytes())[..16]
            ),
            attempt_id,
            prepared_sha256,
            consumed_grant_sha256,
            disposition,
            worker_receipt_id,
            result_sha256,
            artifact_ids,
            changed_state_ids,
            verification,
            cleanup,
            completion_verified,
            terminal_sha256: "0".repeat(64),
        };
        terminal.terminal_sha256 = terminal_digest(&terminal);
        Ok(terminal)
    }

    /// Verifies digest and no-false-completion invariants.
    #[must_use]
    pub fn verify(&self) -> bool {
        self.terminal_sha256 == terminal_digest(self)
            && valid_id(&self.receipt_id)
            && self.worker_receipt_id.as_deref().is_none_or(valid_id)
            && self.completion_verified
                == (matches!(
                    self.disposition,
                    ToolWorkerDisposition::Succeeded | ToolWorkerDisposition::NoOp
                ) && self.worker_receipt_id.is_some()
                    && self.verification == ToolVerificationState::Passed
                    && self.cleanup == ToolCleanupState::Complete)
    }
}

/// Stable pre-launch refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolCompositionError {
    /// Common tool and policy registries disagree.
    RegistryMismatch,
    /// Call or exact identities are malformed.
    InvalidCall,
    /// Required preflight is not exact and current.
    PreflightDenied,
    /// Policy or approval is absent.
    AuthorityDenied,
    /// Exact grant is absent or stale.
    GrantNotCurrent,
    /// Duplicate or excessive attempt.
    RepeatedAttempt,
    /// Atomic consumption failed before launch.
    GrantConsumptionFailed,
}

impl ToolCompositionError {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RegistryMismatch => "tool.composition.registry-mismatch",
            Self::InvalidCall => "tool.composition.call-invalid",
            Self::PreflightDenied => "tool.composition.preflight-denied",
            Self::AuthorityDenied => "tool.composition.authority-denied",
            Self::GrantNotCurrent => "tool.composition.grant-not-current",
            Self::RepeatedAttempt => "tool.composition.attempt-repeated",
            Self::GrantConsumptionFailed => "tool.composition.grant-consumption-failed",
        }
    }
}

/// Prepares one exact call after preflight and policy, before grant consumption.
#[allow(clippy::too_many_arguments)]
pub fn prepare_tool_attempt(
    tools: &ToolRegistry,
    policies: &ToolCompositionRegistry,
    probes: &PreflightRegistry,
    guard: &mut ToolAttemptGuard,
    call: &ToolCall,
    observations: &[PreflightObservation],
    authority: ToolAuthorityState,
    now_epoch_ms: u64,
    call_depth: u8,
    attempt_id: &str,
    mut targets: Vec<String>,
    mut preimages: Vec<String>,
    mut expected_effects: Vec<String>,
    mut postconditions: Vec<String>,
) -> Result<PreparedToolAttempt, ToolCompositionError> {
    tools
        .validate_arguments(call)
        .map_err(|_| ToolCompositionError::InvalidCall)?;
    let policy = policies
        .get(&call.tool_id, &call.tool_version)
        .ok_or(ToolCompositionError::RegistryMismatch)?;
    if policy.policy_sha256 != policy_digest(policy) || !valid_id(attempt_id) {
        return Err(ToolCompositionError::InvalidCall);
    }
    canonical_ids(&mut targets, false)?;
    canonical_hashes(&mut preimages)?;
    canonical_ids(&mut expected_effects, false)?;
    canonical_ids(&mut postconditions, false)?;
    let subject = sha256_json(&(call, &targets, &preimages));
    let preflight_sha256 =
        validate_preflights(policy, probes, observations, &subject, now_epoch_ms)?;
    if !authority.policy_allowed
        || (policy.approval_rule == ToolApprovalRule::ExplicitUserApproval
            && !authority.approval_current)
    {
        return Err(ToolCompositionError::AuthorityDenied);
    }
    if !authority.grant_current {
        return Err(ToolCompositionError::GrantNotCurrent);
    }
    let record = guard
        .record_attempt(call, call_depth)
        .map_err(|_| ToolCompositionError::RepeatedAttempt)?;
    let mut prepared = PreparedToolAttempt {
        call: call.clone(),
        attempt_id: attempt_id.to_owned(),
        policy_sha256: policy.policy_sha256.clone(),
        preflight_sha256,
        targets,
        preimage_sha256s: preimages,
        expected_effects,
        postconditions,
        attempt_sequence: record.sequence,
        prepared_sha256: "0".repeat(64),
    };
    prepared.prepared_sha256 = prepared_digest(&prepared);
    Ok(prepared)
}

/// Consumes one grant, launches once, verifies, and emits one terminal record.
pub fn execute_prepared_tool_attempt(
    attempt: &PreparedToolAttempt,
    ledger: &mut ToolExecutionLedger,
    current: &ToolDispatchRevalidation,
    grant: &mut impl ToolGrantConsumptionPort,
    worker: &mut impl ToolWorkerPort,
    verifier: &mut impl ToolVerificationPort,
) -> Result<ToolTerminalAttemptRecord, ToolCompositionError> {
    if attempt.prepared_sha256 != prepared_digest(attempt) {
        return Err(ToolCompositionError::InvalidCall);
    }
    if current.policy_sha256 != attempt.policy_sha256
        || current.preflight_sha256 != attempt.preflight_sha256
        || !current.policy_allowed
        || !current.approval_current
    {
        return Err(ToolCompositionError::AuthorityDenied);
    }
    if !current.grant_current {
        return Err(ToolCompositionError::GrantNotCurrent);
    }
    if !ledger
        .committed_attempt_ids
        .insert(attempt.attempt_id.clone())
    {
        return Err(ToolCompositionError::RepeatedAttempt);
    }
    let consumed = grant
        .consume_exact(attempt)
        .map_err(|_| ToolCompositionError::GrantConsumptionFailed)?;
    if !valid_sha256(&consumed) {
        return Err(ToolCompositionError::GrantConsumptionFailed);
    }
    let report = worker.launch(attempt, &consumed);
    let valid_report = valid_sha256(&report.result_sha256)
        && report.receipt_id.as_deref().is_none_or(valid_id)
        && valid_sorted_ids(&report.artifact_ids, true)
        && valid_sorted_ids(&report.changed_state_ids, true);
    let verification = if valid_report {
        verifier.verify(attempt, &report)
    } else {
        ToolVerificationState::Uncertain
    };
    let disposition = if valid_report {
        report.disposition
    } else {
        ToolWorkerDisposition::Uncertain
    };
    let completion_verified = matches!(
        disposition,
        ToolWorkerDisposition::Succeeded | ToolWorkerDisposition::NoOp
    ) && report.receipt_id.is_some()
        && verification == ToolVerificationState::Passed
        && report.cleanup == ToolCleanupState::Complete;
    let mut terminal = ToolTerminalAttemptRecord {
        attempt_id: attempt.attempt_id.clone(),
        prepared_sha256: attempt.prepared_sha256.clone(),
        consumed_grant_sha256: consumed,
        disposition,
        receipt_id: format!(
            "terminal-receipt:{}",
            &sha256_hex(attempt.attempt_id.as_bytes())[..16]
        ),
        worker_receipt_id: report.receipt_id,
        result_sha256: report.result_sha256,
        artifact_ids: report.artifact_ids,
        changed_state_ids: report.changed_state_ids,
        verification,
        cleanup: report.cleanup,
        completion_verified,
        terminal_sha256: "0".repeat(64),
    };
    terminal.terminal_sha256 = terminal_digest(&terminal);
    ledger
        .terminals
        .insert(attempt.attempt_id.clone(), terminal.clone());
    Ok(terminal)
}

fn required_preflights(operation: GrantOperation, tool_id: &str) -> Vec<String> {
    let mut kinds = BTreeSet::from([PreflightProbeKind::Platform, PreflightProbeKind::Resource]);
    match operation {
        GrantOperation::WorkspaceRead
        | GrantOperation::WorkspaceWrite
        | GrantOperation::WorkspaceDelete => {
            kinds.extend([PreflightProbeKind::Workspace, PreflightProbeKind::Storage]);
        }
        GrantOperation::CommandExecute => {
            kinds.extend([
                PreflightProbeKind::Workspace,
                PreflightProbeKind::Executable,
                PreflightProbeKind::Service,
                PreflightProbeKind::NetworkPolicy,
                PreflightProbeKind::Storage,
                PreflightProbeKind::CredentialReference,
            ]);
        }
        GrantOperation::NetworkAccess
        | GrantOperation::GitClone
        | GrantOperation::GitFetch
        | GrantOperation::GitPush
        | GrantOperation::Publish
        | GrantOperation::Send
        | GrantOperation::Upload
        | GrantOperation::Deploy
        | GrantOperation::DatabaseRead
        | GrantOperation::DatabaseWrite
        | GrantOperation::CredentialAccess
        | GrantOperation::ModelInference => {
            kinds.extend([
                PreflightProbeKind::NetworkPolicy,
                PreflightProbeKind::Service,
                PreflightProbeKind::CredentialReference,
            ]);
        }
        _ => {}
    }
    if tool_id.contains("git")
        || matches!(
            operation,
            GrantOperation::GitWorktreeCreate
                | GrantOperation::GitWorktreeRemove
                | GrantOperation::GitBranchFastForward
                | GrantOperation::GitCommit
        )
    {
        kinds.extend([PreflightProbeKind::Git, PreflightProbeKind::Executable]);
    }
    kinds.into_iter().map(|kind| kind.id().to_owned()).collect()
}

fn validate_preflights(
    policy: &ToolCompositionPolicy,
    probes: &PreflightRegistry,
    values: &[PreflightObservation],
    subject: &str,
    now: u64,
) -> Result<String, ToolCompositionError> {
    if values.len() != policy.required_preflight_ids.len() {
        return Err(ToolCompositionError::PreflightDenied);
    }
    for (required, value) in policy.required_preflight_ids.iter().zip(values) {
        let definition = probes
            .get(&value.probe_id, &value.probe_version)
            .ok_or(ToolCompositionError::PreflightDenied)?;
        if required != &value.probe_id
            || value.subject_sha256 != subject
            || !valid_sha256(&value.evidence_sha256)
            || value.evidence_bytes > definition.max_evidence_bytes
            || value.observed_at_epoch_ms > now
            || value.valid_until_epoch_ms <= now
            || value
                .valid_until_epoch_ms
                .saturating_sub(value.observed_at_epoch_ms)
                > definition.freshness_ms
            || value.outcome != PreflightOutcome::Passed
            || value.reason_code.is_some()
        {
            return Err(ToolCompositionError::PreflightDenied);
        }
    }
    Ok(sha256_json(&values))
}

fn canonical_ids(values: &mut [String], empty_allowed: bool) -> Result<(), ToolCompositionError> {
    values.sort();
    if (!empty_allowed && values.is_empty()) || !valid_sorted_ids(values, empty_allowed) {
        return Err(ToolCompositionError::InvalidCall);
    }
    Ok(())
}

fn canonical_hashes(values: &mut [String]) -> Result<(), ToolCompositionError> {
    values.sort();
    if values.len() > MAX_IDENTITIES
        || values.iter().any(|value| !valid_sha256(value))
        || values.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(ToolCompositionError::InvalidCall);
    }
    Ok(())
}

fn valid_sorted_ids(values: &[String], empty_allowed: bool) -> bool {
    (empty_allowed || !values.is_empty())
        && values.len() <= MAX_IDENTITIES
        && values.iter().all(|value| valid_id(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 255 && !value.chars().any(char::is_control)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn policy_digest(value: &ToolCompositionPolicy) -> String {
    let mut copy = value.clone();
    copy.policy_sha256 = "0".repeat(64);
    sha256_json(&copy)
}

fn prepared_digest(value: &PreparedToolAttempt) -> String {
    sha256_json(&(
        &value.call,
        &value.attempt_id,
        &value.policy_sha256,
        &value.preflight_sha256,
        &value.targets,
        &value.preimage_sha256s,
        &value.expected_effects,
        &value.postconditions,
        value.attempt_sequence,
    ))
}

fn terminal_digest(value: &ToolTerminalAttemptRecord) -> String {
    let mut copy = value.clone();
    copy.terminal_sha256 = "0".repeat(64);
    sha256_json(&copy)
}

fn sha256_json(value: &impl Serialize) -> String {
    sha256_hex(&serde_json::to_vec(value).expect("closed composition serialization"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("String write");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ActionId, CONTRACT_SCHEMA_VERSION, ContractPayload, CorrelationId, OperationBinding,
        RequiredGrantTemplate, SchemaId, SchemaReference, ToolCallId, ToolDefinition,
        ToolRiskLevel,
    };

    use super::*;
    use crate::tooling::Tool;

    struct FakeTool(ToolDefinition);

    impl Tool for FakeTool {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }

    fn schema(id: &str) -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw(id),
            schema_version: 1,
            schema_sha256: "a".repeat(64),
        }
    }

    fn definition(id: &str, operation: GrantOperation) -> ToolDefinition {
        let operation = OperationBinding::new(operation);
        ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw(id),
            tool_version: "1.0.0".to_owned(),
            display_name: format!("Fixture {id}"),
            description: "Exact synthetic composition fixture".to_owned(),
            input_schema: schema(&format!("{id}.input")),
            output_schema: schema(&format!("{id}.output")),
            risk_level: ToolRiskLevel::Low,
            declared_effects: vec![operation],
            required_grant: RequiredGrantTemplate {
                operation,
                target_scope: "exact-fixture".to_owned(),
                single_use: true,
            },
            timeout_ms: 1_000,
        }
    }

    fn registry() -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        for (id, operation) in [
            ("fixture.read", GrantOperation::WorkspaceRead),
            ("fixture.git", GrantOperation::WorkspaceRead),
            ("fixture.command", GrantOperation::CommandExecute),
        ] {
            registry
                .register_tool(Box::new(FakeTool(definition(id, operation))))
                .expect("definition");
        }
        registry
    }

    fn call(id: &str, call_id: &str) -> ToolCall {
        let bytes = br#"{"fixture":true}"#.to_vec();
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(call_id),
            correlation_id: CorrelationId::from_raw(format!("correlation-{call_id}")),
            action_id: ActionId::from_raw(format!("action-{call_id}")),
            tool_id: ToolId::from_raw(id),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: schema(&format!("{id}.input")),
                media_type: "application/json".to_owned(),
                sha256: sha256_hex(&bytes),
                bytes,
            },
        }
    }

    fn observations(
        policy: &ToolCompositionPolicy,
        call: &ToolCall,
        targets: &[String],
        preimages: &[String],
    ) -> Vec<PreflightObservation> {
        let subject = sha256_json(&(call, targets, preimages));
        policy
            .required_preflight_ids
            .iter()
            .map(|id| PreflightObservation {
                probe_id: id.clone(),
                probe_version: VERSION.to_owned(),
                subject_sha256: subject.clone(),
                evidence_sha256: sha256_hex(id.as_bytes()),
                evidence_bytes: 128,
                observed_at_epoch_ms: 1_000,
                valid_until_epoch_ms: 2_000,
                outcome: PreflightOutcome::Passed,
                reason_code: None,
            })
            .collect()
    }

    fn prepared(
        registry: &ToolRegistry,
        policies: &ToolCompositionRegistry,
        call: &ToolCall,
        guard: &mut ToolAttemptGuard,
    ) -> Result<PreparedToolAttempt, ToolCompositionError> {
        let targets = vec!["workspace:src/lib.rs".to_owned()];
        let preimages = vec!["b".repeat(64)];
        let policy = policies
            .get(&call.tool_id, &call.tool_version)
            .expect("policy");
        prepare_tool_attempt(
            registry,
            policies,
            &PreflightRegistry::standard(),
            guard,
            call,
            &observations(policy, call, &targets, &preimages),
            ToolAuthorityState {
                policy_allowed: true,
                approval_current: true,
                grant_current: true,
            },
            1_500,
            0,
            &format!("attempt-{}", call.tool_call_id.as_str()),
            targets,
            preimages,
            vec!["effect:observe".to_owned()],
            vec!["postcondition:exact".to_owned()],
        )
    }

    #[derive(Default)]
    struct FakeGrant {
        consumes: u32,
        fail: bool,
    }

    impl ToolGrantConsumptionPort for FakeGrant {
        fn consume_exact(
            &mut self,
            _: &PreparedToolAttempt,
        ) -> Result<String, ToolGrantConsumptionError> {
            self.consumes += 1;
            (!self.fail)
                .then(|| "c".repeat(64))
                .ok_or(ToolGrantConsumptionError::Denied)
        }
    }

    struct FakeWorker {
        launches: u32,
        disposition: ToolWorkerDisposition,
        receipt: bool,
        cleanup: ToolCleanupState,
    }

    impl ToolWorkerPort for FakeWorker {
        fn launch(&mut self, _: &PreparedToolAttempt, _: &str) -> ToolWorkerReport {
            self.launches += 1;
            ToolWorkerReport {
                disposition: self.disposition,
                receipt_id: self.receipt.then(|| format!("receipt-{}", self.launches)),
                result_sha256: "d".repeat(64),
                artifact_ids: vec!["artifact-1".to_owned()],
                changed_state_ids: Vec::new(),
                cleanup: self.cleanup,
            }
        }
    }

    struct FakeVerifier(ToolVerificationState, u32);

    impl ToolVerificationPort for FakeVerifier {
        fn verify(
            &mut self,
            _: &PreparedToolAttempt,
            _: &ToolWorkerReport,
        ) -> ToolVerificationState {
            self.1 += 1;
            self.0
        }
    }

    #[test]
    fn story_16_3_registers_all_exact_preflights_and_maps_every_visible_tool() {
        let probes = PreflightRegistry::standard();
        assert_eq!(probes.definitions().len(), PreflightProbeKind::ALL.len());
        assert_eq!(probes.definitions().len(), 9);
        for kind in PreflightProbeKind::ALL {
            assert!(probes.get(kind.id(), VERSION).is_some());
            assert!(probes.get(kind.id(), "2.0.0").is_none());
        }
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        assert_eq!(policies.policies().len(), tools.list_tools().len());
        for policy in policies.policies() {
            assert_eq!(policy.policy_sha256, policy_digest(policy));
            assert!(!policy.required_preflight_ids.is_empty());
            assert!(
                policy
                    .required_preflight_ids
                    .iter()
                    .all(|id| probes.get(id, VERSION).is_some())
            );
        }
        let command = policies
            .get(&ToolId::from_raw("fixture.command"), VERSION)
            .expect("command");
        for required in [
            PreflightProbeKind::Executable,
            PreflightProbeKind::Service,
            PreflightProbeKind::NetworkPolicy,
            PreflightProbeKind::CredentialReference,
        ] {
            assert!(
                command
                    .required_preflight_ids
                    .contains(&required.id().to_owned())
            );
        }
    }

    #[test]
    fn story_16_3_preflight_policy_grant_launch_verify_and_cleanup_order_is_exact() {
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        let mut guard = ToolAttemptGuard::new(2, 8).expect("guard");
        let attempt = prepared(
            &tools,
            &policies,
            &call("fixture.read", "success"),
            &mut guard,
        )
        .expect("prepared");
        let mut grant = FakeGrant::default();
        let mut worker = FakeWorker {
            launches: 0,
            disposition: ToolWorkerDisposition::Succeeded,
            receipt: true,
            cleanup: ToolCleanupState::Complete,
        };
        let mut verifier = FakeVerifier(ToolVerificationState::Passed, 0);
        let mut ledger = ToolExecutionLedger::default();
        let terminal = execute_prepared_tool_attempt(
            &attempt,
            &mut ledger,
            &ToolDispatchRevalidation::current(&attempt),
            &mut grant,
            &mut worker,
            &mut verifier,
        )
        .expect("terminal");
        assert_eq!((grant.consumes, worker.launches, verifier.1), (1, 1, 1));
        assert!(terminal.completion_verified);
        assert!(terminal.verify());
    }

    #[test]
    fn story_16_3_every_preflight_drift_fails_before_attempt_or_worker() {
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        let call = call("fixture.read", "drift");
        let policy = policies.get(&call.tool_id, VERSION).expect("policy");
        let targets = vec!["workspace:src/lib.rs".to_owned()];
        let preimages = vec!["b".repeat(64)];
        let base = observations(policy, &call, &targets, &preimages);
        for outcome in [
            PreflightOutcome::Missing,
            PreflightOutcome::Ambiguous,
            PreflightOutcome::OutOfBudget,
            PreflightOutcome::Inconsistent,
            PreflightOutcome::Failed,
        ] {
            let mut values = base.clone();
            values[0].outcome = outcome;
            values[0].reason_code = Some("preflight.fixture.denied".to_owned());
            let mut guard = ToolAttemptGuard::new(1, 8).expect("guard");
            assert_eq!(
                prepare_tool_attempt(
                    &tools,
                    &policies,
                    &PreflightRegistry::standard(),
                    &mut guard,
                    &call,
                    &values,
                    ToolAuthorityState {
                        policy_allowed: true,
                        approval_current: true,
                        grant_current: true
                    },
                    1_500,
                    0,
                    "attempt-drift",
                    targets.clone(),
                    preimages.clone(),
                    vec!["effect:observe".to_owned()],
                    vec!["postcondition:exact".to_owned()],
                ),
                Err(ToolCompositionError::PreflightDenied)
            );
        }
        for mutation in 0..5 {
            let mut values = base.clone();
            match mutation {
                0 => values[0].subject_sha256 = "e".repeat(64),
                1 => values[0].probe_version = "2.0.0".to_owned(),
                2 => values[0].evidence_bytes = MAX_EVIDENCE_BYTES + 1,
                3 => values[0].valid_until_epoch_ms = 1_500,
                _ => values.pop().map(|_| ()).expect("observation"),
            }
            let mut guard = ToolAttemptGuard::new(1, 8).expect("guard");
            assert_eq!(
                prepare_tool_attempt(
                    &tools,
                    &policies,
                    &PreflightRegistry::standard(),
                    &mut guard,
                    &call,
                    &values,
                    ToolAuthorityState {
                        policy_allowed: true,
                        approval_current: true,
                        grant_current: true
                    },
                    1_500,
                    0,
                    "attempt-mutation",
                    targets.clone(),
                    preimages.clone(),
                    vec!["effect:observe".to_owned()],
                    vec!["postcondition:exact".to_owned()],
                ),
                Err(ToolCompositionError::PreflightDenied)
            );
        }
    }

    #[test]
    fn story_16_3_duplicate_calls_and_failed_consumption_never_launch() {
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        let call = call("fixture.read", "duplicate");
        let mut guard = ToolAttemptGuard::new(2, 8).expect("guard");
        let attempt = prepared(&tools, &policies, &call, &mut guard).expect("prepared");
        assert_eq!(
            prepared(&tools, &policies, &call, &mut guard),
            Err(ToolCompositionError::RepeatedAttempt)
        );
        let mut grant = FakeGrant {
            consumes: 0,
            fail: true,
        };
        let mut worker = FakeWorker {
            launches: 0,
            disposition: ToolWorkerDisposition::Succeeded,
            receipt: true,
            cleanup: ToolCleanupState::Complete,
        };
        let mut verifier = FakeVerifier(ToolVerificationState::Passed, 0);
        let mut ledger = ToolExecutionLedger::default();
        assert_eq!(
            execute_prepared_tool_attempt(
                &attempt,
                &mut ledger,
                &ToolDispatchRevalidation::current(&attempt),
                &mut grant,
                &mut worker,
                &mut verifier,
            ),
            Err(ToolCompositionError::GrantConsumptionFailed)
        );
        assert_eq!((grant.consumes, worker.launches, verifier.1), (1, 0, 0));
        grant.fail = false;
        assert_eq!(
            execute_prepared_tool_attempt(
                &attempt,
                &mut ledger,
                &ToolDispatchRevalidation::current(&attempt),
                &mut grant,
                &mut worker,
                &mut verifier,
            ),
            Err(ToolCompositionError::RepeatedAttempt)
        );
        assert_eq!((grant.consumes, worker.launches, verifier.1), (1, 0, 0));
    }

    #[test]
    fn story_16_3_post_approval_preflight_policy_and_grant_drift_start_no_worker() {
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        for mutation in 0..5 {
            let mut guard = ToolAttemptGuard::new(1, 8).expect("guard");
            let attempt = prepared(
                &tools,
                &policies,
                &call("fixture.read", &format!("revalidate-{mutation}")),
                &mut guard,
            )
            .expect("prepared");
            let mut current = ToolDispatchRevalidation::current(&attempt);
            let expected = match mutation {
                0 => {
                    current.preflight_sha256 = "e".repeat(64);
                    ToolCompositionError::AuthorityDenied
                }
                1 => {
                    current.policy_sha256 = "e".repeat(64);
                    ToolCompositionError::AuthorityDenied
                }
                2 => {
                    current.policy_allowed = false;
                    ToolCompositionError::AuthorityDenied
                }
                3 => {
                    current.approval_current = false;
                    ToolCompositionError::AuthorityDenied
                }
                _ => {
                    current.grant_current = false;
                    ToolCompositionError::GrantNotCurrent
                }
            };
            let mut ledger = ToolExecutionLedger::default();
            let mut grant = FakeGrant::default();
            let mut worker = FakeWorker {
                launches: 0,
                disposition: ToolWorkerDisposition::Succeeded,
                receipt: true,
                cleanup: ToolCleanupState::Complete,
            };
            let mut verifier = FakeVerifier(ToolVerificationState::Passed, 0);
            assert_eq!(
                execute_prepared_tool_attempt(
                    &attempt,
                    &mut ledger,
                    &current,
                    &mut grant,
                    &mut worker,
                    &mut verifier,
                ),
                Err(expected)
            );
            assert_eq!(ledger.committed_len(), 0);
            assert_eq!((grant.consumes, worker.launches, verifier.1), (0, 0, 0));
        }
    }

    #[test]
    fn story_16_3_all_terminal_states_are_explicit_and_never_false_complete() {
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        for (index, disposition) in [
            ToolWorkerDisposition::Succeeded,
            ToolWorkerDisposition::NoOp,
            ToolWorkerDisposition::Partial,
            ToolWorkerDisposition::Denied,
            ToolWorkerDisposition::Cancelled,
            ToolWorkerDisposition::Failed,
            ToolWorkerDisposition::Uncertain,
            ToolWorkerDisposition::Blocked,
        ]
        .into_iter()
        .enumerate()
        {
            let mut guard = ToolAttemptGuard::new(1, 8).expect("guard");
            let attempt = prepared(
                &tools,
                &policies,
                &call("fixture.read", &format!("terminal-{index}")),
                &mut guard,
            )
            .expect("prepared");
            let mut grant = FakeGrant::default();
            let mut worker = FakeWorker {
                launches: 0,
                disposition,
                receipt: true,
                cleanup: ToolCleanupState::Complete,
            };
            let mut verifier = FakeVerifier(ToolVerificationState::Passed, 0);
            let mut ledger = ToolExecutionLedger::default();
            let terminal = execute_prepared_tool_attempt(
                &attempt,
                &mut ledger,
                &ToolDispatchRevalidation::current(&attempt),
                &mut grant,
                &mut worker,
                &mut verifier,
            )
            .expect("terminal");
            assert_eq!(terminal.disposition, disposition);
            assert_eq!(
                terminal.completion_verified,
                matches!(
                    disposition,
                    ToolWorkerDisposition::Succeeded | ToolWorkerDisposition::NoOp
                )
            );
            assert!(terminal.verify());
        }
    }

    #[test]
    fn story_16_3_receipt_verification_and_cleanup_gaps_force_noncompletion() {
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        for (index, receipt, cleanup, verification) in [
            (
                0,
                false,
                ToolCleanupState::Complete,
                ToolVerificationState::Passed,
            ),
            (
                1,
                true,
                ToolCleanupState::Partial,
                ToolVerificationState::Passed,
            ),
            (
                2,
                true,
                ToolCleanupState::Unknown,
                ToolVerificationState::Uncertain,
            ),
            (
                3,
                true,
                ToolCleanupState::Complete,
                ToolVerificationState::Failed,
            ),
            (
                4,
                true,
                ToolCleanupState::Complete,
                ToolVerificationState::Blocked,
            ),
        ] {
            let mut guard = ToolAttemptGuard::new(1, 8).expect("guard");
            let attempt = prepared(
                &tools,
                &policies,
                &call("fixture.read", &format!("gap-{index}")),
                &mut guard,
            )
            .expect("prepared");
            let mut grant = FakeGrant::default();
            let mut worker = FakeWorker {
                launches: 0,
                disposition: ToolWorkerDisposition::Succeeded,
                receipt,
                cleanup,
            };
            let mut verifier = FakeVerifier(verification, 0);
            let mut ledger = ToolExecutionLedger::default();
            let terminal = execute_prepared_tool_attempt(
                &attempt,
                &mut ledger,
                &ToolDispatchRevalidation::current(&attempt),
                &mut grant,
                &mut worker,
                &mut verifier,
            )
            .expect("terminal");
            assert!(!terminal.completion_verified);
            assert!(terminal.verify());
        }
    }

    #[test]
    fn story_16_3_crash_boundaries_restore_one_uncertain_terminal_without_replay() {
        let tools = registry();
        let policies = ToolCompositionRegistry::for_tools(&tools).expect("policies");
        for (index, boundary) in [
            "before-worker",
            "during-worker",
            "after-worker",
            "after-receipt",
            "after-artifact",
            "after-verification",
            "after-checkpoint",
        ]
        .into_iter()
        .enumerate()
        {
            let mut guard = ToolAttemptGuard::new(1, 8).expect("guard");
            let attempt = prepared(
                &tools,
                &policies,
                &call("fixture.read", &format!("crash-{index}")),
                &mut guard,
            )
            .expect("prepared");
            let mut ledger =
                ToolExecutionLedger::restore(vec![attempt.attempt_id.clone()], Vec::new())
                    .expect("restored launch commitment");
            let terminal = ledger
                .reconcile_uncertain(
                    &attempt.attempt_id,
                    &attempt.prepared_sha256,
                    &"c".repeat(64),
                    ToolCleanupState::Unknown,
                )
                .expect("uncertain reconciliation")
                .clone();
            assert_eq!(
                terminal.disposition,
                ToolWorkerDisposition::Uncertain,
                "{boundary}"
            );
            assert!(!terminal.completion_verified);
            assert!(terminal.verify());

            let mut grant = FakeGrant::default();
            let mut worker = FakeWorker {
                launches: 0,
                disposition: ToolWorkerDisposition::Succeeded,
                receipt: true,
                cleanup: ToolCleanupState::Complete,
            };
            let mut verifier = FakeVerifier(ToolVerificationState::Passed, 0);
            assert_eq!(
                execute_prepared_tool_attempt(
                    &attempt,
                    &mut ledger,
                    &ToolDispatchRevalidation::current(&attempt),
                    &mut grant,
                    &mut worker,
                    &mut verifier,
                ),
                Err(ToolCompositionError::RepeatedAttempt),
                "{boundary}"
            );
            assert_eq!((grant.consumes, worker.launches, verifier.1), (0, 0, 0));

            let restored = ToolExecutionLedger::restore(
                vec![attempt.attempt_id.clone()],
                vec![terminal.clone()],
            )
            .expect("terminal restart");
            assert_eq!(restored.terminal(&attempt.attempt_id), Some(&terminal));
            let mut tampered = terminal;
            tampered.completion_verified = true;
            assert!(
                ToolExecutionLedger::restore(vec![attempt.attempt_id.clone()], vec![tampered],)
                    .is_err()
            );
        }
    }
}
