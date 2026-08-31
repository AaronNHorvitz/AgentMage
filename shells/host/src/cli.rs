//! Strict local CLI parsing and display without storage, model, tool, or network access.

use serde::{Deserialize, Serialize};

use agentmage_kernel_contracts::{
    RuntimeApprovalChallenge, RuntimeEvent, RuntimeEventKind, RuntimeOutcome, RuntimeOutput,
    RuntimeRunRequest,
};
use agentmage_kernel_engine::{
    runtime_coordinator::{verify_runtime_approval_challenge, verify_runtime_outcome},
    runtime_event::verify_runtime_event,
};

use crate::headless::{
    ClientCommand, ClientContentChannel, ClientExitCode, ClientSurface, ConversationClientCommand,
    KnowledgeClientCommand, KnowledgeRetrievalClientMode, KnowledgeWorkflowClient,
    OperationalClientCommand, ThinClientError, ThinClientEvent, ThinClientEventKind,
    VaultClientCommand,
};

const MAX_ARGUMENT_COUNT: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 64 * 1024;

/// Stable AgentMage CLI version, independent from model or protocol versions.
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Closed output format selected before command execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliOutputFormat {
    /// Readable terminal output.
    Human,
    /// One closed JSON object or event per line.
    Json,
}

/// Supported local shell-completion targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionShell {
    /// Bourne Again Shell.
    Bash,
    /// Z shell.
    Zsh,
    /// Friendly Interactive Shell.
    Fish,
}

/// Parsed CLI action before any transport or authority boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliInvocation {
    /// Render complete local help.
    Help,
    /// Render the package version.
    Version,
    /// Render deterministic shell completion.
    Completion(CompletionShell),
    /// Start the public interactive coding client through the shared runtime.
    Code {
        /// Exact output format.
        output: CliOutputFormat,
    },
    /// Submit one exact command through a selected thin-client surface.
    Execute {
        /// Exact thin-client surface.
        surface: ClientSurface,
        /// Exact output format.
        output: CliOutputFormat,
        /// Exact closed command.
        command: ClientCommand,
    },
}

/// Parses an already shell-tokenized argument vector without ambient discovery.
pub fn parse_cli_arguments(arguments: &[String]) -> Result<CliInvocation, ThinClientError> {
    if arguments.len() > MAX_ARGUMENT_COUNT
        || arguments
            .iter()
            .try_fold(0_usize, |total, value| total.checked_add(value.len()))
            .is_none_or(|total| total > MAX_ARGUMENT_BYTES)
        || arguments.iter().any(|value| value.contains('\0'))
    {
        return Err(ThinClientError::SizeExceeded);
    }
    if arguments.is_empty() || matches!(arguments, [value] if value == "--help" || value == "-h") {
        return Ok(CliInvocation::Help);
    }
    if matches!(arguments, [value] if value == "--version" || value == "-V") {
        return Ok(CliInvocation::Version);
    }
    if let [command, shell] = arguments
        && command == "completion"
    {
        let shell = match shell.as_str() {
            "bash" => CompletionShell::Bash,
            "zsh" => CompletionShell::Zsh,
            "fish" => CompletionShell::Fish,
            _ => return Err(ThinClientError::InvalidValue),
        };
        return Ok(CliInvocation::Completion(shell));
    }

    let mut cursor = 0;
    let mut output = CliOutputFormat::Human;
    let mut surface = ClientSurface::InteractiveCli;
    let mut output_seen = false;
    let mut surface_seen = false;
    while let Some(argument) = arguments.get(cursor) {
        match argument.as_str() {
            "--json" if !output_seen => {
                output = CliOutputFormat::Json;
                output_seen = true;
                cursor += 1;
            }
            "--surface" if !surface_seen => {
                let value = arguments
                    .get(cursor + 1)
                    .ok_or(ThinClientError::InvalidValue)?;
                surface = match value.as_str() {
                    "interactive-cli" => ClientSurface::InteractiveCli,
                    "json" => ClientSurface::Json,
                    "sdk" => ClientSurface::Sdk,
                    "acp" => ClientSurface::Acp,
                    _ => return Err(ThinClientError::InvalidValue),
                };
                surface_seen = true;
                cursor += 2;
            }
            _ => break,
        }
    }
    if surface != ClientSurface::InteractiveCli && output != CliOutputFormat::Json {
        return Err(ThinClientError::InvalidValue);
    }
    if matches!(&arguments[cursor..], [command] if command == "code") {
        if surface != ClientSurface::InteractiveCli {
            return Err(ThinClientError::InvalidValue);
        }
        return Ok(CliInvocation::Code { output });
    }
    let command = parse_command(&arguments[cursor..])?;
    command.verify()?;
    Ok(CliInvocation::Execute {
        surface,
        output,
        command,
    })
}

