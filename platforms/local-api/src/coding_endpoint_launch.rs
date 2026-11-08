// SPDX-License-Identifier: Apache-2.0
//! Explicit native read-only development activation of the retained endpoint owner.

use super::*;
use agentmage_host::{
    coding_development_activation::{
        CODING_DEVELOPMENT_ACTIVATION, CodingDevelopmentActivation, CodingDevelopmentKeyProvider,
    },
    coding_development_runtime::OsRuntimeClock,
    coding_endpoint::CodingEndpointConnection,
    coding_endpoint_files::CodingEndpointFiles,
    coding_endpoint_owner::CodingEndpointRuns,
    coding_endpoint_read_verifier::{CodingEndpointReadCheck, CodingEndpointReadVerifier},
    coding_endpoint_startup::{CodingEndpointStartInput, CodingEndpointStartup},
    linux_coding_runtime::OsCodingIdentitySource,
};
use agentmage_kernel_contracts::{
    ActorId, AdapterInstanceId, RuntimeEndpointRunRequest, RuntimeRunId, SessionId, TaskId,
    WorkPacketId, WorkspaceAuthorizationId, WorkspacePath,
};
use agentmage_kernel_engine::{
    instruction_provenance::InstructionEvidenceLedger,
    runtime_coordinator::{seal_runtime_endpoint_run_request, verify_runtime_endpoint_run_request},
};
use agentmage_platform_linux::{
    LinuxDevelopmentPlatformAdapter, LinuxSandboxLimits, LinuxSandboxManifest, LinuxSandboxRunner,
    LinuxWorkerRuntimeFile, open_linux_development_authority, select_development_linux_workspace,
};
use std::path::PathBuf;

const MAX_LAUNCH_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadCheck {
    path: Vec<String>,
    byte_len: u64,
    content_sha256: String,
}

// A trusted, private startup document, never an HTTP/model tool argument. It
// contains metadata only: no source contents or inline endpoint credentials.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Launch {
    version: u32,
    state_root: PathBuf,
    disposable_root: PathBuf,
    workspace_root: PathBuf,
    worker: PathBuf,
    credential_file: Option<PathBuf>,
    request: RuntimeEndpointRunRequest,
    instructions: InstructionEvidenceLedger,
    checks: Vec<ReadCheck>,
}

type Runs = CodingEndpointRuns<
    'static,
    CodingEndpointFiles<OsCodingIdentitySource>,
    CodingEndpointReadVerifier,
    OsRuntimeClock,
>;

// Hold the private canonical parent, refuse aliases/links/devices, bound actual
// reads as well as metadata, and erase temporary configuration/key buffers.
fn private_bytes(path: &Path, limit: u64) -> Result<Zeroizing<Vec<u8>>, ApiError> {
    let parent = path.parent().ok_or(ApiError::Configuration)?;
    if !path.is_absolute()
        || std::fs::canonicalize(parent).map_err(|_| ApiError::Configuration)? != parent
    {
        return Err(ApiError::Configuration);
    }
    let dir = protected_directory(parent)?;
    let file = File::from(
        openat(
            &dir,
            path.file_name().ok_or(ApiError::Configuration)?,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| ApiError::Configuration)?,
    );
    let meta = file.metadata().map_err(|_| ApiError::Io)?;
    if !meta.is_file()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o7777 != 0o600
        || meta.nlink() != 1
        || meta.len() == 0
        || meta.len() > limit
    {
        return Err(ApiError::Configuration);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ApiError::Io)?;
    if bytes.len() as u64 != meta.len() || bytes.len() as u64 > limit {
        return Err(ApiError::Configuration);
    }
    Ok(bytes)
}

fn load(path: &Path) -> Result<Launch, ApiError> {
    let bytes = private_bytes(path, MAX_LAUNCH_BYTES)?;
    let launch: Launch = object_json(&bytes).map_err(|_| ApiError::Configuration)?;
    let shape: Value = serde_json::from_slice(&bytes).map_err(|_| ApiError::Configuration)?;
    if !shape["request"].is_object()
        || !shape["instructions"].is_object()
        || !shape["checks"]
            .as_array()
            .is_some_and(|checks| checks.iter().all(Value::is_object))
    {
        return Err(ApiError::Configuration);
    }
    if launch.version != 1 {
        return Err(ApiError::Configuration);
    }
    verify_runtime_endpoint_run_request(&launch.request).map_err(|_| ApiError::Configuration)?;
    Ok(launch)
}

fn bind_identities(
    mut request: RuntimeEndpointRunRequest,
    run_id: RuntimeRunId,
    session_id: SessionId,
) -> Result<RuntimeEndpointRunRequest, ApiError> {
    // Validate the original seal before deliberate fresh-start identity binding.
    verify_runtime_endpoint_run_request(&request).map_err(|_| ApiError::Configuration)?;
    let task_id = TaskId::from_raw(format!("task-{}", run_id.as_str()));
    request.work_packet.work_packet_id =
        WorkPacketId::from_raw(format!("packet-{}", run_id.as_str()));
    request.work_packet.task_id = task_id.clone();
    request.task.task_id = task_id;
    request.task.session_id = session_id.clone();
    request.run_id = run_id;
    request.session_id = session_id;
    seal_runtime_endpoint_run_request(request).map_err(|_| ApiError::Configuration)
}

