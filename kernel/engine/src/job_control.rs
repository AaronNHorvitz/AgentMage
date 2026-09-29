//! Single-owner reconciliation of durable job control requests (CAP-35).
//!
//! A client may close and reopen, or a second client may attach, while a job
//! runs. Each control request carries its own request identity and the job
//! revision its sender observed. The owner applies a request at most once and
//! answers a retry with the original decision. A request built on a stale
//! revision is refused, as is a transition the job's phase does not allow.
//! Suspension and cancellation of running work are requests: the job enters
//! `Suspending` or `Cancelling`, and only the owner's observation at a safe
//! boundary completes them, so no client can claim that work stopped.
//!
//! Every decision and owner transition is one entry in a hash chain. A
//! restarted owner replays the retained entries, which must reproduce the same
//! chain, before it accepts a new request. The ledger grants no authority and
//! performs no effect; it is the reconciled record the owner acts on.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Largest number of retained entries for one job.
pub const MAX_JOB_LEDGER_ENTRIES: usize = 4_096;
const MAX_IDENTIFIER_BYTES: usize = 128;
const GENESIS_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Closed job phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPhase {
    /// Waiting for the owner to start it.
    Queued,
    /// Held and advanced by its one owner.
    Running,
    /// Suspension was requested; the owner has not yet reached a safe boundary.
    Suspending,
    /// Stopped at a safe boundary; resumable.
    Suspended,
    /// Cancellation was requested; the owner has not yet observed it.
    Cancelling,
    /// Finished successfully.
    Completed,
    /// Finished with a failure.
    Failed,
    /// Cancellation was observed by the owner, or the job never started.
    Cancelled,
}

impl JobPhase {
    /// Whether no further transition is possible.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Control a client may request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobControlAction {
    /// Stop at the next safe boundary and keep the job resumable.
    Suspend,
    /// Continue a suspended job, or withdraw a pending suspension.
    Resume,
    /// Stop the job for good.
    Cancel,
}

/// One client control request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobControlRequest {
    /// Request schema version.
    pub schema_version: u16,
    /// Target job.
    pub job_id: String,
    /// Client-chosen identity; a retry reuses it with identical content.
    pub request_id: String,
    /// Requested control.
    pub action: JobControlAction,
    /// Job revision the sender observed.
    pub observed_revision: u64,
}

/// Owner observation that advances the job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobOwnerEvent {
    /// The owner started or restarted a queued job.
    Started,
    /// The owner reached a safe boundary after a suspension request.
    SuspensionObserved,
    /// The owner observed a cancellation request and stopped owned work.
    CancellationObserved,
    /// The job finished successfully.
    Completed,
    /// The job finished with a failure.
    Failed,
}

/// Closed reason a request was not applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobControlRefusal {
    /// The sender observed an older revision and must observe again.
    StaleRevision,
    /// The request's effect is already in force.
    AlreadyInEffect,
    /// The job is terminal.
    Terminal,
    /// The phase does not allow this control.
    NotAllowed,
}

/// Decision for one control request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum JobControlDecision {
    /// The request changed the job.
    Applied {
        /// Revision after the change.
        revision: u64,
        /// Phase after the change.
        phase: JobPhase,
    },
    /// The request was recorded and not applied.
    Refused {
        /// Why.
        refusal: JobControlRefusal,
        /// Revision the sender should observe.
        revision: u64,
        /// Current phase.
        phase: JobPhase,
    },
}

/// Content-free ledger failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobControlError {
    /// Malformed identity, schema or foreign job.
    Invalid,
    /// A request identity was reused with different content.
    RequestConflict,
    /// The owner event does not follow from the current phase.
    InvalidTransition,
    /// Retained entries do not form this job's exact chain.
    Integrity,
    /// The entry bound is reached.
    Full,
}

