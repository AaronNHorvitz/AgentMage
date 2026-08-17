//! Exact command templates, previews, execution permits, and terminal receipts.

use std::collections::BTreeMap;
use std::fmt;

use agentmage_kernel_contracts::{
    GrantOperation, HeldWorkspaceRoot, OperationOutcome, StateChange,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::authority_transaction::{EffectAuthorization, EffectDriver, EffectLaunch, EffectResult};
use crate::propagation::CancellationToken;

const COMMAND_SCHEMA_VERSION: u16 = 1;
const MAX_COMMANDS: usize = 64;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_EXECUTABLE_BYTES: usize = 4_096;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 4_096;
const MAX_TOTAL_ARGUMENT_BYTES: usize = 64 * 1024;
const MAX_ENVIRONMENT_VARIABLES: usize = 8;
const MAX_ENVIRONMENT_VALUE_BYTES: usize = 256;
const MAX_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TIMEOUT_MS: u64 = 300_000;
const SAFE_ENVIRONMENT_NAMES: [&str; 4] = ["LANG", "LC_ALL", "NO_COLOR", "TZ"];
const PROHIBITED_EXECUTABLE_NAMES: [&str; 18] = [
    "bash",
    "busybox",
    "cmd",
    "csh",
    "dash",
    "env",
    "fish",
    "ksh",
    "node",
    "perl",
    "php",
    "powershell",
    "pwsh",
    "python",
    "python3",
    "ruby",
    "sh",
    "zsh",
];
const PROHIBITED_ARGUMENT_PREFIXES: [&str; 12] = [
    "--config",
    "--editor",
    "--eval",
    "--exec-path",
    "--pager",
    "--plugin",
    "--rc",
    "--receive-pack",
    "--require",
    "--upload-pack",
    "-c",
    "-e",
];

/// Stable reason a command contract or attempt cannot advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    /// A command template field is malformed, ambiguous, or outside its bound.
    InvalidSpec,
    /// The exact template identity and version already exist.
    DuplicateTemplate,
    /// The exact template identity, version, and digest are not registered.
    TemplateNotRegistered,
    /// The request bytes or digest do not equal the prepared command.
    RequestMismatch,
    /// The consumed authority does not bind this command and target.
    AuthorityMismatch,
    /// The platform returned a malformed or over-limit result.
    InvalidPlatformResult,
    /// A terminal command receipt could not be sealed.
    ReceiptFailure,
}

impl CommandError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidSpec => "command.spec.invalid",
            Self::DuplicateTemplate => "command.registry.duplicate",
            Self::TemplateNotRegistered => "command.registry.not_registered",
            Self::RequestMismatch => "command.request.mismatch",
            Self::AuthorityMismatch => "command.authority.mismatch",
            Self::InvalidPlatformResult => "command.result.invalid",
            Self::ReceiptFailure => "command.receipt.failed",
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for CommandError {}

/// Review risk for one exact command template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandRisk {
    /// Deterministic observation with no permitted state change.
    Low,
    /// Bounded observation requiring heightened review.
    Moderate,
}

/// Closed working-directory selection for the initial runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandWorkingDirectory {
    /// A fresh empty in-memory directory owned by this one attempt.
    EmptyScratch,
    /// The exact descriptor-held AgentMage-owned worktree, mounted read-only for this attempt.
    OwnedWorktree,
}

/// Exact process and output ceilings for one command template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandBounds {
    /// Maximum wall-clock process lifetime.
    pub timeout_ms: u64,
    /// Maximum retained standard-output bytes.
    pub stdout_bytes: u64,
    /// Maximum retained standard-error bytes.
    pub stderr_bytes: u64,
    /// Maximum cgroup memory.
    pub memory_bytes: u64,
    /// Maximum cgroup process and thread count.
    pub task_count: u32,
    /// Maximum cgroup CPU percentage.
    pub cpu_percent: u16,
}

impl CommandBounds {
    /// Constructs validated command bounds.
    pub fn new(
        timeout_ms: u64,
        stdout_bytes: u64,
        stderr_bytes: u64,
        memory_bytes: u64,
        task_count: u32,
        cpu_percent: u16,
    ) -> Result<Self, CommandError> {
        let candidate = Self {
            timeout_ms,
            stdout_bytes,
            stderr_bytes,
            memory_bytes,
            task_count,
            cpu_percent,
        };
        validate_bounds(&candidate)?;
        Ok(candidate)
    }
}

/// One immutable complete direct-execution template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    /// Closed command schema version.
    pub schema_version: u16,
    /// Stable command template identity.
    pub template_id: String,
    /// Immutable semantic version for this exact template.
    pub template_version: String,
    /// Exact absolute executable path verified by the platform adapter.
    pub executable: String,
    /// Exact executable bytes expected by the platform adapter.
    pub executable_sha256: String,
    /// Literal argument vector; element zero is never caller controlled.
    pub arguments: Vec<String>,
    /// Closed working-directory selection.
    pub working_directory: CommandWorkingDirectory,
    /// Complete inherited-environment replacement.
    pub environment: BTreeMap<String, String>,
    /// Human review risk.
    pub risk: CommandRisk,
    /// Must remain true for every executable template.
    pub grant_required: bool,
    /// Must remain false until a separately reviewed interactive adapter exists.
    pub interactive: bool,
    /// Must remain false for the initial command pack.
    pub network: bool,
    /// Must remain false; ambient process environment is never inherited.
    pub inherit_environment: bool,
    /// Exact resource ceilings.
    pub bounds: CommandBounds,
    /// Canonical identity of every preceding field with this field zeroed.
    pub spec_sha256: String,
}

