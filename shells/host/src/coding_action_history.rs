//! Action histories of one coding run, kept by each effect owner (Decision 0127).
//!
//! The coding tool boundary keeps one entry for each call whose grant it
//! consumed and for each call a person refused. The host's live runtime service
//! keeps one entry for each job control request the durable job ledger decided.
//! The development host's runtime factory keeps one entry for each model route
//! it selected (Decision 0128).
//! Each owner keeps its own hash chain (Decision 0124) in memory for the run
//! and declares it with the run's other declarations (Decision 0116). A chain
//! that could not keep every entry is never declared. The client replays each
//! declared chain to its head, shows it, and builds a redacted export of a
//! requested range on request. Nothing here grants authority or writes a file.

use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CapabilityGrant, OperationOutcome, RuntimeRunRequest, ToolCall, VersionedContract,
    to_canonical_json,
};
use agentmage_kernel_engine::action_history::{
    ActionAuthorization, ActionHistory, ActionHistoryExport, ActionHistoryExportRequest,
    ActionHistoryHead, ActionHistoryRecord, ActionKind, ActionOutcome, ActionRecordDraft,
};
use agentmage_kernel_engine::job_control::{
    JobControlAction, JobControlDecision, JobControlRefusal, JobControlRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_recoverability::{ExecutedEffectKind, ExecutedEffectOutcome};

/// Largest number of entries one owner keeps for one run.
pub const MAX_RUN_ACTION_ENTRIES: usize = 512;
/// How long an entry is kept after it was recorded: thirty days.
pub const RUN_ACTION_RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1_000;
/// Largest rendered history; longer text ends with an explicit truncation line.
pub const MAX_RUN_ACTION_RENDER_BYTES: usize = 64 * 1024;

/// One declared chain: every retained position and the owner's head.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunActionHistory {
    /// Every position in order.
    pub records: Vec<ActionHistoryRecord>,
    /// The owner's retained end of the chain.
    pub head: ActionHistoryHead,
}

/// Which owner keeps a chain, and so which kinds of entry it may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunActionChain {
    /// The coding tool boundary: tool calls, file writes and commands.
    Effects,
    /// The host's job control service.
    JobControl,
    /// The host's model routing owner (Decision 0128).
    Routes,
}

impl RunActionChain {
    const fn admits(self, kind: ActionKind) -> bool {
        match self {
            Self::Effects => matches!(
                kind,
                ActionKind::ToolCall | ActionKind::FileWrite | ActionKind::CommandRun
            ),
            Self::JobControl => matches!(kind, ActionKind::JobControl),
            Self::Routes => matches!(kind, ActionKind::ModelRoute),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Effects => "effects",
            Self::JobControl => "job-control",
            Self::Routes => "routes",
        }
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Effects => "action history of this run's effects",
            Self::JobControl => "action history of this run's job control",
            Self::Routes => "action history of this run's model routes",
        }
    }
}

/// Content-free failure of a declared chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunActionHistoryError {
    /// The chain does not replay to its head, or holds another owner's kinds.
    Invalid,
    /// The requested export range is not in the chain.
    Range,
    /// The export still contained a secret signature and was withheld.
    SecretDetected,
}

/// One run's history as its owner keeps it. An entry that cannot be kept,
/// because the chain is full, the entry is malformed or its time went
/// backwards, makes the history incomplete, and it is never declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunActionRecorder {
    history: ActionHistory,
    complete: bool,
}

impl Default for RunActionRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl RunActionRecorder {
    /// Starts an empty, complete history.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            history: ActionHistory::new(),
            complete: true,
        }
    }

    /// Keeps one entry, or marks the history incomplete.
    pub fn record(&mut self, draft: Option<ActionRecordDraft>) {
        let kept = draft.is_some_and(|draft| {
            self.history.records().len() < MAX_RUN_ACTION_ENTRIES
                && self.history.append(&draft).is_ok()
        });
        if !kept {
            self.complete = false;
        }
    }

    /// Marks the history incomplete, for an owner that knows it missed an entry.
    pub const fn mark_incomplete(&mut self) {
        self.complete = false;
    }

    /// The whole chain, or `None` when an entry could not be kept.
    #[must_use]
    pub fn declare(&self) -> Option<RunActionHistory> {
        self.complete.then(|| RunActionHistory {
            records: self.history.records().to_vec(),
            head: self.history.head(),
        })
    }
}

/// A trusted effect owner that can declare the action history of one ended run.
pub trait RunActionHistorySource {
    /// The run's whole chain, or `None` when the owner cannot declare every
    /// entry of this exact run.
    fn declare_run_action_history(&self, request: &RuntimeRunRequest) -> Option<RunActionHistory>;
}

/// Which path issued the grant a call consumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantSource {
    /// A person approved the displayed call.
    PersonApproval,
    /// The session preauthorization a person approved covered the call.
    SessionPreauthorization,
}

/// The run a call belongs to.
#[derive(Clone, Copy, Debug)]
pub struct RunIdentity<'run> {
    /// Owning session.
    pub session_id: &'run str,
    /// Owning task.
    pub task_id: &'run str,
    /// Owning run.
    pub run_id: &'run str,
}

