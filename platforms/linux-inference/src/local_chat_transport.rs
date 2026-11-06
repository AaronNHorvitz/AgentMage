// SPDX-License-Identifier: Apache-2.0
//! Bounded local HTTP transport. Only trusted configuration may mint approval;
//! endpoint parsing, model output and endpoint discovery confer no authority.
//! This text-only path neither activates qualified gateway routes nor runs tools.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use zeroize::{Zeroize, Zeroizing};

use crate::local_chat::{
    ChatError, ChatEvent, ChatFinish, ChatMessage, ChatStream, LocalChatEndpoint, encode_chat,
};

const IO_TICK: Duration = Duration::from_millis(50);
const MAX_WIRE: usize = 5 * 1024 * 1024;
const MAX_HEADERS: usize = 16 * 1024;
const MAX_MODELS_BODY: usize = 256 * 1024;

/// Fixed, content-free failures. Partial streamed text is never success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// Invalid credential, limit or discovery configuration.
    Configuration,
    /// Approval does not name this exact address and API base path.
    Approval,
    /// The caller cancelled the operation, including from a streaming callback.
    Cancelled,
    /// Overall or idle deadline expired.
    Timeout,
    /// Connection refused, reset or another socket error; never includes OS text.
    Connection,
    /// The endpoint rejected its credential (401 or 403).
    Authentication,
    /// A redirect was refused without contacting its destination.
    Redirect,
    /// Another non-200 status; the error body is never parsed or displayed.
    Status,
    /// Invalid or unsupported HTTP framing, header or content type.
    Http,
    /// Wire, header, body or model-list ceiling exceeded.
    Limit,
    /// Bounded chat protocol error; never includes upstream text.
    Chat(ChatError),
}

impl TransportError {
    /// Stable display code, safe without conversation, key or upstream bytes.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Configuration => "chat.transport.configuration",
            Self::Approval => "chat.transport.approval",
            Self::Cancelled => "chat.transport.cancelled",
            Self::Timeout => "chat.transport.timeout",
            Self::Connection => "chat.transport.connection",
            Self::Authentication => "chat.transport.authentication",
            Self::Redirect => "chat.transport.redirect-refused",
            Self::Status => "chat.transport.status",
            Self::Http => "chat.transport.http",
            Self::Limit => "chat.transport.limit",
            Self::Chat(error) => error.code(),
        }
    }

    /// Plain-language guidance without a key, endpoint, prompt or upstream text.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Configuration => "Check the key and connection limits.",
            Self::Approval => "Approve this exact local endpoint before connecting.",
            Self::Cancelled => "The connection was cancelled. Any earlier text is partial.",
            Self::Timeout => "The local server took too long. Any earlier text is partial.",
            Self::Connection => {
                "The local server could not be reached. Check its address and port."
            }
            Self::Authentication => "The local server refused the key. Check the configured key.",
            Self::Redirect => {
                "The server redirected the request. Configure its final local API address."
            }
            Self::Status => "The local server rejected the request. Check that its model is ready.",
            Self::Http => "The local server returned an unsupported or malformed HTTP response.",
            Self::Limit => {
                "The server response exceeded a size limit. Any earlier text is partial."
            }
            Self::Chat(ChatError::Endpoint) => {
                "Configure a local HTTP endpoint with an explicit port."
            }
            Self::Chat(ChatError::Request) => "Check the model, messages and output token limit.",
            Self::Chat(_) => {
                "The streamed answer was invalid or incomplete. Any earlier text is partial."
            }
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for TransportError {}
impl From<ChatError> for TransportError {
    fn from(error: ChatError) -> Self {
        Self::Chat(error)
    }
}

/// Owned bearer credential, without Debug, Clone, serialization or an accessor.
/// Its owned allocation and transport request buffer are zeroized on drop.
pub struct BearerKey(Zeroizing<String>);

impl BearerKey {
    /// Takes ownership; rejects empty, oversized and header-injecting values.
    /// The caller must erase any earlier copies and must not log the input.
    pub fn new(value: String) -> Result<Self, TransportError> {
        let value = Zeroizing::new(value);
        if value.is_empty()
            || value.len() > 4096
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~+/=".contains(&b))
        {
            return Err(TransportError::Configuration);
        }
        Ok(Self(value))
    }
}