impl CommandSpec {
    /// Validates and seals one complete command template.
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        template_id: impl Into<String>,
        template_version: impl Into<String>,
        executable: impl Into<String>,
        executable_sha256: impl Into<String>,
        arguments: Vec<String>,
        working_directory: CommandWorkingDirectory,
        environment: BTreeMap<String, String>,
        risk: CommandRisk,
        bounds: CommandBounds,
    ) -> Result<Self, CommandError> {
        let mut candidate = Self {
            schema_version: COMMAND_SCHEMA_VERSION,
            template_id: template_id.into(),
            template_version: template_version.into(),
            executable: executable.into(),
            executable_sha256: executable_sha256.into(),
            arguments,
            working_directory,
            environment,
            risk,
            grant_required: true,
            interactive: false,
            network: false,
            inherit_environment: false,
            bounds,
            spec_sha256: "0".repeat(64),
        };
        validate_spec_shape(&candidate)?;
        candidate.spec_sha256 = canonical_sha256(&candidate)?;
        Ok(candidate)
    }

    /// Revalidates the closed shape and canonical digest.
    pub fn verify(&self) -> Result<(), CommandError> {
        validate_spec_shape(self)?;
        let mut candidate = self.clone();
        candidate.spec_sha256 = "0".repeat(64);
        if canonical_sha256(&candidate)? != self.spec_sha256 {
            return Err(CommandError::InvalidSpec);
        }
        Ok(())
    }
}

/// Exact-version, exact-digest registry of non-authoritative command templates.
#[derive(Clone, Debug)]
pub struct CommandRegistry {
    commands: BTreeMap<(String, String), CommandSpec>,
}

impl CommandRegistry {
    /// Validates and freezes one non-empty command registry.
    pub fn build(commands: Vec<CommandSpec>) -> Result<Self, CommandError> {
        if commands.is_empty() || commands.len() > MAX_COMMANDS {
            return Err(CommandError::InvalidSpec);
        }
        let mut retained = BTreeMap::new();
        for command in commands {
            command.verify()?;
            let key = (
                command.template_id.clone(),
                command.template_version.clone(),
            );
            if retained.insert(key, command).is_some() {
                return Err(CommandError::DuplicateTemplate);
            }
        }
        Ok(Self { commands: retained })
    }

    /// Lists immutable templates in stable identity and version order.
    #[must_use]
    pub fn commands(&self) -> Vec<&CommandSpec> {
        self.commands.values().collect()
    }

    fn resolve(&self, request: &CommandRequest) -> Result<&CommandSpec, CommandError> {
        validate_identifier(&request.command_attempt_id)?;
        validate_identifier(&request.template_id)?;
        validate_version(&request.template_version)?;
        validate_digest(&request.spec_sha256)?;
        let command = self
            .commands
            .get(&(
                request.template_id.clone(),
                request.template_version.clone(),
            ))
            .ok_or(CommandError::TemplateNotRegistered)?;
        if command.spec_sha256 != request.spec_sha256 {
            return Err(CommandError::TemplateNotRegistered);
        }
        command.verify()?;
        Ok(command)
    }
}

/// Minimal exact request selecting one already frozen command template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRequest {
    /// Closed command schema version.
    pub schema_version: u16,
    /// Non-replayable attempt identity.
    pub command_attempt_id: String,
    /// Exact registered template identity.
    pub template_id: String,
    /// Exact registered template version.
    pub template_version: String,
    /// Exact registered template digest.
    pub spec_sha256: String,
}

impl CommandRequest {
    /// Selects one exact registered command template.
    #[must_use]
    pub fn new(command_attempt_id: impl Into<String>, command: &CommandSpec) -> Self {
        Self {
            schema_version: COMMAND_SCHEMA_VERSION,
            command_attempt_id: command_attempt_id.into(),
            template_id: command.template_id.clone(),
            template_version: command.template_version.clone(),
            spec_sha256: command.spec_sha256.clone(),
        }
    }
}

/// Complete deterministic command display bound before authority is requested.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandPreview {
    /// Closed command schema version.
    pub schema_version: u16,
    /// Non-replayable attempt identity.
    pub command_attempt_id: String,
    /// Exact command template identity.
    pub template_id: String,
    /// Exact template version.
    pub template_version: String,
    /// Exact executable path, displayed as a single field rather than shell text.
    pub executable: String,
    /// Exact verified executable digest.
    pub executable_sha256: String,
    /// Ordered literal argument vector.
    pub arguments: Vec<String>,
    /// Closed working directory.
    pub working_directory: CommandWorkingDirectory,
    /// Complete replacement environment.
    pub environment: BTreeMap<String, String>,
    /// Review risk.
    pub risk: CommandRisk,
    /// Whether one separately consumed exact grant is mandatory.
    pub grant_required: bool,
    /// Whether an interactive process is requested.
    pub interactive: bool,
    /// Whether network access is requested.
    pub network: bool,
    /// Whether ambient process environment inheritance is requested.
    pub inherit_environment: bool,
    /// Exact process and output ceilings.
    pub bounds: CommandBounds,
    /// Canonical identity of this preview with this field zeroed.
    pub preview_sha256: String,
}

/// Authority-free prepared bytes and display for one exact command request.
#[derive(Clone, Debug)]
pub struct PreparedCommand {
    request: CommandRequest,
    request_bytes: Vec<u8>,
    request_sha256: String,
    command: CommandSpec,
    preview: CommandPreview,
}

impl PreparedCommand {
    /// Returns exact canonical request bytes for a tool-call payload.
    #[must_use]
    pub fn request_bytes(&self) -> &[u8] {
        &self.request_bytes
    }

    /// Returns the exact request digest a tool call and grant must bind.
    #[must_use]
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    /// Returns the complete deterministic preview.
    #[must_use]
    pub const fn preview(&self) -> &CommandPreview {
        &self.preview
    }

