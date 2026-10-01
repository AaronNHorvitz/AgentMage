//! Executable CLI client for the explicitly activated disposable coding harness.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use agentmage_kernel_contracts::{
    AgentStateKind, RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeEvent,
    RuntimeHunkSelection, RuntimeRunLimits, WorkspaceId,
};
use agentmage_kernel_engine::context_inspection::render_context_inspection;
use agentmage_kernel_engine::job_control::{
    JobControlAction, JobControlDecision, JobControlRefusal, render_job_observation,
};
use agentmage_kernel_engine::run_progress::{
    MAX_PROGRESS_EVENTS, ProgressCeilings, project_run_progress, render_run_progress,
};
use agentmage_platform_linux::{
    LinuxDevelopmentBoundaryError, LinuxDevelopmentBoundaryErrorKind, LinuxDevelopmentConfirmation,
    LinuxDevelopmentConfirmationKind, LinuxDevelopmentHostProcess, LinuxDevelopmentInputLine,
    read_development_confirmation, read_development_line,
};

use crate::cli::{
    CliOutputFormat, CodingDevelopmentCliOptions, render_runtime_approval_human,
    render_runtime_event_human, render_runtime_event_json, render_runtime_outcome_human,
    render_runtime_outcome_json,
};
use crate::cli_runtime::{
    InteractiveCliCancellationPort, InteractiveCliJobControl, InteractiveCliRuntimeError,
    JobStateUnavailable, KeptJobControl, drive_interactive_cli_runtime,
};
use crate::coding_action_history::{
    ActionHistoryExportSelection, EndedRunActionHistories, RunActionChain, StoredRunChain,
    render_action_history_export, render_ended_run_histories, render_run_action_history,
    verified_ended_run_histories,
};
use crate::coding_change_review::{
    ChangeReview, ChangeReviewUnavailable, ReviewTarget, render_change_review, review_coding_write,
};
use crate::coding_client::{
    CodingApprovalPort, CodingClientError, CodingEventSink, JobControlNotice,
};
use crate::coding_development_activation::CodingDevelopmentActivation;
use crate::coding_development_runtime::CodingDevelopmentModel;
use crate::coding_doc_packs::{
    DocPackAnswer, DocPackCommand, DocPackRefusal, DocPackRequest, doc_pack_import_requests,
    doc_pack_refusal_exit, read_doc_pack_source, render_doc_pack_answer, render_doc_pack_refusal,
    verify_doc_pack_receipt,
};
use crate::coding_recoverability::render_recoverability;
use crate::coding_route::render_run_route_receipt;
use crate::coding_support_bundle::{
    InvocationObservation, ObservedRun, SupportBundleAnswer, SupportBundleOutcome,
    offer_support_bundle, render_support_bundle_outcome,
};
use crate::headless::ClientExitCode;
use crate::runtime_ipc::LinuxRuntimeIpcClient;
use crate::runtime_transport::{
    RuntimeJobControl, RuntimeJobStatus, RuntimePreauthorizedCommand, RuntimePrepareInput,
    RuntimeRunDeclarations, RuntimeSessionPreauthorization, RuntimeTransportPort,
};
use agentmage_kernel_engine::runtime_loop::RuntimeSuspensionPoint;

/// Stable content-free failure from the development-only CLI launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingDevelopmentClientError {
    /// The activation roots or marker were invalid.
    Activation,
    /// The exact sibling host could not be launched or authenticated.
    Transport,
    /// The native development boundary failed closed.
    HostBoundary(LinuxDevelopmentBoundaryErrorKind),
    /// The shared runtime or its returned evidence failed closed.
    Runtime,
    /// Terminal output could not be verified or rendered.
    Presentation,
}

impl CodingDevelopmentClientError {
    /// Returns one stable redacted diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Activation => "coding.development.client.activation-denied",
            Self::Transport => "coding.development.client.transport-failed",
            Self::HostBoundary(kind) => kind.code(),
            Self::Runtime => "coding.development.client.runtime-failed",
            Self::Presentation => "coding.development.client.presentation-failed",
        }
    }

    /// Returns the documented process exit class.
    #[must_use]
    pub const fn exit_code(self) -> ClientExitCode {
        match self {
            Self::Activation => ClientExitCode::AuthorityDenied,
            Self::HostBoundary(LinuxDevelopmentBoundaryErrorKind::StartupCancelled) => {
                ClientExitCode::Cancelled
            }
            Self::HostBoundary(LinuxDevelopmentBoundaryErrorKind::ConfirmationInputFailed) => {
                ClientExitCode::ProtocolMismatch
            }
            Self::Transport | Self::HostBoundary(_) | Self::Runtime => {
                ClientExitCode::ServiceUnavailable
            }
            Self::Presentation => ClientExitCode::ProtocolMismatch,
        }
    }
}

impl From<LinuxDevelopmentBoundaryError> for CodingDevelopmentClientError {
    fn from(error: LinuxDevelopmentBoundaryError) -> Self {
        Self::HostBoundary(error.kind())
    }
}

/// Runs one exact development invocation through a separately spawned authenticated host.
/// When a support bundle directory was named, the invocation's bundle is offered after it
/// ends, whatever the outcome, unless it was cancelled (Decision 0128).
pub fn run_coding_development(
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let mut cancellation = InstalledSignalCancellation::install()?
        .with_suspend_resume_probe(options.suspend_resume_probe);
    let mut runs = Vec::new();
    let result = run_invocation(options, output, &mut cancellation, &mut runs);
    if let Some(directory) = &options.support_bundle {
        offer_invocation_support_bundle(options, output, directory, &cancellation, runs, &result);
    }
    result
}

fn run_invocation(
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
    cancellation: &mut InstalledSignalCancellation,
    runs: &mut Vec<ObservedRun>,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let activation = CodingDevelopmentActivation::validate(
        &options.state_root,
        &options.disposable_root,
        &options.workspace_root,
    )
    .map_err(|_| CodingDevelopmentClientError::Activation)?;
    // Decision 0130: an ended run's histories and documentation packs are
    // served by the catalog host, which composes no run.
    if options.ended_run.is_some() || options.doc_pack.is_some() {
        return run_catalog_invocation(&activation, options, output, cancellation);
    }
    cancellation.check_startup()?;
    let mut child = LinuxDevelopmentHostProcess::launch(
        activation.state_root(),
        activation.disposable_root(),
        activation.workspace_root(),
        &options.scenario,
        &options.model,
        options.resume,
    )
    .map_err(CodingDevelopmentClientError::from)?;
    let result = run_with_child(&activation, options, output, &mut child, cancellation, runs);
    if result.is_err() {
        child
            .terminate_and_reap()
            .map_err(CodingDevelopmentClientError::from)?;
        return result;
    }
    let success = child
        .wait_success()
        .map_err(CodingDevelopmentClientError::from)?;
    if !success && result.is_ok() {
        return Err(CodingDevelopmentClientError::Transport);
    }
    result
}

/// Offers the support bundle of this invocation on standard error, after it
/// ended (Decision 0128). A cancelled invocation asks nothing. The bundle
/// never changes the invocation's result.
fn offer_invocation_support_bundle(
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
    directory: &std::path::Path,
    cancellation: &InstalledSignalCancellation,
    runs: Vec<ObservedRun>,
    result: &Result<ClientExitCode, CodingDevelopmentClientError>,
) {
    let observed = invocation_observation(&options.model, runs, result);
    let mut stderr = std::io::stderr().lock();
    let outcome = if invocation_cancelled(cancellation.requested.load(Ordering::Acquire), result) {
        SupportBundleOutcome::Skipped
    } else {
        let preview_id = format!(
            "support-bundle-{}-{}",
            current_epoch_ms().unwrap_or(0),
            std::process::id()
        );
        offer_support_bundle(
            &observed,
            directory,
            &preview_id,
            output,
            &mut stderr,
            &mut current_epoch_ms,
            &mut || support_bundle_answer(read_development_line(&cancellation.requested)),
        )
    };
    let _ = stderr.write_all(render_support_bundle_outcome(&outcome, output).as_bytes());
    let _ = stderr.flush();
}

/// The person's answer to a support bundle preview. Only the exact word
/// `yes` publishes; the platform reader has already removed the line's
/// terminator and surrounding whitespace. Every other line, end of input, a
/// too-long line and a read failure decline; a signal cancels.
fn support_bundle_answer<E>(line: Result<LinuxDevelopmentInputLine, E>) -> SupportBundleAnswer {
    match line {
        Ok(LinuxDevelopmentInputLine::Line(line)) if line == "yes" => {
            SupportBundleAnswer::Confirmed
        }
        Ok(LinuxDevelopmentInputLine::Cancelled) => SupportBundleAnswer::Cancelled,
        Ok(
            LinuxDevelopmentInputLine::Line(_)
            | LinuxDevelopmentInputLine::Ended
            | LinuxDevelopmentInputLine::TooLong,
        )
        | Err(_) => SupportBundleAnswer::Declined,
    }
}

/// What this invocation observed, for its support bundle: the selected
/// source, every run the host served and the failure that ended it, if any.
fn invocation_observation(
    model: &str,
    runs: Vec<ObservedRun>,
    result: &Result<ClientExitCode, CodingDevelopmentClientError>,
) -> InvocationObservation {
    let model = CodingDevelopmentModel::parse(model);
    InvocationObservation {
        profile_id: model
            .map_or("unknown", CodingDevelopmentModel::profile_id)
            .to_owned(),
        scripted: model == Some(CodingDevelopmentModel::Scripted),
        runs,
        failure_code: result.as_ref().err().map(|error| error.code()),
    }
}

/// Whether the invocation was cancelled, so its support bundle asks nothing:
/// a pending signal, a cancelled run or a cancelled startup.
fn invocation_cancelled(
    signal_pending: bool,
    result: &Result<ClientExitCode, CodingDevelopmentClientError>,
) -> bool {
    signal_pending
        || matches!(result, Ok(ClientExitCode::Cancelled))
        || matches!(
            result,
            Err(CodingDevelopmentClientError::HostBoundary(
                LinuxDevelopmentBoundaryErrorKind::StartupCancelled
            ))
        )
}

fn current_epoch_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
}

fn run_with_child(
    activation: &CodingDevelopmentActivation,
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
    child: &mut LinuxDevelopmentHostProcess,
    cancellation: &mut InstalledSignalCancellation,
    runs: &mut Vec<ObservedRun>,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let envelope = child
        .read_launch_envelope_cancellable(&cancellation.requested)
        .map_err(CodingDevelopmentClientError::from)?;
    cancellation.check_startup()?;
    let session = envelope
        .connect_development()
        .map_err(|_| CodingDevelopmentClientError::Transport)?;
    let mut runtime = LinuxRuntimeIpcClient::new(session);
    if options.stale_approval_probe {
        runtime = runtime.with_stale_approval_probe();
    } else if options.replay_approval_probe {
        runtime = runtime.with_replayed_approval_probe();
    } else if options.expired_cursor_probe {
        runtime = runtime.with_expired_cursor_probe();
    } else if options.artifact_integrity_probe {
        runtime = runtime.with_artifact_integrity_probe();
    }
    let workspace_id = format!("coding-development-{}", &activation.marker_sha256()[..24]);
    let mut approvals = TerminalApprovals {
        output,
        preauthorized: options.approve_this_run,
        delay_ms: options.approval_delay_ms,
        cancellation: Arc::clone(&cancellation.requested),
        review_workspace: Some(ReviewWorkspace {
            workspace_id: WorkspaceId::from_raw(workspace_id.clone()),
            root: activation.workspace_root().to_path_buf(),
        }),
    };
    let mut sink = TerminalEventSink::new(output, MAX_PROGRESS_EVENTS);
    cancellation.check_startup()?;
    let preauthorization =
        direct_session_preauthorization(options, &workspace_id, &cancellation.requested)?;
    let profile_id = CodingDevelopmentModel::parse(&options.model)
        .ok_or(CodingDevelopmentClientError::Activation)?
        .profile_id();
    let mut objectives = Vec::with_capacity(options.follow_ups.len() + 1);
    objectives.push(options.objective.as_str());
    objectives.extend(options.follow_ups.iter().map(String::as_str));
    let mut engineering_session_id = None;
    let mut final_exit = ClientExitCode::Success;
    cancellation.check_startup()?;
    for (objective_index, objective) in objectives.into_iter().enumerate() {
        let result = drive_interactive_cli_runtime(
            &mut runtime,
            RuntimePrepareInput {
                resume: options.resume,
                record_session: options.record_session,
                slow_subscriber_probe: options.slow_subscriber_probe,
                preauthorization: preauthorization.clone(),
                engineering_session_id: engineering_session_id.clone(),
                profile_id: profile_id.to_owned(),
                expected_entry_sha256: activation.marker_sha256().to_owned(),
                workspace_id: workspace_id.clone(),
                workspace_root: activation.workspace_root().to_string_lossy().into_owned(),
                prompt: objective.to_owned(),
            },
            &mut approvals,
            &mut sink,
            cancellation,
        );
        if let Err(error) = &result {
            if let Some(transport) = runtime.last_error() {
                eprintln!("{}", transport.code());
            }
            eprintln!("{}", error.code());
        }
        let result = result.map_err(|_| CodingDevelopmentClientError::Runtime)?;
        if let Some(expected) = engineering_session_id.as_ref()
            && expected != &result.request.session_id
        {
            return Err(CodingDevelopmentClientError::Runtime);
        }
        engineering_session_id = Some(result.request.session_id.clone());
        runs.push(ObservedRun {
            request_sha256: result.request.request_sha256.clone(),
            declarations: result.declarations.clone(),
            job: result.job.as_ref().ok().map(|status| status.job.clone()),
        });
        let outcome = match output {
            CliOutputFormat::Human => {
                render_runtime_outcome_human(&result.request, &result.outcome)
            }
            CliOutputFormat::Json => render_runtime_outcome_json(&result.request, &result.outcome),
        }
        .map_err(|_| CodingDevelopmentClientError::Presentation)?;
        let (export, export_notice) = render_requested_export(
            options.action_history_export,
            result.declarations.as_ref(),
            output,
        );
        let rendered = RenderedRunResult {
            artifacts: render_verified_artifacts(&result.verified_artifacts, output),
            export,
            export_notice,
            outcome,
            after_outcome: format!(
                "{}{}{}",
                sink.take_progress(&result.request.limits),
                render_run_declarations(result.declarations.as_ref(), output),
                render_job_state(
                    result.job.as_ref().map_err(|unavailable| *unavailable),
                    &result.job_controls,
                    output
                )
            ),
        };
        write_run_result(
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
            &rendered,
        )
        .map_err(|_| CodingDevelopmentClientError::Presentation)?;
        final_exit = match result.outcome.state {
            AgentStateKind::Success | AgentStateKind::NoOp => ClientExitCode::Success,
            AgentStateKind::Declined | AgentStateKind::Blocked => ClientExitCode::PolicyDenied,
            AgentStateKind::Cancelled => ClientExitCode::Cancelled,
            AgentStateKind::Exhausted => ClientExitCode::ResourceBound,
            _ => ClientExitCode::Uncertain,
        };
        if objective_index == 0
            && final_exit == ClientExitCode::Success
            && options.artifact_release_probe_before_follow_ups
        {
            let reference = result
                .artifacts
                .first()
                .ok_or(CodingDevelopmentClientError::Runtime)?;
            if runtime
                .release_artifact(
                    &result.request.run_id,
                    &result.request.request_sha256,
                    reference,
                )
                .is_ok()
            {
                return Err(CodingDevelopmentClientError::Runtime);
            }
            eprintln!(
                "coding.development.artifact-release-blocked-by-continuity-owner id={}",
                reference.artifact_id.as_str()
            );
        }
        if objective_index == 0
            && final_exit == ClientExitCode::Success
            && options.revoke_preauthorization_before_follow_ups
        {
            let contract = preauthorization
                .as_ref()
                .ok_or(CodingDevelopmentClientError::Runtime)?;
            runtime
                .revoke_session_preauthorization(
                    &result.request.session_id,
                    &contract.preauthorization_sha256,
                )
                .map_err(|_| CodingDevelopmentClientError::Runtime)?;
            eprintln!(
                "session_preauthorization_revoked id={} digest={}",
                contract.preauthorization_id, contract.preauthorization_sha256
            );
        }
        if final_exit != ClientExitCode::Success {
            break;
        }
    }
    runtime
        .shutdown()
        .map_err(|_| CodingDevelopmentClientError::Transport)?;
    Ok(final_exit)
}

