//! Auditable action history (Decision 0124).
//!
//! Each entry records who or what authorized one action, the digest of the
//! effect's identity, its outcome, a stable reason code and the digests of its
//! evidence. Entries carry closed fields, identifiers and digests only: an
//! identifier that is not a plain identifier, or that the shared secret
//! detector flags, is replaced before it is kept, so the history never holds a
//! secret or a raw prompt. Entries form a hash chain that a restarted owner
//! replays exactly, refusing any kept entry that `append` could not have
//! kept. An entry past its retention deadline keeps only its place in the
//! chain; its deadline goes with its content, because no digest could bind it
//! once the content is gone. A manual export is a redacted, bounded byte image that is
//! scanned once more before it is returned; it has no path and writes nothing.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::persistence::detect_secret_classes;

const SCHEMA_VERSION: u16 = 1;
const MAX_ENTRIES: usize = 65_536;
const MAX_EVIDENCE: usize = 32;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_EXPORT_BYTES: usize = 8 * 1024 * 1024;
const REDACTED: &str = "[REDACTED]";
const GENESIS_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Closed kind of an audited action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// A tool call.
    ToolCall,
    /// A file write.
    FileWrite,
    /// A command run.
    CommandRun,
    /// An outbound network request.
    NetworkRequest,
    /// A model route selection.
    ModelRoute,
    /// A memory change.
    MemoryChange,
    /// An extension lifecycle change.
    ExtensionLifecycle,
    /// A job control request.
    JobControl,
}

/// What authorized an action. Parsing is closed: the variant without
/// authority has no fields rather than being a unit variant, so a member
/// beside its tag is refused; its encoding is the same (Decision 0127).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionAuthorization {
    /// A kernel grant.
    Grant {
        /// Grant identity.
        grant_id: String,
        /// Digest of the exact grant.
        grant_sha256: String,
    },
    /// A person's recorded decision.
    PersonDecision {
        /// Digest of the decision record.
        decision_sha256: String,
    },
    /// Nothing authorized it; only a denied or cancelled action may say so.
    Unauthorized {},
}

/// Closed outcome of an audited action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    /// The effect happened as authorized.
    Succeeded,
    /// The action failed; its effect may be partial.
    Failed,
    /// Policy refused it before any effect.
    Denied,
    /// It was cancelled before any effect.
    Cancelled,
    /// Whether the effect happened is not known.
    Uncertain,
}

impl ActionOutcome {
    const fn needs_authorization(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Uncertain)
    }
}

/// What the owner of an action reports, before redaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionRecordDraft {
    /// Closed kind.
    pub action_kind: ActionKind,
    /// The owner's identity for the action.
    pub action_id: String,
    /// What authorized it.
    pub authorization: ActionAuthorization,
    /// Digest of the effect's identity, computed by its owner.
    pub effect_sha256: String,
    /// Closed outcome.
    pub outcome: ActionOutcome,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Digests of the evidence records, sorted and unique.
    pub evidence_sha256s: Vec<String>,
    /// When the owner recorded it.
    pub recorded_at_epoch_ms: u64,
    /// Exclusive retention deadline.
    pub retain_until_epoch_ms: u64,
}

/// One kept entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionHistoryEntry {
    /// Entry schema version.
    pub schema_version: u16,
    /// One-based position in the chain.
    pub sequence: u64,
    /// Closed kind.
    pub action_kind: ActionKind,
    /// Action identity, or the redaction marker.
    pub action_id: String,
    /// Authorization, with a redacted grant identity if it was replaced.
    pub authorization: ActionAuthorization,
    /// Digest of the effect's identity.
    pub effect_sha256: String,
    /// Closed outcome.
    pub outcome: ActionOutcome,
    /// Reason code, or the redaction marker.
    pub reason_code: String,
    /// Evidence digests.
    pub evidence_sha256s: Vec<String>,
    /// When the owner recorded it.
    pub recorded_at_epoch_ms: u64,
    /// Exclusive retention deadline.
    pub retain_until_epoch_ms: u64,
    /// How many fields were replaced before the entry was kept.
    pub redacted_fields: u16,
    /// Digest of the previous entry, or the genesis digest.
    pub previous_entry_sha256: String,
    /// Digest of this entry with this field zeroed.
    pub entry_sha256: String,
}

