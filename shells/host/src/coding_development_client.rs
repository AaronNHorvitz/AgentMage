//! Executable CLI client for the explicitly activated disposable coding harness.

use std::io::{self, BufRead};
use std::process::{Child, Command, Stdio};

use agentmage_kernel_contracts::{
    AgentStateKind, RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeEvent,
};
use agentmage_platform_linux::LinuxLaunchEnvelope;

use crate::cli::{
    CliOutputFormat, CodingDevelopmentCliOptions, render_runtime_approval_human,
    render_runtime_event_human, render_runtime_event_json, render_runtime_outcome_human,
    render_runtime_outcome_json,
};
use crate::cli_runtime::{NeverCancelInteractiveCli, drive_interactive_cli_runtime};
use crate::coding_client::{CodingApprovalPort, CodingClientError, CodingEventSink};
use crate::coding_development_activation::CodingDevelopmentActivation;
use crate::coding_development_runtime::SCRIPTED_PROFILE_ID;
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
    let mut approvals = TerminalApprovals {
        output,
        preauthorized: options.approve_this_run,
    };
    let mut sink = TerminalEventSink { output };
    let workspace_id = format!("coding-development-{}", &activation.marker_sha256()[..24]);
    let result = drive_interactive_cli_runtime(
        &mut runtime,
        RuntimePrepareInput {
            engineering_session_id: None,
            profile_id: SCRIPTED_PROFILE_ID.to_owned(),
            expected_entry_sha256: activation.marker_sha256().to_owned(),
            workspace_id,
            workspace_root: activation.workspace_root().to_string_lossy().into_owned(),
            prompt: options.objective.clone(),
        },
        &mut approvals,
        &mut sink,
        &mut NeverCancelInteractiveCli,
    );
    if let Err(error) = &result {
        if let Some(transport) = runtime.last_error() {
            eprintln!("{}", transport.code());
        }
        eprintln!("{}", error.code());
    }
    let result = result.map_err(|_| CodingDevelopmentClientError::Runtime);
    let shutdown = runtime
        .shutdown()
        .map_err(|_| CodingDevelopmentClientError::Transport);
    let result = result?;
    shutdown?;
    let rendered = match output {
        CliOutputFormat::Human => render_runtime_outcome_human(&result.request, &result.outcome),
        CliOutputFormat::Json => render_runtime_outcome_json(&result.request, &result.outcome),
    }
    .map_err(|_| CodingDevelopmentClientError::Presentation)?;
    println!("{rendered}");
    Ok(match result.outcome.state {
        AgentStateKind::Success | AgentStateKind::NoOp => ClientExitCode::Success,
        AgentStateKind::Declined | AgentStateKind::Blocked => ClientExitCode::PolicyDenied,
        AgentStateKind::Cancelled => ClientExitCode::Cancelled,
        AgentStateKind::Exhausted => ClientExitCode::ResourceBound,
        _ => ClientExitCode::Uncertain,
    })
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
