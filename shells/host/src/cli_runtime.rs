//! Verified interactive CLI driver over the shared caller-neutral runtime transport.

use std::collections::BTreeMap;
use std::fmt;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CancellationId, RuntimeApprovalChallenge, RuntimeArtifactRef,
    RuntimeEvent, RuntimeEventCursor, RuntimeEventKind, RuntimeOutcome, RuntimeOutput,
    RuntimeRunRequest, RuntimeSessionMode,
};
use agentmage_kernel_engine::{
    job_control::{
        JobControlAction, JobControlDecision, JobControlRefusal, JobControlRequest, JobPhase,
    },
    model_routing::{
        AuthenticatedRoutingEnvelope, MeasuredRoutingService, NativeRoutingAuditView,
        RoutingAuthenticationVerifier,
    },
    runtime_artifact::{MAX_RUNTIME_ARTIFACT_BYTES, MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT},
    runtime_coordinator::{
        seal_runtime_run_request, verify_runtime_approval_challenge, verify_runtime_outcome,
        verify_runtime_run_request,
    },
    runtime_event::RuntimeEventSequence,
    runtime_loop::RuntimeSuspensionPoint,
};
use sha2::{Digest, Sha256};

use crate::coding_client::{
    CodingApprovalPort, CodingClientError, CodingEventSink, JobControlNotice,
    runtime_approval_response_with_selection,
};
use crate::runtime_transport::{
    RuntimeJobControl, RuntimeJobStatus, RuntimePrepareInput as NativeChatPrepareInput,
    RuntimeRunDeclarations, RuntimeTransportError as NativeChatRuntimeError,
    RuntimeTransportPort as NativeChatRuntimePort, RuntimeTransportStep as NativeChatRuntimeStep,
    is_suspension_event, resumed_run_request,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// Most control requests one cancellation, suspension or resumption sends: a
/// request built on a job revision that has since moved is refused as stale,
/// observed again and sent once more (Decisions 0120 and 0122).
const MAX_JOB_CONTROL_ATTEMPTS: usize = 4;
/// Most job control answers kept for one run's presentation.
const MAX_KEPT_JOB_CONTROLS: usize = 16;

/// Routes one spreadsheet artifact call through the shared host dispatcher without CLI semantics.
pub fn dispatch_cli_spreadsheet_artifact(
    kind: agentmage_capability_read_only::ArtifactToolKind,
    request_bytes: &[u8],
    service: &crate::spreadsheet_source_artifact::SpreadsheetSourceArtifactService,
    workspace_read_authorized: bool,
    signal: agentmage_capability_read_only::ArtifactExecutionSignal,
    ledger: &mut agentmage_capability_read_only::ArtifactAttemptLedger,
) -> Result<
    agentmage_capability_read_only::ArtifactResult,
    agentmage_capability_read_only::ArtifactDispatchError,
> {
    crate::spreadsheet_source_artifact::dispatch_spreadsheet_source_artifact(
        kind,
        request_bytes,
        service,
        workspace_read_authorized,
        signal,
        ledger,
    )
}

/// Stable failure from the installed-interface measured-routing adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractiveCliRoutingError {
    /// The kernel rejected authentication or the exact routing request.
    Routing,
    /// The installed interface could not present the verified native audit view.
    Presentation,
}

/// Presentation-only sink for one verified native model-routing audit view.
pub trait InteractiveCliRoutingSink {
    /// Presents the complete content-minimized decision without gaining model authority.
    fn present(&mut self, view: &NativeRoutingAuditView) -> Result<(), InteractiveCliRoutingError>;
}

/// Drives one authenticated installed-interface request through the kernel product router.
///
/// The host adapter cannot provide candidates, select a model, alter the audit view, retry a
/// refusal, or gain model/runtime authority. The service owns the exact catalog and appends the
/// canonical audit before this presentation-only adapter receives it.
pub fn drive_interactive_cli_routing<V, S>(
    service: &mut MeasuredRoutingService<V>,
    envelope: AuthenticatedRoutingEnvelope,
    sink: &mut S,
) -> Result<NativeRoutingAuditView, InteractiveCliRoutingError>
where
    V: RoutingAuthenticationVerifier,
    S: InteractiveCliRoutingSink,
{
    let view = service
        .route(envelope)
        .map_err(|_| InteractiveCliRoutingError::Routing)?
        .clone();
    sink.present(&view)?;
    Ok(view)
}

/// Stable content-free failure from the interactive CLI runtime boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractiveCliRuntimeError {
    /// The trusted host runtime was unavailable or refused the operation.
    Runtime,
    /// The host-framed request did not match the user's exact selected inputs.
    Request,
    /// The returned event, artifact, approval, or outcome evidence was inconsistent.
    Evidence,
    /// The protected approval channel could not return one exact decision.
    Approval,
    /// The terminal presentation sink could not accept verified output.
    Presentation,
    /// The local cancellation source failed before returning an exact identity.
    Cancellation,
    /// The terminal run could not be released from the bounded host registry.
    Release,
}

impl InteractiveCliRuntimeError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Runtime => "cli.runtime.unavailable",
            Self::Request => "cli.runtime.request_denied",
            Self::Evidence => "cli.runtime.evidence_denied",
            Self::Approval => "cli.runtime.approval_unavailable",
            Self::Presentation => "cli.runtime.presentation_failed",
            Self::Cancellation => "cli.runtime.cancellation_failed",
            Self::Release => "cli.runtime.release_failed",
        }
    }
}

impl fmt::Display for InteractiveCliRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for InteractiveCliRuntimeError {}

/// Protected local cancellation source observed only at coordinator boundaries.
pub trait InteractiveCliCancellationPort {
    /// Returns one exact cancellation-tree identity, or `None` to continue normally.
    fn poll(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError>;

    /// Returns a person's request to suspend or resume the run, if any
    /// (Decision 0122); `suspended` says whether the run is stopped at a safe
    /// boundary now. The default never asks for one.
    fn poll_job_control(
        &mut self,
        request: &RuntimeRunRequest,
        suspended: bool,
    ) -> Result<Option<InteractiveCliJobControl>, InteractiveCliRuntimeError> {
        let _ = (request, suspended);
        Ok(None)
    }
}

/// A person's request to pause or continue a run through the host's job
/// ledger (Decision 0122). Cancellation has its own identity-bearing path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractiveCliJobControl {
    /// Stop at the next committed safe boundary and keep the job resumable.
    Suspend,
    /// Continue a suspended run, or withdraw a pending suspension.
    Resume,
}

impl InteractiveCliJobControl {
    const fn action(self) -> JobControlAction {
        match self {
            Self::Suspend => JobControlAction::Suspend,
            Self::Resume => JobControlAction::Resume,
        }
    }
}

/// Cancellation source used when no terminal interrupt has been requested.
#[derive(Clone, Copy, Debug, Default)]
pub struct NeverCancelInteractiveCli;

impl InteractiveCliCancellationPort for NeverCancelInteractiveCli {
    fn poll(
        &mut self,
        _request: &RuntimeRunRequest,
    ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
        Ok(None)
    }
}

/// Why a run's job state is not shown (review F1 of `8a3a341e`). Each reason
/// names only what the driver observed, never a cause it cannot know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStateUnavailable {
    /// The host answered that it offers no job state for this run.
    NotOffered,
    /// The host refused the status request or did not answer it.
    NotAnswered,
    /// The host's answer did not describe this exact run.
    NotDescribed,
}

/// One job control request of this client and the host's verified answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptJobControl {
    /// The requested control.
    pub action: JobControlAction,
    /// The host's answer.
    pub answer: RuntimeJobControl,
}

/// Verified result returned after one interactive CLI run reaches a terminal state.
#[derive(Clone, Debug, PartialEq)]
pub struct InteractiveCliRuntimeResult {
    /// Exact host-framed request the run ended under. A run continued from a
    /// checkpoint inside the host ends under that cursor-bound request.
    pub request: RuntimeRunRequest,
    /// Canonical terminal outcome verified against the event stream.
    pub outcome: RuntimeOutcome,
    /// Complete verified path-free artifact references retained by the coordinator.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Full-payload verification summaries produced through bounded host-owned pages.
    pub verified_artifacts: Vec<VerifiedRuntimeArtifact>,
    /// Number of canonical events independently verified and presented.
    pub presented_events: u64,
    /// Host declarations about this run, read before release; `None` when the
    /// host offers none or they do not name this exact run (Decision 0116).
    pub declarations: Option<RuntimeRunDeclarations>,
    /// The run's reconciled job state, read before release (Decision 0120),
    /// or what the driver observed instead.
    pub job: Result<RuntimeJobStatus, JobStateUnavailable>,
    /// The host's answers to this client's job control requests, in order,
    /// at most sixteen.
    pub job_controls: Vec<KeptJobControl>,
}

/// Content-free result of reading and hashing one complete retained runtime artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedRuntimeArtifact {
    /// Exact verified path-free artifact reference.
    pub reference: RuntimeArtifactRef,
    /// Number of bounded pages read to verify the complete immutable payload.
    pub page_count: u64,
}

/// How long a suspended run's driver waits between looks at its local controls.
const SUSPENDED_POLL: std::time::Duration = std::time::Duration::from_millis(50);

/// Runs one interactive CLI operation through the same host-owned runtime port as native Chat.
///
/// The driver owns no model, tool, policy, grant, storage, filesystem, or process authority. It
/// submits the host-framed request unchanged, independently verifies every returned boundary,
/// presents only verified events, and returns exact approval or cancellation choices. A person's
/// suspension and resumption are job control requests; a run the host continues from its
/// checkpoint is followed under the cursor-bound request the driver derives and verifies itself
/// (Decision 0122).
pub fn drive_interactive_cli_runtime<P, A, S, C>(
    runtime: &mut P,
    input: NativeChatPrepareInput,
    approvals: &mut A,
    sink: &mut S,
    cancellation: &mut C,
) -> Result<InteractiveCliRuntimeResult, InteractiveCliRuntimeError>
where
    P: NativeChatRuntimePort + ?Sized,
    A: CodingApprovalPort,
    S: CodingEventSink,
    C: InteractiveCliCancellationPort,
{
    let request = runtime.prepare(input.clone()).map_err(map_runtime_error)?;
    if verify_prepared_request(&request, &input).is_err() {
        let _ = runtime.release(&request.run_id, &request.request_sha256);
        return Err(InteractiveCliRuntimeError::Request);
    }
    let mut step = match runtime.start(request.clone()) {
        Ok(step) => step,
        Err(error) => {
            let _ = runtime.release(&request.run_id, &request.request_sha256);
            return Err(map_runtime_error(error));
        }
    };
    let mut driver = RunDriver {
        verifier: InteractiveCliRuntimeVerifier::new(request)?,
        job_controls: Vec::new(),
        control_sequence: 0,
        announced: None,
    };
    let result = driver.drive(runtime, &mut step, approvals, sink, cancellation);
    if result.is_err() {
        best_effort_release(runtime, driver.verifier.request());
    }
    result
}

/// State of one driven run. The verifier holds the request the host holds
/// for the run.
struct RunDriver {
    verifier: InteractiveCliRuntimeVerifier,
    job_controls: Vec<KeptJobControl>,
    control_sequence: u64,
    /// The suspension boundary already presented.
    announced: Option<RuntimeEventCursor>,
}

