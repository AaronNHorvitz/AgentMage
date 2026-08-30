//! Verifier-owned construction of exact workflow terminal results.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalStateChange, CanonicalTerminalOutcome,
    CanonicalTerminalResult, to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::engineering_records::ValidateCanonicalRecord;
use crate::workflow_verifier::VerifiedWorkflowEvidence;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_RESULTS: usize = 256;

/// Closed non-success reason supplied by deterministic runtime state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowNonSuccessKind {
    /// A prerequisite blocks further work.
    Blocked,
    /// Policy or authority denied the operation.
    Denied,
    /// Deterministic execution or verification failed.
    Failed,
    /// Cancellation and cleanup completed.
    Cancelled,
    /// The workflow exceeded its time ceiling.
    TimedOut,
    /// A resource ceiling was exhausted.
    ResourceExhausted,
    /// Effect or state truth cannot be reconciled.
    Uncertain,
}

impl WorkflowNonSuccessKind {
    const fn outcome(self) -> CanonicalTerminalOutcome {
        match self {
            Self::Blocked => CanonicalTerminalOutcome::Blocked,
            Self::Denied => CanonicalTerminalOutcome::Denied,
            Self::Failed => CanonicalTerminalOutcome::Failed,
            Self::Cancelled => CanonicalTerminalOutcome::Cancelled,
            Self::TimedOut => CanonicalTerminalOutcome::TimedOut,
            Self::ResourceExhausted => CanonicalTerminalOutcome::ResourceExhausted,
            Self::Uncertain => CanonicalTerminalOutcome::Uncertain,
        }
    }
}

/// Exact runtime-owned context for one non-success terminal result.
#[derive(Clone, Copy, Debug)]
pub struct WorkflowNonSuccess<'a> {
    /// Owning workflow identity.
    pub workflow_id: &'a str,
    /// Closed non-success class; no generic fallback exists.
    pub kind: WorkflowNonSuccessKind,
    /// Complete ordered verifier-result identities available at termination.
    pub verification_result_ids: &'a [String],
    /// Digest of the last state established by deterministic evidence.
    pub last_verified_state_sha256: &'a str,
    /// Stable content-free diagnostic code.
    pub diagnostic_code: &'a str,
    /// Bounded deterministic safe next action.
    pub safe_next_action: &'a str,
}

/// Stable content-free reason terminal-result construction was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowTerminalError {
    /// Terminal or workflow identity is malformed.
    InvalidIdentity,
    /// The non-success context is incomplete, duplicated, or malformed.
    InvalidNonSuccess,
    /// Canonical result serialization or validation failed.
    InvalidResult,
}

impl WorkflowTerminalError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentity => "workflow.terminal.identity_invalid",
            Self::InvalidNonSuccess => "workflow.terminal.non_success_invalid",
            Self::InvalidResult => "workflow.terminal.result_invalid",
        }
    }
}

impl std::fmt::Display for WorkflowTerminalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for WorkflowTerminalError {}

/// Constructs verified success or verified no-op from the opaque current-evidence proof.
///
/// No public path accepts a boolean, model claim, exit status, or raw result list as success.
pub fn resolve_verified_terminal(
    terminal_result_id: &str,
    evidence: &VerifiedWorkflowEvidence,
) -> Result<CanonicalTerminalResult, WorkflowTerminalError> {
    if !valid_id(terminal_result_id) {
        return Err(WorkflowTerminalError::InvalidIdentity);
    }
    let outcome = match evidence.observed_state_change() {
        CanonicalStateChange::Changed => CanonicalTerminalOutcome::VerifiedSuccess,
        CanonicalStateChange::NotChanged => CanonicalTerminalOutcome::VerifiedNoOp,
        CanonicalStateChange::Uncertain => return Err(WorkflowTerminalError::InvalidResult),
    };
    build_result(
        terminal_result_id,
        evidence.workflow_id(),
        outcome,
        evidence.verification_result_ids(),
        evidence.verified_state_sha256(),
        None,
        None,
    )
}

/// Constructs exactly one of the seven explicit non-success terminal outcomes.
pub fn resolve_non_success_terminal(
    terminal_result_id: &str,
    context: WorkflowNonSuccess<'_>,
) -> Result<CanonicalTerminalResult, WorkflowTerminalError> {
    if !valid_id(terminal_result_id) || !valid_id(context.workflow_id) {
        return Err(WorkflowTerminalError::InvalidIdentity);
    }
    if !valid_id(context.diagnostic_code)
        || context.safe_next_action.is_empty()
        || context.safe_next_action.len() > 512
        || !valid_sha256(context.last_verified_state_sha256)
        || context.verification_result_ids.len() > MAX_RESULTS
        || context
            .verification_result_ids
            .iter()
            .any(|identity| !valid_id(identity))
        || context
            .verification_result_ids
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != context.verification_result_ids.len()
    {
        return Err(WorkflowTerminalError::InvalidNonSuccess);
    }
    build_result(
        terminal_result_id,
        context.workflow_id,
        context.kind.outcome(),
        context.verification_result_ids,
        context.last_verified_state_sha256,
        Some(context.diagnostic_code),
        Some(context.safe_next_action),
    )
}

fn build_result(
    terminal_result_id: &str,
    workflow_id: &str,
    outcome: CanonicalTerminalOutcome,
    verification_result_ids: &[String],
    last_verified_state_sha256: &str,
    diagnostic_code: Option<&str>,
    safe_next_action: Option<&str>,
) -> Result<CanonicalTerminalResult, WorkflowTerminalError> {
    let mut result = CanonicalTerminalResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        terminal_result_id: terminal_result_id.to_owned(),
        workflow_id: workflow_id.to_owned(),
        outcome,
        verification_result_ids: verification_result_ids.to_vec(),
        last_verified_state_sha256: last_verified_state_sha256.to_owned(),
        diagnostic_code: diagnostic_code.map(str::to_owned),
        safe_next_action: safe_next_action.map(str::to_owned),
        established_by: "agentmage-runtime-verifier".to_owned(),
        result_sha256: ZERO_SHA256.to_owned(),
    };
    result.result_sha256 = terminal_result_sha256(&result)?;
    result
        .validate_canonical()
        .map_err(|_| WorkflowTerminalError::InvalidResult)?;
    Ok(result)
}

fn terminal_result_sha256(
    value: &CanonicalTerminalResult,
) -> Result<String, WorkflowTerminalError> {
    let mut unsigned = value.clone();
    unsigned.result_sha256 = ZERO_SHA256.to_owned();
    let bytes = to_canonical_json(&unsigned).map_err(|_| WorkflowTerminalError::InvalidResult)?;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    Ok(encoded)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
