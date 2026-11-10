// SPDX-License-Identifier: Apache-2.0
//! Offline execution of the original owner's already-approved local Git plans.

use super::*;
use crate::sandbox::supervision::{Launch, LaunchOwner, SupervisedResult};
use rustix::io::fcntl_dupfd_cloexec;

/// Closed supervisor owner. Paths and descriptors can only be captured here.
#[derive(Debug)]
pub(crate) struct LinuxRepositoryMutationScope {
    scope: LinuxRepositoryScope,
    roots: [OwnedFd; 3],
}

impl LinuxRepositoryMutationScope {
    fn capture(scope: &LinuxRepositoryScope) -> Result<Self, LinuxRepositoryError> {
        if scope.linked_read_only.is_some()
            || scope.git_directory == scope.checkout_root
            || !scope.git_directory.starts_with(&scope.checkout_root)
            || scope.owned_root.starts_with(&scope.checkout_root)
            || scope.checkout_root.starts_with(&scope.owned_root)
            || [
                &scope.checkout_root,
                &scope.git_directory,
                &scope.owned_root,
            ]
            .into_iter()
            .any(|p| !admissible_projection_path(p))
        {
            return Err(denied());
        }
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let owner = Self {
            scope: scope.clone(),
            roots: [
                open(&scope.checkout_root, flags, Mode::empty()).map_err(|_| denied())?,
                open(&scope.git_directory, flags, Mode::empty()).map_err(|_| denied())?,
                open(&scope.owned_root, flags, Mode::empty()).map_err(|_| denied())?,
            ],
        };
        owner.revalidate()?;
        Ok(owner)
    }

    pub(crate) fn revalidate(&self) -> Result<(), LinuxRepositoryError> {
        if path_identity(&self.scope.checkout_root)? != self.scope.repository_path_sha256 {
            return Err(denied());
        }
        for (path, fd) in [
            &self.scope.checkout_root,
            &self.scope.git_directory,
            &self.scope.owned_root,
        ]
        .into_iter()
        .zip(&self.roots)
        {
            if verify_owned_directory(path)? != *path {
                return Err(denied());
            }
            let held = fstat(fd).map_err(|_| denied())?;
            let current = fs::symlink_metadata(path).map_err(|_| denied())?;
            if held.st_dev != current.dev()
                || held.st_ino != current.ino()
                || held.st_uid != getuid().as_raw()
                || held.st_mode & 0o022 != 0
                || FileType::from_raw_mode(held.st_mode) != FileType::Directory
            {
                return Err(denied());
            }
        }
        Ok(())
    }

    fn projections(&self) -> Result<Vec<OwnedFd>, LinuxRepositoryError> {
        self.revalidate()?;
        self.roots
            .iter()
            .map(|fd| fcntl_dupfd_cloexec(fd, 3).map_err(|_| denied()))
            .collect()
    }
}

fn denied() -> LinuxRepositoryError {
    error(LinuxRepositoryErrorKind::PlanDenied)
}

// Persistent Git worktree pointers must name the same paths on host and guest.
// Refuse collisions with the worker's trusted runtime and private namespaces.
fn admissible_projection_path(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .skip(1)
            .all(|c| matches!(c, Component::Normal(_)))
        && path
            .to_str()
            .is_some_and(|s| !s.chars().any(char::is_control))
        && [
            "/usr", "/lib", "/lib64", "/etc", "/proc", "/dev", "/app", "/run", "/sys",
        ]
        .into_iter()
        .all(|reserved| !path.starts_with(reserved) && !Path::new(reserved).starts_with(path))
}

pub(super) fn execute(
    git: &LinuxGitArtifact,
    scope: &LinuxRepositoryScope,
    invocation: &HardenedGitInvocation,
    cancellation: &CancellationToken,
) -> InvocationStatus {
    execute_with(scope, invocation, cancellation, |owner, arguments| {
        run_worker(
            git,
            owner,
            arguments,
            &invocation.environment,
            None,
            cancellation,
            Instant::now() + PROCESS_TIMEOUT,
        )
    })
}

