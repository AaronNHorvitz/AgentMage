//! Exact `llama-server` process and private Unix-socket transport driver.

use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, DecodingProfile, EncodedModelContext, ExactModelProfile,
    ModelCancellationProbe, ModelHealth, ModelHealthState, ModelLoadReceipt,
    ModelManifestObservation, ModelProfileId, ModelResourceReport, ModelRunRequest, ModelRunResult,
    ModelRunTerminalState, ModelRuntimeFailure, ModelRuntimeIdentity, ModelStreamId,
    ModelStreamSink, ModelUnloadReceipt, RuntimeIsolationObservation, StreamedModelFragment,
    TokenCountResult,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::NativeModelDriver;

const MAX_HTTP_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_HEALTH_RESPONSE_BYTES: usize = 4 * 1024;
const MAX_COMPLETION_BYTES: usize = 16 * 1024 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 32 * 1024;
const MAX_SSE_EVENT_BYTES: usize = 1024 * 1024;
const STREAM_POLL_INTERVAL: Duration = Duration::from_millis(50);
const EXPECTED_CONTEXT_TOKENS: u32 = 8192;
const MAX_UNIX_SOCKET_PATH_BYTES: usize = 107;
const BWRAP_PATH: &str = "/usr/bin/bwrap";
const BWRAP_SHA256: &str = "139bf12775025adf5c8523d119c5ad2950281335573708fd839c60181a3886dc";
const NVIDIA_SMI_PATH: &str = "/usr/bin/nvidia-smi";
const NVIDIA_SMI_SHA256: &str = "915f6e333651d7bf03252e605743ae1d5cf1587d85f436a25aa5ff6c462cd982";
const GUEST_RUNTIME_ROOT: &str = "/runtime";
const GUEST_MODEL_PATH: &str = "/model/model.gguf";
const GUEST_SOCKET_ROOT: &str = "/run/agentmage";
const GUEST_SOCKET_PATH: &str = "/run/agentmage/llama-server.sock";
const SANDBOX_READ_ONLY_DIRECTORIES: &[&str] = &[
    "/usr/lib64",
    "/usr/share/vulkan",
    "/usr/share/glvnd",
    "/usr/share/nvidia",
    "/etc/vulkan",
    "/etc/glvnd",
    "/etc/nvidia",
    "/sys",
    "/proc/driver/nvidia",
];
const SANDBOX_DEVICE_PATHS: &[&str] = &[
    "/dev/dri",
    "/dev/nvidia0",
    "/dev/nvidiactl",
    "/dev/nvidia-uvm",
    "/dev/nvidia-uvm-tools",
    "/dev/nvidia-modeset",
    "/dev/nvidia-caps",
];
const EXPECTED_SERVER_SHA256: &str =
    "f7c93f0de9fed7b596e68557bd4d42a084204027f87d61c893777908e4c62bfe";

const RUNTIME_FILES: &[(&str, u64, &str)] = &[
    ("bin/llama-server", 17_896, EXPECTED_SERVER_SHA256),
    (
        "lib/libggml-base.so.0.19.0",
        910_816,
        "c09ef1afbe033faba844ddaa41bfdd02b359036df9d7cbc94c558896d2cfa431",
    ),
    (
        "lib/libggml-cpu-x64.so",
        878_128,
        "6234b3b90a03d1b27d5ca12e303e290b7060e319ffc206d828d0bfe1dee24350",
    ),
    (
        "lib/libggml-vulkan.so",
        52_926_320,
        "5dc6de442289a361d10b703a8e96dca0aefd826fedc5a84060ff34647c538f29",
    ),
    (
        "lib/libggml.so.0.19.0",
        54_936,
        "ffd6a736ad58e2d9407ed427252520ffbd4bca8dd5de467ecb2bd446a7a3d75b",
    ),
    (
        "lib/libllama-common.so.0.1.0",
        6_250_560,
        "4a3a21572819b3d19bb2aeec098c90aa0a04bbd2b47a1f2a77e4be17f0c3dc37",
    ),
    (
        "lib/libllama-server-impl.so",
        7_557_544,
        "0cb631d0e6d558a15605653aac3fcaa4d58b8c6f7ea382b300e18341a8034830",
    ),
    (
        "lib/libllama.so.0.1.0",
        4_220_304,
        "46d42c6f97f1ad8d1225ba43c9f475be910d0dccdc2326277a0e647e3dade4f7",
    ),
    (
        "lib/libmtmd.so.0.1.0",
        1_796_912,
        "7602cc169c768fad2bea4a4263804087f1867db167471d36ee6315942119adbf",
    ),
];

const RUNTIME_LINKS: &[(&str, &str)] = &[
    ("lib/libggml-base.so", "libggml-base.so.0"),
    ("lib/libggml-base.so.0", "libggml-base.so.0.19.0"),
    ("lib/libggml.so", "libggml.so.0"),
    ("lib/libggml.so.0", "libggml.so.0.19.0"),
    ("lib/libllama-common.so", "libllama-common.so.0"),
    ("lib/libllama-common.so.0", "libllama-common.so.0.1.0"),
    ("lib/libllama.so", "libllama.so.0"),
    ("lib/libllama.so.0", "libllama.so.0.1.0"),
    ("lib/libmtmd.so", "libmtmd.so.0"),
    ("lib/libmtmd.so.0", "libmtmd.so.0.1.0"),
];

/// Exact platform-owned paths and immutable runtime identity.
pub struct LlamaServerDriverConfig {
    runtime_root: PathBuf,
    model_path: PathBuf,
    socket_path: PathBuf,
    identity: ModelRuntimeIdentity,
    startup_timeout: Duration,
}

impl LlamaServerDriverConfig {
    /// Creates a path-bound configuration. Paths are verified again before load.
    pub fn new(
        runtime_root: PathBuf,
        model_path: PathBuf,
        socket_path: PathBuf,
        identity: ModelRuntimeIdentity,
        startup_timeout: Duration,
    ) -> Result<Self, ModelRuntimeFailure> {
        if !runtime_root.is_absolute()
            || !model_path.is_absolute()
            || !socket_path.is_absolute()
            || startup_timeout.is_zero()
            || startup_timeout > Duration::from_secs(600)
            || !valid_socket_path(&socket_path)
        {
            return Err(failure("model.llama-driver.configuration-invalid", false));
        }
        Ok(Self {
            runtime_root,
            model_path,
            socket_path,
            identity,
            startup_timeout,
        })
    }
}

fn valid_socket_path(path: &Path) -> bool {
    path.as_os_str().as_bytes().len() <= MAX_UNIX_SOCKET_PATH_BYTES
        && path.file_name().and_then(|name| name.to_str()) == Some("llama-server.sock")
}

