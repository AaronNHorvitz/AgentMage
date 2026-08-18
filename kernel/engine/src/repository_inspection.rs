//! Typed, read-only Git inspection plans and nonforgeable execution permits.

use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;

use agentmage_kernel_contracts::{
    GrantOperation, HeldWorkspaceRoot, OperationOutcome, StateChange, WorkspaceId, WorkspacePath,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::authority_transaction::{EffectAuthorization, EffectDriver, EffectLaunch, EffectResult};
use crate::propagation::CancellationToken;

const INSPECTION_SCHEMA_VERSION: u16 = 1;
const MAX_GIT_RECORDS: u32 = 1_000;
const MAX_GIT_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_GIT_PATHS: usize = 256;
const MAX_GIT_REVISION_BYTES: usize = 256;
const MAX_PLATFORM_CODE_BYTES: usize = 128;

/// Stable content-free refusal at the repository-inspection boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryInspectionError {
    /// A typed request, digest, or fixed command plan is malformed.
    InvalidPlan,
    /// Consumed authority does not bind the exact inspection and held root.
    AuthorityMismatch,
    /// The platform returned malformed, inconsistent, or over-limit evidence.
    InvalidPlatformResult,
}

impl RepositoryInspectionError {
    /// Returns a stable machine-readable failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPlan => "repository.inspection.plan.invalid",
            Self::AuthorityMismatch => "repository.inspection.authority.mismatch",
            Self::InvalidPlatformResult => "repository.inspection.platform.invalid",
        }
    }
}

impl fmt::Display for RepositoryInspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for RepositoryInspectionError {}

/// Closed read-only repository operation admitted by the kernel executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryInspectionOperation {
    /// Porcelain-v2 status including branch and untracked state.
    Status,
    /// Current symbolic branch.
    CurrentBranch,
    /// Configured upstream of the current branch.
    Upstream,
    /// Local and remote branch references.
    BranchList,
    /// Bounded commit log.
    Log,
    /// Worktree-versus-index diff.
    Diff,
    /// Index-versus-HEAD diff.
    StagedDiff,
    /// One exact commit or object presentation.
    Show,
    /// Registered worktree list.
    WorktreeList,
    /// Exact object type.
    Object,
    /// Exact ref resolution.
    Ref,
    /// Dirty-tree determination.
    DirtyTree,
    /// Untracked path inspection.
    UntrackedFiles,
}

impl RepositoryInspectionOperation {
    /// Complete fixed operation inventory admitted by the kernel.
    pub const ALL: [Self; 13] = [
        Self::Status,
        Self::CurrentBranch,
        Self::Upstream,
        Self::BranchList,
        Self::Log,
        Self::Diff,
        Self::StagedDiff,
        Self::Show,
        Self::WorktreeList,
        Self::Object,
        Self::Ref,
        Self::DirtyTree,
        Self::UntrackedFiles,
    ];
}

/// Typed authority-free input from which the kernel reconstructs one Git invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryInspectionRequest {
    /// Request schema version. Only version one is accepted.
    pub schema_version: u16,
    /// Exact fixed operation.
    pub operation: RepositoryInspectionOperation,
    /// Optional exact revision or ref.
    pub revision: Option<String>,
    /// Optional exact hexadecimal object identity.
    pub object_id: Option<String>,
    /// Canonical workspace-relative path component lists.
    pub pathspecs: Vec<Vec<String>>,
    /// Maximum parsed records.
    pub max_records: u32,
    /// Maximum admitted output bytes.
    pub max_output_bytes: u64,
}

/// Kernel-validated immutable execution plan for one read-only Git inspection.
pub struct PreparedRepositoryInspection {
    request: RepositoryInspectionRequest,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    stdin: Vec<u8>,
    call_argument_sha256: String,
    operation_plan_sha256: String,
    execution_plan_sha256: String,
}