fn direct_session_preauthorization(
    options: &CodingDevelopmentCliOptions,
    workspace_id: &str,
    cancellation: &AtomicBool,
) -> Result<Option<RuntimeSessionPreauthorization>, CodingDevelopmentClientError> {
    let requested = options.preauthorize_workspace_reads
        || !options.preauthorized_paths.is_empty()
        || !options.preauthorized_commands.is_empty();
    if !requested {
        return Ok(None);
    }
    if options.approve_this_run
        || options.preauthorization_budget == 0
        || options.preauthorization_minutes == 0
    {
        return Err(CodingDevelopmentClientError::Activation);
    }
    let mut writable_paths = options
        .preauthorized_paths
        .iter()
        .map(|path| {
            if path.starts_with('/') || path.contains('\\') {
                return Err(CodingDevelopmentClientError::Activation);
            }
            let components = path.split('/').map(str::to_owned).collect::<Vec<_>>();
            if components.is_empty()
                || components.iter().any(|component| {
                    component.is_empty() || matches!(component.as_str(), "." | "..")
                })
            {
                return Err(CodingDevelopmentClientError::Activation);
            }
            Ok(components)
        })
        .collect::<Result<Vec<_>, _>>()?;
    writable_paths.sort();
    if writable_paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CodingDevelopmentClientError::Activation);
    }
    let mut command_templates = options
        .preauthorized_commands
        .iter()
        .map(|selector| {
            let fields = selector.split('@').collect::<Vec<_>>();
            let [template_id, template_version, template_sha256] = fields.as_slice() else {
                return Err(CodingDevelopmentClientError::Activation);
            };
            Ok(RuntimePreauthorizedCommand {
                template_id: (*template_id).to_owned(),
                template_version: (*template_version).to_owned(),
                template_sha256: (*template_sha256).to_owned(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    command_templates.sort_by(|left, right| {
        (
            &left.template_id,
            &left.template_version,
            &left.template_sha256,
        )
            .cmp(&(
                &right.template_id,
                &right.template_version,
                &right.template_sha256,
            ))
    });
    if command_templates.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CodingDevelopmentClientError::Activation);
    }
    let approved_at_epoch_ms = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CodingDevelopmentClientError::Activation)?
            .as_millis(),
    )
    .map_err(|_| CodingDevelopmentClientError::Activation)?;
    let lifetime_ms = options
        .preauthorization_minutes
        .checked_mul(60_000)
        .ok_or(CodingDevelopmentClientError::Activation)?;
    let contract = RuntimeSessionPreauthorization {
        schema_version: 1,
        preauthorization_id: format!("coding-preauthorization-{approved_at_epoch_ms:016x}"),
        workspace_id: workspace_id.to_owned(),
        writable_paths,
        command_templates,
        allow_workspace_reads: options.preauthorize_workspace_reads,
        maximum_operations: options.preauthorization_budget,
        approved_at_epoch_ms,
        expires_at_epoch_ms: approved_at_epoch_ms
            .checked_add(lifetime_ms)
            .ok_or(CodingDevelopmentClientError::Activation)?,
        revoked_at_epoch_ms: None,
        preauthorization_sha256: "0".repeat(64),
    }
    .seal()
    .map_err(|_| CodingDevelopmentClientError::Activation)?;
    let rendered =
        serde_json::to_string(&contract).map_err(|_| CodingDevelopmentClientError::Presentation)?;
    eprintln!("session_preauthorization_proposed {rendered}");
    eprint!("Type preauthorize to approve this exact bounded session contract: ");
    match read_development_confirmation(
        LinuxDevelopmentConfirmationKind::SessionPreauthorization,
        cancellation,
    )
    .map_err(CodingDevelopmentClientError::from)?
    {
        LinuxDevelopmentConfirmation::Confirmed => {}
        LinuxDevelopmentConfirmation::Declined => {
            return Err(CodingDevelopmentClientError::Runtime);
        }
        LinuxDevelopmentConfirmation::Cancelled => {
            return Err(CodingDevelopmentClientError::HostBoundary(
                LinuxDevelopmentBoundaryErrorKind::StartupCancelled,
            ));
        }
    }
    eprintln!(
        "session_preauthorization_accepted id={} digest={} expires={} budget={}",
        contract.preauthorization_id,
        contract.preauthorization_sha256,
        contract.expires_at_epoch_ms,
        contract.maximum_operations,
    );
    Ok(Some(contract))
}

/// The development CLI's local controls: SIGINT or SIGTERM cancels the run,
/// SIGUSR1 asks to suspend it at its next safe boundary and SIGUSR2 asks to
/// resume it (Decision 0122). Each is a request the host decides.
struct InstalledSignalCancellation {
    requested: Arc<AtomicBool>,
    suspend: Arc<AtomicBool>,
    resume: Arc<AtomicBool>,
    sequence: u64,
    probe: SuspendResumeProbe,
    registrations: Vec<signal_hook::SigId>,
}

/// Development probe: one suspension at the first control point, then one
/// resumption once the run is suspended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuspendResumeProbe {
    Off,
    Suspend,
    AwaitSuspended,
    Done,
}

impl SuspendResumeProbe {
    fn next(&mut self, suspended: bool) -> Option<InteractiveCliJobControl> {
        match *self {
            Self::Suspend => {
                *self = Self::AwaitSuspended;
                Some(InteractiveCliJobControl::Suspend)
            }
            Self::AwaitSuspended if suspended => {
                *self = Self::Done;
                Some(InteractiveCliJobControl::Resume)
            }
            _ => None,
        }
    }
}

impl InstalledSignalCancellation {
    fn install() -> Result<Self, CodingDevelopmentClientError> {
        let requested = Arc::new(AtomicBool::new(false));
        let suspend = Arc::new(AtomicBool::new(false));
        let resume = Arc::new(AtomicBool::new(false));
        let mut registrations = Vec::with_capacity(4);
        for (signal, flag) in [
            (signal_hook::consts::SIGINT, &requested),
            (signal_hook::consts::SIGTERM, &requested),
            (signal_hook::consts::SIGUSR1, &suspend),
            (signal_hook::consts::SIGUSR2, &resume),
        ] {
            match signal_hook::flag::register(signal, Arc::clone(flag)) {
                Ok(registration) => registrations.push(registration),
                Err(_) => {
                    for registration in registrations {
                        signal_hook::low_level::unregister(registration);
                    }
                    return Err(CodingDevelopmentClientError::Transport);
                }
            }
        }
        Ok(Self {
            requested,
            suspend,
            resume,
            sequence: 0,
            probe: SuspendResumeProbe::Off,
            registrations,
        })
    }

    fn with_suspend_resume_probe(mut self, enabled: bool) -> Self {
        if enabled {
            self.probe = SuspendResumeProbe::Suspend;
        }
        self
    }

    fn check_startup(&self) -> Result<(), CodingDevelopmentClientError> {
        if self.requested.load(Ordering::Acquire) {
            Err(CodingDevelopmentClientError::HostBoundary(
                LinuxDevelopmentBoundaryErrorKind::StartupCancelled,
            ))
        } else {
            Ok(())
        }
    }
}

impl Drop for InstalledSignalCancellation {
    fn drop(&mut self) {
        for registration in self.registrations.drain(..) {
            signal_hook::low_level::unregister(registration);
        }
    }
}

impl InteractiveCliCancellationPort for InstalledSignalCancellation {
    fn poll(
        &mut self,
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
    ) -> Result<Option<agentmage_kernel_contracts::CancellationId>, InteractiveCliRuntimeError>
    {
        if !self.requested.swap(false, Ordering::AcqRel) {
            return Ok(None);
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(InteractiveCliRuntimeError::Cancellation)?;
        Ok(Some(agentmage_kernel_contracts::CancellationId::from_raw(
            format!(
                "cli-signal-{}-{:016x}",
                request.run_id.as_str(),
                self.sequence
            ),
        )))
    }

    fn poll_job_control(
        &mut self,
        _request: &agentmage_kernel_contracts::RuntimeRunRequest,
        suspended: bool,
    ) -> Result<Option<InteractiveCliJobControl>, InteractiveCliRuntimeError> {
        if let Some(control) = self.probe.next(suspended) {
            return Ok(Some(control));
        }
        if self.suspend.swap(false, Ordering::AcqRel) {
            return Ok(Some(InteractiveCliJobControl::Suspend));
        }
        if self.resume.swap(false, Ordering::AcqRel) {
            return Ok(Some(InteractiveCliJobControl::Resume));
        }
        Ok(None)
    }
}

struct TerminalEventSink {
    output: CliOutputFormat,
    /// Verified events of the current run, kept only to project its progress.
    run_events: Vec<RuntimeEvent>,
    /// Most events kept for one run.
    capacity: usize,
    /// Whether an event of the current run was not kept. A prefix of a valid
    /// stream verifies, so a truncated run is never projected.
    overflowed: bool,
}

impl TerminalEventSink {
    fn new(output: CliOutputFormat, capacity: usize) -> Self {
        Self {
            output,
            run_events: Vec::new(),
            capacity,
            overflowed: false,
        }
    }