struct LoadedRuntime {
    profile_id: ModelProfileId,
    manifest_sha256: String,
    token_counter: String,
    decoding: DecodingProfile,
    child: Child,
    runtime_pid: u32,
    loaded_at: Instant,
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileSnapshot {
    device: u64,
    inode: u64,
    mode: u32,
    owner: u32,
    group: u32,
    links: u64,
    bytes: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl FileSnapshot {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            owner: metadata.uid(),
            group: metadata.gid(),
            links: metadata.nlink(),
            bytes: metadata.size(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VerifiedModel {
    profile_id: ModelProfileId,
    manifest_sha256: String,
    artifact_sha256: String,
    snapshot: FileSnapshot,
}

/// Exact native `llama-server` driver used behind `LinuxNativeModelAdapter`.
pub struct LlamaServerDriver {
    config: LlamaServerDriverConfig,
    loaded: Option<LoadedRuntime>,
    verified_model: RefCell<Option<VerifiedModel>>,
}

impl LlamaServerDriver {
    /// Creates an unloaded driver without touching the runtime or model store.
    #[must_use]
    pub const fn new(config: LlamaServerDriverConfig) -> Self {
        Self {
            config,
            loaded: None,
            verified_model: RefCell::new(None),
        }
    }

    fn verify_runtime_tree(&self) -> Result<(), ModelRuntimeFailure> {
        exact_directory(&self.config.runtime_root, 0o555)?;
        for (relative, bytes, digest) in RUNTIME_FILES {
            exact_file(&self.config.runtime_root.join(relative), *bytes, digest)?;
        }
        for (relative, target) in RUNTIME_LINKS {
            let path = self.config.runtime_root.join(relative);
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| failure("model.llama-driver.runtime-link-missing", false))?;
            if !metadata.file_type().is_symlink()
                || fs::read_link(&path).ok().as_deref() != Some(Path::new(target))
            {
                return Err(failure("model.llama-driver.runtime-link-drift", false));
            }
        }
        Ok(())
    }

    fn verify_sandbox_dependencies(&self) -> Result<(), ModelRuntimeFailure> {
        exact_root_owned_file(Path::new(BWRAP_PATH), BWRAP_SHA256)?;
        exact_root_owned_file(Path::new(NVIDIA_SMI_PATH), NVIDIA_SMI_SHA256)?;
        for path in SANDBOX_READ_ONLY_DIRECTORIES {
            exact_root_owned_directory(Path::new(path))?;
        }
        for path in SANDBOX_DEVICE_PATHS {
            exact_root_owned_device_or_directory(Path::new(path))?;
        }
        Ok(())
    }

    fn verify_model(&self, profile: &ExactModelProfile) -> Result<(), ModelRuntimeFailure> {
        let metadata = fs::symlink_metadata(&self.config.model_path)
            .map_err(|_| failure("model.llama-driver.file-unavailable", false))?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.nlink() != 1
            || metadata.len() != profile.artifact.bytes
            || metadata.mode() & 0o777 != 0o600
        {
            return Err(failure("model.llama-driver.file-identity", false));
        }
        let parent = self
            .config
            .model_path
            .parent()
            .ok_or_else(|| failure("model.llama-driver.model-parent-invalid", false))?;
        let metadata = fs::symlink_metadata(parent)
            .map_err(|_| failure("model.llama-driver.model-parent-invalid", false))?;
        if !metadata.is_dir() || metadata.mode() & 0o077 != 0 {
            return Err(failure("model.llama-driver.model-parent-public", false));
        }
        let before = FileSnapshot::from_metadata(
            &fs::symlink_metadata(&self.config.model_path)
                .map_err(|_| failure("model.llama-driver.file-unavailable", false))?,
        );
        if self.verified_model.borrow().as_ref()
            == Some(&VerifiedModel {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                artifact_sha256: profile.artifact.sha256.clone(),
                snapshot: before,
            })
        {
            return Ok(());
        }
        if sha256_file(&self.config.model_path)? != profile.artifact.sha256 {
            *self.verified_model.borrow_mut() = None;
            return Err(failure("model.llama-driver.file-identity", false));
        }
        let after = FileSnapshot::from_metadata(
            &fs::symlink_metadata(&self.config.model_path)
                .map_err(|_| failure("model.llama-driver.file-unavailable", false))?,
        );
        if before != after {
            *self.verified_model.borrow_mut() = None;
            return Err(failure(
                "model.llama-driver.file-changed-during-hash",
                false,
            ));
        }
        *self.verified_model.borrow_mut() = Some(VerifiedModel {
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            artifact_sha256: profile.artifact.sha256.clone(),
            snapshot: after,
        });
        Ok(())
    }

    fn client(&self) -> UnixHttpClient {
        UnixHttpClient::new(self.config.socket_path.clone())
    }

    fn wait_until_ready(&mut self) -> Result<(), ModelRuntimeFailure> {
        let deadline = Instant::now() + self.config.startup_timeout;
        loop {
            if self
                .loaded
                .as_mut()
                .and_then(|loaded| loaded.child.try_wait().ok().flatten())
                .is_some()
            {
                return Err(failure("model.llama-driver.process-exited", true));
            }
            if self.client().health().is_ok() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(failure("model.llama-driver.startup-timeout", true));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn stop_loaded(&mut self) -> Result<(ModelProfileId, u64), ModelRuntimeFailure> {
        let Some(mut loaded) = self.loaded.take() else {
            return Err(failure("model.llama-driver.not-loaded", false));
        };
        let started = Instant::now();
        let _ = loaded.child.kill();
        loaded
            .child
            .wait()
            .map_err(|_| failure("model.llama-driver.process-reap-failed", true))?;
        if loaded.runtime_pid != 0 {
            let deadline = Instant::now() + Duration::from_secs(2);
            while Path::new(&format!("/proc/{}", loaded.runtime_pid)).exists()
                && Instant::now() < deadline
            {
                thread::sleep(Duration::from_millis(10));
            }
            if Path::new(&format!("/proc/{}", loaded.runtime_pid)).exists() {
                return Err(failure("model.llama-driver.descendant-reap-failed", true));
            }
        }
        if self.config.socket_path.exists() {
            fs::remove_file(&self.config.socket_path)
                .map_err(|_| failure("model.llama-driver.socket-cleanup-failed", true))?;
        }
        let elapsed = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        Ok((loaded.profile_id, elapsed))
    }
}

impl Drop for LlamaServerDriver {
    fn drop(&mut self) {
        if let Some(loaded) = self.loaded.as_mut() {
            let _ = loaded.child.kill();
            let _ = loaded.child.wait();
        }
        if self.loaded.is_some() && self.config.socket_path.exists() {
            let _ = fs::remove_file(&self.config.socket_path);
        }
    }
}

impl NativeModelDriver for LlamaServerDriver {
    fn verify_manifest(
        &self,
        profile: &ExactModelProfile,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
        if self.loaded.is_some()
            || profile.runtime != self.config.identity
            || profile.context.max_context_tokens != EXPECTED_CONTEXT_TOKENS
            || profile.modalities != [agentmage_kernel_contracts::ModelModality::Text]
        {
            return Err(failure("model.llama-driver.profile-invalid", false));
        }
        self.verify_runtime_tree()?;
        self.verify_model(profile)?;
        Ok(ModelManifestObservation {
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            artifact_sha256: profile.artifact.sha256.clone(),
            tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
            template_sha256: profile.codec.template_sha256.clone(),
            codec_sha256: profile.codec.codec_sha256.clone(),
            runtime: self.config.identity.clone(),
        })
    }

    fn load(
        &mut self,
        profile: &ExactModelProfile,
        isolation: &RuntimeIsolationObservation,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
        let started = Instant::now();
        self.verify_manifest(profile)?;
        let socket_parent = self
            .config
            .socket_path
            .parent()
            .ok_or_else(|| failure("model.llama-driver.socket-parent-invalid", false))?;
        exact_directory(socket_parent, 0o700)?;
        if self.config.socket_path.exists() {
            return Err(failure("model.llama-driver.socket-exists", false));
        }
        self.verify_sandbox_dependencies()?;
        let mut command = Command::new(BWRAP_PATH);
        command
            .args(sandbox_arguments(
                &self.config.runtime_root,
                &self.config.model_path,
                socket_parent,
            ))
            .arg("--")
            .arg(format!("{GUEST_RUNTIME_ROOT}/bin/llama-server"))
            .args(launch_arguments(
                GUEST_MODEL_PATH,
                profile.profile_id.as_str(),
                GUEST_SOCKET_PATH,
            ))
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command
            .spawn()
            .map_err(|_| failure("model.llama-driver.process-start-failed", true))?;
        self.loaded = Some(LoadedRuntime {
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            token_counter: profile.context.token_counter.clone(),
            decoding: profile.decoding.clone(),
            child,
            runtime_pid: 0,
            loaded_at: Instant::now(),
            input_tokens: 0,
            output_tokens: 0,
        });
        if let Err(error) = self.wait_until_ready() {
            let _ = self.stop_loaded();
            return Err(error);
        }
        let finalized = (|| {
            fs::set_permissions(&self.config.socket_path, fs::Permissions::from_mode(0o600))
                .map_err(|_| failure("model.llama-driver.socket-mode-failed", true))?;
            let supervisor_pid = self
                .loaded
                .as_ref()
                .expect("loaded state retained")
                .child
                .id();
            let runtime_pid = exact_runtime_descendant(supervisor_pid)?;
            verify_live_sandbox(runtime_pid, &self.config.socket_path)?;
            Ok(runtime_pid)
        })();
        let runtime_pid = match finalized {
            Ok(runtime_pid) => runtime_pid,
            Err(error) => {
                let _ = self.stop_loaded();
                return Err(error);
            }
        };
        self.loaded
            .as_mut()
            .expect("loaded state retained")
            .runtime_pid = runtime_pid;
        Ok(ModelLoadReceipt {
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            adapter_id: self.config.identity.adapter_id.clone(),
            elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            isolation: isolation.clone(),
        })
    }

    fn unload(
        &mut self,
        profile_id: &ModelProfileId,
    ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure> {
        if self.loaded.as_ref().map(|loaded| &loaded.profile_id) != Some(profile_id) {
            return Err(failure("model.llama-driver.profile-mismatch", false));
        }
        let (unloaded, elapsed_ms) = self.stop_loaded()?;
        Ok(ModelUnloadReceipt {
            profile_id: unloaded,
            adapter_id: self.config.identity.adapter_id.clone(),
            empty: true,
            elapsed_ms,
        })
    }

    fn health(&self) -> ModelHealth {
        let (profile_id, state, reason) = match self.loaded.as_ref() {
            None => (
                None,
                ModelHealthState::Unloaded,
                "model.llama-driver.unloaded",
            ),
            Some(loaded) if self.client().health().is_ok() => (
                Some(loaded.profile_id.clone()),
                ModelHealthState::Ready,
                "model.llama-driver.ready",
            ),
            Some(loaded) => (
                Some(loaded.profile_id.clone()),
                ModelHealthState::Failed,
                "model.llama-driver.health-failed",
            ),
        };
        ModelHealth {
            adapter_id: self.config.identity.adapter_id.clone(),
            profile_id,
            state,
            reason_code: reason.to_owned(),
            observed_at_ms: self
                .loaded
                .as_ref()
                .map_or(0, |loaded| loaded.loaded_at.elapsed().as_millis() as u64),
        }
    }

    fn count_tokens(
        &self,
        context: &EncodedModelContext,
    ) -> Result<TokenCountResult, ModelRuntimeFailure> {
        let loaded = self
            .loaded
            .as_ref()
            .ok_or_else(|| failure("model.llama-driver.not-loaded", false))?;
        if loaded.profile_id != context.profile_id {
            return Err(failure(
                "model.llama-driver.context-profile-mismatch",
                false,
            ));
        }
        let tokens = self.client().token_count(&context.bytes)?;
        Ok(TokenCountResult {
            profile_id: context.profile_id.clone(),
            context_packet_id: context.context_packet_id.clone(),
            tokens,
            counter: loaded.token_counter.clone(),
            packet_sha256: context.sha256.clone(),
        })
    }

    fn stream(
        &mut self,
        request: &ModelRunRequest,
        context: &EncodedModelContext,
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelRunResult, ModelRuntimeFailure> {
        let loaded = self
            .loaded
            .as_ref()
            .ok_or_else(|| failure("model.llama-driver.not-loaded", false))?;
        if loaded.profile_id != request.profile_id
            || loaded.profile_id != context.profile_id
            || loaded.manifest_sha256 != request.manifest_sha256
            || loaded.decoding.profile_id != request.decoding_profile_id
        {
            return Err(failure("model.llama-driver.request-mismatch", false));
        }
        let started = Instant::now();
        let stream_id =
            ModelStreamId::from_raw(format!("stream:{}", request.model_run_id.as_str()));
        let completion = self.client().completion_stream(
            &context.bytes,
            request,
            &loaded.decoding,
            cancellation,
            &stream_id,
            sink,
        )?;
        let response_sha256 = sha256(&completion.bytes);
        let loaded = self.loaded.as_mut().expect("loaded state retained");
        loaded.input_tokens = loaded.input_tokens.saturating_add(0);
        loaded.output_tokens = loaded.output_tokens.saturating_add(completion.tokens);
        let resident_memory_bytes = resident_memory_bytes(loaded.runtime_pid);
        let accelerator_memory_bytes = accelerator_memory_bytes(loaded.runtime_pid)?;
        Ok(ModelRunResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: request.model_run_id.clone(),
            stream_id,
            correlation_id: request.correlation_id.clone(),
            terminal_state: completion.terminal_state,
            fragment_count: completion.fragments,
            response_sha256,
            proposal: None,
            failure: completion.failure,
            resources: ModelResourceReport {
                adapter_id: self.config.identity.adapter_id.clone(),
                profile_id: request.profile_id.clone(),
                model_run_id: Some(request.model_run_id.clone()),
                resident_memory_bytes,
                accelerator_memory_bytes,
                input_tokens: loaded.input_tokens,
                output_tokens: completion.tokens,
                elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            },
        })
    }

    fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure> {
        let loaded = self
            .loaded
            .as_ref()
            .ok_or_else(|| failure("model.llama-driver.not-loaded", false))?;
        let accelerator_memory_bytes = accelerator_memory_bytes(loaded.runtime_pid)?;
        Ok(ModelResourceReport {
            adapter_id: self.config.identity.adapter_id.clone(),
            profile_id: loaded.profile_id.clone(),
            model_run_id: None,
            resident_memory_bytes: resident_memory_bytes(loaded.runtime_pid),
            accelerator_memory_bytes,
            input_tokens: loaded.input_tokens,
            output_tokens: loaded.output_tokens,
            elapsed_ms: loaded
                .loaded_at
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        })
    }
}

fn launch_arguments<'a>(model: &'a str, profile_id: &'a str, socket: &'a str) -> [&'a str; 16] {
    [
        "--model",
        model,
        "--alias",
        profile_id,
        "--host",
        socket,
        "--ctx-size",
        "8192",
        "--parallel",
        "1",
        "--n-gpu-layers",
        "999",
        "--no-webui",
        "--no-slots",
        "--jinja",
        "--no-context-shift",
    ]
}

fn sandbox_arguments(runtime_root: &Path, model_path: &Path, socket_root: &Path) -> Vec<OsString> {
    let mut arguments = [
        "--unshare-all",
        "--unshare-user",
        "--disable-userns",
        "--new-session",
        "--die-with-parent",
        "--clearenv",
        "--cap-drop",
        "ALL",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--dir",
        GUEST_RUNTIME_ROOT,
        "--dir",
        "/model",
        "--dir",
        "/run",
        "--dir",
        GUEST_SOCKET_ROOT,
        "--dir",
        "/usr",
        "--dir",
        "/usr/share",
        "--dir",
        "/etc",
        "--dir",
        "/proc/driver",
        "--ro-bind",
    ]
    .into_iter()
    .map(OsString::from)
    .collect::<Vec<_>>();
    arguments.push(runtime_root.as_os_str().to_owned());
    arguments.push(OsString::from(GUEST_RUNTIME_ROOT));
    arguments.push(OsString::from("--ro-bind"));
    arguments.push(model_path.as_os_str().to_owned());
    arguments.push(OsString::from(GUEST_MODEL_PATH));
    arguments.push(OsString::from("--bind"));
    arguments.push(socket_root.as_os_str().to_owned());
    arguments.push(OsString::from(GUEST_SOCKET_ROOT));
    for path in SANDBOX_READ_ONLY_DIRECTORIES {
        arguments.push(OsString::from("--ro-bind"));
        arguments.push(OsString::from(path));
        arguments.push(OsString::from(path));
    }
    arguments.extend(["--symlink", "usr/lib64", "/lib64"].map(OsString::from));
    for path in SANDBOX_DEVICE_PATHS {
        arguments.push(OsString::from("--dev-bind"));
        arguments.push(OsString::from(path));
        arguments.push(OsString::from(path));
    }
    for (name, value) in [
        ("LD_LIBRARY_PATH", "/runtime/lib:/usr/lib64"),
        ("PATH", "/runtime/bin"),
        ("HOME", "/nonexistent"),
        ("TMPDIR", "/tmp"),
        ("XDG_RUNTIME_DIR", "/tmp"),
        ("LANG", "C"),
    ] {
        arguments.extend(["--setenv", name, value].map(OsString::from));
    }
    arguments.extend(["--chdir", "/runtime/lib"].map(OsString::from));
    arguments
}

struct Completion {
    bytes: Vec<u8>,
    tokens: u32,
    fragments: u32,
    terminal_state: ModelRunTerminalState,
    failure: Option<ModelRuntimeFailure>,
}

#[derive(Clone, Copy)]
enum Endpoint {
    Health,
    Tokenize,
}

impl Endpoint {
    const fn method(self) -> &'static str {
        match self {
            Self::Health => "GET",
            Self::Tokenize => "POST",
        }
    }

    const fn path(self) -> &'static str {
        match self {
            Self::Health => "/health",
            Self::Tokenize => "/tokenize",
        }
    }
}

struct UnixHttpClient {
    socket_path: PathBuf,
}

impl UnixHttpClient {
    const fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    fn health(&self) -> Result<(), ModelRuntimeFailure> {
        let body = self.request(Endpoint::Health, None, MAX_HEALTH_RESPONSE_BYTES, 1_000)?;
        let value: Value = serde_json::from_slice(&body)
            .map_err(|_| failure("model.llama-driver.health-invalid", true))?;
        if value.get("status").and_then(Value::as_str) != Some("ok") {
            return Err(failure("model.llama-driver.health-not-ready", true));
        }
        Ok(())
    }

    fn token_count(&self, context: &[u8]) -> Result<u32, ModelRuntimeFailure> {
        let content = std::str::from_utf8(context)
            .map_err(|_| failure("model.llama-driver.context-not-utf8", false))?;
        let body = serde_json::to_vec(&json!({
            "content": content,
            "add_special": false,
            "parse_special": true,
            "with_pieces": false
        }))
        .map_err(|_| failure("model.llama-driver.token-request-invalid", false))?;
        let response = self.request(
            Endpoint::Tokenize,
            Some(&body),
            MAX_HTTP_RESPONSE_BYTES,
            30_000,
        )?;
        let value: Value = serde_json::from_slice(&response)
            .map_err(|_| failure("model.llama-driver.token-response-invalid", true))?;
        let count = value
            .get("tokens")
            .and_then(Value::as_array)
            .ok_or_else(|| failure("model.llama-driver.token-response-invalid", true))?
            .len();
        u32::try_from(count).map_err(|_| failure("model.llama-driver.token-count-overflow", false))
    }

    fn completion_stream(
        &self,
        context: &[u8],
        request: &ModelRunRequest,
        decoding: &DecodingProfile,
        cancellation: Option<&dyn ModelCancellationProbe>,
        stream_id: &ModelStreamId,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<Completion, ModelRuntimeFailure> {
        let prompt = std::str::from_utf8(context)
            .map_err(|_| failure("model.llama-driver.context-not-utf8", false))?;
        let body = serde_json::to_vec(&json!({
            "prompt": prompt,
            "n_predict": request.max_output_tokens,
            "stream": true,
            "cache_prompt": false,
            "return_tokens": true,
            "return_progress": false,
            "sse_ping_interval": 1,
            "temperature": decoding.temperature,
            "top_p": decoding.top_p,
            "top_k": decoding.top_k,
            "repeat_penalty": decoding.repeat_penalty,
            "seed": decoding.seed,
            "samplers": decoding.sampler_order,
            "stop": ["<|eot|>"],
            "id_slot": 0
        }))
        .map_err(|_| failure("model.llama-driver.completion-request-invalid", false))?;
        let mut state = SseCompletionState::new(request, stream_id, sink);
        if cancellation_requested(cancellation, request)? {
            return state.interrupted(ModelRunTerminalState::Cancelled);
        }
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| failure("model.llama-driver.socket-connect-failed", true))?;
        stream
            .set_read_timeout(Some(STREAM_POLL_INTERVAL))
            .and_then(|()| {
                stream.set_write_timeout(Some(Duration::from_millis(
                    request.timeout_ms.clamp(1, 3_600_000),
                )))
            })
            .map_err(|_| failure("model.llama-driver.socket-timeout-failed", true))?;
        let header = format!(
            "POST /completion HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        stream
            .write_all(header.as_bytes())
            .and_then(|()| stream.write_all(&body))
            .map_err(|_| failure("model.llama-driver.http-write-failed", true))?;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(request.timeout_ms))
            .ok_or_else(|| failure("model.llama-driver.deadline-invalid", false))?;
        let mut reader = PollingUnixReader::new(stream, deadline, cancellation, request);
        match read_streaming_response(&mut reader, &mut state, request.max_output_tokens) {
            Ok(()) => state.complete(),
            Err(StreamReadError::Cancelled) => state.interrupted(ModelRunTerminalState::Cancelled),
            Err(StreamReadError::TimedOut) => state.interrupted(ModelRunTerminalState::TimedOut),
            Err(StreamReadError::Failed(error)) => Err(error),
        }
    }

    fn request(
        &self,
        endpoint: Endpoint,
        body: Option<&[u8]>,
        maximum: usize,
        timeout_ms: u64,
    ) -> Result<Vec<u8>, ModelRuntimeFailure> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .map_err(|_| failure("model.llama-driver.socket-connect-failed", true))?;
        let timeout = Duration::from_millis(timeout_ms.clamp(1, 3_600_000));
        stream
            .set_read_timeout(Some(timeout))
            .and_then(|()| stream.set_write_timeout(Some(timeout)))
            .map_err(|_| failure("model.llama-driver.socket-timeout-failed", true))?;
        let payload = body.unwrap_or_default();
        let header = format!(
            "{} {} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            endpoint.method(),
            endpoint.path(),
            payload.len()
        );
        stream
            .write_all(header.as_bytes())
            .and_then(|()| stream.write_all(payload))
            .map_err(|_| failure("model.llama-driver.http-write-failed", true))?;
        let mut response = Vec::new();
        stream
            .take(u64::try_from(maximum + 1).expect("bounded response limit"))
            .read_to_end(&mut response)
            .map_err(|_| failure("model.llama-driver.http-read-failed", true))?;
        if response.len() > maximum {
            return Err(failure("model.llama-driver.http-response-oversized", false));
        }
        parse_http_response(&response)
    }
}

enum StreamReadError {
    Cancelled,
    TimedOut,
    Failed(ModelRuntimeFailure),
}

struct PollingUnixReader<'a> {
    stream: UnixStream,
    pending: Vec<u8>,
    offset: usize,
    deadline: Instant,
    cancellation: Option<&'a dyn ModelCancellationProbe>,
    request: &'a ModelRunRequest,
}

impl<'a> PollingUnixReader<'a> {
    fn new(
        stream: UnixStream,
        deadline: Instant,
        cancellation: Option<&'a dyn ModelCancellationProbe>,
        request: &'a ModelRunRequest,
    ) -> Self {
        Self {
            stream,
            pending: Vec::new(),
            offset: 0,
            deadline,
            cancellation,
            request,
        }
    }

