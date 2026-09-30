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
use crate::coding_change_review::{
    ChangeReview, ChangeReviewUnavailable, ReviewTarget, render_change_review, review_coding_write,
};
use crate::coding_client::{
    CodingApprovalPort, CodingClientError, CodingEventSink, JobControlNotice,
};
use crate::coding_development_activation::CodingDevelopmentActivation;
use crate::coding_development_runtime::CodingDevelopmentModel;
use crate::coding_recoverability::render_recoverability;
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
pub fn run_coding_development(
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let activation = CodingDevelopmentActivation::validate(
        &options.state_root,
        &options.disposable_root,
        &options.workspace_root,
    )
    .map_err(|_| CodingDevelopmentClientError::Activation)?;
    let mut cancellation = InstalledSignalCancellation::install()?
        .with_suspend_resume_probe(options.suspend_resume_probe);
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
    let result = run_with_child(&activation, options, output, &mut child, &mut cancellation);
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

fn run_with_child(
    activation: &CodingDevelopmentActivation,
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
    child: &mut LinuxDevelopmentHostProcess,
    cancellation: &mut InstalledSignalCancellation,
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
        for verified in &result.verified_artifacts {
            match output {
                CliOutputFormat::Human => println!(
                    "artifact_verified id={} media_type={} sha256={} bytes={} pages={}",
                    verified.reference.artifact_id.as_str(),
                    verified.reference.media_type,
                    verified.reference.payload_sha256,
                    verified.reference.byte_size,
                    verified.page_count,
                ),
                CliOutputFormat::Json => println!(
                    "{}",
                    serde_json::json!({
                        "type": "runtime_artifact_verified",
                        "artifact_id": verified.reference.artifact_id.as_str(),
                        "media_type": verified.reference.media_type,
                        "payload_sha256": verified.reference.payload_sha256,
                        "byte_size": verified.reference.byte_size,
                        "page_count": verified.page_count,
                    })
                ),
            }
        }
        let rendered = match output {
            CliOutputFormat::Human => {
                render_runtime_outcome_human(&result.request, &result.outcome)
            }
            CliOutputFormat::Json => render_runtime_outcome_json(&result.request, &result.outcome),
        }
        .map_err(|_| CodingDevelopmentClientError::Presentation)?;
        println!("{rendered}");
        // Stderr keeps the machine stream's contract that the outcome is last on stdout.
        eprint!("{}", sink.take_progress(&result.request.limits));
        eprint!(
            "{}",
            render_run_declarations(result.declarations.as_ref(), output)
        );
        eprint!(
            "{}",
            render_job_state(
                result.job.as_ref().map_err(|unavailable| *unavailable),
                &result.job_controls,
                output
            )
        );
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
/// recoverability of its effects and the view of each composed context. Each
/// part the host could not declare completely is shown as unavailable.
fn render_run_declarations(
    declarations: Option<&RuntimeRunDeclarations>,
    output: CliOutputFormat,
) -> String {
    let recoverability = declarations.and_then(|value| value.recoverability.as_ref());
    let contexts = declarations.and_then(|value| value.context_inspections.as_ref());
    if output == CliOutputFormat::Json {
        return format!(
            "{}\n",
            serde_json::json!({
                "type": "run_declarations",
                "recoverability_available": recoverability.is_some(),
                "recoverability": recoverability,
                "context_inspections_available": contexts.is_some(),
                "context_inspections": contexts,
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
    rendered
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
        let declarations = RuntimeRunDeclarations {
            schema_version: 1,
            run_id: agentmage_kernel_contracts::RuntimeRunId::from_raw("run-cli"),
            request_sha256: "c".repeat(64),
            recoverability: Some(report.clone()),
            context_inspections: Some(vec![view.clone(), view]),
        };
        let human = render_run_declarations(Some(&declarations), CliOutputFormat::Human);
        assert!(human.starts_with("recoverability of this run's effects: "));
        assert!(human.contains("operation-test external or uncertain: reconcile manually"));
        assert!(human.contains("context view 1 of 2:\n"));
        assert!(human.contains("context view 2 of 2:\n"));
        assert!(!human.contains("undone"));
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
