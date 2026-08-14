//! Exact `llama-server` process and private Unix-socket transport driver.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CancellationSignal, ClosedModelProposal, DecodingProfile,
    EncodedModelContext, ExactModelProfile, ModelHealth, ModelHealthState, ModelLoadReceipt,
    ModelManifestObservation, ModelProfileId, ModelResourceReport, ModelRunRequest, ModelRunResult,
    ModelRunTerminalState, ModelRuntimeFailure, ModelRuntimeIdentity, ModelStreamId,
    ModelStreamSink, ModelUnloadReceipt, RuntimeIsolationObservation, StreamedModelFragment,
    TokenCountResult, from_json,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::NativeModelDriver;

const MAX_HTTP_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
const MAX_HEALTH_RESPONSE_BYTES: usize = 4 * 1024;
const MAX_COMPLETION_BYTES: usize = 16 * 1024 * 1024;
const EXPECTED_CONTEXT_TOKENS: u32 = 8192;
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
            || socket_path.file_name().and_then(|name| name.to_str()) != Some("llama-server.sock")
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

struct LoadedRuntime {
    profile_id: ModelProfileId,
    manifest_sha256: String,
    token_counter: String,
    decoding: DecodingProfile,
    child: Child,
    loaded_at: Instant,
    input_tokens: u32,
    output_tokens: u32,
}

/// Exact native `llama-server` driver used behind `LinuxNativeModelAdapter`.
pub struct LlamaServerDriver {
    config: LlamaServerDriverConfig,
    loaded: Option<LoadedRuntime>,
}

impl LlamaServerDriver {
    /// Creates an unloaded driver without touching the runtime or model store.
    #[must_use]
    pub const fn new(config: LlamaServerDriverConfig) -> Self {
        Self {
            config,
            loaded: None,
        }
    }