    fn read_until(&mut self, delimiter: &[u8], maximum: usize) -> Result<Vec<u8>, StreamReadError> {
        let mut output = Vec::new();
        while !output.ends_with(delimiter) {
            if output.len() >= maximum {
                return Err(StreamReadError::Failed(failure(
                    "model.llama-driver.http-field-oversized",
                    false,
                )));
            }
            output.push(self.read_byte()?);
        }
        Ok(output)
    }

    fn read_line(&mut self, maximum: usize) -> Result<Vec<u8>, StreamReadError> {
        let mut line = self.read_until(b"\r\n", maximum.saturating_add(2))?;
        line.truncate(line.len() - 2);
        Ok(line)
    }

    fn read_exact_bytes(&mut self, length: usize) -> Result<Vec<u8>, StreamReadError> {
        let mut output = Vec::with_capacity(length);
        while output.len() < length {
            output.push(self.read_byte()?);
        }
        Ok(output)
    }

    fn read_byte(&mut self) -> Result<u8, StreamReadError> {
        loop {
            if self.offset < self.pending.len() {
                let byte = self.pending[self.offset];
                self.offset += 1;
                return Ok(byte);
            }
            self.pending.clear();
            self.offset = 0;
            self.check_stop()?;
            let mut buffer = [0_u8; 8192];
            match self.stream.read(&mut buffer) {
                Ok(0) => {
                    return Err(StreamReadError::Failed(failure(
                        "model.llama-driver.http-unexpected-eof",
                        true,
                    )));
                }
                Ok(length) => self.pending.extend_from_slice(&buffer[..length]),
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    continue;
                }
                Err(_) => {
                    return Err(StreamReadError::Failed(failure(
                        "model.llama-driver.http-read-failed",
                        true,
                    )));
                }
            }
        }
    }