impl RunDriver {
    fn drive<P, A, S, C>(
        &mut self,
        runtime: &mut P,
        step: &mut NativeChatRuntimeStep,
        approvals: &mut A,
        sink: &mut S,
        cancellation: &mut C,
    ) -> Result<InteractiveCliRuntimeResult, InteractiveCliRuntimeError>
    where
        P: NativeChatRuntimePort + ?Sized,
        A: CodingApprovalPort,
        S: CodingEventSink,
        C: InteractiveCliCancellationPort,
    {
        loop {
            self.verifier.accept(step)?;
            for event in &step.events {
                sink.present(event).map_err(map_client_error)?;
            }
            // Every event of this step is presented; a step kept while
            // suspended is accepted again without them.
            step.events.clear();
            if let Some(outcome) = step.outcome.take() {
                return self.finish(runtime, outcome, std::mem::take(&mut step.artifacts));
            }
            if let Some(point) = &step.suspended
                && self.announced.as_ref() != Some(&point.event_cursor)
            {
                sink.present_job_control(JobControlNotice::Suspended(point))
                    .map_err(map_client_error)?;
                self.announced = Some(point.event_cursor.clone());
            }
            let request = self.verifier.request().clone();
            // A person's stop precedes every other choice.
            if let Some(cancellation_id) = cancellation.poll(&request)? {
                *step = self.cancel_run(runtime, sink, step, &cancellation_id)?;
                continue;
            }
            if let Some(control) =
                cancellation.poll_job_control(&request, step.suspended.is_some())?
                && let Some(next) = self.control_run(runtime, sink, step, control)?
            {
                *step = next;
                continue;
            }
            if step.suspended.is_some() {
                // Nothing runs until a person resumes or cancels the run.
                std::thread::sleep(SUSPENDED_POLL);
                continue;
            }
            *step = if let Some(challenge) = step.approval.as_ref() {
                let (disposition, selection) = approvals
                    .decide_with_selection(challenge)
                    .map_err(map_client_error)?;
                let response =
                    runtime_approval_response_with_selection(challenge, disposition, selection);
                match cancellation.poll(&request)? {
                    Some(cancellation_id) => {
                        self.cancel_run(runtime, sink, step, &cancellation_id)?
                    }
                    None => runtime
                        .advance(
                            &request.run_id,
                            &request.request_sha256,
                            self.verifier.cursor().as_ref(),
                            Some(&response),
                        )
                        .map_err(map_runtime_error)?,
                }
            } else {
                // A live host may return a verified nonterminal progress page while inference or
                // a native tool continues on its owned worker. Keep the control path responsive
                // without granting any new action or busy-spinning the local IPC channel.
                std::thread::sleep(std::time::Duration::from_millis(10));
                self.advance(runtime)?
            };
        }
    }

    fn advance<P>(
        &self,
        runtime: &mut P,
    ) -> Result<NativeChatRuntimeStep, InteractiveCliRuntimeError>
    where
        P: NativeChatRuntimePort + ?Sized,
    {
        let request = self.verifier.request();
        runtime
            .advance(
                &request.run_id,
                &request.request_sha256,
                self.verifier.cursor().as_ref(),
                None,
            )
            .map_err(map_runtime_error)
    }

    fn finish<P>(
        &mut self,
        runtime: &mut P,
        outcome: RuntimeOutcome,
        artifacts: Vec<RuntimeArtifactRef>,
    ) -> Result<InteractiveCliRuntimeResult, InteractiveCliRuntimeError>
    where
        P: NativeChatRuntimePort + ?Sized,
    {
        let request = self.verifier.request().clone();
        let verified_artifacts = verify_complete_artifacts(runtime, &request, &artifacts)?;
        // Declarations are read while the ended run is still held; a
        // transport without them, or a failed read, shows them unavailable.
        let declarations = runtime
            .run_declarations(&request.run_id, &request.request_sha256)
            .ok()
            .and_then(|declarations| verified_run_declarations(declarations, &request));
        let job = match runtime.job_status(&request.run_id, &request.request_sha256) {
            Ok(status) if status.describes(&request) => Ok(status),
            Ok(_) => Err(JobStateUnavailable::NotDescribed),
            Err(NativeChatRuntimeError::JobControlUnavailable) => {
                Err(JobStateUnavailable::NotOffered)
            }
            Err(_) => Err(JobStateUnavailable::NotAnswered),
        };
        runtime
            .release(&request.run_id, &request.request_sha256)
            .map_err(|_| InteractiveCliRuntimeError::Release)?;
        Ok(InteractiveCliRuntimeResult {
            request,
            outcome,
            artifacts,
            verified_artifacts,
            presented_events: self.verifier.event_count(),
            declarations,
            job,
            job_controls: std::mem::take(&mut self.job_controls),
        })
    }

    /// Requests cancellation of the run through the host's durable job ledger
    /// (Decision 0120) and returns the next boundary. Only a transport that
    /// offers no job control at all is cancelled directly. A suspended run the
    /// host ends from its checkpoint is followed under the resumed request.
    fn cancel_run<P, S>(
        &mut self,
        runtime: &mut P,
        sink: &mut S,
        step: &NativeChatRuntimeStep,
        cancellation_id: &CancellationId,
    ) -> Result<NativeChatRuntimeStep, InteractiveCliRuntimeError>
    where
        P: NativeChatRuntimePort + ?Sized,
        S: CodingEventSink,
    {
        let digest: [u8; 32] = Sha256::digest(cancellation_id.as_str().as_bytes()).into();
        let prefix = format!("cancel-{}", &lower_hex(&digest)[..32]);
        match self.request_job_control(runtime, sink, step, JobControlAction::Cancel, &prefix)? {
            ControlOutcome::Unavailable(NativeChatRuntimeError::JobControlUnavailable) => {
                let request = self.verifier.request();
                runtime
                    .cancel(
                        &request.run_id,
                        &request.request_sha256,
                        cancellation_id.clone(),
                        self.verifier.cursor().as_ref(),
                    )
                    .map_err(map_runtime_error)
            }
            ControlOutcome::Unavailable(error) => Err(map_runtime_error(error)),
            ControlOutcome::Released => released(sink, step, JobControlAction::Cancel),
            ControlOutcome::Decided => self.advance(runtime),
        }
    }

    /// Sends a person's suspension or resumption through the host's job
    /// ledger (Decision 0122). A host that does not take it leaves the run
    /// unchanged, which is shown; the run then continues as before. Returns the
    /// next boundary when the host continued the run from its checkpoint, and
    /// fails when the host released the suspended run instead.
    fn control_run<P, S>(
        &mut self,
        runtime: &mut P,
        sink: &mut S,
        step: &NativeChatRuntimeStep,
        control: InteractiveCliJobControl,
    ) -> Result<Option<NativeChatRuntimeStep>, InteractiveCliRuntimeError>
    where
        P: NativeChatRuntimePort + ?Sized,
        S: CodingEventSink,
    {
        let action = control.action();
        let run_id = self.verifier.request().run_id.clone();
        let digest: [u8; 32] =
            Sha256::digest(format!("{}:{}", run_id.as_str(), self.control_sequence).as_bytes())
                .into();
        self.control_sequence = self
            .control_sequence
            .checked_add(1)
            .ok_or(InteractiveCliRuntimeError::Cancellation)?;
        let word = match control {
            InteractiveCliJobControl::Suspend => "suspend",
            InteractiveCliJobControl::Resume => "resume",
        };
        let prefix = format!("{word}-{}", &lower_hex(&digest)[..32]);
        let request_sha256 = self.verifier.request().request_sha256.clone();
        match self.request_job_control(runtime, sink, step, action, &prefix)? {
            ControlOutcome::Unavailable(_) => {
                sink.present_job_control(JobControlNotice::NotTaken { action })
                    .map_err(map_client_error)?;
                Ok(None)
            }
            ControlOutcome::Released => released(sink, step, action),
            ControlOutcome::Decided if self.verifier.request().request_sha256 != request_sha256 => {
                self.advance(runtime).map(Some)
            }
            ControlOutcome::Decided => Ok(None),
        }
    }

    /// Reads the job's status, sends one control request naming the revision
    /// it observed and keeps the verified answer. A stale refusal is observed
    /// again and sent under a new request identity, within a fixed bound. An
    /// answer about the request that continues a suspended run from its
    /// checkpoint rebinds the driver to that request.
    fn request_job_control<P, S>(
        &mut self,
        runtime: &mut P,
        sink: &mut S,
        step: &NativeChatRuntimeStep,
        action: JobControlAction,
        prefix: &str,
    ) -> Result<ControlOutcome, InteractiveCliRuntimeError>
    where
        P: NativeChatRuntimePort + ?Sized,
        S: CodingEventSink,
    {
        for attempt in 0..MAX_JOB_CONTROL_ATTEMPTS {
            let request = self.verifier.request().clone();
            let status = match runtime.job_status(&request.run_id, &request.request_sha256) {
                Ok(status) if status.describes(&request) => status,
                Ok(_) => return Err(InteractiveCliRuntimeError::Evidence),
                Err(error) if attempt == 0 => return Ok(ControlOutcome::Unavailable(error)),
                Err(error) => return Err(map_runtime_error(error)),
            };
            let control = JobControlRequest {
                schema_version: 1,
                job_id: request.run_id.as_str().to_owned(),
                request_id: format!("{prefix}-{attempt}"),
                action,
                observed_revision: status.job.revision,
            };
            let answer =
                match runtime.control_job(&request.run_id, &request.request_sha256, &control) {
                    Ok(answer) => answer,
                    // A suspended run the host no longer holds was released
                    // when it could not continue the run after the ledger
                    // decided (review F2 of `3c69304c`).
                    Err(_)
                        if step.suspended.is_some()
                            && matches!(
                                runtime.job_status(&request.run_id, &request.request_sha256),
                                Err(NativeChatRuntimeError::RunUnavailable)
                            ) =>
                    {
                        return Ok(ControlOutcome::Released);
                    }
                    Err(error) if action != JobControlAction::Cancel => {
                        return Ok(ControlOutcome::Unavailable(error));
                    }
                    Err(error) => return Err(map_runtime_error(error)),
                };
            if !answer.answers(&request, &control) {
                // Only a decision that resumed or cancelled a suspended run
                // continues it, under the request bound to its boundary.
                let point = step
                    .suspended
                    .as_ref()
                    .ok_or(InteractiveCliRuntimeError::Evidence)?;
                let resumed = self.verifier.resumed_request(point)?;
                let continued = matches!(
                    (action, answer.decision),
                    (
                        JobControlAction::Resume,
                        JobControlDecision::Applied {
                            phase: JobPhase::Queued,
                            ..
                        }
                    ) | (
                        JobControlAction::Cancel,
                        JobControlDecision::Applied {
                            phase: JobPhase::Cancelled,
                            ..
                        }
                    )
                );
                if !continued || !answer.answers(&resumed, &control) {
                    return Err(InteractiveCliRuntimeError::Evidence);
                }
                self.verifier.rebind(resumed)?;
                sink.present_job_control(JobControlNotice::Continued(point))
                    .map_err(map_client_error)?;
            }
            sink.present_job_control(JobControlNotice::Answered {
                action,
                answer: &answer,
            })
            .map_err(map_client_error)?;
            let stale = matches!(
                answer.decision,
                JobControlDecision::Refused {
                    refusal: JobControlRefusal::StaleRevision,
                    ..
                }
            );
            if self.job_controls.len() < MAX_KEPT_JOB_CONTROLS {
                self.job_controls.push(KeptJobControl { action, answer });
            }
            if !stale {
                break;
            }
        }
        Ok(ControlOutcome::Decided)
    }
}

/// What became of one job control request.
enum ControlOutcome {
    /// The host decided it through its ledger, or refused it as stale too often.
    Decided,
    /// The host's first status read failed, or it refused the request before
    /// its ledger, with this error.
    Unavailable(NativeChatRuntimeError),
    /// The host no longer holds the suspended run: it could not continue it
    /// after the request.
    Released,
}

/// Shows that the host released the suspended run of this step and ends the
/// drive: nothing continues the run here, and its job keeps the phase its
/// ledger recorded (review F2 of `3c69304c`).
fn released<S, T>(
    sink: &mut S,
    step: &NativeChatRuntimeStep,
    action: JobControlAction,
) -> Result<T, InteractiveCliRuntimeError>
where
    S: CodingEventSink,
{
    let point = step
        .suspended
        .as_ref()
        .ok_or(InteractiveCliRuntimeError::Evidence)?;
    sink.present_job_control(JobControlNotice::Released { action, point })
        .map_err(map_client_error)?;
    Err(InteractiveCliRuntimeError::Runtime)
}