    /// Returns the exact frozen command template.
    #[must_use]
    pub const fn command(&self) -> &CommandSpec {
        &self.command
    }
}

/// Resolves and prepares one exact command without granting or executing it.
pub fn prepare_command(
    registry: &CommandRegistry,
    request: CommandRequest,
) -> Result<PreparedCommand, CommandError> {
    if request.schema_version != COMMAND_SCHEMA_VERSION {
        return Err(CommandError::RequestMismatch);
    }
    let command = registry.resolve(&request)?.clone();
    let request_bytes = serde_json::to_vec(&request).map_err(|_| CommandError::RequestMismatch)?;
    let request_sha256 = sha256_hex(&request_bytes);
    let mut preview = CommandPreview {
        schema_version: COMMAND_SCHEMA_VERSION,
        command_attempt_id: request.command_attempt_id.clone(),
        template_id: command.template_id.clone(),
        template_version: command.template_version.clone(),
        executable: command.executable.clone(),
        executable_sha256: command.executable_sha256.clone(),
        arguments: command.arguments.clone(),
        working_directory: command.working_directory,
        environment: command.environment.clone(),
        risk: command.risk,
        grant_required: command.grant_required,
        interactive: command.interactive,
        network: command.network,
        inherit_environment: command.inherit_environment,
        bounds: command.bounds,
        preview_sha256: "0".repeat(64),
    };
    preview.preview_sha256 = canonical_sha256(&preview)?;
    Ok(PreparedCommand {
        request,
        request_bytes,
        request_sha256,
        command,
        preview,
    })
}

/// Terminal reason reported by a trusted platform command executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandTermination {
    /// Process exited and a code was observed.
    Exited,
    /// User or parent cancellation terminated the process tree.
    Cancelled,
    /// The exact wall-clock deadline terminated the process tree.
    TimedOut,
    /// Output crossed one configured byte ceiling.
    OutputLimit,
    /// Isolation or launch failed before a command result was observed.
    LaunchFailed,
}

/// Bounded platform observation for one command attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPlatformResult {
    /// Terminal reason.
    pub termination: CommandTermination,
    /// Observed process exit code when available.
    pub exit_code: Option<i32>,
    /// Observed terminating signal when available.
    pub signal: Option<i32>,
    /// Retained standard output.
    pub stdout: Vec<u8>,
    /// Digest of complete standard output, including bytes beyond retention.
    pub stdout_sha256: String,
    /// Total standard-output bytes observed before closure.
    pub stdout_total_bytes: u64,
    /// Retained standard error.
    pub stderr: Vec<u8>,
    /// Digest of complete standard error, including bytes beyond retention.
    pub stderr_sha256: String,
    /// Total standard-error bytes observed before closure.
    pub stderr_total_bytes: u64,
    /// Monotonic elapsed duration.
    pub elapsed_ms: u64,
    /// Whether platform process-tree cleanup was verified.
    pub descendants_terminated: bool,
    /// Stable content-free platform detail code.
    pub platform_code: String,
}

/// Terminal command receipt supplementing the kernel authority receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandReceipt {
    /// Closed command schema version.
    pub schema_version: u16,
    /// Exact authority transaction.
    pub authority_transaction_id: String,
    /// Exact operation attempt.
    pub operation_attempt_id: String,
    /// Exact command attempt.
    pub command_attempt_id: String,
    /// Exact template identity.
    pub template_id: String,
    /// Exact template version.
    pub template_version: String,
    /// Exact command template digest.
    pub spec_sha256: String,
    /// Exact request digest.
    pub request_sha256: String,
    /// Exact approved preview digest.
    pub preview_sha256: String,
    /// Terminal process reason.
    pub termination: CommandTermination,
    /// Deterministically classified operation outcome.
    pub outcome: OperationOutcome,
    /// Observed exit code.
    pub exit_code: Option<i32>,
    /// Observed signal.
    pub signal: Option<i32>,
    /// Digest of complete standard output.
    pub stdout_sha256: String,
    /// Total standard-output bytes.
    pub stdout_total_bytes: u64,
    /// Retained standard-output bytes.
    pub stdout_retained_bytes: u64,
    /// Whether standard output was truncated.
    pub stdout_truncated: bool,
    /// Digest of complete standard error.
    pub stderr_sha256: String,
    /// Total standard-error bytes.
    pub stderr_total_bytes: u64,
    /// Retained standard-error bytes.
    pub stderr_retained_bytes: u64,
    /// Whether standard error was truncated.
    pub stderr_truncated: bool,
    /// Observed monotonic elapsed duration.
    pub elapsed_ms: u64,
    /// Whether the platform verified descendant cleanup.
    pub descendants_terminated: bool,
    /// Stable content-free platform detail code.
    pub platform_code: String,
    /// Canonical digest of this receipt with this field zeroed.
    pub receipt_sha256: String,
}

