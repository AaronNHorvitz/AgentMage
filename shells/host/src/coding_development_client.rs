//! Executable CLI client for the explicitly activated disposable coding harness.

use std::io::{self, BufRead};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use agentmage_kernel_contracts::{
    AgentStateKind, RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeEvent,
};
use agentmage_platform_linux::LinuxLaunchEnvelope;

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
use crate::runtime_transport::RuntimePrepareInput;

/// Stable content-free failure from the development-only CLI launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingDevelopmentClientError {
    /// The activation roots or marker were invalid.
    Activation,
    /// The exact sibling host could not be launched or authenticated.
    Transport,
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
            Self::Runtime => "coding.development.client.runtime-failed",
            Self::Presentation => "coding.development.client.presentation-failed",
        }
    }

    /// Returns the documented process exit class.
    #[must_use]
    pub const fn exit_code(self) -> ClientExitCode {
        match self {
            Self::Activation => ClientExitCode::AuthorityDenied,
            Self::Transport | Self::Runtime => ClientExitCode::ServiceUnavailable,
            Self::Presentation => ClientExitCode::ProtocolMismatch,
        }
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
    let host = sibling_host_path()?;
    let mut child = Command::new(host)
        .arg("--coding-development-host")
        .arg(activation.state_root())
        .arg(activation.disposable_root())
        .arg(activation.workspace_root())
        .arg(&options.scenario)
        .arg(&options.model)
        .arg(if options.resume { "resume" } else { "new" })
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|_| CodingDevelopmentClientError::Transport)?;
    let result = run_with_child(&activation, options, output, &mut child);
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|_| CodingDevelopmentClientError::Transport)?;
    if !status.success() && result.is_ok() {
        return Err(CodingDevelopmentClientError::Transport);
    }
    result
}

fn run_with_child(
    activation: &CodingDevelopmentActivation,
    options: &CodingDevelopmentCliOptions,
    output: CliOutputFormat,
    child: &mut Child,
) -> Result<ClientExitCode, CodingDevelopmentClientError> {
    let stdout = child
        .stdout
        .as_mut()
        .ok_or(CodingDevelopmentClientError::Transport)?;
    let envelope =
        LinuxLaunchEnvelope::read(stdout).map_err(|_| CodingDevelopmentClientError::Transport)?;
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
    }
    let mut approvals = TerminalApprovals {
        output,
        preauthorized: options.approve_this_run,
        delay_ms: options.approval_delay_ms,
    };
    let mut sink = TerminalEventSink { output };
    let mut cancellation = InstalledSignalCancellation::install()?;
    let workspace_id = format!("coding-development-{}", &activation.marker_sha256()[..24]);
    let profile_id = CodingDevelopmentModel::parse(&options.model)
        .ok_or(CodingDevelopmentClientError::Activation)?
        .profile_id();
    let mut objectives = Vec::with_capacity(options.follow_ups.len() + 1);
    objectives.push(options.objective.as_str());
    objectives.extend(options.follow_ups.iter().map(String::as_str));
    let mut engineering_session_id = None;
    let mut final_exit = ClientExitCode::Success;
    for objective in objectives {
        let result = drive_interactive_cli_runtime(
            &mut runtime,
            RuntimePrepareInput {
                resume: options.resume,
                slow_subscriber_probe: options.slow_subscriber_probe,
                engineering_session_id: engineering_session_id.clone(),
                profile_id: profile_id.to_owned(),
                expected_entry_sha256: activation.marker_sha256().to_owned(),
                workspace_id: workspace_id.clone(),
                workspace_root: activation.workspace_root().to_string_lossy().into_owned(),
                prompt: objective.to_owned(),
            },
            &mut approvals,
            &mut sink,
            &mut cancellation,
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
                    "artifact_verified id={} sha256={} bytes={} pages={}",
                    verified.reference.artifact_id.as_str(),
                    verified.reference.payload_sha256,
                    verified.reference.byte_size,
                    verified.page_count,
                ),
                CliOutputFormat::Json => println!(
                    "{}",
                    serde_json::json!({
                        "type": "runtime_artifact_verified",
                        "artifact_id": verified.reference.artifact_id.as_str(),
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
        if final_exit != ClientExitCode::Success {
            break;
        }
    }
    runtime
        .shutdown()
        .map_err(|_| CodingDevelopmentClientError::Transport)?;
    Ok(final_exit)
}

struct InstalledSignalCancellation {
    requested: Arc<AtomicBool>,
    sequence: u64,
}

impl InstalledSignalCancellation {
    fn install() -> Result<Self, CodingDevelopmentClientError> {
        let requested = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&requested))
            .map_err(|_| CodingDevelopmentClientError::Transport)?;
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&requested))
            .map_err(|_| CodingDevelopmentClientError::Transport)?;
        Ok(Self {
            requested,
            sequence: 0,
        })
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

fn sibling_host_path() -> Result<std::path::PathBuf, CodingDevelopmentClientError> {
    let current = std::env::current_exe().map_err(|_| CodingDevelopmentClientError::Transport)?;
    let host = current.with_file_name("agentmage-host");
    host.canonicalize()
        .map_err(|_| CodingDevelopmentClientError::Transport)
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
            if self.delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(self.delay_ms));
            }
            return Ok(RuntimeApprovalDisposition::Allow);
        }
        if self.output == CliOutputFormat::Json {
            eprintln!("approval_required {rendered}");
        } else {
            eprintln!("{rendered}");
        }
        eprint!("Approve this exact operation? Type yes to allow: ");
        let mut line = String::new();
        io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|_| CodingClientError::Approval)?;
        Ok(if line.trim() == "yes" {
            RuntimeApprovalDisposition::Allow
        } else {
            RuntimeApprovalDisposition::Deny
        })
    }
}
