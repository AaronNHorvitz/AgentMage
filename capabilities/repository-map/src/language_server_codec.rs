//! Language server message codec (Decision 0126).
//!
//! Frames and classifies the JSON-RPC 2.0 messages of a language server and
//! turns its diagnostics, references and rename previews into sealed,
//! untrusted language service observations. The codec starts no process,
//! reads no file and applies no edit: an adapter owns the server process under
//! ordinary tool authority, the visible sources arrive as the exact bytes the
//! request granted, and a rename preview keeps only digests of its replacement
//! text. Every frame, message, position, path and edit is checked, and
//! anything else fails closed. The workspace root's URI maps the server's URIs
//! to workspace paths and is never recorded.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use agentmage_kernel_contracts::WorkspacePath;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::{
    LanguageServiceDescriptor, LanguageServiceError, LanguageServiceItem, LanguageServiceItemKind,
    LanguageServiceObservation, LanguageServiceObservationStatus, LanguageServiceRequest,
    LanguageServiceRequestKind, SourceRange, seal_language_service_observation,
};

const MAX_HEADER_BYTES: usize = 1024;
const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_LENGTH_DIGITS: usize = 7;
const MAX_METHOD_BYTES: usize = 128;
const MAX_TEXT_ID_BYTES: usize = 128;
const MAX_URI_BYTES: usize = 8 * 1024;
const MAX_LABEL_BYTES: usize = 256;
const MAX_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_NEW_TEXT_BYTES: usize = 1024 * 1024;
const MAX_NEW_NAME_BYTES: usize = 1024;
const MAX_EDITS: usize = 10_000;
const REQUEST_CANCELLED: i64 = -32_800;
const METHOD_NOT_FOUND: i64 = -32_601;
const PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";
const EMPTY_RESULT: &str = "language-server.empty-result";

/// Content-free codec failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageServerCodecError {
    /// A frame header is malformed, duplicated, unknown or oversized. The
    /// decoder that saw it refuses every later byte.
    FrameInvalid,
    /// A frame body is empty or larger than its bound.
    FrameSize,
    /// The body is not exactly one closed JSON-RPC 2.0 message.
    MessageInvalid,
    /// The message does not answer the expected request or notification.
    UnexpectedMessage,
    /// The root, visible sources or request do not match the operation.
    ContextInvalid,
    /// A position, path or edit of the result is invalid.
    ResultInvalid,
    /// The observation could not be sealed.
    Observation(LanguageServiceError),
}

impl LanguageServerCodecError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::FrameInvalid => "language-server.frame-invalid",
            Self::FrameSize => "language-server.frame-size",
            Self::MessageInvalid => "language-server.message-invalid",
            Self::UnexpectedMessage => "language-server.unexpected-message",
            Self::ContextInvalid => "language-server.context-invalid",
            Self::ResultInvalid => "language-server.result-invalid",
            Self::Observation(error) => error.code(),
        }
    }
}

/// Incremental frame decoder with bounded buffering.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LanguageServerFrameDecoder {
    buffer: Vec<u8>,
    failed: bool,
}

impl LanguageServerFrameDecoder {
    /// An empty decoder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: Vec::new(),
            failed: false,
        }
    }

    /// Appends received bytes and returns every complete body in order. After
    /// a failure the decoder refuses everything, and the owner stops the
    /// server.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Vec<u8>>, LanguageServerCodecError> {
        if self.failed {
            return Err(LanguageServerCodecError::FrameInvalid);
        }
        self.buffer.extend_from_slice(bytes);
        let mut bodies = Vec::new();
        loop {
            match next_frame(&self.buffer) {
                Ok(Some((start, length))) => {
                    bodies.push(self.buffer[start..start + length].to_vec());
                    self.buffer.drain(..start + length);
                }
                Ok(None) => return Ok(bodies),
                Err(error) => {
                    self.failed = true;
                    self.buffer.clear();
                    return Err(error);
                }
            }
        }
    }

    /// Whether the decoder is usable and holds no partial frame.
    #[must_use]
    pub fn is_idle(&self) -> bool {
        !self.failed && self.buffer.is_empty()
    }
}

fn next_frame(buffer: &[u8]) -> Result<Option<(usize, usize)>, LanguageServerCodecError> {
    let window = &buffer[..buffer.len().min(MAX_HEADER_BYTES + 4)];
    let Some(end) = window.windows(4).position(|part| part == b"\r\n\r\n") else {
        return if buffer.len() >= MAX_HEADER_BYTES + 4 {
            Err(LanguageServerCodecError::FrameInvalid)
        } else {
            Ok(None)
        };
    };
    let length = parse_headers(&buffer[..end])?;
    let start = end + 4;
    Ok((buffer.len() >= start + length).then_some((start, length)))
}

/// Parses a header block and returns the body length. Only `Content-Length`,
/// once, and an optional `Content-Type` declaring UTF-8 are accepted.
fn parse_headers(header: &[u8]) -> Result<usize, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::FrameInvalid;
    if header
        .iter()
        .any(|byte| !matches!(byte, b' '..=b'~' | b'\t' | b'\r' | b'\n'))
    {
        return Err(invalid);
    }
    let text = std::str::from_utf8(header).map_err(|_| invalid)?;
    let mut length = None;
    let mut content_type = false;
    for line in text.split("\r\n") {
        let (name, value) = line.split_once(':').ok_or(invalid)?;
        if line.contains(['\r', '\n'])
            || name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(invalid);
        }
        let value = value.trim_matches([' ', '\t']);
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some()
                || value.is_empty()
                || !value.bytes().all(|byte| byte.is_ascii_digit())
                || (value.len() > 1 && value.starts_with('0'))
            {
                return Err(invalid);
            }
            if value.len() > MAX_LENGTH_DIGITS {
                return Err(LanguageServerCodecError::FrameSize);
            }
            let parsed = value.parse::<usize>().map_err(|_| invalid)?;
            if parsed == 0 || parsed > MAX_BODY_BYTES {
                return Err(LanguageServerCodecError::FrameSize);
            }
            length = Some(parsed);
        } else if name.eq_ignore_ascii_case("content-type") {
            if content_type || !utf8_content_type(value) {
                return Err(invalid);
            }
            content_type = true;
        } else {
            return Err(invalid);
        }
    }
    length.ok_or(invalid)
}

/// A media type whose only parameter, if any, declares the UTF-8 charset.
fn utf8_content_type(value: &str) -> bool {
    let mut parts = value.split(';');
    let media = parts.next().unwrap_or_default().trim();
    media.split_once('/').is_some_and(|(kind, subtype)| {
        !kind.is_empty() && !subtype.is_empty() && !subtype.contains('/')
    }) && parts.all(|parameter| {
        parameter.split_once('=').is_some_and(|(name, charset)| {
            let charset = charset.trim().trim_matches('"');
            name.trim().eq_ignore_ascii_case("charset")
                && (charset.eq_ignore_ascii_case("utf-8") || charset.eq_ignore_ascii_case("utf8"))
        })
    })
}

/// Frames one message body.
pub fn encode_language_server_frame(body: &[u8]) -> Result<Vec<u8>, LanguageServerCodecError> {
    if body.is_empty() || body.len() > MAX_BODY_BYTES {
        return Err(LanguageServerCodecError::FrameSize);
    }
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(body);
    Ok(frame)
}

/// Identity of a request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LanguageServerRequestId {
    /// A non-negative integer.
    Number(u64),
    /// A bounded printable string.
    Text(String),
}

impl LanguageServerRequestId {
    fn value(&self) -> Value {
        match self {
            Self::Number(number) => Value::from(*number),
            Self::Text(text) => Value::from(text.as_str()),
        }
    }
}

/// Outcome carried by a response.
#[derive(Clone, Debug, PartialEq)]
pub enum LanguageServerResponse {
    /// The request's result.
    Result(Value),
    /// A failure. Only its code is kept; its message is never read.
    Error {
        /// The error code.
        code: i64,
    },
}

/// One classified message from a server.
#[derive(Clone, Debug, PartialEq)]
pub enum LanguageServerMessage {
    /// A response to one of the owner's requests.
    Response {
        /// The request it answers.
        id: LanguageServerRequestId,
        /// Its result or failure.
        response: LanguageServerResponse,
    },
    /// A notification.
    Notification {
        /// Method name.
        method: String,
        /// Parameters, an object or an array.
        params: Option<Value>,
    },
    /// A request from the server. The codec never acts on it; the owner
    /// answers with [`encode_server_request_refusal`] unless a qualified
    /// adapter handles it under ordinary tool authority.
    ServerRequest {
        /// Its identity.
        id: LanguageServerRequestId,
        /// Method name.
        method: String,
    },
}

