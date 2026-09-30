//! Declares which coding-session effects can be reversed (Decision 0108).
//!
//! Classification only: this module grants nothing, performs no rollback and
//! never reports an effect as undone. A recoverable write is restored only by a
//! fresh, separately approved inverse write through the existing rollback tool.
//! Created files, commands and uncertain effects are surfaced for the user to
//! reconcile; a version-control reset is never presented as reversing them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    OperationOutcome, RuntimeArtifactRef, RuntimeRunRequest, StateChange, ToolResult,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_dispatch::PreparedNativeCodingCall;
use crate::coding_history::{
    CODING_CHANGE_RECORD_MEDIA_TYPE, CodingChangeRecord, RetainedCodingChange, valid_identifier,
    valid_record_path, verify_retained_change,
};

/// Largest number of effects assessed in one report.
pub const MAX_RECOVERABILITY_EFFECTS: usize = 512;
/// Largest rendered declaration; longer text ends with an explicit truncation line.
pub const MAX_RECOVERABILITY_RENDER_BYTES: usize = 64 * 1024;

/// One session effect supplied by the trusted runtime from its own receipts.
///
/// The seal on a write record is an unkeyed digest: it proves the record is
/// internally consistent, not who produced it. The caller builds this list from
/// the canonical artifact store and receipts it owns, never from client input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEffect {
    /// A retained structured write with its exact preimage.
    Write(Box<RetainedCodingChange>),
    /// A newly created file; deletion is not an admitted inverse operation.
    Create {
        /// Controlled operation identity.
        operation_id: String,
        /// Canonical workspace-relative path components.
        path: Vec<String>,
    },
    /// A registered command or validation run whose external effects are not tracked.
    Command {
        /// Controlled operation identity.
        operation_id: String,
    },
    /// An effect whose terminal outcome is uncertain.
    Uncertain {
        /// Controlled operation identity.
        operation_id: String,
    },
}

impl SessionEffect {
    fn operation_id(&self) -> &str {
        match self {
            Self::Write(change) => &change.record.operation_id,
            Self::Create { operation_id, .. }
            | Self::Command { operation_id }
            | Self::Uncertain { operation_id } => operation_id,
        }
    }
}

/// Closed recoverability class for one effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Recoverability {
    /// A fresh approved inverse write would restore this change's exact preimage.
    Recoverable,
    /// The file already holds this change's preimage; the change is not present.
    AlreadyReverted,
    /// Current bytes differ from what an inverse write expects; it would overwrite other edits.
    Conflict,
    /// No admitted inverse operation exists for this effect.
    NotRecoverable,
    /// External or uncertain effects that only the user can reconcile.
    ExternalOrUncertain,
}

/// Content-free assessment of one effect.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectAssessment {
    /// Controlled operation identity.
    pub operation_id: String,
    /// Recoverability class.
    pub recoverability: Recoverability,
    /// Stable reason code.
    pub reason_code: String,
}

/// Sealed declaration for one session, or for one run of it when `run_id` is
/// present; not an approval or a rollback result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoverabilityReport {
    /// Report schema version.
    pub schema_version: u16,
    /// Owning durable session.
    pub session_id: String,
    /// Owning coding task.
    pub task_id: String,
    /// The one run whose effects are declared, when the scope is a run
    /// (Decision 0116). Absent for a whole-session declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// One assessment per supplied effect, in input order.
    pub assessments: Vec<EffectAssessment>,
    /// Recoverable writes, newest first; inverse writes must follow this order.
    pub revert_order: Vec<String>,
    /// Whether every effect is recoverable or already reverted.
    pub fully_recoverable: bool,
    /// Whether any external or uncertain effect needs user reconciliation.
    pub requires_reconciliation: bool,
    /// Digest sealing every preceding field.
    pub report_sha256: String,
}

/// Content-free refusal to assess.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoverabilityError {
    /// A write record is invalid or belongs to another session or task.
    ForeignOrInvalid,
    /// An operation identity appears more than once.
    Duplicate,
    /// Too many effects, or an identity or path outside the record rules.
    Invalid,
}

/// Classifies every supplied effect. `current_sha256` reports the digest of a
/// path's current bytes, or `None` when the file is absent or unreadable.
pub fn assess_recoverability(
    session_id: &str,
    task_id: &str,
    effects: &[SessionEffect],
    current_sha256: &dyn Fn(&[String]) -> Option<String>,
) -> Result<RecoverabilityReport, RecoverabilityError> {
    assess(session_id, task_id, None, effects, current_sha256)
}

/// Classifies the effects of one run, declared by the run's effect owner
/// (Decision 0116). The report names the run, so it never reads as a
/// declaration about the whole session.
pub fn assess_run_recoverability(
    session_id: &str,
    task_id: &str,
    run_id: &str,
    effects: &[SessionEffect],
    current_sha256: &dyn Fn(&[String]) -> Option<String>,
) -> Result<RecoverabilityReport, RecoverabilityError> {
    assess(session_id, task_id, Some(run_id), effects, current_sha256)
}