/// What the tool boundary knows about one call whose grant it consumed.
#[derive(Clone, Copy, Debug)]
pub struct ExecutedAction<'call> {
    /// Run the call belongs to.
    pub run: RunIdentity<'call>,
    /// Runtime operation identity.
    pub operation_id: &'call str,
    /// Exact admitted call.
    pub call: &'call ToolCall,
    /// What the call can do.
    pub kind: &'call ExecutedEffectKind,
    /// How the grant was issued.
    pub source: GrantSource,
    /// The exact kernel grant the call consumed.
    pub grant: &'call CapabilityGrant,
    /// Digest of the person's response or of the session preauthorization.
    pub decision_sha256: &'call str,
    /// Digest of the displayed preview.
    pub preview_sha256: &'call str,
    /// Digest of the runtime authority binding.
    pub authority_sha256: &'call str,
    /// Digest of the operation receipt, when the call returned one.
    pub receipt_sha256: Option<&'call str>,
    /// How the call ended.
    pub outcome: ExecutedEffectOutcome,
    /// Trusted time the grant was issued, from the coordinator's clock.
    pub authorized_at_epoch_ms: u64,
}

/// What the tool boundary knows about one call a person refused or narrowed.
#[derive(Clone, Copy, Debug)]
pub struct RefusedAction<'call> {
    /// Run the call belongs to.
    pub run: RunIdentity<'call>,
    /// Runtime operation identity.
    pub operation_id: &'call str,
    /// Exact proposed call.
    pub call: &'call ToolCall,
    /// What the call could have done.
    pub kind: &'call ExecutedEffectKind,
    /// Digest of the person's response.
    pub decision_sha256: &'call str,
    /// Digest of the displayed preview.
    pub preview_sha256: &'call str,
    /// Stable refusal code.
    pub reason_code: &'call str,
    /// Trusted time of the decision, from the coordinator's clock.
    pub decided_at_epoch_ms: u64,
}

/// What the job control service knows about one decided control request.
#[derive(Clone, Copy, Debug)]
pub struct DecidedJobControl<'request> {
    /// Authenticated client scope the ledger recorded the request under.
    pub client_scope: &'request str,
    /// Exact control request.
    pub request: &'request JobControlRequest,
    /// The ledger's decision.
    pub decision: JobControlDecision,
    /// The ledger's head after the decision.
    pub ledger_head_sha256: &'request str,
    /// Host clock time of the decision.
    pub decided_at_epoch_ms: u64,
}

const fn effect_action_kind(kind: &ExecutedEffectKind) -> ActionKind {
    match kind {
        ExecutedEffectKind::Write | ExecutedEffectKind::Create { .. } => ActionKind::FileWrite,
        ExecutedEffectKind::Command => ActionKind::CommandRun,
        ExecutedEffectKind::NoEffect => ActionKind::ToolCall,
    }
}

/// Digest of one call's identity within its run: the owner's effect digest.
fn effect_identity_sha256(
    run: RunIdentity<'_>,
    operation_id: &str,
    call: &ToolCall,
) -> Option<String> {
    let call_sha256 = contract_sha256(call)?;
    let identity = serde_json::json!({
        "schema_version": 1,
        "record_type": "agentmage-coding-effect-identity",
        "session_id": run.session_id,
        "task_id": run.task_id,
        "run_id": run.run_id,
        "operation_id": operation_id,
        "tool_call_sha256": call_sha256,
    });
    serde_json::to_vec(&identity)
        .ok()
        .map(|bytes| sha256_hex(&bytes))
}

fn contract_sha256<T: VersionedContract>(value: &T) -> Option<String> {
    to_canonical_json(value)
        .ok()
        .map(|bytes| sha256_hex(&bytes))
}

/// Sorted, unique evidence digests.
fn evidence(values: &[&str]) -> Vec<String> {
    let mut evidence = values
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    evidence.sort();
    evidence.dedup();
    evidence
}

/// The entry for one call whose grant was consumed. The grant authorized it;
/// the decision the grant came from is evidence, and the reason code says
/// whether a person approved the call or a session preauthorization covered it.
#[must_use]
pub fn executed_action_draft(action: &ExecutedAction<'_>) -> Option<ActionRecordDraft> {
    let (outcome, ended) = match action.outcome {
        ExecutedEffectOutcome::Completed { outcome, .. } => match outcome {
            OperationOutcome::Succeeded => (ActionOutcome::Succeeded, "succeeded"),
            OperationOutcome::Failed => (ActionOutcome::Failed, "failed"),
            OperationOutcome::TimedOut => (ActionOutcome::Failed, "timed-out"),
            OperationOutcome::Cancelled => (ActionOutcome::Cancelled, "cancelled"),
            OperationOutcome::Denied => (ActionOutcome::Denied, "denied"),
            OperationOutcome::Uncertain => (ActionOutcome::Uncertain, "uncertain"),
        },
        ExecutedEffectOutcome::Unknown => (ActionOutcome::Uncertain, "unknown"),
    };
    let source = match action.source {
        GrantSource::PersonApproval => "approved",
        GrantSource::SessionPreauthorization => "preauthorized",
    };
    let mut digests = vec![
        action.decision_sha256,
        action.preview_sha256,
        action.authority_sha256,
    ];
    digests.extend(action.receipt_sha256);
    Some(ActionRecordDraft {
        action_kind: effect_action_kind(action.kind),
        action_id: action.operation_id.to_owned(),
        authorization: ActionAuthorization::Grant {
            grant_id: action.grant.grant_id.as_str().to_owned(),
            grant_sha256: contract_sha256(action.grant)?,
        },
        effect_sha256: effect_identity_sha256(action.run, action.operation_id, action.call)?,
        outcome,
        reason_code: format!("coding.{source}.{ended}"),
        evidence_sha256s: evidence(&digests),
        recorded_at_epoch_ms: action.authorized_at_epoch_ms,
        retain_until_epoch_ms: action
            .authorized_at_epoch_ms
            .checked_add(RUN_ACTION_RETENTION_MS)?,
    })
}