fn parse_command(arguments: &[String]) -> Result<ClientCommand, ThinClientError> {
    let Some(first) = arguments.first().map(String::as_str) else {
        return Err(ThinClientError::InvalidValue);
    };
    match first {
        "chat" => Ok(ClientCommand::Chat {
            message: joined(&arguments[1..])?,
        }),
        "conversations" => parse_conversations(&arguments[1..]),
        "resume" => parse_exact_branch(&arguments[1..]),
        "vault" => parse_vault(&arguments[1..]),
        "knowledge" => parse_knowledge(&arguments[1..]),
        "checkpoint" if arguments.len() == 1 => operation(OperationalClientCommand::Checkpoint),
        "handoff" if arguments.len() == 1 => operation(OperationalClientCommand::Handoff),
        "audit" if arguments.len() == 1 => operation(OperationalClientCommand::Audit),
        "memory" => parse_memory(&arguments[1..]),
        "export" if arguments.len() == 2 => operation(OperationalClientCommand::Export {
            profile: arguments[1].clone(),
        }),
        "import" if arguments.len() == 2 => operation(OperationalClientCommand::Import {
            manifest_sha256: arguments[1].clone(),
        }),
        "doctor" | "diagnostics" if arguments.len() == 1 => {
            operation(OperationalClientCommand::Diagnostics)
        }
        _ => Err(ThinClientError::InvalidValue),
    }
}

fn parse_conversations(arguments: &[String]) -> Result<ClientCommand, ThinClientError> {
    let action = match arguments.first().map(String::as_str) {
        Some("list") => {
            let mut from = None;
            let mut to = None;
            let mut cursor = 1;
            while cursor < arguments.len() {
                let target = match arguments[cursor].as_str() {
                    "--from" if from.is_none() => &mut from,
                    "--to" if to.is_none() => &mut to,
                    _ => return Err(ThinClientError::InvalidValue),
                };
                *target = Some(
                    arguments
                        .get(cursor + 1)
                        .ok_or(ThinClientError::InvalidValue)?
                        .clone(),
                );
                cursor += 2;
            }
            ConversationClientCommand::List { from, to }
        }
        Some("search") => ConversationClientCommand::Search {
            query: joined(&arguments[1..])?,
        },
        Some("show") if arguments.len() == 2 => ConversationClientCommand::Show {
            conversation_id: arguments[1].clone(),
        },
        Some("open") if arguments.len() == 2 => ConversationClientCommand::Open {
            conversation_id: arguments[1].clone(),
        },
        Some("resume") if arguments.len() == 2 => ConversationClientCommand::Resume {
            conversation_id: arguments[1].clone(),
        },
        _ => return Err(ThinClientError::InvalidValue),
    };
    Ok(ClientCommand::Conversations { action })
}

fn parse_exact_branch(arguments: &[String]) -> Result<ClientCommand, ThinClientError> {
    let [conversation_id, turn_option, turn_id] = arguments else {
        return Err(ThinClientError::InvalidValue);
    };
    if turn_option != "--turn" {
        return Err(ThinClientError::InvalidValue);
    }
    Ok(ClientCommand::Conversations {
        action: ConversationClientCommand::Branch {
            conversation_id: conversation_id.clone(),
            turn_id: turn_id.clone(),
        },
    })
}

fn parse_vault(arguments: &[String]) -> Result<ClientCommand, ThinClientError> {
    let action = match arguments.first().map(String::as_str) {
        Some("search") => VaultClientCommand::Search {
            query: joined(&arguments[1..])?,
        },
        Some("note") if arguments.len() == 3 && arguments[1] == "show" => {
            VaultClientCommand::NoteShow {
                note_id: arguments[2].clone(),
            }
        }
        Some("links") if arguments.len() == 2 => VaultClientCommand::Links {
            note_id: arguments[1].clone(),
        },
        Some("backlinks") if arguments.len() == 2 => VaultClientCommand::Backlinks {
            note_id: arguments[1].clone(),
        },
        Some("tasks") if arguments.len() == 1 => VaultClientCommand::Tasks,
        _ => return Err(ThinClientError::InvalidValue),
    };
    Ok(ClientCommand::Vault { action })
}