// The closed worker is shared by existing approved local effects and dormant
// exact candidate construction. No caller-selected program or host fallback.
fn run_worker(
    git: &LinuxGitArtifact,
    owner: Arc<LinuxRepositoryMutationScope>,
    arguments: &[String],
    environment: &std::collections::BTreeMap<String, String>,
    input: Option<OwnedFd>,
    cancellation: &CancellationToken,
    deadline: Instant,
) -> Result<SupervisedResult, InvocationStatus> {
    revalidate_git_artifact(git).map_err(|_| InvocationStatus::Denied)?;
    let manifest = Arc::new(
        LinuxRepositoryInspectionManifest::verify(
            "/usr/bin/systemd-run",
            "/usr/bin/systemctl",
            "/usr/bin/bwrap",
            git.launch_path(),
        )
        .map_err(|_| InvocationStatus::Denied)?,
    );
    let policy = compile_seccomp_policy().map_err(|_| InvocationStatus::Denied)?;
    crate::sandbox::supervision::admission_snapshot().map_err(supervision_failure)?;
    let mut nonce = [0_u8; 12];
    getrandom(&mut nonce, GetRandomFlags::empty()).map_err(|_| InvocationStatus::Denied)?;
    let unit = format!("agentmage-repository-git-{}.service", hex(&nonce));
    let projections = owner.projections().map_err(|_| InvocationStatus::Denied)?;
    let filter = crate::sandbox::supervision::FilterPipe::new().map_err(supervision_failure)?;
    let mut command = worker_command(&manifest, &owner, arguments, environment, &unit, &filter);
    if let Some(input) = input {
        command.stdin(Stdio::from(fs::File::from(input)));
    }
    owner.revalidate().map_err(|_| InvocationStatus::Denied)?;
    crate::sandbox::supervision::run_owned(
        command,
        Launch {
            owner: LaunchOwner::RepositoryGit(manifest, owner),
            projections,
            unit,
            deadline,
            stdout_limit: MAX_OBSERVATION_BYTES,
            stderr_limit: MAX_OBSERVATION_BYTES,
            filter_pipe: Some(filter),
        },
        &policy,
        &crate::sandbox::SandboxCancellation::Observed(cancellation),
    )
    .map_err(supervision_failure)
}

// Private deterministic seam: product dispatch above always uses the closed worker.
fn execute_with(
    scope: &LinuxRepositoryScope,
    invocation: &HardenedGitInvocation,
    cancellation: &CancellationToken,
    dispatch: impl FnOnce(
        Arc<LinuxRepositoryMutationScope>,
        &[String],
    ) -> Result<SupervisedResult, InvocationStatus>,
) -> InvocationStatus {
    if invocation.verify().is_err() || invocation.network || scope.linked_read_only.is_some() {
        return InvocationStatus::Denied;
    }
    if cancellation.is_cancelled() {
        return InvocationStatus::Cancelled;
    }
    let Some(arguments) = map_arguments(invocation, scope) else {
        return InvocationStatus::Denied;
    };
    let Ok(owner) = LinuxRepositoryMutationScope::capture(scope) else {
        return InvocationStatus::Denied;
    };
    let owner = Arc::new(owner);
    let result = dispatch(owner.clone(), &arguments);
    // A changed projection makes attribution uncertain, even on cancellation/error.
    if owner.revalidate().is_err() {
        return InvocationStatus::CleanupFailed;
    }
    let result = match result {
        Ok(r) => r,
        Err(status) => return status,
    };
    if !result.output_complete {
        return InvocationStatus::CleanupFailed;
    }
    let succeeded = matches!(result.outcome, Ok(OperationOutcome::Succeeded));
    match result.outcome {
        Ok(OperationOutcome::Cancelled) => InvocationStatus::Cancelled,
        Ok(OperationOutcome::TimedOut) => InvocationStatus::TimedOut,
        Err(e) if e.kind() == crate::sandbox::LinuxSandboxErrorKind::OutputLimitExceeded => {
            InvocationStatus::OutputLimit
        }
        Err(e) => supervision_failure(e),
        Ok(OperationOutcome::Succeeded | OperationOutcome::Failed) => {
            if cancellation.is_cancelled() {
                InvocationStatus::Cancelled
            } else if result.stdout.total > MAX_OBSERVATION_BYTES
                || result.stderr.total > MAX_OBSERVATION_BYTES
            {
                InvocationStatus::OutputLimit
            } else if succeeded && result.status.is_some_and(|s| s.success()) {
                InvocationStatus::Succeeded
            } else {
                InvocationStatus::Failed
            }
        }
        _ => InvocationStatus::Failed,
    }
}