/// Verifies a terminal receipt against the exact prepared command and closed outcome rules.
#[must_use]
pub fn verify_command_receipt(prepared: &PreparedCommand, receipt: &CommandReceipt) -> bool {
    let expected_outcome = match receipt.termination {
        CommandTermination::Exited if receipt.exit_code == Some(0) => OperationOutcome::Succeeded,
        CommandTermination::Exited
        | CommandTermination::OutputLimit
        | CommandTermination::LaunchFailed => OperationOutcome::Failed,
        CommandTermination::Cancelled => OperationOutcome::Cancelled,
        CommandTermination::TimedOut => OperationOutcome::TimedOut,
    };
    let mut canonical = receipt.clone();
    canonical.receipt_sha256 = "0".repeat(64);
    receipt.schema_version == COMMAND_SCHEMA_VERSION
        && receipt.command_attempt_id == prepared.request.command_attempt_id
        && receipt.template_id == prepared.command.template_id
        && receipt.template_version == prepared.command.template_version
        && receipt.spec_sha256 == prepared.command.spec_sha256
        && receipt.request_sha256 == prepared.request_sha256
        && receipt.preview_sha256 == prepared.preview.preview_sha256
        && receipt.outcome == expected_outcome
        && (receipt.termination == CommandTermination::Exited) == receipt.exit_code.is_some()
        && receipt.stdout_retained_bytes <= receipt.stdout_total_bytes
        && receipt.stderr_retained_bytes <= receipt.stderr_total_bytes
        && receipt.stdout_retained_bytes <= prepared.command.bounds.stdout_bytes
        && receipt.stderr_retained_bytes <= prepared.command.bounds.stderr_bytes
        && receipt.stdout_truncated == (receipt.stdout_total_bytes > receipt.stdout_retained_bytes)
        && receipt.stderr_truncated == (receipt.stderr_total_bytes > receipt.stderr_retained_bytes)
        && receipt.elapsed_ms <= prepared.command.bounds.timeout_ms.saturating_add(10_000)
        && (!matches!(
            receipt.termination,
            CommandTermination::Cancelled | CommandTermination::TimedOut
        ) || receipt.descendants_terminated)
        && valid_identifier(&receipt.authority_transaction_id)
        && valid_identifier(&receipt.operation_attempt_id)
        && valid_identifier(&receipt.platform_code)
        && validate_digest(&receipt.stdout_sha256).is_ok()
        && validate_digest(&receipt.stderr_sha256).is_ok()
        && validate_digest(&receipt.receipt_sha256).is_ok()
        && canonical_sha256(&canonical).as_ref() == Ok(&receipt.receipt_sha256)
}

/// Nonforgeable platform launch permit created only after exact authority validation.
pub struct CommandLaunchPermit<'command> {
    command: &'command CommandSpec,
}

impl CommandLaunchPermit<'_> {
    /// Returns the exact registered command selected by consumed authority.
    #[must_use]
    pub const fn command(&self) -> &CommandSpec {
        self.command
    }
}

impl fmt::Debug for CommandLaunchPermit<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandLaunchPermit")
            .field("template_id", &self.command.template_id)
            .field("template_version", &self.command.template_version)
            .finish_non_exhaustive()
    }
}

/// Trusted platform executor whose launch requires a kernel-created permit.
pub trait BoundedCommandExecutor {
    /// Platform-owned held workspace-root type accepted by this executor.
    type WorkingDirectory: HeldWorkspaceRoot;

    /// Executes one exact non-interactive command inside the platform sandbox.
    fn execute(
        &mut self,
        permit: CommandLaunchPermit<'_>,
        working_directory: &Self::WorkingDirectory,
        cancellation: &CancellationToken,
    ) -> CommandPlatformResult;
}

/// Inert command driver crossing the effect boundary only with consumed authority.
pub struct CommandEffectDriver<E, H> {
    executor: E,
    held_working_directory: H,
    prepared: PreparedCommand,
    cancellation: CancellationToken,
    receipt: Option<CommandReceipt>,
    error: Option<CommandError>,
}

impl<E, H> CommandEffectDriver<E, H> {
    /// Creates an inert driver; construction does not execute or grant authority.
    #[must_use]
    pub const fn new(
        executor: E,
        held_working_directory: H,
        prepared: PreparedCommand,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            executor,
            held_working_directory,
            prepared,
            cancellation,
            receipt: None,
            error: None,
        }
    }

    /// Takes the command-specific terminal receipt after mediated execution.
    pub fn take_receipt(&mut self) -> Option<CommandReceipt> {
        self.receipt.take()
    }

    /// Takes a content-free command-boundary error.
    pub fn take_error(&mut self) -> Option<CommandError> {
        self.error.take()
    }

    /// Returns the executor after this attempt closes.
    #[must_use]
    pub fn into_executor(self) -> E {
        self.executor
    }
}

impl<E, H> fmt::Debug for CommandEffectDriver<E, H> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommandEffectDriver")
            .field("template_id", &self.prepared.command.template_id)
            .field("has_receipt", &self.receipt.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl<E, H> EffectDriver for CommandEffectDriver<E, H>
where
    E: BoundedCommandExecutor<WorkingDirectory = H>,
    H: HeldWorkspaceRoot,
{
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        let call = authorization.call();
        if authorization.operation().operation() != GrantOperation::CommandExecute
            || !authorization.authorizes_held_workspace_root(&self.held_working_directory)
            || call.arguments.sha256 != self.prepared.request_sha256
            || call.arguments.bytes != self.prepared.request_bytes
            || sha256_hex(&call.arguments.bytes) != call.arguments.sha256
        {
            self.error = Some(CommandError::AuthorityMismatch);
            return EffectLaunch::failed();
        }

        let platform = if self.cancellation.is_cancelled() {
            CommandPlatformResult {
                termination: CommandTermination::Cancelled,
                exit_code: None,
                signal: None,
                stdout: Vec::new(),
                stdout_sha256: sha256_hex(&[]),
                stdout_total_bytes: 0,
                stderr: Vec::new(),
                stderr_sha256: sha256_hex(&[]),
                stderr_total_bytes: 0,
                elapsed_ms: 0,
                descendants_terminated: true,
                platform_code: "command.cancelled.before_launch".to_owned(),
            }
        } else {
            self.executor.execute(
                CommandLaunchPermit {
                    command: &self.prepared.command,
                },
                &self.held_working_directory,
                &self.cancellation,
            )
        };

        match seal_receipt(&authorization, &self.prepared, platform) {
            Ok(receipt) => {
                let effect = EffectResult::from_redacted_material(
                    receipt.outcome,
                    receipt.receipt_sha256.as_bytes(),
                    StateChange::NotChanged,
                );
                self.receipt = Some(receipt);
                EffectLaunch::completed(effect)
            }
            Err(error) => {
                self.error = Some(error);
                EffectLaunch::failed()
            }
        }
    }
}

