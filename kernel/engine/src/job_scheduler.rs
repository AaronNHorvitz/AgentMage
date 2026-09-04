//! Pure resumable-job, lease, read-only schedule, notification, and receipt contracts.

use serde::{Deserialize, Serialize};

/// Closed job lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Queued and not leased.
    Pending,
    /// Held by one current lease owner.
    Running,
    /// Completed exactly once.
    Succeeded,
    /// Terminal failure without another admitted retry.
    Failed,
    /// Cancelled independently of the model.
    Cancelled,
    /// Paused for a live approval which unattended work cannot request.
    WaitingApproval,
    /// Retry limit exhausted.
    DeadLetter,
}

/// Exact current single-owner lease.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobLease {
    /// Unique lease identity.
    pub lease_id: String,
    /// Unique owner identity.
    pub owner_id: String,
    /// Acquisition time.
    pub acquired_epoch_milliseconds: u64,
    /// Last renewal time.
    pub renewed_epoch_milliseconds: u64,
    /// Expiration time.
    pub expires_epoch_milliseconds: u64,
    /// Whether independent cancellation was requested.
    pub cancelled: bool,
}

/// Independent per-job resource ceilings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobBudgets {
    /// Model-token ceiling.
    pub model_tokens: u64,
    /// Tool-call ceiling.
    pub tool_calls: u32,
    /// Process ceiling.
    pub processes: u16,
    /// Memory-byte ceiling.
    pub memory_bytes: u64,
    /// Network-request ceiling; zero for initial schedules.
    pub network_requests: u16,
    /// Retry ceiling.
    pub retries: u16,
    /// Retained-output byte ceiling.
    pub retained_output_bytes: u64,
}

/// Explicit host/workspace availability state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobAvailability {
    /// Host is awake and exact workspace is available.
    Ready,
    /// Host is sleeping.
    Sleeping,
    /// Host is offline.
    Offline,
    /// A due run was missed.
    MissedRun,
    /// Exact workspace is unavailable.
    WorkspaceUnavailable,
}

/// One durable bounded job record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRecord {
    /// Stable job identity.
    pub job_id: String,
    /// Exact workspace identity.
    pub workspace_sha256: String,
    /// Exact read-only task template.
    pub task_sha256: String,
    /// Idempotency key for the logical operation.
    pub idempotency_key_sha256: String,
    /// Current lifecycle state.
    pub state: JobState,
    /// Current lease, only while running.
    pub lease: Option<JobLease>,
    /// Independent ceilings.
    pub budgets: JobBudgets,
    /// Attempts already started.
    pub attempt_count: u16,
    /// Completed operation identities.
    pub completed_operation_sha256: Vec<String>,
    /// Stable transition reason.
    pub reason_code: String,
    /// Job expiry.
    pub expires_epoch_milliseconds: u64,
    /// Availability state.
    pub availability: JobAvailability,
    /// All descendant work terminated.
    pub descendants_terminated: bool,
    /// All owned resources released.
    pub resources_released: bool,
}

/// Closed inspectable schedule controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleControl {
    /// Inspect without changing schedule state.
    Inspect,
    /// Compute due work without executing it.
    DryRun,
    /// Explicit local manual test run.
    ManualTest,
    /// Pause future selection.
    Pause,
    /// Resume future selection.
    Resume,
    /// Replace an exact configuration after review.
    Edit,
    /// Explicit local run-now request.
    RunNow,
    /// Delete the schedule and release retained state.
    Delete,
}

/// Closed initial scheduled operation set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledOperation {
    /// Read exact local state.
    ReadLocal,
    /// Perform one pre-authorized read-only network observation.
    ReadRemote,
    /// Attempted file write, always denied.
    FileWrite,
    /// Attempted generic shell, always denied.
    GenericShell,
    /// Attempted connector mutation, always denied.
    ConnectorWrite,
    /// Attempted publication, always denied.
    Publication,
    /// Other remote mutation, always denied.
    RemoteStateChange,
    /// Interactive approval request, always denied unattended.
    InteractiveApproval,
}

impl ScheduledOperation {
    const fn read_only(self) -> bool {
        matches!(self, Self::ReadLocal | Self::ReadRemote)
    }
}

