// SPDX-License-Identifier: Apache-2.0
//! Text-only local API. Control credentials authorize explicit settings choices;
//! ordinary clients cannot turn discovery or model output into authority.

use std::collections::VecDeque;
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
use std::time::{Duration, Instant};

use rustix::fs::{Mode, OFlags, open, openat};
use serde::Deserialize;
use serde_json::{Value, json};
use zeroize::Zeroizing;

use agentmage_capability_repository_map::coding_proposal::{
    CODING_PROPOSAL_INSTRUCTION, CodingSource, check_coding_proposal, prepare_coding_proposal,
};

use crate::local_chat::{ChatFinish, ChatMessage, ChatRole, LocalChatEndpoint, encode_chat};
use crate::local_chat_transport::{
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

#[derive(Default)]
struct State {
    configured: Option<Arc<Configuration>>,
    active: Option<ChatCancellation>,
    revision: u64,
    sequence: u64,
    activity: VecDeque<Value>,
}

impl State {
    fn record(&mut self, action: &'static str, outcome: &'static str) {
        self.sequence += 1;
        self.activity.push_back(json!({"sequence":self.sequence,"settings_revision":self.revision,"endpoint":self.configured.as_ref().map(|c|&c.endpoint),"action":action,"outcome":outcome}));
        if self.activity.len() > 128 {
            self.activity.pop_front();
        }
    }
}

/// Serves four bounded connection workers on a caller-owned loopback listener.
/// All routes require bearer authentication and exact numeric Host; browser
/// origins/cookies/fetch metadata are refused and no CORS headers are emitted.
/// Only one upstream operation is active, leaving workers for cancellation.
/// On shutdown, cancels the active operation and joins every owned worker.
pub fn serve(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
) -> Result<(), ApiError> {
    let address = listener.local_addr().map_err(|_| ApiError::Io)?;
    if !address.ip().is_loopback() {
        return Err(ApiError::Configuration);
    }
    listener.set_nonblocking(true).map_err(|_| ApiError::Io)?;
    let state = Mutex::new(State::default());
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
            "This request is not allowed. Settings require the separate control credential and an explicit endpoint choice."
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
    if matches!(req.path.as_str(), "/v1/settings" | "/v1/probe") && !req.control {
        return error(socket, 403);
    }
    match route {
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
            // This is an effect-free proposal over caller-supplied snapshots.
            // Source paths are data, never resolved against the service host.
            let context =
                match prepare_coding_proposal(&input.task, &input.files, input.max_message_bytes) {
                    Ok(context) => context,
                    Err(e) => return response(socket, 400, json!({"error":e.code()})),
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
                return error(socket, 400);
            }
            let (mut op, config) = match begin(state, stop, "code.proposal") {
                Ok(operation) => operation,
                Err(status) => return error(socket, status),
            };
            let Some(config) = config else {
                return error(socket, 428);
            };
            headers(socket, 200, "application/x-ndjson", None)?;
            event(
                socket,
                json!({"type":"context", "version":1,
                "message_bytes":context.message_bytes(), "files":context.outlines(),
                "excerpts":context.excerpts(), "authority":"proposal-only"}),
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
                        "applied":false,"tested":false,"authority":"proposal-only"}),
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
        ("GET", "/v1/status") => {
            let state = state.lock().map_err(|_| ApiError::Io)?;
            response(
                socket,
                200,
                json!({"version":1,"configured":state.configured.is_some(),"settings_revision":state.revision,"busy":state.active.is_some()}),
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
            state.revision += 1;
            state.record("settings.revoke", "accepted");
            response(socket, 200, json!({"configured":false}))
        }
        ("POST", "/v1/cancel") if req.body.is_empty() => {
            let mut state = state.lock().map_err(|_| ApiError::Io)?;
            let active = state.active.is_some();
            if let Some(cancel) = &state.active {
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