    fn check_stop(&self) -> Result<(), StreamReadError> {
        if Instant::now() >= self.deadline {
            return Err(StreamReadError::TimedOut);
        }
        match cancellation_requested(self.cancellation, self.request) {
            Ok(true) => Err(StreamReadError::Cancelled),
            Ok(false) => Ok(()),
            Err(error) => Err(StreamReadError::Failed(error)),
        }
    }
}

struct SseCompletionState<'a> {
    request: ModelRunRequest,
    stream_id: ModelStreamId,
    sink: &'a mut dyn ModelStreamSink,
    event_buffer: Vec<u8>,
    bytes: Vec<u8>,
    tokens: u32,
    fragments: u32,
    stop_seen: bool,
    done_seen: bool,
}

impl<'a> SseCompletionState<'a> {
    fn new(
        request: &ModelRunRequest,
        stream_id: &ModelStreamId,
        sink: &'a mut dyn ModelStreamSink,
    ) -> Self {
        Self {
            request: request.clone(),
            stream_id: stream_id.clone(),
            sink,
            event_buffer: Vec::new(),
            bytes: Vec::new(),
            tokens: 0,
            fragments: 0,
            stop_seen: false,
            done_seen: false,
        }
    }

    fn feed(&mut self, bytes: &[u8], max_output_tokens: u32) -> Result<(), ModelRuntimeFailure> {
        if self.event_buffer.len().saturating_add(bytes.len()) > MAX_SSE_EVENT_BYTES {
            return Err(failure("model.llama-driver.sse-event-oversized", false));
        }
        self.event_buffer.extend_from_slice(bytes);
        while let Some((end, delimiter_bytes)) = next_sse_event(&self.event_buffer) {
            let mut remainder = self.event_buffer.split_off(end + delimiter_bytes);
            let event = self.event_buffer[..end].to_vec();
            std::mem::swap(&mut self.event_buffer, &mut remainder);
            self.process_event(&event, max_output_tokens)?;
        }
        Ok(())
    }

