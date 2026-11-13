// SPDX-License-Identifier: Apache-2.0
//! Bounded local HTTP transport. Only trusted configuration may mint approval;
//! endpoint parsing, model output and endpoint discovery confer no authority.
//! This text-only path neither activates qualified gateway routes nor runs tools.

use std::cell::Cell;
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    EndpointModelBody, EndpointModelContext, EndpointModelFinish, EndpointModelLimits,
    EndpointModelRequest, EndpointModelResult, EndpointModelSelection, ModelOperationControl,
    ModelOperationStop,
};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::local_chat::{
    ChatError, ChatEvent, ChatFinish, ChatMessage, ChatResponseFormat, ChatResponseMode,
    ChatStream, ChatThinkingMode, ChatUsage, LocalChatEndpoint, MAX_STREAM_BYTES, MAX_TEXT_BYTES,
    decode_complete_chat, encode_chat_request,
};

const IO_TICK: Duration = Duration::from_millis(50);
const MAX_WIRE: usize = 5 * 1024 * 1024;
const MAX_HEADERS: usize = 16 * 1024;
const MAX_MODELS_BODY: usize = 256 * 1024;

// Hash and count the exact serde encoding without allocating another copy of
// private messages. Only the frozen request body retains their serialized text.
#[derive(Default)]
struct MessageFingerprint {
    hash: Sha256,
    bytes: usize,
}

impl Write for MessageFingerprint {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("chat message size"))?;
        self.hash.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Fixed, content-free failures. Partial streamed text is never success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// Invalid credential, limit or discovery configuration.
    Configuration,
    /// The borrowed run owner stopped this operation; never an acknowledgement.
    OperationStopped(ModelOperationStop),
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
            Self::OperationStopped(stop) => stop.code(),
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
            Self::OperationStopped(_) => {
                "The run stopped this connection. Any earlier text is partial."
            }
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
    response_mode: ChatResponseMode,
    response_format: ChatResponseFormat,
    thinking_mode: ChatThinkingMode,
}

/// Completion confirmed at HTTP and codec EOF. Counts remain server reports;
/// they never qualify a model or supply verified local resource observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatCompletion {
    /// Normal stop or visible output limit.
    pub finish: ChatFinish,
    /// Optional final provider-reported usage; absence is never zero.
    pub usage: Option<ChatUsage>,
}

/// Shared selected-endpoint request observations, without credentials or authority.
/// The historical name remains an alias for existing transport consumers.
pub type EndpointChatBinding = EndpointModelRequest;

/// An inert, single-use chat request bound to its credential-owning transport.
/// No Debug, Clone or deserialization can expose/reconstruct the private body.
/// Preparing it performs no network I/O; dropping it erases the encoded body.
pub struct PreparedEndpointChat<'a> {
    transport: PreparedTransport<'a>,
    binding: EndpointChatBinding,
    body: Zeroizing<Vec<u8>>,
}

// Preparation either borrows the original configuration or retains that exact
// immutable instance. Neither variant clones credentials or resolves settings.
enum PreparedTransport<'a> {
    Borrowed(&'a LocalChatTransport),
    Shared(Arc<LocalChatTransport>),
}

impl std::ops::Deref for PreparedTransport<'_> {
    type Target = LocalChatTransport;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(transport) => transport,
            Self::Shared(transport) => transport,
        }
    }
}