/// Classifies one message body. Unknown members, duplicate keys and any
/// shape other than a response, a notification or a server request fail.
pub fn decode_language_server_message(
    body: &[u8],
) -> Result<LanguageServerMessage, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::MessageInvalid;
    if body.is_empty() || body.len() > MAX_BODY_BYTES {
        return Err(LanguageServerCodecError::FrameSize);
    }
    let StrictValue(value) = serde_json::from_slice(body).map_err(|_| invalid)?;
    let Value::Object(mut object) = value else {
        return Err(invalid);
    };
    // Each shape below removes its members and refuses anything left over.
    if object.remove("jsonrpc") != Some(Value::from("2.0")) {
        return Err(invalid);
    }
    let id = object.remove("id").map(request_id).transpose()?;
    if let Some(method) = object.remove("method") {
        let method = method_name(method)?;
        let params = object.remove("params");
        if !object.is_empty()
            || params
                .as_ref()
                .is_some_and(|params| !params.is_object() && !params.is_array())
        {
            return Err(invalid);
        }
        return Ok(match id {
            Some(id) => LanguageServerMessage::ServerRequest { id, method },
            None => LanguageServerMessage::Notification { method, params },
        });
    }
    let id = id.ok_or(invalid)?;
    let response = match (object.remove("result"), object.remove("error")) {
        (Some(result), None) => LanguageServerResponse::Result(result),
        (None, Some(error)) => LanguageServerResponse::Error {
            code: error_code(error)?,
        },
        _ => return Err(invalid),
    };
    if !object.is_empty() {
        return Err(invalid);
    }
    Ok(LanguageServerMessage::Response { id, response })
}

fn request_id(value: Value) -> Result<LanguageServerRequestId, LanguageServerCodecError> {
    match value {
        Value::Number(number) => number
            .as_u64()
            .map(LanguageServerRequestId::Number)
            .ok_or(LanguageServerCodecError::MessageInvalid),
        Value::String(text)
            if !text.is_empty()
                && text.len() <= MAX_TEXT_ID_BYTES
                && text.bytes().all(|byte| byte.is_ascii_graphic()) =>
        {
            Ok(LanguageServerRequestId::Text(text))
        }
        _ => Err(LanguageServerCodecError::MessageInvalid),
    }
}

fn method_name(value: Value) -> Result<String, LanguageServerCodecError> {
    match value {
        Value::String(method)
            if !method.is_empty()
                && method.len() <= MAX_METHOD_BYTES
                && method.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'$' | b'.' | b'_' | b'-')
                }) =>
        {
            Ok(method)
        }
        _ => Err(LanguageServerCodecError::MessageInvalid),
    }
}

fn error_code(value: Value) -> Result<i64, LanguageServerCodecError> {
    let Value::Object(error) = value else {
        return Err(LanguageServerCodecError::MessageInvalid);
    };
    if error
        .keys()
        .any(|key| !matches!(key.as_str(), "code" | "message" | "data"))
        || !error.get("message").is_some_and(Value::is_string)
    {
        return Err(LanguageServerCodecError::MessageInvalid);
    }
    error
        .get("code")
        .and_then(Value::as_i64)
        .ok_or(LanguageServerCodecError::MessageInvalid)
}

/// Frames the owner's refusal of a server request.
pub fn encode_server_request_refusal(
    id: &LanguageServerRequestId,
) -> Result<Vec<u8>, LanguageServerCodecError> {
    encode_message(&json!({
        "jsonrpc": "2.0",
        "id": id.value(),
        "error": {"code": METHOD_NOT_FOUND, "message": "refused by the owner"},
    }))
}

fn encode_message(value: &Value) -> Result<Vec<u8>, LanguageServerCodecError> {
    let body = serde_json::to_vec(value).map_err(|_| LanguageServerCodecError::MessageInvalid)?;
    encode_language_server_frame(&body)
}

/// A JSON value whose objects never repeat a key.
struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictVisitor).map(StrictValue)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without repeated keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(StrictValue(item)) = sequence.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut object = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let StrictValue(value) = map.next_value()?;
            if object.insert(key, value).is_some() {
                return Err(de::Error::custom("repeated key"));
            }
        }
        Ok(Value::Object(object))
    }
}

/// Which unit a position's character offset counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageServerPositionEncoding {
    /// UTF-8 code units.
    Utf8,
    /// UTF-16 code units, the protocol's default.
    Utf16,
    /// Unicode scalar values.
    Utf32,
}

impl LanguageServerPositionEncoding {
    fn width(self, character: char) -> u64 {
        match self {
            Self::Utf8 => character.len_utf8() as u64,
            Self::Utf16 => character.len_utf16() as u64,
            Self::Utf32 => 1,
        }
    }
}

/// Exact bytes of one file the request made visible.
#[derive(Clone, Copy, Debug)]
pub struct LanguageServerVisibleSource<'a> {
    /// Its workspace path, one of the request's visible files.
    pub path: &'a WorkspacePath,
    /// Its complete bytes, matching the request's digest and length.
    pub bytes: &'a [u8],
}

/// Everything a decode or an encoded request needs besides the message.
#[derive(Clone, Copy, Debug)]
pub struct LanguageServerCodecContext<'a> {
    /// The sealed descriptor of the service.
    pub descriptor: &'a LanguageServiceDescriptor,
    /// The request the message answers.
    pub request: &'a LanguageServiceRequest,
    /// `file` URI of the authorized workspace root, ending in `/`. It maps
    /// the server's URIs to workspace paths and is never recorded.
    pub root_uri: &'a str,
    /// Unit of a position's character offset, as negotiated by the adapter.
    pub position_encoding: LanguageServerPositionEncoding,
    /// Exact bytes of the visible files that positions refer to.
    pub sources: &'a [LanguageServerVisibleSource<'a>],
}

struct Prepared<'a> {
    context: LanguageServerCodecContext<'a>,
    sources: BTreeMap<&'a WorkspacePath, (Lines<'a>, &'a str)>,
}

fn prepare(
    context: LanguageServerCodecContext<'_>,
    kind: LanguageServiceRequestKind,
) -> Result<Prepared<'_>, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ContextInvalid;
    let root = context.root_uri;
    if context.request.kind != kind
        || context.request.descriptor_sha256 != context.descriptor.descriptor_sha256
        || root.len() > MAX_URI_BYTES
        || !root.starts_with("file:///")
        || !root.ends_with('/')
        || root.contains(['?', '#'])
        || !root.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(invalid);
    }
    let mut sources = BTreeMap::new();
    for source in context.sources {
        let allowed = context
            .request
            .allowed_files
            .iter()
            .find(|file| &file.path == source.path)
            .ok_or(invalid)?;
        if allowed.source_sha256 != sha256_hex(source.bytes)
            || allowed.source_byte_len != source.bytes.len() as u64
        {
            return Err(invalid);
        }
        let text = std::str::from_utf8(source.bytes).map_err(|_| invalid)?;
        if sources
            .insert(
                source.path,
                (Lines::new(text), allowed.source_sha256.as_str()),
            )
            .is_some()
        {
            return Err(invalid);
        }
    }
    Ok(Prepared { context, sources })
}