impl PreparedRepositoryInspection {
    /// Returns the exact typed request.
    #[must_use]
    pub const fn request(&self) -> &RepositoryInspectionRequest {
        &self.request
    }

    /// Returns literal arguments after the pinned `git` executable.
    #[must_use]
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    /// Returns the complete replacement environment.
    #[must_use]
    pub const fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }

    /// Returns exact standard-input bytes, nonempty only for object inspection.
    #[must_use]
    pub fn stdin(&self) -> &[u8] {
        &self.stdin
    }

    /// Returns the exact outer tool-call argument digest.
    #[must_use]
    pub fn call_argument_sha256(&self) -> &str {
        &self.call_argument_sha256
    }

    /// Returns the profile-bound operation-plan digest shown for approval.
    #[must_use]
    pub fn operation_plan_sha256(&self) -> &str {
        &self.operation_plan_sha256
    }

    /// Returns the digest of this fixed kernel execution plan.
    #[must_use]
    pub fn execution_plan_sha256(&self) -> &str {
        &self.execution_plan_sha256
    }

    fn verify(&self) -> Result<(), RepositoryInspectionError> {
        let rebuilt = prepare_repository_inspection(
            self.request.clone(),
            self.call_argument_sha256.clone(),
            self.operation_plan_sha256.clone(),
        )?;
        if rebuilt.arguments != self.arguments
            || rebuilt.environment != self.environment
            || rebuilt.stdin != self.stdin
            || rebuilt.execution_plan_sha256 != self.execution_plan_sha256
        {
            return Err(RepositoryInspectionError::InvalidPlan);
        }
        Ok(())
    }
}

impl fmt::Debug for PreparedRepositoryInspection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedRepositoryInspection")
            .field("operation", &self.request.operation)
            .field("argument_count", &self.arguments.len())
            .field("stdin_bytes", &self.stdin.len())
            .field("execution_plan_sha256", &self.execution_plan_sha256)
            .finish_non_exhaustive()
    }
}