fn seal_receipt(
    authorization: &EffectAuthorization<'_>,
    prepared: &PreparedCommand,
    platform: CommandPlatformResult,
) -> Result<CommandReceipt, CommandError> {
    validate_platform_result(&prepared.command.bounds, &platform)?;
    let outcome = match platform.termination {
        CommandTermination::Exited if platform.exit_code == Some(0) => OperationOutcome::Succeeded,
        CommandTermination::Exited
        | CommandTermination::OutputLimit
        | CommandTermination::LaunchFailed => OperationOutcome::Failed,
        CommandTermination::Cancelled => OperationOutcome::Cancelled,
        CommandTermination::TimedOut => OperationOutcome::TimedOut,
    };
    let mut receipt = CommandReceipt {
        schema_version: COMMAND_SCHEMA_VERSION,
        authority_transaction_id: authorization.transaction_id().as_str().to_owned(),
        operation_attempt_id: authorization.attempt_id().as_str().to_owned(),
        command_attempt_id: prepared.request.command_attempt_id.clone(),
        template_id: prepared.command.template_id.clone(),
        template_version: prepared.command.template_version.clone(),
        spec_sha256: prepared.command.spec_sha256.clone(),
        request_sha256: prepared.request_sha256.clone(),
        preview_sha256: prepared.preview.preview_sha256.clone(),
        termination: platform.termination,
        outcome,
        exit_code: platform.exit_code,
        signal: platform.signal,
        stdout_sha256: platform.stdout_sha256,
        stdout_total_bytes: platform.stdout_total_bytes,
        stdout_retained_bytes: platform.stdout.len() as u64,
        stdout_truncated: platform.stdout_total_bytes > platform.stdout.len() as u64,
        stderr_sha256: platform.stderr_sha256,
        stderr_total_bytes: platform.stderr_total_bytes,
        stderr_retained_bytes: platform.stderr.len() as u64,
        stderr_truncated: platform.stderr_total_bytes > platform.stderr.len() as u64,
        elapsed_ms: platform.elapsed_ms,
        descendants_terminated: platform.descendants_terminated,
        platform_code: platform.platform_code,
        receipt_sha256: "0".repeat(64),
    };
    receipt.receipt_sha256 = canonical_sha256(&receipt)?;
    Ok(receipt)
}

fn validate_spec_shape(spec: &CommandSpec) -> Result<(), CommandError> {
    if spec.schema_version != COMMAND_SCHEMA_VERSION
        || !spec.grant_required
        || spec.interactive
        || spec.network
        || spec.inherit_environment
        || !valid_identifier(&spec.template_id)
        || validate_version(&spec.template_version).is_err()
        || !valid_executable(&spec.executable)
        || validate_digest(&spec.executable_sha256).is_err()
        || spec.arguments.len() > MAX_ARGUMENTS
        || spec.environment.len() > MAX_ENVIRONMENT_VARIABLES
        || validate_bounds(&spec.bounds).is_err()
        || validate_digest(&spec.spec_sha256).is_err()
    {
        return Err(CommandError::InvalidSpec);
    }
    let total = spec
        .arguments
        .iter()
        .try_fold(0_usize, |sum, argument| sum.checked_add(argument.len()));
    if total.is_none_or(|total| total > MAX_TOTAL_ARGUMENT_BYTES)
        || spec
            .arguments
            .iter()
            .any(|argument| !valid_argument(argument))
        || spec.environment.iter().any(|(name, value)| {
            !SAFE_ENVIRONMENT_NAMES.contains(&name.as_str()) || !valid_environment_value(value)
        })
    {
        return Err(CommandError::InvalidSpec);
    }
    Ok(())
}

fn validate_bounds(bounds: &CommandBounds) -> Result<(), CommandError> {
    if !(1..=MAX_TIMEOUT_MS).contains(&bounds.timeout_ms)
        || !(1..=MAX_OUTPUT_BYTES).contains(&bounds.stdout_bytes)
        || !(1..=MAX_OUTPUT_BYTES).contains(&bounds.stderr_bytes)
        || !(16 * 1024 * 1024..=4 * 1024 * 1024 * 1024).contains(&bounds.memory_bytes)
        || !(1..=64).contains(&bounds.task_count)
        || !(1..=400).contains(&bounds.cpu_percent)
    {
        return Err(CommandError::InvalidSpec);
    }
    Ok(())
}

fn validate_platform_result(
    bounds: &CommandBounds,
    result: &CommandPlatformResult,
) -> Result<(), CommandError> {
    if result.stdout.len() as u64 > bounds.stdout_bytes
        || result.stderr.len() as u64 > bounds.stderr_bytes
        || result.stdout_total_bytes < result.stdout.len() as u64
        || result.stderr_total_bytes < result.stderr.len() as u64
        || validate_digest(&result.stdout_sha256).is_err()
        || validate_digest(&result.stderr_sha256).is_err()
        || (result.stdout_total_bytes == result.stdout.len() as u64
            && sha256_hex(&result.stdout) != result.stdout_sha256)
        || (result.stderr_total_bytes == result.stderr.len() as u64
            && sha256_hex(&result.stderr) != result.stderr_sha256)
        || result.elapsed_ms > bounds.timeout_ms.saturating_add(10_000)
        || !valid_identifier(&result.platform_code)
        || (!result.descendants_terminated
            && matches!(
                result.termination,
                CommandTermination::Cancelled | CommandTermination::TimedOut
            ))
        || (result.termination == CommandTermination::Exited && result.exit_code.is_none())
        || (result.termination != CommandTermination::Exited && result.exit_code.is_some())
    {
        return Err(CommandError::InvalidPlatformResult);
    }
    Ok(())
}

