//! Bounded chat encoding and incremental event decoding, without transport authority.
//!
//! A parsed endpoint is configuration, not a network grant or model qualification.
//! This module opens no sockets, reads no files and owns no credentials. All model
//! text remains untrusted data; consumers must render it as text and never execute it.

use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

const MAX_REQUEST_BYTES: usize = 256 * 1024;
const MAX_EVENT_BYTES: usize = 64 * 1024;
pub(super) const MAX_STREAM_BYTES: usize = 4 * 1024 * 1024;
pub(super) const MAX_TEXT_BYTES: usize = 1024 * 1024;

/// Content-free connection or protocol error, safe to display without upstream bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatError {
    /// Supply an HTTP loopback address with an explicit nonzero port and API path.
    Endpoint,
    /// The model, conversation or token ceiling is invalid or too large.
    Request,
    /// Upstream bytes exceeded a fixed event, stream or text ceiling.
    Limit,
    /// The stream is malformed, names another model, or proposes tools.
    Protocol,
    /// The server ended without a complete answer and completion marker.
    Incomplete,
    /// An earlier decoding error makes further consumption invalid.
    Failed,
}

impl ChatError {
    /// Stable codes never contain the endpoint, prompt, key or upstream response.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Endpoint => "chat.endpoint.invalid",
            Self::Request => "chat.request.invalid",
            Self::Limit => "chat.response.limit",
            Self::Protocol => "chat.response.invalid",
            Self::Incomplete => "chat.response.incomplete",
            Self::Failed => "chat.response.failed",
        }
    }
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ChatError {}

/// An explicit local HTTP address, canonicalized without DNS or environment lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalChatEndpoint {
    address: SocketAddr,
    api_path: String,
}

impl LocalChatEndpoint {
    /// Accepts loopback IP literals or `localhost`, an explicit port, and an
    /// optional API path (default `/v1`). A final slash is allowed. User info,
    /// queries, fragments, encoded path bytes and traversal are refused.
    /// `localhost` is pinned to IPv4 loopback; no resolver is invoked.
    pub fn parse(input: &str) -> Result<Self, ChatError> {
        if input.len() > 1024 || !input.is_ascii() {
            return Err(ChatError::Endpoint);
        }
        let rest = input.strip_prefix("http://").ok_or(ChatError::Endpoint)?;
        let (authority, path) = rest.split_once('/').unwrap_or((rest, "v1"));
        let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
            bracketed.split_once("]:").ok_or(ChatError::Endpoint)?
        } else {
            authority.rsplit_once(':').ok_or(ChatError::Endpoint)?
        };
        if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ChatError::Endpoint);
        }
        let port: u16 = port.parse().map_err(|_| ChatError::Endpoint)?;
        let ip: IpAddr = if host == "localhost" {
            IpAddr::from([127, 0, 0, 1])
        } else {
            host.parse().map_err(|_| ChatError::Endpoint)?
        };
        // Require bracketed IPv6, and forbid brackets around an IPv4 literal.
        if port == 0 || !ip.is_loopback() || authority.starts_with('[') != ip.is_ipv6() {
            return Err(ChatError::Endpoint);
        }
        if path.starts_with('/') {
            return Err(ChatError::Endpoint);
        }
        let path = path.strip_suffix('/').unwrap_or(path);
        let path = if path.is_empty() { "v1" } else { path };
        if path.split('/').any(|segment| {
            segment.is_empty()
                || segment == "."
                || segment == ".."
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        }) {
            return Err(ChatError::Endpoint);
        }
        Ok(Self {
            address: SocketAddr::new(ip, port),
            api_path: format!("/{path}"),
        })
    }

    /// Exact numeric destination for a future authorized transport, with no DNS.
    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    /// Canonical API base; contains no credentials or query parameters.
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://{}{}", self.address, self.api_path)
    }

    /// Model listing path relative to the configured server.
    #[must_use]
    pub fn models_path(&self) -> String {
        format!("{}/models", self.api_path)
    }

    /// Streaming chat path relative to the configured server.
    #[must_use]
    pub fn chat_path(&self) -> String {
        format!("{}/chat/completions", self.api_path)
    }
}