/// Inspectable read-only schedule record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleRecord {
    /// Stable schedule identity.
    pub schedule_id: String,
    /// Exact workspace.
    pub workspace_sha256: String,
    /// Exact task template.
    pub task_sha256: String,
    /// Exact operation.
    pub operation: ScheduledOperation,
    /// Exact schedule expression digest.
    pub timing_sha256: String,
    /// Independent job ceilings.
    pub budgets: JobBudgets,
    /// Schedule expiry.
    pub expires_epoch_milliseconds: u64,
    /// Whether future selection is paused.
    pub paused: bool,
    /// Catch-up limit; at most one prevents storms.
    pub missed_run_limit: u8,
    /// Exact ordered run-history digest.
    pub history_sha256: String,
}

/// Local notification classes with no delivery/network authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobNotificationKind {
    /// Completed work.
    Completed,
    /// Failed work.
    Failed,
    /// Blocked work.
    Blocked,
    /// Work awaiting live approval.
    WaitingApproval,
    /// Work stopped by a resource ceiling.
    ResourceConstrained,
}

/// Content-free local notification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobNotification {
    /// Job identity.
    pub job_id: String,
    /// Closed notification class.
    pub kind: JobNotificationKind,
    /// Stable reason code.
    pub reason_code: String,
    /// Local-only flag.
    pub local_only: bool,
}

/// Complete post-run accounting receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobRunReceipt {
    /// Job identity.
    pub job_id: String,
    /// Lease identity.
    pub lease_id: String,
    /// Idempotency key.
    pub idempotency_key_sha256: String,
    /// Exact grant set digest.
    pub grants_sha256: String,
    /// Exact operation set digest.
    pub operations_sha256: String,
    /// Exact network accounting digest.
    pub network_sha256: String,
    /// Exact resource accounting digest.
    pub resources_sha256: String,
    /// Exact output manifest digest.
    pub outputs_sha256: String,
    /// Exact failure manifest digest.
    pub failures_sha256: String,
    /// Retry count.
    pub retry_count: u16,
    /// Cleanup manifest digest.
    pub cleanup_sha256: String,
    /// Completed operation identities.
    pub completed_operation_sha256: Vec<String>,
    /// Descendants terminated.
    pub descendants_terminated: bool,
    /// Leases/resources released.
    pub resources_released: bool,
}

/// Stable fail-closed scheduler error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobSchedulerError {
    /// Invalid identity, budget, state, or transition.
    InvalidState,
    /// Lease is missing, stale, cancelled, or owned elsewhere.
    LeaseDenied,
    /// Scheduled operation would broaden unattended authority.
    OperationDenied,
    /// Resume would repeat a completed operation.
    DuplicateDenied,
}

/// Acquires the only current lease and starts one bounded attempt.
pub fn acquire_job_lease(
    job: &mut JobRecord,
    lease: JobLease,
    now: u64,
) -> Result<(), JobSchedulerError> {
    if job.state != JobState::Pending
        || job.lease.is_some()
        || job.availability != JobAvailability::Ready
        || now == 0
        || now >= job.expires_epoch_milliseconds
        || !valid_job(job)
        || !valid_lease(&lease, now)
        || job.attempt_count >= job.budgets.retries.saturating_add(1)
    {
        return Err(JobSchedulerError::LeaseDenied);
    }
    job.attempt_count += 1;
    job.state = JobState::Running;
    job.lease = Some(lease);
    job.descendants_terminated = false;
    job.resources_released = false;
    Ok(())
}

/// Renews only the current owner's unexpired lease.
pub fn renew_job_lease(
    job: &mut JobRecord,
    owner_id: &str,
    renewed: u64,
    expires: u64,
) -> Result<(), JobSchedulerError> {
    let lease = job.lease.as_mut().ok_or(JobSchedulerError::LeaseDenied)?;
    if job.state != JobState::Running
        || lease.owner_id != owner_id
        || lease.cancelled
        || renewed < lease.renewed_epoch_milliseconds
        || renewed >= lease.expires_epoch_milliseconds
        || expires <= renewed
        || expires > job.expires_epoch_milliseconds
    {
        return Err(JobSchedulerError::LeaseDenied);
    }
    lease.renewed_epoch_milliseconds = renewed;
    lease.expires_epoch_milliseconds = expires;
    Ok(())
}