/// The entry for one call a person refused, or narrowed into a derived call.
#[must_use]
pub fn refused_action_draft(action: &RefusedAction<'_>) -> Option<ActionRecordDraft> {
    Some(ActionRecordDraft {
        action_kind: effect_action_kind(action.kind),
        action_id: action.operation_id.to_owned(),
        authorization: ActionAuthorization::PersonDecision {
            decision_sha256: action.decision_sha256.to_owned(),
        },
        effect_sha256: effect_identity_sha256(action.run, action.operation_id, action.call)?,
        outcome: ActionOutcome::Denied,
        reason_code: action.reason_code.to_owned(),
        evidence_sha256s: evidence(&[action.preview_sha256]),
        recorded_at_epoch_ms: action.decided_at_epoch_ms,
        retain_until_epoch_ms: action
            .decided_at_epoch_ms
            .checked_add(RUN_ACTION_RETENTION_MS)?,
    })
}

const fn control_name(action: JobControlAction) -> &'static str {
    match action {
        JobControlAction::Suspend => "suspend",
        JobControlAction::Resume => "resume",
        JobControlAction::Cancel => "cancel",
    }
}

/// The entry for one control request the job ledger decided. The client's
/// request, recorded under its authenticated scope, is the person's decision.
#[must_use]
pub fn job_control_draft(control: &DecidedJobControl<'_>) -> Option<ActionRecordDraft> {
    let request_sha256 = serde_json::to_vec(&serde_json::json!({
        "record_type": "agentmage-job-control-request",
        "client_scope": control.client_scope,
        "request": control.request,
    }))
    .ok()
    .map(|bytes| sha256_hex(&bytes))?;
    let effect_sha256 = serde_json::to_vec(&serde_json::json!({
        "record_type": "agentmage-job-control-effect",
        "job_id": control.request.job_id,
        "action": control.request.action,
        "observed_revision": control.request.observed_revision,
    }))
    .ok()
    .map(|bytes| sha256_hex(&bytes))?;
    let action = control_name(control.request.action);
    let (outcome, reason_code) = match control.decision {
        JobControlDecision::Applied { .. } => (
            ActionOutcome::Succeeded,
            format!("job.control.{action}.applied"),
        ),
        JobControlDecision::Refused { refusal, .. } => (
            ActionOutcome::Denied,
            format!(
                "job.control.{action}.{}",
                match refusal {
                    JobControlRefusal::StaleRevision => "stale-revision",
                    JobControlRefusal::AlreadyInEffect => "already-in-effect",
                    JobControlRefusal::Terminal => "terminal",
                    JobControlRefusal::NotAllowed => "not-allowed",
                }
            ),
        ),
    };
    Some(ActionRecordDraft {
        action_kind: ActionKind::JobControl,
        action_id: control.request.request_id.clone(),
        authorization: ActionAuthorization::PersonDecision {
            decision_sha256: request_sha256,
        },
        effect_sha256,
        outcome,
        reason_code,
        evidence_sha256s: evidence(&[control.ledger_head_sha256]),
        recorded_at_epoch_ms: control.decided_at_epoch_ms,
        retain_until_epoch_ms: control
            .decided_at_epoch_ms
            .checked_add(RUN_ACTION_RETENTION_MS)?,
    })
}

/// Replays one declared chain to its head and checks that every kept entry is
/// a kind its owner keeps. The client uses only a chain that passes.
pub fn verify_run_action_history(
    history: &RunActionHistory,
    chain: RunActionChain,
) -> Result<ActionHistory, RunActionHistoryError> {
    if history.records.len() > MAX_RUN_ACTION_ENTRIES
        || history.records.iter().any(|record| match record {
            ActionHistoryRecord::Kept(entry) => !chain.admits(entry.action_kind),
            ActionHistoryRecord::Expired { .. } => false,
        })
    {
        return Err(RunActionHistoryError::Invalid);
    }
    ActionHistory::replay(history.records.clone(), &history.head)
        .map_err(|_| RunActionHistoryError::Invalid)
}

/// A requested export: one chain and an inclusive, one-based range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionHistoryExportSelection {
    /// Which chain.
    pub chain: RunActionChain,
    /// First position.
    pub from_sequence: u64,
    /// Last position, inclusive.
    pub to_sequence: u64,
}

impl ActionHistoryExportSelection {
    /// Parses `CHAIN:FROM:TO`, where the chain is `effects`, `job-control` or
    /// `routes` and the positions are one-based, inclusive and in order.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let mut parts = value.split(':');
        let chain = match parts.next()? {
            "effects" => RunActionChain::Effects,
            "job-control" => RunActionChain::JobControl,
            "routes" => RunActionChain::Routes,
            _ => return None,
        };
        let position = |part: Option<&str>| {
            part.filter(|text| {
                !text.is_empty()
                    && text.len() <= 6
                    && !text.starts_with('0')
                    && text.bytes().all(|byte| byte.is_ascii_digit())
            })
            .and_then(|text| text.parse::<u64>().ok())
        };
        let from_sequence = position(parts.next())?;
        let to_sequence = position(parts.next())?;
        (parts.next().is_none()
            && from_sequence <= to_sequence
            && to_sequence <= MAX_RUN_ACTION_ENTRIES as u64)
            .then_some(Self {
                chain,
                from_sequence,
                to_sequence,
            })
    }
}