/// Line starts and content ends of one source; `\r\n`, `\n` and `\r` each
/// end a line.
struct Lines<'a> {
    text: &'a str,
    starts: Vec<usize>,
    ends: Vec<usize>,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        let mut ends = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => {
                    ends.push(index);
                    index += 1;
                    starts.push(index);
                }
                b'\r' => {
                    ends.push(index);
                    index += if bytes.get(index + 1) == Some(&b'\n') {
                        2
                    } else {
                        1
                    };
                    starts.push(index);
                }
                _ => index += 1,
            }
        }
        ends.push(bytes.len());
        Self { text, starts, ends }
    }

    /// Byte offset, one-based line and byte column of a position. A
    /// character offset past the line's content means the line's end; one
    /// inside a character is invalid.
    fn offset(
        &self,
        line: u64,
        character: u64,
        encoding: LanguageServerPositionEncoding,
    ) -> Option<(usize, u32, u32)> {
        let index = usize::try_from(line).ok()?;
        let start = *self.starts.get(index)?;
        let end = self.ends[index];
        let mut units = 0_u64;
        let mut offset = start;
        for character_value in self.text[start..end].chars() {
            if units >= character {
                break;
            }
            units += encoding.width(character_value);
            if units > character {
                return None;
            }
            offset += character_value.len_utf8();
        }
        Some((
            offset,
            u32::try_from(index + 1).ok()?,
            u32::try_from(offset - start).ok()?,
        ))
    }

    /// Zero-based line and character offset of a byte on a character
    /// boundary outside a line break.
    fn position(
        &self,
        byte: usize,
        encoding: LanguageServerPositionEncoding,
    ) -> Option<(u64, u64)> {
        if byte > self.text.len() || !self.text.is_char_boundary(byte) {
            return None;
        }
        let index = self.starts.partition_point(|start| *start <= byte) - 1;
        if byte > self.ends[index] {
            return None;
        }
        let units = self.text[self.starts[index]..byte]
            .chars()
            .map(|character| encoding.width(character))
            .sum();
        Some((index as u64, units))
    }
}

fn position(value: Option<&Value>) -> Result<(u64, u64), LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ResultInvalid;
    let Some(Value::Object(object)) = value else {
        return Err(invalid);
    };
    if object.len() != 2 {
        return Err(invalid);
    }
    Ok((
        object.get("line").and_then(Value::as_u64).ok_or(invalid)?,
        object
            .get("character")
            .and_then(Value::as_u64)
            .ok_or(invalid)?,
    ))
}

fn source_range(
    prepared: &Prepared<'_>,
    path: &WorkspacePath,
    value: Option<&Value>,
) -> Result<SourceRange, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ResultInvalid;
    let Some(Value::Object(object)) = value else {
        return Err(invalid);
    };
    let (lines, _) = prepared.sources.get(path).ok_or(invalid)?;
    let encoding = prepared.context.position_encoding;
    let (start_line, start_character) = position(object.get("start"))?;
    let (end_line, end_character) = position(object.get("end"))?;
    let (start_byte, start_line, start_column) = lines
        .offset(start_line, start_character, encoding)
        .ok_or(invalid)?;
    let (end_byte, end_line, end_column) = lines
        .offset(end_line, end_character, encoding)
        .ok_or(invalid)?;
    if object.len() != 2 || end_byte < start_byte {
        return Err(invalid);
    }
    Ok(SourceRange {
        start_byte: start_byte as u64,
        end_byte: end_byte as u64,
        start_line,
        end_line,
        start_column,
        end_column,
    })
}

/// Maps a server URI under the root to a workspace path.
fn uri_path(
    prepared: &Prepared<'_>,
    value: Option<&Value>,
) -> Result<WorkspacePath, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ResultInvalid;
    let Some(Value::String(uri)) = value else {
        return Err(invalid);
    };
    if uri.len() > MAX_URI_BYTES {
        return Err(invalid);
    }
    let relative = uri.strip_prefix(prepared.context.root_uri).ok_or(invalid)?;
    let components = relative
        .split('/')
        .map(decode_segment)
        .collect::<Result<Vec<_>, _>>()?;
    WorkspacePath::new(
        prepared.context.request.source_path.workspace_id().clone(),
        components,
    )
    .map_err(|_| invalid)
}

fn decode_segment(segment: &str) -> Result<String, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ResultInvalid;
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            let high = bytes.get(index + 1).copied().and_then(hex_value);
            let low = bytes.get(index + 2).copied().and_then(hex_value);
            let (Some(high), Some(low)) = (high, low) else {
                return Err(invalid);
            };
            decoded.push((high << 4) | low);
            index += 3;
        } else if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&byte) {
            decoded.push(byte);
            index += 1;
        } else {
            return Err(invalid);
        }
    }
    String::from_utf8(decoded).map_err(|_| invalid)
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// The URI of a workspace path under the root; every byte outside the
/// unreserved set is percent-encoded.
fn path_uri(root: &str, path: &WorkspacePath) -> String {
    let mut uri = root.to_owned();
    for (index, component) in path.components().iter().enumerate() {
        if index > 0 {
            uri.push('/');
        }
        for byte in component.as_str().bytes() {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                uri.push(char::from(byte));
            } else {
                let _ = write!(uri, "%{byte:02X}");
            }
        }
    }
    uri
}

struct Found {
    path: WorkspacePath,
    range: SourceRange,
    metadata_sha256: String,
    replacement_sha256: Option<String>,
}

fn seal(
    prepared: &Prepared<'_>,
    status: LanguageServiceObservationStatus,
    items: Vec<LanguageServiceItem>,
    body: Option<&[u8]>,
    terminal_code: Option<&str>,
    truncated: bool,
) -> Result<LanguageServiceObservation, LanguageServerCodecError> {
    seal_language_service_observation(
        prepared.context.descriptor,
        prepared.context.request.clone(),
        status,
        items,
        body.map(sha256_hex),
        terminal_code.map(str::to_owned),
        truncated,
    )
    .map_err(LanguageServerCodecError::Observation)
}

fn terminal(
    prepared: &Prepared<'_>,
    status: LanguageServiceObservationStatus,
    code: &str,
) -> Result<LanguageServiceObservation, LanguageServerCodecError> {
    seal(prepared, status, Vec::new(), None, Some(code), false)
}

/// Sorts, numbers and bounds the items; more items than the descriptor
/// allows yield a partial observation.
fn finish(
    prepared: &Prepared<'_>,
    kind: LanguageServiceItemKind,
    prefix: &str,
    mut found: Vec<Found>,
    body: &[u8],
) -> Result<LanguageServiceObservation, LanguageServerCodecError> {
    found.sort_by(|left, right| {
        (
            &left.path,
            left.range.start_byte,
            left.range.end_byte,
            &left.metadata_sha256,
        )
            .cmp(&(
                &right.path,
                right.range.start_byte,
                right.range.end_byte,
                &right.metadata_sha256,
            ))
    });
    let limit =
        usize::try_from(prepared.context.descriptor.maximum_response_items).unwrap_or(usize::MAX);
    let truncated = found.len() > limit;
    found.truncate(limit);
    let mut items = Vec::with_capacity(found.len());
    for (index, value) in found.into_iter().enumerate() {
        let (_, source_sha256) = prepared
            .sources
            .get(&value.path)
            .ok_or(LanguageServerCodecError::ResultInvalid)?;
        items.push(LanguageServiceItem {
            item_id: format!("{prefix}-{index:06}"),
            kind,
            path: value.path,
            range: value.range,
            source_sha256: (*source_sha256).to_owned(),
            metadata_sha256: value.metadata_sha256,
            replacement_sha256: value.replacement_sha256,
            untrusted_output: true,
            write_authority: false,
        });
    }
    if truncated {
        seal(
            prepared,
            LanguageServiceObservationStatus::Partial,
            items,
            Some(body),
            Some("language-server.items-truncated"),
            true,
        )
    } else {
        seal(
            prepared,
            LanguageServiceObservationStatus::Complete,
            items,
            Some(body),
            None,
            false,
        )
    }
}

/// The result of the expected response, or the terminal observation of its
/// failure.
fn response_result(
    prepared: &Prepared<'_>,
    expected_id: &LanguageServerRequestId,
    body: &[u8],
) -> Result<Result<Value, LanguageServiceObservation>, LanguageServerCodecError> {
    match decode_language_server_message(body)? {
        LanguageServerMessage::Response { id, response } if &id == expected_id => match response {
            LanguageServerResponse::Result(value) => Ok(Ok(value)),
            LanguageServerResponse::Error { code } if code == REQUEST_CANCELLED => terminal(
                prepared,
                LanguageServiceObservationStatus::Cancelled,
                "language-server.request-cancelled",
            )
            .map(Err),
            LanguageServerResponse::Error { .. } => terminal(
                prepared,
                LanguageServiceObservationStatus::Unavailable,
                "language-server.error-response",
            )
            .map(Err),
        },
        _ => Err(LanguageServerCodecError::UnexpectedMessage),
    }
}

fn optional_label(value: Option<&Value>) -> Result<Option<&str>, LanguageServerCodecError> {
    match value {
        None => Ok(None),
        Some(Value::String(text)) if text.len() <= MAX_LABEL_BYTES => Ok(Some(text)),
        Some(_) => Err(LanguageServerCodecError::ResultInvalid),
    }
}