fn admit(
    launch: Launch,
    run_id: RuntimeRunId,
    session_id: SessionId,
    runs: &mut Runs,
) -> Result<RuntimeEndpointRunRequest, ApiError> {
    let activation = CodingDevelopmentActivation::validate(
        &launch.state_root,
        &launch.disposable_root,
        &launch.workspace_root,
    )
    .map_err(|_| ApiError::Configuration)?;
    let platform = LinuxDevelopmentPlatformAdapter::activate(
        CODING_DEVELOPMENT_ACTIVATION,
        activation.marker_sha256().to_owned(),
        AdapterInstanceId::from_raw(format!("adapter-{}", run_id.as_str())),
    )
    .map_err(|_| ApiError::Configuration)?;
    let workspace = Arc::new(
        select_development_linux_workspace(
            &platform,
            activation.workspace_root(),
            launch.request.workspace_id.clone(),
            WorkspaceAuthorizationId::from_raw(format!("workspace-{}", run_id.as_str())),
        )
        .map_err(|_| ApiError::Configuration)?,
    );
    let request = bind_identities(launch.request, run_id, session_id)?;
    let checks = launch
        .checks
        .into_iter()
        .map(|check| {
            let path = WorkspacePath::new(request.workspace_id.clone(), check.path)
                .map_err(|_| ApiError::Configuration)?;
            CodingEndpointReadCheck::new(path, check.byte_len, check.content_sha256)
                .map_err(|_| ApiError::Configuration)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let prepared = CodingEndpointStartup::prepare(
        request,
        workspace,
        ActorId::from_raw("local-endpoint-development-user"),
        &launch.instructions,
    )
    .map_err(|_| ApiError::Configuration)?;
    let verifier = prepared
        .read_verifier(checks)
        .map_err(|_| ApiError::Configuration)?;
    let endpoint = LocalChatEndpoint::parse(&prepared.request().model_selection.endpoint)
        .map_err(|_| ApiError::Configuration)?;

    // Preserve original root-owned launch/worker admission, including all
    // ancestry, descriptor, hash and offline seccomp checks. No fallback.
    let runtime_files = ["libgcc_s.so.1", "libc.so.6", "ld-linux-x86-64.so.2"]
        .into_iter()
        .map(|name| {
            Ok(LinuxWorkerRuntimeFile::new(
                std::fs::canonicalize(Path::new("/usr/lib64").join(name))
                    .map_err(|_| ApiError::Configuration)?,
                Path::new("/lib64").join(name),
            ))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let manifest = LinuxSandboxManifest::verify(
        "/usr/bin/systemd-run",
        "/usr/bin/bwrap",
        &launch.worker,
        &runtime_files,
    )
    .map_err(|error| {
        eprintln!("code.endpoint.native-admission: {error}");
        ApiError::Configuration
    })?;
    let sandbox = LinuxSandboxRunner::new(manifest, LinuxSandboxLimits::default())
        .map_err(|_| ApiError::Configuration)?;
    activation
        .revalidate()
        .map_err(|_| ApiError::Configuration)?;
    // Credentials and canonical state are opened only after native admission.
    let credential = launch
        .credential_file
        .as_deref()
        .map(|path| {
            let bytes = private_bytes(path, 4096)?;
            let key = std::str::from_utf8(&bytes).map_err(|_| ApiError::Configuration)?;
            BearerKey::new(key.to_owned()).map_err(|_| ApiError::Configuration)
        })
        .transpose()?;
    let connection = CodingEndpointConnection::Shared {
        approval: Arc::new(LocalChatApproval::for_endpoint(endpoint.clone())),
        transport: Arc::new(
            LocalChatTransport::new(endpoint, credential, TransportLimits::default())
                .map_err(|_| ApiError::Configuration)?,
        ),
    };
    let mut provider =
        CodingDevelopmentKeyProvider::open(&activation).map_err(|_| ApiError::Configuration)?;
    let authority = open_linux_development_authority(
        &platform,
        activation.state_root(),
        &mut provider,
        clock_ms()?,
    )
    .map_err(|_| ApiError::Configuration)?;
    activation
        .revalidate()
        .map_err(|_| ApiError::Configuration)?;
    prepared
        .admit(
            runs,
            CodingEndpointStartInput {
                connection,
                sandbox,
                authority,
                identities: OsCodingIdentitySource,
                verifier,
                clock: OsRuntimeClock,
                sources: &[],
                source_budget: 2048,
                include_usage: true,
            },
        )
        .map_err(|_| ApiError::Configuration)
}

/// Starts one explicitly approved disposable read-only development run. The
/// caller approves reading the private launch/credential files, selecting the
/// marked workspace and creating private development authority state. This is
/// not production activation, source/model disclosure consent or a tool grant.
/// Each model advance and exact native read still requires separate approval.
/// No initial source excerpts, arbitrary verifier or native model profile.
pub fn serve_coding_endpoint_read_development(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    launch_file: &Path,
) -> Result<(), ApiError> {
    coding_endpoint_api::serve_admitted_coding_endpoint(
        listener,
        tokens,
        stop,
        CodingEndpointObservationAccess::ControlOnly,
        |run_id, session_id, runs| admit(load(launch_file)?, run_id, session_id, runs),
    )
}

#[cfg(test)]
#[path = "coding_endpoint_launch_tests.rs"]
mod tests;
