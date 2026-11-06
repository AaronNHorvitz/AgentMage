#![deny(missing_docs)]
#![forbid(unsafe_code)]
// SPDX-License-Identifier: Apache-2.0
//! Local API with explicit source scopes and separately approved exact writes.
//! Ordinary clients cannot turn discovery or model output into write authority.

/// Terminal client over this service's versioned coding routes.
pub mod coding_cli;
mod coding_repository_api;

use std::collections::{BTreeMap, VecDeque};
use std::fs::{DirBuilder, File};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rustix::fs::{Mode, OFlags, open, openat};
use serde::Deserialize;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use agentmage_capability_repository_map::coding_proposal::{
    CODING_PROPOSAL_INSTRUCTION, CodingSource, check_coding_proposal, prepare_coding_proposal,
};

use agentmage_platform_linux::{
    LinuxCodingPendingWrite, LinuxCodingReadScope, LinuxCodingReplacement, LinuxCodingRepository,
    discover_linux_coding_map,
};

use agentmage_platform_linux_inference::local_chat::{
    ChatFinish, ChatMessage, ChatRole, LocalChatEndpoint, encode_chat,
};
use agentmage_platform_linux_inference::local_chat_transport::{
    BearerKey, ChatCancellation, LocalChatApproval, LocalChatTransport, LocalProbeApproval,
    TransportLimits, probe_endpoints,
};

const MAX_HEADER: usize = 8192;
const MAX_BODY: usize = 256 * 1024;
const REQUEST_TIME: Duration = Duration::from_secs(2);
const WRITE_TIME: Duration = Duration::from_millis(250);

/// Fixed content-free failure. Request bytes, keys and OS errors are never logged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiError {
    /// Invalid listener, credentials, protected files or service operation.
    Configuration,
    /// I/O or worker failure; no request content is included.
    Io,
}

/// Two distinct random 256-bit bearer tokens, with no debug, clone or accessor.
/// The control token belongs only to trusted person-facing settings controls.
pub struct ApiTokens {
    client: Zeroizing<String>,
    control: Zeroizing<String>,
}

impl ApiTokens {
    /// Takes ownership of distinct 64-character lowercase hex tokens. Use random
    /// tokens, never passwords. Earlier copies remain the caller's responsibility.
    pub fn new(client: String, control: String) -> Result<Self, ApiError> {
        let client = Zeroizing::new(client);
        let control = Zeroizing::new(control);
        let valid = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if !valid(&client) || !valid(&control) || equal(&client, &control) {
            return Err(ApiError::Configuration);
        }
        Ok(Self { client, control })
    }

    /// Creates a new private directory and two random token files. Never replaces
    /// an existing directory or credential. This is an explicit setup operation.
    pub fn initialize(directory: &Path) -> Result<(), ApiError> {
        DirBuilder::new()
            .mode(0o700)
            .create(directory)
            .map_err(|_| ApiError::Configuration)?;
        let dir = protected_directory(directory)?;
        for name in ["client.token", "control.token"] {
            let mut random = Zeroizing::new([0_u8; 32]);
            if rustix::rand::getrandom(&mut *random, rustix::rand::GetRandomFlags::empty())
                != Ok(32)
            {
                return Err(ApiError::Io);
            }
            let token = Zeroizing::new(
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            );
            let mut file = File::from(
                openat(
                    &dir,
                    name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(|_| ApiError::Configuration)?,
            );
            file.write_all(token.as_bytes())
                .and_then(|()| file.sync_all())
                .map_err(|_| ApiError::Io)?;
        }
        Ok(())
    }

    /// Loads bounded, owner-only regular files through a held private directory.
    /// Symlink token files, hard links and permissive modes are refused.
    pub fn load(directory: &Path) -> Result<Self, ApiError> {
        let dir = protected_directory(directory)?;
        let read = |name| -> Result<String, ApiError> {
            let file = File::from(
                openat(
                    &dir,
                    name,
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
                || meta.len() != 64
            {
                return Err(ApiError::Configuration);
            }
            let mut bytes = Zeroizing::new(String::new());
            file.take(65)
                .read_to_string(&mut bytes)
                .map_err(|_| ApiError::Configuration)?;
            Ok(std::mem::take(&mut *bytes))
        };
        let client = Zeroizing::new(read("client.token")?);
        let control = Zeroizing::new(read("control.token")?);
        Self::new(client.to_string(), control.to_string())
    }
}

fn protected_directory(path: &Path) -> Result<File, ApiError> {
    let dir = File::from(
        open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| ApiError::Configuration)?,
    );
    let meta = dir.metadata().map_err(|_| ApiError::Io)?;
    if !meta.is_dir()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o7777 != 0o700
    {
        return Err(ApiError::Configuration);
    }
    Ok(dir)
}

fn equal(a: &str, b: &str) -> bool {
    // Fixed-size accepted tokens; do not use prefix or early-exit comparison.
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0_u8, |d, (x, y)| d | (x ^ y)) == 0
}

struct Configuration {
    endpoint: String,
    transport: LocalChatTransport,
    approval: LocalChatApproval,
}

struct WorkspaceGrant {
    scope: LinuxCodingReadScope,
    model_context_revision: Option<u64>,
}

struct PendingChange {
    workspace_revision: u64,
    scope: Arc<WorkspaceGrant>,
    write: LinuxCodingPendingWrite,
}

#[derive(Default)]
struct State {
    repository: Option<Arc<Mutex<LinuxCodingRepository>>>,
    pending_change: Option<PendingChange>,
    write_records: VecDeque<Value>,
    write_sequence: u64,
    workspace: Option<Arc<WorkspaceGrant>>,
    workspace_revision: u64,
    configured: Option<Arc<Configuration>>,
    active: Option<ChatCancellation>,
    active_cancellable: bool,
    revision: u64,
    sequence: u64,
    activity: VecDeque<Value>,
}

impl State {
    fn record_write(&mut self, mut record: Value) {
        self.write_sequence += 1;
        record["sequence"] = json!(self.write_sequence);
        self.write_records.push_back(record);
        if self.write_records.len() > 8 {
            self.write_records.pop_front();
        }
    }