/// Reconstructs one fixed Git invocation independently of capability-pack planning.
pub fn prepare_repository_inspection(
    request: RepositoryInspectionRequest,
    call_argument_sha256: impl Into<String>,
    operation_plan_sha256: impl Into<String>,
) -> Result<PreparedRepositoryInspection, RepositoryInspectionError> {
    validate_request(&request)?;
    let call_argument_sha256 = call_argument_sha256.into();
    let operation_plan_sha256 = operation_plan_sha256.into();
    if !is_sha256(&call_argument_sha256) || !is_sha256(&operation_plan_sha256) {
        return Err(RepositoryInspectionError::InvalidPlan);
    }
    let mut arguments = hardened_prefix();
    let stdin = Vec::new();
    match request.operation {
        RepositoryInspectionOperation::Status | RepositoryInspectionOperation::DirtyTree => {
            arguments.extend(
                [
                    "status",
                    "--porcelain=v2",
                    "--branch",
                    "-z",
                    "--untracked-files=all",
                ]
                .map(str::to_owned),
            );
        }
        RepositoryInspectionOperation::CurrentBranch => {
            arguments.extend(["symbolic-ref", "--quiet", "--short", "HEAD"].map(str::to_owned))
        }
        RepositoryInspectionOperation::Upstream => arguments.extend(
            [
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ]
            .map(str::to_owned),
        ),
        RepositoryInspectionOperation::BranchList => arguments.extend(
            [
                "for-each-ref",
                "--sort=refname",
                "--format=%(refname)%00%(objectname)%00%(upstream)%00",
                "refs/heads",
                "refs/remotes",
            ]
            .map(str::to_owned),
        ),
        RepositoryInspectionOperation::Log => {
            arguments.extend([
                "log".to_owned(),
                "--no-decorate".to_owned(),
                "--no-show-signature".to_owned(),
                format!("--max-count={}", request.max_records),
                "--format=%H%x00%P%x00%an%x00%ae%x00%at%x00%s%x00".to_owned(),
            ]);
            append_revision(&mut arguments, request.revision.as_deref());
        }
        RepositoryInspectionOperation::Diff | RepositoryInspectionOperation::StagedDiff => {
            arguments.push("diff".to_owned());
            arguments.extend(
                ["--no-ext-diff", "--no-textconv", "--binary", "--no-renames"].map(str::to_owned),
            );
            if request.operation == RepositoryInspectionOperation::StagedDiff {
                arguments.push("--cached".to_owned());
            }
            append_pathspecs(&mut arguments, &request.pathspecs);
        }
        RepositoryInspectionOperation::Show => {
            arguments.extend(
                [
                    "show",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--binary",
                    "--no-renames",
                    "--no-show-signature",
                    "--format=%H%x00%P%x00%an%x00%ae%x00%at%x00%s%x00",
                ]
                .map(str::to_owned),
            );
            append_revision(&mut arguments, request.revision.as_deref());
            append_pathspecs(&mut arguments, &request.pathspecs);
        }
        RepositoryInspectionOperation::WorktreeList => {
            arguments.extend(["worktree", "list", "--porcelain", "-z"].map(str::to_owned))
        }
        RepositoryInspectionOperation::Object => {
            arguments.extend(["cat-file", "-t"].map(str::to_owned));
            arguments.push(
                request
                    .object_id
                    .as_deref()
                    .expect("validated object identity")
                    .to_owned(),
            );
        }
        RepositoryInspectionOperation::Ref => {
            arguments.extend(["rev-parse", "--verify", "--end-of-options"].map(str::to_owned));
            arguments.push(format!(
                "{}^{{object}}",
                request.revision.as_deref().expect("validated revision")
            ));
        }
        RepositoryInspectionOperation::UntrackedFiles => arguments
            .extend(["ls-files", "--others", "--exclude-standard", "-z"].map(str::to_owned)),
    }
    let environment = hardened_environment();
    let execution_plan_sha256 = plan_digest(
        &request,
        &arguments,
        &environment,
        &stdin,
        &call_argument_sha256,
        &operation_plan_sha256,
    )?;
    Ok(PreparedRepositoryInspection {
        request,
        arguments,
        environment,
        stdin,
        call_argument_sha256,
        operation_plan_sha256,
        execution_plan_sha256,
    })
}

/// Terminal reason observed by a trusted repository-inspection executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryInspectionTermination {
    /// Git exited and supplied an exit code.
    Exited,
    /// Cancellation stopped the process tree.
    Cancelled,
    /// The fixed deadline stopped the process tree.
    TimedOut,
    /// The absolute output ceiling stopped the process tree.
    OutputLimit,
    /// Isolation or process launch failed.
    LaunchFailed,
}

/// Bounded platform evidence from one repository-inspection process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryInspectionPlatformResult {
    /// Terminal reason.
    pub termination: RepositoryInspectionTermination,
    /// Exit code when the process exited normally.
    pub exit_code: Option<i32>,
    /// Complete retained standard output within the absolute ceiling.
    pub stdout: Vec<u8>,
    /// Digest of retained standard output.
    pub stdout_sha256: String,
    /// Total retained standard-output bytes.
    pub stdout_bytes: u64,
    /// Digest of bounded standard error, whose bytes are not exposed.
    pub stderr_sha256: String,
    /// Total bounded standard-error bytes.
    pub stderr_bytes: u64,
    /// Monotonic elapsed duration.
    pub elapsed_ms: u64,
    /// Whether the complete process tree was reconciled.
    pub descendants_terminated: bool,
    /// Stable content-free platform code.
    pub platform_code: String,
}

/// Nonforgeable permit exposing one verified repository-inspection plan to a platform adapter.
pub struct RepositoryInspectionLaunchPermit<'plan> {
    prepared: &'plan PreparedRepositoryInspection,
}