/// Roles admitted by this text-only chat codec; none can authorize a tool.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    /// Explicit application or user instructions.
    System,
    /// Text supplied by the person.
    User,
    /// Previous untrusted model text.
    Assistant,
}

/// A borrowed message, deliberately without a Debug implementation to avoid logs.
#[derive(Serialize)]
pub struct ChatMessage<'a> {
    /// Conversation role.
    pub role: ChatRole,
    /// Plain text only; attachments and tool calls are separate future contracts.
    pub content: &'a str,
}

fn valid_model(model: &str) -> bool {
    !model.is_empty() && model.len() <= 256 && !model.chars().any(char::is_control)
}

/// Trusted choice of upstream response framing, without changing any grant or
/// response ceiling. Complete responses emit text only after validated EOF.
#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatResponseMode {
    /// Incremental server-sent events; the ordinary conversation default.
    #[default]
    Stream,
    /// One bounded JSON response, avoiding per-token event framing overhead.
    Complete,
}

/// Trusted upstream output request, separate from framing and edit validation.
/// Servers may reject or ignore it; no output gains authority from this setting.
#[derive(Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ChatResponseFormat {
    /// Ordinary conversation; omit the upstream format constraint.
    #[default]
    Text,
    /// Request a JSON object from a supporting endpoint. Consumers still validate.
    JsonObject,
    /// Request a named strict schema from a supporting endpoint. This remains
    /// an untrusted generation result, never a validation or permission receipt.
    JsonSchema(ChatOutputSchema),
}

/// An explicitly chosen, repository-independent output schema. The engine never
/// resolves schema references or reads a file/URL named in schema data.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(try_from = "OutputSchemaInput")]
pub struct ChatOutputSchema {
    name: String,
    schema: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputSchemaInput {
    name: String,
    schema: serde_json::Value,
}

impl TryFrom<OutputSchemaInput> for ChatOutputSchema {
    type Error = ChatError;

    fn try_from(input: OutputSchemaInput) -> Result<Self, Self::Error> {
        Self::new(input.name, input.schema)
    }
}

impl ChatOutputSchema {
    /// Accept an inline object schema and an ASCII name. External/reference
    /// keywords are refused before any request; the provider checks schema
    /// semantics. The existing request byte ceiling also includes this schema.
    pub fn new(name: String, schema: serde_json::Value) -> Result<Self, ChatError> {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
            || !schema.is_object()
            || schema.get("type").and_then(serde_json::Value::as_str) != Some("object")
        {
            return Err(ChatError::Request);
        }
        let mut pending = vec![&schema];
        while let Some(value) = pending.pop() {
            match value {
                serde_json::Value::Object(fields) => {
                    if fields.keys().any(|key| {
                        matches!(
                            key.as_str(),
                            "$ref" | "$dynamicRef" | "$recursiveRef" | "$schema" | "$id"
                        )
                    }) {
                        return Err(ChatError::Request);
                    }
                    pending.extend(fields.values());
                }
                serde_json::Value::Array(items) => pending.extend(items),
                _ => {}
            }
        }
        Ok(Self { name, schema })
    }
}

/// Trusted request for a supporting endpoint's optional thinking control.
/// It changes generation only; the endpoint can ignore or reject the extension.
#[derive(Clone, Copy, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChatThinkingMode {
    /// Omit the extension and retain the endpoint's own default.
    #[default]
    ProviderDefault,
    /// Request `chat_template_kwargs: {"enable_thinking": false}`.
    Disabled,
}