    fn record(&mut self, action: &'static str, outcome: &'static str) {
        self.sequence += 1;
        self.activity.push_back(json!({"sequence":self.sequence,"settings_revision":self.revision,"endpoint":self.configured.as_ref().map(|c|&c.endpoint),"workspace_revision":self.workspace_revision,"action":action,"outcome":outcome}));
        if self.activity.len() > 128 {
            self.activity.pop_front();
        }
    }
}

/// Serves four bounded connection workers on a caller-owned loopback listener.
/// All routes require bearer authentication and exact numeric Host; browser
/// origins/cookies/fetch metadata are refused and no CORS headers are emitted.
/// Only one source or upstream operation is active, leaving workers for cancellation.
/// On shutdown, cancels the active operation and joins every owned worker.
pub fn serve(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
) -> Result<(), ApiError> {
    serve_state(listener, tokens, stop, State::default())
}

/// Serves the same API with an explicitly chosen private canonical authority
/// directory for non-production repository operations. Starting this service
/// authorizes encrypted state writes there, not repository commands or effects.
/// Keep the control token unchanged: its domain-separated key opens this state.
pub fn serve_coding(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    state_root: &Path,
) -> Result<(), ApiError> {
    if !listener
        .local_addr()
        .map_err(|_| ApiError::Io)?
        .ip()
        .is_loopback()
    {
        return Err(ApiError::Configuration);
    }
    let repository =
        LinuxCodingRepository::open(state_root, tokens.control.as_bytes(), clock_ms()?)
            .map_err(|_| ApiError::Configuration)?;
    serve_state(
        listener,
        tokens,
        stop,
        State {
            repository: Some(Arc::new(Mutex::new(repository))),
            ..State::default()
        },
    )
}

fn serve_state(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    initial: State,
) -> Result<(), ApiError> {
    let address = listener.local_addr().map_err(|_| ApiError::Io)?;
    if !address.ip().is_loopback() {
        return Err(ApiError::Configuration);
    }
    listener.set_nonblocking(true).map_err(|_| ApiError::Io)?;
    let state = Mutex::new(initial);
    thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..4 {
            let (listener, tokens, stop, state) = (&listener, &tokens, &stop, &state);
            let worker = thread::Builder::new().spawn_scoped(scope, move || {
                while !stop.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut socket, peer)) if peer.ip().is_loopback() => {
                            let _ = handle(&mut socket, address, tokens, state, stop);
                        }
                        Ok(_) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5))
                        }
                        Err(_) => {
                            stop.store(true, Ordering::Relaxed);
                            return Err(ApiError::Io);
                        }
                    }
                }
                Ok(())
            });
            match worker {
                Ok(worker) => workers.push(worker),
                Err(_) => {
                    stop.store(true, Ordering::Relaxed);
                    return Err(ApiError::Io);
                }
            }
        }
        while !stop.load(Ordering::Relaxed) {
            if workers.iter().any(|worker| worker.is_finished()) {
                stop.store(true, Ordering::Relaxed);
            }
            thread::sleep(Duration::from_millis(10));
        }
        if let Some(cancel) = &state.lock().map_err(|_| ApiError::Io)?.active {
            cancel.cancel();
        }
        for worker in workers {
            worker.join().map_err(|_| ApiError::Io)??;
        }
        Ok(())
    })
}

struct Request {
    method: String,
    path: String,
    control: bool,
    body: Zeroizing<Vec<u8>>,
}

// One request per connection; no pipelining, chunked input, upgrades or proxies.
fn request(
    socket: &mut TcpStream,
    address: SocketAddr,
    tokens: &ApiTokens,
    stop: &AtomicBool,
) -> Result<Request, u16> {
    socket
        .set_read_timeout(Some(Duration::from_millis(50)))
        .map_err(|_| 400_u16)?;
    let deadline = Instant::now() + REQUEST_TIME;
    let mut header = Zeroizing::new(Vec::new());
    loop {
        if header.len() >= MAX_HEADER {
            return Err(413);
        }
        let mut byte = [0];
        read(socket, &mut byte, deadline, stop)?;
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = std::str::from_utf8(&header).map_err(|_| 400_u16)?;
    let mut lines = text[..text.len() - 4].split("\r\n");
    let mut first = lines.next().ok_or(400_u16)?.split(' ');
    let method = first.next().ok_or(400_u16)?.to_owned();
    let path = first.next().ok_or(400_u16)?.to_owned();
    if first.next() != Some("HTTP/1.1")
        || first.next().is_some()
        || !path.starts_with("/v1/")
        || path.contains(['?', '#', '%'])
    {
        return Err(400);
    }
    let mut host = None;
    let mut auth = None;
    let mut length = None;
    let mut media = None;
    let mut seen = std::collections::BTreeSet::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(400_u16)?;
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !value.bytes().all(|b| (32..=126).contains(&b))
        {
            return Err(400);
        }
        let name = name.to_ascii_lowercase();
        if !seen.insert(name.clone()) {
            return Err(400);
        }
        let value = value.trim();
        match name.as_str() {
            "host" => host = Some(value),
            "authorization" => auth = value.strip_prefix("Bearer "),
            "content-length" => {
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(400);
                }
                length = Some(value.parse::<usize>().map_err(|_| 413_u16)?);
            }
            "content-type" => media = Some(value),
            "content-encoding" => return Err(415),
            "origin" | "cookie" | "transfer-encoding" | "expect" | "upgrade" | "forwarded" => {
                return Err(403);
            }
            n if n.starts_with("sec-fetch-") || n.starts_with("x-forwarded-") => return Err(403),
            "connection" if !value.eq_ignore_ascii_case("close") => return Err(400),
            _ => {}
        }
    }
    if host != Some(address.to_string().as_str()) {
        return Err(403);
    }
    let auth = auth.ok_or(401_u16)?;
    let control = equal(auth, &tokens.control);
    if !control && !equal(auth, &tokens.client) {
        return Err(401);
    }
    let length = length.ok_or(411_u16)?;
    if length > MAX_BODY {
        return Err(413);
    }
    if length > 0 && media != Some("application/json") {
        return Err(415);
    }
    if (method == "GET" || method == "DELETE") && length != 0 {
        return Err(400);
    }
    let mut body = Zeroizing::new(vec![0; length]);
    let mut offset = 0;
    while offset < length {
        offset += read(socket, &mut body[offset..], deadline, stop)?;
    }
    Ok(Request {
        method,
        path,
        control,
        body,
    })
}