fn parse_knowledge(arguments: &[String]) -> Result<ClientCommand, ThinClientError> {
    let [run, workflow, tail @ ..] = arguments else {
        return Err(ThinClientError::InvalidValue);
    };
    if run != "run" || tail.len() > 1 || tail.first().is_some_and(|value| value != "--semantic") {
        return Err(ThinClientError::InvalidValue);
    }
    let workflow = match workflow.as_str() {
        "daily-setup" => KnowledgeWorkflowClient::DailySetup,
        "daily-briefing" => KnowledgeWorkflowClient::DailyBriefing,
        "issue-intake" => KnowledgeWorkflowClient::IssueIntake,
        "handoff" => KnowledgeWorkflowClient::Handoff,
        "meeting-cleanup" => KnowledgeWorkflowClient::MeetingCleanup,
        "repository-learning" => KnowledgeWorkflowClient::RepositoryLearning,
        "plain-workspace-steward" => KnowledgeWorkflowClient::PlainWorkspaceSteward,
        "obsidian-vault-steward" => KnowledgeWorkflowClient::ObsidianVaultSteward,
        _ => return Err(ThinClientError::InvalidValue),
    };
    let retrieval_mode = if tail.is_empty() {
        KnowledgeRetrievalClientMode::Lexical
    } else {
        KnowledgeRetrievalClientMode::ApprovedLocalSemantic
    };
    Ok(ClientCommand::Knowledge {
        action: KnowledgeClientCommand::Run {
            workflow,
            retrieval_mode,
        },
    })
}

fn parse_memory(arguments: &[String]) -> Result<ClientCommand, ThinClientError> {
    let action = match arguments.first().map(String::as_str) {
        Some("inspect") if arguments.len() <= 2 => OperationalClientCommand::MemoryInspect {
            memory_id: arguments.get(1).cloned(),
        },
        Some("correct") if arguments.len() >= 3 => OperationalClientCommand::MemoryCorrect {
            memory_id: arguments[1].clone(),
            replacement: joined(&arguments[2..])?,
        },
        _ => return Err(ThinClientError::InvalidValue),
    };
    operation(action)
}

fn operation(action: OperationalClientCommand) -> Result<ClientCommand, ThinClientError> {
    Ok(ClientCommand::Operations { action })
}

fn joined(arguments: &[String]) -> Result<String, ThinClientError> {
    if arguments.is_empty() {
        return Err(ThinClientError::InvalidValue);
    }
    let value = arguments.join(" ");
    if value.trim().is_empty() || value.len() > MAX_ARGUMENT_BYTES {
        return Err(ThinClientError::InvalidValue);
    }
    Ok(value)
}

/// Renders one verified event as bounded human-readable terminal text.
pub fn render_human_event(event: &ThinClientEvent) -> Result<String, ThinClientError> {
    if !event.verify() {
        return Err(ThinClientError::Malformed);
    }
    let rendered = match &event.kind {
        ThinClientEventKind::Started => format!("started {}", event.stream_id),
        ThinClientEventKind::Status { status } => format!(
            "workspace={} model={} permission={} conversation={} plan={} writable={} offline={}",
            status.workspace_id,
            status.model_profile_id.as_deref().unwrap_or("none"),
            status.permission_profile_id,
            status.conversation_id.as_deref().unwrap_or("none"),
            status.plan_step_id.as_deref().unwrap_or("none"),
            status.writable_roots.join(","),
            status.offline,
        ),
        ThinClientEventKind::Content { channel, text, .. } => {
            format!("{}: {text}", channel_label(*channel))
        }
        ThinClientEventKind::ApprovalRequired {
            operation,
            preview_sha256,
            expires_at_epoch_ms,
        } => format!(
            "approval required operation={operation:?} preview={preview_sha256} expires={expires_at_epoch_ms}"
        ),
        ThinClientEventKind::Receipt {
            receipt_id,
            receipt_sha256,
            outcome,
        } => format!("receipt={receipt_id} outcome={outcome} sha256={receipt_sha256}"),
        ThinClientEventKind::Completed { final_state_sha256 } => {
            format!("completed state={final_state_sha256}")
        }
        ThinClientEventKind::Denied { code } => format!("denied code={code}"),
        ThinClientEventKind::Cancelled { code } => format!("cancelled code={code}"),
    };
    if rendered.len() > MAX_ARGUMENT_BYTES {
        return Err(ThinClientError::SizeExceeded);
    }
    Ok(rendered)
}