/// What one entry records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum JobLedgerRecord {
    /// The job was queued by its owner.
    Created {
        /// Owning runtime identity.
        owner_id: String,
    },
    /// A client control request and its decision.
    Control {
        /// Exact request.
        request: JobControlRequest,
        /// Digest of the canonical request.
        request_sha256: String,
        /// Decision.
        decision: JobControlDecision,
    },
    /// An owner observation.
    Owner {
        /// Event.
        event: JobOwnerEvent,
    },
}

/// One sealed chain entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobLedgerEntry {
    /// Entry schema version.
    pub schema_version: u16,
    /// Owning job.
    pub job_id: String,
    /// Zero-based position.
    pub sequence: u64,
    /// Recorded fact.
    pub record: JobLedgerRecord,
    /// Job revision after this entry; a refusal leaves it unchanged.
    pub revision: u64,
    /// Phase after this entry.
    pub phase: JobPhase,
    /// Previous entry digest, or zeroes for the first.
    pub prior_entry_sha256: String,
    /// Digest of this entry with this field zeroed.
    pub entry_sha256: String,
}

/// Current reconciled state, suitable for any attached client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct JobObservation {
    /// Job identity.
    pub job_id: String,
    /// Current phase.
    pub phase: JobPhase,
    /// Current revision; controls must name it.
    pub revision: u64,
    /// Whether a cancellation request was accepted at any point.
    pub cancellation_requested: bool,
    /// Last entry digest.
    pub head_sha256: String,
}

/// Single-owner job control ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobControlLedger {
    job_id: String,
    owner_id: String,
    phase: JobPhase,
    revision: u64,
    cancellation_requested: bool,
    entries: Vec<JobLedgerEntry>,
    requests: BTreeMap<String, (String, JobControlDecision)>,
}

impl JobControlLedger {
    /// Queues a new job under one owner.
    pub fn create(job_id: &str, owner_id: &str) -> Result<Self, JobControlError> {
        if !valid_identifier(job_id) || !valid_identifier(owner_id) {
            return Err(JobControlError::Invalid);
        }
        let mut ledger = Self {
            job_id: job_id.to_owned(),
            owner_id: owner_id.to_owned(),
            phase: JobPhase::Queued,
            revision: 0,
            cancellation_requested: false,
            entries: Vec::new(),
            requests: BTreeMap::new(),
        };
        ledger.append(
            JobLedgerRecord::Created {
                owner_id: owner_id.to_owned(),
            },
            0,
            JobPhase::Queued,
        )?;
        Ok(ledger)
    }

    /// Rebuilds the ledger from retained entries, which must reproduce exactly.
    pub fn replay(job_id: &str, entries: &[JobLedgerEntry]) -> Result<Self, JobControlError> {
        let Some(JobLedgerEntry {
            record: JobLedgerRecord::Created { owner_id },
            ..
        }) = entries.first()
        else {
            return Err(JobControlError::Integrity);
        };
        let mut ledger = Self::create(job_id, owner_id).map_err(|_| JobControlError::Integrity)?;
        for entry in &entries[1..] {
            match &entry.record {
                JobLedgerRecord::Created { .. } => return Err(JobControlError::Integrity),
                JobLedgerRecord::Control { request, .. } => {
                    ledger
                        .control(request)
                        .map_err(|_| JobControlError::Integrity)?;
                }
                JobLedgerRecord::Owner { event } => {
                    ledger
                        .observe_owner(*event)
                        .map_err(|_| JobControlError::Integrity)?;
                }
            }
        }
        if ledger.entries != entries {
            return Err(JobControlError::Integrity);
        }
        Ok(ledger)
    }