fn verify_complete_artifacts<P>(
    runtime: &mut P,
    request: &RuntimeRunRequest,
    artifacts: &[RuntimeArtifactRef],
) -> Result<Vec<VerifiedRuntimeArtifact>, InteractiveCliRuntimeError>
where
    P: NativeChatRuntimePort + ?Sized,
{
    const PAGE_BYTES: u32 = 4 * 1024;
    let mut verified = Vec::with_capacity(artifacts.len());
    for reference in artifacts {
        let maximum_pages = reference.byte_size.div_ceil(u64::from(PAGE_BYTES));
        let mut offset = 0_u64;
        let mut page_count = 0_u64;
        let mut digest = Sha256::new();
        loop {
            let page = runtime
                .read_artifact_page(
                    &request.run_id,
                    &request.request_sha256,
                    reference,
                    offset,
                    PAGE_BYTES,
                )
                .map_err(map_runtime_error)?;
            page_count = page_count
                .checked_add(1)
                .ok_or(InteractiveCliRuntimeError::Evidence)?;
            let page_len = u64::try_from(page.bytes.len())
                .map_err(|_| InteractiveCliRuntimeError::Evidence)?;
            let next = offset
                .checked_add(page_len)
                .ok_or(InteractiveCliRuntimeError::Evidence)?;
            let page_digest: [u8; 32] = Sha256::digest(&page.bytes).into();
            let page_sha256 = lower_hex(&page_digest);
            if page.reference != *reference
                || page.offset != offset
                || page.bytes.is_empty()
                || page.bytes.len() > PAGE_BYTES as usize
                || page.page_sha256 != page_sha256
                || next > reference.byte_size
                || page_count > maximum_pages
                || page.complete != (next == reference.byte_size)
                || page.next_offset != (!page.complete).then_some(next)
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
            digest.update(&page.bytes);
            if page.complete {
                break;
            }
            offset = next;
        }
        let payload_digest: [u8; 32] = digest.finalize().into();
        if lower_hex(&payload_digest) != reference.payload_sha256 {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        verified.push(VerifiedRuntimeArtifact {
            reference: reference.clone(),
            page_count,
        });
    }
    Ok(verified)
}

fn lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

/// Keeps declarations only for this exact run. A recoverability declaration
/// whose seal, scope or summary does not verify, and an action history that
/// does not replay to its head or holds another owner's kinds (Decision
/// 0127), is dropped as unavailable.
fn verified_run_declarations(
    mut declarations: RuntimeRunDeclarations,
    request: &RuntimeRunRequest,
) -> Option<RuntimeRunDeclarations> {
    if declarations.schema_version != crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION
        || declarations.run_id != request.run_id
        || declarations.request_sha256 != request.request_sha256
    {
        return None;
    }
    if declarations.recoverability.as_ref().is_some_and(|report| {
        crate::coding_recoverability::verify_run_recoverability(
            report,
            request.session_id.as_str(),
            request.task.task_id.as_str(),
            request.run_id.as_str(),
        )
        .is_err()
    }) {
        declarations.recoverability = None;
    }
    for (history, chain) in [
        (
            &mut declarations.effect_history,
            crate::coding_action_history::RunActionChain::Effects,
        ),
        (
            &mut declarations.job_control_history,
            crate::coding_action_history::RunActionChain::JobControl,
        ),
    ] {
        if history.as_ref().is_some_and(|value| {
            crate::coding_action_history::verify_run_action_history(value, chain).is_err()
        }) {
            *history = None;
        }
    }
    Some(declarations)
}

fn best_effort_release<P>(runtime: &mut P, request: &RuntimeRunRequest)
where
    P: NativeChatRuntimePort + ?Sized,
{
    let _ = runtime.release(&request.run_id, &request.request_sha256);
}

fn verify_prepared_request(
    request: &RuntimeRunRequest,
    input: &NativeChatPrepareInput,
) -> Result<(), InteractiveCliRuntimeError> {
    verify_runtime_run_request(request).map_err(|_| InteractiveCliRuntimeError::Request)?;
    if !matches!(
        request.mode,
        RuntimeSessionMode::EphemeralReadOnly | RuntimeSessionMode::ControlledWrite
    ) || request.event_cursor.is_some() != input.resume
        || request.model_profile.profile_id.as_str() != input.profile_id
        || request.workspace_id.as_str() != input.workspace_id
        || request.task.objective != input.prompt
        || input
            .engineering_session_id
            .as_ref()
            .is_some_and(|session_id| &request.session_id != session_id)
    {
        return Err(InteractiveCliRuntimeError::Request);
    }
    Ok(())
}

#[derive(Clone)]
struct InteractiveCliRuntimeVerifier {
    request: RuntimeRunRequest,
    base_request_sha256: String,
    sequence: RuntimeEventSequence,
    events: BTreeMap<String, String>,
    artifacts: BTreeMap<String, RuntimeArtifactRef>,
    last_event: Option<RuntimeEvent>,
}

impl InteractiveCliRuntimeVerifier {
    fn new(request: RuntimeRunRequest) -> Result<Self, InteractiveCliRuntimeError> {
        let base_request_sha256 = if request.event_cursor.is_some() {
            let mut base = request.clone();
            base.event_cursor = None;
            seal_runtime_run_request(base)
                .map_err(|_| InteractiveCliRuntimeError::Evidence)?
                .request_sha256
        } else {
            request.request_sha256.clone()
        };
        Ok(Self {
            request,
            base_request_sha256,
            sequence: RuntimeEventSequence::new(),
            events: BTreeMap::new(),
            artifacts: BTreeMap::new(),
            last_event: None,
        })
    }

    fn accept(&mut self, step: &NativeChatRuntimeStep) -> Result<(), InteractiveCliRuntimeError> {
        if step.run_id != self.request.run_id
            || step.request_sha256 != self.request.request_sha256
            || (step.approval.is_some() && step.outcome.is_some())
            || (step.suspended.is_some() && (step.approval.is_some() || step.outcome.is_some()))
            || (step.events.is_empty()
                && self.sequence.event_count() == 0
                && step.approval.is_none()
                && step.outcome.is_none())
            || u64::try_from(step.events.len()).map_or(true, |event_count| {
                self.sequence
                    .event_count()
                    .checked_add(event_count)
                    .is_none_or(|total| total > u64::from(self.request.limits.max_events))
            })
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }

        let mut candidate = self.clone();
        for event in &step.events {
            candidate.accept_event(event)?;
        }
        candidate.verify_artifacts(&step.artifacts)?;
        match (&step.approval, &step.outcome) {
            (Some(challenge), None) => candidate.verify_approval(challenge)?,
            (None, Some(outcome)) => candidate.verify_outcome(outcome, &step.artifacts)?,
            (None, None) if !candidate.sequence.is_terminal() => {}
            _ => return Err(InteractiveCliRuntimeError::Evidence),
        }
        // A suspended run stopped right after committing this checkpoint.
        if let Some(point) = &step.suspended
            && !is_suspension_event(candidate.last_event.as_ref(), point)
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        *self = candidate;
        Ok(())
    }

    /// The request the host holds for the run.
    const fn request(&self) -> &RuntimeRunRequest {
        &self.request
    }

    /// The request that continues the run from the boundary it stopped at,
    /// derived here from the verified stream (Decision 0122). The boundary's
    /// commit event must be the last verified event.
    fn resumed_request(
        &self,
        point: &RuntimeSuspensionPoint,
    ) -> Result<RuntimeRunRequest, InteractiveCliRuntimeError> {
        if !is_suspension_event(self.last_event.as_ref(), point) {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        resumed_run_request(&self.request, &point.event_cursor)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)
    }

    /// Follows the run under the request that continues it. Its base request
    /// is the one this run started under, so the stream binding is unchanged.
    fn rebind(&mut self, resumed: RuntimeRunRequest) -> Result<(), InteractiveCliRuntimeError> {
        let mut base = resumed.clone();
        base.event_cursor = None;
        let base_request_sha256 = seal_runtime_run_request(base)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)?
            .request_sha256;
        if resumed.run_id != self.request.run_id
            || resumed.event_cursor.is_none()
            || base_request_sha256 != self.base_request_sha256
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        self.request = resumed;
        Ok(())
    }

    fn accept_event(&mut self, event: &RuntimeEvent) -> Result<(), InteractiveCliRuntimeError> {
        if event.run_id != self.request.run_id
            || event.session_id != self.request.session_id
            || event.task_id != self.request.task.task_id
            || event.policy_id != self.request.policy_id
            || (event.sequence == 0
                && !matches!(
                    &event.kind,
                    RuntimeEventKind::RunStarted { request_sha256 }
                        if request_sha256 == &self.base_request_sha256
                ))
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        self.sequence
            .push(event)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)?;
        if self
            .events
            .insert(
                event.event_id.as_str().to_owned(),
                event.event_sha256.clone(),
            )
            .is_some()
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        if let RuntimeEventKind::ArtifactCreated {
            artifact_id,
            manifest_sha256,
        } = &event.kind
        {
            let payload = event
                .payload_reference
                .as_ref()
                .ok_or(InteractiveCliRuntimeError::Evidence)?;
            if payload.artifact_id != *artifact_id
                || !valid_sha256(manifest_sha256)
                || self.artifacts.len() >= MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
            let reference = RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: artifact_id.clone(),
                manifest_sha256: manifest_sha256.clone(),
                payload_sha256: payload.sha256.clone(),
                byte_size: payload.byte_size,
                media_type: payload.media_type.clone(),
            };
            if self
                .artifacts
                .insert(artifact_id.as_str().to_owned(), reference)
                .is_some()
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
        }
        self.last_event = Some(event.clone());
        Ok(())
    }

    fn verify_artifacts(
        &self,
        artifacts: &[RuntimeArtifactRef],
    ) -> Result<(), InteractiveCliRuntimeError> {
        if artifacts.len() != self.artifacts.len()
            || artifacts.len() > MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        let mut observed = BTreeMap::new();
        for reference in artifacts {
            if reference.schema_version != CONTRACT_SCHEMA_VERSION
                || !valid_identifier(reference.artifact_id.as_str())
                || !valid_sha256(&reference.manifest_sha256)
                || !valid_sha256(&reference.payload_sha256)
                || reference.byte_size == 0
                || reference.byte_size > MAX_RUNTIME_ARTIFACT_BYTES
                || !valid_media_type(&reference.media_type)
                || observed
                    .insert(reference.artifact_id.as_str(), reference)
                    .is_some()
                || self.artifacts.get(reference.artifact_id.as_str()) != Some(reference)
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
        }
        Ok(())
    }

    fn verify_approval(
        &self,
        challenge: &RuntimeApprovalChallenge,
    ) -> Result<(), InteractiveCliRuntimeError> {
        verify_runtime_approval_challenge(challenge)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)?;
        let Some(RuntimeEvent {
            turn_id: Some(turn_id),
            operation_id: Some(operation_id),
            kind:
                RuntimeEventKind::PermissionRequested {
                    approval_id,
                    operation,
                    preview_sha256,
                    expires_at_epoch_ms,
                },
            ..
        }) = self.last_event.as_ref()
        else {
            return Err(InteractiveCliRuntimeError::Evidence);
        };
        if self.sequence.is_terminal()
            || challenge.run_id != self.request.run_id
            || challenge.task_id != self.request.task.task_id
            || challenge.turn_id != *turn_id
            || challenge.operation_id != *operation_id
            || challenge.approval_id != *approval_id
            || challenge.operation != *operation
            || challenge.preview_sha256 != *preview_sha256
            || challenge.expires_at_epoch_ms != *expires_at_epoch_ms
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        Ok(())
    }

    fn verify_outcome(
        &self,
        outcome: &RuntimeOutcome,
        artifacts: &[RuntimeArtifactRef],
    ) -> Result<(), InteractiveCliRuntimeError> {
        verify_runtime_outcome(outcome, &self.request)
            .map_err(|_| InteractiveCliRuntimeError::Evidence)?;
        let Some(RuntimeEvent {
            causation_event_id: Some(causation_event_id),
            previous_event_sha256,
            kind:
                RuntimeEventKind::RunTerminal {
                    state,
                    outcome_sha256,
                },
            ..
        }) = self.last_event.as_ref()
        else {
            return Err(InteractiveCliRuntimeError::Evidence);
        };
        if !self.sequence.is_terminal()
            || outcome.state != *state
            || outcome.outcome_sha256 != *outcome_sha256
            || outcome.prior_event_id != *causation_event_id
            || outcome.prior_event_sha256 != *previous_event_sha256
            || self.events.get(causation_event_id.as_str()) != Some(previous_event_sha256)
        {
            return Err(InteractiveCliRuntimeError::Evidence);
        }
        if let Some(RuntimeOutput::Artifact { reference }) = &outcome.output {
            let Some(retained) = artifacts
                .iter()
                .find(|item| item.artifact_id == reference.artifact_id)
            else {
                return Err(InteractiveCliRuntimeError::Evidence);
            };
            if retained.payload_sha256 != reference.sha256
                || retained.byte_size != reference.byte_size
                || retained.media_type != reference.media_type
            {
                return Err(InteractiveCliRuntimeError::Evidence);
            }
        }
        Ok(())
    }

    fn cursor(&self) -> Option<RuntimeEventCursor> {
        self.last_event.as_ref().map(|event| RuntimeEventCursor {
            run_id: event.run_id.clone(),
            event_id: event.event_id.clone(),
            sequence: event.sequence,
            event_sha256: event.event_sha256.clone(),
        })
    }

    const fn event_count(&self) -> u64 {
        self.sequence.event_count()
    }
}