/// Renders one verified event as a closed JSON line.
pub fn render_json_event(event: &ThinClientEvent) -> Result<String, ThinClientError> {
    if !event.verify() {
        return Err(ThinClientError::Malformed);
    }
    let rendered = serde_json::to_string(event).map_err(|_| ThinClientError::Malformed)?;
    if rendered.len() > MAX_ARGUMENT_BYTES {
        return Err(ThinClientError::SizeExceeded);
    }
    Ok(rendered)
}

/// Renders one verified shared-runtime event as bounded human-readable terminal text.
pub fn render_runtime_event_human(event: &RuntimeEvent) -> Result<String, ThinClientError> {
    verify_runtime_event(event).map_err(|_| ThinClientError::Malformed)?;
    let detail = match &event.kind {
        RuntimeEventKind::PermissionRequested {
            operation,
            preview_sha256,
            expires_at_epoch_ms,
            ..
        } => format!(
            "permission_requested operation={operation:?} preview={preview_sha256} expires={expires_at_epoch_ms}"
        ),
        RuntimeEventKind::PermissionDecided { disposition, .. } => {
            format!("permission_decided disposition={disposition:?}")
        }
        RuntimeEventKind::ArtifactCreated {
            artifact_id,
            manifest_sha256,
        } => format!(
            "artifact_created id={} manifest={manifest_sha256}",
            artifact_id.as_str()
        ),
        RuntimeEventKind::Progress { code } => format!("progress code={code}"),
        RuntimeEventKind::RunTerminal { state, .. } => {
            format!("run_terminal state={state:?}")
        }
        _ => runtime_event_label(&event.kind).to_owned(),
    };
    bounded_render(format!("event={} {detail}", event.sequence))
}

/// Renders one verified shared-runtime event as a closed JSON line.
pub fn render_runtime_event_json(event: &RuntimeEvent) -> Result<String, ThinClientError> {
    verify_runtime_event(event).map_err(|_| ThinClientError::Malformed)?;
    bounded_render(serde_json::to_string(event).map_err(|_| ThinClientError::Malformed)?)
}

/// Renders the complete content-minimized protected approval challenge.
pub fn render_runtime_approval_human(
    challenge: &RuntimeApprovalChallenge,
) -> Result<String, ThinClientError> {
    verify_runtime_approval_challenge(challenge).map_err(|_| ThinClientError::Malformed)?;
    bounded_render(format!(
        "approval={} operation={:?} tool_call={} preview={} expires={} confirmation={}",
        challenge.approval_id.as_str(),
        challenge.operation,
        challenge.tool_call_id.as_str(),
        challenge.preview_sha256,
        challenge.expires_at_epoch_ms,
        challenge.challenge_sha256,
    ))
}

/// Renders one verified terminal runtime outcome with artifact-backed output references intact.
pub fn render_runtime_outcome_human(
    request: &RuntimeRunRequest,
    outcome: &RuntimeOutcome,
) -> Result<String, ThinClientError> {
    verify_runtime_outcome(outcome, request).map_err(|_| ThinClientError::Malformed)?;
    let output = match &outcome.output {
        None => "none".to_owned(),
        Some(RuntimeOutput::Inline { payload }) => format!(
            "inline media={} bytes={} sha256={}",
            payload.media_type,
            payload.bytes.len(),
            payload.sha256
        ),
        Some(RuntimeOutput::Artifact { reference }) => format!(
            "artifact id={} media={} bytes={} sha256={}",
            reference.artifact_id.as_str(),
            reference.media_type,
            reference.byte_size,
            reference.sha256
        ),
    };
    let answer_evidence = outcome.answer_evidence.as_ref().map_or_else(
        || "none".to_owned(),
        |answer| format!("inferred:{}", answer.assignments.len()),
    );
    bounded_render(format!(
        "state={:?} turns={} model_calls={} tool_calls={} evidence={} answer_evidence={answer_evidence} receipts={} unresolved={} output={output} outcome={}",
        outcome.state,
        outcome.turn_count,
        outcome.model_call_count,
        outcome.tool_call_count,
        outcome.evidence.len(),
        outcome.receipt_ids.len(),
        outcome.unresolved_codes.join(","),
        outcome.outcome_sha256,
    ))
}