    fn verify_runtime_tree(&self) -> Result<(), ModelRuntimeFailure> {
        let root = exact_directory(&self.config.runtime_root, 0o555)?;
        if root.nlink() < 2 {
            return Err(failure("model.llama-driver.runtime-root-invalid", false));
        }
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

    fn verify_model(&self, profile: &ExactModelProfile) -> Result<(), ModelRuntimeFailure> {
        exact_file(
            &self.config.model_path,
            profile.artifact.bytes,
            &profile.artifact.sha256,
        )?;
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
        let elapsed = loaded
            .loaded_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let _ = loaded.child.kill();
        loaded
            .child
            .wait()
            .map_err(|_| failure("model.llama-driver.process-reap-failed", true))?;
        if self.config.socket_path.exists() {
            fs::remove_file(&self.config.socket_path)
                .map_err(|_| failure("model.llama-driver.socket-cleanup-failed", true))?;
        }
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
        let server = self.config.runtime_root.join("bin/llama-server");
        let library = self.config.runtime_root.join("lib");
        let socket = self
            .config
            .socket_path
            .to_str()
            .ok_or_else(|| failure("model.llama-driver.socket-path-invalid", false))?;
        let model = self
            .config
            .model_path
            .to_str()
            .ok_or_else(|| failure("model.llama-driver.model-path-invalid", false))?;
        let child = Command::new(server)
            .args(launch_arguments(model, profile.profile_id.as_str(), socket))
            .env_clear()
            .env("LD_LIBRARY_PATH", &library)
            .env("PATH", "/usr/bin:/bin")
            .current_dir(&self.config.runtime_root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| failure("model.llama-driver.process-start-failed", true))?;
        self.loaded = Some(LoadedRuntime {
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            token_counter: profile.context.token_counter.clone(),
            decoding: profile.decoding.clone(),
            child,
            loaded_at: Instant::now(),
            input_tokens: 0,
            output_tokens: 0,
        });
        if let Err(error) = self.wait_until_ready() {
            let _ = self.stop_loaded();
            return Err(error);
        }
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
        cancellation: Option<&CancellationSignal>,
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
        let completion = if cancellation.is_some() {
            Completion {
                bytes: b"cancelled".to_vec(),
                tokens: 0,
                terminal_state: ModelRunTerminalState::Cancelled,
                proposal: None,
                failure: Some(failure("model.llama-driver.cancelled", false)),
            }
        } else {
            self.client().completion(
                &context.bytes,
                request.max_output_tokens,
                request.timeout_ms,
                &loaded.decoding,
            )?
        };
        let stream_id =
            ModelStreamId::from_raw(format!("stream:{}", request.model_run_id.as_str()));
        let response_sha256 = sha256(&completion.bytes);
        sink.accept(StreamedModelFragment {
            schema_version: CONTRACT_SCHEMA_VERSION,
            stream_id: stream_id.clone(),
            model_run_id: request.model_run_id.clone(),
            correlation_id: request.correlation_id.clone(),
            sequence: 0,
            bytes: completion.bytes,
            sha256: response_sha256.clone(),
            terminal: true,
        })?;
        let loaded = self.loaded.as_mut().expect("loaded state retained");
        loaded.input_tokens = loaded.input_tokens.saturating_add(0);
        loaded.output_tokens = loaded.output_tokens.saturating_add(completion.tokens);
        Ok(ModelRunResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: request.model_run_id.clone(),
            stream_id,
            correlation_id: request.correlation_id.clone(),
            terminal_state: completion.terminal_state,
            fragment_count: 1,
            response_sha256,
            proposal: completion.proposal,
            failure: completion.failure,
            resources: ModelResourceReport {
                adapter_id: self.config.identity.adapter_id.clone(),
                profile_id: request.profile_id.clone(),
                model_run_id: Some(request.model_run_id.clone()),
                resident_memory_bytes: resident_memory_bytes(loaded.child.id()),
                accelerator_memory_bytes: 0,
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
        Ok(ModelResourceReport {
            adapter_id: self.config.identity.adapter_id.clone(),
            profile_id: loaded.profile_id.clone(),
            model_run_id: None,
            resident_memory_bytes: resident_memory_bytes(loaded.child.id()),
            accelerator_memory_bytes: 0,
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

struct Completion {
    bytes: Vec<u8>,
    tokens: u32,
    terminal_state: ModelRunTerminalState,
    proposal: Option<ClosedModelProposal>,
    failure: Option<ModelRuntimeFailure>,
}

#[derive(Clone, Copy)]
enum Endpoint {
    Health,
    Tokenize,
    Completion,
}

impl Endpoint {
    const fn method(self) -> &'static str {
        match self {
            Self::Health => "GET",
            Self::Tokenize | Self::Completion => "POST",
        }
    }

    const fn path(self) -> &'static str {
        match self {
            Self::Health => "/health",
            Self::Tokenize => "/tokenize",
            Self::Completion => "/completion",
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

    fn completion(
        &self,
        context: &[u8],
        max_output_tokens: u32,
        timeout_ms: u64,
        decoding: &DecodingProfile,
    ) -> Result<Completion, ModelRuntimeFailure> {
        let prompt = std::str::from_utf8(context)
            .map_err(|_| failure("model.llama-driver.context-not-utf8", false))?;
        let body = serde_json::to_vec(&json!({
            "prompt": prompt,
            "n_predict": max_output_tokens,
            "stream": false,
            "cache_prompt": false,
            "return_tokens": true,
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
        let response = self.request(
            Endpoint::Completion,
            Some(&body),
            MAX_HTTP_RESPONSE_BYTES,
            timeout_ms,
        )?;
        let value: Value = serde_json::from_slice(&response)
            .map_err(|_| failure("model.llama-driver.completion-response-invalid", true))?;
        let content = value
            .get("content")
            .and_then(Value::as_str)
            .ok_or_else(|| failure("model.llama-driver.completion-response-invalid", true))?
            .as_bytes()
            .to_vec();
        if content.is_empty() || content.len() > MAX_COMPLETION_BYTES {
            return Err(failure("model.llama-driver.completion-size-invalid", false));
        }
        let tokens = value
            .get("tokens")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let tokens = u32::try_from(tokens)
            .map_err(|_| failure("model.llama-driver.completion-token-overflow", false))?;
        if tokens > max_output_tokens {
            return Err(failure("model.llama-driver.completion-token-limit", false));
        }
        match from_json::<ClosedModelProposal>(&content) {
            Ok(proposal) => Ok(Completion {
                bytes: content,
                tokens,
                terminal_state: ModelRunTerminalState::Proposed,
                proposal: Some(proposal),
                failure: None,
            }),
            Err(_) if plain_text(&content) => Ok(Completion {
                bytes: content,
                tokens,
                terminal_state: ModelRunTerminalState::AdvisoryText,
                proposal: None,
                failure: None,
            }),
            Err(_) => Ok(Completion {
                bytes: content,
                tokens,
                terminal_state: ModelRunTerminalState::Rejected,
                proposal: None,
                failure: Some(failure("model.llama-driver.completion-rejected", false)),
            }),
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
    let mut buffer = [0_u8; 4 * 1024 * 1024];
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
        && !text.chars().any(char::is_control)
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
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;

    use agentmage_kernel_contracts::{DecodingProfile, ModelRunTerminalState};
    use serde_json::{Value, json};

    use super::{Endpoint, UnixHttpClient, launch_arguments, parse_http_response, plain_text};

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
    fn completion_uses_exact_sampling_tuple_and_classifies_inert_output() {
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
        let (completion, request) = exchange(
            response(json!({"content": "bounded advisory", "tokens": [1, 2]})),
            |client| {
                client
                    .completion(b"encoded context", 8, 1000, &decoding)
                    .expect("completion")
            },
        );
        assert_eq!(
            completion.terminal_state,
            ModelRunTerminalState::AdvisoryText
        );
        assert!(completion.proposal.is_none());
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
                "stream": false,
                "cache_prompt": false,
                "return_tokens": true,
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

        let (completion, _) =
            exchange(response(json!({"content": "{}", "tokens": []})), |client| {
                client
                    .completion(b"encoded context", 8, 1000, &decoding)
                    .expect("closed rejection")
            });
        assert_eq!(completion.terminal_state, ModelRunTerminalState::Rejected);
        assert!(completion.proposal.is_none());
        assert!(completion.failure.is_some());
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
            b"line\nbreak".to_vec(),
        ] {
            assert!(!plain_text(&value));
        }
    }
}