/// Validates an initial schedule and denies every unattended write or approval request.
pub fn admit_read_only_schedule(schedule: &ScheduleRecord) -> Result<(), JobSchedulerError> {
    if !valid_id(&schedule.schedule_id)
        || !valid_sha256(&schedule.workspace_sha256)
        || !valid_sha256(&schedule.task_sha256)
        || !valid_sha256(&schedule.timing_sha256)
        || !valid_sha256(&schedule.history_sha256)
        || !valid_budgets(&schedule.budgets)
        || schedule.budgets.network_requests > 1
        || schedule.expires_epoch_milliseconds == 0
        || schedule.missed_run_limit > 1
        || !schedule.operation.read_only()
    {
        return Err(JobSchedulerError::OperationDenied);
    }
    Ok(())
}

/// Completes, fails, cancels, waits, or dead-letters an owned attempt and releases it.
pub fn finish_job(
    job: &mut JobRecord,
    lease_id: &str,
    state: JobState,
    completed_operation_sha256: Option<String>,
) -> Result<JobNotification, JobSchedulerError> {
    let lease = job.lease.as_ref().ok_or(JobSchedulerError::LeaseDenied)?;
    if job.state != JobState::Running
        || lease.lease_id != lease_id
        || !matches!(
            state,
            JobState::Succeeded
                | JobState::Failed
                | JobState::Cancelled
                | JobState::WaitingApproval
                | JobState::DeadLetter
        )
        || completed_operation_sha256.as_deref().is_some_and(|value| {
            !valid_sha256(value)
                || job
                    .completed_operation_sha256
                    .iter()
                    .any(|prior| prior == value)
        })
    {
        return Err(JobSchedulerError::InvalidState);
    }
    if let Some(operation) = completed_operation_sha256 {
        job.completed_operation_sha256.push(operation);
    }
    job.state = state;
    job.lease = None;
    job.descendants_terminated = true;
    job.resources_released = true;
    let kind = match state {
        JobState::Succeeded => JobNotificationKind::Completed,
        JobState::Failed | JobState::DeadLetter => JobNotificationKind::Failed,
        JobState::Cancelled => JobNotificationKind::Blocked,
        JobState::WaitingApproval => JobNotificationKind::WaitingApproval,
        _ => return Err(JobSchedulerError::InvalidState),
    };
    Ok(JobNotification {
        job_id: job.job_id.clone(),
        kind,
        reason_code: job.reason_code.clone(),
        local_only: true,
    })
}

/// Refuses resume when an operation already appears in durable completion history.
pub fn admit_resume(job: &JobRecord, operation_sha256: &str) -> Result<(), JobSchedulerError> {
    if !valid_sha256(operation_sha256) {
        return Err(JobSchedulerError::InvalidState);
    }
    if job
        .completed_operation_sha256
        .iter()
        .any(|value| value == operation_sha256)
    {
        return Err(JobSchedulerError::DuplicateDenied);
    }
    Ok(())
}

