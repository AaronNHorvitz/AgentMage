// SPDX-License-Identifier: Apache-2.0
//! Explicit development startup with native worktree creation and strict edits.

use super::*;
use agentmage_host::{
    coding_development_runtime::OsRuntimeClock,
    coding_endpoint::CodingEndpointConnection,
    coding_endpoint_edit_previews::CodingEndpointValidation,
    coding_endpoint_prepared_startup::{CodingEndpointPreparedKind, CodingEndpointPreparedStartup},
};
use agentmage_kernel_contracts::{
    RuntimeEndpointRunRequest, RuntimeRunId, SessionId, VerifierCandidate, VerifierId,
};
use agentmage_kernel_engine::{
    command_runner::{CommandSpec, CommandWorkingDirectory},
    instruction_provenance::InstructionEvidenceLedger,
    runtime_loop::{RuntimePortFailure, RuntimeVerificationInput, RuntimeVerifierPort},
    validation_template::{ValidationKind, ValidationParserKind},
};
use coding_endpoint_launch::{bind_identities, private_bytes};
use std::{collections::BTreeSet, io, os::fd::AsFd, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Launch {
    version: u32,
    state_root: PathBuf,
    repository_root: PathBuf,
    owned_root: PathBuf,
    base_commit: String,
    credential_file: Option<PathBuf>,
    request: RuntimeEndpointRunRequest,
    instructions: InstructionEvidenceLedger,
    paths: Vec<String>,
    source_budget: usize,
    validation: Option<Validation>,
}

// Private startup metadata only. Neither HTTP nor a model can supply this
// definition, and parsing it grants no command execution authority.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Validation {
    command: CommandSpec,
    kind: ValidationKind,
    minimum_test_count: u32,
    output_format: ValidationParserKind,
}

impl Validation {
    fn into_native(self) -> Result<CodingEndpointValidation, ApiError> {
        self.command.verify().map_err(|_| ApiError::Configuration)?;
        if self.command.working_directory != CommandWorkingDirectory::OwnedWorktree
            || !self.kind.is_test()
            || self.minimum_test_count == 0
        {
            return Err(ApiError::Configuration);
        }
        let output_format = match self.output_format {
            ValidationParserKind::AgentMageJsonV1 => {
                agentmage_platform_linux::LinuxCodingTestFormat::AgentMageJsonV1
            }
            ValidationParserKind::RustLibtestTextV1 => {
                agentmage_platform_linux::LinuxCodingTestFormat::RustLibtestTextV1
            }
            ValidationParserKind::PythonUnittestTextV1 => {
                agentmage_platform_linux::LinuxCodingTestFormat::PythonUnittestTextV1
            }
            ValidationParserKind::ProcessExitV1 => return Err(ApiError::Configuration),
        };
        Ok(CodingEndpointValidation {
            command: self.command,
            kind: self.kind,
            minimum_test_count: self.minimum_test_count,
            output_format,
        })
    }
}

fn load(path: &Path, kind: CodingEndpointPreparedKind) -> Result<Launch, ApiError> {
    let bytes = private_bytes(path, 1024 * 1024)?;
    let launch: Launch = object_json(&bytes).map_err(|_| ApiError::Configuration)?;
    let shape: Value = serde_json::from_slice(&bytes).map_err(|_| ApiError::Configuration)?;
    if launch.version != 1 || !shape["request"].is_object() || !shape["instructions"].is_object() {
        return Err(ApiError::Configuration);
    }
    match kind {
        CodingEndpointPreparedKind::Edit
            if !shape.as_object().unwrap().contains_key("validation") => {}
        CodingEndpointPreparedKind::EditAndTest
            if shape["validation"].is_object()
                && shape["validation"]["command"].is_object()
                && shape["validation"]["command"]["bounds"].is_object()
                && shape["validation"]["command"]["environment"].is_object() => {}
        _ => return Err(ApiError::Configuration),
    }
    Ok(launch)
}