/// Exact destination approval minted only by trusted application configuration
/// after the person chooses a local model endpoint for conversations. Never expose this constructor
/// as a model tool or interpret discovery as approval to send conversation data.
#[derive(Clone)]
pub struct LocalChatApproval(LocalChatEndpoint);

impl LocalChatApproval {
    /// Records the person's explicit endpoint choice for conversations.
    #[must_use]
    pub fn for_endpoint(endpoint: LocalChatEndpoint) -> Self {
        Self(endpoint)
    }
}

/// Independent total and inactivity bounds; the total cannot exceed five minutes.
#[derive(Clone, Copy, Debug)]
pub struct TransportLimits {
    /// Wall time across connection, request writes, headers and entire response.
    pub total: Duration,
    /// Maximum inactivity while sending or receiving; never extends total time.
    pub idle: Duration,
}

impl Default for TransportLimits {
    fn default() -> Self {
        Self {
            total: Duration::from_secs(120),
            idle: Duration::from_secs(30),
        }
    }
}

impl TransportLimits {
    fn validate(self) -> Result<(), TransportError> {
        if self.total.is_zero()
            || self.total > Duration::from_secs(300)
            || self.idle.is_zero()
            || self.idle > self.total
        {
            return Err(TransportError::Configuration);
        }
        Ok(())
    }
}

/// Shareable one-way cancellation. No worker threads or child processes are started.
#[derive(Clone, Default)]
pub struct ChatCancellation(Arc<AtomicBool>);

impl ChatCancellation {
    /// Requests cancellation; an in-flight socket observes it within an I/O tick
    /// (50 ms, subject to scheduler delay). Caller callbacks must return promptly.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    /// Whether cancellation has been requested. This observes the flag only;
    /// it grants no authority and performs no I/O.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    fn check(&self) -> Result<(), TransportError> {
        if self.is_cancelled() {
            Err(TransportError::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Credential-owning connection configuration; construction performs no I/O.
/// Each operation opens exactly one socket to the approved numeric address,
/// bypassing DNS, proxies, ambient credentials, cookies, redirects and retries.
pub struct LocalChatTransport {
    endpoint: LocalChatEndpoint,
    credential: Option<BearerKey>,
    limits: TransportLimits,
}

impl LocalChatTransport {
    /// Configures an explicit local endpoint. Credentials belong to this exact
    /// instance; replacing the destination requires a new transport and approval.
    pub fn new(
        endpoint: LocalChatEndpoint,
        credential: Option<BearerKey>,
        limits: TransportLimits,
    ) -> Result<Self, TransportError> {
        limits.validate()?;
        Ok(Self {
            endpoint,
            credential,
            limits,
        })
    }

    fn authorized(&self, approval: &LocalChatApproval) -> Result<(), TransportError> {
        if self.endpoint != approval.0 {
            return Err(TransportError::Approval);
        }
        Ok(())
    }

    /// First-run connection test and model listing for the configured endpoint.
    /// IDs are bounded untrusted data, never instructions or permission changes.
    /// Empty lists are valid and mean no selectable model, not a chat success.
    pub fn models(
        &self,
        approval: &LocalChatApproval,
        cancel: &ChatCancellation,
    ) -> Result<Vec<String>, TransportError> {
        self.authorized(approval)?;
        let mut wire = self.request(
            "GET",
            &self.endpoint.models_path(),
            "application/json",
            &[],
            cancel,
        )?;
        let framing = wire.headers("application/json")?;
        let mut body = Zeroizing::new(Vec::new());
        wire.body(framing, MAX_MODELS_BODY, |bytes| {
            body.extend_from_slice(bytes);
            Ok(())
        })?;
        let value: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| TransportError::Http)?;
        let data = value
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or(TransportError::Http)?;
        if data.len() > 256 {
            return Err(TransportError::Limit);
        }
        let mut models = Vec::new();
        let mut seen = BTreeSet::new();
        for item in data {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or(TransportError::Http)?;
            if id.is_empty()
                || id.len() > 256
                || id.chars().any(char::is_control)
                || !seen.insert(id)
            {
                return Err(TransportError::Http);
            }
            models.push(id.to_owned());
        }
        wire.check()?;
        Ok(models)
    }

    /// Streams untrusted plain text before completion. The callback receives only
    /// text: success/finish is returned only after valid HTTP and codec EOF.
    /// On any error all earlier text must be labelled partial by the consumer.
    /// No automatic retry, hidden prompt, tool execution or output logging occurs.
    pub fn chat(
        &self,
        approval: &LocalChatApproval,
        model: &str,
        messages: &[ChatMessage<'_>],
        max_tokens: u32,
        cancel: &ChatCancellation,
        mut text: impl FnMut(&str),
    ) -> Result<ChatFinish, TransportError> {
        self.authorized(approval)?;
        cancel.check()?;
        let body = Zeroizing::new(encode_chat(model, messages, max_tokens)?);
        let mut codec = ChatStream::new(model)?;
        let mut wire = self.request(
            "POST",
            &self.endpoint.chat_path(),
            "text/event-stream",
            &body,
            cancel,
        )?;
        let framing = wire.headers("text/event-stream")?;
        let mut finish = None;
        wire.body(framing, 4 * 1024 * 1024, |bytes| {
            for event in codec.feed(bytes)? {
                match event {
                    ChatEvent::Text(value) => text(&value),
                    ChatEvent::Finished(reason) => finish = Some(reason),
                }
                cancel.check()?;
            }
            Ok(())
        })?;
        codec.end()?;
        wire.check()?;
        finish.ok_or(ChatError::Incomplete.into())
    }

    fn request<'a>(
        &self,
        method: &str,
        path: &str,
        accept: &str,
        body: &[u8],
        cancel: &'a ChatCancellation,
    ) -> Result<Wire<'a>, TransportError> {
        let started = Instant::now();
        cancel.check()?;
        let stream =
            TcpStream::connect_timeout(&self.endpoint.address(), IO_TICK.min(self.limits.total))
                .map_err(|e| io_error(&e))?;
        let mut wire = Wire {
            stream,
            cancel,
            limits: self.limits,
            started,
            progress: Instant::now(),
            buffer: [0; 4096],
            pos: 0,
            end: 0,
            wire_bytes: 0,
        };
        wire.check()?;
        let mut header = Zeroizing::new(format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAccept: {accept}\r\nConnection: close\r\n",
            self.endpoint.address()
        ));
        if let Some(key) = &self.credential {
            // Reserve the entire remaining header before copying the secret;
            // later appends must not reallocate an allocation containing it.
            header.reserve(key.0.len() + 128);
            header.push_str("Authorization: Bearer ");
            header.push_str(&key.0);
            header.push_str("\r\n");
        }
        if method == "POST" {
            header.push_str("Content-Type: application/json\r\n");
        }
        header.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
        wire.write_all(header.as_bytes())?;
        wire.write_all(body)?;
        Ok(wire)
    }
}