impl<'plan> RepositoryInspectionLaunchPermit<'plan> {
    /// Returns the exact kernel-verified plan.
    #[must_use]
    pub const fn prepared(&self) -> &'plan PreparedRepositoryInspection {
        self.prepared
    }
}

impl fmt::Debug for RepositoryInspectionLaunchPermit<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RepositoryInspectionLaunchPermit")
            .field("operation", &self.prepared.request.operation)
            .finish_non_exhaustive()
    }
}

/// Trusted platform executor whose launch requires a kernel-created permit.
pub trait BoundedRepositoryInspectionExecutor {
    /// Platform-owned held workspace-root type accepted by this executor.
    type WorkingDirectory: HeldWorkspaceRoot;

    /// Executes one exact offline read-only Git inspection.
    fn execute(
        &mut self,
        permit: RepositoryInspectionLaunchPermit<'_>,
        working_directory: &Self::WorkingDirectory,
        cancellation: &CancellationToken,
    ) -> RepositoryInspectionPlatformResult;
}

/// Inert repository-inspection driver crossing the effect boundary once.
pub struct RepositoryInspectionEffectDriver<E, H> {
    executor: E,
    held_working_directory: H,
    prepared: PreparedRepositoryInspection,
    cancellation: CancellationToken,
    result: Option<RepositoryInspectionPlatformResult>,
    error: Option<RepositoryInspectionError>,
}

impl<E, H> RepositoryInspectionEffectDriver<E, H> {
    /// Creates an inert driver without launching Git or granting authority.
    #[must_use]
    pub const fn new(
        executor: E,
        held_working_directory: H,
        prepared: PreparedRepositoryInspection,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            executor,
            held_working_directory,
            prepared,
            cancellation,
            result: None,
            error: None,
        }
    }

    /// Takes bounded platform evidence after the authority transaction closes.
    pub fn take_result(&mut self) -> Option<RepositoryInspectionPlatformResult> {
        self.result.take()
    }

    /// Takes a content-free inspection-boundary error.
    pub fn take_error(&mut self) -> Option<RepositoryInspectionError> {
        self.error.take()
    }

    /// Returns the executor after this attempt closes.
    #[must_use]
    pub fn into_executor(self) -> E {
        self.executor
    }
}

impl<E, H> fmt::Debug for RepositoryInspectionEffectDriver<E, H> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RepositoryInspectionEffectDriver")
            .field("prepared", &self.prepared)
            .field("has_result", &self.result.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl<E, H> EffectDriver for RepositoryInspectionEffectDriver<E, H>
where
    E: BoundedRepositoryInspectionExecutor,
    H: Borrow<E::WorkingDirectory>,
{
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        let held = self.held_working_directory.borrow();
        let call = authorization.call();
        let side_effects = authorization.expected_side_effects();
        if authorization.operation().operation() != GrantOperation::WorkspaceRead
            || !authorization.authorizes_held_workspace_root(held)
            || call.arguments.sha256 != self.prepared.call_argument_sha256
            || sha256_hex(&call.arguments.bytes) != call.arguments.sha256
            || side_effects.len() != 1
            || side_effects[0].operation != authorization.operation()
            || side_effects[0].target_indexes != [0]
            || side_effects[0].details_sha256 != self.prepared.operation_plan_sha256
            || self.prepared.verify().is_err()
        {
            self.error = Some(RepositoryInspectionError::AuthorityMismatch);
            return EffectLaunch::failed();
        }

        let platform = if self.cancellation.is_cancelled() {
            RepositoryInspectionPlatformResult {
                termination: RepositoryInspectionTermination::Cancelled,
                exit_code: None,
                stdout: Vec::new(),
                stdout_sha256: sha256_hex(&[]),
                stdout_bytes: 0,
                stderr_sha256: sha256_hex(&[]),
                stderr_bytes: 0,
                elapsed_ms: 0,
                descendants_terminated: true,
                platform_code: "repository.inspection.cancelled.before-launch".to_owned(),
            }
        } else {
            self.executor.execute(
                RepositoryInspectionLaunchPermit {
                    prepared: &self.prepared,
                },
                held,
                &self.cancellation,
            )
        };
        if validate_platform_result(&platform).is_err() {
            self.error = Some(RepositoryInspectionError::InvalidPlatformResult);
            return EffectLaunch::failed();
        }
        let outcome = platform_outcome(&platform);
        let state_change = if platform.descendants_terminated {
            StateChange::NotChanged
        } else {
            StateChange::Uncertain
        };
        let effect = EffectResult::from_redacted_material(
            outcome,
            platform.stdout_sha256.as_bytes(),
            state_change,
        );
        self.result = Some(platform);
        EffectLaunch::completed(effect)
    }
}