fn assess(
    session_id: &str,
    task_id: &str,
    run_id: Option<&str>,
    effects: &[SessionEffect],
    current_sha256: &dyn Fn(&[String]) -> Option<String>,
) -> Result<RecoverabilityReport, RecoverabilityError> {
    if effects.len() > MAX_RECOVERABILITY_EFFECTS
        || !valid_identifier(session_id)
        || !valid_identifier(task_id)
        || run_id.is_some_and(|run_id| !valid_identifier(run_id))
    {
        return Err(RecoverabilityError::Invalid);
    }
    let mut seen = BTreeSet::new();
    let mut chains: BTreeMap<&[String], Vec<usize>> = BTreeMap::new();
    for (index, effect) in effects.iter().enumerate() {
        // The same bounded single-line rule as change records, so no identity
        // can add lines to or grow the rendered declaration.
        if !valid_identifier(effect.operation_id()) {
            return Err(RecoverabilityError::Invalid);
        }
        if let SessionEffect::Create { path, .. } = effect
            && !valid_record_path(path)
        {
            return Err(RecoverabilityError::Invalid);
        }
        if !seen.insert(effect.operation_id()) {
            return Err(RecoverabilityError::Duplicate);
        }
        if let SessionEffect::Write(change) = effect {
            verify_retained_change(change).map_err(|_| RecoverabilityError::ForeignOrInvalid)?;
            if change.record.session_id != session_id || change.record.task_id != task_id {
                return Err(RecoverabilityError::ForeignOrInvalid);
            }
            chains
                .entry(change.record.path.as_slice())
                .or_default()
                .push(index);
        }
    }
    let mut classes: Vec<Option<(Recoverability, &'static str)>> = vec![None; effects.len()];
    for (path, indexes) in &chains {
        // Walk newest to oldest: `expected` is what the file should hold once
        // every newer recoverable write on this path has been inverted.
        let mut expected = current_sha256(path);
        let mut blocked = None;
        for &index in indexes.iter().rev() {
            let SessionEffect::Write(change) = &effects[index] else {
                unreachable!("chains hold only writes")
            };
            let record = &change.record;
            let (class, next_expected, next_blocked) = if let Some(reason) = blocked {
                (
                    (Recoverability::Conflict, reason),
                    expected.clone(),
                    blocked,
                )
            } else {
                match expected.as_deref() {
                    None => (
                        (Recoverability::Conflict, "file-unavailable"),
                        None,
                        Some("file-unavailable"),
                    ),
                    Some(current) if current == record.postimage_sha256 => (
                        (
                            Recoverability::Recoverable,
                            "inverse-write-restores-preimage",
                        ),
                        Some(record.preimage_sha256.clone()),
                        None,
                    ),
                    Some(current) if current == record.preimage_sha256 => (
                        (Recoverability::AlreadyReverted, "preimage-already-present"),
                        expected.clone(),
                        None,
                    ),
                    Some(_) => (
                        (Recoverability::Conflict, "current-bytes-differ"),
                        expected.clone(),
                        Some("newer-conflict-blocks-older"),
                    ),
                }
            };
            classes[index] = Some(class);
            expected = next_expected;
            blocked = next_blocked;
        }
    }
    let mut assessments = Vec::with_capacity(effects.len());
    for (effect, class) in effects.iter().zip(classes) {
        let (recoverability, reason_code) = match effect {
            SessionEffect::Write(_) => class.expect("every write is classified"),
            SessionEffect::Create { .. } => (
                Recoverability::NotRecoverable,
                "create-has-no-admitted-inverse",
            ),
            SessionEffect::Command { .. } => (
                Recoverability::ExternalOrUncertain,
                "command-effects-not-tracked",
            ),
            SessionEffect::Uncertain { .. } => (
                Recoverability::ExternalOrUncertain,
                "effect-outcome-uncertain",
            ),
        };
        assessments.push(EffectAssessment {
            operation_id: effect.operation_id().to_owned(),
            recoverability,
            reason_code: reason_code.to_owned(),
        });
    }
    let revert_order = assessments
        .iter()
        .rev()
        .filter(|assessment| assessment.recoverability == Recoverability::Recoverable)
        .map(|assessment| assessment.operation_id.clone())
        .collect();
    let fully_recoverable = assessments.iter().all(|assessment| {
        matches!(
            assessment.recoverability,
            Recoverability::Recoverable | Recoverability::AlreadyReverted
        )
    });
    let requires_reconciliation = assessments
        .iter()
        .any(|assessment| assessment.recoverability == Recoverability::ExternalOrUncertain);
    let mut report = RecoverabilityReport {
        schema_version: 1,
        session_id: session_id.to_owned(),
        task_id: task_id.to_owned(),
        run_id: run_id.map(str::to_owned),
        assessments,
        revert_order,
        fully_recoverable,
        requires_reconciliation,
        report_sha256: "0".repeat(64),
    };
    report.report_sha256 = report_digest(&report)?;
    Ok(report)
}

fn report_digest(report: &RecoverabilityReport) -> Result<String, RecoverabilityError> {
    let mut unsealed = report.clone();
    unsealed.report_sha256 = "0".repeat(64);
    serde_json::to_vec(&unsealed)
        .map(|bytes| hex_sha256(&bytes))
        .map_err(|_| RecoverabilityError::Invalid)
}

/// Closed reason codes of an assessment, one set per class.
const fn reason_codes(class: Recoverability) -> &'static [&'static str] {
    match class {
        Recoverability::Recoverable => &["inverse-write-restores-preimage"],
        Recoverability::AlreadyReverted => &["preimage-already-present"],
        Recoverability::Conflict => &[
            "file-unavailable",
            "current-bytes-differ",
            "newer-conflict-blocks-older",
        ],
        Recoverability::NotRecoverable => &["create-has-no-admitted-inverse"],
        Recoverability::ExternalOrUncertain => {
            &["command-effects-not-tracked", "effect-outcome-uncertain"]
        }
    }
}