/// One position of the chain: a kept entry, or the place of an entry whose
/// retention deadline passed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionHistoryRecord {
    /// The entry is kept.
    Kept(ActionHistoryEntry),
    /// The entry's retention deadline passed; only its place remains.
    Expired {
        /// Position in the chain.
        sequence: u64,
        /// Digest of the previous entry.
        previous_entry_sha256: String,
        /// Digest the entry had.
        entry_sha256: String,
    },
}

impl ActionHistoryRecord {
    const fn sequence(&self) -> u64 {
        match self {
            Self::Kept(entry) => entry.sequence,
            Self::Expired { sequence, .. } => *sequence,
        }
    }

    fn previous(&self) -> &str {
        match self {
            Self::Kept(entry) => &entry.previous_entry_sha256,
            Self::Expired {
                previous_entry_sha256,
                ..
            } => previous_entry_sha256,
        }
    }

    fn digest(&self) -> &str {
        match self {
            Self::Kept(entry) => &entry.entry_sha256,
            Self::Expired { entry_sha256, .. } => entry_sha256,
        }
    }
}

/// The retained end of the chain, kept by the owner with every append.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionHistoryHead {
    /// Number of positions.
    pub count: u64,
    /// Digest of the last entry, or the genesis digest.
    pub head_sha256: String,
}

/// Which positions a manual export covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionHistoryExportRequest {
    /// First position, one-based.
    pub from_sequence: u64,
    /// Last position, inclusive.
    pub to_sequence: u64,
}

/// A redacted manual export: bytes only, with no path and no write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionHistoryExport {
    /// Canonical JSON of the exported positions.
    pub bytes: Vec<u8>,
    /// Digest of the bytes.
    pub export_sha256: String,
    /// Kept entries exported.
    pub kept_count: u64,
    /// Expired positions exported as places only.
    pub expired_count: u64,
    /// Fields replaced across the exported entries.
    pub redacted_fields: u64,
    /// Fixed false write marker.
    pub write_enabled: bool,
}

/// Content-free action history failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionHistoryError {
    /// A digest, bound, time, order or combination is malformed.
    InvalidInput,
    /// An action that may have had an effect names no authorization.
    AuthorizationMissing,
    /// The history is full.
    Full,
    /// Retained positions do not form this exact chain.
    Integrity,
    /// An export still contained a secret signature and was withheld.
    SecretDetected,
}

/// Append-only action history with one owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionHistory {
    records: Vec<ActionHistoryRecord>,
    last_recorded_at_epoch_ms: u64,
}

