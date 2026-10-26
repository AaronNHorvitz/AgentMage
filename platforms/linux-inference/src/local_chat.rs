//! Bounded chat encoding and incremental event decoding, without transport authority.
//!
//! A parsed endpoint is configuration, not a network grant or model qualification.
//! This module opens no sockets, reads no files and owns no credentials. All model
//! text remains untrusted data; consumers must render it as text and never execute it.

use std::net::{IpAddr, SocketAddr};

use serde::Serialize;
use serde_json::Value;

const MAX_REQUEST_BYTES: usize = 256 * 1024;
const MAX_EVENT_BYTES: usize = 64 * 1024;
const MAX_STREAM_BYTES: usize = 4 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 1024 * 1024;

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

/// Encodes a single-choice streaming request, with no implicit system prompt,
/// tools, fallback or provider fields. Returned bytes are private conversation data.
pub fn encode_chat(
    model: &str,
    messages: &[ChatMessage<'_>],
    max_tokens: u32,
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
    }
    let encoded = serde_json::to_vec(&Request {
        model,
        messages,
        stream: true,
        max_tokens,
        n: 1,
    })
    .map_err(|_| ChatError::Request)?;
    if encoded.len() > MAX_REQUEST_BYTES {
        return Err(ChatError::Request);
    }
    Ok(encoded)
}

/// Why the server finished producing text. A token ceiling is visible to the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatFinish {
    /// The server reported a normal stop.
    Stop,
    /// The answer was cut short by the server's token ceiling.
    Length,
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
/// are refused. The transport must separately enforce status, media type,
/// HTTP framing, wire/header/time limits, cancellation and no redirects/proxies.
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
        })
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
        Ok(())
    }
}

#[cfg(test)]
mod tests;