fn supervision_failure(e: crate::sandbox::LinuxSandboxError) -> InvocationStatus {
    match e.kind() {
        crate::sandbox::LinuxSandboxErrorKind::CleanupUncertain
        | crate::sandbox::LinuxSandboxErrorKind::CancellationUnavailable => {
            InvocationStatus::CleanupFailed
        }
        _ => InvocationStatus::Failed,
    }
}

fn worker_command(
    manifest: &LinuxRepositoryInspectionManifest,
    owner: &LinuxRepositoryMutationScope,
    arguments: &[String],
    environment: &std::collections::BTreeMap<String, String>,
    unit: &str,
    filter: &crate::sandbox::supervision::FilterPipe,
) -> Command {
    let pid = std::process::id();
    let runtime = format!("/run/user/{}", getuid().as_raw());
    let mut c = Command::new(&manifest.systemd_run.launch_path);
    c.env_clear()
        .env("XDG_RUNTIME_DIR", &runtime)
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={runtime}/bus"),
        )
        .args([
            "--user",
            "--wait",
            "--quiet",
            "--pipe",
            "--collect",
            "--expand-environment=no",
        ])
        .arg(format!("--unit={unit}"))
        .args([
            "--property=NoNewPrivileges=yes",
            "--property=RestrictSUIDSGID=yes",
            "--property=LockPersonality=yes",
            "--property=RestrictAddressFamilies=AF_UNIX AF_NETLINK",
            "--property=MemorySwapMax=0",
            "--property=MemoryMax=268435456",
            "--property=TasksMax=16",
            "--property=CPUQuota=100%",
            "--property=RuntimeMaxSec=31s",
            "--property=Restart=no",
            "--property=JobRunningTimeoutSec=30s",
            "--property=TimeoutStartSec=30s",
            "--property=KillMode=control-group",
            "--property=SendSIGKILL=yes",
            "--property=TimeoutStopSec=2s",
        ]);
    // systemd allocates 3:Git, 4:filter, 5:checkout, 6:metadata, 7:owned root.
    for (fd, name) in [
        (manifest.git.descriptor.as_raw_fd(), "git"),
        (filter.read_descriptor(), "seccomp"),
        (owner.roots[0].as_raw_fd(), "checkout"),
        (owner.roots[1].as_raw_fd(), "metadata"),
        (owner.roots[2].as_raw_fd(), "owned"),
    ] {
        c.arg(format!(
            "--property=OpenFile=/proc/{pid}/fd/{fd}:{name}:read-only"
        ));
    }
    c.arg(&manifest.bubblewrap.launch_path).args([
        "--unshare-all",
        "--unshare-user",
        "--disable-userns",
        "--new-session",
        "--die-with-parent",
        "--clearenv",
    ]);
    for (key, value) in environment {
        c.arg("--setenv").arg(key).arg(value);
    }
    c.args([
        "--cap-drop",
        "ALL",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/app",
        "--dir",
        "/usr",
        "--dir",
        "/etc",
    ]);
    add_inspection_runtime_mounts(&mut c);
    c.args(["--ro-bind-fd", "3", GUEST_GIT_EXECUTABLE]);
    // Installed Git helpers can be symlinks back to /usr/bin/git. Worktree
    // creation invokes built-in subcommands through those helpers. Link their target
    // to the verified guest executable; bind-fd consumes its source descriptor.
    // Never expose the host bin directory.
    c.args(["--symlink", GUEST_GIT_EXECUTABLE, "/usr/bin/git"]);
    // Never mount a host ancestor. Bubblewrap creates empty guest parents.
    c.args(["--ro-bind-fd", "5"])
        .arg(&owner.scope.checkout_root)
        .args(["--bind-fd", "6"])
        .arg(&owner.scope.git_directory)
        .args(["--bind-fd", "7"])
        .arg(&owner.scope.owned_root)
        .arg("--chdir")
        .arg(&owner.scope.owned_root)
        .args([
            "--seccomp",
            "4",
            "--",
            GUEST_GIT_EXECUTABLE,
            "--no-replace-objects",
        ])
        .arg(format!(
            "--work-tree={}",
            owner.scope.checkout_root.display()
        ))
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}

#[cfg(test)]
#[path = "repository_execution_tests.rs"]
mod tests;

#[path = "repository_candidate.rs"]
mod candidate;
pub(crate) use candidate::CandidateSession;

pub(crate) use candidate::index_path as candidate_index_path;