/// Seals the diagnostics a server published for the request's source file.
/// Each diagnostic keeps its range and a digest of its severity, code, source
/// and message; its related information, tags and data are not kept.
pub fn observe_language_server_diagnostics(
    context: LanguageServerCodecContext<'_>,
    body: &[u8],
) -> Result<LanguageServiceObservation, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ResultInvalid;
    let prepared = prepare(context, LanguageServiceRequestKind::Diagnostics)?;
    let LanguageServerMessage::Notification {
        method,
        params: Some(Value::Object(params)),
    } = decode_language_server_message(body)?
    else {
        return Err(LanguageServerCodecError::UnexpectedMessage);
    };
    if method != PUBLISH_DIAGNOSTICS {
        return Err(LanguageServerCodecError::UnexpectedMessage);
    }
    if params
        .keys()
        .any(|key| !matches!(key.as_str(), "uri" | "version" | "diagnostics"))
        || params.get("version").is_some_and(|value| !value.is_i64())
    {
        return Err(invalid);
    }
    let path = uri_path(&prepared, params.get("uri"))?;
    if path != context.request.source_path {
        return Err(LanguageServerCodecError::UnexpectedMessage);
    }
    let Some(Value::Array(diagnostics)) = params.get("diagnostics") else {
        return Err(invalid);
    };
    if diagnostics.len() > MAX_EDITS {
        return Err(invalid);
    }
    let mut found = Vec::with_capacity(diagnostics.len());
    for diagnostic in diagnostics {
        let Value::Object(object) = diagnostic else {
            return Err(invalid);
        };
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "range"
                    | "severity"
                    | "code"
                    | "codeDescription"
                    | "source"
                    | "message"
                    | "tags"
                    | "relatedInformation"
                    | "data"
            )
        }) {
            return Err(invalid);
        }
        let severity = match object.get("severity") {
            None => None,
            Some(value) => Some(
                value
                    .as_u64()
                    .filter(|severity| (1..=4).contains(severity))
                    .ok_or(invalid)?,
            ),
        };
        let code = match object.get("code") {
            None => Value::Null,
            Some(value @ Value::Number(number)) if number.is_i64() => value.clone(),
            Some(value @ Value::String(text)) if text.len() <= MAX_LABEL_BYTES => value.clone(),
            Some(_) => return Err(invalid),
        };
        let source = optional_label(object.get("source"))?;
        let Some(Value::String(message)) = object.get("message") else {
            return Err(invalid);
        };
        if message.len() > MAX_MESSAGE_BYTES {
            return Err(invalid);
        }
        found.push(Found {
            range: source_range(&prepared, &path, object.get("range"))?,
            path: path.clone(),
            metadata_sha256: sha256_json(&json!({
                "code": code,
                "message": message,
                "severity": severity,
                "source": source,
            })),
            replacement_sha256: None,
        });
    }
    finish(
        &prepared,
        LanguageServiceItemKind::Diagnostic,
        "diagnostic",
        found,
        body,
    )
}

/// Seals the references a server returned for the request's position. An
/// empty answer is not evidence of absence and is sealed as unavailable.
pub fn observe_language_server_references(
    context: LanguageServerCodecContext<'_>,
    expected_id: &LanguageServerRequestId,
    body: &[u8],
) -> Result<LanguageServiceObservation, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ResultInvalid;
    let prepared = prepare(context, LanguageServiceRequestKind::References)?;
    let locations = match response_result(&prepared, expected_id, body)? {
        Err(observation) => return Ok(observation),
        Ok(Value::Null) => Vec::new(),
        Ok(Value::Array(locations)) => locations,
        Ok(_) => return Err(invalid),
    };
    if locations.is_empty() {
        return terminal(
            &prepared,
            LanguageServiceObservationStatus::Unavailable,
            EMPTY_RESULT,
        );
    }
    if locations.len() > MAX_EDITS {
        return Err(invalid);
    }
    let mut found = Vec::with_capacity(locations.len());
    for location in &locations {
        let Value::Object(object) = location else {
            return Err(invalid);
        };
        if object.len() != 2 {
            return Err(invalid);
        }
        let path = uri_path(&prepared, object.get("uri"))?;
        found.push(Found {
            range: source_range(&prepared, &path, object.get("range"))?,
            path,
            metadata_sha256: sha256_json(&json!({"kind": "reference"})),
            replacement_sha256: None,
        });
    }
    finish(
        &prepared,
        LanguageServiceItemKind::Reference,
        "reference",
        found,
        body,
    )
}

/// Seals the text edits of a rename preview. File creation, renaming and
/// deletion, two edit forms at once and overlapping edits are refused as a
/// whole. Nothing is applied: a later write needs the ordinary change
/// approval.
pub fn observe_language_server_rename(
    context: LanguageServerCodecContext<'_>,
    expected_id: &LanguageServerRequestId,
    body: &[u8],
) -> Result<LanguageServiceObservation, LanguageServerCodecError> {
    let invalid = LanguageServerCodecError::ResultInvalid;
    let prepared = prepare(context, LanguageServiceRequestKind::RenamePreview)?;
    let edit = match response_result(&prepared, expected_id, body)? {
        Err(observation) => return Ok(observation),
        Ok(Value::Null) => Map::new(),
        Ok(Value::Object(edit)) => edit,
        Ok(_) => return Err(invalid),
    };
    if edit.keys().any(|key| {
        !matches!(
            key.as_str(),
            "changes" | "documentChanges" | "changeAnnotations"
        )
    }) {
        return Err(invalid);
    }
    let mut edits = Vec::new();
    match (edit.get("changes"), edit.get("documentChanges")) {
        (Some(_), Some(_)) => {
            return terminal(
                &prepared,
                LanguageServiceObservationStatus::Rejected,
                "language-server.ambiguous-edit",
            );
        }
        (Some(Value::Object(changes)), None) => {
            for (uri, list) in changes {
                let path = uri_path(&prepared, Some(&Value::from(uri.as_str())))?;
                let Value::Array(list) = list else {
                    return Err(invalid);
                };
                edits.extend(list.iter().map(|value| (path.clone(), value)));
            }
        }
        (None, Some(Value::Array(changes))) => {
            for change in changes {
                let Value::Object(change) = change else {
                    return Err(invalid);
                };
                if change.contains_key("kind") {
                    return terminal(
                        &prepared,
                        LanguageServiceObservationStatus::Rejected,
                        "language-server.file-operation-refused",
                    );
                }
                let (Some(Value::Object(document)), Some(Value::Array(list)), 2) = (
                    change.get("textDocument"),
                    change.get("edits"),
                    change.len(),
                ) else {
                    return Err(invalid);
                };
                if document
                    .keys()
                    .any(|key| !matches!(key.as_str(), "uri" | "version"))
                    || document
                        .get("version")
                        .is_some_and(|version| !version.is_null() && !version.is_i64())
                {
                    return Err(invalid);
                }
                let path = uri_path(&prepared, document.get("uri"))?;
                edits.extend(list.iter().map(|value| (path.clone(), value)));
            }
        }
        (None, None) => {}
        _ => return Err(invalid),
    }
    if edits.len() > MAX_EDITS {
        return Err(invalid);
    }
    if edits.is_empty() {
        return terminal(
            &prepared,
            LanguageServiceObservationStatus::Unavailable,
            EMPTY_RESULT,
        );
    }
    let mut found = Vec::with_capacity(edits.len());
    for (path, value) in edits {
        let Value::Object(object) = value else {
            return Err(invalid);
        };
        let Some(Value::String(new_text)) = object.get("newText") else {
            return Err(invalid);
        };
        if object
            .keys()
            .any(|key| !matches!(key.as_str(), "range" | "newText" | "annotationId"))
            || optional_label(object.get("annotationId")).is_err()
            || new_text.len() > MAX_NEW_TEXT_BYTES
        {
            return Err(invalid);
        }
        let range = source_range(&prepared, &path, object.get("range"))?;
        let replacement_sha256 = sha256_hex(new_text.as_bytes());
        found.push(Found {
            metadata_sha256: sha256_json(&json!({
                "kind": "rename-edit",
                "replacement_sha256": replacement_sha256,
            })),
            path,
            range,
            replacement_sha256: Some(replacement_sha256),
        });
    }
    found.sort_by(|left, right| {
        (&left.path, left.range.start_byte, left.range.end_byte).cmp(&(
            &right.path,
            right.range.start_byte,
            right.range.end_byte,
        ))
    });
    if found.windows(2).any(|pair| {
        pair[0].path == pair[1].path
            && (pair[1].range.start_byte < pair[0].range.end_byte
                || pair[1].range.start_byte == pair[0].range.start_byte)
    }) {
        return terminal(
            &prepared,
            LanguageServiceObservationStatus::Rejected,
            "language-server.overlapping-edits",
        );
    }
    finish(
        &prepared,
        LanguageServiceItemKind::RenameEdit,
        "rename-edit",
        found,
        body,
    )
}