    /// Truthful progress of the run just presented (Decision 0110); the buffer
    /// is then cleared for the next run.
    fn take_progress(&mut self, limits: &RuntimeRunLimits) -> String {
        let events = std::mem::take(&mut self.run_events);
        let overflowed = std::mem::take(&mut self.overflowed);
        let progress = if overflowed {
            Err(())
        } else {
            project_run_progress(
                &events,
                ProgressCeilings {
                    turns: Some(u64::from(limits.max_turns)),
                    model_calls: Some(u64::from(limits.max_model_calls)),
                    tool_calls: Some(u64::from(limits.max_tool_calls)),
                },
            )
            .map_err(|_| ())
        };
        match (self.output, progress) {
            (CliOutputFormat::Human, Ok(progress)) => render_run_progress(&progress),
            (CliOutputFormat::Json, Ok(progress)) => format!(
                "{}\n",
                serde_json::json!({"type": "run_progress", "available": true, "progress": progress})
            ),
            (CliOutputFormat::Human, Err(_)) => {
                "run progress unavailable: the presented stream is not one complete run\n"
                    .to_owned()
            }
            (CliOutputFormat::Json, Err(_)) => format!(
                "{}\n",
                serde_json::json!({"type": "run_progress", "available": false})
            ),
        }
    }
}

/// Renders the host's declarations about one ended run (Decision 0116): the
/// recoverability of its effects, the view of each composed context and the
/// run's action histories (Decision 0127). Each part the host could not
/// declare completely is shown as unavailable.
/// The requests of one documentation pack command, with the manifest an
/// import sends. An import reads and checks its pack before any host is
/// launched.
fn doc_pack_requests(
    command: &DocPackCommand,
) -> Result<
    (
        Option<agentmage_capability_knowledge::DocPackManifest>,
        Vec<DocPackRequest>,
    ),
    DocPackRefusal,
> {
    Ok(match command {
        DocPackCommand::Import {
            directory,
            allowed_licenses,
            refresh,
        } => {
            let source = read_doc_pack_source(directory)?;
            let requests = doc_pack_import_requests(&source, allowed_licenses, *refresh);
            (Some(source.manifest), requests)
        }
        DocPackCommand::List => (None, vec![DocPackRequest::List {}]),
        DocPackCommand::Inspect { pack_id } => (
            None,
            vec![DocPackRequest::Inspect {
                pack_id: pack_id.clone(),
            }],
        ),
        DocPackCommand::Delete { pack_id, version } => (
            None,
            vec![DocPackRequest::Delete {
                pack_id: pack_id.clone(),
                version: *version,
            }],
        ),
        DocPackCommand::Search {
            terms,
            pack_id,
            include_history,
        } => (
            None,
            vec![DocPackRequest::Search {
                terms: terms.clone(),
                pack_id: pack_id.clone(),
                include_history: *include_history,
            }],
        ),
    })
}

/// Runs one catalog operation through the catalog host (Decision 0130).
fn run_catalog_invocation(
    activation: &CodingDevelopmentActivation,
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
    cancellation: &mut InstalledSignalCancellation,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let json = output == CliOutputFormat::Json;
    let doc_pack = match options
        .doc_pack
        .as_deref()
        .map(doc_pack_requests)
        .transpose()
    {
        Ok(requests) => requests,
        Err(refusal) => {
            eprint!("{}", render_doc_pack_refusal(refusal, json));
            return Ok(doc_pack_refusal_exit(refusal));
        }
    };
    cancellation.check_startup()?;
    let mut child = LinuxDevelopmentHostProcess::launch_catalog(
        activation.state_root(),
        activation.disposable_root(),
        activation.workspace_root(),
    )
    .map_err(CodingDevelopmentClientError::from)?;
    let result = catalog_with_child(options, output, &mut child, cancellation, doc_pack);
    if result.is_err() {
        child
            .terminate_and_reap()
            .map_err(CodingDevelopmentClientError::from)?;
        return result;
    }
    let success = child
        .wait_success()
        .map_err(CodingDevelopmentClientError::from)?;
    if !success {
        return Err(CodingDevelopmentClientError::Transport);
    }
    result
}

fn catalog_with_child(
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
    child: &mut LinuxDevelopmentHostProcess,
    cancellation: &mut InstalledSignalCancellation,
    doc_pack: Option<(
        Option<agentmage_capability_knowledge::DocPackManifest>,
        Vec<DocPackRequest>,
    )>,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let envelope = child
        .read_launch_envelope_cancellable(&cancellation.requested)
        .map_err(CodingDevelopmentClientError::from)?;
    cancellation.check_startup()?;
    let session = envelope
        .connect_development()
        .map_err(|_| CodingDevelopmentClientError::Transport)?;
    let mut runtime = LinuxRuntimeIpcClient::new(session);
    let shown = match (&options.ended_run, doc_pack) {
        (Some(run_id), None) => {
            show_ended_run(&mut runtime, run_id, options.action_history_export, output)
        }
        (None, Some((sent, requests))) => run_doc_pack_requests(
            &mut runtime,
            sent.as_ref(),
            requests,
            output,
            &cancellation.requested,
        ),
        _ => Err(CodingDevelopmentClientError::Activation),
    };
    runtime
        .shutdown()
        .map_err(|_| CodingDevelopmentClientError::Transport)?;
    shown
}

/// Sends each documentation pack request in order and writes each answer.
/// A staged answer and each accepted chunk must acknowledge exactly what was
/// sent, and an import receipt is kept only when it names the sent manifest
/// and recomputes. A refusal ends the operation with its exit class; a
/// cancellation between requests stops sending, and the host discards a
/// staged import when the session ends.
fn run_doc_pack_requests(
    runtime: &mut impl RuntimeTransportPort,
    sent: Option<&agentmage_capability_knowledge::DocPackManifest>,
    requests: Vec<DocPackRequest>,
    output: CliOutputFormat,
    cancellation: &AtomicBool,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let json = output == CliOutputFormat::Json;
    for request in requests {
        if cancellation.load(Ordering::Acquire) {
            eprint!(
                "{}",
                if json {
                    "{\"type\":\"doc_pack_cancelled\"}\n"
                } else {
                    "doc pack operation cancelled; nothing was imported\n"
                }
            );
            return Ok(ClientExitCode::Cancelled);
        }
        let expected = match &request {
            DocPackRequest::ImportChunk { path, offset, text } => {
                Some((path.clone(), offset + text.len() as u64))
            }
            _ => None,
        };
        let answer = runtime
            .doc_pack(request)
            .inspect_err(|error| eprintln!("{}", error.code()))
            .map_err(|_| CodingDevelopmentClientError::Runtime)?;
        let acknowledged = match (&answer, &expected, sent) {
            (DocPackAnswer::Refused { refusal }, _, _) => {
                eprint!("{}", render_doc_pack_refusal(*refusal, json));
                return Ok(doc_pack_refusal_exit(*refusal));
            }
            (
                DocPackAnswer::ImportStaged {
                    pack_id,
                    version,
                    manifest_sha256,
                },
                None,
                Some(sent),
            ) => {
                *pack_id == sent.pack_id
                    && *version == sent.version
                    && *manifest_sha256 == sent.manifest_sha256
            }
            (
                DocPackAnswer::ChunkAccepted {
                    path,
                    received_bytes,
                },
                Some((sent_path, sent_bytes)),
                Some(_),
            ) => path == sent_path && received_bytes == sent_bytes,
            (DocPackAnswer::Imported { receipt, .. }, None, Some(sent)) => {
                verify_doc_pack_receipt(receipt, sent)
            }
            (
                DocPackAnswer::Listed { .. }
                | DocPackAnswer::Inspected { .. }
                | DocPackAnswer::Deleted { .. }
                | DocPackAnswer::Found { .. },
                None,
                None,
            ) => true,
            _ => false,
        };
        if !acknowledged {
            eprintln!("coding.development.client.doc-pack-answer-denied");
            return Err(CodingDevelopmentClientError::Presentation);
        }
        print!("{}", render_doc_pack_answer(&answer, json));
    }
    Ok(ClientExitCode::Success)
}

/// Reads one ended run's stored histories back from the host and writes them
/// on standard output, a requested export first (Decision 0129). An answer
/// for another run or schema is refused; a chain that does not verify is
/// shown as unavailable.
fn show_ended_run(
    runtime: &mut impl RuntimeTransportPort,
    run_id: &str,
    export: Option<ActionHistoryExportSelection>,
    output: CliOutputFormat,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let answer = runtime
        .ended_run_action_histories(&agentmage_kernel_contracts::RuntimeRunId::from_raw(run_id))
        .inspect_err(|error| eprintln!("{}", error.code()))
        .map_err(|_| CodingDevelopmentClientError::Runtime)?;
    let verified = verified_ended_run_histories(answer, run_id);
    let (stdout, notice) = render_ended_run_view(run_id, verified.as_ref(), export, output);
    print!("{stdout}");
    eprint!("{notice}");
    if verified.is_some() {
        Ok(ClientExitCode::Success)
    } else {
        Err(CodingDevelopmentClientError::Runtime)
    }
}

/// An ended run's view: a requested export of one stored chain, then the
/// stored histories, for standard output, and a content-free notice for
/// standard error when the export could not be made.
fn render_ended_run_view(
    run_id: &str,
    answer: Option<&EndedRunActionHistories>,
    export: Option<ActionHistoryExportSelection>,
    output: CliOutputFormat,
) -> (String, String) {
    let (mut stdout, notice) = export.map_or_else(
        || (String::new(), String::new()),
        |selection| {
            let history = answer
                .and_then(|value| value.chain(selection.chain))
                .map(StoredRunChain::history);
            render_action_history_export(
                selection,
                history.as_ref(),
                output == CliOutputFormat::Json,
            )
        },
    );
    match output {
        CliOutputFormat::Json => {
            stdout.push_str(&format!(
                "{}\n",
                serde_json::json!({
                    "type": "ended_run_action_histories",
                    "run_id": run_id,
                    "available": answer.is_some(),
                    "effects": answer.and_then(|value| value.effects.as_ref()),
                    "job_control": answer.and_then(|value| value.job_control.as_ref()),
                    "routes": answer.and_then(|value| value.routes.as_ref()),
                })
            ));
        }
        CliOutputFormat::Human => stdout.push_str(&render_ended_run_histories(run_id, answer)),
    }
    (stdout, notice)
}

/// One run's rendered result, in the order [`write_run_result`] writes it
/// (review F3 of `e5f34910`).
struct RenderedRunResult {
    /// Verified artifact rows, on standard output.
    artifacts: String,
    /// A requested action history export, on standard output.
    export: String,
    /// A notice instead of an unavailable export, on standard error.
    export_notice: String,
    /// The outcome row, the run's last line on standard output.
    outcome: String,
    /// Progress, declarations and job state, on standard error.
    after_outcome: String,
}

/// Writes one run's result: verified artifact rows and a requested export
/// precede the outcome, which stays the run's last line on standard output;
/// everything after it goes to standard error, so the machine stream keeps
/// its contract (Decision 0127).
fn write_run_result(
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    rendered: &RenderedRunResult,
) -> std::io::Result<()> {
    stdout.write_all(rendered.artifacts.as_bytes())?;
    stdout.write_all(rendered.export.as_bytes())?;
    stdout.flush()?;
    stderr.write_all(rendered.export_notice.as_bytes())?;
    stderr.flush()?;
    writeln!(stdout, "{}", rendered.outcome)?;
    stdout.flush()?;
    stderr.write_all(rendered.after_outcome.as_bytes())?;
    stderr.flush()
}

fn render_verified_artifacts(
    verified: &[crate::cli_runtime::VerifiedRuntimeArtifact],
    output: CliOutputFormat,
) -> String {
    let mut rendered = String::new();
    for verified in verified {
        let line = match output {
            CliOutputFormat::Human => format!(
                "artifact_verified id={} media_type={} sha256={} bytes={} pages={}",
                verified.reference.artifact_id.as_str(),
                verified.reference.media_type,
                verified.reference.payload_sha256,
                verified.reference.byte_size,
                verified.page_count,
            ),
            CliOutputFormat::Json => serde_json::json!({
                "type": "runtime_artifact_verified",
                "artifact_id": verified.reference.artifact_id.as_str(),
                "media_type": verified.reference.media_type,
                "payload_sha256": verified.reference.payload_sha256,
                "byte_size": verified.reference.byte_size,
                "page_count": verified.page_count,
            })
            .to_string(),
        };
        rendered.push_str(&line);
        rendered.push('\n');
    }
    rendered
}

fn render_run_declarations(
    declarations: Option<&RuntimeRunDeclarations>,
    output: CliOutputFormat,
) -> String {
    let recoverability = declarations.and_then(|value| value.recoverability.as_ref());
    let contexts = declarations.and_then(|value| value.context_inspections.as_ref());
    let effect_history = declarations.and_then(|value| value.effect_history.as_ref());
    let job_control_history = declarations.and_then(|value| value.job_control_history.as_ref());
    let route_receipt = declarations.and_then(|value| value.route_receipt.as_ref());
    let route_history = declarations.and_then(|value| value.route_history.as_ref());
    if output == CliOutputFormat::Json {
        return format!(
            "{}\n",
            serde_json::json!({
                "type": "run_declarations",
                "recoverability_available": recoverability.is_some(),
                "recoverability": recoverability,
                "context_inspections_available": contexts.is_some(),
                "context_inspections": contexts,
                "effect_history_available": effect_history.is_some(),
                "effect_history": effect_history,
                "job_control_history_available": job_control_history.is_some(),
                "job_control_history": job_control_history,
                "route_receipt_available": route_receipt.is_some(),
                "route_receipt": route_receipt,
                "route_history_available": route_history.is_some(),
                "route_history": route_history,
            })
        );
    }
    let mut rendered = recoverability.map_or_else(
        || "recoverability of this run's effects: unavailable; the host could not declare them completely\n".to_owned(),
        render_recoverability,
    );
    match contexts {
        Some(views) => {
            for (index, view) in views.iter().enumerate() {
                rendered.push_str(&format!("context view {} of {}:\n", index + 1, views.len()));
                rendered.push_str(&render_context_inspection(view));
            }
        }
        None => rendered.push_str(
            "context views: unavailable; the host could not show every composed context\n",
        ),
    }
    rendered.push_str(&render_run_action_history(
        RunActionChain::Effects,
        effect_history,
    ));
    rendered.push_str(&render_run_action_history(
        RunActionChain::JobControl,
        job_control_history,
    ));
    rendered.push_str(&render_run_route_receipt(route_receipt));
    rendered.push_str(&render_run_action_history(
        RunActionChain::Routes,
        route_history,
    ));
    rendered
}

/// The requested export of one ended run's action history (Decision 0127):
/// its exact bytes for standard output, printed before the outcome, and a
/// content-free notice for standard error when no export could be made.
fn render_requested_export(
    selection: Option<ActionHistoryExportSelection>,
    declarations: Option<&RuntimeRunDeclarations>,
    output: CliOutputFormat,
) -> (String, String) {
    let Some(selection) = selection else {
        return (String::new(), String::new());
    };
    let history = declarations.and_then(|value| match selection.chain {
        RunActionChain::Effects => value.effect_history.as_ref(),
        RunActionChain::JobControl => value.job_control_history.as_ref(),
        RunActionChain::Routes => value.route_history.as_ref(),
    });
    render_action_history_export(selection, history, output == CliOutputFormat::Json)
}

/// The words for a control request.
const fn control_words(action: JobControlAction) -> &'static str {
    match action {
        JobControlAction::Suspend => "suspension",
        JobControlAction::Resume => "resumption",
        JobControlAction::Cancel => "cancellation",
    }
}

/// One control answer for a person: the request, the decision and the job
/// state after it.
fn render_job_control_answer(action: JobControlAction, answer: &RuntimeJobControl) -> String {
    let decision = match answer.decision {
        JobControlDecision::Applied { .. } => "applied",
        JobControlDecision::Refused { refusal, .. } => match refusal {
            JobControlRefusal::StaleRevision => {
                "refused: the job changed since this client observed it"
            }
            JobControlRefusal::AlreadyInEffect => "refused: already in effect",
            JobControlRefusal::Terminal => "refused: the job has ended",
            JobControlRefusal::NotAllowed => "refused: not allowed in this phase",
        },
    };
    format!(
        "{} request {decision}; {}",
        control_words(action),
        render_job_observation(&answer.status.job)
    )
}

/// The job state of one ended run and each of this client's control answers,
/// shown on standard error (Decisions 0120 and 0122). An unavailable state
/// names only what the driver observed (review F1 of `8a3a341e`).
fn render_job_state(
    job: Result<&RuntimeJobStatus, JobStateUnavailable>,
    controls: &[KeptJobControl],
    output: CliOutputFormat,
) -> String {
    let (reason_code, reason_text) = match job {
        Ok(_) => (None, ""),
        Err(JobStateUnavailable::NotOffered) => (
            Some("not_offered"),
            "the host offers no job state for this run",
        ),
        Err(JobStateUnavailable::NotAnswered) => (
            Some("not_answered"),
            "the host did not answer the status request",
        ),
        Err(JobStateUnavailable::NotDescribed) => (
            Some("not_described"),
            "the host's answer did not describe this run",
        ),
    };
    if output == CliOutputFormat::Json {
        return format!(
            "{}\n",
            serde_json::json!({
                "type": "job_state",
                "available": job.is_ok(),
                "reason": reason_code,
                "job": job.ok().map(|status| &status.job),
                "controls": controls
                    .iter()
                    .map(|control| serde_json::json!({
                        "action": control.action,
                        "decision": control.answer.decision,
                        "job": control.answer.status.job,
                    }))
                    .collect::<Vec<_>>(),
            })
        );
    }
    let mut rendered = String::new();
    for control in controls {
        rendered.push_str(&format!(
            "job control: {}\n",
            render_job_control_answer(control.action, &control.answer)
        ));
    }
    match job {
        Ok(status) => rendered.push_str(&format!(
            "job state: {}\n",
            render_job_observation(&status.job)
        )),
        Err(_) => rendered.push_str(&format!("job state: unavailable; {reason_text}\n")),
    }
    rendered
}