fn prepare(
    launch: &Launch,
    run: RuntimeRunId,
    session: SessionId,
    kind: CodingEndpointPreparedKind,
) -> Result<CodingEndpointPreparedStartup, ApiError> {
    let request = bind_identities(launch.request.clone(), run, session)?;
    let startup = CodingEndpointPreparedStartup::prepare(request, kind, &launch.instructions)
        .map_err(|_| ApiError::Configuration)?;
    // Refuse unusable source metadata before native inspection or state writes.
    if launch.paths.is_empty()
        || launch.paths.len() > 128
        || launch.paths.iter().collect::<BTreeSet<_>>().len() != launch.paths.len()
        || !(2048..=32768).contains(&launch.source_budget)
    {
        return Err(ApiError::Configuration);
    }
    startup
        .check_sources(
            &launch
                .paths
                .iter()
                .map(|path| CodingSource {
                    path: path.clone(),
                    content: String::new(),
                })
                .collect::<Vec<_>>(),
        )
        .map_err(|_| ApiError::Configuration)?;
    // The original native owner uses this fixed label; it is not provenance.
    if startup.request().workspace_id.as_str() != "local-coding-worktree-create" {
        return Err(ApiError::Configuration);
    }
    let endpoint = LocalChatEndpoint::parse(&startup.request().model_selection.endpoint)
        .map_err(|_| ApiError::Configuration)?;
    if endpoint.base_url() != startup.request().model_selection.endpoint {
        return Err(ApiError::Configuration);
    }
    // Reject path aliases and overlap before opening encrypted state. Native
    // held-root validation still rechecks each root around every effect.
    let roots = [
        &launch.state_root,
        &launch.repository_root,
        &launch.owned_root,
    ];
    for (i, root) in roots.iter().enumerate() {
        if !root.is_absolute()
            || std::fs::canonicalize(root).ok().as_ref() != Some(*root)
            || roots
                .iter()
                .enumerate()
                .any(|(j, other)| i != j && root.starts_with(other))
        {
            return Err(ApiError::Configuration);
        }
    }
    Ok(startup)
}

// An exact bounded terminal decision, not a reusable boolean launch permission.
// The native owner rechecks the digest, expiry, roots, Git observations and grant.
fn creation_decision(reader: &mut impl Read, preview_sha256: &str) -> Result<bool, ApiError> {
    if preview_sha256.len() != 64
        || !preview_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ApiError::Configuration);
    }
    let mut line = Vec::new();
    for _ in 0..=80 {
        let mut byte = [0];
        if reader.read(&mut byte).map_err(|_| ApiError::Io)? != 1 {
            return Err(ApiError::Configuration);
        }
        if byte[0] == b'\n' {
            return match line.as_slice() {
                b"deny" => Ok(false),
                text if text == format!("approve {preview_sha256}").as_bytes() => Ok(true),
                _ => Err(ApiError::Configuration),
            };
        }
        line.push(byte[0]);
    }
    Err(ApiError::Configuration)
}

// Nonblocking single-byte reads keep shutdown and the native preview lifetime
// observable even for an idle terminal/pipe. Restore the inherited fd flags.
struct ApprovalInput<F: AsFd> {
    input: F,
    original_flags: OFlags,
    stop: Arc<AtomicBool>,
    deadline: Instant,
}
impl<F: AsFd> ApprovalInput<F> {
    fn new(input: F, stop: Arc<AtomicBool>) -> Result<Self, ApiError> {
        let original_flags = rustix::fs::fcntl_getfl(&input).map_err(|_| ApiError::Io)?;
        rustix::fs::fcntl_setfl(&input, original_flags | OFlags::NONBLOCK)
            .map_err(|_| ApiError::Io)?;
        Ok(Self {
            input,
            original_flags,
            stop,
            deadline: Instant::now() + Duration::from_secs(120),
        })
    }
}
impl<F: AsFd> Read for ApprovalInput<F> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.stop.load(Ordering::Acquire) || Instant::now() >= self.deadline {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "approval stopped",
                ));
            }
            match rustix::io::read(&self.input, &mut *buffer) {
                Ok(n) => return Ok(n),
                Err(rustix::io::Errno::AGAIN) => thread::sleep(Duration::from_millis(10)),
                Err(error) => return Err(error.into()),
            }
        }
    }
}
impl<F: AsFd> Drop for ApprovalInput<F> {
    fn drop(&mut self) {
        let _ = rustix::fs::fcntl_setfl(&self.input, self.original_flags);
    }
}