impl Default for ActionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionHistory {
    /// An empty history.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            records: Vec::new(),
            last_recorded_at_epoch_ms: 0,
        }
    }

    /// Redacts and appends one action. Times never go backwards.
    pub fn append(
        &mut self,
        draft: &ActionRecordDraft,
    ) -> Result<ActionHistoryEntry, ActionHistoryError> {
        if self.records.len() >= MAX_ENTRIES {
            return Err(ActionHistoryError::Full);
        }
        if draft.recorded_at_epoch_ms < self.last_recorded_at_epoch_ms
            || draft.retain_until_epoch_ms <= draft.recorded_at_epoch_ms
            || !valid_sha256(&draft.effect_sha256)
            || draft.evidence_sha256s.len() > MAX_EVIDENCE
            || draft
                .evidence_sha256s
                .iter()
                .any(|value| !valid_sha256(value))
            || draft
                .evidence_sha256s
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(ActionHistoryError::InvalidInput);
        }
        if draft.outcome.needs_authorization()
            && matches!(draft.authorization, ActionAuthorization::Unauthorized {})
        {
            return Err(ActionHistoryError::AuthorizationMissing);
        }
        let mut redacted_fields = 0_u16;
        let action_id = kept_identifier(&draft.action_id, &mut redacted_fields)?;
        let reason_code = kept_identifier(&draft.reason_code, &mut redacted_fields)?;
        let authorization = match &draft.authorization {
            ActionAuthorization::Grant {
                grant_id,
                grant_sha256,
            } => {
                if !valid_sha256(grant_sha256) {
                    return Err(ActionHistoryError::InvalidInput);
                }
                ActionAuthorization::Grant {
                    grant_id: kept_identifier(grant_id, &mut redacted_fields)?,
                    grant_sha256: grant_sha256.clone(),
                }
            }
            ActionAuthorization::PersonDecision { decision_sha256 } => {
                if !valid_sha256(decision_sha256) {
                    return Err(ActionHistoryError::InvalidInput);
                }
                draft.authorization.clone()
            }
            ActionAuthorization::Unauthorized {} => ActionAuthorization::Unauthorized {},
        };
        let sequence = u64::try_from(self.records.len())
            .map_err(|_| ActionHistoryError::Full)?
            .checked_add(1)
            .ok_or(ActionHistoryError::Full)?;
        let mut entry = ActionHistoryEntry {
            schema_version: SCHEMA_VERSION,
            sequence,
            action_kind: draft.action_kind,
            action_id,
            authorization,
            effect_sha256: draft.effect_sha256.clone(),
            outcome: draft.outcome,
            reason_code,
            evidence_sha256s: draft.evidence_sha256s.clone(),
            recorded_at_epoch_ms: draft.recorded_at_epoch_ms,
            retain_until_epoch_ms: draft.retain_until_epoch_ms,
            redacted_fields,
            previous_entry_sha256: self.head().head_sha256,
            entry_sha256: GENESIS_SHA256.to_owned(),
        };
        entry.entry_sha256 = entry_digest(&entry)?;
        self.records.push(ActionHistoryRecord::Kept(entry.clone()));
        self.last_recorded_at_epoch_ms = draft.recorded_at_epoch_ms;
        Ok(entry)
    }

    /// Keeps only the place of every entry whose retention deadline passed.
    /// Returns how many entries expired now.
    pub fn apply_retention(&mut self, now_epoch_ms: u64) -> u64 {
        let mut expired = 0;
        for record in &mut self.records {
            if let ActionHistoryRecord::Kept(entry) = record
                && entry.retain_until_epoch_ms <= now_epoch_ms
            {
                *record = ActionHistoryRecord::Expired {
                    sequence: entry.sequence,
                    previous_entry_sha256: entry.previous_entry_sha256.clone(),
                    entry_sha256: entry.entry_sha256.clone(),
                };
                expired += 1;
            }
        }
        expired
    }

    /// The retained end of the chain.
    #[must_use]
    pub fn head(&self) -> ActionHistoryHead {
        ActionHistoryHead {
            count: self.records.len() as u64,
            head_sha256: self.records.last().map_or_else(
                || GENESIS_SHA256.to_owned(),
                |record| record.digest().to_owned(),
            ),
        }
    }

    /// Every position in order.
    #[must_use]
    pub fn records(&self) -> &[ActionHistoryRecord] {
        &self.records
    }

    /// Rebuilds a history from retained positions, which must form exactly
    /// the chain that ends at the retained head. A kept entry must also be one
    /// that `append` could have kept (Decision 0125): it cannot be redacted
    /// again without changing its digest, so anything else is refused.
    pub fn replay(
        records: Vec<ActionHistoryRecord>,
        head: &ActionHistoryHead,
    ) -> Result<Self, ActionHistoryError> {
        if records.len() > MAX_ENTRIES {
            return Err(ActionHistoryError::Integrity);
        }
        let mut previous = GENESIS_SHA256.to_owned();
        let mut last_recorded_at_epoch_ms = 0;
        for (index, record) in records.iter().enumerate() {
            let expected_sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(ActionHistoryError::Integrity)?;
            if record.sequence() != expected_sequence
                || record.previous() != previous
                || !valid_sha256(record.digest())
            {
                return Err(ActionHistoryError::Integrity);
            }
            if let ActionHistoryRecord::Kept(entry) = record {
                if entry.schema_version != SCHEMA_VERSION
                    || !kept_entry_is_well_formed(entry)
                    || entry_digest(entry)? != entry.entry_sha256
                    || entry.recorded_at_epoch_ms < last_recorded_at_epoch_ms
                {
                    return Err(ActionHistoryError::Integrity);
                }
                last_recorded_at_epoch_ms = entry.recorded_at_epoch_ms;
            }
            record.digest().clone_into(&mut previous);
        }
        let history = Self {
            records,
            last_recorded_at_epoch_ms,
        };
        if &history.head() != head {
            return Err(ActionHistoryError::Integrity);
        }
        Ok(history)
    }

    /// Builds a redacted manual export of the requested positions.
    pub fn export(
        &self,
        request: ActionHistoryExportRequest,
    ) -> Result<ActionHistoryExport, ActionHistoryError> {
        let count = self.records.len() as u64;
        if request.from_sequence == 0
            || request.from_sequence > request.to_sequence
            || request.to_sequence > count
        {
            return Err(ActionHistoryError::InvalidInput);
        }
        let from = usize::try_from(request.from_sequence - 1)
            .map_err(|_| ActionHistoryError::InvalidInput)?;
        let to =
            usize::try_from(request.to_sequence).map_err(|_| ActionHistoryError::InvalidInput)?;
        let records = &self.records[from..to];
        let mut kept_count = 0_u64;
        let mut expired_count = 0_u64;
        let mut redacted_fields = 0_u64;
        for record in records {
            match record {
                ActionHistoryRecord::Kept(entry) => {
                    kept_count += 1;
                    redacted_fields += u64::from(entry.redacted_fields);
                }
                ActionHistoryRecord::Expired { .. } => expired_count += 1,
            }
        }
        let bytes = serde_json::to_vec(&ExportDocument {
            schema_version: SCHEMA_VERSION,
            record_type: "agentmage-action-history-export",
            from_sequence: request.from_sequence,
            to_sequence: request.to_sequence,
            head: self.head(),
            records,
        })
        .map_err(|_| ActionHistoryError::InvalidInput)?;
        if bytes.len() > MAX_EXPORT_BYTES {
            return Err(ActionHistoryError::Full);
        }
        if !detect_secret_classes("action-history-export", &bytes).is_empty() {
            return Err(ActionHistoryError::SecretDetected);
        }
        Ok(ActionHistoryExport {
            export_sha256: sha256_hex(&bytes),
            bytes,
            kept_count,
            expired_count,
            redacted_fields,
            write_enabled: false,
        })
    }
}