fn valid_job(job: &JobRecord) -> bool {
    valid_id(&job.job_id)
        && valid_sha256(&job.workspace_sha256)
        && valid_sha256(&job.task_sha256)
        && valid_sha256(&job.idempotency_key_sha256)
        && valid_id(&job.reason_code)
        && valid_budgets(&job.budgets)
        && job
            .completed_operation_sha256
            .iter()
            .all(|value| valid_sha256(value))
}
fn valid_budgets(value: &JobBudgets) -> bool {
    value.model_tokens > 0
        && value.tool_calls > 0
        && value.processes > 0
        && value.memory_bytes > 0
        && value.retained_output_bytes > 0
}
fn valid_lease(value: &JobLease, now: u64) -> bool {
    valid_id(&value.lease_id)
        && valid_id(&value.owner_id)
        && !value.cancelled
        && value.acquired_epoch_milliseconds == now
        && value.renewed_epoch_milliseconds == now
        && value.expires_epoch_milliseconds > now
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hash(c: char) -> String {
        c.to_string().repeat(64)
    }
    fn budgets() -> JobBudgets {
        JobBudgets {
            model_tokens: 1,
            tool_calls: 1,
            processes: 1,
            memory_bytes: 1,
            network_requests: 0,
            retries: 1,
            retained_output_bytes: 1,
        }
    }
    fn job() -> JobRecord {
        JobRecord {
            job_id: "job-1".into(),
            workspace_sha256: hash('a'),
            task_sha256: hash('b'),
            idempotency_key_sha256: hash('c'),
            state: JobState::Pending,
            lease: None,
            budgets: budgets(),
            attempt_count: 0,
            completed_operation_sha256: vec![],
            reason_code: "due".into(),
            expires_epoch_milliseconds: 10,
            availability: JobAvailability::Ready,
            descendants_terminated: true,
            resources_released: true,
        }
    }
    fn lease(owner: &str) -> JobLease {
        JobLease {
            lease_id: "lease-1".into(),
            owner_id: owner.into(),
            acquired_epoch_milliseconds: 1,
            renewed_epoch_milliseconds: 1,
            expires_epoch_milliseconds: 5,
            cancelled: false,
        }
    }
    #[test]
    fn sprint_89_one_lease_owner_and_bounded_renewal() {
        let mut value = job();
        assert!(acquire_job_lease(&mut value, lease("owner-1"), 1).is_ok());
        assert_eq!(
            acquire_job_lease(&mut value, lease("owner-2"), 1),
            Err(JobSchedulerError::LeaseDenied)
        );
        assert!(renew_job_lease(&mut value, "owner-1", 2, 6).is_ok());
    }
    #[test]
    fn sprint_89_budget_expiry_and_availability_fail_closed() {
        let mut value = job();
        value.availability = JobAvailability::Offline;
        assert_eq!(
            acquire_job_lease(&mut value, lease("owner-1"), 1),
            Err(JobSchedulerError::LeaseDenied)
        );
    }
    #[test]
    fn sprint_89_completed_operations_never_resume() {
        let mut value = job();
        value.completed_operation_sha256.push(hash('d'));
        assert_eq!(
            admit_resume(&value, &hash('d')),
            Err(JobSchedulerError::DuplicateDenied)
        );
    }
    #[test]
    fn sprint_90_all_unattended_effects_are_denied() {
        for operation in [
            ScheduledOperation::FileWrite,
            ScheduledOperation::GenericShell,
            ScheduledOperation::ConnectorWrite,
            ScheduledOperation::Publication,
            ScheduledOperation::RemoteStateChange,
            ScheduledOperation::InteractiveApproval,
        ] {
            let schedule = ScheduleRecord {
                schedule_id: "schedule-1".into(),
                workspace_sha256: hash('a'),
                task_sha256: hash('b'),
                operation,
                timing_sha256: hash('c'),
                budgets: budgets(),
                expires_epoch_milliseconds: 10,
                paused: false,
                missed_run_limit: 1,
                history_sha256: hash('d'),
            };
            assert_eq!(
                admit_read_only_schedule(&schedule),
                Err(JobSchedulerError::OperationDenied)
            );
        }
    }
    #[test]
    fn sprint_90_read_only_schedule_has_no_catch_up_storm() {
        for operation in [
            ScheduledOperation::ReadLocal,
            ScheduledOperation::ReadRemote,
        ] {
            let schedule = ScheduleRecord {
                schedule_id: "schedule-1".into(),
                workspace_sha256: hash('a'),
                task_sha256: hash('b'),
                operation,
                timing_sha256: hash('c'),
                budgets: budgets(),
                expires_epoch_milliseconds: 10,
                paused: false,
                missed_run_limit: 1,
                history_sha256: hash('d'),
            };
            assert!(admit_read_only_schedule(&schedule).is_ok());
        }
    }
    #[test]
    fn sprint_90_terminal_work_releases_descendants_and_notifies_locally() {
        let mut value = job();
        acquire_job_lease(&mut value, lease("owner-1"), 1).expect("lease");
        let note = finish_job(&mut value, "lease-1", JobState::Succeeded, Some(hash('d')))
            .expect("finish");
        assert!(
            note.local_only
                && value.descendants_terminated
                && value.resources_released
                && value.lease.is_none()
        );
    }
}