struct RunControl {
    deadline: ProposalDispatchControl,
    stop: Arc<AtomicBool>,
}
impl ModelOperationControl for RunControl {
    fn remaining_ms(&self) -> Result<std::num::NonZeroU64, ModelOperationStop> {
        if self.stop.load(Ordering::Acquire) {
            return Err(ModelOperationStop::Cancelled);
        }
        self.deadline.remaining_ms()
    }
}

// Neither development command binds an independent task/current-source checker.
// Even a parsed test pass cannot certify any completion candidate.
struct UnverifiedEdits(VerifierId);
impl RuntimeVerifierPort for UnverifiedEdits {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
}

/// Start an explicitly approved development edit service from a private launch
/// file. Starting permits private configuration/key reads, encrypted state and
/// full-repository offline Git inspection. It does not permit worktree creation:
/// print its native preview, then require `approve <exact preview_sha256>` and a
/// newline on stdin within 120 seconds (or `deny`). Original native admission,
/// expiry and grants remain mandatory. Only observed creation success transfers
/// the same live owner to the existing strict HTTP coordinator. No path adoption.
///
/// Each HTTP advance separately consents to reading/disclosing the configured
/// paths; every edit needs its own exact protected approval. No model dispatch
/// occurs at startup. No tests, verified completion, apply, commit, push, cleanup,
/// durable recovery or production activation is provided by this command.
pub fn serve_coding_endpoint_edit_development(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    launch_file: &Path,
) -> Result<(), ApiError> {
    serve_development(
        listener,
        tokens,
        stop,
        launch_file,
        CodingEndpointPreparedKind::Edit,
    )
}

/// Start the original strict edit/test/repair coordinator from a private launch
/// document with a fixed validation definition and the explicit combined catalog.
/// Launch inspection consent includes bounded whole-worktree reinspection
/// before the final summary, including hidden files, configuration and tests;
/// it permits no disclosure of those additional bytes. Original worktree
/// creation needs its exact stdin decision; every model disclosure, edit and test
/// needs separate HTTP approval. Native Git, command, worker and budget checks
/// remain mandatory. The development verifier refuses completion certification;
/// there is no apply, commit, push, cleanup or durable recovery.
pub fn serve_coding_endpoint_coding_development(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    launch_file: &Path,
) -> Result<(), ApiError> {
    serve_development(
        listener,
        tokens,
        stop,
        launch_file,
        CodingEndpointPreparedKind::EditAndTest,
    )
}