fn valid_argument(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ARGUMENT_BYTES
        && !value.starts_with('@')
        && !PROHIBITED_ARGUMENT_PREFIXES
            .iter()
            .any(|prefix| value == *prefix || value.starts_with(&format!("{prefix}=")))
        && !value.bytes().any(|byte| {
            byte.is_ascii_control()
                || matches!(
                    byte,
                    b'$' | b'`' | b';' | b'&' | b'|' | b'<' | b'>' | b'*' | b'?' | b'[' | b']'
                )
        })
}

fn valid_environment_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ENVIRONMENT_VALUE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'.' | b'/' | b':' | b'-')
        })
}

fn valid_executable(value: &str) -> bool {
    value.starts_with('/')
        && value.len() <= MAX_EXECUTABLE_BYTES
        && !value.ends_with('/')
        && !value.split('/').any(|component| component == "..")
        && value
            .rsplit('/')
            .next()
            .is_some_and(|name| !PROHIBITED_EXECUTABLE_NAMES.contains(&name))
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'+' | b'.' | b'-')
        })
}

fn validate_version(value: &str) -> Result<(), CommandError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if value.len() > 64
        || parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(CommandError::InvalidSpec);
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), CommandError> {
    if valid_identifier(value) {
        Ok(())
    } else {
        Err(CommandError::InvalidSpec)
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn validate_digest(value: &str) -> Result<(), CommandError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(CommandError::InvalidSpec)
    }
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, CommandError> {
    let bytes = serde_json::to_vec(value).map_err(|_| CommandError::ReceiptFailure)?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        BoundedCommandExecutor, CommandBounds, CommandEffectDriver, CommandError,
        CommandLaunchPermit, CommandPlatformResult, CommandRegistry, CommandRequest, CommandRisk,
        CommandSpec, CommandTermination, CommandWorkingDirectory, prepare_command,
    };
    use crate::authority_transaction::{
        AuthorityTransactionCoordinator, AuthorityTransactionRequest,
    };
    use crate::grants::{DerivedOperationGrantRequest, GrantIssuer, SessionReadGrantRequest};
    use crate::policy::{
        PolicyDocument, PolicyEngine, PolicyEvaluationContext, ScopeRules, ToolPolicyBinding,
    };
    use crate::propagation::CancellationToken;
    use crate::test_target::scope;
    use crate::tooling::{Tool, ToolRegistry};
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, AuthorityTransactionId,
        AuthorizedWorkspaceHandle, BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId,
        CancellationReason, CancellationSignal, CapabilityGrant, ContractPayload, CorrelationId,
        DataSensitivity, GrantId, GrantNonce, GrantOperation, GrantSideEffect, HeldWorkspaceRoot,
        OperationAttemptId, OperationBinding, OperationOutcome, PathPlatform,
        RequiredGrantTemplate, SchemaId, SchemaReference, SessionId, TaskId, ToolCall, ToolCallId,
        ToolDefinition, ToolId, ToolRiskLevel, WorkspaceAuthorizationId, WorkspaceId,
        WorkspaceObjectIdentity,
    };

    fn spec(arguments: Vec<String>) -> Result<CommandSpec, CommandError> {
        CommandSpec::seal(
            "fixture.printf",
            "1.0.0",
            "/usr/bin/printf",
            "1".repeat(64),
            arguments,
            CommandWorkingDirectory::OwnedWorktree,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Low,
            CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100)?,
        )
    }

    #[test]
    fn exact_template_prepares_canonical_request_and_preview() {
        let spec = spec(vec!["agentmage-ok".to_owned()]).expect("spec");
        let registry = CommandRegistry::build(vec![spec.clone()]).expect("registry");
        let prepared = prepare_command(
            &registry,
            CommandRequest::new("command-attempt-0001", &spec),
        )
        .expect("prepared");
        assert_eq!(prepared.command(), &spec);
        assert_eq!(prepared.request_sha256().len(), 64);
        assert_eq!(prepared.preview().preview_sha256.len(), 64);
        assert_eq!(prepared.preview().arguments, ["agentmage-ok"]);
    }

    #[test]
    fn shell_grammar_response_files_and_credential_environment_fail_closed() {
        for argument in ["$(id)", "`id`", ";touch", "*.rs", "@response"] {
            assert_eq!(
                spec(vec![argument.to_owned()]),
                Err(CommandError::InvalidSpec),
                "{argument}"
            );
        }
        let mut command = spec(vec!["safe".to_owned()]).expect("baseline");
        command
            .environment
            .insert("AWS_SECRET_ACCESS_KEY".to_owned(), "fixture".to_owned());
        assert_eq!(command.verify(), Err(CommandError::InvalidSpec));
        command = spec(vec!["safe".to_owned()]).expect("baseline");
        command.executable = "/usr/bin/bash".to_owned();
        assert_eq!(command.verify(), Err(CommandError::InvalidSpec));
    }

    #[test]
    fn owned_worktree_mode_uses_one_exact_held_workspace_root() {
        let fixture = authority_fixture();
        let target = agentmage_kernel_contracts::GrantTarget::held_workspace_root(&fixture.held)
            .expect("held root target");
        assert_eq!(
            fixture.prepared.command().working_directory,
            CommandWorkingDirectory::OwnedWorktree
        );
        assert!(target.matches_held_workspace_root(&fixture.held));
    }

    #[test]
    fn registry_rejects_digest_drift_duplicates_and_unknown_versions() {
        let spec = spec(vec!["safe".to_owned()]).expect("baseline");
        assert!(matches!(
            CommandRegistry::build(vec![spec.clone(), spec.clone()]),
            Err(CommandError::DuplicateTemplate)
        ));
        let registry = CommandRegistry::build(vec![spec.clone()]).expect("registry");
        let mut request = CommandRequest::new("command-attempt-0001", &spec);
        request.spec_sha256 = "2".repeat(64);
        assert!(matches!(
            prepare_command(&registry, request),
            Err(CommandError::TemplateNotRegistered)
        ));
    }

    struct FixtureTool(ToolDefinition);

    impl Tool for FixtureTool {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }

    #[derive(Debug)]
    struct SyntheticHeldRoot {
        workspace_id: WorkspaceId,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        identity: WorkspaceObjectIdentity,
    }

    impl AuthorizedWorkspaceHandle for SyntheticHeldRoot {
        fn workspace_id(&self) -> &WorkspaceId {
            &self.workspace_id
        }

        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            &self.authorization_id
        }

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            &self.adapter_instance_id
        }

        fn platform(&self) -> PathPlatform {
            self.identity.platform()
        }
    }

    impl HeldWorkspaceRoot for SyntheticHeldRoot {
        fn root_identity(&self) -> &WorkspaceObjectIdentity {
            &self.identity
        }
    }

    struct FakeExecutor {
        launches: usize,
        result: Option<CommandPlatformResult>,
    }

    impl BoundedCommandExecutor for FakeExecutor {
        type WorkingDirectory = SyntheticHeldRoot;

        fn execute(
            &mut self,
            permit: CommandLaunchPermit<'_>,
            working_directory: &Self::WorkingDirectory,
            _cancellation: &CancellationToken,
        ) -> CommandPlatformResult {
            assert_eq!(permit.command().template_id, "fixture.printf");
            assert_eq!(working_directory.workspace_id.as_str(), "workspace-0001");
            self.launches += 1;
            self.result.take().expect("one fake result")
        }
    }

    struct AuthorityFixture {
        registry: ToolRegistry,
        issuer: GrantIssuer,
        policy: PolicyEngine,
        grant: CapabilityGrant,
        call: ToolCall,
        context: PolicyEvaluationContext,
        approval_id: ApprovalId,
        held: SyntheticHeldRoot,
        prepared: super::PreparedCommand,
    }

    fn command_schema() -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw("command-request"),
            schema_version: 1,
            schema_sha256: "1".repeat(64),
        }
    }

    fn rules<T: Ord>(value: T) -> ScopeRules<T> {
        ScopeRules {
            allowed: BTreeSet::from([value]),
            denied: BTreeSet::new(),
        }
    }

    fn authority_fixture() -> AuthorityFixture {
        let command = spec(vec!["agentmage-ok".to_owned()]).expect("command");
        let commands = CommandRegistry::build(vec![command.clone()]).expect("command registry");
        let prepared = prepare_command(
            &commands,
            CommandRequest::new("command-attempt-0001", &command),
        )
        .expect("prepared command");
        let operation = OperationBinding::new(GrantOperation::CommandExecute);
        let tool_id = ToolId::from_raw("command.run");
        let action_id = ActionId::from_raw("action-command-0001");
        let actor_id = ActorId::from_raw("actor-local-0001");
        let session_id = SessionId::from_raw("session-0001");
        let task_id = TaskId::from_raw("task-0001");
        let held = SyntheticHeldRoot {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-0001"),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [1; 32],
                [2; 32],
            ),
        };
        let target =
            agentmage_kernel_contracts::GrantTarget::held_workspace_root(&held).expect("target");
        let document = PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules(actor_id.clone()),
            tasks: rules(task_id.clone()),
            actions: rules(action_id.clone()),
            tools: rules(ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
            }),
            operations: rules(operation),
            targets: rules(target.clone()),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        };
        let policy = PolicyEngine::new(document).expect("policy");
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FixtureTool(ToolDefinition {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
                display_name: "Bounded command".to_owned(),
                description: "Runs one exact direct command".to_owned(),
                input_schema: command_schema(),
                output_schema: command_schema(),
                risk_level: ToolRiskLevel::Moderate,
                declared_effects: vec![operation],
                required_grant: RequiredGrantTemplate {
                    operation,
                    target_scope: "exact-held-working-directory".to_owned(),
                    single_use: true,
                },
                timeout_ms: command.bounds.timeout_ms,
            })))
            .expect("tool registry");
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-command-0001"),
            correlation_id: CorrelationId::from_raw("correlation-command-0001"),
            action_id: action_id.clone(),
            tool_id: tool_id.clone(),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: command_schema(),
                media_type: "application/json".to_owned(),
                bytes: prepared.request_bytes().to_vec(),
                sha256: prepared.request_sha256().to_owned(),
            },
        };
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-command-0001"),
                actor_id: actor_id.clone(),
                session_id: session_id.clone(),
                task_id: task_id.clone(),
                targets: vec![scope(&[])],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Ephemeral,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 60_000,
                nonce: GrantNonce::from_raw("nonce-parent-command-0001"),
                maximum_derived_operations: 1,
                preview_sha256: "2".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .expect("parent grant");
        let approval_id = ApprovalId::from_raw("approval-command-0001");
        let grant = issuer
            .derive_operation(
                &parent.grant_id,
                DerivedOperationGrantRequest {
                    grant_id: GrantId::from_raw("grant-command-0001"),
                    approval_id: approval_id.clone(),
                    action_id: action_id.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    operation,
                    tool_id: tool_id.clone(),
                    tool_version: "1.0.0".to_owned(),
                    targets: vec![target.clone()],
                    argument_sha256: prepared.request_sha256().to_owned(),
                    preimages: Vec::new(),
                    expected_side_effects: vec![GrantSideEffect {
                        operation,
                        target_indexes: vec![0],
                        details_sha256: prepared.preview().preview_sha256.clone(),
                    }],
                    rollback_description: "No state change is permitted".to_owned(),
                    issued_at_epoch_ms: 2_000,
                    expires_at_epoch_ms: 30_000,
                    nonce: GrantNonce::from_raw("nonce-command-0001"),
                    preview_sha256: prepared.preview().preview_sha256.clone(),
                    policy_sha256: policy.policy_sha256().to_owned(),
                },
            )
            .expect("operation grant");
        let context = PolicyEvaluationContext {
            actor_id,
            session_id,
            task_id,
            action_id,
            action_kind: ActionKind::DeterministicTool,
            tool_id,
            tool_version: "1.0.0".to_owned(),
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: 3_000,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        AuthorityFixture {
            registry,
            issuer,
            policy,
            grant,
            call,
            context,
            approval_id,
            held,
            prepared,
        }
    }

    fn successful_platform_result() -> CommandPlatformResult {
        CommandPlatformResult {
            termination: CommandTermination::Exited,
            exit_code: Some(0),
            signal: None,
            stdout: b"agentmage-ok".to_vec(),
            stdout_sha256: super::sha256_hex(b"agentmage-ok"),
            stdout_total_bytes: 12,
            stderr: Vec::new(),
            stderr_sha256: super::sha256_hex(&[]),
            stderr_total_bytes: 0,
            elapsed_ms: 4,
            descendants_terminated: true,
            platform_code: "fixture.command.exited".to_owned(),
        }
    }

    #[test]
    fn consumed_exact_grant_is_required_before_one_executor_launch_and_receipt() {
        let mut fixture = authority_fixture();
        let cancellation = CancellationToken::root(
            BoundaryKind::Tool,
            fixture.grant.task_id.clone(),
            fixture.call.correlation_id.clone(),
        );
        let executor = FakeExecutor {
            launches: 0,
            result: Some(successful_platform_result()),
        };
        let mut driver =
            CommandEffectDriver::new(executor, fixture.held, fixture.prepared, cancellation);
        let request = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw("transaction-command-0001"),
            OperationAttemptId::from_raw("attempt-command-0001"),
            fixture.approval_id,
            fixture.grant.grant_id,
            fixture.call,
            fixture.context,
            4_000,
            "1970-01-01T00:00:04Z",
        )
        .expect("transaction request");
        let receipt = AuthorityTransactionCoordinator::new()
            .execute_effect(
                &fixture.registry,
                &mut fixture.issuer,
                &fixture.policy,
                request,
                &mut driver,
            )
            .expect("mediated command");
        assert_eq!(receipt.outcome, OperationOutcome::Succeeded);
        let command_receipt = driver.take_receipt().expect("command receipt");
        assert_eq!(command_receipt.outcome, OperationOutcome::Succeeded);
        assert_eq!(command_receipt.exit_code, Some(0));
        assert_eq!(
            command_receipt.stdout_sha256,
            super::sha256_hex(b"agentmage-ok")
        );
        assert!(command_receipt.descendants_terminated);
        assert!(super::verify_command_receipt(
            &driver.prepared,
            &command_receipt
        ));
        for sequence in 0_u8..8 {
            let mut changed = command_receipt.clone();
            match sequence {
                0 => changed.command_attempt_id.push('x'),
                1 => changed.spec_sha256 = "9".repeat(64),
                2 => changed.outcome = OperationOutcome::Failed,
                3 => changed.exit_code = Some(1),
                4 => changed.stdout_total_bytes = 1,
                5 => changed.platform_code.clear(),
                6 => changed.elapsed_ms = 1_000_000,
                _ => changed.receipt_sha256 = "8".repeat(64),
            }
            assert!(!super::verify_command_receipt(&driver.prepared, &changed));
        }
        assert_eq!(driver.into_executor().launches, 1);
    }

    #[test]
    fn cancellation_before_launch_emits_one_receipt_and_never_calls_executor() {
        let mut fixture = authority_fixture();
        let cancellation = CancellationToken::root(
            BoundaryKind::Tool,
            fixture.grant.task_id.clone(),
            fixture.call.correlation_id.clone(),
        );
        cancellation
            .cancel(CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("cancel-command-0001"),
                correlation_id: fixture.call.correlation_id.clone(),
                task_id: fixture.grant.task_id.clone(),
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Tool,
            })
            .expect("cancel");
        let executor = FakeExecutor {
            launches: 0,
            result: None,
        };
        let mut driver =
            CommandEffectDriver::new(executor, fixture.held, fixture.prepared, cancellation);
        let request = AuthorityTransactionRequest::new(
            AuthorityTransactionId::from_raw("transaction-command-cancel-0001"),
            OperationAttemptId::from_raw("attempt-command-cancel-0001"),
            fixture.approval_id,
            fixture.grant.grant_id,
            fixture.call,
            fixture.context,
            4_000,
            "1970-01-01T00:00:04Z",
        )
        .expect("transaction request");
        let receipt = AuthorityTransactionCoordinator::new()
            .execute_effect(
                &fixture.registry,
                &mut fixture.issuer,
                &fixture.policy,
                request,
                &mut driver,
            )
            .expect("cancelled command");
        assert_eq!(receipt.outcome, OperationOutcome::Cancelled);
        let command_receipt = driver.take_receipt().expect("command receipt");
        assert_eq!(command_receipt.outcome, OperationOutcome::Cancelled);
        assert_eq!(command_receipt.termination, CommandTermination::Cancelled);
        assert!(super::verify_command_receipt(
            &driver.prepared,
            &command_receipt
        ));
        assert_eq!(driver.into_executor().launches, 0);
    }
}