    fn process_event(
        &mut self,
        event: &[u8],
        max_output_tokens: u32,
    ) -> Result<(), ModelRuntimeFailure> {
        let lines = event
            .split(|byte| *byte == b'\n')
            .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        if !lines.is_empty() && lines.iter().all(|line| line.starts_with(b":")) {
            return Ok(());
        }
        if lines.len() != 1 || !lines[0].starts_with(b"data: ") {
            return Err(failure("model.llama-driver.sse-event-invalid", false));
        }
        let data = &lines[0][6..];
        if data == b"[DONE]" {
            if !self.stop_seen || self.done_seen {
                return Err(failure("model.llama-driver.sse-done-invalid", false));
            }
            self.done_seen = true;
            return Ok(());
        }
        if self.stop_seen || self.done_seen {
            return Err(failure("model.llama-driver.sse-event-after-stop", false));
        }
        let value: Value = serde_json::from_slice(data)
            .map_err(|_| failure("model.llama-driver.sse-json-invalid", false))?;
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| failure("model.llama-driver.sse-content-invalid", false))?;
        let tokens = value
            .get("tokens")
            .and_then(Value::as_array)
            .ok_or_else(|| failure("model.llama-driver.sse-tokens-invalid", false))?;
        if tokens.iter().any(|token| {
            token
                .as_u64()
                .is_none_or(|token| token > u64::from(u32::MAX))
        }) {
            return Err(failure("model.llama-driver.sse-tokens-invalid", false));
        }
        let token_count = u32::try_from(tokens.len())
            .map_err(|_| failure("model.llama-driver.sse-token-overflow", false))?;
        self.tokens = self
            .tokens
            .checked_add(token_count)
            .filter(|tokens| *tokens <= max_output_tokens)
            .ok_or_else(|| failure("model.llama-driver.completion-token-limit", false))?;
        let terminal = value
            .get("stop")
            .and_then(Value::as_bool)
            .ok_or_else(|| failure("model.llama-driver.sse-stop-invalid", false))?;
        if terminal
            && value
                .get("stop_type")
                .and_then(Value::as_str)
                .is_some_and(|kind| !matches!(kind, "eos" | "limit" | "word"))
        {
            return Err(failure("model.llama-driver.sse-stop-type-invalid", false));
        }
        let content = content.as_bytes();
        if self.bytes.len().saturating_add(content.len()) > MAX_COMPLETION_BYTES {
            return Err(failure("model.llama-driver.completion-size-invalid", false));
        }
        self.bytes.extend_from_slice(content);
        if !content.is_empty() || terminal {
            self.emit(content.to_vec(), terminal)?;
        }
        self.stop_seen = terminal;
        Ok(())
    }

    fn emit(&mut self, bytes: Vec<u8>, terminal: bool) -> Result<(), ModelRuntimeFailure> {
        let sequence = self.fragments;
        self.fragments = self
            .fragments
            .checked_add(1)
            .ok_or_else(|| failure("model.llama-driver.fragment-overflow", false))?;
        self.sink.accept(StreamedModelFragment {
            schema_version: CONTRACT_SCHEMA_VERSION,
            stream_id: self.stream_id.clone(),
            model_run_id: self.request.model_run_id.clone(),
            correlation_id: self.request.correlation_id.clone(),
            sequence,
            sha256: sha256(&bytes),
            bytes,
            terminal,
        })
    }

    fn interrupted(
        mut self,
        terminal_state: ModelRunTerminalState,
    ) -> Result<Completion, ModelRuntimeFailure> {
        if self.stop_seen {
            return Err(failure(
                "model.llama-driver.sse-interrupted-after-stop",
                false,
            ));
        }
        self.emit(Vec::new(), true)?;
        let code = match terminal_state {
            ModelRunTerminalState::Cancelled => "model.llama-driver.cancelled",
            ModelRunTerminalState::TimedOut => "model.llama-driver.timed-out",
            _ => return Err(failure("model.llama-driver.interruption-invalid", false)),
        };
        Ok(Completion {
            bytes: self.bytes,
            tokens: self.tokens,
            fragments: self.fragments,
            terminal_state,
            failure: Some(failure(code, false)),
        })
    }

    fn complete(self) -> Result<Completion, ModelRuntimeFailure> {
        for (failed, code) in [
            (
                !self.event_buffer.is_empty(),
                "model.llama-driver.sse-partial-event",
            ),
            (!self.stop_seen, "model.llama-driver.sse-stop-missing"),
            (
                self.fragments == 0,
                "model.llama-driver.sse-fragments-missing",
            ),
            (
                self.bytes.is_empty(),
                "model.llama-driver.sse-content-missing",
            ),
        ] {
            if failed {
                return Err(failure(code, false));
            }
        }
        let (terminal_state, failure) = match self
            .bytes
            .iter()
            .copied()
            .find(|byte| !byte.is_ascii_whitespace())
        {
            Some(b'{') | Some(b'[') => (ModelRunTerminalState::Proposed, None),
            _ if plain_text(&self.bytes) => (ModelRunTerminalState::AdvisoryText, None),
            _ => (
                ModelRunTerminalState::Rejected,
                Some(failure("model.llama-driver.completion-rejected", false)),
            ),
        };
        Ok(Completion {
            bytes: self.bytes,
            tokens: self.tokens,
            fragments: self.fragments,
            terminal_state,
            failure,
        })
    }
}

fn next_sse_event(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|position| (position, 2));
    let crlf = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| (position, 4));
    match (lf, crlf) {
        (Some(left), Some(right)) => Some(if left.0 <= right.0 { left } else { right }),
        (Some(found), None) | (None, Some(found)) => Some(found),
        (None, None) => None,
    }
}

fn cancellation_requested(
    cancellation: Option<&dyn ModelCancellationProbe>,
    request: &ModelRunRequest,
) -> Result<bool, ModelRuntimeFailure> {
    let signal = cancellation
        .map(ModelCancellationProbe::observe)
        .transpose()?
        .flatten();
    if signal
        .as_ref()
        .is_some_and(|signal| signal.correlation_id != request.correlation_id)
    {
        return Err(failure("model.llama-driver.cancellation-mismatch", false));
    }
    Ok(signal.is_some())
}

fn read_streaming_response(
    reader: &mut PollingUnixReader<'_>,
    state: &mut SseCompletionState<'_>,
    max_output_tokens: u32,
) -> Result<(), StreamReadError> {
    let headers = reader.read_until(b"\r\n\r\n", MAX_HTTP_HEADER_BYTES)?;
    validate_streaming_headers(&headers[..headers.len() - 4]).map_err(StreamReadError::Failed)?;
    let mut decoded_bytes = 0_usize;
    loop {
        let line = reader.read_line(32)?;
        if line.is_empty() || line.contains(&b';') {
            return Err(StreamReadError::Failed(failure(
                "model.llama-driver.http-chunk-size-invalid",
                false,
            )));
        }
        let line = std::str::from_utf8(&line).map_err(|_| {
            StreamReadError::Failed(failure("model.llama-driver.http-chunk-size-invalid", false))
        })?;
        let length = usize::from_str_radix(line, 16).map_err(|_| {
            StreamReadError::Failed(failure("model.llama-driver.http-chunk-size-invalid", false))
        })?;
        if length == 0 {
            if !reader.read_line(MAX_HTTP_HEADER_BYTES)?.is_empty() {
                return Err(StreamReadError::Failed(failure(
                    "model.llama-driver.http-trailer-prohibited",
                    false,
                )));
            }
            break;
        }
        decoded_bytes = decoded_bytes.checked_add(length).ok_or_else(|| {
            StreamReadError::Failed(failure("model.llama-driver.http-response-oversized", false))
        })?;
        if decoded_bytes > MAX_HTTP_RESPONSE_BYTES {
            return Err(StreamReadError::Failed(failure(
                "model.llama-driver.http-response-oversized",
                false,
            )));
        }
        let chunk = reader.read_exact_bytes(length)?;
        if reader.read_exact_bytes(2)? != b"\r\n" {
            return Err(StreamReadError::Failed(failure(
                "model.llama-driver.http-chunk-framing-invalid",
                false,
            )));
        }
        state
            .feed(&chunk, max_output_tokens)
            .map_err(StreamReadError::Failed)?;
        reader.check_stop()?;
    }
    Ok(())
}

fn validate_streaming_headers(headers: &[u8]) -> Result<(), ModelRuntimeFailure> {
    let headers = std::str::from_utf8(headers)
        .map_err(|_| failure("model.llama-driver.http-malformed", true))?;
    let mut lines = headers.split("\r\n");
    if lines.next() != Some("HTTP/1.1 200 OK") {
        return Err(failure("model.llama-driver.http-status", true));
    }
    let mut chunked = false;
    let mut event_stream = false;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(failure("model.llama-driver.http-malformed", true));
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("content-length") {
            return Err(failure("model.llama-driver.http-stream-length", false));
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            if chunked || !value.eq_ignore_ascii_case("chunked") {
                return Err(failure("model.llama-driver.http-transfer-encoding", false));
            }
            chunked = true;
        }
        if name.eq_ignore_ascii_case("content-type") {
            if event_stream
                || !value
                    .split(';')
                    .next()
                    .is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
            {
                return Err(failure("model.llama-driver.http-content-type", false));
            }
            event_stream = true;
        }
    }
    if !chunked || !event_stream {
        return Err(failure("model.llama-driver.http-stream-contract", false));
    }
    Ok(())
}

fn parse_http_response(response: &[u8]) -> Result<Vec<u8>, ModelRuntimeFailure> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| failure("model.llama-driver.http-malformed", true))?;
    let headers = std::str::from_utf8(&response[..split])
        .map_err(|_| failure("model.llama-driver.http-malformed", true))?;
    let mut lines = headers.split("\r\n");
    if lines.next() != Some("HTTP/1.1 200 OK") {
        return Err(failure("model.llama-driver.http-status", true));
    }
    let mut content_length = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(failure("model.llama-driver.http-malformed", true));
        };
        if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(failure("model.llama-driver.http-transfer-encoding", false));
        }
        if name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                return Err(failure("model.llama-driver.http-length-duplicate", false));
            }
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let body = &response[split + 4..];
    if content_length != Some(body.len()) {
        return Err(failure("model.llama-driver.http-length-mismatch", false));
    }
    Ok(body.to_vec())
}

fn exact_root_owned_file(path: &Path, digest: &str) -> Result<(), ModelRuntimeFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| failure("model.llama-driver.sandbox-file-unavailable", false))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != 0
        || metadata.nlink() != 1
        || metadata.mode() & 0o022 != 0
        || sha256_file(path)? != digest
    {
        return Err(failure("model.llama-driver.sandbox-file-identity", false));
    }
    Ok(())
}

fn exact_root_owned_directory(path: &Path) -> Result<(), ModelRuntimeFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| failure("model.llama-driver.sandbox-directory-unavailable", false))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != 0
        || metadata.mode() & 0o022 != 0
    {
        return Err(failure(
            "model.llama-driver.sandbox-directory-identity",
            false,
        ));
    }
    Ok(())
}