#[derive(Serialize)]
struct ExportDocument<'a> {
    schema_version: u16,
    record_type: &'static str,
    from_sequence: u64,
    to_sequence: u64,
    head: ActionHistoryHead,
    records: &'a [ActionHistoryRecord],
}

/// Keeps a plain identifier and replaces anything else with the redaction
/// marker. The replaced value leaves no digest behind.
fn kept_identifier(value: &str, redacted: &mut u16) -> Result<String, ActionHistoryError> {
    if value.is_empty() {
        return Err(ActionHistoryError::InvalidInput);
    }
    if plain_identifier(value) {
        Ok(value.to_owned())
    } else {
        *redacted = redacted
            .checked_add(1)
            .ok_or(ActionHistoryError::InvalidInput)?;
        Ok(REDACTED.to_owned())
    }
}

/// A bounded identifier of lowercase letters, digits, `.`, `-` and `:` that
/// the shared secret detector does not flag.
fn plain_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b':')
        })
        && detect_secret_classes("action-history-field", value.as_bytes()).is_empty()
}

/// Whether a replayed entry holds only what `append` keeps: every identifier
/// is the redaction marker or a plain identifier, the entry counts its
/// markers, and every digest, bound and the authorization rule hold.
fn kept_entry_is_well_formed(entry: &ActionHistoryEntry) -> bool {
    let mut identifiers = vec![entry.action_id.as_str(), entry.reason_code.as_str()];
    let authorization = match &entry.authorization {
        ActionAuthorization::Grant {
            grant_id,
            grant_sha256,
        } => {
            identifiers.push(grant_id);
            valid_sha256(grant_sha256)
        }
        ActionAuthorization::PersonDecision { decision_sha256 } => valid_sha256(decision_sha256),
        ActionAuthorization::Unauthorized {} => !entry.outcome.needs_authorization(),
    };
    let markers = identifiers
        .iter()
        .filter(|value| **value == REDACTED)
        .count();
    authorization
        && identifiers
            .iter()
            .all(|value| *value == REDACTED || plain_identifier(value))
        && usize::from(entry.redacted_fields) == markers
        && entry.retain_until_epoch_ms > entry.recorded_at_epoch_ms
        && valid_sha256(&entry.effect_sha256)
        && entry.evidence_sha256s.len() <= MAX_EVIDENCE
        && entry
            .evidence_sha256s
            .iter()
            .all(|value| valid_sha256(value))
        && entry
            .evidence_sha256s
            .windows(2)
            .all(|pair| pair[0] < pair[1])
}