/// Verifies a declaration received from the host for one exact run: its seal,
/// scope, bounds, closed reason codes and the consistency of its summary
/// fields. It cannot show that the host listed every effect; the host owns
/// that, and the declaration grants nothing.
pub fn verify_run_recoverability(
    report: &RecoverabilityReport,
    session_id: &str,
    task_id: &str,
    run_id: &str,
) -> Result<(), RecoverabilityError> {
    let mut seen = BTreeSet::new();
    let valid_items = report.assessments.iter().all(|assessment| {
        valid_identifier(&assessment.operation_id)
            && seen.insert(assessment.operation_id.as_str())
            && reason_codes(assessment.recoverability).contains(&assessment.reason_code.as_str())
    });
    let revert_order = report
        .assessments
        .iter()
        .rev()
        .filter(|assessment| assessment.recoverability == Recoverability::Recoverable)
        .map(|assessment| assessment.operation_id.as_str())
        .collect::<Vec<_>>();
    if report.schema_version != 1
        || report.session_id != session_id
        || report.task_id != task_id
        || report.run_id.as_deref() != Some(run_id)
        || report.assessments.len() > MAX_RECOVERABILITY_EFFECTS
        || !valid_items
        || report.revert_order != revert_order
        || report.fully_recoverable
            != report.assessments.iter().all(|assessment| {
                matches!(
                    assessment.recoverability,
                    Recoverability::Recoverable | Recoverability::AlreadyReverted
                )
            })
        || report.requires_reconciliation
            != report
                .assessments
                .iter()
                .any(|assessment| assessment.recoverability == Recoverability::ExternalOrUncertain)
        || report.report_sha256 != report_digest(report)?
    {
        return Err(RecoverabilityError::Invalid);
    }
    Ok(())
}

/// What one executed native call can do to the workspace, from its prepared form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutedEffectKind {
    /// A structured patch, a selected write or an inverse write.
    Write,
    /// A controlled creation of the named path.
    Create {
        /// Canonical workspace-relative path components.
        path: Vec<String>,
    },
    /// A registered command or validation run.
    Command,
    /// A read, a Git inspection or a history read: no workspace effect.
    NoEffect,
}

impl ExecutedEffectKind {
    /// Classifies one prepared native call.
    #[must_use]
    pub fn of(prepared: &PreparedNativeCodingCall) -> Self {
        match prepared {
            PreparedNativeCodingCall::StructuredPatch { .. }
            | PreparedNativeCodingCall::Rollback { .. }
            | PreparedNativeCodingCall::HunkSelection { .. } => Self::Write,
            PreparedNativeCodingCall::ControlledCreate { proposal } => Self::Create {
                path: proposal.path.clone(),
            },
            PreparedNativeCodingCall::Command { .. }
            | PreparedNativeCodingCall::Validation { .. } => Self::Command,
            PreparedNativeCodingCall::ReadOnly { .. }
            | PreparedNativeCodingCall::GitInspection { .. }
            | PreparedNativeCodingCall::ChangeHistory { .. } => Self::NoEffect,
        }
    }
}

/// What the effect owner observed when one executed call ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutedEffectOutcome {
    /// The call returned a result with this outcome and whether state changed.
    Completed {
        /// Terminal outcome of the result.
        outcome: OperationOutcome,
        /// Whether the result reported a state change.
        changed: bool,
    },
    /// The call failed after its authority was consumed; its effect is unknown.
    Unknown,
}

/// One executed call as its effect owner recorded it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutedEffect {
    /// Runtime operation identity of the call.
    pub operation_id: String,
    /// What the call can do.
    pub kind: ExecutedEffectKind,
    /// How it ended.
    pub outcome: ExecutedEffectOutcome,
}

/// Builds one run's effect list from the effect owner's own execution records
/// and the change records it published, never from client input (Decision
/// 0116). A changed write whose record is missing, and any effectful call whose
/// outcome is unknown or uncertain, is surfaced as uncertain. A command counts
/// as an effect unless it was denied, because a failed, cancelled or timed-out
/// command may have run.
pub fn run_effects(
    executed: &[ExecutedEffect],
    changes: &[RetainedCodingChange],
) -> Result<Vec<SessionEffect>, RecoverabilityError> {
    if executed.len() > MAX_RECOVERABILITY_EFFECTS || changes.len() > MAX_RECOVERABILITY_EFFECTS {
        return Err(RecoverabilityError::Invalid);
    }
    let mut records = BTreeMap::new();
    for change in changes {
        verify_retained_change(change).map_err(|_| RecoverabilityError::ForeignOrInvalid)?;
        if records
            .insert(change.record.operation_id.as_str(), change)
            .is_some()
        {
            return Err(RecoverabilityError::Duplicate);
        }
    }
    let mut used = BTreeSet::new();
    let mut effects = Vec::new();
    for effect in executed {
        let operation_id = effect.operation_id.clone();
        let uncertain = || SessionEffect::Uncertain {
            operation_id: operation_id.clone(),
        };
        let declared = match (&effect.kind, effect.outcome) {
            (ExecutedEffectKind::NoEffect, _) => None,
            (_, ExecutedEffectOutcome::Unknown)
            | (
                _,
                ExecutedEffectOutcome::Completed {
                    outcome: OperationOutcome::Uncertain,
                    ..
                },
            ) => Some(uncertain()),
            (
                ExecutedEffectKind::Write,
                ExecutedEffectOutcome::Completed {
                    outcome: OperationOutcome::Succeeded,
                    changed: true,
                },
            ) => Some(
                records
                    .get(effect.operation_id.as_str())
                    .map_or_else(uncertain, |change| {
                        used.insert(effect.operation_id.as_str());
                        SessionEffect::Write(Box::new((*change).clone()))
                    }),
            ),
            (
                ExecutedEffectKind::Create { path },
                ExecutedEffectOutcome::Completed {
                    outcome: OperationOutcome::Succeeded,
                    ..
                },
            ) => Some(SessionEffect::Create {
                operation_id: operation_id.clone(),
                path: path.clone(),
            }),
            (ExecutedEffectKind::Command, ExecutedEffectOutcome::Completed { outcome, .. })
                if outcome != OperationOutcome::Denied =>
            {
                Some(SessionEffect::Command {
                    operation_id: operation_id.clone(),
                })
            }
            _ => None,
        };
        effects.extend(declared);
    }
    // Every published record belongs to one changed write of this run.
    if used.len() != records.len() {
        return Err(RecoverabilityError::ForeignOrInvalid);
    }
    Ok(effects)
}