impl PreparedEndpointChat<'_> {
    /// Narrow this frozen request's decoded text ceiling before dispatch.
    /// Zero or any increase refuses without connecting. The body, destination,
    /// credentials, wire/body limits and original deadlines stay unchanged.
    /// The transport rejects an overflowing fragment before forwarding it.
    pub fn with_response_text_limit(mut self, bytes: usize) -> Result<Self, TransportError> {
        if bytes == 0 || bytes > self.binding.limits().response_text_limit_bytes {
            return Err(TransportError::Configuration);
        }
        let mut limits = self.binding.limits().clone();
        limits.response_text_limit_bytes = bytes;
        self.binding = EndpointModelRequest::from_prepared_body(
            self.transport.model_selection(self.binding.model()),
            self.binding.context.clone(),
            self.binding.body().clone(),
            limits,
        );
        Ok(self)
    }

    /// Describes exactly the immutable JSON body that execute will send.
    #[must_use]
    pub const fn binding(&self) -> &EndpointChatBinding {
        &self.binding
    }

    /// Sends this request once, through its original transport and approval.
    /// Text remains partial until valid HTTP and codec EOF. Failure never yields
    /// a completed usage observation and never retries. Cancellation is rechecked
    /// before connecting, including when requested after preparation.
    pub fn execute(
        self,
        cancel: &ChatCancellation,
        text: impl FnMut(&str),
    ) -> Result<ChatCompletion, TransportError> {
        self.execute_recorded(cancel, text)
            .map(|result| ChatCompletion {
                finish: match result.finish {
                    EndpointModelFinish::Stop => ChatFinish::Stop,
                    EndpointModelFinish::OutputTokenLimit => ChatFinish::Length,
                },
                usage: result.token_accounting.provider_reported(),
            })
    }

    /// Sends the same single-use request and records a completed endpoint exchange.
    /// Only valid HTTP and codec EOF produce a result. Text callbacks remain
    /// provisional; failures, cancellation and malformed streams return no result.
    /// Time is local dispatch time, not server/GPU time. No memory is inferred.
    pub fn execute_recorded(
        self,
        cancel: &ChatCancellation,
        text: impl FnMut(&str),
    ) -> Result<EndpointModelResult, TransportError> {
        self.execute_inner(cancel, None, text)
    }

    /// Dispatches under the existing run owner's cooperative control as well as
    /// the transport's original deadlines and cancellation. A stop yields no
    /// completed observation or cancellation acknowledgement. Control is checked
    /// before connecting, between socket waits and fragments, and after EOF.
    /// It cannot change the frozen body, transport, credentials or approval.
    pub fn execute_recorded_controlled(
        self,
        cancel: &ChatCancellation,
        control: &dyn ModelOperationControl,
        text: impl FnMut(&str),
    ) -> Result<EndpointModelResult, TransportError> {
        self.execute_inner(cancel, Some(control), text)
    }

    fn execute_inner(
        self,
        cancel: &ChatCancellation,
        control: Option<&dyn ModelOperationControl>,
        mut text: impl FnMut(&str),
    ) -> Result<EndpointModelResult, TransportError> {
        let started = Instant::now();
        cancel.check()?;
        let control = DispatchControl::new(control)?;
        let mut response_hash = Sha256::new();
        let mut response_bytes = 0;
        let completion = self.transport.chat_encoded(
            &self.binding,
            &self.body,
            cancel,
            &control,
            |fragment| {
                response_hash.update(fragment.as_bytes());
                response_bytes += fragment.len(); // Transport enforces the frozen text ceiling.
                text(fragment);
            },
        )?;
        cancel.check()?;
        control.remaining()?;
        let elapsed = started.elapsed();
        Ok(EndpointModelResult::from_completed_response(
            self.binding,
            match completion.finish {
                ChatFinish::Stop => EndpointModelFinish::Stop,
                ChatFinish::Length => EndpointModelFinish::OutputTokenLimit,
            },
            response_hash
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            response_bytes,
            elapsed,
            completion.usage,
        ))
    }
}