    /// Applies one client request at most once. A retry with identical content
    /// returns the original decision without a new entry.
    pub fn control(
        &mut self,
        request: &JobControlRequest,
    ) -> Result<JobControlDecision, JobControlError> {
        if request.schema_version != 1
            || request.job_id != self.job_id
            || !valid_identifier(&request.request_id)
        {
            return Err(JobControlError::Invalid);
        }
        let request_sha256 = canonical_sha256(request)?;
        if let Some((recorded, decision)) = self.requests.get(&request.request_id) {
            return if *recorded == request_sha256 {
                Ok(*decision)
            } else {
                Err(JobControlError::RequestConflict)
            };
        }
        let refused = |refusal| JobControlDecision::Refused {
            refusal,
            revision: self.revision,
            phase: self.phase,
        };
        let next = if request.observed_revision != self.revision {
            Err(JobControlRefusal::StaleRevision)
        } else if self.phase.is_terminal() {
            Err(JobControlRefusal::Terminal)
        } else {
            transition(self.phase, request.action)
        };
        let decision = match next {
            Ok(phase) => JobControlDecision::Applied {
                revision: self.revision + 1,
                phase,
            },
            Err(refusal) => refused(refusal),
        };
        let (revision, phase) = match decision {
            JobControlDecision::Applied { revision, phase } => (revision, phase),
            JobControlDecision::Refused { .. } => (self.revision, self.phase),
        };
        self.append(
            JobLedgerRecord::Control {
                request: request.clone(),
                request_sha256: request_sha256.clone(),
                decision,
            },
            revision,
            phase,
        )?;
        if matches!(decision, JobControlDecision::Applied { .. })
            && request.action == JobControlAction::Cancel
        {
            self.cancellation_requested = true;
        }
        self.requests
            .insert(request.request_id.clone(), (request_sha256, decision));
        Ok(decision)
    }

    /// Records one owner observation.
    pub fn observe_owner(&mut self, event: JobOwnerEvent) -> Result<u64, JobControlError> {
        let phase = match (self.phase, event) {
            (JobPhase::Queued, JobOwnerEvent::Started) => JobPhase::Running,
            (JobPhase::Suspending, JobOwnerEvent::SuspensionObserved) => JobPhase::Suspended,
            (JobPhase::Cancelling, JobOwnerEvent::CancellationObserved) => JobPhase::Cancelled,
            // Work may finish before a pending request reaches a safe boundary;
            // the finished state wins and the request stays in the ledger.
            (
                JobPhase::Running | JobPhase::Suspending | JobPhase::Cancelling,
                JobOwnerEvent::Completed,
            ) => JobPhase::Completed,
            (
                JobPhase::Running | JobPhase::Suspending | JobPhase::Cancelling,
                JobOwnerEvent::Failed,
            ) => JobPhase::Failed,
            _ => return Err(JobControlError::InvalidTransition),
        };
        let revision = self.revision + 1;
        self.append(JobLedgerRecord::Owner { event }, revision, phase)?;
        Ok(revision)
    }

    /// Current reconciled state.
    #[must_use]
    pub fn observation(&self) -> JobObservation {
        JobObservation {
            job_id: self.job_id.clone(),
            phase: self.phase,
            revision: self.revision,
            cancellation_requested: self.cancellation_requested,
            head_sha256: self.entries.last().map_or_else(
                || GENESIS_SHA256.to_owned(),
                |entry| entry.entry_sha256.clone(),
            ),
        }
    }

    /// Retained entries in order.
    #[must_use]
    pub fn entries(&self) -> &[JobLedgerEntry] {
        &self.entries
    }

    /// Owning runtime identity.
    #[must_use]
    pub fn owner_id(&self) -> &str {
        &self.owner_id
    }

    fn append(
        &mut self,
        record: JobLedgerRecord,
        revision: u64,
        phase: JobPhase,
    ) -> Result<(), JobControlError> {
        if self.entries.len() >= MAX_JOB_LEDGER_ENTRIES {
            return Err(JobControlError::Full);
        }
        let mut entry = JobLedgerEntry {
            schema_version: 1,
            job_id: self.job_id.clone(),
            sequence: self.entries.len() as u64,
            record,
            revision,
            phase,
            prior_entry_sha256: self.entries.last().map_or_else(
                || GENESIS_SHA256.to_owned(),
                |entry| entry.entry_sha256.clone(),
            ),
            entry_sha256: GENESIS_SHA256.to_owned(),
        };
        entry.entry_sha256 = canonical_sha256(&entry)?;
        self.entries.push(entry);
        self.revision = revision;
        self.phase = phase;
        Ok(())
    }
}