fn validate_request(
    request: &RepositoryInspectionRequest,
) -> Result<(), RepositoryInspectionError> {
    if request.schema_version != INSPECTION_SCHEMA_VERSION
        || request.max_records == 0
        || request.max_records > MAX_GIT_RECORDS
        || request.max_output_bytes == 0
        || request.max_output_bytes > MAX_GIT_OUTPUT_BYTES
        || request.pathspecs.len() > MAX_GIT_PATHS
        || request.pathspecs.iter().any(|path| !valid_path(path))
        || request
            .revision
            .as_deref()
            .is_some_and(|value| !valid_revision(value))
        || request
            .object_id
            .as_deref()
            .is_some_and(|value| !valid_object_id(value))
    {
        return Err(RepositoryInspectionError::InvalidPlan);
    }
    let revision_shape = match request.operation {
        RepositoryInspectionOperation::Show | RepositoryInspectionOperation::Ref => {
            request.revision.is_some()
        }
        RepositoryInspectionOperation::Log => true,
        _ => request.revision.is_none(),
    };
    let object_shape =
        (request.operation == RepositoryInspectionOperation::Object) == request.object_id.is_some();
    let path_shape = matches!(
        request.operation,
        RepositoryInspectionOperation::Diff
            | RepositoryInspectionOperation::StagedDiff
            | RepositoryInspectionOperation::Show
    ) || request.pathspecs.is_empty();
    if !revision_shape || !object_shape || !path_shape {
        return Err(RepositoryInspectionError::InvalidPlan);
    }
    Ok(())
}

fn validate_platform_result(
    result: &RepositoryInspectionPlatformResult,
) -> Result<(), RepositoryInspectionError> {
    if result.stdout.len() as u64 != result.stdout_bytes
        || result.stdout_bytes > MAX_GIT_OUTPUT_BYTES
        || result.stderr_bytes > MAX_GIT_OUTPUT_BYTES
        || sha256_hex(&result.stdout) != result.stdout_sha256
        || !is_sha256(&result.stderr_sha256)
        || result.platform_code.is_empty()
        || result.platform_code.len() > MAX_PLATFORM_CODE_BYTES
        || !result
            .platform_code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
        || match result.termination {
            RepositoryInspectionTermination::Exited => result.exit_code.is_none(),
            RepositoryInspectionTermination::Cancelled
            | RepositoryInspectionTermination::TimedOut
            | RepositoryInspectionTermination::OutputLimit
            | RepositoryInspectionTermination::LaunchFailed => result.exit_code.is_some(),
        }
    {
        return Err(RepositoryInspectionError::InvalidPlatformResult);
    }
    Ok(())
}

fn platform_outcome(result: &RepositoryInspectionPlatformResult) -> OperationOutcome {
    if !result.descendants_terminated {
        return OperationOutcome::Uncertain;
    }
    match result.termination {
        RepositoryInspectionTermination::Exited if result.exit_code == Some(0) => {
            OperationOutcome::Succeeded
        }
        RepositoryInspectionTermination::Exited
        | RepositoryInspectionTermination::OutputLimit
        | RepositoryInspectionTermination::LaunchFailed => OperationOutcome::Failed,
        RepositoryInspectionTermination::Cancelled => OperationOutcome::Cancelled,
        RepositoryInspectionTermination::TimedOut => OperationOutcome::TimedOut,
    }
}