/// Builds the redacted, rescanned export of the selected range of a verified
/// chain. The bytes are returned; nothing is written.
pub fn export_run_action_history(
    history: &RunActionHistory,
    selection: ActionHistoryExportSelection,
) -> Result<ActionHistoryExport, RunActionHistoryError> {
    let replayed = verify_run_action_history(history, selection.chain)?;
    replayed
        .export(ActionHistoryExportRequest {
            from_sequence: selection.from_sequence,
            to_sequence: selection.to_sequence,
        })
        .map_err(|error| match error {
            agentmage_kernel_engine::action_history::ActionHistoryError::SecretDetected => {
                RunActionHistoryError::SecretDetected
            }
            _ => RunActionHistoryError::Range,
        })
}

const fn kind_name(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::ToolCall => "tool_call",
        ActionKind::FileWrite => "file_write",
        ActionKind::CommandRun => "command_run",
        ActionKind::NetworkRequest => "network_request",
        ActionKind::ModelRoute => "model_route",
        ActionKind::MemoryChange => "memory_change",
        ActionKind::ExtensionLifecycle => "extension_lifecycle",
        ActionKind::JobControl => "job_control",
    }
}

const fn outcome_name(outcome: ActionOutcome) -> &'static str {
    match outcome {
        ActionOutcome::Succeeded => "succeeded",
        ActionOutcome::Failed => "failed",
        ActionOutcome::Denied => "denied",
        ActionOutcome::Cancelled => "cancelled",
        ActionOutcome::Uncertain => "uncertain",
    }
}

/// Bounded text for one chain, or an unavailable line. Entries are shown by
/// kind, identity, outcome, authorization and reason; digests are shortened.
#[must_use]
pub fn render_run_action_history(
    chain: RunActionChain,
    history: Option<&RunActionHistory>,
) -> String {
    let title = chain.title();
    let Some(history) = history else {
        return format!("{title}: unavailable; the host could not declare it completely\n");
    };
    let mut output = format!(
        "{title}: {} {}, head {}\n",
        history.head.count,
        if history.head.count == 1 {
            "entry"
        } else {
            "entries"
        },
        short(&history.head.head_sha256)
    );
    for (index, record) in history.records.iter().enumerate() {
        let line = match record {
            ActionHistoryRecord::Kept(entry) => {
                let authorization = match &entry.authorization {
                    ActionAuthorization::Grant { grant_id, .. } => format!("grant {grant_id}"),
                    ActionAuthorization::PersonDecision { decision_sha256 } => {
                        format!("person decision {}", short(decision_sha256))
                    }
                    ActionAuthorization::Unauthorized {} => "no authorization".to_owned(),
                };
                format!(
                    "- {} {} {} {}; {authorization}; {}\n",
                    entry.sequence,
                    kind_name(entry.action_kind),
                    entry.action_id,
                    outcome_name(entry.outcome),
                    entry.reason_code
                )
            }
            ActionHistoryRecord::Expired { sequence, .. } => {
                format!("- {sequence} expired: only its place is kept\n")
            }
        };
        if output.len() + line.len() > MAX_RUN_ACTION_RENDER_BYTES {
            let _ = writeln!(
                output,
                "... history truncated; {} of {} entries not shown",
                history.records.len() - index,
                history.records.len()
            );
            break;
        }
        output.push_str(&line);
    }
    output
}

/// The export of one selection as the lines a client prints: the exact
/// export bytes for standard output, preceded in text form by a header, and a
/// content-free notice for standard error when no export was made.
#[must_use]
pub fn render_action_history_export(
    selection: ActionHistoryExportSelection,
    history: Option<&RunActionHistory>,
    json: bool,
) -> (String, String) {
    let chain = selection.chain.name();
    let exported = history
        .ok_or(RunActionHistoryError::Invalid)
        .and_then(|history| export_run_action_history(history, selection));
    let export = match exported {
        Ok(export) => export,
        Err(error) => {
            let reason = match error {
                RunActionHistoryError::Invalid => "unavailable",
                RunActionHistoryError::Range => "range",
                RunActionHistoryError::SecretDetected => "secret-detected",
            };
            let notice = if json {
                format!(
                    "{}\n",
                    serde_json::json!({
                        "type": "action_history_export",
                        "available": false,
                        "chain": chain,
                        "from_sequence": selection.from_sequence,
                        "to_sequence": selection.to_sequence,
                        "reason": reason,
                    })
                )
            } else {
                format!(
                    "action_history_export chain={chain} from={} to={} unavailable reason={reason}\n",
                    selection.from_sequence, selection.to_sequence
                )
            };
            return (String::new(), notice);
        }
    };
    // Export bytes are JSON text with no line break, so they print exactly.
    let document = String::from_utf8_lossy(&export.bytes);
    let stdout = if json {
        format!(
            "{}\n",
            serde_json::json!({
                "type": "action_history_export",
                "available": true,
                "chain": chain,
                "from_sequence": selection.from_sequence,
                "to_sequence": selection.to_sequence,
                "export_sha256": export.export_sha256,
                "kept_count": export.kept_count,
                "expired_count": export.expired_count,
                "redacted_fields": export.redacted_fields,
                "document": document,
            })
        )
    } else {
        format!(
            "action_history_export chain={chain} from={} to={} sha256={} bytes={} kept={} expired={} redacted_fields={}\n{document}\n",
            selection.from_sequence,
            selection.to_sequence,
            export.export_sha256,
            export.bytes.len(),
            export.kept_count,
            export.expired_count,
            export.redacted_fields,
        )
    };
    (stdout, String::new())
}

fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: RunIdentity<'static> = RunIdentity {
        session_id: "session-0001",
        task_id: "task-0001",
        run_id: "run-0001",
    };
    // Identities in the forms the runtime and the Linux boundary mint.
    const OPERATION: &str = "operation:0123456789abcdef0123456789abcdef";

    fn digest(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn call() -> ToolCall {
        serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v2/valid/tool_call.json"
        ))
        .unwrap()
    }

    fn grant() -> CapabilityGrant {
        let mut grant: CapabilityGrant = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v2/valid/capability_grant.json"
        ))
        .unwrap();
        grant.grant_id =
            agentmage_kernel_contracts::GrantId::from_raw("grant-00000000000000000000000000000007");
        grant
    }

    struct Facts {
        call: ToolCall,
        grant: CapabilityGrant,
        kind: ExecutedEffectKind,
        decision: String,
        preview: String,
        authority: String,
        receipt: String,
    }

    impl Facts {
        fn new(kind: ExecutedEffectKind) -> Self {
            Self {
                call: call(),
                grant: grant(),
                kind,
                decision: digest('d'),
                preview: digest('b'),
                authority: digest('c'),
                receipt: digest('a'),
            }
        }

        fn executed(&self, outcome: ExecutedEffectOutcome, at: u64) -> ExecutedAction<'_> {
            ExecutedAction {
                run: RUN,
                operation_id: OPERATION,
                call: &self.call,
                kind: &self.kind,
                source: GrantSource::PersonApproval,
                grant: &self.grant,
                decision_sha256: &self.decision,
                preview_sha256: &self.preview,
                authority_sha256: &self.authority,
                receipt_sha256: Some(&self.receipt),
                outcome,
                authorized_at_epoch_ms: at,
            }
        }

        fn refused(&self, reason_code: &'static str, at: u64) -> RefusedAction<'_> {
            RefusedAction {
                run: RUN,
                operation_id: OPERATION,
                call: &self.call,
                kind: &self.kind,
                decision_sha256: &self.decision,
                preview_sha256: &self.preview,
                reason_code,
                decided_at_epoch_ms: at,
            }
        }
    }

    const fn completed(outcome: OperationOutcome) -> ExecutedEffectOutcome {
        ExecutedEffectOutcome::Completed {
            outcome,
            changed: true,
        }
    }

    fn control(
        request_id: &str,
        action: JobControlAction,
        decision: JobControlDecision,
    ) -> (JobControlRequest, JobControlDecision) {
        (
            JobControlRequest {
                schema_version: 1,
                job_id: RUN.run_id.to_owned(),
                request_id: request_id.to_owned(),
                action,
                observed_revision: 2,
            },
            decision,
        )
    }

    fn decided<'a>(
        scope: &'a str,
        request: &'a JobControlRequest,
        decision: JobControlDecision,
        head: &'a str,
        at: u64,
    ) -> DecidedJobControl<'a> {
        DecidedJobControl {
            client_scope: scope,
            request,
            decision,
            ledger_head_sha256: head,
            decided_at_epoch_ms: at,
        }
    }

    #[test]
    fn an_executed_call_is_kept_with_its_grant_decision_evidence_and_outcome() {
        let write = Facts::new(ExecutedEffectKind::Write);
        let draft =
            executed_action_draft(&write.executed(completed(OperationOutcome::Succeeded), 10))
                .unwrap();
        assert_eq!(draft.action_kind, ActionKind::FileWrite);
        assert_eq!(draft.action_id, OPERATION);
        assert_eq!(
            draft.authorization,
            ActionAuthorization::Grant {
                grant_id: "grant-00000000000000000000000000000007".to_owned(),
                grant_sha256: contract_sha256(&write.grant).unwrap(),
            }
        );
        assert_eq!(draft.outcome, ActionOutcome::Succeeded);
        assert_eq!(draft.reason_code, "coding.approved.succeeded");
        // Decision, preview, binding and receipt digests, sorted and unique.
        assert_eq!(
            draft.evidence_sha256s,
            vec![digest('a'), digest('b'), digest('c'), digest('d')]
        );
        assert_eq!(draft.recorded_at_epoch_ms, 10);
        assert_eq!(draft.retain_until_epoch_ms, 10 + RUN_ACTION_RETENTION_MS);
        // The effect digest binds the run, the operation and the exact call.
        let mut other_run = write.executed(completed(OperationOutcome::Succeeded), 10);
        other_run.run.run_id = "run-0002";
        let mut other_call = Facts::new(ExecutedEffectKind::Write);
        other_call.call.tool_version = "9.9.9".to_owned();
        for changed in [
            executed_action_draft(&other_run).unwrap(),
            executed_action_draft(&other_call.executed(completed(OperationOutcome::Succeeded), 10))
                .unwrap(),
        ] {
            assert_ne!(changed.effect_sha256, draft.effect_sha256);
        }
        // Realistic identities are kept, not redacted.
        let mut recorder = RunActionRecorder::new();
        recorder.record(Some(draft));
        let declared = recorder.declare().unwrap();
        let ActionHistoryRecord::Kept(entry) = &declared.records[0] else {
            panic!("kept")
        };
        assert_eq!(
            (entry.redacted_fields, entry.action_id.as_str()),
            (0, OPERATION)
        );
        // A preauthorized call names its source; an unreturned call has no receipt.
        let mut preauthorized = write.executed(ExecutedEffectOutcome::Unknown, 11);
        preauthorized.source = GrantSource::SessionPreauthorization;
        preauthorized.receipt_sha256 = None;
        preauthorized.preview_sha256 = &write.decision;
        let draft = executed_action_draft(&preauthorized).unwrap();
        assert_eq!(draft.reason_code, "coding.preauthorized.unknown");
        assert_eq!(draft.outcome, ActionOutcome::Uncertain);
        assert_eq!(draft.evidence_sha256s, vec![digest('c'), digest('d')]);
    }

    #[test]
    fn kinds_and_outcomes_map_closed() {
        for (kind, expected) in [
            (ExecutedEffectKind::Write, ActionKind::FileWrite),
            (
                ExecutedEffectKind::Create {
                    path: vec!["src".to_owned(), "new.py".to_owned()],
                },
                ActionKind::FileWrite,
            ),
            (ExecutedEffectKind::Command, ActionKind::CommandRun),
            (ExecutedEffectKind::NoEffect, ActionKind::ToolCall),
        ] {
            let facts = Facts::new(kind);
            let draft =
                executed_action_draft(&facts.executed(completed(OperationOutcome::Failed), 1))
                    .unwrap();
            assert_eq!(draft.action_kind, expected);
        }
        let facts = Facts::new(ExecutedEffectKind::Command);
        for (outcome, expected, code) in [
            (
                OperationOutcome::Succeeded,
                ActionOutcome::Succeeded,
                "succeeded",
            ),
            (OperationOutcome::Failed, ActionOutcome::Failed, "failed"),
            (
                OperationOutcome::TimedOut,
                ActionOutcome::Failed,
                "timed-out",
            ),
            (
                OperationOutcome::Cancelled,
                ActionOutcome::Cancelled,
                "cancelled",
            ),
            (OperationOutcome::Denied, ActionOutcome::Denied, "denied"),
            (
                OperationOutcome::Uncertain,
                ActionOutcome::Uncertain,
                "uncertain",
            ),
        ] {
            let draft = executed_action_draft(&facts.executed(completed(outcome), 1)).unwrap();
            assert_eq!(draft.outcome, expected);
            assert_eq!(draft.reason_code, format!("coding.approved.{code}"));
        }
    }

    #[test]
    fn a_refused_call_is_kept_as_the_persons_denial() {
        let facts = Facts::new(ExecutedEffectKind::Write);
        let draft = refused_action_draft(&facts.refused("runtime.coding.user-denied", 7)).unwrap();
        assert_eq!(draft.action_kind, ActionKind::FileWrite);
        assert_eq!(
            draft.authorization,
            ActionAuthorization::PersonDecision {
                decision_sha256: digest('d'),
            }
        );
        assert_eq!(draft.outcome, ActionOutcome::Denied);
        assert_eq!(draft.reason_code, "runtime.coding.user-denied");
        assert_eq!(draft.evidence_sha256s, vec![digest('b')]);
        assert_eq!(draft.recorded_at_epoch_ms, 7);
        // The same call executed has the same effect digest.
        let executed =
            executed_action_draft(&facts.executed(completed(OperationOutcome::Succeeded), 8))
                .unwrap();
        assert_eq!(executed.effect_sha256, draft.effect_sha256);
    }

    #[test]
    fn a_decided_control_request_is_kept_under_its_client_scope() {
        let head = digest('e');
        let (cancel, applied) = control(
            "cancel-0123456789abcdef0123456789abcdef-1",
            JobControlAction::Cancel,
            JobControlDecision::Applied {
                revision: 3,
                phase: agentmage_kernel_engine::job_control::JobPhase::Cancelling,
            },
        );
        let draft = job_control_draft(&decided("client-a", &cancel, applied, &head, 5)).unwrap();
        assert_eq!(draft.action_kind, ActionKind::JobControl);
        assert_eq!(draft.action_id, cancel.request_id);
        assert_eq!(draft.outcome, ActionOutcome::Succeeded);
        assert_eq!(draft.reason_code, "job.control.cancel.applied");
        assert_eq!(draft.evidence_sha256s, vec![head.clone()]);
        let ActionAuthorization::PersonDecision { decision_sha256 } = &draft.authorization else {
            panic!("person decision")
        };
        // The decision digest binds the client scope; the effect digest does not
        // depend on who asked or on the request identity.
        let other = job_control_draft(&decided("client-b", &cancel, applied, &head, 5)).unwrap();
        assert_ne!(other.authorization, draft.authorization);
        assert_eq!(other.effect_sha256, draft.effect_sha256);
        let mut renamed = cancel.clone();
        renamed.request_id = "cancel-other-1".to_owned();
        let renamed_draft =
            job_control_draft(&decided("client-a", &renamed, applied, &head, 5)).unwrap();
        assert_eq!(renamed_draft.effect_sha256, draft.effect_sha256);
        assert_ne!(renamed_draft.authorization, draft.authorization);
        assert!(decision_sha256.len() == 64);
        for (action, refusal, code) in [
            (
                JobControlAction::Suspend,
                JobControlRefusal::StaleRevision,
                "job.control.suspend.stale-revision",
            ),
            (
                JobControlAction::Resume,
                JobControlRefusal::AlreadyInEffect,
                "job.control.resume.already-in-effect",
            ),
            (
                JobControlAction::Cancel,
                JobControlRefusal::Terminal,
                "job.control.cancel.terminal",
            ),
            (
                JobControlAction::Suspend,
                JobControlRefusal::NotAllowed,
                "job.control.suspend.not-allowed",
            ),
        ] {
            let (request, refused) = control(
                "suspend-1",
                action,
                JobControlDecision::Refused {
                    refusal,
                    revision: 2,
                    phase: agentmage_kernel_engine::job_control::JobPhase::Running,
                },
            );
            let draft =
                job_control_draft(&decided("client-a", &request, refused, &head, 5)).unwrap();
            assert_eq!(
                (draft.outcome, draft.reason_code.as_str()),
                (ActionOutcome::Denied, code)
            );
        }
    }

    #[test]
    fn an_entry_that_cannot_be_kept_leaves_the_history_undeclared() {
        let facts = Facts::new(ExecutedEffectKind::Command);
        let draft =
            |at| executed_action_draft(&facts.executed(completed(OperationOutcome::Succeeded), at));
        let mut recorder = RunActionRecorder::new();
        assert_eq!(recorder.declare().unwrap().head.count, 0);
        recorder.record(draft(10));
        recorder.record(draft(12));
        let declared = recorder.declare().unwrap();
        assert_eq!(declared.head.count, 2);
        assert!(verify_run_action_history(&declared, RunActionChain::Effects).is_ok());
        // Time going backwards, a draft that could not be built, a malformed
        // entry and a full chain each leave the history undeclared.
        let mut backwards = recorder.clone();
        backwards.record(draft(11));
        let mut missing = recorder.clone();
        missing.record(None);
        let mut malformed = recorder.clone();
        let mut bad = draft(13).unwrap();
        bad.effect_sha256 = "short".to_owned();
        malformed.record(Some(bad));
        let mut marked = recorder.clone();
        marked.mark_incomplete();
        let mut full = RunActionRecorder::new();
        for _ in 0..MAX_RUN_ACTION_ENTRIES {
            full.record(draft(20));
        }
        assert!(full.declare().is_some());
        full.record(draft(20));
        for recorder in [backwards, missing, malformed, marked, full] {
            assert_eq!(recorder.declare(), None);
        }
        // A retention deadline past the end of time cannot be kept.
        assert_eq!(draft(u64::MAX - 1), None);
    }

    fn effects_history() -> RunActionHistory {
        let facts = Facts::new(ExecutedEffectKind::Write);
        let mut recorder = RunActionRecorder::new();
        recorder.record(executed_action_draft(
            &facts.executed(completed(OperationOutcome::Succeeded), 10),
        ));
        recorder.record(refused_action_draft(
            &facts.refused("runtime.coding.user-denied", 11),
        ));
        recorder.record(executed_action_draft(
            &facts.executed(completed(OperationOutcome::Failed), 12),
        ));
        recorder.declare().unwrap()
    }

    #[test]
    fn a_declared_chain_is_used_only_when_it_replays_and_holds_its_owners_kinds() {
        let history = effects_history();
        assert_eq!(
            verify_run_action_history(&history, RunActionChain::Effects)
                .unwrap()
                .head(),
            history.head
        );
        // Another owner's kinds, either way round, a changed entry, a dropped
        // entry and a foreign head are refused.
        assert_eq!(
            verify_run_action_history(&history, RunActionChain::JobControl),
            Err(RunActionHistoryError::Invalid)
        );
        let (cancel, applied) = control(
            "cancel-1",
            JobControlAction::Cancel,
            JobControlDecision::Applied {
                revision: 3,
                phase: agentmage_kernel_engine::job_control::JobPhase::Cancelling,
            },
        );
        let head = digest('e');
        let mut controls = RunActionRecorder::new();
        controls.record(job_control_draft(&decided(
            "client-a", &cancel, applied, &head, 5,
        )));
        let controls = controls.declare().unwrap();
        assert!(verify_run_action_history(&controls, RunActionChain::JobControl).is_ok());
        assert_eq!(
            verify_run_action_history(&controls, RunActionChain::Effects),
            Err(RunActionHistoryError::Invalid)
        );
        // The route chain keeps model routes only, and no other chain keeps
        // them (Decision 0128).
        for chain in [RunActionChain::Effects, RunActionChain::JobControl] {
            assert!(!chain.admits(ActionKind::ModelRoute));
        }
        for kind in [
            ActionKind::ToolCall,
            ActionKind::FileWrite,
            ActionKind::CommandRun,
            ActionKind::JobControl,
        ] {
            assert!(!RunActionChain::Routes.admits(kind));
        }
        assert!(RunActionChain::Routes.admits(ActionKind::ModelRoute));
        for other in [&history, &controls] {
            assert_eq!(
                verify_run_action_history(other, RunActionChain::Routes),
                Err(RunActionHistoryError::Invalid)
            );
        }
        let mut changed = history.clone();
        if let ActionHistoryRecord::Kept(entry) = &mut changed.records[1] {
            entry.outcome = ActionOutcome::Cancelled;
        }
        let mut dropped = history.clone();
        dropped.records.pop();
        let mut foreign = history.clone();
        foreign.head.head_sha256 = digest('f');
        for invalid in [changed, dropped, foreign] {
            assert_eq!(
                verify_run_action_history(&invalid, RunActionChain::Effects),
                Err(RunActionHistoryError::Invalid)
            );
        }
        // The declared form crosses JSON closed.
        let mut value = serde_json::to_value(&history).unwrap();
        assert_eq!(
            serde_json::from_value::<RunActionHistory>(value.clone()).unwrap(),
            history
        );
        value["complete"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RunActionHistory>(value).is_err());
    }

    #[test]
    fn export_selections_parse_closed() {
        assert_eq!(
            ActionHistoryExportSelection::parse("effects:1:3"),
            Some(ActionHistoryExportSelection {
                chain: RunActionChain::Effects,
                from_sequence: 1,
                to_sequence: 3,
            })
        );
        assert_eq!(
            ActionHistoryExportSelection::parse("job-control:2:2").map(|value| value.chain),
            Some(RunActionChain::JobControl)
        );
        assert_eq!(
            ActionHistoryExportSelection::parse("routes:1:1").map(|value| value.chain),
            Some(RunActionChain::Routes)
        );
        for invalid in [
            "",
            "effects",
            "effects:1",
            "effects:0:1",
            "effects:3:1",
            "effects:01:2",
            "effects:1:2:3",
            "effects:1:+2",
            "effects:1: 2",
            "memory:1:1",
            "route:1:1",
            "Routes:1:1",
            "Effects:1:1",
            "effects:1:513",
            "effects:1:9999999",
        ] {
            assert_eq!(
                ActionHistoryExportSelection::parse(invalid),
                None,
                "{invalid}"
            );
        }
    }

    #[test]
    fn an_export_prints_its_exact_redacted_bytes_in_both_formats() {
        let history = effects_history();
        let selection = ActionHistoryExportSelection {
            chain: RunActionChain::Effects,
            from_sequence: 2,
            to_sequence: 3,
        };
        let export = export_run_action_history(&history, selection).unwrap();
        assert_eq!((export.kept_count, export.expired_count), (2, 0));
        assert!(!export.write_enabled);
        let (stdout, stderr) = render_action_history_export(selection, Some(&history), true);
        assert!(stderr.is_empty());
        assert_eq!(stdout.lines().count(), 1);
        let row: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(row["type"], "action_history_export");
        assert_eq!(row["chain"], "effects");
        let document = row["document"].as_str().unwrap();
        assert_eq!(sha256_hex(document.as_bytes()), export.export_sha256);
        assert_eq!(row["export_sha256"], export.export_sha256);
        let parsed: serde_json::Value = serde_json::from_str(document).unwrap();
        assert_eq!(parsed["record_type"], "agentmage-action-history-export");
        assert_eq!(parsed["records"].as_array().unwrap().len(), 2);
        let (text, stderr) = render_action_history_export(selection, Some(&history), false);
        assert!(stderr.is_empty());
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("action_history_export chain=effects from=2 to=3 sha256="));
        assert!(lines[0].contains(&export.export_sha256));
        assert_eq!(lines[1].as_bytes(), export.bytes.as_slice());
        // A range past the chain, another owner's chain and an unavailable
        // history print a content-free notice on standard error only.
        for (selection, history, reason) in [
            (
                ActionHistoryExportSelection {
                    to_sequence: 4,
                    ..selection
                },
                Some(&history),
                "range",
            ),
            (
                ActionHistoryExportSelection {
                    chain: RunActionChain::JobControl,
                    ..selection
                },
                Some(&history),
                "unavailable",
            ),
            (selection, None, "unavailable"),
        ] {
            for json in [true, false] {
                let (stdout, notice) = render_action_history_export(selection, history, json);
                assert!(stdout.is_empty());
                assert!(notice.contains(reason), "{notice}");
                assert!(!notice.contains(OPERATION));
            }
        }
    }

    #[test]
    fn the_view_shows_each_entry_or_says_the_history_is_unavailable() {
        let history = effects_history();
        let text = render_run_action_history(RunActionChain::Effects, Some(&history));
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with("action history of this run's effects: 3 entries, head "));
        assert_eq!(
            lines[1],
            format!(
                "- 1 file_write {OPERATION} succeeded; grant grant-00000000000000000000000000000007; coding.approved.succeeded"
            )
        );
        assert_eq!(
            lines[2],
            format!(
                "- 2 file_write {OPERATION} denied; person decision {}; runtime.coding.user-denied",
                &digest('d')[..12]
            )
        );
        assert!(lines[3].ends_with(
            "failed; grant grant-00000000000000000000000000000007; coding.approved.failed"
        ));
        assert_eq!(
            render_run_action_history(RunActionChain::JobControl, None),
            "action history of this run's job control: unavailable; the host could not declare it completely\n"
        );
        let mut expired = history.clone();
        let mut replayed = verify_run_action_history(&expired, RunActionChain::Effects).unwrap();
        replayed.apply_retention(11 + RUN_ACTION_RETENTION_MS);
        expired.records = replayed.records().to_vec();
        let text = render_run_action_history(RunActionChain::Effects, Some(&expired));
        assert!(text.contains("- 1 expired: only its place is kept\n"));
        assert!(text.contains("- 2 expired: only its place is kept\n"));
        // A long history ends with an explicit truncation line.
        let facts = Facts::new(ExecutedEffectKind::NoEffect);
        let mut recorder = RunActionRecorder::new();
        for at in 0..MAX_RUN_ACTION_ENTRIES as u64 {
            recorder.record(executed_action_draft(
                &facts.executed(completed(OperationOutcome::Succeeded), at),
            ));
        }
        let long = recorder.declare().unwrap();
        let text = render_run_action_history(RunActionChain::Effects, Some(&long));
        assert!(text.len() <= MAX_RUN_ACTION_RENDER_BYTES + 128);
        assert!(
            text.lines()
                .last()
                .unwrap()
                .starts_with("... history truncated; ")
        );
    }
}