const fn transition(
    phase: JobPhase,
    action: JobControlAction,
) -> Result<JobPhase, JobControlRefusal> {
    use JobControlAction::{Cancel, Resume, Suspend};
    use JobPhase::{Cancelling, Queued, Running, Suspended, Suspending};
    match (phase, action) {
        (Queued, Suspend) => Ok(Suspended),
        (Running, Suspend) => Ok(Suspending),
        (Suspending | Suspended, Suspend) | (Queued | Running, Resume) => {
            Err(JobControlRefusal::AlreadyInEffect)
        }
        (Suspended, Resume) => Ok(Queued),
        (Suspending, Resume) => Ok(Running),
        // Nothing runs, so cancellation completes at once.
        (Queued | Suspended, Cancel) => Ok(JobPhase::Cancelled),
        (Running | Suspending, Cancel) => Ok(Cancelling),
        (Cancelling, Cancel) => Err(JobControlRefusal::AlreadyInEffect),
        (Cancelling, Suspend | Resume) => Err(JobControlRefusal::NotAllowed),
        (JobPhase::Completed | JobPhase::Failed | JobPhase::Cancelled, _) => {
            Err(JobControlRefusal::Terminal)
        }
    }
}

/// Bounded text for a person.
#[must_use]
pub fn render_job_observation(observation: &JobObservation) -> String {
    let mut output = String::new();
    let phase = match observation.phase {
        JobPhase::Queued => "queued",
        JobPhase::Running => "running",
        JobPhase::Suspending => "suspension requested; still running until a safe boundary",
        JobPhase::Suspended => "suspended at a safe boundary",
        JobPhase::Cancelling => "cancellation requested; still running until the owner stops it",
        JobPhase::Completed => "completed",
        JobPhase::Failed => "failed",
        JobPhase::Cancelled => "cancelled",
    };
    let _ = write!(
        output,
        "job {:?} revision {}: {phase}",
        observation.job_id, observation.revision
    );
    if observation.cancellation_requested && observation.phase != JobPhase::Cancelled {
        output.push_str(" (a cancellation request was accepted)");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, JobControlError> {
    let bytes = serde_json::to_vec(value).map_err(|_| JobControlError::Invalid)?;
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(encoded, "{byte:02x}");
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: &str = "job-fixture";

    fn request(id: &str, action: JobControlAction, observed_revision: u64) -> JobControlRequest {
        JobControlRequest {
            schema_version: 1,
            job_id: JOB.to_owned(),
            request_id: id.to_owned(),
            action,
            observed_revision,
        }
    }

    fn running() -> JobControlLedger {
        let mut ledger = JobControlLedger::create(JOB, "owner-fixture").unwrap();
        assert_eq!(ledger.observe_owner(JobOwnerEvent::Started), Ok(1));
        ledger
    }

    #[test]
    fn cancellation_is_requested_once_and_completed_only_by_the_owner() {
        let mut ledger = running();
        let first = request("client-a-1", JobControlAction::Cancel, 1);
        let applied = JobControlDecision::Applied {
            revision: 2,
            phase: JobPhase::Cancelling,
        };
        assert_eq!(ledger.control(&first), Ok(applied));
        let entries = ledger.entries().len();
        // A client that reconnects and retries gets the original decision.
        assert_eq!(ledger.control(&first), Ok(applied));
        assert_eq!(ledger.entries().len(), entries);
        // A second client built its request on the older revision.
        assert_eq!(
            ledger.control(&request("client-b-1", JobControlAction::Cancel, 1)),
            Ok(JobControlDecision::Refused {
                refusal: JobControlRefusal::StaleRevision,
                revision: 2,
                phase: JobPhase::Cancelling,
            })
        );
        assert_eq!(
            ledger.control(&request("client-b-2", JobControlAction::Cancel, 2)),
            Ok(JobControlDecision::Refused {
                refusal: JobControlRefusal::AlreadyInEffect,
                revision: 2,
                phase: JobPhase::Cancelling,
            })
        );
        let text = render_job_observation(&ledger.observation());
        assert!(
            text.contains("still running until the owner stops it"),
            "{text}"
        );
        assert_eq!(
            ledger.observe_owner(JobOwnerEvent::CancellationObserved),
            Ok(3)
        );
        let observation = ledger.observation();
        assert_eq!(observation.phase, JobPhase::Cancelled);
        assert!(observation.cancellation_requested);
        assert_eq!(
            ledger.control(&request("client-a-2", JobControlAction::Resume, 3)),
            Ok(JobControlDecision::Refused {
                refusal: JobControlRefusal::Terminal,
                revision: 3,
                phase: JobPhase::Cancelled,
            })
        );
        assert_eq!(
            ledger.observe_owner(JobOwnerEvent::Started),
            Err(JobControlError::InvalidTransition)
        );
    }

    #[test]
    fn suspension_waits_for_a_safe_boundary_and_resume_requeues() {
        let mut ledger = running();
        assert_eq!(
            ledger.control(&request("s1", JobControlAction::Suspend, 1)),
            Ok(JobControlDecision::Applied {
                revision: 2,
                phase: JobPhase::Suspending,
            })
        );
        // Withdrawing a pending suspension keeps the job running.
        assert_eq!(
            ledger.control(&request("r1", JobControlAction::Resume, 2)),
            Ok(JobControlDecision::Applied {
                revision: 3,
                phase: JobPhase::Running,
            })
        );
        ledger
            .control(&request("s2", JobControlAction::Suspend, 3))
            .unwrap();
        assert_eq!(
            ledger.control(&request("s3", JobControlAction::Suspend, 4)),
            Ok(JobControlDecision::Refused {
                refusal: JobControlRefusal::AlreadyInEffect,
                revision: 4,
                phase: JobPhase::Suspending,
            })
        );
        assert_eq!(
            ledger.observe_owner(JobOwnerEvent::SuspensionObserved),
            Ok(5)
        );
        assert_eq!(
            ledger.control(&request("r2", JobControlAction::Resume, 5)),
            Ok(JobControlDecision::Applied {
                revision: 6,
                phase: JobPhase::Queued,
            })
        );
        assert_eq!(ledger.observe_owner(JobOwnerEvent::Started), Ok(7));
        assert_eq!(ledger.observe_owner(JobOwnerEvent::Completed), Ok(8));
        assert_eq!(ledger.observation().phase, JobPhase::Completed);
    }

    #[test]
    fn work_that_never_started_is_cancelled_or_suspended_at_once() {
        let mut ledger = JobControlLedger::create(JOB, "owner-fixture").unwrap();
        assert_eq!(
            ledger.control(&request("s1", JobControlAction::Suspend, 0)),
            Ok(JobControlDecision::Applied {
                revision: 1,
                phase: JobPhase::Suspended,
            })
        );
        assert_eq!(
            ledger.control(&request("c1", JobControlAction::Cancel, 1)),
            Ok(JobControlDecision::Applied {
                revision: 2,
                phase: JobPhase::Cancelled,
            })
        );
        // A job that finishes before its cancellation is observed stays finished.
        let mut ledger = running();
        ledger
            .control(&request("c1", JobControlAction::Cancel, 1))
            .unwrap();
        assert_eq!(ledger.observe_owner(JobOwnerEvent::Completed), Ok(3));
        let observation = ledger.observation();
        assert_eq!(observation.phase, JobPhase::Completed);
        assert!(render_job_observation(&observation).contains("cancellation request was accepted"));
    }

    #[test]
    fn conflicting_foreign_and_malformed_requests_are_refused_without_entries() {
        let mut ledger = running();
        ledger
            .control(&request("same-id", JobControlAction::Suspend, 1))
            .unwrap();
        let entries = ledger.entries().len();
        assert_eq!(
            ledger.control(&request("same-id", JobControlAction::Cancel, 2)),
            Err(JobControlError::RequestConflict)
        );
        let mut foreign = request("other", JobControlAction::Cancel, 2);
        foreign.job_id = "another-job".to_owned();
        let mut schema = request("other", JobControlAction::Cancel, 2);
        schema.schema_version = 2;
        for invalid in [
            foreign,
            schema,
            request("line\nbreak", JobControlAction::Cancel, 2),
            request("", JobControlAction::Cancel, 2),
            request(&"x".repeat(129), JobControlAction::Cancel, 2),
        ] {
            assert_eq!(ledger.control(&invalid), Err(JobControlError::Invalid));
        }
        assert_eq!(ledger.entries().len(), entries);
        assert_eq!(
            JobControlLedger::create("bad id", "owner").err(),
            Some(JobControlError::Invalid)
        );
        assert!(serde_json::from_str::<JobControlRequest>(
            r#"{"schema_version":1,"job_id":"j","request_id":"r","action":"cancel","observed_revision":0,"force":true}"#
        )
        .is_err());
    }

    #[test]
    fn a_restarted_owner_replays_exactly_and_refuses_tampered_chains() {
        let mut ledger = running();
        ledger
            .control(&request("s1", JobControlAction::Suspend, 1))
            .unwrap();
        ledger
            .control(&request("stale", JobControlAction::Cancel, 1))
            .unwrap();
        ledger
            .observe_owner(JobOwnerEvent::SuspensionObserved)
            .unwrap();
        let retained: Vec<JobLedgerEntry> =
            serde_json::from_str(&serde_json::to_string(ledger.entries()).unwrap()).unwrap();
        let replayed = JobControlLedger::replay(JOB, &retained).unwrap();
        assert_eq!(replayed, ledger);
        assert_eq!(replayed.observation(), ledger.observation());
        // The replayed owner still answers a retry with the original decision.
        let mut replayed = replayed;
        assert_eq!(
            replayed.control(&request("s1", JobControlAction::Suspend, 1)),
            ledger.control(&request("s1", JobControlAction::Suspend, 1))
        );

        let mut changed_decision = retained.clone();
        if let JobLedgerRecord::Control { decision, .. } = &mut changed_decision[2].record {
            *decision = JobControlDecision::Refused {
                refusal: JobControlRefusal::NotAllowed,
                revision: 1,
                phase: JobPhase::Running,
            };
        }
        let mut dropped = retained.clone();
        dropped.remove(2);
        let mut reordered = retained.clone();
        reordered.swap(2, 3);
        let mut duplicated = retained.clone();
        duplicated.insert(3, retained[2].clone());
        for tampered in [changed_decision, dropped, reordered, duplicated, Vec::new()] {
            assert_eq!(
                JobControlLedger::replay(JOB, &tampered).err(),
                Some(JobControlError::Integrity)
            );
        }
        assert_eq!(
            JobControlLedger::replay("another-job", &retained).err(),
            Some(JobControlError::Integrity)
        );
    }

    #[test]
    fn the_entry_bound_is_enforced() {
        let mut ledger = running();
        let mut index = 0;
        loop {
            let revision = ledger.observation().revision;
            match ledger.control(&request(
                &format!("r{index}"),
                JobControlAction::Resume,
                revision,
            )) {
                Ok(_) => index += 1,
                Err(error) => {
                    assert_eq!(error, JobControlError::Full);
                    break;
                }
            }
        }
        assert_eq!(ledger.entries().len(), MAX_JOB_LEDGER_ENTRIES);
        assert_eq!(
            ledger.observe_owner(JobOwnerEvent::Completed),
            Err(JobControlError::Full)
        );
    }
}