fn exact_root_owned_device_or_directory(path: &Path) -> Result<(), ModelRuntimeFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| failure("model.llama-driver.sandbox-device-unavailable", false))?;
    if metadata.file_type().is_symlink()
        || metadata.uid() != 0
        || (!metadata.is_dir() && !metadata.file_type().is_char_device())
    {
        return Err(failure("model.llama-driver.sandbox-device-identity", false));
    }
    Ok(())
}

fn process_parent(pid: u32) -> Option<u32> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, suffix) = stat.rsplit_once(") ")?;
    suffix.split_whitespace().nth(1)?.parse().ok()
}

fn descendant_of(mut pid: u32, ancestor: u32) -> bool {
    for _ in 0..32 {
        let Some(parent) = process_parent(pid) else {
            return false;
        };
        if parent == ancestor {
            return true;
        }
        if parent <= 1 || parent == pid {
            return false;
        }
        pid = parent;
    }
    false
}

fn exact_runtime_descendant(supervisor_pid: u32) -> Result<u32, ModelRuntimeFailure> {
    let mut matches = Vec::new();
    let entries = fs::read_dir("/proc")
        .map_err(|_| failure("model.llama-driver.process-observation-failed", true))?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        if !descendant_of(pid, supervisor_pid) {
            continue;
        }
        if fs::read_link(format!("/proc/{pid}/exe")).ok().as_deref()
            == Some(Path::new("/runtime/bin/llama-server"))
        {
            matches.push(pid);
        }
    }
    if matches.len() != 1 {
        return Err(failure("model.llama-driver.runtime-process-identity", true));
    }
    Ok(matches[0])
}

fn status_value<'a>(status: &'a str, key: &str) -> Option<&'a str> {
    status
        .lines()
        .find_map(|line| line.strip_prefix(key).map(str::trim))
}

fn verify_live_sandbox(pid: u32, socket_path: &Path) -> Result<(), ModelRuntimeFailure> {
    let namespace = |process: &str, name: &str| fs::read_link(format!("/proc/{process}/ns/{name}"));
    for name in ["mnt", "net", "pid", "user", "ipc", "uts"] {
        let runtime = namespace(&pid.to_string(), name)
            .map_err(|_| failure("model.llama-driver.namespace-unavailable", true))?;
        let current = namespace("self", name)
            .map_err(|_| failure("model.llama-driver.namespace-unavailable", true))?;
        if runtime == current {
            return Err(failure("model.llama-driver.namespace-not-isolated", false));
        }
    }
    let status = fs::read_to_string(format!("/proc/{pid}/status"))
        .map_err(|_| failure("model.llama-driver.status-unavailable", true))?;
    if status_value(&status, "NoNewPrivs:") != Some("1")
        || status_value(&status, "CapEff:") != Some("0000000000000000")
    {
        return Err(failure(
            "model.llama-driver.privilege-isolation-failed",
            false,
        ));
    }
    let environment = fs::read(format!("/proc/{pid}/environ"))
        .map_err(|_| failure("model.llama-driver.environment-unavailable", true))?;
    let mut variables = environment
        .split(|byte| *byte == 0)
        .filter(|value| !value.is_empty())
        .map(Vec::from)
        .collect::<Vec<_>>();
    variables.sort();
    let mut expected = [
        b"HOME=/nonexistent".to_vec(),
        b"LANG=C".to_vec(),
        b"LD_LIBRARY_PATH=/runtime/lib:/usr/lib64".to_vec(),
        b"PATH=/runtime/bin".to_vec(),
        b"PWD=/runtime/lib".to_vec(),
        b"TMPDIR=/tmp".to_vec(),
        b"XDG_RUNTIME_DIR=/tmp".to_vec(),
    ];
    expected.sort();
    if variables != expected.to_vec() {
        return Err(failure("model.llama-driver.environment-not-closed", false));
    }
    for prohibited in ["home", "var/home", "root", "workspace"] {
        if Path::new(&format!("/proc/{pid}/root/{prohibited}")).exists() {
            return Err(failure("model.llama-driver.host-path-visible", false));
        }
    }
    let interfaces = fs::read_to_string(format!("/proc/{pid}/net/dev"))
        .map_err(|_| failure("model.llama-driver.network-observation-failed", true))?;
    let names = interfaces
        .lines()
        .skip(2)
        .filter_map(|line| line.split_once(':').map(|(name, _)| name.trim()))
        .collect::<Vec<_>>();
    if !names.is_empty() && names != ["lo"] {
        return Err(failure(
            "model.llama-driver.network-interface-visible",
            false,
        ));
    }
    let socket = fs::symlink_metadata(socket_path)
        .map_err(|_| failure("model.llama-driver.socket-unavailable", true))?;
    if !socket.file_type().is_socket() || socket.mode() & 0o777 != 0o600 || socket.nlink() != 1 {
        return Err(failure("model.llama-driver.socket-identity-invalid", false));
    }
    Ok(())
}

fn exact_directory(path: &Path, mode: u32) -> Result<fs::Metadata, ModelRuntimeFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| failure("model.llama-driver.directory-unavailable", false))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() || metadata.mode() & 0o777 != mode {
        return Err(failure("model.llama-driver.directory-invalid", false));
    }
    Ok(metadata)
}

fn exact_file(path: &Path, bytes: u64, digest: &str) -> Result<(), ModelRuntimeFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| failure("model.llama-driver.file-unavailable", false))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.len() != bytes
        || sha256_file(path)? != digest
    {
        return Err(failure("model.llama-driver.file-identity", false));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, ModelRuntimeFailure> {
    let mut stream =
        fs::File::open(path).map_err(|_| failure("model.llama-driver.file-unavailable", false))?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 4 * 1024 * 1024];
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|_| failure("model.llama-driver.file-read-failed", false))?;
        if count == 0 {
            return Ok(lowercase_hex(&digest.finalize()));
        }
        digest.update(&buffer[..count]);
    }
}

fn sha256(bytes: &[u8]) -> String {
    lowercase_hex(&Sha256::digest(bytes))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn plain_text(bytes: &[u8]) -> bool {
    if bytes.len() > 16 * 1024 {
        return false;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    !text.is_empty()
        && text
            .chars()
            .all(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        && !matches!(
            text.trim_start().as_bytes().first(),
            Some(b'{') | Some(b'[')
        )
}

fn resident_memory_bytes(pid: u32) -> u64 {
    fs::read_to_string(format!("/proc/{pid}/statm"))
        .ok()
        .and_then(|value| value.split_whitespace().nth(1)?.parse::<u64>().ok())
        .map_or(0, |pages| pages.saturating_mul(4096))
}

fn accelerator_memory_bytes(pid: u32) -> Result<u64, ModelRuntimeFailure> {
    let output = Command::new(NVIDIA_SMI_PATH)
        .args([
            "--query-compute-apps=pid,used_gpu_memory",
            "--format=csv,noheader,nounits",
        ])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .output()
        .map_err(|_| failure("model.llama-driver.accelerator-observation-failed", true))?;
    if !output.status.success() || !output.stderr.is_empty() || output.stdout.len() > 64 * 1024 {
        return Err(failure(
            "model.llama-driver.accelerator-observation-invalid",
            true,
        ));
    }
    parse_accelerator_memory(&output.stdout, pid)
}

fn parse_accelerator_memory(bytes: &[u8], pid: u32) -> Result<u64, ModelRuntimeFailure> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| failure("model.llama-driver.accelerator-observation-invalid", true))?;
    let mut matches = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let Some((candidate, memory)) = line.split_once(',') else {
            return Err(failure(
                "model.llama-driver.accelerator-observation-invalid",
                true,
            ));
        };
        let candidate = candidate
            .trim()
            .parse::<u32>()
            .map_err(|_| failure("model.llama-driver.accelerator-observation-invalid", true))?;
        let memory = memory
            .trim()
            .parse::<u64>()
            .map_err(|_| failure("model.llama-driver.accelerator-observation-invalid", true))?;
        if candidate == pid {
            matches.push(memory);
        }
    }
    if matches.len() != 1 || matches[0] == 0 {
        return Err(failure(
            "model.llama-driver.accelerator-process-missing",
            true,
        ));
    }
    matches[0]
        .checked_mul(1024 * 1024)
        .ok_or_else(|| failure("model.llama-driver.accelerator-memory-overflow", false))
}