/// Renders one verified terminal runtime outcome as closed JSON.
pub fn render_runtime_outcome_json(
    request: &RuntimeRunRequest,
    outcome: &RuntimeOutcome,
) -> Result<String, ThinClientError> {
    verify_runtime_outcome(outcome, request).map_err(|_| ThinClientError::Malformed)?;
    bounded_render(serde_json::to_string(outcome).map_err(|_| ThinClientError::Malformed)?)
}

const fn runtime_event_label(kind: &RuntimeEventKind) -> &'static str {
    match kind {
        RuntimeEventKind::RunStarted { .. } => "run_started",
        RuntimeEventKind::TurnStarted => "turn_started",
        RuntimeEventKind::TurnCompleted { .. } => "turn_completed",
        RuntimeEventKind::ModelRequested { .. } => "model_requested",
        RuntimeEventKind::ModelCompleted { .. } => "model_completed",
        RuntimeEventKind::ModelFailed { .. } => "model_failed",
        RuntimeEventKind::ToolRequested { .. } => "tool_requested",
        RuntimeEventKind::ToolStarted { .. } => "tool_started",
        RuntimeEventKind::ToolCompleted { .. } => "tool_completed",
        RuntimeEventKind::ToolFailed { .. } => "tool_failed",
        RuntimeEventKind::PermissionRequested { .. } => "permission_requested",
        RuntimeEventKind::PermissionDecided { .. } => "permission_decided",
        RuntimeEventKind::FileObserved { .. } => "file_observed",
        RuntimeEventKind::FileModified { .. } => "file_modified",
        RuntimeEventKind::ArtifactCreated { .. } => "artifact_created",
        RuntimeEventKind::SourceAdmitted { .. } => "source_admitted",
        RuntimeEventKind::ExtractionStarted { .. } => "extraction_started",
        RuntimeEventKind::ExtractionCompleted { .. } => "extraction_completed",
        RuntimeEventKind::ExtractionBlocked { .. } => "extraction_blocked",
        RuntimeEventKind::SectionIndexed { .. } => "section_indexed",
        RuntimeEventKind::ContextDisposition { .. } => "context_disposition",
        RuntimeEventKind::PreflightObserved { .. } => "preflight_observed",
        RuntimeEventKind::AttemptStarted { .. } => "attempt_started",
        RuntimeEventKind::AttemptEnded { .. } => "attempt_ended",
        RuntimeEventKind::VerificationObserved { .. } => "verification_observed",
        RuntimeEventKind::RetryDecided { .. } => "retry_decided",
        RuntimeEventKind::RecoveryDecided { .. } => "recovery_decided",
        RuntimeEventKind::TerminalDiagnostic { .. } => "terminal_diagnostic",
        RuntimeEventKind::CheckpointCommitted { .. } => "checkpoint_committed",
        RuntimeEventKind::CancellationRequested { .. } => "cancellation_requested",
        RuntimeEventKind::CancellationObserved { .. } => "cancellation_observed",
        RuntimeEventKind::Progress { .. } => "progress",
        RuntimeEventKind::Metric { .. } => "metric",
        RuntimeEventKind::RunTerminal { .. } => "run_terminal",
    }
}

fn bounded_render(rendered: String) -> Result<String, ThinClientError> {
    if rendered.len() > MAX_ARGUMENT_BYTES {
        Err(ThinClientError::SizeExceeded)
    } else {
        Ok(rendered)
    }
}

const fn channel_label(channel: ClientContentChannel) -> &'static str {
    match channel {
        ClientContentChannel::Content => "content",
        ClientContentChannel::Progress => "progress",
        ClientContentChannel::Preview => "preview",
        ClientContentChannel::Diff => "diff",
        ClientContentChannel::Citation => "citation",
        ClientContentChannel::Error => "error",
    }
}