/// One job control event as it happens, for standard error (Decision 0122).
fn render_job_control_notice(notice: JobControlNotice<'_>, output: CliOutputFormat) -> String {
    let boundary = |point: &RuntimeSuspensionPoint| {
        serde_json::json!({
            "checkpoint_id": point.checkpoint_id.as_str(),
            "checkpoint_sha256": point.checkpoint_sha256,
            "event_sequence": point.event_cursor.sequence,
        })
    };
    let (json, human) = match notice {
        JobControlNotice::Answered { action, answer } => (
            serde_json::json!({
                "type": "job_control_answer",
                "action": action,
                "decision": answer.decision,
                "job": answer.status.job,
            }),
            format!(
                "job control answer: {}",
                render_job_control_answer(action, answer)
            ),
        ),
        JobControlNotice::NotTaken { action } => (
            serde_json::json!({"type": "job_control_not_taken", "action": action}),
            format!(
                "job control: the host did not take the {} request; the run continues unchanged",
                control_words(action)
            ),
        ),
        JobControlNotice::Suspended(point) => {
            let mut json = boundary(point);
            json["type"] = serde_json::Value::from("run_suspended");
            (
                json,
                format!(
                    "run suspended at a safe boundary: checkpoint {:?} after event {}; \
                     send SIGUSR2 to resume it or SIGINT to cancel it",
                    point.checkpoint_id.as_str(),
                    point.event_cursor.sequence
                ),
            )
        }
        JobControlNotice::Continued(point) => {
            let mut json = boundary(point);
            json["type"] = serde_json::Value::from("run_continued");
            (
                json,
                format!(
                    "run continues from checkpoint {:?} after event {} through a new composition",
                    point.checkpoint_id.as_str(),
                    point.event_cursor.sequence
                ),
            )
        }
        JobControlNotice::Released { action, point } => {
            let mut json = boundary(point);
            json["type"] = serde_json::Value::from("run_released");
            json["action"] = serde_json::json!(action);
            (
                json,
                format!(
                    "job control: the host could not continue the run suspended at checkpoint \
                     {:?} after event {} after the {} request and no longer holds it; the job \
                     keeps the phase its ledger recorded",
                    point.checkpoint_id.as_str(),
                    point.event_cursor.sequence,
                    control_words(action)
                ),
            )
        }
    };
    match output {
        CliOutputFormat::Human => format!("{human}\n"),
        CliOutputFormat::Json => format!("{json}\n"),
    }
}

impl CodingEventSink for TerminalEventSink {
    fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
        let rendered = match self.output {
            CliOutputFormat::Human => render_runtime_event_human(event),
            CliOutputFormat::Json => render_runtime_event_json(event),
        }
        .map_err(|_| CodingClientError::Presentation)?;
        println!("{rendered}");
        if self.run_events.len() < self.capacity {
            self.run_events.push(event.clone());
        } else {
            self.overflowed = true;
        }
        Ok(())
    }

    fn present_job_control(
        &mut self,
        notice: JobControlNotice<'_>,
    ) -> Result<(), CodingClientError> {
        // Standard error keeps the machine stream's outcome last on stdout.
        eprint!("{}", render_job_control_notice(notice, self.output));
        Ok(())
    }
}

struct TerminalApprovals {
    output: CliOutputFormat,
    preauthorized: bool,
    delay_ms: u64,
    cancellation: Arc<AtomicBool>,
    review_workspace: Option<ReviewWorkspace>,
}

/// Largest current file read to render a write challenge's hunk review.
const MAX_REVIEW_READ_BYTES: u64 = 4 * 1024 * 1024;

/// Disposable workspace used only to render a digest-bound hunk review
/// (Decision 0112). It never supplies bytes to the host or to a write.
struct ReviewWorkspace {
    workspace_id: WorkspaceId,
    root: PathBuf,
}

impl ReviewWorkspace {
    /// Opens each component relative to the held parent without following a
    /// link, then checks and reads the opened file itself, so no path can be
    /// swapped between the check and the read.
    fn read(&self, components: &[String]) -> ReviewTarget {
        use std::io::Read as _;

        use rustix::fs::{FileType, Mode, OFlags, fstat, open, openat};
        use rustix::io::Errno;

        let Some((name, parents)) = components.split_last() else {
            return ReviewTarget::Refused;
        };
        if components.iter().any(|component| {
            component.is_empty()
                || matches!(component.as_str(), "." | "..")
                || component.contains(['/', '\0'])
        }) {
            return ReviewTarget::Refused;
        }
        let Ok(mut directory) = open(
            &self.root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        ) else {
            return ReviewTarget::Refused;
        };
        for parent in parents {
            directory = match openat(
                &directory,
                parent.as_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(next) => next,
                Err(Errno::NOENT) => return ReviewTarget::Absent,
                Err(_) => return ReviewTarget::Refused,
            };
        }
        let file = match openat(
            &directory,
            name.as_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(file) => file,
            Err(Errno::NOENT) => return ReviewTarget::Absent,
            Err(_) => return ReviewTarget::Refused,
        };
        let Ok(stat) = fstat(&file) else {
            return ReviewTarget::Refused;
        };
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
            || u64::try_from(stat.st_size).map_or(true, |size| size > MAX_REVIEW_READ_BYTES)
        {
            return ReviewTarget::Refused;
        }
        let mut bytes = Vec::new();
        match std::fs::File::from(file)
            .take(MAX_REVIEW_READ_BYTES + 1)
            .read_to_end(&mut bytes)
        {
            Ok(read) if read as u64 <= MAX_REVIEW_READ_BYTES => ReviewTarget::Bytes(bytes),
            _ => ReviewTarget::Refused,
        }
    }

    fn review(
        &self,
        challenge: &RuntimeApprovalChallenge,
    ) -> Option<Result<ChangeReview, ChangeReviewUnavailable>> {
        review_coding_write(challenge, &self.workspace_id, &|components| {
            self.read(components)
        })
    }

    #[cfg(test)]
    fn render(
        &self,
        challenge: &RuntimeApprovalChallenge,
        output: CliOutputFormat,
    ) -> Option<String> {
        self.review(challenge)
            .map(|review| format_review(challenge, &review, output))
    }
}

fn format_review(
    challenge: &RuntimeApprovalChallenge,
    review: &Result<ChangeReview, ChangeReviewUnavailable>,
    output: CliOutputFormat,
) -> String {
    {
        match output {
            CliOutputFormat::Human => render_change_review(review),
            CliOutputFormat::Json => {
                let value = match review {
                    Ok(review) => serde_json::json!({
                        "type": "change_review",
                        "approval_id": challenge.approval_id.as_str(),
                        "available": true,
                        "path": review.path,
                        "hunk_count": review.hunk_count,
                        "hunk_ids": review.hunk_ids,
                        "selectable": review.selectable,
                        "selected_from": review.selected_from,
                        "preimage_sha256": review.preimage_sha256,
                        "postimage_sha256": review.postimage_sha256,
                        "rendered": review.rendered,
                    }),
                    Err(reason) => serde_json::json!({
                        "type": "change_review",
                        "approval_id": challenge.approval_id.as_str(),
                        "available": false,
                        "code": reason.code(),
                    }),
                };
                format!("{value}\n")
            }
        }
    }
}

/// Most unparsable selections before the answer counts as a refusal.
const MAX_SELECTION_ATTEMPTS: usize = 3;

/// Parses `select` followed by one-based hunk numbers, separated by spaces or
/// commas, into sorted unique numbers naming at least one and fewer than all.
fn parse_hunk_selection(line: &str, hunk_count: usize) -> Option<Vec<usize>> {
    let numbers = line.strip_prefix("select")?;
    if !numbers.starts_with([' ', ',']) {
        return None;
    }
    let mut selected = std::collections::BTreeSet::new();
    for part in numbers.split([' ', ',']).filter(|part| !part.is_empty()) {
        let number: usize = part.parse().ok()?;
        if number == 0 || number > hunk_count {
            return None;
        }
        selected.insert(number);
    }
    (!selected.is_empty() && selected.len() < hunk_count).then(|| selected.into_iter().collect())
}

/// The exact selection a person's hunk numbers name in one review.
fn hunk_selection(review: &ChangeReview, numbers: &[usize]) -> RuntimeHunkSelection {
    let mut accepted_hunk_ids = numbers
        .iter()
        .map(|number| review.hunk_ids[number - 1].clone())
        .collect::<Vec<_>>();
    accepted_hunk_ids.sort();
    RuntimeHunkSelection {
        preimage_sha256: review.preimage_sha256.clone(),
        proposal_sha256: review.postimage_sha256.clone(),
        accepted_hunk_ids,
    }
}

/// Supplies the real private terminal port to the synthetic interactive-driver tests.
#[cfg(all(test, feature = "source-artifacts", feature = "workflow-supervisor"))]
pub(crate) fn terminal_approval_for_test(
    preauthorized: bool,
    cancellation: Arc<AtomicBool>,
) -> impl CodingApprovalPort {
    TerminalApprovals {
        output: CliOutputFormat::Json,
        preauthorized,
        delay_ms: if preauthorized { 10_000 } else { 0 },
        cancellation,
        review_workspace: None,
    }
}

fn wait_for_approval_delay(
    delay_ms: u64,
    cancellation: &AtomicBool,
) -> Result<bool, CodingClientError> {
    if delay_ms > 10_000 {
        return Err(CodingClientError::Approval);
    }
    let deadline = Instant::now()
        .checked_add(Duration::from_millis(delay_ms))
        .ok_or(CodingClientError::Approval)?;
    loop {
        if cancellation.load(Ordering::Acquire) {
            return Ok(false);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(true);
        }
        std::thread::sleep(remaining.min(Duration::from_millis(25)));
    }
}

impl CodingApprovalPort for TerminalApprovals {
    fn decide(
        &mut self,
        challenge: &RuntimeApprovalChallenge,
    ) -> Result<RuntimeApprovalDisposition, CodingClientError> {
        self.decide_with_selection(challenge)
            .map(|(disposition, _)| disposition)
    }

    fn decide_with_selection(
        &mut self,
        challenge: &RuntimeApprovalChallenge,
    ) -> Result<(RuntimeApprovalDisposition, Option<RuntimeHunkSelection>), CodingClientError> {
        let rendered =
            render_runtime_approval_human(challenge).map_err(|_| CodingClientError::Approval)?;
        let review = self
            .review_workspace
            .as_ref()
            .and_then(|workspace| workspace.review(challenge));
        let review_text = review
            .as_ref()
            .map(|review| format_review(challenge, review, self.output));
        if self.preauthorized {
            eprintln!("preauthorized_for_this_run {rendered}");
            if let Some(review) = &review_text {
                eprint!("{review}");
            }
            return Ok((
                if wait_for_approval_delay(self.delay_ms, &self.cancellation)? {
                    RuntimeApprovalDisposition::Allow
                } else {
                    // The interactive driver polls this unconsumed flag before
                    // transmitting any approval response, so canonical cancel wins.
                    RuntimeApprovalDisposition::Deny
                },
                None,
            ));
        }
        if self.output == CliOutputFormat::Json {
            eprintln!("approval_required {rendered}");
        } else {
            eprintln!("{rendered}");
        }
        if let Some(review) = &review_text {
            eprint!("{review}");
        }
        if let Some(Ok(review)) = review
            .as_ref()
            .filter(|review| review.as_ref().is_ok_and(|review| review.selectable))
        {
            return self.decide_selectable(review);
        }
        eprint!("Approve this exact operation? Type yes to allow: ");
        match read_development_confirmation(
            LinuxDevelopmentConfirmationKind::OperationApproval,
            &self.cancellation,
        )
        .map_err(|_| CodingClientError::Approval)?
        {
            LinuxDevelopmentConfirmation::Confirmed => {
                Ok((RuntimeApprovalDisposition::Allow, None))
            }
            LinuxDevelopmentConfirmation::Declined | LinuxDevelopmentConfirmation::Cancelled => {
                // Cancellation remains pending for the existing driver's next
                // poll; this value must not be sent ahead of that cancel.
                Ok((RuntimeApprovalDisposition::Deny, None))
            }
        }
    }
}

impl TerminalApprovals {
    /// Asks on the terminal for yes, a selection of hunks, or anything else as
    /// a refusal (Decision 0114).
    fn decide_selectable(
        &mut self,
        review: &ChangeReview,
    ) -> Result<(RuntimeApprovalDisposition, Option<RuntimeHunkSelection>), CodingClientError> {
        let cancellation = &self.cancellation;
        decide_selection(review, &mut std::io::stderr(), &mut || {
            read_development_line(cancellation).map_err(|_| CodingClientError::Approval)
        })
    }
}