/// The URI and position of the request's source byte.
fn request_position(prepared: &Prepared<'_>) -> Result<(String, Value), LanguageServerCodecError> {
    let request = prepared.context.request;
    let invalid = LanguageServerCodecError::ContextInvalid;
    let (lines, _) = prepared.sources.get(&request.source_path).ok_or(invalid)?;
    let byte = usize::try_from(request.source_byte).map_err(|_| invalid)?;
    let (line, character) = lines
        .position(byte, prepared.context.position_encoding)
        .ok_or(invalid)?;
    Ok((
        path_uri(prepared.context.root_uri, &request.source_path),
        json!({"line": line, "character": character}),
    ))
}

/// Frames a references request for the request's source position.
pub fn encode_language_server_references_request(
    context: LanguageServerCodecContext<'_>,
    id: u64,
    include_declaration: bool,
) -> Result<Vec<u8>, LanguageServerCodecError> {
    let prepared = prepare(context, LanguageServiceRequestKind::References)?;
    let (uri, position) = request_position(&prepared)?;
    encode_message(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/references",
        "params": {
            "textDocument": {"uri": uri},
            "position": position,
            "context": {"includeDeclaration": include_declaration},
        },
    }))
}

/// Frames a rename request. The new name must be the one whose digest the
/// request carries.
pub fn encode_language_server_rename_request(
    context: LanguageServerCodecContext<'_>,
    id: u64,
    new_name: &str,
) -> Result<Vec<u8>, LanguageServerCodecError> {
    let prepared = prepare(context, LanguageServiceRequestKind::RenamePreview)?;
    if new_name.is_empty()
        || new_name.len() > MAX_NEW_NAME_BYTES
        || new_name.chars().any(char::is_control)
        || context.request.subject_sha256.as_deref()
            != Some(sha256_hex(new_name.as_bytes()).as_str())
    {
        return Err(LanguageServerCodecError::ContextInvalid);
    }
    let (uri, position) = request_position(&prepared)?;
    encode_message(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/rename",
        "params": {
            "textDocument": {"uri": uri},
            "position": position,
            "newName": new_name,
        },
    }))
}

