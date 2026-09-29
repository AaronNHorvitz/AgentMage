//! Declares which coding-session effects can be reversed (Decision 0108).
//!
//! Classification only: this module grants nothing, performs no rollback and
//! never reports an effect as undone. A recoverable write is restored only by a
//! fresh, separately approved inverse write through the existing rollback tool.
//! Created files, commands and uncertain effects are surfaced for the user to
//! reconcile; a version-control reset is never presented as reversing them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::coding_history::{
    RetainedCodingChange, valid_identifier, valid_record_path, verify_retained_change,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EffectAssessment {
    /// Controlled operation identity.
    pub operation_id: String,
    /// Recoverability class.
    pub recoverability: Recoverability,
    /// Stable reason code.
    pub reason_code: &'static str,
}

/// Sealed declaration for one session; not an approval or a rollback result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecoverabilityReport {
    /// Report schema version.
    pub schema_version: u16,
    /// Owning durable session.
    pub session_id: String,
    /// Owning coding task.
    pub task_id: String,
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
    if effects.len() > MAX_RECOVERABILITY_EFFECTS
        || !valid_identifier(session_id)
        || !valid_identifier(task_id)
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
            reason_code,
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
        assessments,
        revert_order,
        fully_recoverable,
        requires_reconciliation,
        report_sha256: "0".repeat(64),
    };
    report.report_sha256 =
        hex_sha256(&serde_json::to_vec(&report).map_err(|_| RecoverabilityError::Invalid)?);
    Ok(report)
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
    let _ = writeln!(output, "recoverability: {summary}");
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