const fn map_runtime_error(_error: NativeChatRuntimeError) -> InteractiveCliRuntimeError {
    InteractiveCliRuntimeError::Runtime
}

const fn map_client_error(error: CodingClientError) -> InteractiveCliRuntimeError {
    match error {
        CodingClientError::Approval => InteractiveCliRuntimeError::Approval,
        CodingClientError::Presentation => InteractiveCliRuntimeError::Presentation,
        CodingClientError::Runtime | CodingClientError::EventStream => {
            InteractiveCliRuntimeError::Evidence
        }
    }
}

fn valid_identifier(value: &str) -> bool {
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
        && value != ZERO_SHA256
}

fn valid_media_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'-' | b'.'))
}

#[cfg(all(test, feature = "source-artifacts", feature = "workflow-supervisor"))]
mod tests {
    use agentmage_kernel_contracts::{
        AgentStateKind, ApprovalId, ContextSensitivity, ContractPayload, CorrelationId, GrantId,
        GrantOperation, ModelRunId, OperationBinding, RuntimeApprovalDisposition,
        RuntimeApprovalPresentation, RuntimeEventId, RuntimeEventRetention,
        RuntimeEventRetentionKind, RuntimeOperationId, RuntimePermissionDisposition, RuntimeTurnId,
        SchemaId, SchemaReference, ToolCallId, ToolId, ToolRiskLevel,
    };
    use agentmage_kernel_engine::{
        model_routing::{
            MeasuredRoutingRequest, RoutingActionRisk, RoutingBudget, RoutingPlatform,
            RoutingResourceState, RoutingTaskClass,
        },
        runtime_coordinator::{seal_runtime_approval_challenge, seal_runtime_outcome},
        runtime_event::{runtime_event_persistence, seal_runtime_event},
    };

    use super::*;

    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    use agentmage_kernel_contracts::RuntimeArtifactId;

    struct RoutingVerifier;

    impl RoutingAuthenticationVerifier for RoutingVerifier {
        fn verify(&self, actor_id: &str, session_id: &str, authentication_sha256: &str) -> bool {
            actor_id == "actor-installed-1"
                && session_id == "session-installed-1"
                && authentication_sha256 == "e".repeat(64)
        }
    }

    #[derive(Default)]
    struct RoutingSink(Vec<NativeRoutingAuditView>);

    impl InteractiveCliRoutingSink for RoutingSink {
        fn present(
            &mut self,
            view: &NativeRoutingAuditView,
        ) -> Result<(), InteractiveCliRoutingError> {
            self.0.push(view.clone());
            Ok(())
        }
    }

    fn routing_envelope(authentication_sha256: String) -> AuthenticatedRoutingEnvelope {
        AuthenticatedRoutingEnvelope {
            actor_id: "actor-installed-1".to_owned(),
            session_id: "session-installed-1".to_owned(),
            authentication_sha256,
            request: MeasuredRoutingRequest {
                task_id: "task-installed-routing-1".to_owned(),
                task_class: RoutingTaskClass::Coding,
                action_risk: RoutingActionRisk::Moderate,
                budget: RoutingBudget::Standard,
                platform: RoutingPlatform::FedoraX86_64,
                required_context_tokens: 8_192,
                required_tool_proposals: 2,
                resources: RoutingResourceState {
                    available_memory_bytes: 32 * 1024 * 1024 * 1024,
                    available_context_tokens: 16_384,
                    current: true,
                },
                manual_profile_id: None,
                policy_sha256: "a".repeat(64),
                benchmark_generation_sha256: "b".repeat(64),
            },
        }
    }

    #[test]
    fn installed_interface_presents_only_the_kernel_owned_zero_profile_audit() {
        let mut service = MeasuredRoutingService::new(RoutingVerifier, Vec::new(), "c".repeat(64))
            .expect("routing service");
        let mut sink = RoutingSink::default();
        assert_eq!(
            drive_interactive_cli_routing(
                &mut service,
                routing_envelope("d".repeat(64)),
                &mut sink,
            ),
            Err(InteractiveCliRoutingError::Routing)
        );
        assert!(sink.0.is_empty());
        assert!(service.audit_log().is_empty());

        let view = drive_interactive_cli_routing(
            &mut service,
            routing_envelope("e".repeat(64)),
            &mut sink,
        )
        .expect("visible blocked routing view");
        assert_eq!(view.result_code, "model.routing.no-eligible-profile");
        assert!(view.selected_profile.is_none());
        assert!(!view.frontier_transfer);
        assert!(!view.model_confidence_used);
        assert_eq!(sink.0, [view]);
        assert_eq!(service.audit_log().len(), 1);
    }

    struct ScriptedPort {
        input: NativeChatPrepareInput,
        request: RuntimeRunRequest,
        start: Option<NativeChatRuntimeStep>,
        advance: Option<NativeChatRuntimeStep>,
        cancel: Option<NativeChatRuntimeStep>,
        released: usize,
        advances: usize,
        cancellations: usize,
        declarations: Option<RuntimeRunDeclarations>,
        job: Option<ScriptedJob>,
    }

    /// A host job ledger that refuses its first `stale_refusals` requests as
    /// stale, as if another client had moved the job, and then applies one.
    struct ScriptedJob {
        revision: u64,
        phase: agentmage_kernel_engine::job_control::JobPhase,
        stale_refusals: usize,
        requests: Vec<JobControlRequest>,
        foreign_status: bool,
        status_error: Option<NativeChatRuntimeError>,
    }

    impl ScriptedJob {
        fn new(stale_refusals: usize) -> Self {
            Self {
                revision: 1,
                phase: agentmage_kernel_engine::job_control::JobPhase::Running,
                stale_refusals,
                requests: Vec::new(),
                foreign_status: false,
                status_error: None,
            }
        }

        fn status(&self, request: &RuntimeRunRequest) -> RuntimeJobStatus {
            RuntimeJobStatus {
                schema_version: 1,
                run_id: request.run_id.clone(),
                request_sha256: if self.foreign_status {
                    "f".repeat(64)
                } else {
                    request.request_sha256.clone()
                },
                job: agentmage_kernel_engine::job_control::JobObservation {
                    job_id: request.run_id.as_str().to_owned(),
                    phase: self.phase,
                    revision: self.revision,
                    cancellation_requested: self.phase
                        != agentmage_kernel_engine::job_control::JobPhase::Running,
                    head_sha256: format!("{:064x}", self.revision),
                },
            }
        }
    }