fn failure(code: &str, dependency: bool) -> ModelRuntimeFailure {
    ModelRuntimeFailure {
        code: code.to_owned(),
        retryable_after_correction: false,
        dependency_recovery_required: dependency,
        contract_error: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    use agentmage_kernel_contracts::{
        BoundaryKind, CancellationId, CancellationReason, CancellationSignal, ContextPacketId,
        CorrelationId, DecodingProfile, ModelAdapterId, ModelCancellationProbe, ModelProfileId,
        ModelRunId, ModelRunRequest, ModelRunTerminalState, ModelRuntimeFailure, ModelStreamId,
        ModelStreamSink, StreamedModelFragment, TaskId,
    };
    use serde_json::{Value, json};

    use super::{
        CONTRACT_SCHEMA_VERSION, Endpoint, FileSnapshot, GUEST_MODEL_PATH, GUEST_RUNTIME_ROOT,
        GUEST_SOCKET_PATH, GUEST_SOCKET_ROOT, SANDBOX_DEVICE_PATHS, SANDBOX_READ_ONLY_DIRECTORIES,
        UnixHttpClient, exact_directory, launch_arguments, parse_accelerator_memory,
        parse_http_response, plain_text, sandbox_arguments, valid_socket_path,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-llama-http-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create unique test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn exchange<T>(
        response: Vec<u8>,
        operation: impl FnOnce(&UnixHttpClient) -> T,
    ) -> (T, Vec<u8>) {
        let directory = TestDirectory::new();
        let socket = directory.0.join("llama-server.sock");
        let listener = UnixListener::bind(&socket).expect("listener");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = vec![0_u8; 64 * 1024];
            let count = stream.read(&mut request).expect("request");
            stream.write_all(&response).expect("response");
            request.truncate(count);
            request
        });
        let client = UnixHttpClient::new(socket.clone());
        let value = operation(&client);
        let request = server.join().expect("server");
        fs::remove_file(socket).ok();
        (value, request)
    }

    fn exchange_parts<T>(
        parts: Vec<(Vec<u8>, Duration)>,
        operation: impl FnOnce(&UnixHttpClient) -> T,
    ) -> (T, Vec<u8>) {
        let directory = TestDirectory::new();
        let socket = directory.0.join("llama-server.sock");
        let listener = UnixListener::bind(&socket).expect("listener");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = vec![0_u8; 64 * 1024];
            let count = stream.read(&mut request).expect("request");
            for (part, delay) in parts {
                if stream.write_all(&part).is_err() {
                    break;
                }
                thread::sleep(delay);
            }
            request.truncate(count);
            request
        });
        let client = UnixHttpClient::new(socket.clone());
        let value = operation(&client);
        let request = server.join().expect("server");
        fs::remove_file(socket).ok();
        (value, request)
    }

    fn response(value: serde_json::Value) -> Vec<u8> {
        let body = serde_json::to_vec(&value).expect("JSON");
        [
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )
            .into_bytes(),
            body,
        ]
        .concat()
    }

    fn streaming_response(events: &[Vec<u8>]) -> Vec<u8> {
        let mut response =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n"
                .to_vec();
        for event in events {
            response.extend_from_slice(format!("{:x}\r\n", event.len()).as_bytes());
            response.extend_from_slice(event);
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"0\r\n\r\n");
        response
    }

    fn streaming_headers() -> Vec<u8> {
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n"
            .to_vec()
    }

    fn chunk(bytes: &[u8]) -> Vec<u8> {
        [
            format!("{:x}\r\n", bytes.len()).into_bytes(),
            bytes.to_vec(),
            b"\r\n".to_vec(),
        ]
        .concat()
    }

    fn sse(value: Value) -> Vec<u8> {
        let value = serde_json::to_vec(&value).expect("SSE JSON");
        [b"data: ".as_slice(), value.as_slice(), b"\n\n".as_slice()].concat()
    }

    fn run_request() -> ModelRunRequest {
        ModelRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: ModelRunId::from_raw("run-1"),
            correlation_id: CorrelationId::from_raw("correlation-1"),
            context_packet_id: ContextPacketId::from_raw("context-1"),
            profile_id: ModelProfileId::from_raw("profile-1"),
            manifest_sha256: "a".repeat(64),
            adapter_id: ModelAdapterId::from_raw("adapter-1"),
            decoding_profile_id: "diagnostic-repeatability-v1".to_owned(),
            max_output_tokens: 8,
            timeout_ms: 1_000,
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        fragments: Vec<StreamedModelFragment>,
    }

    impl ModelStreamSink for RecordingSink {
        fn accept(&mut self, fragment: StreamedModelFragment) -> Result<(), ModelRuntimeFailure> {
            self.fragments.push(fragment);
            Ok(())
        }
    }

    struct DelayedCancellation {
        observations: AtomicUsize,
        cancel_at: usize,
        signal: CancellationSignal,
    }

    impl ModelCancellationProbe for DelayedCancellation {
        fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
            let observation = self.observations.fetch_add(1, Ordering::SeqCst) + 1;
            Ok((observation >= self.cancel_at).then(|| self.signal.clone()))
        }
    }

    fn cancellation_probe(cancel_at: usize) -> DelayedCancellation {
        DelayedCancellation {
            observations: AtomicUsize::new(0),
            cancel_at,
            signal: CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("cancel-1"),
                correlation_id: CorrelationId::from_raw("correlation-1"),
                task_id: TaskId::from_raw("task-1"),
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Shell,
            },
        }
    }

    #[test]
    fn exact_health_and_tokenize_requests_use_only_closed_endpoints() {
        let ((), request) = exchange(response(json!({"status": "ok"})), |client| {
            client.health().expect("health");
        });
        assert!(request.starts_with(b"GET /health HTTP/1.1\r\n"));
        let (value, request) = exchange(response(json!({"tokens": [1, 2, 3]})), |client| {
            client
                .request(Endpoint::Tokenize, Some(b"{}"), 4096, 1000)
                .expect("tokenize")
        });
        assert!(request.starts_with(b"POST /tokenize HTTP/1.1\r\n"));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&value).unwrap()["tokens"],
            json!([1, 2, 3])
        );
    }

    #[test]
    fn launch_tuple_is_fixed_to_one_private_socket_and_slot() {
        assert_eq!(
            launch_arguments(
                "/models/model.gguf",
                "exact-profile",
                "/run/private/llama-server.sock"
            ),
            [
                "--model",
                "/models/model.gguf",
                "--alias",
                "exact-profile",
                "--host",
                "/run/private/llama-server.sock",
                "--ctx-size",
                "8192",
                "--parallel",
                "1",
                "--n-gpu-layers",
                "999",
                "--no-webui",
                "--no-slots",
                "--jinja",
                "--no-context-shift",
            ]
        );
    }

    #[test]
    fn sandbox_tuple_exposes_only_runtime_model_gpu_support_and_private_socket() {
        let arguments = sandbox_arguments(
            Path::new("/trusted/runtime"),
            Path::new("/private/model.gguf"),
            Path::new("/run/user/1000/private"),
        );
        let values = arguments
            .iter()
            .map(|value| value.to_str().expect("UTF-8 fixture"))
            .collect::<Vec<_>>();
        for required in [
            "--unshare-all",
            "--unshare-user",
            "--disable-userns",
            "--new-session",
            "--die-with-parent",
            "--clearenv",
            "--cap-drop",
            "ALL",
            GUEST_RUNTIME_ROOT,
            GUEST_MODEL_PATH,
            GUEST_SOCKET_ROOT,
            "/trusted/runtime",
            "/private/model.gguf",
            "/run/user/1000/private",
        ] {
            assert!(values.contains(&required), "missing {required}");
        }
        for required in SANDBOX_READ_ONLY_DIRECTORIES
            .iter()
            .chain(SANDBOX_DEVICE_PATHS)
        {
            assert!(values.contains(required));
        }
        for prohibited in [
            "--share-net",
            "/home",
            "/var/home",
            "/workspace",
            "/root",
            "--ro-bind-try",
            "--dev-bind-try",
        ] {
            assert!(!values.contains(&prohibited), "admitted {prohibited}");
        }
        assert_eq!(
            launch_arguments(GUEST_MODEL_PATH, "profile", GUEST_SOCKET_PATH)[0..6],
            [
                "--model",
                GUEST_MODEL_PATH,
                "--alias",
                "profile",
                "--host",
                GUEST_SOCKET_PATH
            ]
        );
    }

    #[test]
    fn unix_socket_path_is_fixed_name_and_bounded_by_linux_address_limit() {
        assert!(valid_socket_path(Path::new(
            "/run/user/1000/am/llama-server.sock"
        )));
        assert!(!valid_socket_path(Path::new(
            "/run/user/1000/am/other.sock"
        )));
        let oversized = format!("/{}/llama-server.sock", "a".repeat(100));
        assert!(!valid_socket_path(Path::new(&oversized)));
    }

    #[test]
    fn exact_directory_does_not_assume_filesystem_link_count_semantics() {
        let directory = TestDirectory::new();
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o555))
            .expect("read-only directory");
        exact_directory(&directory.0, 0o555).expect("exact directory");
    }

    #[test]
    fn verified_file_snapshot_detects_inode_mode_and_link_drift() {
        let directory = TestDirectory::new();
        let path = directory.0.join("artifact");
        fs::write(&path, b"fixture").expect("fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("private fixture");
        let original = FileSnapshot::from_metadata(&fs::symlink_metadata(&path).expect("metadata"));

        let link = directory.0.join("artifact-link");
        fs::hard_link(&path, &link).expect("hard link");
        let linked = FileSnapshot::from_metadata(&fs::symlink_metadata(&path).expect("metadata"));
        assert_ne!(linked, original);
        fs::remove_file(link).expect("remove fixture link");

        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).expect("mode change");
        let changed_mode =
            FileSnapshot::from_metadata(&fs::symlink_metadata(&path).expect("metadata"));
        assert_ne!(changed_mode, original);

        let replacement = directory.0.join("replacement");
        fs::write(&replacement, b"fixture").expect("replacement");
        fs::rename(&replacement, &path).expect("replace fixture");
        let replaced = FileSnapshot::from_metadata(&fs::symlink_metadata(&path).expect("metadata"));
        assert_ne!(replaced.inode, original.inode);
    }

    #[test]
    fn accelerator_observation_is_exact_bounded_and_process_specific() {
        assert_eq!(
            parse_accelerator_memory(b"10, 512\n42, 15622\n", 42).expect("exact process"),
            15_622 * 1024 * 1024
        );
        for value in [
            b"10, 512\n".as_slice(),
            b"42, 0\n".as_slice(),
            b"42, 1\n42, 2\n".as_slice(),
            b"42, value\n".as_slice(),
            b"42 1\n".as_slice(),
            b"\xff\n".as_slice(),
        ] {
            assert!(parse_accelerator_memory(value, 42).is_err());
        }
    }

    #[test]
    fn completion_streams_exact_sampling_tuple_and_classifies_inert_output() {
        let decoding = DecodingProfile {
            profile_id: "diagnostic-repeatability-v1".to_owned(),
            sampler_order: vec!["greedy".to_owned()],
            temperature: 0.0,
            top_p: 1.0,
            top_k: 1,
            repeat_penalty: 1.0,
            seed: 42,
            max_output_tokens: 8,
        };
        let mut crlf_event = sse(json!({"content": "bounded ", "tokens": [1], "stop": false}));
        crlf_event.truncate(crlf_event.len() - 2);
        crlf_event.extend_from_slice(b"\r\n\r\n");
        let events = [
            crlf_event,
            sse(json!({"content": "advisory", "tokens": [2], "stop": false})),
            sse(json!({"content": "", "tokens": [], "stop": true, "stop_type": "eos"})),
            b"data: [DONE]\n\n".to_vec(),
        ];
        let ((completion, fragments), request) = exchange(streaming_response(&events), |client| {
            let request = run_request();
            let stream_id = ModelStreamId::from_raw("stream-1");
            let mut sink = RecordingSink::default();
            let completion = client
                .completion_stream(
                    b"encoded context",
                    &request,
                    &decoding,
                    None,
                    &stream_id,
                    &mut sink,
                )
                .expect("completion");
            (completion, sink.fragments)
        });
        assert_eq!(
            completion.terminal_state,
            ModelRunTerminalState::AdvisoryText
        );
        assert_eq!(completion.bytes, b"bounded advisory");
        assert_eq!(completion.tokens, 2);
        assert_eq!(completion.fragments, 3);
        assert_eq!(fragments.len(), 3);
        assert!(!fragments[0].terminal);
        assert!(!fragments[1].terminal);
        assert!(fragments[2].terminal);
        assert!(fragments[2].bytes.is_empty());
        let body = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| &request[index + 4..])
            .expect("HTTP body");
        let body: Value = serde_json::from_slice(body).expect("request JSON");
        assert_eq!(
            body,
            json!({
                "prompt": "encoded context",
                "n_predict": 8,
                "stream": true,
                "cache_prompt": false,
                "return_tokens": true,
                "return_progress": false,
                "sse_ping_interval": 1,
                "temperature": 0.0,
                "top_p": 1.0,
                "top_k": 1,
                "repeat_penalty": 1.0,
                "seed": 42,
                "samplers": ["greedy"],
                "stop": ["<|eot|>"],
                "id_slot": 0
            })
        );

        // The native /completion endpoint closes after its stop event. Some
        // transports also expose the implementation's optional [DONE] marker.
        let structured = [
            sse(json!({"content": "{}", "tokens": [1], "stop": false})),
            sse(json!({"content": "", "tokens": [], "stop": true, "stop_type": "limit"})),
        ];
        let ((completion, _), _) = exchange(streaming_response(&structured), |client| {
            let request = run_request();
            let stream_id = ModelStreamId::from_raw("stream-1");
            let mut sink = RecordingSink::default();
            let completion = client
                .completion_stream(
                    b"encoded context",
                    &request,
                    &decoding,
                    None,
                    &stream_id,
                    &mut sink,
                )
                .expect("structured candidate");
            (completion, sink.fragments)
        });
        assert_eq!(completion.terminal_state, ModelRunTerminalState::Proposed);
        assert!(completion.failure.is_none());
    }

    #[test]
    fn live_probe_cancels_after_a_fragment_and_closes_the_stream_once() {
        let first = sse(json!({"content": "partial", "tokens": [1], "stop": false}));
        let mut initial = streaming_headers();
        initial.extend_from_slice(&chunk(&first));
        let probe = cancellation_probe(3);
        let ((completion, fragments), _) = exchange_parts(
            vec![
                (initial, Duration::from_millis(150)),
                (b"0\r\n\r\n".to_vec(), Duration::ZERO),
            ],
            |client| {
                let request = run_request();
                let stream_id = ModelStreamId::from_raw("stream-1");
                let mut sink = RecordingSink::default();
                let completion = client
                    .completion_stream(
                        b"encoded context",
                        &request,
                        &DecodingProfile {
                            profile_id: request.decoding_profile_id.clone(),
                            sampler_order: vec!["greedy".to_owned()],
                            temperature: 0.0,
                            top_p: 1.0,
                            top_k: 1,
                            repeat_penalty: 1.0,
                            seed: 42,
                            max_output_tokens: 8,
                        },
                        Some(&probe),
                        &stream_id,
                        &mut sink,
                    )
                    .expect("cancelled completion");
                (completion, sink.fragments)
            },
        );
        assert_eq!(completion.terminal_state, ModelRunTerminalState::Cancelled);
        assert_eq!(completion.bytes, b"partial");
        assert_eq!(completion.tokens, 1);
        assert_eq!(completion.fragments, 2);
        assert_eq!(fragments.len(), 2);
        assert_eq!(fragments[0].bytes, b"partial");
        assert!(!fragments[0].terminal);
        assert!(fragments[1].bytes.is_empty());
        assert!(fragments[1].terminal);
    }

    #[test]
    fn deadline_interrupts_a_silent_stream_with_one_terminal_fragment() {
        let mut request = run_request();
        request.timeout_ms = 75;
        let ((completion, fragments), _) = exchange_parts(
            vec![
                (streaming_headers(), Duration::from_millis(150)),
                (b"0\r\n\r\n".to_vec(), Duration::ZERO),
            ],
            |client| {
                let stream_id = ModelStreamId::from_raw("stream-1");
                let mut sink = RecordingSink::default();
                let completion = client
                    .completion_stream(
                        b"encoded context",
                        &request,
                        &DecodingProfile {
                            profile_id: request.decoding_profile_id.clone(),
                            sampler_order: vec!["greedy".to_owned()],
                            temperature: 0.0,
                            top_p: 1.0,
                            top_k: 1,
                            repeat_penalty: 1.0,
                            seed: 42,
                            max_output_tokens: 8,
                        },
                        None,
                        &stream_id,
                        &mut sink,
                    )
                    .expect("timed out completion");
                (completion, sink.fragments)
            },
        );
        assert_eq!(completion.terminal_state, ModelRunTerminalState::TimedOut);
        assert!(completion.bytes.is_empty());
        assert_eq!(completion.fragments, 1);
        assert_eq!(fragments.len(), 1);
        assert!(fragments[0].terminal);
        assert!(fragments[0].bytes.is_empty());
    }

    #[test]
    fn malformed_chunked_sse_contracts_fail_without_advisory_fallback() {
        let incomplete = streaming_response(&[sse(
            json!({"content": "partial", "tokens": [1], "stop": false}),
        )]);
        let bad_stop = streaming_response(&[
            sse(json!({"content": "x", "tokens": [1], "stop": true, "stop_type": "other"})),
            b"data: [DONE]\n\n".to_vec(),
        ]);
        let wrong_headers =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 0\r\n\r\n"
                .to_vec();
        let mut extended_chunk = streaming_headers();
        extended_chunk.extend_from_slice(b"1;extension=true\r\nx\r\n0\r\n\r\n");
        for response in [incomplete, bad_stop, wrong_headers, extended_chunk] {
            let (failed, _) = exchange(response, |client| {
                let request = run_request();
                let stream_id = ModelStreamId::from_raw("stream-1");
                let mut sink = RecordingSink::default();
                client
                    .completion_stream(
                        b"encoded context",
                        &request,
                        &DecodingProfile {
                            profile_id: request.decoding_profile_id.clone(),
                            sampler_order: vec!["greedy".to_owned()],
                            temperature: 0.0,
                            top_p: 1.0,
                            top_k: 1,
                            repeat_penalty: 1.0,
                            seed: 42,
                            max_output_tokens: 8,
                        },
                        None,
                        &stream_id,
                        &mut sink,
                    )
                    .is_err()
            });
            assert!(failed);
        }
    }

    #[test]
    fn malformed_status_length_chunking_and_trailing_data_fail_closed() {
        for response in [
            b"HTTP/1.1 500 Error\r\nContent-Length: 0\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}x".to_vec(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n".to_vec(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n".to_vec(),
        ] {
            assert!(parse_http_response(&response).is_err());
        }
    }

    #[test]
    fn only_bounded_nonstructured_utf8_can_be_advisory_text() {
        assert!(plain_text(b"bounded advisory"));
        for value in [
            b"{\"grant\":true}".to_vec(),
            b"[1,2]".to_vec(),
            vec![0xff],
            vec![b'x'; 16 * 1024 + 1],
        ] {
            assert!(!plain_text(&value));
        }
        assert!(plain_text(b"line\nbreak"));
    }
}