/// Encodes a single-choice streaming request, with no implicit system prompt,
/// tools, fallback or provider fields. Returned bytes are private conversation data.
pub fn encode_chat(
    model: &str,
    messages: &[ChatMessage<'_>],
    max_tokens: u32,
) -> Result<Vec<u8>, ChatError> {
    encode_chat_request(
        model,
        messages,
        max_tokens,
        false,
        ChatResponseMode::Stream,
        &ChatResponseFormat::Text,
        ChatThinkingMode::ProviderDefault,
    )
}

/// Requests optional provider-reported token usage, without granting admission
/// or treating a server report as a verified tokenizer observation.
pub fn encode_chat_with_usage(
    model: &str,
    messages: &[ChatMessage<'_>],
    max_tokens: u32,
) -> Result<Vec<u8>, ChatError> {
    encode_chat_request(
        model,
        messages,
        max_tokens,
        true,
        ChatResponseMode::Stream,
        &ChatResponseFormat::Text,
        ChatThinkingMode::ProviderDefault,
    )
}

pub(super) fn encode_chat_request(
    model: &str,
    messages: &[ChatMessage<'_>],
    max_tokens: u32,
    include_usage: bool,
    mode: ChatResponseMode,
    format: &ChatResponseFormat,
    thinking: ChatThinkingMode,
) -> Result<Vec<u8>, ChatError> {
    if !valid_model(model)
        || !(1..=32768).contains(&max_tokens)
        || messages.is_empty()
        || messages.len() > 256
        || messages.iter().any(|message| message.content.is_empty())
        || messages
            .iter()
            .map(|message| message.content.len())
            .fold(0_usize, usize::saturating_add)
            > MAX_REQUEST_BYTES
        || !matches!(
            messages.last().map(|message| message.role),
            Some(ChatRole::User)
        )
    {
        return Err(ChatError::Request);
    }
    #[derive(Serialize)]
    struct Request<'a, 'b> {
        model: &'a str,
        messages: &'a [ChatMessage<'b>],
        stream: bool,
        max_tokens: u32,
        n: u8,
        #[serde(skip_serializing_if = "Option::is_none")]
        stream_options: Option<StreamOptions>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_format: Option<ResponseFormat<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        chat_template_kwargs: Option<TemplateOptions>,
    }
    #[derive(Serialize)]
    struct StreamOptions {
        include_usage: bool,
    }
    #[derive(Serialize)]
    struct ResponseFormat<'a> {
        r#type: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        json_schema: Option<SchemaRequest<'a>>,
    }
    #[derive(Serialize)]
    struct SchemaRequest<'a> {
        name: &'a str,
        strict: bool,
        schema: &'a serde_json::Value,
    }
    #[derive(Serialize)]
    struct TemplateOptions {
        enable_thinking: bool,
    }
    // Reserve the closed wire ceiling before any private bytes are written.
    // Reject overflow in the writer, rather than allowing Vec to reallocate
    // (and leave an earlier plaintext allocation behind).
    let mut encoded = PrivateRequestWriter::reserved();
    serde_json::to_writer(
        &mut encoded,
        &Request {
            model,
            messages,
            stream: mode == ChatResponseMode::Stream,
            max_tokens,
            n: 1,
            chat_template_kwargs: (thinking == ChatThinkingMode::Disabled).then_some(
                TemplateOptions {
                    enable_thinking: false,
                },
            ),
            response_format: match format {
                ChatResponseFormat::Text => None,
                ChatResponseFormat::JsonObject => Some(ResponseFormat {
                    r#type: "json_object",
                    json_schema: None,
                }),
                ChatResponseFormat::JsonSchema(schema) => Some(ResponseFormat {
                    r#type: "json_schema",
                    json_schema: Some(SchemaRequest {
                        name: &schema.name,
                        strict: true,
                        schema: &schema.schema,
                    }),
                }),
            },
            stream_options: (include_usage && mode == ChatResponseMode::Stream).then_some(
                StreamOptions {
                    include_usage: true,
                },
            ),
        },
    )
    .map_err(|_| ChatError::Request)?;
    // Transfer the only allocation to the caller; it now owns its private bytes.
    Ok(std::mem::take(&mut *encoded.0))
}