    impl NativeChatRuntimePort for ScriptedPort {
        fn prepare(
            &mut self,
            input: NativeChatPrepareInput,
        ) -> Result<RuntimeRunRequest, NativeChatRuntimeError> {
            if input != self.input {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            Ok(self.request.clone())
        }

        fn start(
            &mut self,
            request: RuntimeRunRequest,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            if request != self.request {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            self.start
                .take()
                .ok_or(NativeChatRuntimeError::RunUnavailable)
        }

        fn advance(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            after_event_cursor: Option<&RuntimeEventCursor>,
            response: Option<&agentmage_kernel_contracts::RuntimeApprovalResponse>,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            self.advances += 1;
            verify_call(&self.request, run_id, request_sha256, after_event_cursor)?;
            // After an applied cancellation the host stops the run.
            if response.is_none()
                && let Some(job) = self.job.as_mut()
                && job.phase == agentmage_kernel_engine::job_control::JobPhase::Cancelling
            {
                job.phase = agentmage_kernel_engine::job_control::JobPhase::Cancelled;
                job.revision += 1;
                return self
                    .cancel
                    .take()
                    .ok_or(NativeChatRuntimeError::RunUnavailable);
            }
            let challenge = self.start.as_ref().and_then(|step| step.approval.as_ref());
            if challenge.is_some() || response.is_none() {
                return Err(NativeChatRuntimeError::ApprovalDenied);
            }
            self.advance
                .take()
                .ok_or(NativeChatRuntimeError::RunUnavailable)
        }

        fn cancel(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            _cancellation_id: CancellationId,
            after_event_cursor: Option<&RuntimeEventCursor>,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            self.cancellations += 1;
            verify_call(&self.request, run_id, request_sha256, after_event_cursor)?;
            self.cancel
                .take()
                .ok_or(NativeChatRuntimeError::RunUnavailable)
        }

        fn run_declarations(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
        ) -> Result<RuntimeRunDeclarations, NativeChatRuntimeError> {
            if run_id != &self.request.run_id
                || request_sha256 != self.request.request_sha256
                || self.released > 0
            {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            self.declarations
                .clone()
                .ok_or(NativeChatRuntimeError::RequestDenied)
        }

        fn job_status(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
        ) -> Result<RuntimeJobStatus, NativeChatRuntimeError> {
            if run_id != &self.request.run_id
                || request_sha256 != self.request.request_sha256
                || self.released > 0
            {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            let job = self
                .job
                .as_ref()
                .ok_or(NativeChatRuntimeError::JobControlUnavailable)?;
            if let Some(error) = job.status_error {
                return Err(error);
            }
            Ok(job.status(&self.request))
        }

        fn control_job(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            request: &JobControlRequest,
        ) -> Result<RuntimeJobControl, NativeChatRuntimeError> {
            use agentmage_kernel_engine::job_control::JobPhase;
            if run_id != &self.request.run_id || request_sha256 != self.request.request_sha256 {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            let job = self
                .job
                .as_mut()
                .ok_or(NativeChatRuntimeError::JobControlUnavailable)?;
            job.requests.push(request.clone());
            if job.stale_refusals > 0 {
                job.stale_refusals -= 1;
                // Another client's accepted request moved the job first.
                job.revision += 1;
            }
            let decision = if request.observed_revision == job.revision {
                job.revision += 1;
                job.phase = JobPhase::Cancelling;
                JobControlDecision::Applied {
                    revision: job.revision,
                    phase: job.phase,
                }
            } else {
                JobControlDecision::Refused {
                    refusal: JobControlRefusal::StaleRevision,
                    revision: job.revision,
                    phase: job.phase,
                }
            };
            Ok(RuntimeJobControl {
                decision,
                status: job.status(&self.request),
            })
        }

        fn release(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
        ) -> Result<(), NativeChatRuntimeError> {
            if run_id != &self.request.run_id || request_sha256 != self.request.request_sha256 {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            self.released += 1;
            Ok(())
        }
    }

    fn verify_call(
        request: &RuntimeRunRequest,
        run_id: &agentmage_kernel_contracts::RuntimeRunId,
        request_sha256: &str,
        cursor: Option<&RuntimeEventCursor>,
    ) -> Result<(), NativeChatRuntimeError> {
        if run_id != &request.run_id || request_sha256 != request.request_sha256 || cursor.is_none()
        {
            return Err(NativeChatRuntimeError::RequestDenied);
        }
        Ok(())
    }

    #[derive(Default)]
    struct RecordingSink(Vec<RuntimeEvent>);

    impl CodingEventSink for RecordingSink {
        fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
            self.0.push(event.clone());
            Ok(())
        }
    }

    struct PanicApproval;

    impl CodingApprovalPort for PanicApproval {
        fn decide(
            &mut self,
            _challenge: &RuntimeApprovalChallenge,
        ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
            panic!("terminal fixture must not request approval")
        }
    }

    struct DenyApproval;

    impl CodingApprovalPort for DenyApproval {
        fn decide(
            &mut self,
            _challenge: &RuntimeApprovalChallenge,
        ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
            Ok(RuntimeApprovalDisposition::Deny)
        }
    }

    struct CancelOnce(Option<CancellationId>);

    impl InteractiveCliCancellationPort for CancelOnce {
        fn poll(
            &mut self,
            _request: &RuntimeRunRequest,
        ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
            Ok(self.0.take())
        }
    }

    struct FailingSink;

    impl CodingEventSink for FailingSink {
        fn present(&mut self, _event: &RuntimeEvent) -> Result<(), CodingClientError> {
            Err(CodingClientError::Presentation)
        }
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn completed_shared_runtime_uses_exact_request_stream_outcome_and_release() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(step(&request, events.clone(), None, Some(outcome.clone()))),
            advance: None,
            cancel: None,
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: None,
        };
        let mut sink = RecordingSink::default();
        let result = drive_interactive_cli_runtime(
            &mut port,
            input,
            &mut PanicApproval,
            &mut sink,
            &mut NeverCancelInteractiveCli,
        )
        .expect("terminal runtime completes");

        assert_eq!(result.request, request);
        assert_eq!(result.outcome, outcome);
        assert_eq!(result.presented_events, events.len() as u64);
        assert_eq!(sink.0, events);
        assert_eq!(port.released, 1);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn run_declarations_are_kept_only_for_this_run_and_a_report_only_when_it_verifies() {
        // Review V3 of 8fbd2bc6: declarations naming another run are dropped
        // whole; a report for another run, or one whose seal no longer
        // matches, is dropped as unavailable while the context views stay.
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let report_for = |run_id: &str| {
            crate::coding_recoverability::assess_run_recoverability(
                request.session_id.as_str(),
                request.task.task_id.as_str(),
                run_id,
                &[crate::coding_recoverability::SessionEffect::Command {
                    operation_id: "command-1".to_owned(),
                }],
                &|_| None,
            )
            .unwrap()
        };
        let history = |kind| {
            use agentmage_kernel_engine::action_history::{
                ActionAuthorization, ActionOutcome, ActionRecordDraft,
            };
            let mut recorder = crate::coding_action_history::RunActionRecorder::new();
            recorder.record(Some(ActionRecordDraft {
                action_kind: kind,
                action_id: "command-1".to_owned(),
                authorization: ActionAuthorization::PersonDecision {
                    decision_sha256: "4".repeat(64),
                },
                effect_sha256: "5".repeat(64),
                outcome: ActionOutcome::Denied,
                reason_code: "runtime.coding.user-denied".to_owned(),
                evidence_sha256s: Vec::new(),
                recorded_at_epoch_ms: 1,
                retain_until_epoch_ms: 2,
            }));
            recorder.declare()
        };
        use agentmage_kernel_engine::action_history::ActionKind;
        let valid = RuntimeRunDeclarations {
            schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
            run_id: request.run_id.clone(),
            request_sha256: request.request_sha256.clone(),
            recoverability: Some(report_for(request.run_id.as_str())),
            context_inspections: Some(Vec::new()),
            effect_history: history(ActionKind::CommandRun),
            job_control_history: history(ActionKind::JobControl),
        };
        let mut foreign_run = valid.clone();
        foreign_run.run_id = agentmage_kernel_contracts::RuntimeRunId::from_raw("another-run");
        let mut foreign_request = valid.clone();
        foreign_request.request_sha256 = "f".repeat(64);
        let mut foreign_report = valid.clone();
        foreign_report.recoverability = Some(report_for("another-run"));
        let mut tampered = valid.clone();
        let report = tampered.recoverability.as_mut().unwrap();
        report.requires_reconciliation = !report.requires_reconciliation;
        let without_report = RuntimeRunDeclarations {
            recoverability: None,
            ..valid.clone()
        };
        // Decision 0127: an older schema is dropped whole; a history that does
        // not replay, or that holds another owner's kinds, is dropped alone.
        let older_schema = RuntimeRunDeclarations {
            schema_version: 1,
            ..valid.clone()
        };
        let mut tampered_history = valid.clone();
        tampered_history
            .effect_history
            .as_mut()
            .unwrap()
            .head
            .head_sha256 = "6".repeat(64);
        let without_effects = RuntimeRunDeclarations {
            effect_history: None,
            ..valid.clone()
        };
        let swapped = RuntimeRunDeclarations {
            effect_history: valid.job_control_history.clone(),
            job_control_history: valid.effect_history.clone(),
            ..valid.clone()
        };
        let without_histories = RuntimeRunDeclarations {
            effect_history: None,
            job_control_history: None,
            ..valid.clone()
        };
        for (declarations, expected) in [
            (Some(valid.clone()), Some(valid.clone())),
            (Some(foreign_run), None),
            (Some(foreign_request), None),
            (Some(older_schema), None),
            (Some(foreign_report), Some(without_report.clone())),
            (Some(tampered), Some(without_report)),
            (Some(tampered_history), Some(without_effects)),
            (Some(swapped), Some(without_histories)),
            (None, None),
        ] {
            let input = input(&request);
            let mut port = ScriptedPort {
                input: input.clone(),
                request: request.clone(),
                start: Some(step(&request, events.clone(), None, Some(outcome.clone()))),
                advance: None,
                cancel: None,
                released: 0,
                advances: 0,
                cancellations: 0,
                declarations,
                job: None,
            };
            let result = drive_interactive_cli_runtime(
                &mut port,
                input,
                &mut PanicApproval,
                &mut RecordingSink::default(),
                &mut NeverCancelInteractiveCli,
            )
            .expect("terminal runtime completes");
            assert_eq!(result.declarations, expected);
            assert_eq!(port.released, 1);
        }
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn protected_denial_and_cancellation_return_through_exact_runtime_cursor() {
        let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
        let fixture = approval_fixture(&request);
        let input = input(&request);
        let mut denied_port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting.clone()),
            advance: Some(fixture.denied.clone()),
            cancel: None,
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: None,
        };
        let mut denied_sink = RecordingSink::default();
        let denied = drive_interactive_cli_runtime(
            &mut denied_port,
            input.clone(),
            &mut DenyApproval,
            &mut denied_sink,
            &mut NeverCancelInteractiveCli,
        )
        .expect("exact denial completes");
        assert_eq!(denied.outcome.state, AgentStateKind::Declined);
        assert_eq!(denied_port.released, 1);

        let mut cancelled_port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting),
            advance: None,
            cancel: Some(fixture.cancelled),
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: None,
        };
        let mut cancelled_sink = RecordingSink::default();
        let cancelled = drive_interactive_cli_runtime(
            &mut cancelled_port,
            input,
            &mut PanicApproval,
            &mut cancelled_sink,
            &mut CancelOnce(Some(CancellationId::from_raw("cli-cancellation-0001"))),
        )
        .expect("exact cancellation completes");
        assert_eq!(cancelled.outcome.state, AgentStateKind::Cancelled);
        assert_eq!(cancelled_port.released, 1);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn a_cancellation_is_a_job_control_request_observed_again_after_a_stale_refusal() {
        // Decision 0120: a host that keeps a job ledger is never cancelled
        // directly; each request names the revision this client observed.
        use agentmage_kernel_engine::job_control::JobPhase;
        let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
        let fixture = approval_fixture(&request);
        let input = input(&request);
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting.clone()),
            advance: None,
            cancel: Some(fixture.cancelled.clone()),
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: Some(ScriptedJob::new(1)),
        };
        let result = drive_interactive_cli_runtime(
            &mut port,
            input.clone(),
            &mut PanicApproval,
            &mut RecordingSink::default(),
            &mut CancelOnce(Some(CancellationId::from_raw("cli-cancellation-0001"))),
        )
        .expect("a ledger cancellation completes");
        assert_eq!(result.outcome.state, AgentStateKind::Cancelled);
        assert_eq!(port.cancellations, 0);
        assert_eq!(port.released, 1);
        let requests = &port.job.as_ref().unwrap().requests;
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests
                .iter()
                .map(|control| control.observed_revision)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        for (attempt, control) in requests.iter().enumerate() {
            assert_eq!(control.action, JobControlAction::Cancel);
            assert_eq!(control.job_id, request.run_id.as_str());
            assert!(control.request_id.starts_with("cancel-"));
            assert!(control.request_id.ends_with(&format!("-{attempt}")));
            assert_eq!(control.request_id.len(), "cancel-".len() + 32 + 2);
        }
        assert_eq!(requests[0].request_id[..39], requests[1].request_id[..39]);
        assert_eq!(result.job_controls.len(), 2);
        assert!(
            result
                .job_controls
                .iter()
                .all(|kept| kept.action == JobControlAction::Cancel)
        );
        assert!(matches!(
            result.job_controls[0].answer.decision,
            JobControlDecision::Refused {
                refusal: JobControlRefusal::StaleRevision,
                revision: 2,
                ..
            }
        ));
        assert!(matches!(
            result.job_controls[1].answer.decision,
            JobControlDecision::Applied {
                revision: 3,
                phase: JobPhase::Cancelling,
            }
        ));
        let job = result
            .job
            .expect("the ended job's state is read before release");
        assert_eq!((job.job.phase, job.job.revision), (JobPhase::Cancelled, 4));

        // A status that does not describe this run is refused as evidence,
        // and nothing is requested or cancelled directly.
        let mut job = ScriptedJob::new(0);
        job.foreign_status = true;
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting.clone()),
            advance: None,
            cancel: Some(fixture.cancelled.clone()),
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: Some(job),
        };
        assert_eq!(
            drive_interactive_cli_runtime(
                &mut port,
                input.clone(),
                &mut PanicApproval,
                &mut RecordingSink::default(),
                &mut CancelOnce(Some(CancellationId::from_raw("cli-cancellation-0001"))),
            )
            .err(),
            Some(InteractiveCliRuntimeError::Evidence)
        );
        assert!(port.job.as_ref().unwrap().requests.is_empty());
        assert_eq!((port.cancellations, port.released), (0, 1));

