//! Explicit development-only process and private-state effects.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::ipc::LinuxLaunchEnvelope;

const MAX_DEVELOPMENT_HOST_BYTES: u64 = 512 * 1024 * 1024;

/// Stable content-free failure from the explicit Linux development boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxDevelopmentBoundaryErrorKind {
    /// A caller supplied a value outside the closed development contract.
    InvalidInput,
    /// The exact sibling host executable was absent, linked, or unsafe.
    UnsafeExecutable,
    /// The sibling host could not be launched.
    LaunchFailed,
    /// The direct one-use launch envelope could not be read.
    TransferFailed,
    /// The child could not be terminated or reaped.
    ProcessControlFailed,
    /// A private development directory was absent or unsafe.
    UnsafeDirectory,
    /// A bounded owner-only development record could not be retained.
    RetentionFailed,
}

/// Content-free Linux development boundary error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxDevelopmentBoundaryError {
    kind: LinuxDevelopmentBoundaryErrorKind,
}

impl LinuxDevelopmentBoundaryError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(&self) -> LinuxDevelopmentBoundaryErrorKind {
        self.kind
    }
}

/// Platform-owned handle for one exact sibling development host.
pub struct LinuxDevelopmentHostProcess {
    child: Child,
}

impl LinuxDevelopmentHostProcess {
    /// Launches the exact sibling host with the closed development operation.
    pub fn launch(
        state_root: &Path,
        disposable_root: &Path,
        workspace_root: &Path,
        scenario: &str,
        model: &str,
        resume: bool,
    ) -> Result<Self, LinuxDevelopmentBoundaryError> {
        if !matches!(
            scenario,
            "no-op"
                | "failed-test-repair"
                | "slow-cancel"
                | "restart-repair"
                | "new-file"
                | "multi-file"
                | "rollback"
                | "false-completion"
                | "overflow"
                | "disk-pressure"
                | "output-pressure"
        ) || !matches!(model, "scripted" | "muse" | "gpt-oss")
            || [state_root, disposable_root, workspace_root]
                .iter()
                .any(|path| !path.is_absolute())
        {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::InvalidInput,
            ));
        }
        let current = env::current_exe()
            .and_then(fs::canonicalize)
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable))?;
        let sibling = current.with_file_name("agentmage-host");
        let host = fs::canonicalize(&sibling)
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable))?;
        if host != sibling {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable,
            ));
        }
        let metadata = fs::symlink_metadata(&host)
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable))?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.mode() & 0o111 == 0
            || metadata.len() == 0
            || metadata.len() > MAX_DEVELOPMENT_HOST_BYTES
        {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::UnsafeExecutable,
            ));
        }
        let child = Command::new(host)
            .arg("--coding-development-host")
            .arg(state_root)
            .arg(disposable_root)
            .arg(workspace_root)
            .arg(scenario)
            .arg(model)
            .arg(if resume { "resume" } else { "new" })
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::LaunchFailed))?;
        Ok(Self { child })
    }

    /// Reads the one-use authenticated launch envelope from the child's direct pipe.
    pub fn read_launch_envelope(
        &mut self,
    ) -> Result<LinuxLaunchEnvelope, LinuxDevelopmentBoundaryError> {
        let output =
            self.child.stdout.as_mut().ok_or_else(|| {
                development_error(LinuxDevelopmentBoundaryErrorKind::TransferFailed)
            })?;
        LinuxLaunchEnvelope::read(output)
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::TransferFailed))
    }

    /// Requests termination of this exact owned child.
    pub fn terminate(&mut self) -> Result<(), LinuxDevelopmentBoundaryError> {
        self.child
            .kill()
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::ProcessControlFailed))
    }

    /// Reaps the exact owned child and returns whether it exited successfully.
    pub fn wait_success(&mut self) -> Result<bool, LinuxDevelopmentBoundaryError> {
        self.child
            .wait()
            .map(|status| status.success())
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::ProcessControlFailed))
    }
}

impl Drop for LinuxDevelopmentHostProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Creates or verifies one exact owner-only development directory.
pub fn ensure_private_development_directory(
    path: &Path,
) -> Result<(), LinuxDevelopmentBoundaryError> {
    if !path.is_absolute() {
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory,
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory))?;
    verify_private_directory(parent)?;
    match fs::create_dir(path) {
        Ok(()) => fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory))?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => {
            return Err(development_error(
                LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory,
            ));
        }
    }
    verify_private_directory(path)
}

/// Retains one rejected development-model response and its bounded metadata.
///
/// Both files are new owner-only ordinary files. A metadata failure removes only
/// the raw file created by this call; existing or substituted paths are never
/// overwritten or recursively removed.
pub fn retain_rejected_development_output(
    root: &Path,
    identity: &str,
    response: &[u8],
    metadata: &[u8],
) -> Result<(), LinuxDevelopmentBoundaryError> {
    ensure_private_development_directory(root)?;
    if identity.len() != 24 || !identity.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::InvalidInput,
        ));
    }
    let raw_path = root.join(format!("{identity}.response.bin"));
    let metadata_path = root.join(format!("{identity}.metadata.json"));
    write_private_new(&raw_path, response)?;
    if let Err(error) = write_private_new(&metadata_path, metadata) {
        let _ = fs::remove_file(&raw_path);
        return Err(error);
    }
    Ok(())
}

fn verify_private_directory(path: &Path) -> Result<(), LinuxDevelopmentBoundaryError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory))?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::UnsafeDirectory,
        ));
    }
    Ok(())
}

fn write_private_new(path: &Path, bytes: &[u8]) -> Result<(), LinuxDevelopmentBoundaryError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::RetentionFailed))?;
    if file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::RetentionFailed,
        ));
    }
    let metadata = file
        .metadata()
        .map_err(|_| development_error(LinuxDevelopmentBoundaryErrorKind::RetentionFailed))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() != bytes.len() as u64
    {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(development_error(
            LinuxDevelopmentBoundaryErrorKind::RetentionFailed,
        ));
    }
    Ok(())
}

const fn development_error(
    kind: LinuxDevelopmentBoundaryErrorKind,
) -> LinuxDevelopmentBoundaryError {
    LinuxDevelopmentBoundaryError { kind }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        LinuxDevelopmentBoundaryErrorKind, LinuxDevelopmentHostProcess,
        ensure_private_development_directory, retain_rejected_development_output,
    };

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn private_development_retention_is_new_owner_only_and_bounded() {
        let fixture = private_fixture();
        let retained = fixture.join("rejections");
        ensure_private_development_directory(&retained).expect("private retained root");
        retain_rejected_development_output(
            &retained,
            "0123456789abcdef01234567",
            b"raw-response",
            br#"{"disposition":"rejected"}"#,
        )
        .expect("retained pair");
        let raw = retained.join("0123456789abcdef01234567.response.bin");
        let metadata = retained.join("0123456789abcdef01234567.metadata.json");
        assert_eq!(fs::read(&raw).expect("raw"), b"raw-response");
        assert_eq!(
            fs::symlink_metadata(&raw).expect("raw metadata").mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::symlink_metadata(&metadata)
                .expect("record metadata")
                .mode()
                & 0o777,
            0o600
        );
        assert!(
            retain_rejected_development_output(
                &retained,
                "0123456789abcdef01234567",
                b"substitute",
                b"{}",
            )
            .is_err()
        );
        assert_eq!(fs::read(raw).expect("original remains"), b"raw-response");
        fs::remove_dir_all(fixture).expect("fixture cleanup");
    }

    #[test]
    fn development_host_rejects_open_ended_operation_before_launch() {
        let fixture = private_fixture();
        let error = LinuxDevelopmentHostProcess::launch(
            &fixture,
            &fixture,
            &fixture,
            "arbitrary",
            "scripted",
            false,
        )
        .err()
        .expect("closed scenario");
        assert_eq!(
            error.kind(),
            LinuxDevelopmentBoundaryErrorKind::InvalidInput
        );
        fs::remove_dir_all(fixture).expect("fixture cleanup");
    }

    fn private_fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agentmage-development-boundary-{}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("fixture root");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("fixture mode");
        path
    }
}