struct PrivateRequestWriter(Zeroizing<Vec<u8>>);

impl PrivateRequestWriter {
    fn reserved() -> Self {
        Self(Zeroizing::new(Vec::with_capacity(MAX_REQUEST_BYTES)))
    }
}

impl std::io::Write for PrivateRequestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_REQUEST_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("chat request limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Why the server finished producing text. A token ceiling is visible to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatFinish {
    /// The server reported a normal stop.
    Stop,
    /// The answer was cut short by the server's token ceiling.
    Length,
}

/// Untrusted provider counts, structurally checked but never local tokenizer,
/// memory, hardware or model-identity evidence. Missing reports stay absent.
pub use agentmage_kernel_contracts::ProviderTokenUsage as ChatUsage;

fn parse_chat_usage(value: &Value, max_tokens: u32) -> Result<ChatUsage, ChatError> {
    let count = |name| {
        value
            .get(name)
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or(ChatError::Protocol)
    };
    let usage = ChatUsage {
        prompt_tokens: count("prompt_tokens")?,
        completion_tokens: count("completion_tokens")?,
        total_tokens: count("total_tokens")?,
    };
    if usage.prompt_tokens.checked_add(usage.completion_tokens) != Some(usage.total_tokens)
        || usage.completion_tokens > max_tokens
    {
        return Err(ChatError::Protocol);
    }
    Ok(usage)
}

/// Decode a whole, HTTP-framed response before exposing any text. Typed fields
/// reject duplicate critical keys; unrelated metadata stays untrusted and inert.
pub(super) fn decode_complete_chat(
    bytes: &[u8],
    model: &str,
    usage_limit: Option<u32>,
) -> Result<(Zeroizing<String>, ChatFinish, Option<ChatUsage>), ChatError> {
    #[derive(Deserialize)]
    struct Completion {
        model: String,
        choices: Vec<Choice>,
        usage: Option<Value>,
        error: Option<Value>,
    }
    #[derive(Deserialize)]
    struct Choice {
        index: u64,
        message: Message,
        finish_reason: String,
    }
    #[derive(Deserialize)]
    struct Message {
        role: String,
        content: String,
        tool_calls: Option<Value>,
        function_call: Option<Value>,
        refusal: Option<Value>,
    }
    if bytes.len() > MAX_STREAM_BYTES {
        return Err(ChatError::Limit);
    }
    let mut response: Completion =
        serde_json::from_slice(bytes).map_err(|_| ChatError::Protocol)?;
    if response.model != model || response.error.is_some() || response.choices.len() != 1 {
        return Err(ChatError::Protocol);
    }
    let choice = response.choices.pop().ok_or(ChatError::Protocol)?;
    let content = Zeroizing::new(choice.message.content);
    if choice.index != 0
        || choice.message.role != "assistant"
        || choice.message.tool_calls.is_some()
        || choice.message.function_call.is_some()
        || choice.message.refusal.is_some()
    {
        return Err(ChatError::Protocol);
    }
    if content.len() > MAX_TEXT_BYTES {
        return Err(ChatError::Limit);
    }
    let finish = match choice.finish_reason.as_str() {
        "stop" => ChatFinish::Stop,
        "length" => ChatFinish::Length,
        _ => return Err(ChatError::Protocol),
    };
    if content.is_empty() {
        return Err(ChatError::Incomplete);
    }
    let usage = match (usage_limit, response.usage) {
        (Some(limit), Some(value)) => Some(parse_chat_usage(&value, limit)?),
        _ => None,
    };
    Ok((content, finish, usage))
}

/// Incremental untrusted text, or a completion marker pending transport EOF.
#[derive(Debug, PartialEq, Eq)]
pub enum ChatEvent {
    /// Plain model text; never markup or an instruction to execute.
    Text(String),
    /// A finish reason followed by the completion marker; `end` must still succeed.
    Finished(ChatFinish),
}