/// Probe result for one explicitly allowed candidate; no credential or chat is
/// sent. A successful probe is not authorization for a subsequent conversation.
pub struct EndpointProbe {
    /// Numeric destination and base path tested.
    pub endpoint: LocalChatEndpoint,
    /// Listing result or content-free failure; no endpoint is chosen silently.
    pub models: Result<Vec<String>, TransportError>,
}

/// Separate probe-only permission: cannot authorize chat or forward a credential.
pub struct LocalProbeApproval(Vec<LocalChatEndpoint>);

impl LocalProbeApproval {
    /// Records up to eight candidates the person permits the application to test.
    /// This must come from trusted configuration, never model output.
    pub fn for_endpoints(endpoints: Vec<LocalChatEndpoint>) -> Result<Self, TransportError> {
        if endpoints.is_empty() || endpoints.len() > 8 {
            return Err(TransportError::Configuration);
        }
        Ok(Self(endpoints))
    }
}

/// Tests up to eight user-approved discovery candidates without credentials,
/// DNS, port scans or environment reads. One total budget covers all candidates.
/// Cancellation aborts discovery; ordinary per-candidate failures are retained.
pub fn probe_endpoints(
    approval: &LocalProbeApproval,
    limits: TransportLimits,
    cancel: &ChatCancellation,
) -> Result<Vec<EndpointProbe>, TransportError> {
    limits.validate()?;
    let started = Instant::now();
    let mut results = Vec::new();
    for endpoint in &approval.0 {
        cancel.check()?;
        let remaining = limits
            .total
            .checked_sub(started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or(TransportError::Timeout)?;
        let connection = LocalChatTransport::new(
            endpoint.clone(),
            None,
            TransportLimits {
                total: remaining,
                idle: limits.idle.min(remaining),
            },
        )?;
        // Internal listing authority only: never returned to the caller.
        let models = connection.models(&LocalChatApproval(endpoint.clone()), cancel);
        cancel.check()?;
        results.push(EndpointProbe {
            endpoint: endpoint.clone(),
            models,
        });
    }
    Ok(results)
}

fn io_error(error: &std::io::Error) -> TransportError {
    match error.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => TransportError::Timeout,
        _ => TransportError::Connection,
    }
}