/// Returns the complete deterministic command reference.
#[must_use]
pub const fn command_help() -> &'static str {
    "AgentMage local CLI\n\
Usage: agentmage [--json] [--surface interactive-cli|json|sdk|acp] COMMAND\n\
\n\
Commands:\n\
  code\n\
  chat MESSAGE\n\
  conversations list [--from YYYY-MM-DD] [--to YYYY-MM-DD]\n\
  conversations search QUERY\n\
  conversations show|open|resume ID\n\
  resume ID --turn TURN_ID\n\
  vault search QUERY\n\
  vault note show ID\n\
  vault links|backlinks ID\n\
  vault tasks\n\
  knowledge run WORKFLOW [--semantic]\n\
  checkpoint | handoff | audit | doctor | diagnostics\n\
  memory inspect [ID]\n\
  memory correct ID REPLACEMENT\n\
  export PROFILE\n\
  import MANIFEST_SHA256\n\
  completion bash|zsh|fish\n\
\n\
Headless surfaces require an exact predeclared, bounded, unexpired grant.\n"
}

/// Returns deterministic completion source without executing a shell or external program.
#[must_use]
pub const fn shell_completion(shell: CompletionShell) -> &'static str {
    match shell {
        CompletionShell::Bash => {
            "complete -W 'code chat conversations resume vault knowledge checkpoint handoff audit memory export import doctor diagnostics completion' agentmage\n"
        }
        CompletionShell::Zsh => {
            "compdef '_arguments 1:command:(code chat conversations resume vault knowledge checkpoint handoff audit memory export import doctor diagnostics completion)' agentmage\n"
        }
        CompletionShell::Fish => {
            "complete -c agentmage -f -a 'code chat conversations resume vault knowledge checkpoint handoff audit memory export import doctor diagnostics completion'\n"
        }
    }
}

/// Renders a content-free terminal failure in the selected format.
pub fn render_cli_error(error: ThinClientError, format: CliOutputFormat) -> String {
    match format {
        CliOutputFormat::Human => error.code().to_owned(),
        CliOutputFormat::Json => serde_json::json!({
            "schema_version": 1,
            "kind": "error",
            "code": error.code(),
            "exit_code": error.exit_code().process_code(),
        })
        .to_string(),
    }
}

/// Returns the stable unavailable result used before authenticated transport composition.
#[must_use]
pub const fn unavailable_exit_code() -> ClientExitCode {
    ClientExitCode::ServiceUnavailable
}

#[cfg(test)]
mod tests {
    use crate::headless::{ClientStatusSnapshot, THIN_CLIENT_PROTOCOL_VERSION};

    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn every_required_command_family_parses_to_one_closed_command() {
        let mut cases = vec![
            strings(&["chat", "hello", "world"]),
            strings(&[
                "conversations",
                "list",
                "--from",
                "2026-01-01",
                "--to",
                "2026-12-31",
            ]),
            strings(&["conversations", "search", "exact", "query"]),
            strings(&["conversations", "show", "conversation-01"]),
            strings(&["conversations", "open", "conversation-01"]),
            strings(&["conversations", "resume", "conversation-01"]),
            strings(&["resume", "conversation-01", "--turn", "turn-01"]),
            strings(&["vault", "search", "local", "notes"]),
            strings(&["vault", "note", "show", "note-01"]),
            strings(&["vault", "links", "note-01"]),
            strings(&["vault", "backlinks", "note-01"]),
            strings(&["vault", "tasks"]),
            strings(&["knowledge", "run", "daily-setup"]),
            strings(&["knowledge", "run", "obsidian-vault-steward", "--semantic"]),
            strings(&["checkpoint"]),
            strings(&["handoff"]),
            strings(&["audit"]),
            strings(&["memory", "inspect"]),
            strings(&["memory", "inspect", "memory-01"]),
            strings(&["memory", "correct", "memory-01", "corrected", "text"]),
            strings(&["export", "audit-jsonl"]),
            strings(&["import", &"a".repeat(64)]),
            strings(&["doctor"]),
            strings(&["diagnostics"]),
        ];
        for workflow in [
            "daily-setup",
            "daily-briefing",
            "issue-intake",
            "handoff",
            "meeting-cleanup",
            "repository-learning",
            "plain-workspace-steward",
            "obsidian-vault-steward",
        ] {
            cases.push(strings(&["knowledge", "run", workflow]));
            cases.push(strings(&["knowledge", "run", workflow, "--semantic"]));
        }
        for arguments in cases {
            assert!(
                matches!(
                    parse_cli_arguments(&arguments),
                    Ok(CliInvocation::Execute { .. })
                ),
                "{arguments:?}"
            );
        }
    }