        // A host that keeps a job but cannot answer for it now is never
        // cancelled directly instead.
        let mut job = ScriptedJob::new(0);
        job.status_error = Some(NativeChatRuntimeError::RuntimeEvidenceDenied);
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting.clone()),
            advance: None,
            cancel: Some(fixture.cancelled.clone()),
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: Some(job),
        };
        assert_eq!(
            drive_interactive_cli_runtime(
                &mut port,
                input.clone(),
                &mut PanicApproval,
                &mut RecordingSink::default(),
                &mut CancelOnce(Some(CancellationId::from_raw("cli-cancellation-0001"))),
            )
            .err(),
            Some(InteractiveCliRuntimeError::Runtime)
        );
        assert!(port.job.as_ref().unwrap().requests.is_empty());
        assert_eq!((port.cancellations, port.released), (0, 1));

        // A transport without job control is cancelled directly, and its
        // result has no job state.
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(fixture.awaiting),
            advance: None,
            cancel: Some(fixture.cancelled),
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: None,
        };
        let result = drive_interactive_cli_runtime(
            &mut port,
            input,
            &mut PanicApproval,
            &mut RecordingSink::default(),
            &mut CancelOnce(Some(CancellationId::from_raw("cli-cancellation-0001"))),
        )
        .expect("a direct cancellation completes");
        assert_eq!(result.outcome.state, AgentStateKind::Cancelled);
        assert_eq!(port.cancellations, 1);
        assert_eq!(result.job, Err(JobStateUnavailable::NotOffered));
        assert!(result.job_controls.is_empty());
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn cancellation_during_approval_precedes_every_approval_response() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        // Synthetic transport/event fixture exercising the real interactive
        // driver. This is not a native tool or model qualification run.
        struct StopDuringApproval<A> {
            actual: A,
            requested: Arc<AtomicBool>,
            decisions: usize,
        }
        impl<A: CodingApprovalPort> CodingApprovalPort for StopDuringApproval<A> {
            fn decide(
                &mut self,
                challenge: &RuntimeApprovalChallenge,
            ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
                self.decisions += 1;
                self.requested.store(true, Ordering::Release);
                self.actual.decide(challenge)
            }
        }
        struct ObserveStop {
            requested: Arc<AtomicBool>,
            delivered: usize,
        }
        impl InteractiveCliCancellationPort for ObserveStop {
            fn poll(
                &mut self,
                _: &RuntimeRunRequest,
            ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
                if self.requested.swap(false, Ordering::AcqRel) {
                    self.delivered += 1;
                    Ok(Some(CancellationId::from_raw("cli-cancellation-0001")))
                } else {
                    Ok(None)
                }
            }
        }

        for preauthorized in [false, true] {
            let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
            let fixture = approval_fixture(&request);
            let input = input(&request);
            let mut port = ScriptedPort {
                input: input.clone(),
                request,
                start: Some(fixture.awaiting),
                advance: Some(fixture.denied),
                cancel: Some(fixture.cancelled),
                released: 0,
                advances: 0,
                cancellations: 0,
                declarations: None,
                job: None,
            };
            let requested = Arc::new(AtomicBool::new(false));
            let mut approvals = StopDuringApproval {
                actual: crate::coding_development_client::terminal_approval_for_test(
                    preauthorized,
                    Arc::clone(&requested),
                ),
                requested: Arc::clone(&requested),
                decisions: 0,
            };
            let mut cancellation = ObserveStop {
                requested,
                delivered: 0,
            };
            let mut sink = RecordingSink::default();
            let result = drive_interactive_cli_runtime(
                &mut port,
                input,
                &mut approvals,
                &mut sink,
                &mut cancellation,
            )
            .expect("pending stop is sent through the canonical cancellation port");
            assert_eq!(result.outcome.state, AgentStateKind::Cancelled);
            assert_eq!(approvals.decisions, 1);
            assert_eq!(cancellation.delivered, 1);
            assert_eq!(port.advances, 0);
            assert_eq!(port.cancellations, 1);
            assert!(
                port.advance.is_some(),
                "the denial response must never be advanced"
            );
            assert!(
                port.cancel.is_none(),
                "exactly one cancellation step was consumed"
            );
            assert_eq!(port.released, 1);
            assert!(
                !sink
                    .0
                    .iter()
                    .any(|event| matches!(event.kind, RuntimeEventKind::PermissionDecided { .. }))
            );
        }
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn request_event_artifact_and_outcome_substitution_fail_before_success() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);

        let mut request_port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: None,
            advance: None,
            cancel: None,
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: None,
        };
        request_port.input.profile_id = "substituted-profile".to_owned();
        let substituted_input = request_port.input.clone();
        assert_eq!(
            drive_interactive_cli_runtime(
                &mut request_port,
                substituted_input,
                &mut PanicApproval,
                &mut RecordingSink::default(),
                &mut NeverCancelInteractiveCli,
            ),
            Err(InteractiveCliRuntimeError::Request)
        );
        assert_eq!(request_port.released, 1);

        let mut corrupt_events = events.clone();
        corrupt_events[0].event_sha256 = "f".repeat(64);
        assert_evidence_denied(
            &request,
            &input,
            corrupt_events,
            Vec::new(),
            outcome.clone(),
        );
        assert_evidence_denied(&request, &input, Vec::new(), Vec::new(), outcome.clone());

        let artifact = RuntimeArtifactRef {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("unannounced-artifact-0001"),
            manifest_sha256: "e".repeat(64),
            payload_sha256: "f".repeat(64),
            byte_size: 1,
            media_type: "text/plain".to_owned(),
        };
        assert_evidence_denied(
            &request,
            &input,
            events.clone(),
            vec![artifact],
            outcome.clone(),
        );

        let mut substituted_outcome = outcome;
        substituted_outcome.outcome_sha256 = "f".repeat(64);
        assert_evidence_denied(&request, &input, events, Vec::new(), substituted_outcome);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn story_50_2_confusion_and_interface_bypasses_never_present_or_complete() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);

        let mut replayed = events.clone();
        replayed.insert(1, events[0].clone());
        assert_evidence_denied(&request, &input, replayed, Vec::new(), outcome.clone());

        let mut foreign_session = events.clone();
        foreign_session[0].session_id =
            agentmage_kernel_contracts::SessionId::from_raw("session-foreign");
        foreign_session[0] =
            seal_runtime_event(foreign_session[0].clone()).expect("foreign event reseals");
        assert_evidence_denied(
            &request,
            &input,
            foreign_session,
            Vec::new(),
            outcome.clone(),
        );

        let mut malformed_terminal = events.clone();
        let terminal = malformed_terminal
            .last_mut()
            .expect("terminal event exists");
        let RuntimeEventKind::RunTerminal { state, .. } = &mut terminal.kind else {
            panic!("fixture must end in a terminal event");
        };
        *state = AgentStateKind::Failed;
        *terminal = seal_runtime_event(terminal.clone()).expect("terminal mutation reseals");
        assert_evidence_denied(
            &request,
            &input,
            malformed_terminal,
            Vec::new(),
            outcome.clone(),
        );

        let mut forged_run = step(&request, events.clone(), None, Some(outcome.clone()));
        forged_run.run_id = agentmage_kernel_contracts::RuntimeRunId::from_raw("run-foreign");
        assert_step_evidence_denied(&request, &input, forged_run);

        let mut forged_request = step(&request, events, None, Some(outcome));
        forged_request.request_sha256 = "f".repeat(64);
        assert_step_evidence_denied(&request, &input, forged_request);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn terminal_presentation_failure_releases_without_claiming_completion() {
        let (request, events, outcome, _) =
            crate::runtime_read_tests::completed_native_read_fixture();
        let input = input(&request);
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(step(&request, events, None, Some(outcome))),
            advance: None,
            cancel: None,
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: None,
        };

        assert_eq!(
            drive_interactive_cli_runtime(
                &mut port,
                input,
                &mut PanicApproval,
                &mut FailingSink,
                &mut NeverCancelInteractiveCli,
            ),
            Err(InteractiveCliRuntimeError::Presentation)
        );
        assert_eq!(port.released, 1);
    }

    fn assert_evidence_denied(
        request: &RuntimeRunRequest,
        input: &NativeChatPrepareInput,
        events: Vec<RuntimeEvent>,
        artifacts: Vec<RuntimeArtifactRef>,
        outcome: RuntimeOutcome,
    ) {
        assert_step_evidence_denied(
            request,
            input,
            NativeChatRuntimeStep {
                run_id: request.run_id.clone(),
                request_sha256: request.request_sha256.clone(),
                events,
                artifacts,
                approval: None,
                outcome: Some(outcome),
                suspended: None,
            },
        );
    }

    fn assert_step_evidence_denied(
        request: &RuntimeRunRequest,
        input: &NativeChatPrepareInput,
        step: NativeChatRuntimeStep,
    ) {
        let mut port = ScriptedPort {
            input: input.clone(),
            request: request.clone(),
            start: Some(step),
            advance: None,
            cancel: None,
            released: 0,
            advances: 0,
            cancellations: 0,
            declarations: None,
            job: None,
        };
        let mut sink = RecordingSink::default();
        assert_eq!(
            drive_interactive_cli_runtime(
                &mut port,
                input.clone(),
                &mut PanicApproval,
                &mut sink,
                &mut NeverCancelInteractiveCli,
            ),
            Err(InteractiveCliRuntimeError::Evidence)
        );
        assert!(sink.0.is_empty());
    }

    fn input(request: &RuntimeRunRequest) -> NativeChatPrepareInput {
        NativeChatPrepareInput {
            resume: false,
            record_session: false,
            slow_subscriber_probe: false,
            preauthorization: None,
            engineering_session_id: None,
            profile_id: request.model_profile.profile_id.as_str().to_owned(),
            expected_entry_sha256: "a".repeat(64),
            workspace_id: request.workspace_id.as_str().to_owned(),
            workspace_root: "/tmp/agentmage-cli-runtime-fixture".to_owned(),
            prompt: request.task.objective.clone(),
        }
    }

    fn step(
        request: &RuntimeRunRequest,
        events: Vec<RuntimeEvent>,
        approval: Option<RuntimeApprovalChallenge>,
        outcome: Option<RuntimeOutcome>,
    ) -> NativeChatRuntimeStep {
        NativeChatRuntimeStep {
            run_id: request.run_id.clone(),
            request_sha256: request.request_sha256.clone(),
            events,
            artifacts: Vec::new(),
            approval,
            outcome,
            suspended: None,
        }
    }

    struct ApprovalFixture {
        awaiting: NativeChatRuntimeStep,
        denied: NativeChatRuntimeStep,
        cancelled: NativeChatRuntimeStep,
    }

    fn approval_fixture(request: &RuntimeRunRequest) -> ApprovalFixture {
        let turn_id = RuntimeTurnId::from_raw("cli-turn-0001");
        let operation_id = RuntimeOperationId::from_raw("cli-operation-0001");
        let tool_call_id = ToolCallId::from_raw("cli-tool-call-0001");
        let challenge = seal_runtime_approval_challenge(RuntimeApprovalChallenge {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: request.run_id.clone(),
            task_id: request.task.task_id.clone(),
            turn_id: turn_id.clone(),
            operation_id: operation_id.clone(),
            tool_call_id: tool_call_id.clone(),
            approval_id: ApprovalId::from_raw("cli-approval-0001"),
            proposed_grant_id: GrantId::from_raw("cli-grant-0001"),
            operation: GrantOperation::WorkspaceRead,
            presentation: fixture_approval_presentation(),
            preview_sha256: "a".repeat(64),
            expires_at_epoch_ms: 50_000,
            challenge_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("challenge");
        let mut stream = FixtureStream::new(request);
        let prefix = vec![
            stream.event(
                RuntimeEventKind::RunStarted {
                    request_sha256: request.request_sha256.clone(),
                },
                None,
                None,
            ),
            stream.event(RuntimeEventKind::TurnStarted, Some(&turn_id), None),
            stream.event(
                RuntimeEventKind::ModelRequested {
                    model_run_id: ModelRunId::from_raw("cli-model-run-0001"),
                    request_sha256: "b".repeat(64),
                },
                Some(&turn_id),
                None,
            ),
            stream.event(
                RuntimeEventKind::ModelCompleted {
                    model_run_id: ModelRunId::from_raw("cli-model-run-0001"),
                    result_sha256: "c".repeat(64),
                },
                Some(&turn_id),
                None,
            ),
            stream.event(
                RuntimeEventKind::ToolRequested {
                    tool_call_id,
                    arguments_sha256: "d".repeat(64),
                },
                Some(&turn_id),
                Some(&operation_id),
            ),
            stream.event(
                RuntimeEventKind::PermissionRequested {
                    approval_id: challenge.approval_id.clone(),
                    operation: challenge.operation,
                    preview_sha256: challenge.preview_sha256.clone(),
                    expires_at_epoch_ms: challenge.expires_at_epoch_ms,
                },
                Some(&turn_id),
                Some(&operation_id),
            ),
        ];

        let awaiting = step(request, prefix, Some(challenge.clone()), None);

        let mut denied_stream = stream.clone();
        let mut denied_events = Vec::new();
        denied_events.push(denied_stream.event(
            RuntimeEventKind::PermissionDecided {
                approval_id: challenge.approval_id,
                disposition: RuntimePermissionDisposition::Deny,
                grant_id: None,
                decision_sha256: challenge.challenge_sha256,
            },
            Some(&turn_id),
            Some(&operation_id),
        ));
        denied_events.push(denied_stream.event(
            RuntimeEventKind::TurnCompleted {
                outcome_sha256: "e".repeat(64),
            },
            Some(&turn_id),
            None,
        ));
        let denied_outcome = denied_stream.outcome(AgentStateKind::Declined);
        denied_events.push(denied_stream.event(
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Declined,
                outcome_sha256: denied_outcome.outcome_sha256.clone(),
            },
            None,
            None,
        ));

        let mut cancelled_stream = stream;
        let cancellation_id = CancellationId::from_raw("cli-cancellation-0001");
        let mut cancelled_events = Vec::new();
        cancelled_events.push(cancelled_stream.event(
            RuntimeEventKind::CancellationRequested {
                cancellation_id: cancellation_id.clone(),
            },
            None,
            None,
        ));
        cancelled_events.push(cancelled_stream.event(
            RuntimeEventKind::CancellationObserved { cancellation_id },
            None,
            None,
        ));
        let cancelled_outcome = cancelled_stream.outcome(AgentStateKind::Cancelled);
        cancelled_events.push(cancelled_stream.event(
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Cancelled,
                outcome_sha256: cancelled_outcome.outcome_sha256.clone(),
            },
            None,
            None,
        ));

        ApprovalFixture {
            awaiting,
            denied: step(request, denied_events, None, Some(denied_outcome)),
            cancelled: step(request, cancelled_events, None, Some(cancelled_outcome)),
        }
    }

    fn fixture_approval_presentation() -> RuntimeApprovalPresentation {
        RuntimeApprovalPresentation {
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".to_owned(),
            display_name: "Fixture read".to_owned(),
            risk_level: ToolRiskLevel::Low,
            target_scope: "one fixture object".to_owned(),
            arguments: ContractPayload {
                schema: SchemaReference {
                    schema_id: SchemaId::from_raw("fixture.read.input"),
                    schema_version: 1,
                    schema_sha256: "a".repeat(64),
                },
                media_type: "application/json".to_owned(),
                bytes: b"{}".to_vec(),
                sha256: "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
                    .to_owned(),
            },
            declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
            single_use: true,
            timeout_ms: 1_000,
        }
    }

    #[derive(Clone)]
    struct FixtureStream {
        request: RuntimeRunRequest,
        correlation_id: CorrelationId,
        sequence: u64,
        previous_sha256: String,
        causation_event_id: Option<RuntimeEventId>,
        occurred_at_epoch_ms: u64,
    }

    impl FixtureStream {
        fn new(request: &RuntimeRunRequest) -> Self {
            Self {
                request: request.clone(),
                correlation_id: CorrelationId::from_raw("cli-correlation-0001"),
                sequence: 0,
                previous_sha256: ZERO_SHA256.to_owned(),
                causation_event_id: None,
                occurred_at_epoch_ms: 1_000,
            }
        }

        fn event(
            &mut self,
            kind: RuntimeEventKind,
            turn_id: Option<&RuntimeTurnId>,
            operation_id: Option<&RuntimeOperationId>,
        ) -> RuntimeEvent {
            self.occurred_at_epoch_ms += 1;
            let event_id = RuntimeEventId::from_raw(format!("cli-event-{:04}", self.sequence));
            let persistence = runtime_event_persistence(&kind);
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: event_id.clone(),
                run_id: self.request.run_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                turn_id: turn_id.cloned(),
                operation_id: operation_id.cloned(),
                correlation_id: self.correlation_id.clone(),
                causation_event_id: self.causation_event_id.clone(),
                sequence: self.sequence,
                occurred_at_epoch_ms: self.occurred_at_epoch_ms,
                sensitivity: ContextSensitivity::Internal,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Ephemeral,
                    expires_at_epoch_ms: None,
                },
                persistence,
                policy_id: self.request.policy_id.clone(),
                payload_reference: None,
                kind,
                previous_event_sha256: self.previous_sha256.clone(),
                event_sha256: ZERO_SHA256.to_owned(),
            })
            .expect("event");
            self.sequence += 1;
            self.previous_sha256.clone_from(&event.event_sha256);
            self.causation_event_id = Some(event_id);
            event
        }

        fn outcome(&self, state: AgentStateKind) -> RuntimeOutcome {
            seal_runtime_outcome(
                RuntimeOutcome {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    run_id: self.request.run_id.clone(),
                    session_id: self.request.session_id.clone(),
                    task_id: self.request.task.task_id.clone(),
                    request_sha256: self.request.request_sha256.clone(),
                    state,
                    turn_count: 1,
                    model_call_count: 1,
                    tool_call_count: 0,
                    prior_event_id: self.causation_event_id.clone().expect("prior event exists"),
                    prior_event_sha256: self.previous_sha256.clone(),
                    evidence: Vec::new(),
                    receipt_ids: Vec::new(),
                    unresolved_codes: Vec::new(),
                    output: None,
                    answer_evidence: None,
                    outcome_sha256: ZERO_SHA256.to_owned(),
                },
                &self.request,
            )
            .expect("outcome")
        }
    }

    /// A host over the suspended-run fixture (Decision 0122). Its job ledger
    /// follows the ledger's rules for one client; it suspends the run at the
    /// fixture's checkpoint on the next advance after an applied suspension,
    /// and continues it under the resumed request after a resumption or a
    /// cancellation of the suspended job.
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    struct SuspendingPort {
        fixture: crate::runtime_start_tests::SuspendedRunFixture,
        /// The request the host holds for the run.
        held: RuntimeRunRequest,
        phase: JobPhase,
        revision: u64,
        /// No job control at all.
        no_job_control: bool,
        /// Refuse suspension and resumption before the ledger.
        refuse_suspension: bool,
        /// Answer the resumption that continues the run about a request other
        /// than the resumed one.
        foreign_answer: bool,
        /// Name a boundary other than the one the run stopped at.
        foreign_point: bool,
        /// Refuse the resumption of the suspended run as stale, in an answer
        /// about its resumed request.
        refusal_about_resumed: bool,
        /// Fail to continue the suspended run after the ledger decided, and no
        /// longer hold it.
        fail_continuation: bool,
        /// The host no longer holds the run.
        gone: bool,
        started: bool,
        suspended: bool,
        cancel_signal: Option<CancellationId>,
        ended: bool,
        released: Vec<String>,
        advances: Vec<(String, Option<RuntimeEventCursor>)>,
        /// Advances made when the run stopped and when it continued.
        advances_at_suspension: Option<usize>,
        advances_at_continuation: Option<usize>,
        controls: Vec<JobControlRequest>,
        direct_cancellations: usize,
    }

    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    impl SuspendingPort {
        fn new() -> Self {
            let fixture = crate::runtime_start_tests::SuspendedRunFixture::new();
            let held = fixture.request.clone();
            Self {
                fixture,
                held,
                phase: JobPhase::Running,
                revision: 1,
                no_job_control: false,
                refuse_suspension: false,
                foreign_answer: false,
                foreign_point: false,
                refusal_about_resumed: false,
                fail_continuation: false,
                gone: false,
                started: false,
                suspended: false,
                cancel_signal: None,
                ended: false,
                released: Vec::new(),
                advances: Vec::new(),
                advances_at_suspension: None,
                advances_at_continuation: None,
                controls: Vec::new(),
                direct_cancellations: 0,
            }
        }

        fn status(&self, request: &RuntimeRunRequest) -> RuntimeJobStatus {
            RuntimeJobStatus {
                schema_version: 1,
                run_id: request.run_id.clone(),
                request_sha256: request.request_sha256.clone(),
                job: agentmage_kernel_engine::job_control::JobObservation {
                    job_id: request.run_id.as_str().to_owned(),
                    phase: self.phase,
                    revision: self.revision,
                    cancellation_requested: self.cancel_signal.is_some(),
                    head_sha256: format!("{:064x}", self.revision),
                },
            }
        }

        fn bound(&self, request_sha256: &str) -> Result<(), NativeChatRuntimeError> {
            if request_sha256 == self.held.request_sha256 {
                Ok(())
            } else {
                Err(NativeChatRuntimeError::RequestDenied)
            }
        }

        fn step(
            &self,
            events: Vec<RuntimeEvent>,
            outcome: Option<RuntimeOutcome>,
        ) -> NativeChatRuntimeStep {
            let mut point = self.fixture.point.clone();
            if self.foreign_point {
                point.checkpoint_sha256 = "6".repeat(64);
            }
            NativeChatRuntimeStep {
                run_id: self.held.run_id.clone(),
                request_sha256: self.held.request_sha256.clone(),
                events,
                artifacts: Vec::new(),
                approval: None,
                outcome,
                suspended: self.suspended.then_some(point),
            }
        }

        fn end(&mut self) -> NativeChatRuntimeStep {
            let (events, outcome) = self.fixture.ending(&self.held, self.cancel_signal.as_ref());
            self.ended = true;
            if self.phase == JobPhase::Cancelling {
                self.phase = JobPhase::Cancelled;
                self.revision += 1;
            } else if self.phase == JobPhase::Running {
                self.phase = JobPhase::Completed;
                self.revision += 1;
            }
            self.step(events, Some(outcome))
        }
    }

    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    impl NativeChatRuntimePort for SuspendingPort {
        fn prepare(
            &mut self,
            _input: NativeChatPrepareInput,
        ) -> Result<RuntimeRunRequest, NativeChatRuntimeError> {
            Ok(self.fixture.request.clone())
        }

        fn start(
            &mut self,
            request: RuntimeRunRequest,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            if request != self.held || std::mem::replace(&mut self.started, true) {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            Ok(self.step(self.fixture.stopped.clone(), None))
        }

        fn advance(
            &mut self,
            run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            after_event_cursor: Option<&RuntimeEventCursor>,
            response: Option<&agentmage_kernel_contracts::RuntimeApprovalResponse>,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            self.bound(request_sha256)?;
            if run_id != &self.held.run_id || response.is_some() || self.ended {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            self.advances
                .push((request_sha256.to_owned(), after_event_cursor.cloned()));
            if self.suspended {
                return Ok(self.step(Vec::new(), None));
            }
            if self.phase == JobPhase::Suspending {
                self.suspended = true;
                self.phase = JobPhase::Suspended;
                self.revision += 1;
                self.advances_at_suspension = Some(self.advances.len());
                return Ok(self.step(Vec::new(), None));
            }
            Ok(self.end())
        }

        fn cancel(
            &mut self,
            _run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            cancellation_id: CancellationId,
            _after_event_cursor: Option<&RuntimeEventCursor>,
        ) -> Result<NativeChatRuntimeStep, NativeChatRuntimeError> {
            self.bound(request_sha256)?;
            self.direct_cancellations += 1;
            self.cancel_signal = Some(cancellation_id);
            Ok(self.end())
        }

        fn job_status(
            &mut self,
            _run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
        ) -> Result<RuntimeJobStatus, NativeChatRuntimeError> {
            if self.gone {
                return Err(NativeChatRuntimeError::RunUnavailable);
            }
            self.bound(request_sha256)?;
            if self.no_job_control {
                return Err(NativeChatRuntimeError::JobControlUnavailable);
            }
            Ok(self.status(&self.held))
        }

        fn control_job(
            &mut self,
            _run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
            request: &JobControlRequest,
        ) -> Result<RuntimeJobControl, NativeChatRuntimeError> {
            if self.gone {
                return Err(NativeChatRuntimeError::RunUnavailable);
            }
            self.bound(request_sha256)?;
            if self.no_job_control {
                return Err(NativeChatRuntimeError::JobControlUnavailable);
            }
            if self.refuse_suspension && request.action != JobControlAction::Cancel {
                return Err(NativeChatRuntimeError::RequestDenied);
            }
            self.controls.push(request.clone());
            if self.refusal_about_resumed
                && self.suspended
                && request.action == JobControlAction::Resume
            {
                return Ok(RuntimeJobControl {
                    decision: JobControlDecision::Refused {
                        refusal: JobControlRefusal::StaleRevision,
                        revision: self.revision,
                        phase: self.phase,
                    },
                    status: self.status(&self.fixture.resumed()),
                });
            }
            let next = match (self.phase, request.action) {
                _ if request.observed_revision != self.revision => None,
                (JobPhase::Running, JobControlAction::Suspend) => Some(JobPhase::Suspending),
                (JobPhase::Suspended, JobControlAction::Resume) => Some(JobPhase::Queued),
                (JobPhase::Suspended, JobControlAction::Cancel) => Some(JobPhase::Cancelled),
                (JobPhase::Running, JobControlAction::Cancel) => Some(JobPhase::Cancelling),
                _ => None,
            };
            let Some(phase) = next else {
                return Ok(RuntimeJobControl {
                    decision: JobControlDecision::Refused {
                        refusal: JobControlRefusal::StaleRevision,
                        revision: self.revision,
                        phase: self.phase,
                    },
                    status: self.status(&self.held),
                });
            };
            self.revision += 1;
            self.phase = phase;
            let decision = JobControlDecision::Applied {
                revision: self.revision,
                phase,
            };
            if request.action == JobControlAction::Cancel {
                self.cancel_signal = Some(CancellationId::from_raw(format!(
                    "coding-job-cancel-{}",
                    self.revision
                )));
            }
            let continuing =
                self.suspended && matches!(phase, JobPhase::Queued | JobPhase::Cancelled);
            if continuing && self.fail_continuation {
                // The job keeps the phase the ledger decided; the host no
                // longer holds the run.
                self.gone = true;
                return Err(NativeChatRuntimeError::RuntimeFailed);
            }
            if continuing {
                // The host continues the run from its checkpoint.
                self.advances_at_continuation = Some(self.advances.len());
                self.suspended = false;
                self.held = self.fixture.resumed();
                if phase == JobPhase::Queued {
                    self.phase = JobPhase::Running;
                    self.revision += 1;
                }
            }
            let mut described = self.held.clone();
            if self.foreign_answer && continuing {
                described.request_sha256 = "7".repeat(64);
            }
            Ok(RuntimeJobControl {
                decision,
                status: self.status(&described),
            })
        }

        fn release(
            &mut self,
            _run_id: &agentmage_kernel_contracts::RuntimeRunId,
            request_sha256: &str,
        ) -> Result<(), NativeChatRuntimeError> {
            self.released.push(request_sha256.to_owned());
            if self.gone {
                return Err(NativeChatRuntimeError::RunUnavailable);
            }
            self.bound(request_sha256)
        }
    }

    /// A person who pauses the run at once and then, after the run was seen
    /// suspended for [`SUSPENDED_WAIT_POLLS`] polls, resumes or cancels it.
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    struct PauseThen {
        cancel: bool,
        asked_suspend: bool,
        /// Job control polls made while the run was suspended.
        suspended_polls: usize,
        done: bool,
    }

    /// Polls a suspended run waits for before it is resumed or cancelled.
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    const SUSPENDED_WAIT_POLLS: usize = 3;

    /// Polls after which a person still facing a suspended run gives up, so a
    /// driver that keeps waiting fails the test instead of hanging it.
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    const SUSPENDED_GIVE_UP_POLLS: usize = 40;

    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    impl InteractiveCliCancellationPort for PauseThen {
        fn poll(
            &mut self,
            _request: &RuntimeRunRequest,
        ) -> Result<Option<CancellationId>, InteractiveCliRuntimeError> {
            if self.cancel && self.suspended_polls >= SUSPENDED_WAIT_POLLS && !self.done {
                self.done = true;
                return Ok(Some(CancellationId::from_raw("cli-cancellation-suspended")));
            }
            Ok(None)
        }

        fn poll_job_control(
            &mut self,
            _request: &RuntimeRunRequest,
            suspended: bool,
        ) -> Result<Option<InteractiveCliJobControl>, InteractiveCliRuntimeError> {
            if !std::mem::replace(&mut self.asked_suspend, true) {
                return Ok(Some(InteractiveCliJobControl::Suspend));
            }
            if suspended {
                self.suspended_polls += 1;
                if self.suspended_polls > SUSPENDED_GIVE_UP_POLLS {
                    return Err(InteractiveCliRuntimeError::Cancellation);
                }
                if !self.done && !self.cancel && self.suspended_polls >= SUSPENDED_WAIT_POLLS {
                    self.done = true;
                    return Ok(Some(InteractiveCliJobControl::Resume));
                }
            }
            Ok(None)
        }
    }

    /// Records each event and job control notice in order.
    #[derive(Default)]
    struct NoticeSink {
        events: Vec<RuntimeEvent>,
        notices: Vec<String>,
    }

    impl CodingEventSink for NoticeSink {
        fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
            self.events.push(event.clone());
            Ok(())
        }

        fn present_job_control(
            &mut self,
            notice: JobControlNotice<'_>,
        ) -> Result<(), CodingClientError> {
            self.notices.push(match notice {
                JobControlNotice::Answered { action, answer } => format!(
                    "answered:{action:?}:{}",
                    match answer.decision {
                        JobControlDecision::Applied { phase, .. } => format!("applied:{phase:?}"),
                        JobControlDecision::Refused { refusal, .. } => {
                            format!("refused:{refusal:?}")
                        }
                    }
                ),
                JobControlNotice::NotTaken { action } => format!("not-taken:{action:?}"),
                JobControlNotice::Suspended(point) => {
                    format!("suspended:{}", point.event_cursor.sequence)
                }
                JobControlNotice::Continued(point) => {
                    format!("continued:{}", point.event_cursor.sequence)
                }
                JobControlNotice::Released { action, point } => {
                    format!("released:{action:?}:{}", point.event_cursor.sequence)
                }
            });
            Ok(())
        }
    }

    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn drive_suspending(
        port: &mut SuspendingPort,
        cancel: bool,
    ) -> (
        Result<InteractiveCliRuntimeResult, InteractiveCliRuntimeError>,
        NoticeSink,
        PauseThen,
    ) {
        let request = port.fixture.request.clone();
        let mut sink = NoticeSink::default();
        let mut person = PauseThen {
            cancel,
            asked_suspend: false,
            suspended_polls: 0,
            done: false,
        };
        let result = drive_interactive_cli_runtime(
            port,
            input(&request),
            &mut PanicApproval,
            &mut sink,
            &mut person,
        );
        (result, sink, person)
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_suspended_run_is_followed_under_the_resumed_request_it_derives() {
        // Decision 0122: a suspension and a resumption are job control
        // requests; the driver verifies the boundary the run stopped at,
        // derives the resumed request itself and follows the run under it.
        let mut port = SuspendingPort::new();
        let request = port.fixture.request.clone();
        let resumed = port.fixture.resumed();
        let point = port.fixture.point.clone();
        let (result, sink, person) = drive_suspending(&mut port, false);
        let result = result.expect("the resumed run completes");
        // Review F3 (c) of `3c69304c`: nothing advances the host while the
        // run is suspended, however long the person takes to resume it.
        assert_eq!(person.suspended_polls, SUSPENDED_WAIT_POLLS);
        assert!(port.advances_at_suspension.is_some());
        assert_eq!(port.advances_at_continuation, port.advances_at_suspension);
        let (ending, outcome) = port.fixture.ending(&resumed, None);
        assert_eq!(result.request, resumed);
        assert_eq!(result.outcome, outcome);
        let mut presented = port.fixture.stopped.clone();
        presented.extend(ending);
        assert_eq!(sink.events, presented);
        assert_eq!(
            sink.notices,
            [
                "answered:Suspend:applied:Suspending".to_owned(),
                format!("suspended:{}", point.event_cursor.sequence),
                format!("continued:{}", point.event_cursor.sequence),
                "answered:Resume:applied:Queued".to_owned(),
            ]
        );
        assert_eq!(
            result
                .job_controls
                .iter()
                .map(|kept| kept.action)
                .collect::<Vec<_>>(),
            [JobControlAction::Suspend, JobControlAction::Resume]
        );
        assert!(result.job_controls[1].answer.status.describes(&resumed));
        let job = result.job.expect("the job state is read before release");
        assert!(job.describes(&resumed));
        assert_eq!(job.job.phase, JobPhase::Completed);
        // Every call before the resumption named the run's request; the
        // continuation starts after the checkpoint under the resumed one.
        let first_resumed = port
            .advances
            .iter()
            .position(|(sha, _)| *sha == resumed.request_sha256)
            .unwrap();
        assert!(
            port.advances[..first_resumed]
                .iter()
                .all(|(sha, _)| *sha == request.request_sha256)
        );
        assert_eq!(
            port.advances[first_resumed].1.as_ref(),
            Some(&point.event_cursor)
        );
        assert_eq!(port.released.len(), 1);
        assert_eq!(port.released[0], resumed.request_sha256);
        let actions = port
            .controls
            .iter()
            .map(|control| (control.action, control.observed_revision))
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            [
                (JobControlAction::Suspend, 1),
                (JobControlAction::Resume, 3)
            ]
        );
        assert!(port.controls[0].request_id.starts_with("suspend-"));
        assert!(port.controls[1].request_id.starts_with("resume-"));
        assert_ne!(port.controls[0].request_id, port.controls[1].request_id);
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_cancelled_suspended_run_ends_under_the_resumed_request() {
        let mut port = SuspendingPort::new();
        let resumed = port.fixture.resumed();
        let (result, sink, person) = drive_suspending(&mut port, true);
        let result = result.expect("the cancelled run ends");
        assert_eq!(person.suspended_polls, SUSPENDED_WAIT_POLLS);
        assert!(port.advances_at_suspension.is_some());
        assert_eq!(port.advances_at_continuation, port.advances_at_suspension);
        assert_eq!(result.request, resumed);
        assert_eq!(result.outcome.state, AgentStateKind::Cancelled);
        assert_eq!(port.direct_cancellations, 0);
        assert_eq!(
            result
                .job_controls
                .iter()
                .map(|kept| kept.action)
                .collect::<Vec<_>>(),
            [JobControlAction::Suspend, JobControlAction::Cancel]
        );
        assert!(
            sink.notices
                .iter()
                .any(|notice| notice.starts_with("continued:"))
        );
        assert_eq!(
            sink.notices.last().unwrap(),
            "answered:Cancel:applied:Cancelled"
        );
        assert_eq!(result.job.unwrap().job.phase, JobPhase::Cancelled);
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_suspension_the_host_does_not_take_leaves_the_run_unchanged() {
        for (no_job_control, reason) in [
            (true, JobStateUnavailable::NotOffered),
            (false, JobStateUnavailable::NotAnswered),
        ] {
            let mut port = SuspendingPort::new();
            port.no_job_control = no_job_control;
            port.refuse_suspension = !no_job_control;
            let request = port.fixture.request.clone();
            let (result, sink, _) = drive_suspending(&mut port, false);
            let result = result.expect("the run continues and completes");
            assert_eq!(result.request, request);
            assert_eq!(result.outcome, port.fixture.ending(&request, None).1);
            assert_eq!(sink.notices, ["not-taken:Suspend".to_owned()]);
            assert!(result.job_controls.is_empty());
            if no_job_control {
                assert_eq!(result.job, Err(reason));
            } else {
                assert_eq!(result.job.unwrap().job.phase, JobPhase::Completed);
            }
        }
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_foreign_boundary_or_resumption_answer_is_refused_as_evidence() {
        // An answer about neither the run's request nor its resumed request,
        // and a boundary that is not the last verified event, never rebind
        // the driver.
        for foreign_answer in [true, false] {
            let mut port = SuspendingPort::new();
            port.foreign_answer = foreign_answer;
            port.foreign_point = !foreign_answer;
            let (result, _, _) = drive_suspending(&mut port, false);
            assert_eq!(result.err(), Some(InteractiveCliRuntimeError::Evidence));
            assert!(!port.ended);
            // A foreign boundary is refused when its step arrives, before any
            // resumption is sent; a foreign answer only after it.
            let actions = port
                .controls
                .iter()
                .map(|control| control.action)
                .collect::<Vec<_>>();
            if foreign_answer {
                assert_eq!(
                    actions,
                    [JobControlAction::Suspend, JobControlAction::Resume]
                );
            } else {
                assert_eq!(actions, [JobControlAction::Suspend]);
            }
        }
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_suspended_run_the_host_released_is_never_shown_as_unchanged() {
        // Review F2 of `3c69304c`: when the host cannot continue a suspended
        // run after the ledger decided its resumption or cancellation, the
        // driver shows that the host released it and ends, instead of
        // waiting on a run that nothing holds.
        for (cancel, action, phase) in [
            (false, JobControlAction::Resume, JobPhase::Queued),
            (true, JobControlAction::Cancel, JobPhase::Cancelled),
        ] {
            let mut port = SuspendingPort::new();
            port.fail_continuation = true;
            let request = port.fixture.request.clone();
            let point = port.fixture.point.clone();
            let (result, sink, _) = drive_suspending(&mut port, cancel);
            assert_eq!(result.err(), Some(InteractiveCliRuntimeError::Runtime));
            assert_eq!(
                sink.notices,
                [
                    "answered:Suspend:applied:Suspending".to_owned(),
                    format!("suspended:{}", point.event_cursor.sequence),
                    format!("released:{action:?}:{}", point.event_cursor.sequence),
                ]
            );
            assert!(port.gone);
            assert_eq!(port.phase, phase);
            assert_eq!(port.held, request);
            assert!(!port.ended);
            // Only the driver's best-effort release after a failure follows.
            assert_eq!(port.released, std::slice::from_ref(&request.request_sha256));
            assert_eq!(port.direct_cancellations, 0);
            assert_eq!(
                port.controls
                    .iter()
                    .map(|control| control.action)
                    .collect::<Vec<_>>(),
                [JobControlAction::Suspend, action]
            );
        }
    }

    #[test]
    #[cfg(all(
        target_os = "linux",
        feature = "source-artifacts",
        feature = "workflow-supervisor"
    ))]
    fn a_refused_resumption_about_the_resumed_request_never_rebinds() {
        // Review F3 (b) of `3c69304c`: an answer that describes the resumed
        // request rebinds the driver only for a decision that continued the
        // run; a refusal about it is refused as evidence.
        let mut port = SuspendingPort::new();
        port.refusal_about_resumed = true;
        let request = port.fixture.request.clone();
        let (result, sink, _) = drive_suspending(&mut port, false);
        assert_eq!(result.err(), Some(InteractiveCliRuntimeError::Evidence));
        assert!(
            !sink
                .notices
                .iter()
                .any(|notice| notice.starts_with("continued:"))
        );
        assert_eq!(port.held, request);
        assert!(port.suspended);
        assert!(!port.ended);
        assert_eq!(
            port.controls
                .iter()
                .map(|control| control.action)
                .collect::<Vec<_>>(),
            [JobControlAction::Suspend, JobControlAction::Resume]
        );
    }
}