/// Asks for yes, a selection of hunks, or anything else as a refusal. A
/// selection refuses the whole call and asks the host for a separately
/// approved write of only those hunks (Decision 0114). An over-long or
/// unacceptable selection is answered and asked again, within a bound.
fn decide_selection(
    review: &ChangeReview,
    prompts: &mut dyn Write,
    next_line: &mut dyn FnMut() -> Result<LinuxDevelopmentInputLine, CodingClientError>,
) -> Result<(RuntimeApprovalDisposition, Option<RuntimeHunkSelection>), CodingClientError> {
    for _ in 0..MAX_SELECTION_ATTEMPTS {
        let _ = write!(
            prompts,
            "Approve this exact operation? Type yes to allow, or select and hunk numbers (for example: select 1 {}) to refuse it and request a separate write of only those hunks: ",
            review.hunk_count
        );
        let _ = prompts.flush();
        let line = match next_line()? {
            LinuxDevelopmentInputLine::Line(line) => line,
            // The reader discarded the rest of the line; ask again.
            LinuxDevelopmentInputLine::TooLong => {
                let _ = writeln!(prompts, "selection not accepted: the line is too long");
                continue;
            }
            // Cancellation stays pending for the driver's next poll.
            LinuxDevelopmentInputLine::Ended | LinuxDevelopmentInputLine::Cancelled => {
                return Ok((RuntimeApprovalDisposition::Deny, None));
            }
        };
        if line == "yes" {
            return Ok((RuntimeApprovalDisposition::Allow, None));
        }
        if !line.starts_with("select") {
            return Ok((RuntimeApprovalDisposition::Deny, None));
        }
        if let Some(numbers) = parse_hunk_selection(&line, review.hunk_count) {
            return Ok((
                RuntimeApprovalDisposition::Narrow,
                Some(hunk_selection(review, &numbers)),
            ));
        }
        let _ = writeln!(
            prompts,
            "selection not accepted: name at least one and fewer than all of hunks 1 to {}",
            review.hunk_count
        );
    }
    Ok((RuntimeApprovalDisposition::Deny, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_overlong_selection_is_answered_and_asked_again() {
        // Review V4 of 8fbd2bc6: through the prompt, an over-long line is
        // not accepted, the person is asked again, and a later selection
        // narrows the call.
        let review = ChangeReview {
            path: vec!["src".to_owned(), "calc.py".to_owned()],
            preimage_sha256: "a".repeat(64),
            postimage_sha256: "b".repeat(64),
            hunk_count: 3,
            hunk_ids: vec![
                "hunk-1".to_owned(),
                "hunk-2".to_owned(),
                "hunk-3".to_owned(),
            ],
            selectable: true,
            selected_from: None,
            rendered: String::new(),
        };
        let answer = |lines: Vec<LinuxDevelopmentInputLine>| {
            let mut lines = lines.into_iter();
            let mut prompts = Vec::new();
            let decision = decide_selection(&review, &mut prompts, &mut || {
                lines.next().ok_or(CodingClientError::Approval)
            });
            (decision, String::from_utf8(prompts).unwrap(), lines.count())
        };
        let (decision, prompts, unread) = answer(vec![
            LinuxDevelopmentInputLine::TooLong,
            LinuxDevelopmentInputLine::Line("select 3 1".to_owned()),
        ]);
        assert_eq!(
            decision.unwrap(),
            (
                RuntimeApprovalDisposition::Narrow,
                Some(RuntimeHunkSelection {
                    preimage_sha256: "a".repeat(64),
                    proposal_sha256: "b".repeat(64),
                    accepted_hunk_ids: vec!["hunk-1".to_owned(), "hunk-3".to_owned()],
                })
            )
        );
        assert_eq!(prompts.matches("Approve this exact operation?").count(), 2);
        assert_eq!(
            prompts
                .matches("selection not accepted: the line is too long")
                .count(),
            1
        );
        assert_eq!(unread, 0);
        // Every hunk, or a number out of range, is not a selection either.
        let (decision, prompts, _) = answer(vec![
            LinuxDevelopmentInputLine::Line("select 1 2 3".to_owned()),
            LinuxDevelopmentInputLine::Line("yes".to_owned()),
        ]);
        assert_eq!(decision.unwrap(), (RuntimeApprovalDisposition::Allow, None));
        assert!(prompts.contains("name at least one and fewer than all of hunks 1 to 3"));
        // The attempts are bounded, and a bounded run of over-long lines
        // is a refusal, never an approval.
        let (decision, prompts, unread) = answer(vec![
            LinuxDevelopmentInputLine::TooLong;
            MAX_SELECTION_ATTEMPTS + 1
        ]);
        assert_eq!(decision.unwrap(), (RuntimeApprovalDisposition::Deny, None));
        assert_eq!(
            prompts.matches("Approve this exact operation?").count(),
            MAX_SELECTION_ATTEMPTS
        );
        assert_eq!(unread, 1);
        // End of input and cancellation refuse at once.
        for ending in [
            LinuxDevelopmentInputLine::Ended,
            LinuxDevelopmentInputLine::Cancelled,
        ] {
            let (decision, _, _) = answer(vec![ending]);
            assert_eq!(decision.unwrap(), (RuntimeApprovalDisposition::Deny, None));
        }
    }

    #[test]
    fn run_declarations_show_recoverability_and_each_context_view_or_say_unavailable() {
        use agentmage_kernel_contracts::{
            ContextItemKind, ContextOmissionReason, ContextSensitivity,
        };
        use agentmage_kernel_engine::context_inspection::{
            ContextInspection, InspectedContextItem,
        };
        let report = crate::coding_recoverability::assess_run_recoverability(
            "session-cli",
            "task-cli",
            "run-cli",
            &[crate::coding_recoverability::SessionEffect::Command {
                operation_id: "operation-test".to_owned(),
            }],
            &|_| None,
        )
        .unwrap();
        let item = |id: &str, omission| InspectedContextItem {
            item_id: id.to_owned(),
            kind: ContextItemKind::Instruction,
            sensitivity: ContextSensitivity::Internal,
            source_id: format!("source-{id}"),
            source_revision: "revision".to_owned(),
            content_sha256: "a".repeat(64),
            token_count: 10,
            byte_count: 40,
            omission,
        };
        let view = ContextInspection {
            schema_version: 1,
            context_packet_id: "packet-cli".to_owned(),
            packet_sha256: "b".repeat(64),
            token_counter_id: "counter-cli".to_owned(),
            max_tokens: 100,
            used_tokens: 10,
            max_bytes: 1_000,
            used_bytes: 40,
            pinned: vec![item("pinned", None)],
            selected: Vec::new(),
            omitted: vec![item("left-out", Some(ContextOmissionReason::Budget))],
            kind_totals: Vec::new(),
        };
        let effects =
            action_history(agentmage_kernel_engine::action_history::ActionKind::FileWrite);
        let declarations = RuntimeRunDeclarations {
            schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
            run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-cli"),
            request_sha256: "c".repeat(64),
            recoverability: Some(report.clone()),
            context_inspections: Some(vec![view.clone(), view]),
            effect_history: Some(effects.clone()),
            job_control_history: None,
            route_receipt: None,
            route_history: None,
        };
        let human = render_run_declarations(Some(&declarations), CliOutputFormat::Human);
        assert!(human.starts_with("recoverability of this run's effects: "));
        assert!(human.contains("operation-test external or uncertain: reconcile manually"));
        assert!(human.contains("context view 1 of 2:\n"));
        assert!(human.contains("context view 2 of 2:\n"));
        assert!(!human.contains("undone"));
        // Decision 0127: each action history follows, or is unavailable.
        assert!(human.contains("action history of this run's effects: 1 entry, head "));
        assert!(human.contains(
            "- 1 file_write operation-test succeeded; grant grant-test; coding.approved.succeeded\n"
        ));
        assert!(human.contains(
            "action history of this run's job control: unavailable; the host could not declare it completely\n"
        ));
        // Decision 0128: an undeclared route says so, as does its history.
        assert!(human.ends_with(
            "model route: unavailable; the host could not declare a verified local-only route\n\
             action history of this run's model routes: unavailable; the host could not declare it completely\n"
        ));
        let json: serde_json::Value = serde_json::from_str(
            render_run_declarations(Some(&declarations), CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["type"], "run_declarations");
        assert_eq!(json["recoverability_available"], true);
        assert_eq!(
            json["recoverability"],
            serde_json::to_value(&report).unwrap()
        );
        assert_eq!(json["context_inspections"].as_array().unwrap().len(), 2);
        assert_eq!(json["effect_history_available"], true);
        assert_eq!(
            json["effect_history"],
            serde_json::to_value(&effects).unwrap()
        );
        assert_eq!(json["job_control_history_available"], false);
        assert!(json["job_control_history"].is_null());
        assert_eq!(json["route_receipt_available"], false);
        assert!(json["route_receipt"].is_null());
        assert_eq!(json["route_history_available"], false);
        assert!(json["route_history"].is_null());

        // Nothing declared, or only part of it, is said to be unavailable.
        let human = render_run_declarations(None, CliOutputFormat::Human);
        assert!(human.contains("recoverability of this run's effects: unavailable"));
        assert!(human.contains("context views: unavailable"));
        let partial = RuntimeRunDeclarations {
            recoverability: None,
            context_inspections: None,
            ..declarations
        };
        let json: serde_json::Value = serde_json::from_str(
            render_run_declarations(Some(&partial), CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["recoverability_available"], false);
        assert_eq!(json["context_inspections_available"], false);
    }

    #[test]
    #[cfg(all(feature = "source-artifacts", feature = "workflow-supervisor"))]
    fn a_declared_route_is_shown_with_its_history_in_both_formats() {
        // Decision 0128: the verified receipt and route history follow the
        // other declarations; in JSON they are inside the declarations row.
        let (request, _, _, _) = crate::runtime_read_tests::completed_native_read_fixture();
        let route =
            crate::coding_route::route_development_run(&request, "contract-test", 5_000).unwrap();
        let declarations = RuntimeRunDeclarations {
            schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
            run_id: request.run_id.clone(),
            request_sha256: request.request_sha256.clone(),
            recoverability: None,
            context_inspections: None,
            effect_history: None,
            job_control_history: None,
            route_receipt: Some(route.receipt.clone()),
            route_history: route.history.clone(),
        };
        let human = render_run_declarations(Some(&declarations), CliOutputFormat::Human);
        assert!(human.contains(&format!(
            "model route: local-only; selected {} (strict_local); data conversation, workspace_excerpts, tool_outputs; 1 considered; model-gateway.route.qualified-selected; receipt {}\n",
            request.model_profile.profile_id.as_str(),
            &route.receipt.receipt_sha256[..12]
        )));
        assert!(human.contains("action history of this run's model routes: 1 entry, head "));
        assert!(human.contains(&format!(
            "- 1 model_route {} succeeded; person decision {}; model-gateway.route.qualified-selected\n",
            request.run_id.as_str(),
            &request.request_sha256[..12]
        )));
        let json: serde_json::Value = serde_json::from_str(
            render_run_declarations(Some(&declarations), CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["route_receipt_available"], true);
        assert_eq!(
            json["route_receipt"],
            serde_json::to_value(&route.receipt).unwrap()
        );
        assert_eq!(json["route_history_available"], true);
        assert_eq!(
            json["route_history"],
            serde_json::to_value(route.history.as_ref().unwrap()).unwrap()
        );
    }

    /// One succeeded entry of the given kind, authorized by a grant.
    fn action_history(
        kind: agentmage_kernel_engine::action_history::ActionKind,
    ) -> crate::coding_action_history::RunActionHistory {
        use agentmage_kernel_engine::action_history::{
            ActionAuthorization, ActionOutcome, ActionRecordDraft,
        };
        let mut recorder = crate::coding_action_history::RunActionRecorder::new();
        recorder.record(Some(ActionRecordDraft {
            action_kind: kind,
            action_id: "operation-test".to_owned(),
            authorization: ActionAuthorization::Grant {
                grant_id: "grant-test".to_owned(),
                grant_sha256: "7".repeat(64),
            },
            effect_sha256: "8".repeat(64),
            outcome: ActionOutcome::Succeeded,
            reason_code: "coding.approved.succeeded".to_owned(),
            evidence_sha256s: vec!["9".repeat(64)],
            recorded_at_epoch_ms: 1,
            retain_until_epoch_ms: 2,
        }));
        recorder.declare().unwrap()
    }

    #[test]
    fn a_requested_export_is_rendered_or_noticed_in_both_formats() {
        // Decision 0127: the export of the selected chain is rendered for
        // standard output; an unavailable chain or range renders a
        // content-free notice for standard error instead. The order in which
        // they are written is tested below (review F3 of e5f34910).
        use agentmage_kernel_engine::action_history::ActionKind;
        let declarations = RuntimeRunDeclarations {
            schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
            run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-cli"),
            request_sha256: "c".repeat(64),
            recoverability: None,
            context_inspections: None,
            effect_history: Some(action_history(ActionKind::CommandRun)),
            job_control_history: Some(action_history(ActionKind::JobControl)),
            route_receipt: None,
            route_history: Some(action_history(ActionKind::ModelRoute)),
        };
        let selection = |text| ActionHistoryExportSelection::parse(text).unwrap();
        assert_eq!(
            render_requested_export(None, Some(&declarations), CliOutputFormat::Json),
            (String::new(), String::new())
        );
        for (text, chain) in [
            ("effects:1:1", "effects"),
            ("job-control:1:1", "job-control"),
            ("routes:1:1", "routes"),
        ] {
            let (stdout, stderr) = render_requested_export(
                Some(selection(text)),
                Some(&declarations),
                CliOutputFormat::Json,
            );
            assert!(stderr.is_empty());
            let row: serde_json::Value = serde_json::from_str(stdout.trim_end()).unwrap();
            assert_eq!(
                (row["type"].as_str(), row["chain"].as_str()),
                (Some("action_history_export"), Some(chain))
            );
            assert_eq!(row["available"], true);
            let (stdout, stderr) = render_requested_export(
                Some(selection(text)),
                Some(&declarations),
                CliOutputFormat::Human,
            );
            assert!(stderr.is_empty());
            assert!(stdout.starts_with(&format!(
                "action_history_export chain={chain} from=1 to=1 sha256="
            )));
            assert_eq!(stdout.lines().count(), 2);
        }
        for declared in [None, Some(&declarations)] {
            let wanted = if declared.is_some() {
                "effects:1:2"
            } else {
                "effects:1:1"
            };
            for output in [CliOutputFormat::Human, CliOutputFormat::Json] {
                let (stdout, stderr) =
                    render_requested_export(Some(selection(wanted)), declared, output);
                assert!(stdout.is_empty());
                assert!(stderr.contains(if declared.is_some() {
                    "range"
                } else {
                    "unavailable"
                }));
            }
        }
    }

    #[test]
    fn a_requested_export_is_written_before_the_outcome_which_stays_last_on_stdout() {
        // Review F3 of e5f34910: run_with_child writes each run's result
        // through write_run_result, so its order is asserted on the bytes it
        // writes to each stream.
        use agentmage_kernel_engine::action_history::ActionKind;
        let declarations = RuntimeRunDeclarations {
            schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
            run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-cli"),
            request_sha256: "c".repeat(64),
            recoverability: None,
            context_inspections: None,
            effect_history: Some(action_history(ActionKind::CommandRun)),
            job_control_history: None,
            route_receipt: None,
            route_history: None,
        };
        let selection = |text| ActionHistoryExportSelection::parse(text);
        for output in [CliOutputFormat::Human, CliOutputFormat::Json] {
            for (wanted, exported) in [("effects:1:1", true), ("job-control:1:1", false)] {
                let (export, export_notice) =
                    render_requested_export(selection(wanted), Some(&declarations), output);
                let rendered = RenderedRunResult {
                    artifacts: "artifact-row\n".to_owned(),
                    export,
                    export_notice,
                    outcome: "outcome-row".to_owned(),
                    after_outcome: render_run_declarations(Some(&declarations), output),
                };
                let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
                write_run_result(&mut stdout, &mut stderr, &rendered).unwrap();
                let stdout = String::from_utf8(stdout).unwrap();
                let stderr = String::from_utf8(stderr).unwrap();
                let lines = stdout.lines().collect::<Vec<_>>();
                // The outcome is the last line on standard output.
                assert_eq!(lines.last(), Some(&"outcome-row"));
                assert_eq!(lines.first(), Some(&"artifact-row"));
                let export_row = lines
                    .iter()
                    .position(|line| line.contains("action_history_export"));
                if exported {
                    // The export sits between the artifacts and the outcome,
                    // and its notice stream stays empty.
                    let export_row = export_row.expect("export row");
                    assert!(0 < export_row && export_row < lines.len() - 1);
                    assert!(!stderr.contains("action_history_export"));
                } else {
                    // An unavailable chain prints only a notice, on stderr.
                    assert_eq!(export_row, None);
                    assert_eq!(lines, ["artifact-row", "outcome-row"]);
                    assert!(stderr.starts_with(&rendered.export_notice));
                    assert!(!rendered.export_notice.is_empty());
                }
                // Everything after the outcome goes to standard error.
                assert!(stderr.ends_with(&rendered.after_outcome));
                assert!(!stdout.contains(rendered.after_outcome.trim_end()));
                assert!(!stderr.contains("outcome-row"));
            }
        }
    }

    #[test]
    fn a_support_bundle_observes_the_invocation_and_is_skipped_only_when_cancelled() {
        // Decision 0128: the bundle names the selected source and the
        // failure that ended the invocation; a cancelled invocation, by
        // signal, run or startup, asks nothing.
        let failed = Err(CodingDevelopmentClientError::HostBoundary(
            LinuxDevelopmentBoundaryErrorKind::TransferFailed,
        ));
        let observed = invocation_observation("scripted", Vec::new(), &failed);
        assert_eq!(
            observed.profile_id,
            crate::coding_development_runtime::SCRIPTED_PROFILE_ID
        );
        assert!(observed.scripted && observed.runs.is_empty());
        assert_eq!(
            observed.failure_code,
            Some("linux.development.launch-envelope.failed")
        );
        let served = invocation_observation("muse", Vec::new(), &Ok(ClientExitCode::Success));
        assert_eq!(
            served.profile_id,
            crate::coding_development_runtime::MUSE_DEVELOPMENT_PROFILE_ID
        );
        assert!(!served.scripted && served.failure_code.is_none());
        let startup_cancelled = Err(CodingDevelopmentClientError::HostBoundary(
            LinuxDevelopmentBoundaryErrorKind::StartupCancelled,
        ));
        for (signal_pending, result, cancelled) in [
            (false, Ok(ClientExitCode::Success), false),
            (false, failed, false),
            (false, Ok(ClientExitCode::PolicyDenied), false),
            (true, Ok(ClientExitCode::Success), true),
            (false, Ok(ClientExitCode::Cancelled), true),
            (false, startup_cancelled, true),
        ] {
            assert_eq!(
                invocation_cancelled(signal_pending, &result),
                cancelled,
                "{result:?}"
            );
        }
    }

    use crate::runtime_transport::RuntimeTransportError;

    /// A catalog host that answers documentation pack requests from a
    /// script and records what it was sent.
    struct ScriptedCatalog {
        answers: std::collections::VecDeque<Result<DocPackAnswer, RuntimeTransportError>>,
        sent: Vec<DocPackRequest>,
    }

    impl RuntimeTransportPort for ScriptedCatalog {
        fn prepare(
            &mut self,
            _input: RuntimePrepareInput,
        ) -> Result<agentmage_kernel_contracts::RuntimeRunRequest, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }
        fn start(
            &mut self,
            _request: agentmage_kernel_contracts::RuntimeRunRequest,
        ) -> Result<crate::runtime_transport::RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }
        fn advance(
            &mut self,
            _run_id: &agentmage_kernel_contracts::RuntimeRunId,
            _request_sha256: &str,
            _after_event_cursor: Option<&agentmage_kernel_contracts::RuntimeEventCursor>,
            _response: Option<&agentmage_kernel_contracts::RuntimeApprovalResponse>,
        ) -> Result<crate::runtime_transport::RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }
        fn cancel(
            &mut self,
            _run_id: &agentmage_kernel_contracts::RuntimeRunId,
            _request_sha256: &str,
            _cancellation_id: agentmage_kernel_contracts::CancellationId,
            _after_event_cursor: Option<&agentmage_kernel_contracts::RuntimeEventCursor>,
        ) -> Result<crate::runtime_transport::RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }
        fn release(
            &mut self,
            _run_id: &agentmage_kernel_contracts::RuntimeRunId,
            _request_sha256: &str,
        ) -> Result<(), RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }
        fn doc_pack(
            &mut self,
            request: DocPackRequest,
        ) -> Result<DocPackAnswer, RuntimeTransportError> {
            self.sent.push(request);
            self.answers
                .pop_front()
                .unwrap_or(Err(RuntimeTransportError::RuntimeFailed))
        }
    }

    #[test]
    fn a_pack_import_keeps_only_answers_that_acknowledge_what_was_sent() {
        // Decision 0130: the staged answer, each chunk and the receipt must
        // acknowledge exactly what the CLI sent; a refusal stops the import
        // with its exit class; cancellation stops sending.
        use crate::coding_doc_packs::{
            DocPackImportReceiptView, DocPackSource, doc_pack_receipt_sha256,
        };
        use agentmage_capability_knowledge::{
            DocPackFile, DocPackManifest, DocPackMediaType, DocPackVersion, seal_doc_pack_manifest,
        };
        let text = "Offline notes.\n";
        let version = DocPackVersion {
            major: 1,
            minor: 0,
            patch: 0,
        };
        let manifest = seal_doc_pack_manifest(DocPackManifest {
            schema_version: 1,
            pack_id: "build-tool-guide".to_owned(),
            version,
            title: "Build tool guide".to_owned(),
            publisher: "Synthetic sample".to_owned(),
            license: "LicenseRef-sample".to_owned(),
            retrieved_on: "2026-01-01".to_owned(),
            fresh_for_days: 30,
            files: vec![DocPackFile {
                path: "notes.txt".to_owned(),
                media_type: DocPackMediaType::PlainText,
                byte_len: text.len() as u64,
                sha256: {
                    use sha2::Digest as _;
                    sha2::Sha256::digest(text.as_bytes())
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect()
                },
            }],
            total_bytes: text.len() as u64,
            manifest_sha256: String::new(),
        })
        .unwrap();
        let source = DocPackSource {
            manifest: manifest.clone(),
            files: [("notes.txt".to_owned(), text.to_owned())].into(),
        };
        let requests = doc_pack_import_requests(&source, &["LicenseRef-sample".to_owned()], None);
        let staged = DocPackAnswer::ImportStaged {
            pack_id: manifest.pack_id.clone(),
            version,
            manifest_sha256: manifest.manifest_sha256.clone(),
        };
        let accepted = DocPackAnswer::ChunkAccepted {
            path: "notes.txt".to_owned(),
            received_bytes: text.len() as u64,
        };
        let mut receipt = DocPackImportReceiptView {
            pack_id: manifest.pack_id.clone(),
            version,
            manifest_sha256: manifest.manifest_sha256.clone(),
            license: manifest.license.clone(),
            superseded: None,
            file_count: 1,
            total_bytes: text.len() as u64,
            fragment_count: 1,
            withheld_fragment_count: 0,
            network_used: false,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = doc_pack_receipt_sha256(&receipt).unwrap();
        let imported = |receipt: DocPackImportReceiptView| DocPackAnswer::Imported {
            receipt,
            retention: Vec::new(),
        };
        let run = |answers: Vec<Result<DocPackAnswer, RuntimeTransportError>>, cancelled: bool| {
            let mut catalog = ScriptedCatalog {
                answers: answers.into(),
                sent: Vec::new(),
            };
            let result = run_doc_pack_requests(
                &mut catalog,
                Some(&manifest),
                requests.clone(),
                CliOutputFormat::Json,
                &AtomicBool::new(cancelled),
            );
            (result, catalog.sent.len())
        };
        assert_eq!(
            run(
                vec![
                    Ok(staged.clone()),
                    Ok(accepted.clone()),
                    Ok(imported(receipt.clone()))
                ],
                false
            ),
            (Ok(ClientExitCode::Success), 3)
        );
        let mut tampered = receipt.clone();
        tampered.fragment_count = 2;
        let other_staged = DocPackAnswer::ImportStaged {
            pack_id: manifest.pack_id.clone(),
            version,
            manifest_sha256: "a".repeat(64),
        };
        let short_chunk = DocPackAnswer::ChunkAccepted {
            path: "notes.txt".to_owned(),
            received_bytes: 1,
        };
        let other_path = DocPackAnswer::ChunkAccepted {
            path: "other.txt".to_owned(),
            received_bytes: text.len() as u64,
        };
        let listed = DocPackAnswer::Listed {
            packs: Vec::new(),
            retention: Vec::new(),
        };
        for (answers, sent) in [
            (vec![Ok(other_staged)], 1),
            (vec![Ok(listed.clone())], 1),
            (vec![Ok(staged.clone()), Ok(short_chunk)], 2),
            (vec![Ok(staged.clone()), Ok(other_path)], 2),
            (vec![Ok(staged.clone()), Ok(staged.clone())], 2),
            (
                vec![
                    Ok(staged.clone()),
                    Ok(accepted.clone()),
                    Ok(imported(tampered)),
                ],
                3,
            ),
            (
                vec![Ok(staged.clone()), Ok(accepted.clone()), Ok(listed)],
                3,
            ),
        ] {
            assert_eq!(
                run(answers, false),
                (Err(CodingDevelopmentClientError::Presentation), sent)
            );
        }
        // A refusal stops sending and exits with its class; a transport
        // failure is a runtime failure; cancellation sends nothing.
        assert_eq!(
            run(
                vec![
                    Ok(staged),
                    Ok(DocPackAnswer::Refused {
                        refusal: DocPackRefusal::ChunkInvalid
                    })
                ],
                false
            ),
            (Ok(ClientExitCode::InvalidInput), 2)
        );
        assert_eq!(
            run(vec![Err(RuntimeTransportError::RuntimeFailed)], false),
            (Err(CodingDevelopmentClientError::Runtime), 1)
        );
        assert_eq!(run(Vec::new(), true), (Ok(ClientExitCode::Cancelled), 0));
        // An operation that is not an import takes only its own answer kind.
        let mut catalog = ScriptedCatalog {
            answers: vec![Ok(accepted)].into(),
            sent: Vec::new(),
        };
        assert_eq!(
            run_doc_pack_requests(
                &mut catalog,
                None,
                vec![DocPackRequest::List {}],
                CliOutputFormat::Human,
                &AtomicBool::new(false),
            ),
            Err(CodingDevelopmentClientError::Presentation)
        );
    }

    #[test]
    fn only_the_exact_word_publishes_a_support_bundle() {
        // Review F3 of `bc1e2d39`: the mapping from the read line to the
        // answer. The reader trims the line, so any other spelling, a prefix,
        // a repetition or a lone letter is a different answer and declines.
        let line = |text: &str| Ok::<_, ()>(LinuxDevelopmentInputLine::Line(text.to_owned()));
        assert_eq!(
            support_bundle_answer(line("yes")),
            SupportBundleAnswer::Confirmed
        );
        for declined in [
            "yes ",
            " yes",
            "Yes",
            "YES",
            "yesyes",
            "yes please",
            "y",
            "",
        ] {
            assert_eq!(
                support_bundle_answer(line(declined)),
                SupportBundleAnswer::Declined,
                "{declined:?}"
            );
        }
        for declined in [
            Ok(LinuxDevelopmentInputLine::Ended),
            Ok(LinuxDevelopmentInputLine::TooLong),
            Err(()),
        ] {
            assert_eq!(
                support_bundle_answer(declined),
                SupportBundleAnswer::Declined
            );
        }
        assert_eq!(
            support_bundle_answer(Ok::<_, ()>(LinuxDevelopmentInputLine::Cancelled)),
            SupportBundleAnswer::Cancelled
        );
    }

    #[test]
    fn an_ended_runs_stored_histories_are_shown_with_a_requested_export() {
        // Decision 0129: the view of an ended run goes to standard output,
        // after a requested export of one stored chain; an export of a chain
        // that is not stored leaves a notice on standard error.
        use crate::coding_action_history::{
            ENDED_RUN_HISTORIES_SCHEMA_VERSION, EndedRunActionHistories, StoredRunChain,
        };
        use agentmage_kernel_engine::action_history::ActionKind;
        let history = action_history(ActionKind::FileWrite);
        let answer = EndedRunActionHistories {
            schema_version: ENDED_RUN_HISTORIES_SCHEMA_VERSION,
            run_id: "run-ended".to_owned(),
            effects: Some(StoredRunChain {
                records: history.records.clone(),
                head: history.head.clone(),
                complete: true,
                closed: true,
            }),
            job_control: None,
            routes: None,
        };
        let selection = |text| ActionHistoryExportSelection::parse(text);
        let (stdout, notice) = render_ended_run_view(
            "run-ended",
            Some(&answer),
            selection("effects:1:1"),
            CliOutputFormat::Json,
        );
        assert!(notice.is_empty());
        let rows = stdout
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["type"], "action_history_export");
        assert_eq!(rows[0]["chain"], "effects");
        assert_eq!(rows[1]["type"], "ended_run_action_histories");
        assert_eq!(rows[1]["available"], true);
        assert_eq!(
            rows[1]["effects"],
            serde_json::to_value(answer.effects.as_ref().unwrap()).unwrap()
        );
        assert!(rows[1]["routes"].is_null());
        let (stdout, notice) = render_ended_run_view(
            "run-ended",
            Some(&answer),
            selection("routes:1:1"),
            CliOutputFormat::Human,
        );
        assert!(notice.contains("unavailable"));
        assert!(stdout.starts_with("stored action histories of run run-ended:\n"));
        assert!(stdout.contains("effects: closed when the run ended; complete\n"));
        let (stdout, notice) =
            render_ended_run_view("run-ended", None, None, CliOutputFormat::Json);
        assert!(notice.is_empty());
        let row: serde_json::Value = serde_json::from_str(stdout.trim_end()).unwrap();
        assert_eq!(row["available"], false);
    }

    #[test]
    fn the_job_state_and_each_control_answer_are_shown_in_both_formats() {
        // Decisions 0120 and 0122: the reconciled job state is read before
        // release and shown after each run with every control answer and its
        // action; an unavailable state names only what the driver observed
        // (review F1 of 8a3a341e).
        use agentmage_kernel_engine::job_control::{JobObservation, JobPhase};
        let status = |phase, revision, cancellation_requested| RuntimeJobStatus {
            schema_version: 1,
            run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-cli"),
            request_sha256: "c".repeat(64),
            job: JobObservation {
                job_id: "run-cli".to_owned(),
                phase,
                revision,
                cancellation_requested,
                head_sha256: "d".repeat(64),
            },
        };
        let controls = [
            KeptJobControl {
                action: JobControlAction::Suspend,
                answer: RuntimeJobControl {
                    decision: JobControlDecision::Applied {
                        revision: 2,
                        phase: JobPhase::Suspending,
                    },
                    status: status(JobPhase::Suspending, 2, false),
                },
            },
            KeptJobControl {
                action: JobControlAction::Resume,
                answer: RuntimeJobControl {
                    decision: JobControlDecision::Applied {
                        revision: 4,
                        phase: JobPhase::Queued,
                    },
                    status: status(JobPhase::Running, 5, false),
                },
            },
            KeptJobControl {
                action: JobControlAction::Cancel,
                answer: RuntimeJobControl {
                    decision: JobControlDecision::Refused {
                        refusal: JobControlRefusal::StaleRevision,
                        revision: 6,
                        phase: JobPhase::Running,
                    },
                    status: status(JobPhase::Running, 6, false),
                },
            },
            KeptJobControl {
                action: JobControlAction::Cancel,
                answer: RuntimeJobControl {
                    decision: JobControlDecision::Applied {
                        revision: 7,
                        phase: JobPhase::Cancelling,
                    },
                    status: status(JobPhase::Cancelling, 7, true),
                },
            },
        ];
        let ended = status(JobPhase::Cancelled, 8, true);
        let human = render_job_state(Ok(&ended), &controls, CliOutputFormat::Human);
        let lines = human.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 5);
        assert_eq!(
            lines[0],
            "job control: suspension request applied; job \"run-cli\" revision 2: suspension \
             requested; still running until a safe boundary"
        );
        assert_eq!(
            lines[1],
            "job control: resumption request applied; job \"run-cli\" revision 5: running"
        );
        assert_eq!(
            lines[2],
            "job control: cancellation request refused: the job changed since this client \
             observed it; job \"run-cli\" revision 6: running"
        );
        assert!(lines[3].starts_with("job control: cancellation request applied; "));
        assert!(lines[3].contains("revision 7: cancellation requested"));
        assert_eq!(lines[4], "job state: job \"run-cli\" revision 8: cancelled");
        let json: serde_json::Value = serde_json::from_str(
            render_job_state(Ok(&ended), &controls, CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["type"], "job_state");
        assert_eq!(json["available"], true);
        assert!(json["reason"].is_null());
        assert_eq!(json["job"], serde_json::to_value(&ended.job).unwrap());
        let kept = json["controls"].as_array().unwrap();
        assert_eq!(kept.len(), 4);
        assert_eq!(kept[0]["action"], "suspend");
        assert_eq!(kept[1]["action"], "resume");
        assert_eq!(kept[1]["decision"]["phase"], "queued");
        assert_eq!(kept[1]["job"]["phase"], "running");
        assert_eq!(kept[2]["action"], "cancel");
        assert_eq!(kept[2]["decision"]["refusal"], "stale-revision");
        assert_eq!(kept[3]["decision"]["decision"], "applied");
        assert_eq!(kept[3]["job"]["phase"], "cancelling");

        // An unavailable state names what was observed, never a cause.
        for (unavailable, code, text) in [
            (
                JobStateUnavailable::NotOffered,
                "not_offered",
                "the host offers no job state for this run",
            ),
            (
                JobStateUnavailable::NotAnswered,
                "not_answered",
                "the host did not answer the status request",
            ),
            (
                JobStateUnavailable::NotDescribed,
                "not_described",
                "the host's answer did not describe this run",
            ),
        ] {
            let human = render_job_state(Err(unavailable), &[], CliOutputFormat::Human);
            assert_eq!(human, format!("job state: unavailable; {text}\n"));
            assert!(!human.contains("ledger"));
            let json: serde_json::Value = serde_json::from_str(
                render_job_state(Err(unavailable), &[], CliOutputFormat::Json).trim_end(),
            )
            .unwrap();
            assert_eq!(json["available"], false);
            assert_eq!(json["reason"], code);
            assert!(json["job"].is_null());
            assert!(json["controls"].as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn each_job_control_event_is_shown_as_it_happens_in_both_formats() {
        // Decision 0122: a suspension, its continuation, each answer and a
        // request the host did not take are shown on standard error as they
        // happen.
        use agentmage_kernel_engine::job_control::{JobObservation, JobPhase};
        let point = RuntimeSuspensionPoint {
            checkpoint_id: agentmage_kernel_contracts::SessionCheckpointId::from_raw(
                "checkpoint-cli",
            ),
            checkpoint_sha256: "e".repeat(64),
            event_cursor: agentmage_kernel_contracts::RuntimeEventCursor {
                run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-cli"),
                event_id: agentmage_kernel_contracts::RuntimeEventId::from_raw("event-cli"),
                sequence: 12,
                event_sha256: "f".repeat(64),
            },
        };
        let answer = RuntimeJobControl {
            decision: JobControlDecision::Applied {
                revision: 2,
                phase: JobPhase::Suspending,
            },
            status: RuntimeJobStatus {
                schema_version: 1,
                run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-cli"),
                request_sha256: "c".repeat(64),
                job: JobObservation {
                    job_id: "run-cli".to_owned(),
                    phase: JobPhase::Suspending,
                    revision: 2,
                    cancellation_requested: false,
                    head_sha256: "d".repeat(64),
                },
            },
        };
        let answered = JobControlNotice::Answered {
            action: JobControlAction::Suspend,
            answer: &answer,
        };
        assert_eq!(
            render_job_control_notice(answered, CliOutputFormat::Human),
            "job control answer: suspension request applied; job \"run-cli\" revision 2: \
             suspension requested; still running until a safe boundary\n"
        );
        let json: serde_json::Value = serde_json::from_str(
            render_job_control_notice(answered, CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["type"], "job_control_answer");
        assert_eq!(json["action"], "suspend");
        assert_eq!(json["job"]["phase"], "suspending");

        let suspended =
            render_job_control_notice(JobControlNotice::Suspended(&point), CliOutputFormat::Human);
        assert_eq!(
            suspended,
            "run suspended at a safe boundary: checkpoint \"checkpoint-cli\" after event 12; \
             send SIGUSR2 to resume it or SIGINT to cancel it\n"
        );
        for (notice, kind) in [
            (JobControlNotice::Suspended(&point), "run_suspended"),
            (JobControlNotice::Continued(&point), "run_continued"),
        ] {
            let json: serde_json::Value = serde_json::from_str(
                render_job_control_notice(notice, CliOutputFormat::Json).trim_end(),
            )
            .unwrap();
            assert_eq!(json["type"], kind);
            assert_eq!(json["checkpoint_id"], "checkpoint-cli");
            assert_eq!(json["checkpoint_sha256"], "e".repeat(64));
            assert_eq!(json["event_sequence"], 12);
        }
        assert_eq!(
            render_job_control_notice(JobControlNotice::Continued(&point), CliOutputFormat::Human),
            "run continues from checkpoint \"checkpoint-cli\" after event 12 through a new \
             composition\n"
        );
        let not_taken = JobControlNotice::NotTaken {
            action: JobControlAction::Resume,
        };
        assert_eq!(
            render_job_control_notice(not_taken, CliOutputFormat::Human),
            "job control: the host did not take the resumption request; the run continues \
             unchanged\n"
        );
        let json: serde_json::Value = serde_json::from_str(
            render_job_control_notice(not_taken, CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["type"], "job_control_not_taken");
        assert_eq!(json["action"], "resume");

        // Review F2 of `3c69304c`: a run the host released is never shown as
        // continuing unchanged.
        let released = JobControlNotice::Released {
            action: JobControlAction::Resume,
            point: &point,
        };
        assert_eq!(
            render_job_control_notice(released, CliOutputFormat::Human),
            "job control: the host could not continue the run suspended at checkpoint \
             \"checkpoint-cli\" after event 12 after the resumption request and no longer holds \
             it; the job keeps the phase its ledger recorded\n"
        );
        let json: serde_json::Value = serde_json::from_str(
            render_job_control_notice(released, CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["type"], "run_released");
        assert_eq!(json["action"], "resume");
        assert_eq!(json["checkpoint_id"], "checkpoint-cli");
        assert_eq!(json["event_sequence"], 12);
    }

    #[test]
    fn the_suspend_resume_probe_suspends_once_and_resumes_only_a_suspended_run() {
        let mut probe = SuspendResumeProbe::Suspend;
        assert_eq!(probe.next(false), Some(InteractiveCliJobControl::Suspend));
        assert_eq!(probe.next(false), None);
        assert_eq!(probe.next(false), None);
        assert_eq!(probe.next(true), Some(InteractiveCliJobControl::Resume));
        assert_eq!(probe.next(true), None);
        assert_eq!(probe, SuspendResumeProbe::Done);
        let mut off = SuspendResumeProbe::Off;
        assert_eq!(off.next(false), None);
        assert_eq!(off.next(true), None);
    }

    fn limits() -> RuntimeRunLimits {
        RuntimeRunLimits {
            max_turns: 8,
            max_model_calls: 8,
            max_tool_calls: 16,
            max_repeated_tool_calls: 2,
            max_tool_call_depth: 1,
            max_no_progress_turns: 2,
            max_context_refreshes: 2,
            max_events: 256,
            max_elapsed_ms: 60_000,
            max_output_bytes: 65_536,
        }
    }

    /// A sealed two-event run: started, then ended in `state`.
    fn sealed_run(run: &str, state: AgentStateKind) -> Vec<RuntimeEvent> {
        use agentmage_kernel_contracts::{
            CONTRACT_SCHEMA_VERSION, ContextSensitivity, CorrelationId, PolicyId, RuntimeEventId,
            RuntimeEventKind, RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeRunId,
            SessionId, TaskId,
        };
        use agentmage_kernel_engine::runtime_event::{
            runtime_event_persistence, seal_runtime_event,
        };
        let zero = "0".repeat(64);
        let mut events: Vec<RuntimeEvent> = Vec::new();
        for (sequence, kind) in [
            RuntimeEventKind::RunStarted {
                request_sha256: "1".repeat(64),
            },
            RuntimeEventKind::RunTerminal {
                state,
                outcome_sha256: "2".repeat(64),
            },
        ]
        .into_iter()
        .enumerate()
        {
            let previous = events.last();
            events.push(
                seal_runtime_event(RuntimeEvent {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    event_id: RuntimeEventId::from_raw(format!("{run}-event-{sequence}")),
                    run_id: RuntimeRunId::from_raw(run),
                    session_id: SessionId::from_raw("session-cli-progress"),
                    task_id: TaskId::from_raw("task-cli-progress"),
                    turn_id: None,
                    operation_id: None,
                    correlation_id: CorrelationId::from_raw("correlation-cli-progress"),
                    causation_event_id: previous.map(|event| event.event_id.clone()),
                    sequence: sequence as u64,
                    occurred_at_epoch_ms: 1_000 + sequence as u64,
                    sensitivity: ContextSensitivity::Internal,
                    retention: RuntimeEventRetention {
                        kind: RuntimeEventRetentionKind::Ephemeral,
                        expires_at_epoch_ms: None,
                    },
                    persistence: runtime_event_persistence(&kind),
                    policy_id: PolicyId::from_raw("policy-cli-progress"),
                    payload_reference: None,
                    kind,
                    previous_event_sha256: previous
                        .map_or_else(|| zero.clone(), |event| event.event_sha256.clone()),
                    event_sha256: zero.clone(),
                })
                .unwrap(),
            );
        }
        events
    }

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text.trim_end()).unwrap()
    }

    #[test]
    fn run_progress_is_projected_per_run_and_never_claims_review_or_delivery() {
        let limits = limits();
        let mut sink = TerminalEventSink::new(CliOutputFormat::Human, MAX_PROGRESS_EVENTS);
        let text = sink.take_progress(&limits);
        assert!(text.contains("not started"), "{text}");
        assert!(text.contains("turns: 0 of 8 declared (0%)"), "{text}");
        assert!(text.contains("independent review: not established by the runtime"));

        // Two consecutive runs: each projection covers only its own run.
        for output in [CliOutputFormat::Human, CliOutputFormat::Json] {
            sink.output = output;
            for event in sealed_run("run-first", AgentStateKind::Failed) {
                sink.present(&event).unwrap();
            }
            let first = sink.take_progress(&limits);
            assert!(sink.run_events.is_empty());
            for event in sealed_run("run-second", AgentStateKind::Success) {
                sink.present(&event).unwrap();
            }
            let second = sink.take_progress(&limits);
            match output {
                CliOutputFormat::Human => {
                    assert!(
                        first.starts_with("run: ended failed; not verified"),
                        "{first}"
                    );
                    assert!(
                        second.starts_with("run: ended; verified locally"),
                        "{second}"
                    );
                    assert!(!second.contains("failed"), "{second}");
                    assert!(second.contains("delivery: not established by the runtime"));
                }
                CliOutputFormat::Json => {
                    let (first, second) = (json(&first), json(&second));
                    assert_eq!(first["progress"]["run_id"], "run-first");
                    assert_eq!(second["type"], "run_progress");
                    assert_eq!(second["available"], true);
                    assert_eq!(second["progress"]["run_id"], "run-second");
                    assert_eq!(second["progress"]["events"], 2);
                    assert_eq!(second["progress"]["terminal_state"], "SUCCESS");
                    assert_eq!(
                        second["progress"]["independently_reviewed_established"],
                        false
                    );
                    assert_eq!(second["progress"]["delivered_established"], false);
                }
            }
        }
    }

    #[test]
    fn an_unverifiable_or_truncated_run_is_reported_unavailable_in_both_formats() {
        let limits = limits();
        let first = sealed_run("run-a", AgentStateKind::Failed);
        let second = sealed_run("run-b", AgentStateKind::Success);
        // Events of two runs do not form one verified stream.
        let mixed = [first[0].clone(), second[1].clone()];
        let mut sink = TerminalEventSink::new(CliOutputFormat::Human, MAX_PROGRESS_EVENTS);
        for output in [CliOutputFormat::Human, CliOutputFormat::Json] {
            sink.output = output;
            for event in &mixed {
                sink.present(event).unwrap();
            }
            let text = sink.take_progress(&limits);
            match output {
                CliOutputFormat::Human => assert_eq!(
                    text,
                    "run progress unavailable: the presented stream is not one complete run\n"
                ),
                CliOutputFormat::Json => {
                    assert_eq!(
                        json(&text),
                        serde_json::json!({"type": "run_progress", "available": false})
                    );
                }
            }
        }
        // A run longer than the sink keeps is never projected from its prefix,
        // which would verify and claim the run had not ended.
        let mut sink = TerminalEventSink::new(CliOutputFormat::Json, 1);
        for event in &first {
            sink.present(event).unwrap();
        }
        assert_eq!(sink.run_events.len(), 1);
        assert_eq!(json(&sink.take_progress(&limits))["available"], false);
        sink.output = CliOutputFormat::Human;
        for event in &first {
            sink.present(event).unwrap();
        }
        assert!(
            sink.take_progress(&limits)
                .starts_with("run progress unavailable")
        );
        // The mark is cleared with the buffer; a run that fits is projected.
        let mut sink = TerminalEventSink::new(CliOutputFormat::Human, 2);
        for event in &second {
            sink.present(event).unwrap();
        }
        assert!(
            sink.take_progress(&limits)
                .starts_with("run: ended; verified locally")
        );
        assert!(!sink.overflowed);
    }

    #[test]
    fn the_review_reader_stays_inside_the_workspace_and_renders_both_formats() {
        let scratch =
            std::env::temp_dir().join(format!("agentmage-review-workspace-{}", std::process::id()));
        let root = scratch.join("worktree");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(scratch.join("outside.txt"), b"outside\n").unwrap();
        std::fs::write(root.join("src/notes.txt"), b"old text\n").unwrap();
        std::os::unix::fs::symlink(scratch.join("outside.txt"), root.join("src/link.txt")).unwrap();
        let workspace = ReviewWorkspace {
            workspace_id: WorkspaceId::from_raw("coding-development-review"),
            root: root.clone(),
        };
        let path = |text: &str| text.split('/').map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(
            workspace.read(&path("src/notes.txt")),
            ReviewTarget::Bytes(b"old text\n".to_vec())
        );
        std::os::unix::fs::symlink(scratch.join("gone.txt"), root.join("src/dangling.txt"))
            .unwrap();
        std::os::unix::fs::symlink(&scratch, root.join("linked-dir")).unwrap();
        std::fs::create_dir(root.join("src/directory.txt")).unwrap();
        std::fs::File::create(root.join("src/large.txt"))
            .unwrap()
            .set_len(MAX_REVIEW_READ_BYTES + 1)
            .unwrap();
        rustix::fs::mknodat(
            rustix::fs::CWD,
            root.join("src/fifo.txt"),
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::from(0o600),
            0,
        )
        .unwrap();
        for refused in [
            "src/link.txt",
            "src/dangling.txt",
            "src/directory.txt",
            "src/large.txt",
            "src/fifo.txt",
            "linked-dir/outside.txt",
            "src/notes.txt/child.txt",
            "src/../../outside.txt",
            "src",
            "",
        ] {
            assert_eq!(
                workspace.read(&path(refused)),
                ReviewTarget::Refused,
                "{refused}"
            );
        }
        for absent in ["src/missing.txt", "missing-dir/missing.txt"] {
            assert_eq!(
                workspace.read(&path(absent)),
                ReviewTarget::Absent,
                "{absent}"
            );
        }
        // A creation review is shown only where nothing exists.
        let creation = |target: &str| {
            let challenge = crate::coding_change_review::tests_support::write_challenge(
                crate::coding_changes::CONTROLLED_CREATE_TOOL_ID,
                serde_json::to_vec(&serde_json::json!({
                    "schema_version": 1, "creation_id": "creation-1", "path": path(target),
                    "content": "created\n", "mode": 420, "classification": "source_code",
                    "intent_sha256": "a".repeat(64), "change_plan_sha256": "b".repeat(64),
                    "expected_parent_sha256": "c".repeat(64)
                }))
                .unwrap(),
            );
            workspace
                .render(&challenge, CliOutputFormat::Human)
                .unwrap()
        };
        assert!(creation("src/missing.txt").contains("+created\n"));
        for occupied in [
            "src/link.txt",
            "src/dangling.txt",
            "src/directory.txt",
            "src/large.txt",
        ] {
            let text = creation(occupied);
            assert!(
                text.contains("coding.change-review.target-unavailable"),
                "{occupied}: {text}"
            );
            assert!(!text.contains("+created"), "{occupied}: {text}");
        }
        use crate::coding_change_review::tests_support::{sha256_hex, write_challenge};
        let arguments = serde_json::json!({
            "schema_version": 1, "change_id": "change-1", "path": ["src", "notes.txt"],
            "expected_preimage_sha256": sha256_hex(b"old text\n"),
            "intent_sha256": "a".repeat(64), "change_plan_sha256": "b".repeat(64),
            "language": "plain_text", "artifact_class": "documentation",
            "edits": [{"kind": "replace_exact_text", "edit_id": "edit-1",
                       "expected": "old text", "replacement": "new text"}],
            "additional_review_hooks": [], "generated": false, "allow_generated": false
        });
        let challenge = write_challenge(
            crate::coding_changes::STRUCTURED_PATCH_TOOL_ID,
            serde_json::to_vec(&arguments).unwrap(),
        );
        let human = workspace
            .render(&challenge, CliOutputFormat::Human)
            .unwrap();
        assert!(human.contains("-old text\n+new text\n"), "{human}");
        let json = workspace.render(&challenge, CliOutputFormat::Json).unwrap();
        let value: serde_json::Value = serde_json::from_str(json.trim_end()).unwrap();
        assert_eq!(value["type"], "change_review");
        assert_eq!(value["hunk_count"], 1);
        std::fs::write(root.join("src/notes.txt"), b"human edit\n").unwrap();
        let json = workspace.render(&challenge, CliOutputFormat::Json).unwrap();
        assert!(
            json.contains("coding.change-review.preimage-changed"),
            "{json}"
        );
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn a_hunk_selection_names_at_least_one_and_fewer_than_all_hunks() {
        assert_eq!(parse_hunk_selection("select 1 3", 3), Some(vec![1, 3]));
        assert_eq!(parse_hunk_selection("select 3,1, 3", 4), Some(vec![1, 3]));
        for refused in [
            "select",
            "select ",
            "select 0",
            "select 4",
            "select 1 2 3",
            "select x",
            "select -1",
            "selected 1",
            "select1",
            "yes",
        ] {
            assert_eq!(parse_hunk_selection(refused, 3), None, "{refused}");
        }
        let review = ChangeReview {
            path: vec!["src".to_owned(), "notes.txt".to_owned()],
            preimage_sha256: "1".repeat(64),
            postimage_sha256: "2".repeat(64),
            hunk_count: 3,
            hunk_ids: vec!["c".repeat(64), "a".repeat(64), "b".repeat(64)],
            selectable: true,
            selected_from: None,
            rendered: String::new(),
        };
        let selection = hunk_selection(&review, &[1, 3]);
        assert_eq!(
            selection.accepted_hunk_ids,
            ["b".repeat(64), "c".repeat(64)]
        );
        assert_eq!(selection.preimage_sha256, review.preimage_sha256);
        assert_eq!(selection.proposal_sha256, review.postimage_sha256);
    }

    #[test]
    fn a_selectable_review_lists_its_hunks_and_the_derived_write_shows_only_the_selection() {
        use crate::coding_change_review::tests_support::{sha256_hex, write_challenge};
        use crate::coding_changes::{STRUCTURED_PATCH_TOOL_ID, StructuredPatchProposal};
        let scratch = std::env::temp_dir().join(format!(
            "agentmage-selection-workspace-{}",
            std::process::id()
        ));
        let root = scratch.join("worktree");
        std::fs::create_dir_all(root.join("src")).unwrap();
        let text = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\n";
        std::fs::write(root.join("src/notes.txt"), text).unwrap();
        let workspace = ReviewWorkspace {
            workspace_id: WorkspaceId::from_raw("coding-development-selection"),
            root: root.clone(),
        };
        let arguments = serde_json::json!({
            "schema_version": 1, "change_id": "change-1", "path": ["src", "notes.txt"],
            "expected_preimage_sha256": sha256_hex(text.as_bytes()),
            "intent_sha256": "a".repeat(64), "change_plan_sha256": "b".repeat(64),
            "language": "plain_text", "artifact_class": "documentation",
            "edits": [{"kind": "replace_exact_text", "edit_id": "edit-1",
                       "expected": "two", "replacement": "TWO"},
                      {"kind": "replace_exact_text", "edit_id": "edit-2",
                       "expected": "eight", "replacement": "EIGHT"}],
            "additional_review_hooks": [], "generated": false, "allow_generated": false
        });
        let original = write_challenge(
            STRUCTURED_PATCH_TOOL_ID,
            serde_json::to_vec(&arguments).unwrap(),
        );
        let review = workspace.review(&original).unwrap().unwrap();
        assert!(review.selectable);
        assert_eq!(review.hunk_count, 2);
        let human = format_review(&original, &Ok(review.clone()), CliOutputFormat::Human);
        assert!(human.contains("hunk 1 = "), "{human}");
        assert!(human.contains("hunk 2 = "), "{human}");
        let json: serde_json::Value = serde_json::from_str(
            format_review(&original, &Ok(review.clone()), CliOutputFormat::Json).trim_end(),
        )
        .unwrap();
        assert_eq!(json["selectable"], true);
        assert_eq!(json["hunk_ids"].as_array().unwrap().len(), 2);

        // The host derives the write of hunk 2 from the person's selection.
        let selection = hunk_selection(&review, &[2]);
        let proposal: StructuredPatchProposal = serde_json::from_value(arguments).unwrap();
        let scope = crate::coding_changes::CodingWriteScope::new(
            WorkspaceId::from_raw("coding-development-selection"),
            vec![vec!["src".to_owned()]],
        )
        .unwrap();
        let plan = crate::coding_changes::bind_structured_patch_proposal(
            &scope,
            proposal,
            text.as_bytes().to_vec(),
        )
        .unwrap();
        let mut call = original.clone();
        let derived = crate::coding_hunk_selection::derive_hunk_selection_call(
            &scope,
            &agentmage_kernel_contracts::ToolCall {
                schema_version: call.schema_version,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw("c"),
                action_id: agentmage_kernel_contracts::ActionId::from_raw("a"),
                tool_id: call.presentation.tool_id.clone(),
                tool_version: call.presentation.tool_version.clone(),
                arguments: call.presentation.arguments.clone(),
            },
            plan.preimage(),
            plan.postimage(),
            text.as_bytes(),
            &selection,
        )
        .unwrap();
        call.presentation.tool_id = derived.tool_id;
        call.presentation.arguments = derived.arguments;
        let derived_review = workspace.review(&call).unwrap().unwrap();
        assert!(!derived_review.selectable);
        assert_eq!(derived_review.selected_from, Some((1, 2)));
        let text_review = format_review(&call, &Ok(derived_review), CliOutputFormat::Human);
        assert!(
            text_review.contains("only the 1 of 2 you selected"),
            "{text_review}"
        );
        assert!(text_review.contains("-eight\n+EIGHT\n"), "{text_review}");
        assert!(!text_review.contains("TWO"), "{text_review}");
        // A person's later edit makes the derived review unavailable, not misleading.
        std::fs::write(root.join("src/notes.txt"), text.replace("five", "5")).unwrap();
        assert_eq!(
            workspace.review(&call).unwrap(),
            Err(ChangeReviewUnavailable::PreimageChanged)
        );
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn preauthorization_cancellation_remains_unconsumed_and_returns_cancelled_exit() {
        let arguments = [
            "code",
            "--development",
            "--state-root",
            "/synthetic-state",
            "--disposable-root",
            "/synthetic-disposable",
            "--workspace-root",
            "/synthetic-disposable/worktree",
            "--scenario",
            "failed-test-repair",
            "--model",
            "scripted",
            "--objective",
            "Synthetic input component test.",
            "--preauthorize-workspace-reads",
            "--preauthorization-budget",
            "1",
            "--preauthorization-minutes",
            "1",
        ]
        .map(str::to_owned);
        let crate::cli::CliInvocation::Code {
            development: Some(options),
            ..
        } = crate::cli::parse_cli_arguments(&arguments).unwrap()
        else {
            panic!("explicit development options");
        };
        let requested = AtomicBool::new(true);
        let error = direct_session_preauthorization(&options, "synthetic-workspace", &requested)
            .expect_err("pending stop must not wait on stdin or confirm a contract");
        assert_eq!(
            error,
            CodingDevelopmentClientError::HostBoundary(
                LinuxDevelopmentBoundaryErrorKind::StartupCancelled
            )
        );
        assert_eq!(error.exit_code(), ClientExitCode::Cancelled);
        assert!(requested.load(Ordering::Acquire));
    }

    #[test]
    fn approval_delay_enforces_its_existing_ceiling_and_preserves_pending_cancellation() {
        assert_eq!(
            wait_for_approval_delay(0, &AtomicBool::new(false)),
            Ok(true)
        );
        assert_eq!(
            wait_for_approval_delay(10_001, &AtomicBool::new(false)),
            Err(CodingClientError::Approval)
        );
        let requested = AtomicBool::new(true);
        assert_eq!(wait_for_approval_delay(10_000, &requested), Ok(false));
        assert!(requested.load(Ordering::Acquire));
    }

    #[test]
    fn approval_delay_observes_a_later_stop_before_the_full_delay() {
        let requested = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&requested);
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            signal.store(true, Ordering::Release);
        });
        let started = Instant::now();
        let result = wait_for_approval_delay(10_000, &requested);
        writer.join().unwrap();
        assert_eq!(result, Ok(false));
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(requested.load(Ordering::Acquire));
    }

    #[test]
    fn host_launch_refusal_is_preserved_without_relabeling_as_transport() {
        let error = LinuxDevelopmentHostProcess::launch(
            std::path::Path::new("/synthetic-private-state-canary"),
            std::path::Path::new("/synthetic-private-disposable-canary"),
            std::path::Path::new("/synthetic-private-workspace-canary"),
            "unregistered-scenario",
            "scripted",
            false,
        )
        .map_err(CodingDevelopmentClientError::from)
        .err()
        .expect("closed input refuses before launch");
        assert_eq!(
            error,
            CodingDevelopmentClientError::HostBoundary(
                LinuxDevelopmentBoundaryErrorKind::InvalidInput
            )
        );
        assert_eq!(error.code(), "linux.development.input.invalid");
        assert_eq!(error.exit_code(), ClientExitCode::ServiceUnavailable);
        assert!(!format!("{error:?} {}", error.code()).contains("canary"));
    }
}