/// Incremental SSE decoder for HTTP-decoded response body bytes.
///
/// Handles arbitrarily split UTF-8, one leading UTF-8 byte-order mark, LF/CRLF
/// lines, comments, multiline data and usage-only events. Non-ASCII field names
/// and lone carriage returns are refused. The transport must separately enforce
/// status, media type, HTTP framing, wire/header/time limits, cancellation and no
/// redirects/proxies.
/// Fixed ceilings: 64 KiB per event, 4 MiB total body, 1 MiB emitted text. Any
/// decoding failure poisons the decoder; partial text is never a completed answer.
pub struct ChatStream {
    model: String,
    line: Vec<u8>,
    data: String,
    event_bytes: usize,
    stream_bytes: usize,
    text_bytes: usize,
    finish_reason: Option<ChatFinish>,
    first_line: bool,
    complete: bool,
    failed: bool,
    usage_limit: Option<u32>,
    usage: Option<ChatUsage>,
    ended: bool,
}

impl ChatStream {
    /// Binds the exact model ID requested by the caller; names grant no admission.
    pub fn new(model: &str) -> Result<Self, ChatError> {
        if !valid_model(model) {
            return Err(ChatError::Request);
        }
        Ok(Self {
            model: model.to_owned(),
            line: Vec::new(),
            data: String::new(),
            event_bytes: 0,
            stream_bytes: 0,
            text_bytes: 0,
            finish_reason: None,
            first_line: true,
            complete: false,
            failed: false,
            usage_limit: None,
            usage: None,
            ended: false,
        })
    }

    /// Collect one optional final usage report. A report before the finish
    /// reason, a duplicate, invalid counters or output above the requested
    /// ceiling refuses the stream. Legacy text-only decoding is unchanged.
    pub fn with_usage(model: &str, max_tokens: u32) -> Result<Self, ChatError> {
        if !(1..=32768).contains(&max_tokens) {
            return Err(ChatError::Request);
        }
        let mut stream = Self::new(model)?;
        stream.usage_limit = Some(max_tokens);
        Ok(stream)
    }

    /// Usage is available only after valid completion and transport EOF. A
    /// subsequent decoding error revokes it along with completion.
    #[must_use]
    pub fn usage(&self) -> Option<ChatUsage> {
        if self.ended && !self.failed {
            self.usage
        } else {
            None
        }
    }

    /// Consumes one body fragment and returns ordered display events. Any error
    /// requires marking previously shown text as partial and closing the transport.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<ChatEvent>, ChatError> {
        if self.failed {
            return Err(ChatError::Failed);
        }
        let result = self.consume(bytes);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// Checks transport EOF; no completion is inferred from a closed connection.
    pub fn end(&mut self) -> Result<(), ChatError> {
        if self.failed {
            return Err(ChatError::Failed);
        }
        if !self.complete || !self.line.is_empty() || !self.data.is_empty() {
            self.failed = true;
            return Err(ChatError::Incomplete);
        }
        self.ended = true;
        Ok(())
    }