fn serve_development(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    launch_file: &Path,
    kind: CodingEndpointPreparedKind,
) -> Result<(), ApiError> {
    // The service checks listener and stop before invoking any launch factory.
    let secret = tokens.control.clone();
    let startup_stop = stop.clone();
    let factory = |run, session| {
        let mut launch = load(launch_file, kind)?;
        let startup = prepare(&launch, run, session, kind)?;
        // Freeze and check the private definition before opening state or
        // native inspection. Actual manifest/resource checks still run in
        // the original native owner at every test preparation and dispatch.
        let validation = launch
            .validation
            .take()
            .map(Validation::into_native)
            .transpose()?;
        let mut reader = ApprovalInput::new(io::stdin(), startup_stop.clone())?;
        let mut repository =
            LinuxCodingRepository::open(&launch.state_root, secret.as_bytes(), clock_ms()?)
                .map_err(|error| {
                    eprintln!("code.endpoint.edit-startup: {}", error.code());
                    ApiError::Configuration
                })?;
        let preview = repository
            .prepare(
                &launch.repository_root,
                &launch.owned_root,
                &format!("task-{}", &startup.request().request_sha256[..32]),
                &launch.base_commit,
                clock_ms()?,
            )
            .map_err(|error| {
                eprintln!("code.endpoint.edit-startup: {}", error.code());
                ApiError::Configuration
            })?;
        let digest = preview["preview_sha256"]
            .as_str()
            .ok_or(ApiError::Configuration)?;
        {
            let mut output = io::stdout().lock();
            writeln!(output, "{}", json!({"repository_root":launch.repository_root,"owned_root":launch.owned_root,"preview":preview}))
                    .map_err(|_| ApiError::Io)?;
            output.flush().map_err(|_| ApiError::Io)?;
        }
        eprintln!(
            "Review the native creation preview. Enter approve {digest} or deny. No later action is approved by this decision."
        );
        let approved = creation_decision(&mut reader, digest)?;
        if startup_stop.load(Ordering::Acquire) {
            return Err(ApiError::Configuration);
        }
        let result = repository
            .decide(digest, approved, clock_ms()?)
            .map_err(|error| {
                eprintln!("code.endpoint.edit-startup: {}", error.code());
                ApiError::Configuration
            })?;
        if !approved
            || result["outcome"] != "succeeded"
            || repository.created_workspace_id() != Some(&startup.request().workspace_id)
        {
            return Err(ApiError::Configuration);
        }
        drop(reader);
        // Keep credentials unread until original native creation succeeds.
        let endpoint = LocalChatEndpoint::parse(&startup.request().model_selection.endpoint)
            .map_err(|_| ApiError::Configuration)?;
        let credential = launch
            .credential_file
            .as_deref()
            .map(|path| {
                let bytes = private_bytes(path, 4096)?;
                BearerKey::new(
                    std::str::from_utf8(&bytes)
                        .map_err(|_| ApiError::Configuration)?
                        .to_owned(),
                )
                .map_err(|_| ApiError::Configuration)
            })
            .transpose()?;
        let connection = CodingEndpointConnection::Shared {
            approval: Arc::new(LocalChatApproval::for_endpoint(endpoint.clone())),
            transport: Arc::new(
                LocalChatTransport::new(endpoint, credential, TransportLimits::default())
                    .map_err(|_| ApiError::Configuration)?,
            ),
        };
        let control = Arc::new(RunControl {
            deadline: ProposalDispatchControl {
                started: Instant::now(),
                total: Duration::from_millis(startup.request().limits.max_elapsed_ms),
            },
            stop: startup_stop,
        });
        Ok((
            CodingEndpointStrictHttpRun {
                startup,
                repository,
                connection,
                control,
                clock: OsRuntimeClock,
                verifier: UnverifiedEdits(VerifierId::from_raw(
                    if kind == CodingEndpointPreparedKind::Edit {
                        "strict-edits-unverified"
                    } else {
                        "strict-coding-unverified"
                    },
                )),
                paths: launch.paths,
                source_budget: launch.source_budget,
                include_usage: true,
            },
            validation,
        ))
    };
    match kind {
        CodingEndpointPreparedKind::Edit => serve_coding_endpoint_strict_edits(
            listener,
            tokens,
            stop,
            CodingEndpointObservationAccess::ControlOnly,
            |run, session| factory(run, session).map(|(run, _)| run),
        ),
        CodingEndpointPreparedKind::EditAndTest => serve_coding_endpoint_edits_and_tests(
            listener,
            tokens,
            stop,
            CodingEndpointObservationAccess::ControlOnly,
            |run, session| {
                let (run, validation) = factory(run, session)?;
                Ok((run, validation.ok_or(ApiError::Configuration)?))
            },
        ),
        CodingEndpointPreparedKind::Command => Err(ApiError::Configuration),
    }
}

#[cfg(test)]
#[path = "coding_endpoint_edit_launch_tests.rs"]
mod tests;