impl ExecutedEffectOutcome {
    /// How one execution ended: its result, or an error after its authority
    /// was consumed, which leaves the effect unknown.
    #[must_use]
    pub fn of<E>(result: Result<&ToolResult, E>) -> Self {
        result.map_or(Self::Unknown, |result| Self::Completed {
            outcome: result.outcome,
            changed: result.state_change == StateChange::Changed,
        })
    }
}

/// One run's execution and change records, kept by the effect owner that
/// executed the calls and published the records (Decision 0116).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunEffectRecorder {
    executed: Vec<ExecutedEffect>,
    changes: Vec<RetainedCodingChange>,
    complete: bool,
}

impl Default for RunEffectRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl RunEffectRecorder {
    /// Starts an empty, complete record.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            executed: Vec::new(),
            changes: Vec::new(),
            complete: true,
        }
    }

    /// Records how one executed call ended.
    pub fn record_execution(&mut self, effect: ExecutedEffect) {
        if self.executed.len() < MAX_RECOVERABILITY_EFFECTS {
            self.executed.push(effect);
        } else {
            self.complete = false;
        }
    }

    /// Records one published artifact; only a verified change record is kept.
    /// A change record that does not verify makes the record incomplete.
    pub fn record_publication(&mut self, reference: &RuntimeArtifactRef, payload: &[u8]) {
        if reference.media_type != CODING_CHANGE_RECORD_MEDIA_TYPE {
            return;
        }
        let retained = serde_json::from_slice::<CodingChangeRecord>(payload)
            .ok()
            .map(|record| RetainedCodingChange {
                reference: reference.clone(),
                record,
            })
            .filter(|retained| verify_retained_change(retained).is_ok());
        match retained {
            Some(retained) if self.changes.len() < MAX_RECOVERABILITY_EFFECTS => {
                self.changes.push(retained);
            }
            _ => self.complete = false,
        }
    }

    /// Declares the recorded run. An incomplete record is never declared, so
    /// a declaration cannot silently omit an effect.
    pub fn declare(
        &self,
        session_id: &str,
        task_id: &str,
        run_id: &str,
        current_sha256: &dyn Fn(&[String]) -> Option<String>,
    ) -> Result<RecoverabilityReport, RecoverabilityError> {
        if !self.complete {
            return Err(RecoverabilityError::Invalid);
        }
        let effects = run_effects(&self.executed, &self.changes)?;
        assess_run_recoverability(session_id, task_id, run_id, &effects, current_sha256)
    }
}

/// A trusted effect owner that can declare the recoverability of one ended run.
pub trait RunRecoverabilitySource {
    /// Declares the ended run's effects from the owner's own records.
    fn declare_run_recoverability(
        &self,
        request: &RuntimeRunRequest,
    ) -> Result<RecoverabilityReport, RecoverabilityError>;
}