fn entry_digest(entry: &ActionHistoryEntry) -> Result<String, ActionHistoryError> {
    let mut candidate = entry.clone();
    GENESIS_SHA256.clone_into(&mut candidate.entry_sha256);
    serde_json::to_vec(&candidate)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| ActionHistoryError::InvalidInput)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(value: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn draft(action_id: &str, recorded_at: u64) -> ActionRecordDraft {
        ActionRecordDraft {
            action_kind: ActionKind::FileWrite,
            action_id: action_id.to_owned(),
            authorization: ActionAuthorization::Grant {
                grant_id: "grant-write-1".to_owned(),
                grant_sha256: digest('a'),
            },
            effect_sha256: digest('b'),
            outcome: ActionOutcome::Succeeded,
            reason_code: "tool.write.applied".to_owned(),
            evidence_sha256s: vec![digest('c'), digest('d')],
            recorded_at_epoch_ms: recorded_at,
            retain_until_epoch_ms: recorded_at + 1_000,
        }
    }

    // Synthetic canaries, assembled so no complete token appears in source.
    fn canaries() -> Vec<String> {
        vec![
            concat!("gh", "p_", "AMS42CANARYaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_owned(),
            concat!("s", "k-", "am-s42-canary-bbbbbbbbbbbbbbbbbbbbbbbb").to_owned(),
            "Bearer am-s42-canary-token-cccccccc".to_owned(),
            "Please summarize the private design notes in docs/secret-plan.md".to_owned(),
            // Only the plain-identifier charset catches this one.
            "Secret_Value_1".to_owned(),
        ]
    }

    #[test]
    fn an_action_is_kept_with_its_authorization_effect_outcome_and_evidence() {
        let mut history = ActionHistory::new();
        let first = history.append(&draft("write-1", 10)).unwrap();
        assert_eq!(first.sequence, 1);
        assert_eq!(first.previous_entry_sha256, GENESIS_SHA256);
        assert_eq!(first.redacted_fields, 0);
        assert_eq!(first.action_id, "write-1");
        assert_eq!(
            first.authorization,
            ActionAuthorization::Grant {
                grant_id: "grant-write-1".to_owned(),
                grant_sha256: digest('a'),
            }
        );
        let mut decided = draft("write-2", 10);
        decided.authorization = ActionAuthorization::PersonDecision {
            decision_sha256: digest('e'),
        };
        let second = history.append(&decided).unwrap();
        assert_eq!(second.previous_entry_sha256, first.entry_sha256);
        assert_eq!(history.head().count, 2);
        assert_eq!(history.head().head_sha256, second.entry_sha256);
        // A denied or cancelled action may name no authorization; an action
        // that may have had an effect may not.
        for outcome in [ActionOutcome::Denied, ActionOutcome::Cancelled] {
            let mut refused = draft("write-3", 11);
            refused.authorization = ActionAuthorization::Unauthorized {};
            refused.outcome = outcome;
            assert!(history.append(&refused).is_ok());
        }
        for outcome in [
            ActionOutcome::Succeeded,
            ActionOutcome::Failed,
            ActionOutcome::Uncertain,
        ] {
            let mut unauthorized = draft("write-4", 11);
            unauthorized.authorization = ActionAuthorization::Unauthorized {};
            unauthorized.outcome = outcome;
            assert_eq!(
                history.append(&unauthorized),
                Err(ActionHistoryError::AuthorizationMissing)
            );
        }
        assert_eq!(history.head().count, 4);
    }

    #[test]
    fn malformed_digests_times_and_evidence_are_refused_and_change_nothing() {
        let mut history = ActionHistory::new();
        history.append(&draft("write-1", 10)).unwrap();
        let head = history.head();
        let cases: [&dyn Fn(&mut ActionRecordDraft); 8] = [
            &|value| value.recorded_at_epoch_ms = 9,
            &|value| value.retain_until_epoch_ms = value.recorded_at_epoch_ms,
            &|value| value.effect_sha256 = "B".repeat(64),
            &|value| value.evidence_sha256s = vec![digest('d'), digest('c')],
            &|value| value.evidence_sha256s = vec![digest('c'), digest('c')],
            &|value| value.evidence_sha256s = vec![digest('c'); MAX_EVIDENCE + 1],
            &|value| value.action_id = String::new(),
            &|value| {
                value.authorization = ActionAuthorization::PersonDecision {
                    decision_sha256: "short".to_owned(),
                };
            },
        ];
        for change in cases {
            let mut value = draft("write-2", 10);
            change(&mut value);
            assert_eq!(
                history.append(&value),
                Err(ActionHistoryError::InvalidInput)
            );
            assert_eq!(history.head(), head);
        }
    }

    #[test]
    fn secrets_and_raw_prompts_are_replaced_before_they_are_kept_or_exported() {
        let mut history = ActionHistory::new();
        for canary in canaries() {
            let mut value = draft(&canary, 10);
            value.reason_code.clone_from(&canary);
            value.authorization = ActionAuthorization::Grant {
                grant_id: canary.clone(),
                grant_sha256: digest('a'),
            };
            let entry = history.append(&value).unwrap();
            assert_eq!(entry.redacted_fields, 3);
            assert_eq!(entry.action_id, REDACTED);
            assert_eq!(entry.reason_code, REDACTED);
        }
        let export = history
            .export(ActionHistoryExportRequest {
                from_sequence: 1,
                to_sequence: 5,
            })
            .unwrap();
        assert_eq!(export.kept_count, 5);
        assert_eq!(export.redacted_fields, 15);
        assert!(!export.write_enabled);
        let text = String::from_utf8(export.bytes.clone()).unwrap();
        let kept = format!("{:?}", history.records());
        for canary in canaries() {
            let canary_digest = sha256_hex(canary.as_bytes());
            for haystack in [&text, &kept] {
                assert!(!haystack.contains(&canary));
                assert!(!haystack.contains(&canary_digest));
            }
        }
        assert_eq!(export.export_sha256, sha256_hex(&export.bytes));
        // A redacted history replays as it was kept.
        let replayed = ActionHistory::replay(history.records().to_vec(), &history.head()).unwrap();
        assert_eq!(replayed, history);
        // A plain identifier with a separator is kept as it is.
        let kept = history.append(&draft("secret.value-1:a", 11)).unwrap();
        assert_eq!(
            (kept.action_id.as_str(), kept.redacted_fields),
            ("secret.value-1:a", 0)
        );
    }

    type EntryChange<'a> = Box<dyn Fn(&mut ActionHistoryEntry) + 'a>;

    // Replaces the only entry of a one-entry chain by a changed entry with a
    // consistent digest and head, so only the replay rules can refuse it.
    fn replay_changed(
        change: &dyn Fn(&mut ActionHistoryEntry),
    ) -> Result<ActionHistory, ActionHistoryError> {
        let mut history = ActionHistory::new();
        history.append(&draft("write-1", 10)).unwrap();
        let mut records = history.records().to_vec();
        let ActionHistoryRecord::Kept(entry) = &mut records[0] else {
            unreachable!()
        };
        change(entry);
        entry.entry_sha256 = entry_digest(entry).unwrap();
        let head = ActionHistoryHead {
            count: 1,
            head_sha256: entry.entry_sha256.clone(),
        };
        ActionHistory::replay(records, &head)
    }

    #[test]
    fn replay_refuses_an_entry_that_append_could_not_have_kept() {
        // Decision 0125 (review F1): the digest and head are consistent in
        // every case; each change breaks exactly one replay rule.
        assert!(replay_changed(&|_| {}).is_ok());
        let canaries = canaries();
        let long = "a".repeat(MAX_IDENTIFIER_BYTES + 1);
        let cases: Vec<EntryChange<'_>> = vec![
            Box::new(|entry| entry.action_id.clone_from(&canaries[3])),
            Box::new(|entry| entry.reason_code.clone_from(&canaries[1])),
            Box::new(|entry| entry.reason_code.clone_from(&canaries[4])),
            Box::new(|entry| entry.action_id.clone_from(&long)),
            Box::new(|entry| entry.action_id.clear()),
            Box::new(|entry| {
                entry.authorization = ActionAuthorization::Grant {
                    grant_id: canaries[0].clone(),
                    grant_sha256: digest('a'),
                };
            }),
            Box::new(|entry| entry.redacted_fields = 1),
            Box::new(|entry| REDACTED.clone_into(&mut entry.action_id)),
            Box::new(|entry| entry.authorization = ActionAuthorization::Unauthorized {}),
            Box::new(|entry| {
                entry.authorization = ActionAuthorization::Grant {
                    grant_id: "grant-write-1".to_owned(),
                    grant_sha256: "A".repeat(64),
                };
            }),
            Box::new(|entry| {
                entry.authorization = ActionAuthorization::PersonDecision {
                    decision_sha256: "short".to_owned(),
                };
            }),
            Box::new(|entry| entry.retain_until_epoch_ms = entry.recorded_at_epoch_ms),
            Box::new(|entry| entry.effect_sha256 = "B".repeat(64)),
            Box::new(|entry| {
                entry.evidence_sha256s = (0..=MAX_EVIDENCE)
                    .map(|index| format!("{index:064x}"))
                    .collect();
            }),
            Box::new(|entry| entry.evidence_sha256s = vec!["C".repeat(64)]),
            Box::new(|entry| entry.evidence_sha256s = vec![digest('d'), digest('c')]),
            // Review F1 of `7c593b3b`: a repeated evidence digest alone.
            Box::new(|entry| entry.evidence_sha256s = vec![digest('c'), digest('c')]),
        ];
        for change in cases {
            assert_eq!(replay_changed(&*change), Err(ActionHistoryError::Integrity));
        }
        // A redacted entry that counts its markers replays.
        assert!(
            replay_changed(&|entry| {
                REDACTED.clone_into(&mut entry.action_id);
                REDACTED.clone_into(&mut entry.reason_code);
                entry.redacted_fields = 2;
            })
            .is_ok()
        );
        // A denied action may name no authorization.
        assert!(
            replay_changed(&|entry| {
                entry.authorization = ActionAuthorization::Unauthorized {};
                entry.outcome = ActionOutcome::Denied;
            })
            .is_ok()
        );
    }

    #[test]
    fn retained_positions_cross_json_closed_and_replay_unchanged() {
        // Decision 0127: a declared history is parsed closed and replays to
        // the same chain; the variant without authority keeps its encoding.
        let mut history = ActionHistory::new();
        history.append(&draft("write-1", 10)).unwrap();
        let mut refused = draft("write-2", 11);
        refused.authorization = ActionAuthorization::Unauthorized {};
        refused.outcome = ActionOutcome::Denied;
        history.append(&refused).unwrap();
        let mut expiring = draft("write-3", 12);
        expiring.retain_until_epoch_ms = 13;
        history.append(&expiring).unwrap();
        history.apply_retention(13);
        let bytes = serde_json::to_vec(history.records()).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("\"authorization\":{\"kind\":\"unauthorized\"}"));
        let records: Vec<ActionHistoryRecord> = serde_json::from_slice(&bytes).unwrap();
        let head: ActionHistoryHead =
            serde_json::from_slice(&serde_json::to_vec(&history.head()).unwrap()).unwrap();
        // An expired place keeps no time (Decision 0125, N1), so only the
        // positions and the head are compared.
        let replayed = ActionHistory::replay(records, &head).unwrap();
        assert_eq!(
            (replayed.records(), replayed.head()),
            (history.records(), history.head())
        );
        for extra in [
            text.replacen(
                "{\"kind\":\"unauthorized\"}",
                "{\"kind\":\"unauthorized\",\"grant_id\":\"grant-1\"}",
                1,
            ),
            text.replacen("\"kind\":\"grant\"", "\"kind\":\"grant\",\"x\":1", 1),
            text.replacen("\"state\":\"kept\"", "\"state\":\"kept\",\"x\":1", 1),
            text.replacen("\"state\":\"expired\"", "\"state\":\"expired\",\"x\":1", 1),
            text.replacen("\"outcome\":\"denied\"", "\"outcome\":\"undone\"", 1),
            text.replacen(
                "\"action_kind\":\"file_write\"",
                "\"action_kind\":\"other\"",
                1,
            ),
        ] {
            assert_ne!(extra, text);
            assert!(serde_json::from_str::<Vec<ActionHistoryRecord>>(&extra).is_err());
        }
        assert!(
            serde_json::from_value::<ActionHistoryHead>(serde_json::json!({
                "count": 3,
                "head_sha256": history.head().head_sha256,
                "complete": true,
            }))
            .is_err()
        );
    }

    #[test]
    fn the_export_rescan_withholds_a_secret_that_reached_the_history() {
        // Decision 0125 (review F1): append and replay both refuse such an
        // entry, so it is built directly and the rescan is the only guard.
        let mut history = ActionHistory::new();
        history.append(&draft("write-1", 10)).unwrap();
        let ActionHistoryRecord::Kept(entry) = &mut history.records[0] else {
            unreachable!()
        };
        entry.action_id.clone_from(&canaries()[0]);
        entry.entry_sha256 = entry_digest(entry).unwrap();
        assert_eq!(
            history.export(ActionHistoryExportRequest {
                from_sequence: 1,
                to_sequence: 1,
            }),
            Err(ActionHistoryError::SecretDetected)
        );
    }

    #[test]
    fn an_expired_entry_keeps_only_its_place_and_the_chain_still_replays() {
        let mut history = ActionHistory::new();
        history.append(&draft("write-1", 10)).unwrap();
        let mut long = draft("write-2", 20);
        long.retain_until_epoch_ms = 5_000;
        history.append(&long).unwrap();
        assert_eq!(history.apply_retention(1_009), 0);
        assert_eq!(history.apply_retention(1_010), 1);
        assert_eq!(history.apply_retention(1_010), 0);
        assert!(matches!(
            history.records()[0],
            ActionHistoryRecord::Expired { sequence: 1, .. }
        ));
        assert!(matches!(history.records()[1], ActionHistoryRecord::Kept(_)));
        let export = history
            .export(ActionHistoryExportRequest {
                from_sequence: 1,
                to_sequence: 2,
            })
            .unwrap();
        assert_eq!((export.kept_count, export.expired_count), (1, 1));
        assert!(
            !String::from_utf8(export.bytes)
                .unwrap()
                .contains("\"write-1\"")
        );
        let replayed = ActionHistory::replay(history.records().to_vec(), &history.head()).unwrap();
        assert_eq!(replayed, history);
        // Appends continue from the retained head.
        let mut replayed = replayed;
        let third = replayed.append(&draft("write-3", 30)).unwrap();
        assert_eq!(third.sequence, 3);
        assert_eq!(third.previous_entry_sha256, history.head().head_sha256);
    }

    #[test]
    fn a_tampered_reordered_truncated_or_foreign_chain_is_refused() {
        let mut history = ActionHistory::new();
        for (index, id) in ["write-1", "write-2", "write-3"].iter().enumerate() {
            history.append(&draft(id, 10 + index as u64)).unwrap();
        }
        let head = history.head();
        let records = history.records().to_vec();
        let mut changed = records.clone();
        if let ActionHistoryRecord::Kept(entry) = &mut changed[1] {
            entry.outcome = ActionOutcome::Failed;
        }
        let mut swapped = records.clone();
        swapped.swap(0, 1);
        let mut relinked = records.clone();
        if let ActionHistoryRecord::Kept(entry) = &mut relinked[2] {
            entry.previous_entry_sha256 = digest('f');
            entry.entry_sha256 = entry_digest(entry).unwrap();
        }
        for tampered in [changed, swapped, relinked, records[..2].to_vec()] {
            assert_eq!(
                ActionHistory::replay(tampered, &head),
                Err(ActionHistoryError::Integrity)
            );
        }
        let foreign = ActionHistoryHead {
            count: 3,
            head_sha256: digest('f'),
        };
        assert_eq!(
            ActionHistory::replay(records.clone(), &foreign),
            Err(ActionHistoryError::Integrity)
        );
        assert_eq!(ActionHistory::replay(records, &head).unwrap(), history);
    }

    #[test]
    fn an_expired_place_replaced_by_another_breaks_the_next_link() {
        // Decision 0125 (review F2): the head binds only the last digest, so
        // an expired middle position depends on the previous-link check.
        let mut history = ActionHistory::new();
        for (index, id) in ["write-1", "write-2", "write-3"].iter().enumerate() {
            let mut value = draft(id, 10 + index as u64);
            value.retain_until_epoch_ms = if index == 1 { 100 } else { 5_000 };
            history.append(&value).unwrap();
        }
        assert_eq!(history.apply_retention(100), 1);
        let head = history.head();
        let mut records = history.records().to_vec();
        assert!(ActionHistory::replay(records.clone(), &head).is_ok());
        if let ActionHistoryRecord::Expired { entry_sha256, .. } = &mut records[1] {
            *entry_sha256 = digest('f');
        } else {
            unreachable!()
        }
        assert_eq!(
            ActionHistory::replay(records, &head),
            Err(ActionHistoryError::Integrity)
        );
    }

    #[test]
    fn an_export_covers_exactly_the_requested_positions() {
        let mut history = ActionHistory::new();
        for (index, id) in ["write-1", "write-2", "write-3"].iter().enumerate() {
            history.append(&draft(id, 10 + index as u64)).unwrap();
        }
        let export = history
            .export(ActionHistoryExportRequest {
                from_sequence: 2,
                to_sequence: 3,
            })
            .unwrap();
        let text = String::from_utf8(export.bytes).unwrap();
        assert!(!text.contains("\"write-1\""));
        assert!(text.contains("\"write-2\"") && text.contains("\"write-3\""));
        for (from_sequence, to_sequence) in [(0, 1), (3, 2), (1, 4)] {
            assert_eq!(
                history.export(ActionHistoryExportRequest {
                    from_sequence,
                    to_sequence,
                }),
                Err(ActionHistoryError::InvalidInput)
            );
        }
    }
}