enum Framing {
    Close,
    Length(usize),
    Chunked,
}

struct Wire<'a> {
    stream: TcpStream,
    cancel: &'a ChatCancellation,
    limits: TransportLimits,
    started: Instant,
    progress: Instant,
    buffer: [u8; 4096],
    pos: usize,
    end: usize,
    wire_bytes: usize,
}

impl Drop for Wire<'_> {
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
        self.buffer.zeroize();
    }
}

impl Wire<'_> {
    fn check(&self) -> Result<Duration, TransportError> {
        self.cancel.check()?;
        let total = self
            .limits
            .total
            .checked_sub(self.started.elapsed())
            .filter(|d| !d.is_zero());
        let idle = self
            .limits
            .idle
            .checked_sub(self.progress.elapsed())
            .filter(|d| !d.is_zero());
        Ok(total
            .ok_or(TransportError::Timeout)?
            .min(idle.ok_or(TransportError::Timeout)?)
            .min(IO_TICK))
    }

    fn write_all(&mut self, mut bytes: &[u8]) -> Result<(), TransportError> {
        while !bytes.is_empty() {
            self.stream
                .set_write_timeout(Some(self.check()?))
                .map_err(|e| io_error(&e))?;
            match self.stream.write(bytes) {
                Ok(0) => return Err(TransportError::Connection),
                Ok(n) => {
                    bytes = &bytes[n..];
                    self.progress = Instant::now();
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(io_error(&e)),
            }
        }
        Ok(())
    }

    fn fill(&mut self) -> Result<bool, TransportError> {
        self.check()?;
        if self.pos < self.end {
            return Ok(true);
        }
        loop {
            self.stream
                .set_read_timeout(Some(self.check()?))
                .map_err(|e| io_error(&e))?;
            let allowance = MAX_WIRE - self.wire_bytes;
            if allowance == 0 {
                return Err(TransportError::Limit);
            }
            let capacity = self.buffer.len().min(allowance);
            match self.stream.read(&mut self.buffer[..capacity]) {
                Ok(n) => {
                    self.wire_bytes += n;
                    self.pos = 0;
                    self.end = n;
                    if n > 0 {
                        self.progress = Instant::now();
                    }
                    return Ok(n != 0);
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(io_error(&e)),
            }
        }
    }

    fn byte(&mut self) -> Result<u8, TransportError> {
        if !self.fill()? {
            return Err(TransportError::Http);
        }
        let b = self.buffer[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn line(&mut self, remaining: &mut usize) -> Result<String, TransportError> {
        let mut line = Vec::new();
        loop {
            if *remaining == 0 {
                return Err(TransportError::Limit);
            }
            *remaining -= 1;
            let b = self.byte()?;
            if b == b'\r' {
                if *remaining == 0 {
                    return Err(TransportError::Limit);
                }
                *remaining -= 1;
                if self.byte()? != b'\n' {
                    return Err(TransportError::Http);
                }
                return String::from_utf8(line).map_err(|_| TransportError::Http);
            }
            if b != b'\t' && !(0x20..=0x7e).contains(&b) {
                return Err(TransportError::Http);
            }
            line.push(b);
        }
    }

    fn headers(&mut self, media: &str) -> Result<Framing, TransportError> {
        let mut remaining = MAX_HEADERS;
        let status = self.line(&mut remaining)?;
        let mut words = status.splitn(3, ' ');
        if !matches!(words.next(), Some("HTTP/1.1" | "HTTP/1.0")) {
            return Err(TransportError::Http);
        }
        match words.next() {
            Some("200") => {}
            Some("401" | "403") => return Err(TransportError::Authentication),
            Some(code) if code.len() == 3 && code.bytes().all(|b| b.is_ascii_digit()) => {
                return Err(if code.starts_with('3') {
                    TransportError::Redirect
                } else {
                    TransportError::Status
                });
            }
            _ => return Err(TransportError::Http),
        }
        let mut seen = BTreeSet::new();
        let mut length = None;
        let mut chunked = false;
        let mut content_type = false;
        loop {
            let line = self.line(&mut remaining)?;
            if line.is_empty() {
                break;
            }
            let (name, value) = line.split_once(':').ok_or(TransportError::Http)?;
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
            {
                return Err(TransportError::Http);
            }
            let name = name.to_ascii_lowercase();
            let value = value.trim_matches([' ', '\t']);
            if matches!(
                name.as_str(),
                "content-length" | "transfer-encoding" | "content-type" | "content-encoding"
            ) && !seen.insert(name.clone())
            {
                return Err(TransportError::Http);
            }
            match name.as_str() {
                "content-length" => {
                    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(TransportError::Http);
                    }
                    length = Some(value.parse::<usize>().map_err(|_| TransportError::Limit)?);
                }
                "transfer-encoding" => {
                    if !value.eq_ignore_ascii_case("chunked") {
                        return Err(TransportError::Http);
                    }
                    chunked = true;
                }
                "content-type" => {
                    let mut parts = value.split(';');
                    if !parts
                        .next()
                        .unwrap_or("")
                        .trim()
                        .eq_ignore_ascii_case(media)
                    {
                        return Err(TransportError::Http);
                    }
                    for part in parts {
                        let (key, value) =
                            part.trim().split_once('=').ok_or(TransportError::Http)?;
                        if !key.trim().eq_ignore_ascii_case("charset")
                            || !matches!(
                                value.trim().to_ascii_lowercase().as_str(),
                                "utf-8" | "\"utf-8\""
                            )
                        {
                            return Err(TransportError::Http);
                        }
                    }
                    content_type = true;
                }
                "content-encoding" if !value.eq_ignore_ascii_case("identity") => {
                    return Err(TransportError::Http);
                }
                _ => {}
            }
        }
        if !content_type || (chunked && length.is_some()) {
            return Err(TransportError::Http);
        }
        Ok(if chunked {
            Framing::Chunked
        } else if let Some(n) = length {
            Framing::Length(n)
        } else {
            Framing::Close
        })
    }

    fn exact(
        &mut self,
        mut count: usize,
        consume: &mut impl FnMut(&[u8]) -> Result<(), TransportError>,
    ) -> Result<(), TransportError> {
        while count > 0 {
            if !self.fill()? {
                return Err(TransportError::Http);
            }
            let n = count.min(self.end - self.pos);
            consume(&self.buffer[self.pos..self.pos + n])?;
            self.pos += n;
            count -= n;
        }
        Ok(())
    }

    fn body(
        &mut self,
        framing: Framing,
        max: usize,
        mut consume: impl FnMut(&[u8]) -> Result<(), TransportError>,
    ) -> Result<(), TransportError> {
        match framing {
            Framing::Length(n) => {
                if n > max {
                    return Err(TransportError::Limit);
                }
                self.exact(n, &mut consume)?;
            }
            Framing::Close => {
                let mut count = 0;
                while self.fill()? {
                    count += self.end - self.pos;
                    if count > max {
                        return Err(TransportError::Limit);
                    }
                    consume(&self.buffer[self.pos..self.end])?;
                    self.pos = self.end;
                }
                return Ok(());
            }
            Framing::Chunked => {
                let mut count = 0_usize;
                loop {
                    let line = self.line(&mut 128)?;
                    if line.is_empty()
                        || line.len() > 16
                        || !line.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        return Err(TransportError::Http);
                    }
                    let size =
                        usize::from_str_radix(&line, 16).map_err(|_| TransportError::Limit)?;
                    count = count
                        .checked_add(size)
                        .filter(|n| *n <= max)
                        .ok_or(TransportError::Limit)?;
                    if size == 0 {
                        // Trailers and extensions are deliberately unsupported.
                        if self.byte()? != b'\r' || self.byte()? != b'\n' {
                            return Err(TransportError::Http);
                        }
                        break;
                    }
                    self.exact(size, &mut consume)?;
                    if self.byte()? != b'\r' || self.byte()? != b'\n' {
                        return Err(TransportError::Http);
                    }
                }
            }
        }
        // Connection: close requests require actual EOF, even after a framed body.
        // Late bytes cannot turn a provisional codec finish into false success.
        if self.fill()? {
            return Err(TransportError::Http);
        }
        Ok(())
    }
}