    #[test]
    fn global_surface_format_help_version_and_completion_are_stable() {
        assert_eq!(parse_cli_arguments(&[]), Ok(CliInvocation::Help));
        assert_eq!(
            parse_cli_arguments(&strings(&["--version"])),
            Ok(CliInvocation::Version)
        );
        assert_eq!(
            parse_cli_arguments(&strings(&["completion", "bash"])),
            Ok(CliInvocation::Completion(CompletionShell::Bash))
        );
        assert_eq!(
            parse_cli_arguments(&strings(&["code"])),
            Ok(CliInvocation::Code {
                output: CliOutputFormat::Human,
            })
        );
        assert_eq!(
            parse_cli_arguments(&strings(&["--json", "code"])),
            Ok(CliInvocation::Code {
                output: CliOutputFormat::Json,
            })
        );
        let parsed =
            parse_cli_arguments(&strings(&["--json", "--surface", "acp", "vault", "tasks"]));
        assert!(matches!(
            parsed,
            Ok(CliInvocation::Execute {
                surface: ClientSurface::Acp,
                output: CliOutputFormat::Json,
                ..
            })
        ));
        assert!(!command_help().contains("http"));
        assert!(!shell_completion(CompletionShell::Fish).contains("exec"));
        assert!(parse_cli_arguments(&strings(&["--surface", "json", "code"])).is_err());
    }

    #[test]
    fn malformed_missing_duplicate_oversized_and_unsafe_arguments_fail() {
        let cases = [
            strings(&["unknown"]),
            strings(&["chat"]),
            strings(&["conversations", "list", "--from"]),
            strings(&[
                "conversations",
                "list",
                "--from",
                "2026-01-01",
                "--from",
                "2026-02-01",
            ]),
            strings(&["resume", "conversation-01", "--wrong", "turn-01"]),
            strings(&["vault", "note", "note-01"]),
            strings(&["knowledge", "run", "unknown"]),
            strings(&["knowledge", "run", "daily-setup", "--unknown"]),
            strings(&["memory", "correct", "memory-01"]),
            strings(&["--surface", "acp", "vault", "tasks"]),
            strings(&["completion", "powershell"]),
            strings(&["conversations", "list", "--from", "2026-99-99"]),
            strings(&["resume", "../conversation", "--turn", "turn-01"]),
            strings(&["import", "not-a-digest"]),
        ];
        for arguments in cases {
            assert!(parse_cli_arguments(&arguments).is_err(), "{arguments:?}");
        }
        assert_eq!(
            parse_cli_arguments(&["x".repeat(MAX_ARGUMENT_BYTES + 1)]),
            Err(ThinClientError::SizeExceeded)
        );
    }

    #[test]
    fn verified_events_render_status_content_receipts_errors_and_json() {
        let status = ClientStatusSnapshot {
            workspace_id: "workspace-01".to_owned(),
            model_profile_id: Some("model-01".to_owned()),
            permission_profile_id: "permission-01".to_owned(),
            conversation_id: Some("conversation-01".to_owned()),
            plan_step_id: Some("step-01".to_owned()),
            writable_roots: vec!["workspace".to_owned()],
            offline: true,
            status_sha256: "0".repeat(64),
        }
        .seal()
        .expect("status");
        let event = ThinClientEvent {
            schema_version: THIN_CLIENT_PROTOCOL_VERSION,
            request_id: "request-01".to_owned(),
            stream_id: "stream-01".to_owned(),
            sequence: 1,
            kernel_request_sha256: "a".repeat(64),
            policy_sha256: "b".repeat(64),
            cumulative_output_bytes: 100,
            kind: ThinClientEventKind::Status { status },
            previous_event_sha256: "c".repeat(64),
            event_sha256: "0".repeat(64),
        }
        .seal()
        .expect("event");
        let human = render_human_event(&event).expect("human");
        assert!(human.contains("workspace=workspace-01"));
        assert!(human.contains("offline=true"));
        let json = render_json_event(&event).expect("JSON");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).expect("value")["schema_version"],
            THIN_CLIENT_PROTOCOL_VERSION
        );
        let denied = render_cli_error(ThinClientError::AuthorityDenied, CliOutputFormat::Json);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&denied).expect("error")["exit_code"],
            ClientExitCode::AuthorityDenied.process_code()
        );
    }
}