fn sha256_json(value: &Value) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"language-server-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::WorkspaceId;

    use super::*;
    use crate::{
        LanguageServiceCapability, LanguageServiceVisibleFile, RepositoryLanguage,
        grammar_descriptor, seal_language_service_descriptor, verify_language_service_observation,
    };

    const ROOT: &str = "file:///workspace/fixture/";
    const MODULE: &str = "def na\u{ef}ve():\r\n    return \"\u{1f600}\" + missing\n";
    const TEST: &str = "from src.module import na\u{ef}ve\nassert na\u{ef}ve()\n";

    fn path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-language-server"),
            parts.iter().copied(),
        )
        .unwrap()
    }

    fn descriptor(maximum_response_items: u32) -> LanguageServiceDescriptor {
        seal_language_service_descriptor(LanguageServiceDescriptor {
            service_id: "fixture-language-server".to_owned(),
            implementation_version: "0.1.0".to_owned(),
            executable_sha256: "1".repeat(64),
            language: RepositoryLanguage::Python,
            grammar_descriptor_sha256: grammar_descriptor(RepositoryLanguage::Python)
                .descriptor_sha256,
            capabilities: vec![
                LanguageServiceCapability::References,
                LanguageServiceCapability::RenamePreview,
                LanguageServiceCapability::Diagnostics,
            ],
            workspace_root_sha256: "2".repeat(64),
            maximum_response_items,
            timeout_ms: 30_000,
            memory_limit_bytes: 512 * 1024 * 1024,
            cpu_time_limit_ms: 30_000,
            environment_name_allowlist: vec![
                "LANG".to_owned(),
                "LC_ALL".to_owned(),
                "NO_COLOR".to_owned(),
            ],
            read_only: true,
            network_allowed: false,
            workspace_write_allowed: false,
            command_execution_allowed: false,
            package_installation_allowed: false,
            plugin_loading_allowed: false,
            executable_discovery_allowed: false,
            environment_inheritance_allowed: false,
            descriptor_sha256: String::new(),
        })
        .unwrap()
    }

    fn request(
        descriptor: &LanguageServiceDescriptor,
        kind: LanguageServiceRequestKind,
    ) -> LanguageServiceRequest {
        LanguageServiceRequest {
            request_id: "request-codec-1".to_owned(),
            index_sha256: "3".repeat(64),
            descriptor_sha256: descriptor.descriptor_sha256.clone(),
            kind,
            source_path: path(&["src", "module.py"]),
            source_sha256: sha256_hex(MODULE.as_bytes()),
            source_byte_len: MODULE.len() as u64,
            source_byte: MODULE.find("missing").unwrap() as u64,
            subject_sha256: Some(sha256_hex(b"absent")),
            allowed_files: vec![
                LanguageServiceVisibleFile {
                    path: path(&["src", "module.py"]),
                    source_sha256: sha256_hex(MODULE.as_bytes()),
                    source_byte_len: MODULE.len() as u64,
                },
                LanguageServiceVisibleFile {
                    path: path(&["tests", "test module.py"]),
                    source_sha256: sha256_hex(TEST.as_bytes()),
                    source_byte_len: TEST.len() as u64,
                },
            ],
            separate_grant_required: true,
            write_authority: false,
        }
    }

    struct Fixture {
        descriptor: LanguageServiceDescriptor,
        request: LanguageServiceRequest,
        module: WorkspacePath,
        test: WorkspacePath,
    }

    impl Fixture {
        fn new(kind: LanguageServiceRequestKind, maximum_response_items: u32) -> Self {
            let descriptor = descriptor(maximum_response_items);
            let request = request(&descriptor, kind);
            Self {
                descriptor,
                request,
                module: path(&["src", "module.py"]),
                test: path(&["tests", "test module.py"]),
            }
        }

        fn run<T>(&self, f: impl FnOnce(LanguageServerCodecContext<'_>) -> T) -> T {
            let sources = [
                LanguageServerVisibleSource {
                    path: &self.module,
                    bytes: MODULE.as_bytes(),
                },
                LanguageServerVisibleSource {
                    path: &self.test,
                    bytes: TEST.as_bytes(),
                },
            ];
            f(LanguageServerCodecContext {
                descriptor: &self.descriptor,
                request: &self.request,
                root_uri: ROOT,
                position_encoding: LanguageServerPositionEncoding::Utf16,
                sources: &sources,
            })
        }
    }

    fn body(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }

    fn range(start: (u64, u64), end: (u64, u64)) -> Value {
        json!({
            "start": {"line": start.0, "character": start.1},
            "end": {"line": end.0, "character": end.1},
        })
    }

    fn response(result: &Value) -> Vec<u8> {
        body(&json!({"jsonrpc": "2.0", "id": 7, "result": result}))
    }

    const ID: LanguageServerRequestId = LanguageServerRequestId::Number(7);

    #[test]
    fn frames_split_or_joined_across_reads_decode_in_order() {
        let mut decoder = LanguageServerFrameDecoder::new();
        let first = encode_language_server_frame(br#"{"a":1}"#).unwrap();
        assert_eq!(first, b"Content-Length: 7\r\n\r\n{\"a\":1}".to_vec());
        let second =
            b"content-length:  2\r\nContent-Type: application/json; charset=\"UTF-8\"\r\n\r\n[]";
        let mut stream = first.clone();
        stream.extend_from_slice(second);
        for byte in &stream[..stream.len() - 1] {
            assert!(decoder.push(std::slice::from_ref(byte)).unwrap().len() <= 1);
        }
        assert!(!decoder.is_idle());
        assert_eq!(decoder.push(b"]").unwrap(), vec![b"[]".to_vec()]);
        assert!(decoder.is_idle());
        let mut decoder = LanguageServerFrameDecoder::new();
        assert_eq!(
            decoder.push(&stream).unwrap(),
            vec![b"{\"a\":1}".to_vec(), b"[]".to_vec()]
        );
        assert_eq!(
            encode_language_server_frame(b""),
            Err(LanguageServerCodecError::FrameSize)
        );
    }

    #[test]
    fn malformed_unknown_or_oversized_headers_fail_and_the_decoder_stays_failed() {
        let long = format!("X-Padding: {}\r\n", "a".repeat(MAX_HEADER_BYTES));
        let cases: [(&[u8], LanguageServerCodecError); 13] = [
            (b"\r\n\r\n{}", LanguageServerCodecError::FrameInvalid),
            (
                b"Content-Type: text/plain\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (
                b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (
                b"Content-Length: 02\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (
                b"Content-Length: +2\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (
                b"Content-Length: 0\r\n\r\n",
                LanguageServerCodecError::FrameSize,
            ),
            (
                b"Content-Length: 4194305\r\n\r\n",
                LanguageServerCodecError::FrameSize,
            ),
            (
                b"Content-Length: 12345678\r\n\r\n",
                LanguageServerCodecError::FrameSize,
            ),
            (
                b"Content-Length: 2\r\nX-Trace: 1\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (
                b"Content-Length: 2\r\nContent-Type: application/json; charset=latin-1\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (
                b"Content-Length: 2\nX: 1\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (
                b"Content-Length: 2\r\nContent-Type: application/json; boundary=x\r\n\r\n{}",
                LanguageServerCodecError::FrameInvalid,
            ),
            (long.as_bytes(), LanguageServerCodecError::FrameInvalid),
        ];
        for (bytes, error) in cases {
            let mut decoder = LanguageServerFrameDecoder::new();
            assert_eq!(
                decoder.push(bytes),
                Err(error),
                "{}",
                String::from_utf8_lossy(bytes)
            );
            assert!(!decoder.is_idle());
            assert_eq!(
                decoder.push(b"Content-Length: 2\r\n\r\n{}"),
                Err(LanguageServerCodecError::FrameInvalid)
            );
        }
    }

    #[test]
    fn messages_are_classified_and_every_other_shape_is_refused() {
        assert_eq!(
            decode_language_server_message(br#"{"jsonrpc":"2.0","id":"a-1","result":null}"#),
            Ok(LanguageServerMessage::Response {
                id: LanguageServerRequestId::Text("a-1".to_owned()),
                response: LanguageServerResponse::Result(Value::Null),
            })
        );
        assert_eq!(
            decode_language_server_message(
                br#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"no","data":[1]}}"#
            ),
            Ok(LanguageServerMessage::Response {
                id: LanguageServerRequestId::Number(3),
                response: LanguageServerResponse::Error { code: -32_601 },
            })
        );
        assert_eq!(
            decode_language_server_message(
                br#"{"jsonrpc":"2.0","method":"$/progress","params":{}}"#
            ),
            Ok(LanguageServerMessage::Notification {
                method: "$/progress".to_owned(),
                params: Some(json!({})),
            })
        );
        assert_eq!(
            decode_language_server_message(
                br#"{"jsonrpc":"2.0","id":4,"method":"workspace/applyEdit","params":{}}"#
            ),
            Ok(LanguageServerMessage::ServerRequest {
                id: LanguageServerRequestId::Number(4),
                method: "workspace/applyEdit".to_owned(),
            })
        );
        for invalid in [
            &br#"[]"#[..],
            br#"{"jsonrpc":"1.0","id":1,"result":1}"#,
            br#"{"id":1,"result":1}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":1,"extra":1}"#,
            br#"{"jsonrpc":"2.0","id":1,"id":2,"result":1}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":{"a":1,"a":2}}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":1,"error":{"code":1,"message":"x"}}"#,
            br#"{"jsonrpc":"2.0","id":1}"#,
            br#"{"jsonrpc":"2.0","id":null,"result":1}"#,
            br#"{"jsonrpc":"2.0","id":-1,"result":1}"#,
            br#"{"jsonrpc":"2.0","id":1.5,"result":1}"#,
            br#"{"jsonrpc":"2.0","id":"a b","result":1}"#,
            br#"{"jsonrpc":"2.0","result":1}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":1,"params":{}}"#,
            br#"{"jsonrpc":"2.0","method":"bad method"}"#,
            br#"{"jsonrpc":"2.0","method":"a","params":1}"#,
            br#"{"jsonrpc":"2.0","method":"a","result":1}"#,
            br#"{"jsonrpc":"2.0","id":1,"error":{"code":1}}"#,
            br#"{"jsonrpc":"2.0","id":1,"error":{"code":1,"message":"x","extra":1}}"#,
            br#"{"jsonrpc":"2.0","id":1,"error":{"code":1.5,"message":"x"}}"#,
            br#"not json"#,
        ] {
            assert_eq!(
                decode_language_server_message(invalid),
                Err(LanguageServerCodecError::MessageInvalid),
                "{}",
                String::from_utf8_lossy(invalid)
            );
        }
    }

    fn diagnostics(
        fixture: &Fixture,
        diagnostics: &Value,
    ) -> Result<LanguageServiceObservation, LanguageServerCodecError> {
        let message = body(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": format!("{ROOT}src/module.py"), "version": 3, "diagnostics": diagnostics},
        }));
        fixture.run(|context| observe_language_server_diagnostics(context, &message))
    }

    #[test]
    fn diagnostics_map_utf16_positions_to_exact_bytes_and_keep_only_digests() {
        let fixture = Fixture::new(LanguageServiceRequestKind::Diagnostics, 100);
        // "missing" starts after an astral character (two UTF-16 units, four
        // bytes) on the line after a CRLF-terminated line with a two-byte
        // letter.
        let observation = diagnostics(
            &fixture,
            &json!([
                {
                    "range": range((1, 18), (1, 25)),
                    "severity": 1,
                    "code": "undefined-name",
                    "source": "fixture",
                    "message": "SECRET-DIAGNOSTIC-TEXT is undefined",
                    "tags": [1],
                    "relatedInformation": [],
                },
                {"range": range((0, 4), (0, 9)), "message": "unused", "code": 12},
            ]),
        )
        .unwrap();
        assert!(verify_language_service_observation(&observation));
        assert_eq!(
            observation.status,
            LanguageServiceObservationStatus::Complete
        );
        let start = MODULE.find("missing").unwrap() as u64;
        let first_line = MODULE.find("na\u{ef}ve").unwrap() as u64;
        let items = &observation.items;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].item_id, "diagnostic-000000");
        assert_eq!(
            (items[0].range.start_byte, items[0].range.end_byte),
            (first_line, first_line + 6)
        );
        assert_eq!(
            (items[1].range.start_byte, items[1].range.end_byte),
            (start, start + 7)
        );
        assert_eq!(
            (
                items[1].range.start_line,
                items[1].range.start_column,
                items[1].range.end_column
            ),
            (2, 20, 27)
        );
        assert!(
            items
                .iter()
                .all(|item| item.untrusted_output && !item.write_authority)
        );
        let recorded = serde_json::to_string(&observation).unwrap();
        for hidden in [
            "SECRET-DIAGNOSTIC-TEXT",
            "undefined-name",
            "file:///",
            "workspace/fixture",
        ] {
            assert!(!recorded.contains(hidden), "{hidden}");
        }
        // An empty list is a complete answer for diagnostics.
        let empty = diagnostics(&fixture, &json!([])).unwrap();
        assert_eq!(
            (empty.status, empty.items.len()),
            (LanguageServiceObservationStatus::Complete, 0)
        );
        // More diagnostics than the descriptor allows yield a partial answer.
        let bounded = Fixture::new(LanguageServiceRequestKind::Diagnostics, 1);
        let partial = diagnostics(
            &bounded,
            &json!([
                {"range": range((1, 18), (1, 25)), "message": "a"},
                {"range": range((0, 0), (0, 3)), "message": "b"},
            ]),
        )
        .unwrap();
        assert_eq!(partial.status, LanguageServiceObservationStatus::Partial);
        assert_eq!(
            (partial.items.len(), partial.items[0].range.start_byte),
            (1, 0)
        );
        assert_eq!(
            partial.terminal_code.as_deref(),
            Some("language-server.items-truncated")
        );
    }

    #[test]
    fn invalid_diagnostics_and_positions_fail_while_a_character_past_the_line_end_clamps() {
        let fixture = Fixture::new(LanguageServiceRequestKind::Diagnostics, 100);
        let clamped = diagnostics(
            &fixture,
            &json!([{"range": range((0, 3), (0, 400)), "message": "m"}]),
        )
        .unwrap();
        let line_end = MODULE.find('\r').unwrap() as u64;
        assert_eq!(clamped.items[0].range.end_byte, line_end);
        for invalid in [
            // Inside the astral character's two UTF-16 units.
            json!([{"range": range((1, 13), (1, 14)), "message": "m"}]),
            // Past the last line.
            json!([{"range": range((3, 0), (3, 0)), "message": "m"}]),
            // End before start.
            json!([{"range": range((1, 5), (1, 2)), "message": "m"}]),
            json!([{"range": range((0, 0), (0, 1))}]),
            json!([{"range": range((0, 0), (0, 1)), "message": "m", "severity": 5}]),
            json!([{"range": range((0, 0), (0, 1)), "message": "m", "code": 1.5}]),
            json!([{"range": range((0, 0), (0, 1)), "message": "m", "command": {}}]),
            json!([{"range": {"start": {"line": 0, "character": 0}}, "message": "m"}]),
            json!([{"range": {"start": {"line": 0, "character": 0, "x": 1}, "end": {"line": 0, "character": 0}}, "message": "m"}]),
            json!({"not": "a list"}),
        ] {
            assert_eq!(
                diagnostics(&fixture, &invalid),
                Err(LanguageServerCodecError::ResultInvalid),
                "{invalid}"
            );
        }
        // Another file's diagnostics and other notifications are not this
        // request's answer.
        let other = body(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": format!("{ROOT}tests/test%20module.py"), "diagnostics": []},
        }));
        let progress = body(&json!({"jsonrpc": "2.0", "method": "$/progress", "params": {}}));
        for message in [other, progress, response(&json!([]))] {
            assert_eq!(
                fixture.run(|context| observe_language_server_diagnostics(context, &message)),
                Err(LanguageServerCodecError::UnexpectedMessage)
            );
        }
    }

    #[test]
    fn references_across_visible_files_are_sorted_numbered_and_bounded() {
        let fixture = Fixture::new(LanguageServiceRequestKind::References, 100);
        let result = json!([
            {"uri": format!("{ROOT}tests/test%20module.py"), "range": range((1, 7), (1, 12))},
            {"uri": format!("{ROOT}src/module.py"), "range": range((0, 4), (0, 9))},
        ]);
        let observation = fixture
            .run(|context| observe_language_server_references(context, &ID, &response(&result)))
            .unwrap();
        assert!(verify_language_service_observation(&observation));
        let paths = observation
            .items
            .iter()
            .map(|item| (item.item_id.as_str(), &item.path))
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                ("reference-000000", &fixture.module),
                ("reference-000001", &fixture.test)
            ]
        );
        let test_start = TEST.rfind("na\u{ef}ve").unwrap() as u64;
        assert_eq!(observation.items[1].range.start_byte, test_start);
        assert_eq!(
            observation.items[1].source_sha256,
            sha256_hex(TEST.as_bytes())
        );
        assert_eq!(
            observation.raw_response_sha256,
            Some(sha256_hex(&response(&result)))
        );
        let bounded = Fixture::new(LanguageServiceRequestKind::References, 1);
        let partial = bounded
            .run(|context| observe_language_server_references(context, &ID, &response(&result)))
            .unwrap();
        assert_eq!(
            (partial.status, partial.items.len(), partial.truncated),
            (LanguageServiceObservationStatus::Partial, 1, true)
        );
    }

    #[test]
    fn empty_failed_cancelled_and_foreign_answers_never_look_complete() {
        let fixture = Fixture::new(LanguageServiceRequestKind::References, 100);
        let cases = [
            (
                response(&Value::Null),
                LanguageServiceObservationStatus::Unavailable,
                EMPTY_RESULT,
            ),
            (
                response(&json!([])),
                LanguageServiceObservationStatus::Unavailable,
                EMPTY_RESULT,
            ),
            (
                body(
                    &json!({"jsonrpc": "2.0", "id": 7, "error": {"code": -32800, "message": "x"}}),
                ),
                LanguageServiceObservationStatus::Cancelled,
                "language-server.request-cancelled",
            ),
            (
                body(
                    &json!({"jsonrpc": "2.0", "id": 7, "error": {"code": -32603, "message": "SECRET"}}),
                ),
                LanguageServiceObservationStatus::Unavailable,
                "language-server.error-response",
            ),
        ];
        for (message, status, code) in cases {
            let observation = fixture
                .run(|context| observe_language_server_references(context, &ID, &message))
                .unwrap();
            assert!(verify_language_service_observation(&observation));
            assert_eq!(
                (observation.status, observation.terminal_code.as_deref()),
                (status, Some(code))
            );
            assert!(observation.items.is_empty() && observation.raw_response_sha256.is_none());
        }
        let other_id = body(&json!({"jsonrpc": "2.0", "id": 8, "result": []}));
        assert_eq!(
            fixture.run(|context| observe_language_server_references(context, &ID, &other_id)),
            Err(LanguageServerCodecError::UnexpectedMessage)
        );
    }

    #[test]
    fn paths_outside_the_root_traversal_and_unsupplied_files_fail() {
        let fixture = Fixture::new(LanguageServiceRequestKind::References, 100);
        for uri in [
            "file:///workspace/other/src/module.py".to_owned(),
            // Outside the root, although its tail names a visible file.
            "file:///src/module.py".to_owned(),
            format!("{ROOT}src/../src/module.py"),
            format!("{ROOT}src/%2E%2E/module.py"),
            format!("{ROOT}src%2Fmodule.py"),
            format!("{ROOT}src//module.py"),
            format!("{ROOT}tests/test module.py"),
            format!("{ROOT}src/module.py?x=1"),
            format!("{ROOT}src/module.py#top"),
            format!("{ROOT}src/modul%e"),
            format!("{ROOT}src/%FF.py"),
            format!("{ROOT}src/other.py"),
        ] {
            let result = json!([{"uri": uri, "range": range((0, 0), (0, 1))}]);
            assert_eq!(
                fixture.run(|context| observe_language_server_references(
                    context,
                    &ID,
                    &response(&result)
                )),
                Err(LanguageServerCodecError::ResultInvalid),
                "{uri}"
            );
        }
        // A visible file whose bytes were not supplied cannot hold a position.
        let only_module = [LanguageServerVisibleSource {
            path: &fixture.module,
            bytes: MODULE.as_bytes(),
        }];
        let result = json!([{"uri": format!("{ROOT}tests/test%20module.py"), "range": range((0, 0), (0, 1))}]);
        assert_eq!(
            observe_language_server_references(
                LanguageServerCodecContext {
                    descriptor: &fixture.descriptor,
                    request: &fixture.request,
                    root_uri: ROOT,
                    position_encoding: LanguageServerPositionEncoding::Utf16,
                    sources: &only_module,
                },
                &ID,
                &response(&result),
            ),
            Err(LanguageServerCodecError::ResultInvalid)
        );
    }

    #[test]
    fn rename_previews_keep_only_replacement_digests_and_refuse_file_operations() {
        let fixture = Fixture::new(LanguageServiceRequestKind::RenamePreview, 100);
        let edits = json!([
            {"range": range((1, 18), (1, 25)), "newText": "PRESENT_NAME"},
            {"range": range((0, 0), (0, 3)), "newText": "async def", "annotationId": "rename"},
        ]);
        let forms = [
            json!({"changes": {format!("{ROOT}src/module.py"): edits.clone()}}),
            json!({
                "documentChanges": [{"textDocument": {"uri": format!("{ROOT}src/module.py"), "version": 2}, "edits": edits}],
                "changeAnnotations": {"rename": {"label": "x"}},
            }),
        ];
        let mut observed = Vec::new();
        for form in &forms {
            let observation = fixture
                .run(|context| observe_language_server_rename(context, &ID, &response(form)))
                .unwrap();
            assert!(verify_language_service_observation(&observation));
            assert_eq!(
                observation.status,
                LanguageServiceObservationStatus::Complete
            );
            let recorded = serde_json::to_string(&observation).unwrap();
            assert!(!recorded.contains("PRESENT_NAME") && !recorded.contains("async def"));
            observed.push(observation.items);
        }
        assert_eq!(observed[0], observed[1]);
        assert_eq!(
            observed[0][0].replacement_sha256,
            Some(sha256_hex(b"async def"))
        );
        assert_eq!(
            observed[0][1].replacement_sha256,
            Some(sha256_hex(b"PRESENT_NAME"))
        );
        let module = format!("{ROOT}src/module.py");
        let rejected = [
            (
                json!({"documentChanges": [{"kind": "create", "uri": format!("{ROOT}src/new.py")}]}),
                "language-server.file-operation-refused",
            ),
            (
                json!({"documentChanges": [{"kind": "delete", "uri": module.clone()}]}),
                "language-server.file-operation-refused",
            ),
            (
                json!({"changes": {}, "documentChanges": []}),
                "language-server.ambiguous-edit",
            ),
            (
                json!({"changes": {module.clone(): [
                    {"range": range((0, 0), (0, 5)), "newText": "a"},
                    {"range": range((0, 4), (0, 8)), "newText": "b"},
                ]}}),
                "language-server.overlapping-edits",
            ),
            (
                json!({"changes": {module.clone(): [
                    {"range": range((0, 2), (0, 2)), "newText": "a"},
                    {"range": range((0, 2), (0, 2)), "newText": "b"},
                ]}}),
                "language-server.overlapping-edits",
            ),
        ];
        for (form, code) in rejected {
            let observation = fixture
                .run(|context| observe_language_server_rename(context, &ID, &response(&form)))
                .unwrap();
            assert_eq!(
                (observation.status, observation.terminal_code.as_deref()),
                (LanguageServiceObservationStatus::Rejected, Some(code))
            );
        }
        // Adjacent edits do not overlap.
        let adjacent = json!({"changes": {module.clone(): [
            {"range": range((0, 0), (0, 3)), "newText": "a"},
            {"range": range((0, 3), (0, 4)), "newText": "b"},
        ]}});
        assert!(
            fixture
                .run(|context| observe_language_server_rename(context, &ID, &response(&adjacent)))
                .is_ok()
        );
        for invalid in [
            json!({"changes": {module.clone(): [{"range": range((0, 0), (0, 1))}]}}),
            json!({"changes": {module.clone(): [{"range": range((0, 0), (0, 1)), "newText": "a", "extra": 1}]}}),
            json!({"documentChanges": [{"textDocument": {"uri": module.clone(), "version": "2"}, "edits": []}]}),
            json!({"documentChanges": [{"textDocument": {"uri": module.clone()}, "edits": [], "extra": 1}]}),
            json!({"changes": [], "other": 1}),
            json!([]),
        ] {
            assert_eq!(
                fixture.run(|context| observe_language_server_rename(
                    context,
                    &ID,
                    &response(&invalid)
                )),
                Err(LanguageServerCodecError::ResultInvalid),
                "{invalid}"
            );
        }
        let empty = fixture
            .run(|context| {
                observe_language_server_rename(context, &ID, &response(&json!({"changes": {}})))
            })
            .unwrap();
        assert_eq!(empty.terminal_code.as_deref(), Some(EMPTY_RESULT));
    }

    #[test]
    fn the_context_must_match_the_request_bytes_kind_and_root() {
        let fixture = Fixture::new(LanguageServiceRequestKind::References, 100);
        let message = response(&json!([]));
        let changed = MODULE.replace("missing", "present");
        let tampered = [LanguageServerVisibleSource {
            path: &fixture.module,
            bytes: changed.as_bytes(),
        }];
        let foreign_path = path(&["src", "foreign.py"]);
        let foreign = [LanguageServerVisibleSource {
            path: &foreign_path,
            bytes: MODULE.as_bytes(),
        }];
        let twice = [
            LanguageServerVisibleSource {
                path: &fixture.module,
                bytes: MODULE.as_bytes(),
            },
            LanguageServerVisibleSource {
                path: &fixture.module,
                bytes: MODULE.as_bytes(),
            },
        ];
        let module_only = [LanguageServerVisibleSource {
            path: &fixture.module,
            bytes: MODULE.as_bytes(),
        }];
        let cases: [(&str, &[LanguageServerVisibleSource<'_>]); 7] = [
            (ROOT, &tampered),
            (ROOT, &foreign),
            (ROOT, &twice),
            ("file:///workspace/fixture", &module_only),
            ("https://example.test/fixture/", &module_only),
            ("file:///workspace/fixture/?q/", &module_only),
            ("file:///workspace/fix ture/", &module_only),
        ];
        for (root_uri, sources) in cases {
            let context = LanguageServerCodecContext {
                descriptor: &fixture.descriptor,
                request: &fixture.request,
                root_uri,
                position_encoding: LanguageServerPositionEncoding::Utf16,
                sources,
            };
            assert_eq!(
                observe_language_server_references(context, &ID, &message),
                Err(LanguageServerCodecError::ContextInvalid),
                "{root_uri}"
            );
        }
        // A decode of another kind than the request's is refused.
        assert_eq!(
            fixture.run(|context| observe_language_server_rename(context, &ID, &message)),
            Err(LanguageServerCodecError::ContextInvalid)
        );
    }

    #[test]
    fn requests_are_encoded_at_the_request_position_in_the_negotiated_unit() {
        let fixture = Fixture::new(LanguageServiceRequestKind::References, 100);
        let frame = fixture
            .run(|context| encode_language_server_references_request(context, 7, true))
            .unwrap();
        let mut decoder = LanguageServerFrameDecoder::new();
        let bodies = decoder.push(&frame).unwrap();
        let request: Value = serde_json::from_slice(&bodies[0]).unwrap();
        assert_eq!(request["method"], "textDocument/references");
        assert_eq!(
            request["params"]["textDocument"]["uri"],
            format!("{ROOT}src/module.py")
        );
        assert_eq!(
            request["params"]["position"],
            json!({"line": 1, "character": 18})
        );
        assert_eq!(request["params"]["context"]["includeDeclaration"], true);
        let sources = [LanguageServerVisibleSource {
            path: &fixture.module,
            bytes: MODULE.as_bytes(),
        }];
        for (encoding, character) in [
            (LanguageServerPositionEncoding::Utf8, 20),
            (LanguageServerPositionEncoding::Utf32, 17),
        ] {
            let frame = encode_language_server_references_request(
                LanguageServerCodecContext {
                    descriptor: &fixture.descriptor,
                    request: &fixture.request,
                    root_uri: ROOT,
                    position_encoding: encoding,
                    sources: &sources,
                },
                7,
                false,
            )
            .unwrap();
            let body = LanguageServerFrameDecoder::new()
                .push(&frame)
                .unwrap()
                .remove(0);
            let request: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(request["params"]["position"]["character"], character);
        }
        // A path component with a space is percent-encoded and decodes back.
        assert_eq!(
            path_uri(ROOT, &fixture.test),
            format!("{ROOT}tests/test%20module.py")
        );
        let rename = Fixture::new(LanguageServiceRequestKind::RenamePreview, 100);
        assert_eq!(
            rename.run(|context| encode_language_server_rename_request(context, 8, "present")),
            Err(LanguageServerCodecError::ContextInvalid)
        );
        let frame = rename
            .run(|context| encode_language_server_rename_request(context, 8, "absent"))
            .unwrap();
        let body = LanguageServerFrameDecoder::new()
            .push(&frame)
            .unwrap()
            .remove(0);
        let request: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            (
                request["method"].as_str(),
                request["params"]["newName"].as_str()
            ),
            (Some("textDocument/rename"), Some("absent"))
        );
        // A source position inside a line break is refused.
        let mut inside_break = rename.request.clone();
        inside_break.source_byte = MODULE.find('\n').unwrap() as u64;
        let sources = [LanguageServerVisibleSource {
            path: &rename.module,
            bytes: MODULE.as_bytes(),
        }];
        assert_eq!(
            encode_language_server_rename_request(
                LanguageServerCodecContext {
                    descriptor: &rename.descriptor,
                    request: &inside_break,
                    root_uri: ROOT,
                    position_encoding: LanguageServerPositionEncoding::Utf16,
                    sources: &sources,
                },
                8,
                "absent",
            ),
            Err(LanguageServerCodecError::ContextInvalid)
        );
    }

    #[test]
    fn a_server_request_is_answered_only_by_a_refusal() {
        let refusal =
            encode_server_request_refusal(&LanguageServerRequestId::Text("apply-1".to_owned()))
                .unwrap();
        let body = LanguageServerFrameDecoder::new()
            .push(&refusal)
            .unwrap()
            .remove(0);
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["id"], "apply-1");
        assert_eq!(value["error"]["code"], METHOD_NOT_FOUND);
        assert!(value.get("result").is_none());
    }
}