impl LocalChatTransport {
    /// Describes this transport's actual destination and a requested model.
    /// This inert selection contains no credential, approval or model admission.
    /// Preparation still validates the model and exact destination approval.
    #[must_use]
    pub fn model_selection(&self, model: &str) -> EndpointModelSelection {
        EndpointModelSelection {
            endpoint: self.endpoint.base_url(),
            model: model.to_owned(),
        }
    }

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
            response_mode: ChatResponseMode::Stream,
            response_format: ChatResponseFormat::Text,
            thinking_mode: ChatThinkingMode::ProviderDefault,
        })
    }

    /// Select upstream framing while constructing trusted immutable settings.
    /// This performs no I/O and changes no approval, byte ceiling or deadline.
    /// Complete mode has no incremental progress, so idle time still applies to
    /// the wait for its body. No fallback or implicit retry is performed.
    #[must_use]
    pub fn with_response_mode(mut self, mode: ChatResponseMode) -> Self {
        self.response_mode = mode;
        self
    }

    /// Effective upstream framing in this immutable transport configuration.
    #[must_use]
    pub const fn response_mode(&self) -> ChatResponseMode {
        self.response_mode
    }

    /// Request JSON-object output while constructing trusted immutable settings.
    /// No prompt is added, bytes repaired, limits raised or retries introduced.
    /// The endpoint may ignore the request; consumers retain their strict checks.
    #[must_use]
    pub fn with_response_format(mut self, format: ChatResponseFormat) -> Self {
        self.response_format = format;
        self
    }

    /// Requested output format; this is not proof of a valid model response.
    #[must_use]
    pub const fn response_format(&self) -> ChatResponseFormat {
        self.response_format
    }

    /// Select an optional endpoint thinking control in trusted configuration.
    /// No prompt, response validation, limit, approval or retry policy changes.
    #[must_use]
    pub fn with_thinking_mode(mut self, mode: ChatThinkingMode) -> Self {
        self.thinking_mode = mode;
        self
    }

    /// Requested thinking control, not proof that the endpoint honored it.
    #[must_use]
    pub const fn thinking_mode(&self) -> ChatThinkingMode {
        self.thinking_mode
    }

    fn authorized(&self, approval: &LocalChatApproval) -> Result<(), TransportError> {
        if self.endpoint != approval.0 {
            return Err(TransportError::Approval);
        }
        Ok(())
    }

    /// Validates exact endpoint approval and encodes an immutable bounded request.
    /// The binding hashes the actual JSON body, excluding HTTP headers/credentials.
    /// The token ceiling is a server request, not a measured tokenizer budget;
    /// optional provider counts remain unverified even after valid completion.
    pub fn prepare_chat(
        &self,
        approval: &LocalChatApproval,
        model: &str,
        messages: &[ChatMessage<'_>],
        max_tokens: u32,
        include_usage: bool,
    ) -> Result<PreparedEndpointChat<'_>, TransportError> {
        self.authorized(approval)?;
        let body = Zeroizing::new(encode_chat_request(
            model,
            messages,
            max_tokens,
            include_usage,
            self.response_mode,
            self.response_format,
            self.thinking_mode,
        )?);
        // Use the same serde message-array encoding as the frozen body. This
        // observes JSON bytes and roles, never a server's rendered prompt tokens.
        let mut fingerprint = MessageFingerprint::default();
        serde_json::to_writer(&mut fingerprint, messages)
            .map_err(|_| TransportError::Chat(ChatError::Request))?;
        let binding = EndpointModelRequest::from_prepared_body(
            self.model_selection(model),
            EndpointModelContext {
                message_count: messages.len(),
                serialized_messages_bytes: fingerprint.bytes,
                serialized_messages_sha256: fingerprint
                    .hash
                    .finalize()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
            },
            EndpointModelBody {
                request_body_sha256: Sha256::digest(&*body)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
                request_body_bytes: body.len(),
            },
            EndpointModelLimits {
                requested_max_output_tokens: max_tokens,
                provider_usage_requested: include_usage,
                total_timeout: self.limits.total,
                idle_timeout: self.limits.idle,
                response_wire_limit_bytes: MAX_WIRE,
                response_body_limit_bytes: MAX_STREAM_BYTES,
                response_text_limit_bytes: MAX_TEXT_BYTES,
            },
        );
        Ok(PreparedEndpointChat {
            transport: PreparedTransport::Borrowed(self),
            binding,
            body,
        })
    }

    /// Prepares the same single-use request while retaining this exact immutable
    /// credential owner. Approval, encoding and limits use the ordinary path;
    /// the returned request can outlive the caller's settings reference. No
    /// credential copy, socket, background worker or retry is introduced.
    pub fn prepare_chat_shared(
        self: Arc<Self>,
        approval: &LocalChatApproval,
        model: &str,
        messages: &[ChatMessage<'_>],
        max_tokens: u32,
        include_usage: bool,
    ) -> Result<PreparedEndpointChat<'static>, TransportError> {
        let PreparedEndpointChat { binding, body, .. } =
            self.prepare_chat(approval, model, messages, max_tokens, include_usage)?;
        Ok(PreparedEndpointChat {
            transport: PreparedTransport::Shared(self),
            binding,
            body,
        })
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
        let control = DispatchControl::uncontrolled();
        let mut wire = self.request(
            "GET",
            &self.endpoint.models_path(),
            "application/json",
            &[],
            cancel,
            &control,
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
        text: impl FnMut(&str),
    ) -> Result<ChatFinish, TransportError> {
        self.authorized(approval)?;
        cancel.check()?;
        self.prepare_chat(approval, model, messages, max_tokens, false)?
            .execute(cancel, text)
            .map(|r| r.finish)
    }

    /// Opt-in provider usage. Sends `stream_options.include_usage`; servers
    /// may omit usage, which remains None. Refusal never retries without the
    /// option. No usage escapes on partial, cancelled or invalid completion.
    pub fn chat_with_usage(
        &self,
        approval: &LocalChatApproval,
        model: &str,
        messages: &[ChatMessage<'_>],
        max_tokens: u32,
        cancel: &ChatCancellation,
        text: impl FnMut(&str),
    ) -> Result<ChatCompletion, TransportError> {
        self.authorized(approval)?;
        cancel.check()?;
        self.prepare_chat(approval, model, messages, max_tokens, true)?
            .execute(cancel, text)
    }

    fn chat_encoded(
        &self,
        binding: &EndpointModelRequest,
        body: &[u8],
        cancel: &ChatCancellation,
        control: &DispatchControl<'_>,
        mut text: impl FnMut(&str),
    ) -> Result<ChatCompletion, TransportError> {
        if self.response_mode == ChatResponseMode::Complete {
            let mut wire = self.request(
                "POST",
                &self.endpoint.chat_path(),
                "application/json",
                body,
                cancel,
                control,
            )?;
            let framing = wire.headers("application/json")?;
            let mut response = Zeroizing::new(Vec::new());
            wire.body(framing, MAX_STREAM_BYTES, |bytes| {
                response.extend_from_slice(bytes);
                Ok(())
            })?;
            wire.check()?;
            let (content, finish, usage) = decode_complete_chat(
                &response,
                binding.model(),
                binding
                    .limits()
                    .provider_usage_requested
                    .then_some(binding.limits().requested_max_output_tokens),
            )?;
            if content.len() > binding.limits().response_text_limit_bytes {
                return Err(TransportError::Limit);
            }
            wire.check()?;
            if !content.is_empty() {
                text(&content);
            }
            wire.check()?;
            return Ok(ChatCompletion { finish, usage });
        }
        let mut codec = if binding.limits().provider_usage_requested {
            ChatStream::with_usage(
                binding.model(),
                binding.limits().requested_max_output_tokens,
            )?
        } else {
            ChatStream::new(binding.model())?
        };
        let mut wire = self.request(
            "POST",
            &self.endpoint.chat_path(),
            "text/event-stream",
            body,
            cancel,
            control,
        )?;
        let framing = wire.headers("text/event-stream")?;
        let mut finish = None;
        let mut text_bytes = 0_usize;
        wire.body(framing, MAX_STREAM_BYTES, |bytes| {
            for event in codec.feed(bytes)? {
                cancel.check()?;
                control.remaining()?;
                match event {
                    ChatEvent::Text(value) => {
                        text_bytes = text_bytes
                            .checked_add(value.len())
                            .filter(|bytes| *bytes <= binding.limits().response_text_limit_bytes)
                            .ok_or(TransportError::Limit)?;
                        text(&value);
                    }
                    ChatEvent::Finished(reason) => finish = Some(reason),
                }
                cancel.check()?;
                control.remaining()?;
            }
            Ok(())
        })?;
        codec.end()?;
        wire.check()?;
        Ok(ChatCompletion {
            finish: finish.ok_or(ChatError::Incomplete)?,
            usage: codec.usage(),
        })
    }

    fn request<'a>(
        &self,
        method: &str,
        path: &str,
        accept: &str,
        body: &[u8],
        cancel: &'a ChatCancellation,
        control: &'a DispatchControl<'a>,
    ) -> Result<Wire<'a>, TransportError> {
        let started = Instant::now();
        cancel.check()?;
        let connect_time = IO_TICK.min(self.limits.total).min(control.remaining()?);
        let stream = TcpStream::connect_timeout(&self.endpoint.address(), connect_time)
            .map_err(|e| io_error(&e))?;
        let mut wire = Wire {
            stream,
            cancel,
            control,
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

// The borrowed control can narrow or stop dispatch, never renew any previously
// observed positive budget. The original transport deadline remains independent.
struct DispatchControl<'a> {
    owner: Option<&'a dyn ModelOperationControl>,
    remaining: Cell<Option<(Instant, Duration)>>,
}

impl<'a> DispatchControl<'a> {
    fn uncontrolled() -> Self {
        Self {
            owner: None,
            remaining: Cell::new(None),
        }
    }

    fn new(owner: Option<&'a dyn ModelOperationControl>) -> Result<Self, TransportError> {
        let started = Instant::now();
        let initial = owner
            .map(|owner| {
                owner
                    .remaining_ms()
                    .map(|ms| (started, Duration::from_millis(ms.get())))
                    .map_err(TransportError::OperationStopped)
            })
            .transpose()?;
        Ok(Self {
            owner,
            remaining: Cell::new(initial),
        })
    }

    fn remaining(&self) -> Result<Duration, TransportError> {
        let Some(owner) = self.owner else {
            return Ok(Duration::MAX);
        };
        let remaining = Duration::from_millis(
            owner
                .remaining_ms()
                .map_err(TransportError::OperationStopped)?
                .get(),
        );
        let now = Instant::now();
        let (started, previous) = self.remaining.get().ok_or(TransportError::Configuration)?;
        let local = previous
            .checked_sub(now.duration_since(started))
            .filter(|d| !d.is_zero())
            .ok_or(TransportError::OperationStopped(
                ModelOperationStop::TimedOut,
            ))?;
        let bounded = remaining.min(local);
        self.remaining.set(Some((now, bounded)));
        Ok(bounded)
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
    control: &'a DispatchControl<'a>,
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
        let run_remaining = self.control.remaining()?;
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
            .min(run_remaining)
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