    fn consume(&mut self, bytes: &[u8]) -> Result<Vec<ChatEvent>, ChatError> {
        self.stream_bytes = self.stream_bytes.saturating_add(bytes.len());
        if self.stream_bytes > MAX_STREAM_BYTES {
            return Err(ChatError::Limit);
        }
        let mut output = Vec::new();
        for &byte in bytes {
            if self.complete {
                return Err(ChatError::Protocol);
            }
            self.event_bytes += 1;
            if self.event_bytes > MAX_EVENT_BYTES {
                return Err(ChatError::Limit);
            }
            if byte != b'\n' {
                self.line.push(byte);
                continue;
            }
            let raw = std::mem::take(&mut self.line);
            let raw = raw.strip_suffix(b"\r").unwrap_or(&raw);
            // Only CRLF is supported. A lone CR must not fold the next data
            // field into a comment or ignored metadata and lose answer text.
            if raw.contains(&b'\r') {
                return Err(ChatError::Protocol);
            }
            // The line buffer already spans fragments, including a split BOM.
            // Strip it only at body offset zero; its bytes still count toward
            // both budgets. A second or misplaced BOM cannot hide a data field.
            let raw = if std::mem::replace(&mut self.first_line, false) {
                raw.strip_prefix(b"\xef\xbb\xbf").unwrap_or(raw)
            } else {
                raw
            };
            let line = std::str::from_utf8(raw).map_err(|_| ChatError::Protocol)?;
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    self.decode_event(data.trim_end_matches('\n'), &mut output)?;
                }
                self.event_bytes = 0;
            } else if !line.starts_with(':') {
                let (field, value) = line.split_once(':').unwrap_or((line, ""));
                if !field.is_ascii() {
                    return Err(ChatError::Protocol);
                }
                let value = value.strip_prefix(' ').unwrap_or(value);
                match field {
                    "data" => {
                        self.data.push_str(value);
                        self.data.push('\n');
                    }
                    "event" if value != "message" => return Err(ChatError::Protocol),
                    _ => {} // SSE metadata cannot authorize anything.
                }
            }
        }
        Ok(output)
    }

    fn decode_event(&mut self, data: &str, output: &mut Vec<ChatEvent>) -> Result<(), ChatError> {
        if data == "[DONE]" {
            let reason = self.finish_reason.ok_or(ChatError::Incomplete)?;
            if self.text_bytes == 0 {
                return Err(ChatError::Incomplete);
            }
            self.complete = true;
            output.push(ChatEvent::Finished(reason));
            return Ok(());
        }
        let event: Value = serde_json::from_str(data).map_err(|_| ChatError::Protocol)?;
        if event.get("error").is_some()
            || event.get("model").and_then(Value::as_str) != Some(&self.model)
        {
            return Err(ChatError::Protocol);
        }
        let choices = event
            .get("choices")
            .and_then(Value::as_array)
            .ok_or(ChatError::Protocol)?;
        if choices.is_empty() {
            return if event.get("usage").is_some_and(Value::is_object)
                && self.finish_reason.is_some()
            {
                self.collect_usage(&event)?;
                Ok(())
            } else {
                Err(ChatError::Protocol)
            };
        }
        if choices.len() != 1
            || choices[0].get("index").and_then(Value::as_u64) != Some(0)
            || self.finish_reason.is_some()
        {
            return Err(ChatError::Protocol);
        }
        let choice = &choices[0];
        let delta = choice
            .get("delta")
            .and_then(Value::as_object)
            .ok_or(ChatError::Protocol)?;
        if ["tool_calls", "function_call"]
            .iter()
            .any(|field| delta.get(*field).is_some_and(|value| !value.is_null()))
            || delta
                .get("role")
                .is_some_and(|value| value.as_str() != Some("assistant"))
        {
            return Err(ChatError::Protocol);
        }
        if let Some(value) = delta.get("content").filter(|value| !value.is_null()) {
            let text = value.as_str().ok_or(ChatError::Protocol)?;
            self.text_bytes = self.text_bytes.saturating_add(text.len());
            if self.text_bytes > MAX_TEXT_BYTES {
                return Err(ChatError::Limit);
            }
            if !text.is_empty() {
                output.push(ChatEvent::Text(text.to_owned()));
            }
        }
        if let Some(reason) = choice.get("finish_reason").filter(|value| !value.is_null()) {
            self.finish_reason = Some(match reason.as_str() {
                Some("stop") => ChatFinish::Stop,
                Some("length") => ChatFinish::Length,
                _ => return Err(ChatError::Protocol),
            });
        }
        self.collect_usage(&event)?;
        Ok(())
    }

    fn collect_usage(&mut self, event: &Value) -> Result<(), ChatError> {
        if let Some(limit) = self.usage_limit
            && let Some(value) = event.get("usage").filter(|v| !v.is_null())
        {
            if self.finish_reason.is_none() || self.usage.is_some() {
                return Err(ChatError::Protocol);
            }
            self.usage = Some(parse_chat_usage(value, limit)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