fn read(
    socket: &mut TcpStream,
    bytes: &mut [u8],
    deadline: Instant,
    stop: &AtomicBool,
) -> Result<usize, u16> {
    loop {
        if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
            return Err(408);
        }
        match socket.read(bytes) {
            Ok(0) => return Err(400),
            Ok(n) => return Ok(n),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(400),
        }
    }
}

fn write(socket: &mut TcpStream, bytes: &[u8]) -> Result<(), ApiError> {
    socket
        .set_write_timeout(Some(Duration::from_millis(50)))
        .map_err(|_| ApiError::Io)?;
    let deadline = Instant::now() + WRITE_TIME;
    let mut offset = 0;
    while offset < bytes.len() {
        if Instant::now() >= deadline {
            return Err(ApiError::Io);
        }
        match socket.write(&bytes[offset..]) {
            Ok(0) => return Err(ApiError::Io),
            Ok(n) => offset += n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(ApiError::Io),
        }
    }
    Ok(())
}

fn headers(
    socket: &mut TcpStream,
    status: u16,
    media: &str,
    length: Option<usize>,
) -> Result<(), ApiError> {
    let length = length
        .map(|n| format!("Content-Length: {n}\r\n"))
        .unwrap_or_default();
    write(socket, format!("HTTP/1.1 {status} Response\r\nContent-Type: {media}\r\n{length}Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n").as_bytes())
}

fn response(socket: &mut TcpStream, status: u16, value: Value) -> Result<(), ApiError> {
    let bytes = Zeroizing::new(serde_json::to_vec(&value).map_err(|_| ApiError::Io)?);
    headers(socket, status, "application/json", Some(bytes.len()))?;
    write(socket, &bytes)
}