/// Bounded user-facing text. It states how each effect could be reversed, if
/// at all, and never describes an effect as undone.
#[must_use]
pub fn render_recoverability(report: &RecoverabilityReport) -> String {
    let mut output = String::new();
    let summary = if report.assessments.is_empty() {
        "no session effects were recorded; nothing needs restoring or reconciling"
    } else if report.fully_recoverable {
        "every session change can be restored by fresh approved inverse writes, newest first"
    } else if report.requires_reconciliation {
        "some effects are external or uncertain and need your reconciliation; no reset reverses them"
    } else {
        "some changes cannot be restored by an inverse write"
    };
    if report.run_id.is_some() {
        // A run declaration covers only that run's effects (Decision 0116).
        let summary = summary.replacen("session ", "", 1);
        let _ = writeln!(output, "recoverability of this run's effects: {summary}");
    } else {
        let _ = writeln!(output, "recoverability: {summary}");
    }
    for (index, assessment) in report.assessments.iter().enumerate() {
        let class = match assessment.recoverability {
            Recoverability::Recoverable => "recoverable by fresh inverse write",
            Recoverability::AlreadyReverted => "preimage already present",
            Recoverability::Conflict => {
                "conflict: current file differs; resolve before any inverse write"
            }
            Recoverability::NotRecoverable => "not recoverable by an admitted operation",
            Recoverability::ExternalOrUncertain => "external or uncertain: reconcile manually",
        };
        let line = format!(
            "- {} {} ({})\n",
            assessment.operation_id, class, assessment.reason_code
        );
        if output.len() + line.len() > MAX_RECOVERABILITY_RENDER_BYTES {
            let _ = writeln!(
                output,
                "... declaration truncated; {} of {} effects not shown",
                report.assessments.len() - index,
                report.assessments.len()
            );
            break;
        }
        output.push_str(&line);
    }
    output
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding_history::{
        CODING_CHANGE_RECORD_MEDIA_TYPE, CodingChangeRecord, seal_change_record,
    };
    use agentmage_capability_repository_map::{StructuredArtifactClass, StructuredLanguage};
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, RuntimeArtifactId, RuntimeArtifactRef,
    };
    use std::collections::HashMap;

    const SESSION: &str = "session-fixture";
    const TASK: &str = "task-fixture";

    fn digest(text: &str) -> String {
        hex_sha256(text.as_bytes())
    }

    fn write(operation: &str, path: &[&str], before: &str, after: &str) -> SessionEffect {
        let record = seal_change_record(CodingChangeRecord {
            schema_version: 1,
            record_id: format!("change-record:{operation}"),
            session_id: SESSION.to_owned(),
            task_id: TASK.to_owned(),
            producer_run_id: "run-fixture".to_owned(),
            operation_id: operation.to_owned(),
            path: path.iter().map(|part| (*part).to_owned()).collect(),
            language: StructuredLanguage::Python,
            artifact_class: StructuredArtifactClass::Code,
            preimage: before.to_owned(),
            preimage_sha256: digest(before),
            postimage_sha256: digest(after),
            generated: false,
            receipt_id: format!("receipt-{operation}"),
            receipt_sha256: digest(operation),
            created_at_epoch_ms: 1,
            record_sha256: "0".repeat(64),
        })
        .unwrap();
        let bytes = serde_json::to_vec(&record).unwrap();
        SessionEffect::Write(Box::new(RetainedCodingChange {
            reference: RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw(format!("artifact:{operation}")),
                manifest_sha256: digest("manifest"),
                payload_sha256: hex_sha256(&bytes),
                byte_size: bytes.len() as u64,
                media_type: CODING_CHANGE_RECORD_MEDIA_TYPE.to_owned(),
            },
            record,
        }))
    }

    fn files(entries: &[(&[&str], &str)]) -> HashMap<Vec<String>, String> {
        entries
            .iter()
            .map(|(path, text)| {
                (
                    path.iter().map(|part| (*part).to_owned()).collect(),
                    digest(text),
                )
            })
            .collect()
    }

    fn assess(
        effects: &[SessionEffect],
        current: &HashMap<Vec<String>, String>,
    ) -> Result<RecoverabilityReport, RecoverabilityError> {
        assess_recoverability(SESSION, TASK, effects, &|path| current.get(path).cloned())
    }

    fn classes(report: &RecoverabilityReport) -> Vec<(String, Recoverability)> {
        report
            .assessments
            .iter()
            .map(|item| (item.operation_id.clone(), item.recoverability))
            .collect()
    }

    const CALC: &[&str] = &["src", "calc.py"];

    fn retained(effect: SessionEffect) -> RetainedCodingChange {
        match effect {
            SessionEffect::Write(change) => *change,
            _ => unreachable!("fixture builds writes"),
        }
    }

    fn executed(
        operation: &str,
        kind: ExecutedEffectKind,
        outcome: ExecutedEffectOutcome,
    ) -> ExecutedEffect {
        ExecutedEffect {
            operation_id: operation.to_owned(),
            kind,
            outcome,
        }
    }

    const fn completed(outcome: OperationOutcome, changed: bool) -> ExecutedEffectOutcome {
        ExecutedEffectOutcome::Completed { outcome, changed }
    }

    #[test]
    fn a_run_effect_list_comes_from_executed_calls_and_their_published_records() {
        // Decision 0116: the effect owner's own execution records, paired with
        // the change records it published, give the run's effects.
        let new_file = vec!["src".to_owned(), "new.py".to_owned()];
        let executed = [
            executed(
                "read",
                ExecutedEffectKind::NoEffect,
                completed(OperationOutcome::Succeeded, false),
            ),
            executed(
                "w1",
                ExecutedEffectKind::Write,
                completed(OperationOutcome::Succeeded, true),
            ),
            executed(
                "same",
                ExecutedEffectKind::Write,
                completed(OperationOutcome::Succeeded, false),
            ),
            executed(
                "refused",
                ExecutedEffectKind::Write,
                completed(OperationOutcome::Denied, false),
            ),
            executed(
                "create",
                ExecutedEffectKind::Create {
                    path: new_file.clone(),
                },
                completed(OperationOutcome::Succeeded, true),
            ),
            executed(
                "failed-create",
                ExecutedEffectKind::Create {
                    path: new_file.clone(),
                },
                completed(OperationOutcome::Failed, false),
            ),
            executed(
                "test",
                ExecutedEffectKind::Command,
                completed(OperationOutcome::Failed, false),
            ),
            executed(
                "denied-test",
                ExecutedEffectKind::Command,
                completed(OperationOutcome::Denied, false),
            ),
            executed(
                "lost",
                ExecutedEffectKind::Command,
                ExecutedEffectOutcome::Unknown,
            ),
            executed(
                "unsure",
                ExecutedEffectKind::Write,
                completed(OperationOutcome::Uncertain, false),
            ),
            executed(
                "no-record",
                ExecutedEffectKind::Write,
                completed(OperationOutcome::Succeeded, true),
            ),
            executed(
                "read-lost",
                ExecutedEffectKind::NoEffect,
                ExecutedEffectOutcome::Unknown,
            ),
        ];
        let change = retained(write("w1", CALC, "v0\n", "v1\n"));
        let effects = run_effects(&executed, std::slice::from_ref(&change)).unwrap();
        let ids = effects
            .iter()
            .map(SessionEffect::operation_id)
            .collect::<Vec<_>>();
        assert_eq!(ids, ["w1", "create", "test", "lost", "unsure", "no-record"]);
        assert!(matches!(&effects[0], SessionEffect::Write(found) if **found == change));
        assert!(matches!(&effects[1], SessionEffect::Create { path, .. } if *path == new_file));
        assert!(matches!(effects[2], SessionEffect::Command { .. }));
        assert!(
            effects[3..]
                .iter()
                .all(|effect| matches!(effect, SessionEffect::Uncertain { .. }))
        );

        let report = assess_run_recoverability(SESSION, TASK, "run-fixture", &effects, &|path| {
            (path == CALC).then(|| digest("v1\n"))
        })
        .unwrap();
        assert_eq!(report.run_id.as_deref(), Some("run-fixture"));
        assert_eq!(report.revert_order, ["w1"]);
        assert!(report.requires_reconciliation);
        verify_run_recoverability(&report, SESSION, TASK, "run-fixture").unwrap();
        let text = render_recoverability(&report);
        assert!(text.starts_with("recoverability of this run's effects: "));
        assert!(!text.contains("undone"));
        // The declaration crosses the host boundary: it round-trips exactly.
        let decoded: RecoverabilityReport =
            serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert_eq!(decoded, report);

        // A record for no changed write of this run, or twice, is refused.
        assert_eq!(
            run_effects(&executed[..1], std::slice::from_ref(&change)),
            Err(RecoverabilityError::ForeignOrInvalid)
        );
        assert_eq!(
            run_effects(&executed, &[change.clone(), change.clone()]),
            Err(RecoverabilityError::Duplicate)
        );
        let mut tampered = change;
        tampered.record.preimage.push('x');
        assert_eq!(
            run_effects(&executed, &[tampered]),
            Err(RecoverabilityError::ForeignOrInvalid)
        );
    }

    #[test]
    fn the_effect_recorder_declares_only_a_complete_record_of_its_own_run() {
        let change = retained(write("w1", CALC, "v0\n", "v1\n"));
        let payload = serde_json::to_vec(&change.record).unwrap();
        let result = |outcome, state_change| ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: agentmage_kernel_contracts::ToolCallId::from_raw("call"),
            correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw("correlation"),
            outcome,
            output: None,
            validation_issues: Vec::new(),
            evidence: Vec::new(),
            error: None,
            elapsed_ms: 1,
            state_change,
        };
        assert_eq!(
            ExecutedEffectOutcome::of::<()>(Ok(&result(
                OperationOutcome::Succeeded,
                StateChange::Changed
            ))),
            completed(OperationOutcome::Succeeded, true)
        );
        assert_eq!(
            ExecutedEffectOutcome::of(Err::<&ToolResult, _>(())),
            ExecutedEffectOutcome::Unknown
        );
        let mut recorder = RunEffectRecorder::new();
        recorder.record_execution(executed(
            "w1",
            ExecutedEffectKind::Write,
            ExecutedEffectOutcome::of::<()>(Ok(&result(
                OperationOutcome::Succeeded,
                StateChange::Changed,
            ))),
        ));
        recorder.record_execution(executed(
            "t1",
            ExecutedEffectKind::Command,
            ExecutedEffectOutcome::Unknown,
        ));
        // Other artifacts are not change records and are ignored.
        let mut other = change.reference.clone();
        other.media_type = "text/plain".to_owned();
        recorder.record_publication(&other, b"not a record");
        recorder.record_publication(&change.reference, &payload);
        let current = |path: &[String]| (path == CALC).then(|| digest("v1\n"));
        let report = recorder
            .declare(SESSION, TASK, "run-fixture", &current)
            .unwrap();
        assert_eq!(
            classes(&report),
            [
                ("w1".to_owned(), Recoverability::Recoverable),
                ("t1".to_owned(), Recoverability::ExternalOrUncertain)
            ]
        );

        // A change record that does not verify, or too many executions, makes
        // the record incomplete and nothing is declared.
        let mut corrupt = recorder.clone();
        corrupt.record_publication(&change.reference, b"{}");
        assert!(
            corrupt
                .declare(SESSION, TASK, "run-fixture", &current)
                .is_err()
        );
        let mut full = RunEffectRecorder::new();
        for index in 0..=MAX_RECOVERABILITY_EFFECTS {
            full.record_execution(executed(
                &format!("read-{index}"),
                ExecutedEffectKind::NoEffect,
                completed(OperationOutcome::Succeeded, false),
            ));
        }
        assert!(
            full.declare(SESSION, TASK, "run-fixture", &current)
                .is_err()
        );
        // A recorded changed write without its published record is uncertain.
        let mut unrecorded = RunEffectRecorder::default();
        unrecorded.record_execution(executed(
            "w9",
            ExecutedEffectKind::Write,
            completed(OperationOutcome::Succeeded, true),
        ));
        let report = unrecorded
            .declare(SESSION, TASK, "run-fixture", &current)
            .unwrap();
        assert_eq!(
            classes(&report),
            [("w9".to_owned(), Recoverability::ExternalOrUncertain)]
        );
    }

    #[test]
    fn a_received_run_declaration_must_match_its_run_seal_and_summary() {
        let effects = [
            write("w1", CALC, "v0\n", "v1\n"),
            SessionEffect::Command {
                operation_id: "cmd1".to_owned(),
            },
        ];
        let report = assess_run_recoverability(SESSION, TASK, "run-fixture", &effects, &|_| {
            Some(digest("v1\n"))
        })
        .unwrap();
        verify_run_recoverability(&report, SESSION, TASK, "run-fixture").unwrap();
        for (session, task, run) in [
            ("session-other", TASK, "run-fixture"),
            (SESSION, "task-other", "run-fixture"),
            (SESSION, TASK, "run-other"),
        ] {
            assert!(verify_run_recoverability(&report, session, task, run).is_err());
        }
        let session_scope =
            assess_recoverability(SESSION, TASK, &effects, &|_| Some(digest("v1\n"))).unwrap();
        assert!(session_scope.run_id.is_none());
        assert!(verify_run_recoverability(&session_scope, SESSION, TASK, "run-fixture").is_err());
        // A changed seal is refused; every other change stays refused even
        // after resealing, because summaries must follow the assessments and
        // reason codes come from a closed set.
        let mut resealed = report.clone();
        resealed.report_sha256 = "f".repeat(64);
        assert!(verify_run_recoverability(&resealed, SESSION, TASK, "run-fixture").is_err());
        let mutations: [fn(&mut RecoverabilityReport); 6] = [
            |report| report.assessments[0].recoverability = Recoverability::AlreadyReverted,
            |report| report.assessments[1].reason_code = "reset-reverses-it".to_owned(),
            |report| report.fully_recoverable = true,
            |report| report.requires_reconciliation = false,
            |report| report.revert_order.clear(),
            |report| report.assessments[1].operation_id = "w1".to_owned(),
        ];
        for mutate in mutations {
            let mut mutated = report.clone();
            mutate(&mut mutated);
            assert!(verify_run_recoverability(&mutated, SESSION, TASK, "run-fixture").is_err());
            mutated.report_sha256 = report_digest(&mutated).unwrap();
            assert!(verify_run_recoverability(&mutated, SESSION, TASK, "run-fixture").is_err());
        }
        let mut unknown: serde_json::Value = serde_json::to_value(&report).unwrap();
        unknown["undone"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RecoverabilityReport>(unknown).is_err());
    }

    #[test]
    fn a_write_chain_is_recoverable_newest_first_while_the_file_holds_its_postimage() {
        let effects = [
            write("w1", CALC, "v0\n", "v1\n"),
            write("w2", CALC, "v1\n", "v2\n"),
            write("w3", CALC, "v2\n", "v3\n"),
        ];
        let report = assess(&effects, &files(&[(CALC, "v3\n")])).unwrap();
        assert!(
            classes(&report)
                .iter()
                .all(|(_, class)| *class == Recoverability::Recoverable)
        );
        assert_eq!(report.revert_order, ["w3", "w2", "w1"]);
        assert!(report.fully_recoverable);
        assert!(!report.requires_reconciliation);
        let text = render_recoverability(&report);
        assert!(text.contains("newest first"));
        assert!(!text.contains("undone"));
    }

    #[test]
    fn a_human_edit_or_missing_file_is_a_conflict_that_blocks_older_writes() {
        let effects = [
            write("w1", CALC, "v0\n", "v1\n"),
            write("w2", CALC, "v1\n", "v2\n"),
        ];
        for current in [files(&[(CALC, "v2 plus a human edit\n")]), files(&[])] {
            let report = assess(&effects, &current).unwrap();
            assert_eq!(
                classes(&report),
                [
                    ("w1".to_owned(), Recoverability::Conflict),
                    ("w2".to_owned(), Recoverability::Conflict)
                ]
            );
            assert!(report.revert_order.is_empty());
            assert!(!report.fully_recoverable);
        }
        let report = assess(&effects, &files(&[(CALC, "human\n")])).unwrap();
        assert_eq!(report.assessments[1].reason_code, "current-bytes-differ");
        assert_eq!(
            report.assessments[0].reason_code,
            "newer-conflict-blocks-older"
        );
    }

    #[test]
    fn already_reverted_and_broken_chains_are_distinguished() {
        let effects = [
            write("w1", CALC, "v0\n", "v1\n"),
            write("w2", CALC, "v1\n", "v2\n"),
        ];
        // Someone already restored w2's preimage; w1 remains recoverable.
        let report = assess(&effects, &files(&[(CALC, "v1\n")])).unwrap();
        assert_eq!(
            classes(&report),
            [
                ("w1".to_owned(), Recoverability::Recoverable),
                ("w2".to_owned(), Recoverability::AlreadyReverted)
            ]
        );
        assert!(report.fully_recoverable);
        assert_eq!(report.revert_order, ["w1"]);
        // A gap between writes: w2 is recoverable, w1's postimage never matches.
        let effects = [
            write("w1", CALC, "v0\n", "v1\n"),
            write("w2", CALC, "other\n", "v2\n"),
        ];
        let report = assess(&effects, &files(&[(CALC, "v2\n")])).unwrap();
        assert_eq!(
            classes(&report),
            [
                ("w1".to_owned(), Recoverability::Conflict),
                ("w2".to_owned(), Recoverability::Recoverable)
            ]
        );
        assert!(!report.fully_recoverable);
    }

    #[test]
    fn creates_commands_and_uncertain_effects_are_never_claimed_recoverable() {
        let effects = [
            write("w1", CALC, "v0\n", "v1\n"),
            SessionEffect::Create {
                operation_id: "c1".to_owned(),
                path: vec!["src".to_owned(), "new.py".to_owned()],
            },
            SessionEffect::Command {
                operation_id: "cmd1".to_owned(),
            },
            SessionEffect::Uncertain {
                operation_id: "u1".to_owned(),
            },
        ];
        let report = assess(&effects, &files(&[(CALC, "v1\n")])).unwrap();
        assert_eq!(
            classes(&report)[1..],
            [
                ("c1".to_owned(), Recoverability::NotRecoverable),
                ("cmd1".to_owned(), Recoverability::ExternalOrUncertain),
                ("u1".to_owned(), Recoverability::ExternalOrUncertain)
            ]
        );
        assert!(!report.fully_recoverable);
        assert!(report.requires_reconciliation);
        assert_eq!(report.revert_order, ["w1"]);
        let text = render_recoverability(&report);
        assert!(text.contains("no reset reverses them"));
        assert!(text.contains("reconcile manually"));
        assert!(!text.contains("undone"));
    }

    #[test]
    fn independent_paths_foreign_records_duplicates_and_bounds_are_checked() {
        let other: &[&str] = &["src", "other.py"];
        let effects = [
            write("w1", CALC, "a\n", "b\n"),
            write("w2", other, "c\n", "d\n"),
        ];
        let report = assess(&effects, &files(&[(CALC, "b\n"), (other, "changed\n")])).unwrap();
        assert_eq!(
            report.assessments[0].recoverability,
            Recoverability::Recoverable
        );
        assert_eq!(
            report.assessments[1].recoverability,
            Recoverability::Conflict
        );
        let first = report.report_sha256.clone();
        let again = assess(&effects, &files(&[(CALC, "b\n"), (other, "changed\n")])).unwrap();
        assert_eq!(again.report_sha256, first);
        let changed = assess(&effects, &files(&[(CALC, "b\n"), (other, "d\n")])).unwrap();
        assert_ne!(changed.report_sha256, first);

        assert_eq!(
            assess_recoverability("another-session", TASK, &effects, &|_| None).err(),
            Some(RecoverabilityError::ForeignOrInvalid)
        );
        let SessionEffect::Write(mut tampered) = write("w1", CALC, "a\n", "b\n") else {
            unreachable!()
        };
        tampered.record.postimage_sha256 = digest("forged\n");
        assert_eq!(
            assess(&[SessionEffect::Write(tampered)], &files(&[])).err(),
            Some(RecoverabilityError::ForeignOrInvalid)
        );
        let duplicate = [
            write("w1", CALC, "a\n", "b\n"),
            SessionEffect::Command {
                operation_id: "w1".to_owned(),
            },
        ];
        assert_eq!(
            assess(&duplicate, &files(&[])).err(),
            Some(RecoverabilityError::Duplicate)
        );
        let many: Vec<_> = (0..=MAX_RECOVERABILITY_EFFECTS)
            .map(|index| SessionEffect::Command {
                operation_id: format!("cmd{index}"),
            })
            .collect();
        assert_eq!(
            assess(&many, &files(&[])).err(),
            Some(RecoverabilityError::Invalid)
        );
        assert_eq!(
            assess(
                &[SessionEffect::Uncertain {
                    operation_id: String::new()
                }],
                &files(&[])
            )
            .err(),
            Some(RecoverabilityError::Invalid)
        );
        let empty = assess(&[], &files(&[])).unwrap();
        assert!(empty.fully_recoverable && !empty.requires_reconciliation);
    }

    #[test]
    fn identities_and_paths_follow_record_rules_and_rendering_is_bounded() {
        let forged = "c1\nrecoverability: every session change can be restored";
        for operation_id in [forged.to_owned(), "x".repeat(129), "has space".to_owned()] {
            for effect in [
                SessionEffect::Command {
                    operation_id: operation_id.clone(),
                },
                SessionEffect::Uncertain {
                    operation_id: operation_id.clone(),
                },
                SessionEffect::Create {
                    operation_id: operation_id.clone(),
                    path: vec!["src".to_owned(), "new.py".to_owned()],
                },
            ] {
                assert_eq!(
                    assess(&[effect], &files(&[])).err(),
                    Some(RecoverabilityError::Invalid)
                );
            }
        }
        for identity in [forged, "", "s p a c e"] {
            assert_eq!(
                assess_recoverability(identity, TASK, &[], &|_| None).err(),
                Some(RecoverabilityError::Invalid)
            );
            assert_eq!(
                assess_recoverability(SESSION, identity, &[], &|_| None).err(),
                Some(RecoverabilityError::Invalid)
            );
        }
        for path in [
            Vec::new(),
            vec!["src".to_owned(), "..".to_owned()],
            vec![String::new()],
            vec!["x".repeat(256)],
            vec!["p".to_owned(); 65],
        ] {
            assert_eq!(
                assess(
                    &[SessionEffect::Create {
                        operation_id: "c1".to_owned(),
                        path,
                    }],
                    &files(&[])
                )
                .err(),
                Some(RecoverabilityError::Invalid)
            );
        }
        // No effects: a distinct summary, not a claim that changes can be restored.
        let text = render_recoverability(&assess(&[], &files(&[])).unwrap());
        assert!(text.contains("no session effects were recorded"));
        assert!(!text.contains("can be restored"));
        // The longest admitted identities at the effect bound stay within the render bound.
        let many: Vec<_> = (0..MAX_RECOVERABILITY_EFFECTS)
            .map(|index| SessionEffect::Uncertain {
                operation_id: format!("{index:0>128}"),
            })
            .collect();
        let text = render_recoverability(&assess(&many, &files(&[])).unwrap());
        assert!(
            text.len() <= MAX_RECOVERABILITY_RENDER_BYTES + 96,
            "{}",
            text.len()
        );
        assert!(text.ends_with(" effects not shown\n"));
        assert_eq!(
            text.lines().count() - 2,
            text.matches("reconcile manually").count()
        );
    }
}
