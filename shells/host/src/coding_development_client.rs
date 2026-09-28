//! Executable CLI client for the explicitly activated disposable coding harness.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use agentmage_kernel_contracts::{
    AgentStateKind, RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeEvent,
};
use agentmage_platform_linux::{
    LinuxDevelopmentBoundaryError, LinuxDevelopmentBoundaryErrorKind, LinuxDevelopmentConfirmation,
    LinuxDevelopmentConfirmationKind, LinuxDevelopmentHostProcess, read_development_confirmation,
};

use crate::cli::{
    CliOutputFormat, CodingDevelopmentCliOptions, render_runtime_approval_human,
    render_runtime_event_human, render_runtime_event_json, render_runtime_outcome_human,
    render_runtime_outcome_json,
};
use crate::cli_runtime::{
    InteractiveCliCancellationPort, InteractiveCliRuntimeError, drive_interactive_cli_runtime,
};
use crate::coding_client::{CodingApprovalPort, CodingClientError, CodingEventSink};
use crate::coding_development_activation::CodingDevelopmentActivation;
use crate::coding_development_runtime::CodingDevelopmentModel;
use crate::headless::ClientExitCode;
use crate::runtime_ipc::LinuxRuntimeIpcClient;
use crate::runtime_transport::{
    RuntimePreauthorizedCommand, RuntimePrepareInput, RuntimeSessionPreauthorization,
    RuntimeTransportPort,
};

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
    let mut cancellation = InstalledSignalCancellation::install()?;
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
    let mut approvals = TerminalApprovals {
        output,
        preauthorized: options.approve_this_run,
        delay_ms: options.approval_delay_ms,
        cancellation: Arc::clone(&cancellation.requested),
    };
    let mut sink = TerminalEventSink { output };
    cancellation.check_startup()?;
    let workspace_id = format!("coding-development-{}", &activation.marker_sha256()[..24]);
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

struct InstalledSignalCancellation {
    requested: Arc<AtomicBool>,
    sequence: u64,
    registrations: [signal_hook::SigId; 2],
}

impl InstalledSignalCancellation {
    fn install() -> Result<Self, CodingDevelopmentClientError> {
        let requested = Arc::new(AtomicBool::new(false));
        let interrupt =
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&requested))
                .map_err(|_| CodingDevelopmentClientError::Transport)?;
        let terminate =
            match signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&requested))
            {
                Ok(registration) => registration,
                Err(_) => {
                    signal_hook::low_level::unregister(interrupt);
                    return Err(CodingDevelopmentClientError::Transport);
                }
            };
        Ok(Self {
            requested,
            sequence: 0,
            registrations: [interrupt, terminate],
        })
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
        for registration in self.registrations {
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
}

struct TerminalEventSink {
    output: CliOutputFormat,
}

impl CodingEventSink for TerminalEventSink {
    fn present(&mut self, event: &RuntimeEvent) -> Result<(), CodingClientError> {
        let rendered = match self.output {
            CliOutputFormat::Human => render_runtime_event_human(event),
            CliOutputFormat::Json => render_runtime_event_json(event),
        }
        .map_err(|_| CodingClientError::Presentation)?;
        println!("{rendered}");
        Ok(())
    }
}

struct TerminalApprovals {
    output: CliOutputFormat,
    preauthorized: bool,
    delay_ms: u64,
    cancellation: Arc<AtomicBool>,
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
        let rendered =
            render_runtime_approval_human(challenge).map_err(|_| CodingClientError::Approval)?;
        if self.preauthorized {
            eprintln!("preauthorized_for_this_run {rendered}");
            return Ok(
                if wait_for_approval_delay(self.delay_ms, &self.cancellation)? {
                    RuntimeApprovalDisposition::Allow
                } else {
                    // The interactive driver polls this unconsumed flag before
                    // transmitting any approval response, so canonical cancel wins.
                    RuntimeApprovalDisposition::Deny
                },
            );
        }
        if self.output == CliOutputFormat::Json {
            eprintln!("approval_required {rendered}");
        } else {
            eprintln!("{rendered}");
        }
        eprint!("Approve this exact operation? Type yes to allow: ");
        match read_development_confirmation(
            LinuxDevelopmentConfirmationKind::OperationApproval,
            &self.cancellation,
        )
        .map_err(|_| CodingClientError::Approval)?
        {
            LinuxDevelopmentConfirmation::Confirmed => Ok(RuntimeApprovalDisposition::Allow),
            LinuxDevelopmentConfirmation::Declined | LinuxDevelopmentConfirmation::Cancelled => {
                // Cancellation remains pending for the existing driver's next
                // poll; this value must not be sent ahead of that cancel.
                Ok(RuntimeApprovalDisposition::Deny)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