fn hardened_prefix() -> Vec<String> {
    [
        "--no-pager",
        "-c",
        "core.hooksPath=/dev/null",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "diff.external=",
        "-c",
        "diff.trustExitCode=false",
    ]
    .map(str::to_owned)
    .to_vec()
}

fn hardened_environment() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("GIT_CONFIG_GLOBAL".to_owned(), "/dev/null".to_owned()),
        ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
        ("GIT_NO_REPLACE_OBJECTS".to_owned(), "1".to_owned()),
        ("GIT_OPTIONAL_LOCKS".to_owned(), "0".to_owned()),
        ("GIT_PAGER".to_owned(), "cat".to_owned()),
        ("GIT_PROTOCOL_FROM_USER".to_owned(), "0".to_owned()),
        ("GIT_TERMINAL_PROMPT".to_owned(), "0".to_owned()),
        ("HOME".to_owned(), "/nonexistent".to_owned()),
        ("LANG".to_owned(), "C".to_owned()),
        ("LC_ALL".to_owned(), "C".to_owned()),
        ("PAGER".to_owned(), "cat".to_owned()),
    ])
}

fn append_revision(arguments: &mut Vec<String>, revision: Option<&str>) {
    arguments.push("--end-of-options".to_owned());
    if let Some(revision) = revision {
        arguments.push(revision.to_owned());
    }
}

fn append_pathspecs(arguments: &mut Vec<String>, pathspecs: &[Vec<String>]) {
    arguments.push("--".to_owned());
    arguments.extend(pathspecs.iter().map(|path| path.join("/")));
}

fn valid_revision(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_GIT_REVISION_BYTES
        && !value.starts_with('-')
        && !value.starts_with('.')
        && !value.ends_with(['/', '.'])
        && !value.contains("..")
        && !value.contains("//")
        && !value.contains("@{")
        && !value.ends_with(".lock")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._@-".contains(&byte))
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_path(path: &[String]) -> bool {
    WorkspacePath::new(
        WorkspaceId::from_raw("repository-inspection-validation"),
        path.iter().cloned(),
    )
    .is_ok()
}

#[derive(Serialize)]
struct InspectionPlanMaterial<'plan> {
    schema_version: u16,
    request: InspectionRequestMaterial<'plan>,
    arguments: &'plan [String],
    environment: &'plan BTreeMap<String, String>,
    stdin_sha256: String,
    call_argument_sha256: &'plan str,
    operation_plan_sha256: &'plan str,
}

#[derive(Serialize)]
struct InspectionRequestMaterial<'request> {
    schema_version: u16,
    operation: RepositoryInspectionOperation,
    revision: &'request Option<String>,
    object_id: &'request Option<String>,
    pathspecs: &'request [Vec<String>],
    max_records: u32,
    max_output_bytes: u64,
}

fn plan_digest(
    request: &RepositoryInspectionRequest,
    arguments: &[String],
    environment: &BTreeMap<String, String>,
    stdin: &[u8],
    call_argument_sha256: &str,
    operation_plan_sha256: &str,
) -> Result<String, RepositoryInspectionError> {
    serde_json::to_vec(&InspectionPlanMaterial {
        schema_version: INSPECTION_SCHEMA_VERSION,
        request: InspectionRequestMaterial {
            schema_version: request.schema_version,
            operation: request.operation,
            revision: &request.revision,
            object_id: &request.object_id,
            pathspecs: &request.pathspecs,
            max_records: request.max_records,
            max_output_bytes: request.max_output_bytes,
        },
        arguments,
        environment,
        stdin_sha256: sha256_hex(stdin),
        call_argument_sha256,
        operation_plan_sha256,
    })
    .map(|bytes| sha256_hex(&bytes))
    .map_err(|_| RepositoryInspectionError::InvalidPlan)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}