fn error(socket: &mut TcpStream, status: u16) -> Result<(), ApiError> {
    let code = match status {
        401 => "api.authentication",
        403 => "api.authority",
        404 => "api.route",
        409 => "api.busy",
        408 => "api.timeout",
        413 => "api.limit",
        428 => "api.endpoint-choice-required",
        503 => "api.stopping",
        _ => "api.request",
    };
    let message = match status {
        401 => "Use a valid local API bearer token.",
        403 => {
            "This request is not allowed. Use trusted controls for explicit endpoint or workspace choices."
        }
        404 => "Use a supported version 1 API route and method.",
        409 => "Wait for the active connection or cancel it, then retry.",
        408 => "The request took too long. Send one complete bounded request.",
        413 => "Reduce the request header or body size.",
        428 => "Choose a local endpoint in trusted settings before sending conversations.",
        503 => "The local API is stopping.",
        _ => "Check the request method, JSON fields, media type and explicit Content-Length.",
    };
    response(socket, status, json!({"error":code,"message":message}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    endpoint: String,
    #[serde(default, deserialize_with = "secret")]
    key: Option<Zeroizing<String>>,
    approve_conversations: bool,
}

fn secret<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Zeroizing<String>>, D::Error> {
    Option::<String>::deserialize(d).map(|s| s.map(Zeroizing::new))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    endpoints: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chat {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodingProposalRequest {
    model: String,
    task: String,
    files: Vec<CodingSource>,
    max_message_bytes: usize,
    max_tokens: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceSelection {
    root: String,
    paths: Vec<String>,
    approve_read: bool,
    approve_model_context: bool,
    #[serde(default)]
    settings_revision: Option<u64>,
    #[serde(default)]
    settings_endpoint: Option<String>,
    #[serde(default)]
    expected_root_sha256: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapRequest {
    root: String,
    query: String,
    approve_discovery: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceRead {
    workspace_revision: u64,
    paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceSearch {
    workspace_revision: u64,
    paths: Vec<String>,
    query: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceProposal {
    workspace_revision: u64,
    paths: Vec<String>,
    model: String,
    task: String,
    max_message_bytes: usize,
    max_tokens: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceChange {
    workspace_revision: u64,
    paths: Vec<String>,
    expected_sources: Vec<ExpectedSource>,
    task: String,
    max_message_bytes: usize,
    answer: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedSource {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceApply {
    workspace_revision: u64,
    change_set_sha256: String,
    preview_sha256: String,
    approve: bool,
}

fn clock_ms() -> Result<u64, ApiError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_millis()).ok())
        .ok_or(ApiError::Io)
}

// Named serde structs also accept arrays. All API additions require objects;
// typed parsing first still rejects duplicate keys before the shape check.
fn object_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, ()> {
    let value = serde_json::from_slice(bytes).map_err(|_| ())?;
    if !serde_json::from_slice::<Value>(bytes)
        .map_err(|_| ())?
        .is_object()
    {
        return Err(());
    }
    Ok(value)
}

struct Operation<'a> {
    state: &'a Mutex<State>,
    cancel: ChatCancellation,
    action: &'static str,
    outcome: &'static str,
}

impl Drop for Operation<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.record(self.action, self.outcome);
            state.active = None;
            state.active_cancellable = false;
        }
    }
}

fn begin<'a>(
    state: &'a Mutex<State>,
    stop: &AtomicBool,
    action: &'static str,
) -> Result<(Operation<'a>, Option<Arc<Configuration>>), u16> {
    let mut guard = state.lock().map_err(|_| 500_u16)?;
    if stop.load(Ordering::Relaxed) {
        return Err(503);
    }
    if guard.active.is_some() {
        return Err(409);
    }
    let cancel = ChatCancellation::default();
    guard.active = Some(cancel.clone());
    guard.active_cancellable = true;
    guard.record(action, "started");
    Ok((
        Operation {
            state,
            cancel,
            action,
            outcome: "incomplete",
        },
        guard.configured.clone(),
    ))
}

// The operation token prevents grant/settings replacement while collecting and
// using these snapshots. No grant object or absolute path reaches the model.
fn workspace_sources(
    state: &Mutex<State>,
    revision: u64,
    paths: &[String],
    for_model: bool,
) -> Result<Vec<CodingSource>, (u16, &'static str)> {
    let scope = {
        let guard = state.lock().map_err(|_| (500, "code.workspace.state"))?;
        if revision != guard.workspace_revision {
            return Err((409, "code.workspace.stale"));
        }
        let workspace = guard
            .workspace
            .clone()
            .ok_or((428, "code.workspace.choice-required"))?;
        if for_model
            && (workspace.model_context_revision != Some(guard.revision)
                || guard.configured.is_none())
        {
            return Err((403, "code.workspace.disclosure-required"));
        }
        workspace
    };
    scope
        .scope
        .read(paths)
        .map(|files| {
            files
                .into_iter()
                .map(|s| CodingSource {
                    path: s.path,
                    content: s.content,
                })
                .collect()
        })
        .map_err(|e| (400, e.code()))
}

fn coding_proposal(
    socket: &mut TcpStream,
    input: CodingProposalRequest,
    mut op: Operation<'_>,
    config: Option<Arc<Configuration>>,
    workspace_revision: Option<u64>,
) -> Result<(), ApiError> {
    // This is an effect-free proposal over caller-supplied snapshots.
    // Source paths are data, never resolved against the service host.
    let context = match prepare_coding_proposal(&input.task, &input.files, input.max_message_bytes)
    {
        Ok(context) => context,
        Err(e) => {
            op.outcome = e.code();
            return response(socket, 400, json!({"error":e.code()}));
        }
    };
    let messages = [
        ChatMessage {
            role: ChatRole::System,
            content: CODING_PROPOSAL_INSTRUCTION,
        },
        ChatMessage {
            role: ChatRole::User,
            content: context.user_message(),
        },
    ];
    if !(1..=4096).contains(&input.max_tokens)
        || encode_chat(&input.model, &messages, input.max_tokens).is_err()
    {
        op.outcome = "api.request";
        return error(socket, 400);
    }
    let Some(config) = config else {
        op.outcome = "api.endpoint-choice-required";
        return error(socket, 428);
    };
    headers(socket, 200, "application/x-ndjson", None)?;
    event(
        socket,
        json!({"type":"context", "version":1,
        "message_bytes":context.message_bytes(), "files":context.outlines(),
        "excerpts":context.excerpts(), "authority":"proposal-only","workspace_revision":workspace_revision}),
    )?;
    let mut answer = Zeroizing::new(String::new());
    let mut writable = true;
    let mut oversized = false;
    let result = config.transport.chat(
        &config.approval,
        &input.model,
        &messages,
        input.max_tokens,
        &op.cancel,
        |text| {
            if text.len() > (64 * 1024_usize).saturating_sub(answer.len()) {
                oversized = true;
                op.cancel.cancel();
            } else if !oversized {
                answer.push_str(text);
                if writable && event(socket, json!({"type":"text","text":text})).is_err() {
                    writable = false;
                    op.cancel.cancel();
                }
            }
        },
    );
    if !writable {
        op.outcome = "api.client-disconnected";
        return Err(ApiError::Io);
    }
    let refused = if oversized {
        Some("code.proposal.limit")
    } else {
        match result {
            Ok(ChatFinish::Stop) => None,
            Ok(ChatFinish::Length) => Some("code.proposal.incomplete"),
            Err(e) => Some(e.code()),
        }
    };
    if let Some(code) = refused {
        op.outcome = code;
        return event(socket, json!({"type":"error","code":code,"partial":true}));
    }
    match check_coding_proposal(&context, &input.files, &answer) {
        Ok(changes) => {
            op.outcome = "proposed";
            event(
                socket,
                json!({"type":"proposal","version":1,"changes":changes,
                "applied":false,"tested":false,"authority":"proposal-only","workspace_revision":workspace_revision}),
            )
        }
        Err(e) => {
            op.outcome = e.code();
            event(
                socket,
                json!({"type":"error","code":e.code(),"partial":true}),
            )
        }
    }
}

fn handle(
    socket: &mut TcpStream,
    address: SocketAddr,
    tokens: &ApiTokens,
    state: &Mutex<State>,
    stop: &AtomicBool,
) -> Result<(), ApiError> {
    let req = match request(socket, address, tokens, stop) {
        Ok(req) => req,
        Err(status) => return error(socket, status),
    };
    let route = (req.method.as_str(), req.path.as_str());
    if req.path.starts_with("/v1/code/repository") {
        return coding_repository_api::handle_repository(socket, &req, state, stop);
    }
    if matches!(req.path.as_str(), "/v1/settings" | "/v1/probe") && !req.control {
        return error(socket, 403);
    }
    if req.path == "/v1/code/workspace"
        && matches!(req.method.as_str(), "PUT" | "DELETE")
        && !req.control
    {
        return error(socket, 403);
    }
    if req.path == "/v1/code/workspace/apply" && !req.control {
        return error(socket, 403);
    }
    if req.path == "/v1/code/map" && !req.control {
        return error(socket, 403);
    }
    match route {
        ("POST", "/v1/code/map") => {
            let Ok(input) = object_json::<MapRequest>(&req.body) else {
                return error(socket, 400);
            };
            if !input.approve_discovery {
                return error(socket, 403);
            }
            let (mut op, _) = match begin(state, stop, "code.map") {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            let map = match discover_linux_coding_map(Path::new(&input.root), &input.query, || {
                op.cancel.is_cancelled() || stop.load(Ordering::Relaxed)
            }) {
                Ok(map) => map,
                Err(e) => {
                    op.outcome = e.code();
                    return response(socket, 400, json!({"error":e.code()}));
                }
            };
            op.outcome = "mapped";
            let entries = map
                .entries
                .iter()
                .map(|entry| json!({"path":entry.path,"score":entry.score}))
                .collect::<Vec<_>>();
            drop(op);
            response(
                socket,
                200,
                json!({
                    "version":1,"root_sha256":map.root_sha256,"entries":entries,
                    "directories":map.directories,"names":map.names,
                    "excluded":map.excluded,"skipped":map.skipped,
                    "authority":"metadata-only","ranking":"lexical-path",
                    "coverage":"visible paths; hidden and generated objects excluded; not Git ignore rules",
                    "selected":false,"model_context_approved":false,
                    "applied":false,"tested":false
                }),
            )
        }
        ("POST", "/v1/code/workspace/changes") => {
            let Ok(input) = object_json::<WorkspaceChange>(&req.body) else {
                return error(socket, 400);
            };
            let Ok(shape) = serde_json::from_slice::<Value>(&req.body) else {
                return error(socket, 400);
            };
            if !shape["expected_sources"]
                .as_array()
                .is_some_and(|sources| sources.iter().all(Value::is_object))
            {
                return error(socket, 400);
            }
            let (mut op, _) = match begin(state, stop, "code.write.prepare") {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            let prepared = (|| {
                let sources =
                    workspace_sources(state, input.workspace_revision, &input.paths, false)?;
                let context =
                    prepare_coding_proposal(&input.task, &sources, input.max_message_bytes)
                        .map_err(|e| (400, e.code()))?;
                // These must be the exact complete snapshots from the proposal's
                // context event, including files that the model did not edit.
                let actual: BTreeMap<_, _> = context
                    .outlines()
                    .iter()
                    .map(|o| (o.path.clone(), o.sha256.clone()))
                    .collect();
                let expected: BTreeMap<_, _> = input
                    .expected_sources
                    .iter()
                    .map(|s| (s.path.clone(), s.sha256.clone()))
                    .collect();
                if actual.len() != input.expected_sources.len() || actual != expected {
                    return Err((409, "code.proposal.stale"));
                }
                let changes = check_coding_proposal(&context, &sources, &input.answer)
                    .map_err(|e| (400, e.code()))?;
                let scope = state
                    .lock()
                    .map_err(|_| (500, "code.workspace.state"))?
                    .workspace
                    .clone()
                    .ok_or((428, "code.workspace.choice-required"))?;
                let replacements = changes
                    .iter()
                    .map(|c| LinuxCodingReplacement {
                        path: &c.path,
                        before_sha256: &c.before_sha256,
                        content: &c.content,
                    })
                    .collect::<Vec<_>>();
                let write = scope
                    .scope
                    .prepare_write(
                        &replacements,
                        &input.task,
                        clock_ms().map_err(|_| (500, "code.write.clock"))?,
                    )
                    .map_err(|e| (409, e.code()))?;
                Ok(PendingChange {
                    workspace_revision: input.workspace_revision,
                    scope,
                    write,
                })
            })();
            let pending = match prepared {
                Ok(pending) => pending,
                Err((status, code)) => {
                    op.outcome = code;
                    drop(op);
                    return response(socket, status, json!({"error":code}));
                }
            };
            let mut guard = state.lock().map_err(|_| ApiError::Io)?;
            if op.cancel.is_cancelled() || stop.load(Ordering::Relaxed) {
                op.outcome = "code.workspace.cancelled";
                drop(guard);
                drop(op);
                return response(socket, 409, json!({"error":"code.workspace.cancelled"}));
            }
            let value = json!({"workspace_revision":pending.workspace_revision,
                "workspace_root_sha256":pending.write.workspace_root_sha256(),
                "preview":pending.write.preview(),"expires_at_epoch_ms":pending.write.expires_at_epoch_ms(),
                "applied":false,"tested":false,"isolation_verified":false,"retention":"process-only"});
            guard.pending_change = Some(pending);
            op.outcome = "prepared";
            drop(guard);
            drop(op);
            response(socket, 200, value)
        }
        ("GET", "/v1/code/workspace/changes") => {
            let guard = state.lock().map_err(|_| ApiError::Io)?;
            response(
                socket,
                200,
                json!({"retention":"process-only","records":guard.write_records,
                "maximum_records":8,"last_sequence":guard.write_sequence,"history_truncated":guard.write_sequence > 8,
                "pending":guard.pending_change.as_ref().map(|p| json!({
                    "workspace_revision":p.workspace_revision,"preview":p.write.preview(),
                    "workspace_root_sha256":p.write.workspace_root_sha256(),
                    "expires_at_epoch_ms":p.write.expires_at_epoch_ms(),
                    "applied":false,"tested":false,"isolation_verified":false}))}),
            )
        }
        ("POST", "/v1/code/workspace/apply") => {
            let Ok(input) = object_json::<WorkspaceApply>(&req.body) else {
                return error(socket, 400);
            };
            let (mut op, _) = match begin(state, stop, "code.write.apply") {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            let mut guard = state.lock().map_err(|_| ApiError::Io)?;
            let Some(pending) = guard.pending_change.as_ref() else {
                op.outcome = "code.write.choice-required";
                drop(guard);
                drop(op);
                return response(socket, 428, json!({"error":"code.write.choice-required"}));
            };
            if input.workspace_revision != guard.workspace_revision
                || input.workspace_revision != pending.workspace_revision
                || input.change_set_sha256 != pending.write.preview().change_set_sha256
                || input.preview_sha256 != pending.write.preview().preview_sha256
            {
                op.outcome = "code.write.stale";
                drop(guard);
                drop(op);
                return response(socket, 409, json!({"error":"code.write.stale"}));
            }
            if op.cancel.is_cancelled() || stop.load(Ordering::Relaxed) {
                op.outcome = "code.workspace.cancelled";
                drop(guard);
                drop(op);
                return response(socket, 409, json!({"error":"code.workspace.cancelled"}));
            }
            let now = clock_ms()?;
            // Consumption is serialized with cancellation and grant replacement.
            // Never restore a taken pending change, even after error/disconnect.
            let pending = guard.pending_change.take().ok_or(ApiError::Io)?;
            let binding = json!({"workspace_revision":input.workspace_revision,
                "change_set_sha256":input.change_set_sha256,"preview_sha256":input.preview_sha256});
            if !input.approve {
                guard.record_write(json!({"binding":binding,"outcome":"denied","effect":"none"}));
                op.outcome = "denied";
                drop(guard);
                drop(op);
                return response(socket, 200, json!({"outcome":"denied","effect":"none"}));
            }
            // Native write transactions reconcile partial failures as one unit.
            // A cancel request after this point reports non-interruptibility.
            guard.active_cancellable = false;
            guard.record_write(
                json!({"binding":binding,"outcome":"dispatching","effect":"unknown"}),
            );
            drop(guard);
            let result = pending.write.apply(
                &pending.scope.scope,
                &input.change_set_sha256,
                &input.preview_sha256,
                now,
            );
            let (status, value) = match result {
                Ok(value) => {
                    op.outcome = match value["outcome"].as_str() {
                        Some("committed") => "committed",
                        Some("failed_no_change") => "failed_no_change",
                        Some("restored") => "restored",
                        _ => "uncertain",
                    };
                    (200, value)
                }
                Err(e) => {
                    op.outcome = e.code();
                    (
                        409,
                        json!({"error":e.code(),"effect":"unknown","tested":false}),
                    )
                }
            };
            state
                .lock()
                .map_err(|_| ApiError::Io)?
                .record_write(json!({"binding":binding,"result":value}));
            drop(op);
            response(socket, status, value)
        }
        ("POST", "/v1/code/proposals") => {
            let Ok(input) = serde_json::from_slice::<CodingProposalRequest>(&req.body) else {
                return error(socket, 400);
            };
            // Typed named structs can admit positional arrays. Require objects
            // at both request and source levels after bounded typed parsing.
            let Ok(shape) = serde_json::from_slice::<Value>(&req.body) else {
                return error(socket, 400);
            };
            if !shape.is_object()
                || !shape["files"]
                    .as_array()
                    .is_some_and(|files| files.iter().all(Value::is_object))
            {
                return error(socket, 400);
            }
            let (op, config) = match begin(state, stop, "code.proposal") {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            coding_proposal(socket, input, op, config, None)
        }
        ("GET", "/v1/code/workspace") => {
            let guard = state.lock().map_err(|_| ApiError::Io)?;
            response(
                socket,
                200,
                json!({"workspace_revision":guard.workspace_revision,
                "selected":guard.workspace.is_some(), "authority":"read-only",
                "paths":guard.workspace.as_ref().map(|w| w.scope.paths()).unwrap_or_default(),
                "model_context_approved":guard.workspace.as_ref().is_some_and(|w|
                    w.model_context_revision == Some(guard.revision) && guard.configured.is_some())}),
            )
        }
        ("PUT", "/v1/code/workspace") => {
            let Ok(input) = object_json::<WorkspaceSelection>(&req.body) else {
                return error(socket, 400);
            };
            if !input.approve_read {
                return error(socket, 403);
            }
            let (mut op, config) = match begin(state, stop, "code.workspace.select") {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            let displayed_settings_match = {
                let guard = state.lock().map_err(|_| ApiError::Io)?;
                input
                    .settings_revision
                    .is_none_or(|expected| expected == guard.revision)
                    && input.settings_endpoint.as_ref().is_none_or(|expected| {
                        config
                            .as_ref()
                            .is_some_and(|current| expected == &current.endpoint)
                    })
            };
            if !displayed_settings_match {
                op.outcome = "code.workspace.settings-stale";
                drop(op);
                return response(
                    socket,
                    409,
                    json!({"error":"code.workspace.settings-stale"}),
                );
            }
            if input.approve_model_context && config.is_none() {
                return error(socket, 428);
            }
            if let Some(expected) = &input.expected_root_sha256
                && LinuxCodingReadScope::verify_root(Path::new(&input.root), expected).is_err()
            {
                op.outcome = "code.workspace.root-stale";
                return response(socket, 409, json!({"error":op.outcome}));
            }
            let scope = match LinuxCodingReadScope::select(Path::new(&input.root), &input.paths) {
                Ok(scope) => scope,
                Err(e) => {
                    op.outcome = e.code();
                    return response(socket, 400, json!({"error":e.code()}));
                }
            };
            if input
                .expected_root_sha256
                .as_ref()
                .is_some_and(|expected| expected != scope.root_sha256())
            {
                op.outcome = "code.workspace.root-stale";
                return response(socket, 409, json!({"error":op.outcome}));
            }
            let mut guard = state.lock().map_err(|_| ApiError::Io)?;
            if op.cancel.is_cancelled() || stop.load(Ordering::Relaxed) {
                op.outcome = "code.workspace.cancelled";
                return response(socket, 409, json!({"error":op.outcome}));
            }
            guard.pending_change = None;
            guard.workspace_revision += 1;
            guard.workspace = Some(Arc::new(WorkspaceGrant {
                scope,
                model_context_revision: input.approve_model_context.then_some(guard.revision),
            }));
            op.outcome = "accepted";
            response(
                socket,
                200,
                json!({"selected":true,"workspace_revision":guard.workspace_revision,
                "authority":"read-only"}),
            )
        }
        ("DELETE", "/v1/code/workspace") => {
            let mut guard = state.lock().map_err(|_| ApiError::Io)?;
            if guard.active.is_some() {
                return error(socket, 409);
            }
            guard.workspace = None;
            guard.pending_change = None;
            guard.workspace_revision += 1;
            guard.record("code.workspace.revoke", "accepted");
            response(
                socket,
                200,
                json!({"selected":false,"workspace_revision":guard.workspace_revision}),
            )
        }
        ("POST", "/v1/code/workspace/read") | ("POST", "/v1/code/workspace/search") => {
            let (revision, paths, query) = if req.path.ends_with("/search") {
                let Ok(input) = object_json::<WorkspaceSearch>(&req.body) else {
                    return error(socket, 400);
                };
                if input.query.is_empty() || input.query.len() > 128 || input.query.contains('\0') {
                    return error(socket, 400);
                }
                (input.workspace_revision, input.paths, Some(input.query))
            } else {
                let Ok(input) = object_json::<WorkspaceRead>(&req.body) else {
                    return error(socket, 400);
                };
                (input.workspace_revision, input.paths, None)
            };
            let (mut op, _) = match begin(
                state,
                stop,
                if query.is_some() {
                    "code.workspace.search"
                } else {
                    "code.workspace.read"
                },
            ) {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            let sources = match workspace_sources(state, revision, &paths, false) {
                Ok(sources) => sources,
                Err((status, code)) => {
                    op.outcome = code;
                    return response(socket, status, json!({"error":code}));
                }
            };
            if op.cancel.is_cancelled() {
                op.outcome = "code.workspace.cancelled";
                return response(socket, 409, json!({"error":op.outcome}));
            }
            op.outcome = "read";
            if let Some(query) = query {
                let mut matches = Vec::new();
                let mut truncated = false;
                'files: for source in sources {
                    for (start, text) in source.content.match_indices(&query) {
                        if matches.len() == 128 {
                            truncated = true;
                            break 'files;
                        }
                        matches.push(json!({"path":source.path,"start":start,"end":start+text.len(),"text":text}));
                    }
                }
                response(
                    socket,
                    200,
                    json!({"workspace_revision":revision,"matches":matches,
                    "truncated":truncated,"authority":"read-only"}),
                )
            } else {
                response(
                    socket,
                    200,
                    json!({"workspace_revision":revision,"files":sources,"authority":"read-only"}),
                )
            }
        }
        ("POST", "/v1/code/workspace/proposals") => {
            let Ok(input) = object_json::<WorkspaceProposal>(&req.body) else {
                return error(socket, 400);
            };
            let (mut op, config) = match begin(state, stop, "code.workspace.proposal") {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            let files = match workspace_sources(state, input.workspace_revision, &input.paths, true)
            {
                Ok(sources) => sources,
                Err((status, code)) => {
                    op.outcome = code;
                    return response(socket, status, json!({"error":code}));
                }
            };
            coding_proposal(
                socket,
                CodingProposalRequest {
                    model: input.model,
                    task: input.task,
                    files,
                    max_message_bytes: input.max_message_bytes,
                    max_tokens: input.max_tokens,
                },
                op,
                config,
                Some(input.workspace_revision),
            )
        }
        ("GET", "/v1/status") => {
            let state = state.lock().map_err(|_| ApiError::Io)?;
            response(
                socket,
                200,
                json!({"version":1,"configured":state.configured.is_some(),"settings_revision":state.revision,"busy":state.active.is_some(),"cancellable":state.active_cancellable}),
            )
        }
        ("GET", "/v1/activity") => {
            let state = state.lock().map_err(|_| ApiError::Io)?;
            response(
                socket,
                200,
                json!({"retention":"process-only","last_sequence":state.sequence,"events":state.activity}),
            )
        }
        ("GET", "/v1/settings") => {
            let state = state.lock().map_err(|_| ApiError::Io)?;
            response(
                socket,
                200,
                json!({"endpoint":state.configured.as_ref().map(|c| &c.endpoint),"settings_revision":state.revision}),
            )
        }
        ("PUT", "/v1/settings") => {
            let Ok(mut input) = serde_json::from_slice::<Settings>(&req.body) else {
                return error(socket, 400);
            };
            if !input.approve_conversations {
                return error(socket, 403);
            }
            let Ok(endpoint) = LocalChatEndpoint::parse(&input.endpoint) else {
                return error(socket, 400);
            };
            // Approval is minted here only, after authenticated explicit choice.
            let key = match input.key.take() {
                Some(mut s) => match BearerKey::new(std::mem::take(&mut *s)) {
                    Ok(key) => Some(key),
                    Err(_) => return error(socket, 400),
                },
                None => None,
            };
            let approval = LocalChatApproval::for_endpoint(endpoint.clone());
            let transport = LocalChatTransport::new(endpoint, key, TransportLimits::default())
                .map_err(|_| ApiError::Configuration)?;
            let mut state = state.lock().map_err(|_| ApiError::Io)?;
            if state.active.is_some() {
                return error(socket, 409);
            }
            state.configured = Some(Arc::new(Configuration {
                endpoint: input.endpoint,
                transport,
                approval,
            }));
            state.pending_change = None;
            state.revision += 1;
            state.record("settings.choice", "accepted");
            response(socket, 200, json!({"configured":true}))
        }
        ("DELETE", "/v1/settings") => {
            let mut state = state.lock().map_err(|_| ApiError::Io)?;
            if state.active.is_some() {
                return error(socket, 409);
            }
            state.configured = None;
            state.pending_change = None;
            state.revision += 1;
            state.record("settings.revoke", "accepted");
            response(socket, 200, json!({"configured":false}))
        }
        ("POST", "/v1/cancel") if req.body.is_empty() => {
            let mut state = state.lock().map_err(|_| ApiError::Io)?;
            let active = state.active.is_some() && state.active_cancellable;
            if let Some(cancel) = &state.active
                && state.active_cancellable
            {
                cancel.cancel();
            }
            state.record("cancel", "requested");
            response(socket, 200, json!({"cancellation_requested":active}))
        }
        ("POST", "/v1/probe") => {
            let Ok(input) = serde_json::from_slice::<Probe>(&req.body) else {
                return error(socket, 400);
            };
            let Ok(endpoints) = input
                .endpoints
                .iter()
                .map(|s| LocalChatEndpoint::parse(s))
                .collect::<Result<Vec<_>, _>>()
            else {
                return error(socket, 400);
            };
            let Ok(approval) = LocalProbeApproval::for_endpoints(endpoints) else {
                return error(socket, 400);
            };
            let (mut op, _) = match begin(state, stop, "probe") {
                Ok(v) => v,
                Err(s) => return error(socket, s),
            };
            match probe_endpoints(
                &approval,
                TransportLimits {
                    total: Duration::from_secs(5),
                    idle: Duration::from_secs(1),
                },
                &op.cancel,
            ) {
                Ok(results) => {
                    op.outcome = "finished";
                    response(
                        socket,
                        200,
                        json!({"candidates":results.into_iter().zip(input.endpoints).map(|(r,endpoint)| match r.models {
                    Ok(models) => json!({"endpoint":endpoint,"models":models}),
                    Err(e) => json!({"endpoint":endpoint,"error":e.code(),"message":e.message()}),
                }).collect::<Vec<_>>()}),
                    )
                }
                Err(e) => {
                    op.outcome = e.code();
                    response(socket, 502, json!({"error":e.code(),"message":e.message()}))
                }
            }
        }
        ("GET", "/v1/models") | ("POST", "/v1/chat") => {
            let chat = if req.method == "POST" {
                match serde_json::from_slice::<Chat>(&req.body) {
                    Ok(input) => Some(input),
                    Err(_) => return error(socket, 400),
                }
            } else {
                None
            };
            let (mut op, config) =
                match begin(state, stop, if chat.is_some() { "chat" } else { "models" }) {
                    Ok(v) => v,
                    Err(s) => return error(socket, s),
                };
            let Some(config) = config else {
                return error(socket, 428);
            };
            if let Some(chat) = chat {
                let messages = chat
                    .messages
                    .iter()
                    .map(|m| {
                        Ok(ChatMessage {
                            role: match m.role.as_str() {
                                "system" => ChatRole::System,
                                "user" => ChatRole::User,
                                "assistant" => ChatRole::Assistant,
                                _ => return Err(()),
                            },
                            content: &m.content,
                        })
                    })
                    .collect::<Result<Vec<_>, ()>>();
                let Ok(messages) = messages else {
                    return error(socket, 400);
                };
                if encode_chat(&chat.model, &messages, chat.max_tokens).is_err() {
                    return error(socket, 400);
                }
                headers(socket, 200, "application/x-ndjson", None)?;
                let mut writable = true;
                let result = config.transport.chat(
                    &config.approval,
                    &chat.model,
                    &messages,
                    chat.max_tokens,
                    &op.cancel,
                    |text| {
                        if writable && event(socket, json!({"type":"text","text":text})).is_err() {
                            writable = false;
                            op.cancel.cancel();
                        }
                    },
                );
                if !writable {
                    op.outcome = "api.client-disconnected";
                    return Err(ApiError::Io);
                }
                op.outcome = match result {
                    Ok(_) => "finished",
                    Err(e) => e.code(),
                };
                event(
                    socket,
                    match result {
                        Ok(finish) => {
                            json!({"type":"complete","finish":match finish { ChatFinish::Stop => "stop", ChatFinish::Length => "length" }})
                        }
                        Err(e) => {
                            json!({"type":"error","code":e.code(),"message":e.message(),"partial":true})
                        }
                    },
                )
            } else {
                match config.transport.models(&config.approval, &op.cancel) {
                    Ok(models) => {
                        op.outcome = "finished";
                        response(socket, 200, json!({"models":models}))
                    }
                    Err(e) => {
                        op.outcome = e.code();
                        response(socket, 502, json!({"error":e.code(),"message":e.message()}))
                    }
                }
            }
        }
        _ => error(socket, 404),
    }
}

fn event(socket: &mut TcpStream, value: Value) -> Result<(), ApiError> {
    let mut bytes = Zeroizing::new(serde_json::to_vec(&value).map_err(|_| ApiError::Io)?);
    bytes.push(b'\n');
    write(socket, &bytes)
}
