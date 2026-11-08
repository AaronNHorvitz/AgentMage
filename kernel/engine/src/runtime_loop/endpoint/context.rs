// SPDX-License-Identifier: Apache-2.0
//! Bounded endpoint context admission without a native tokenizer or profile.

use std::io::{self, Write};

use agentmage_kernel_contracts::EndpointModelContext;
use sha2::{Digest, Sha256};

use super::RuntimePortFailure;

const MAX_MESSAGES: usize = 256;
const MAX_SERIALIZED_BYTES: usize = 256 * 1024;

/// Closed text roles. No role can authorize a tool or confer trust on its text.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeEndpointMessageRole {
    /// Application instructions supplied by the context owner.
    System,
    /// Person-directed input, including explicitly untrusted source excerpts.
    User,
    /// Previous untrusted model text.
    Assistant,
}

/// Original borrowed text. Deliberately neither Debug nor Deserialize; admission
/// never copies the plaintext into another buffer or reads it from a descriptor.
#[derive(serde::Serialize)]
pub struct RuntimeEndpointMessage<'a> {
    /// Closed conversation role.
    pub role: RuntimeEndpointMessageRole,
    /// Exact original UTF-8 message content.
    pub content: &'a str,
}

/// Immutable content-free observations derived by the kernel from original
/// ordered messages, independently of the prepared transport's descriptor.
/// This is not a native context packet, token budget or disclosure approval.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RuntimeEndpointContextPacket {
    version: u16,
    messages: EndpointModelContext,
    content_bytes: usize,
}

impl RuntimeEndpointContextPacket {
    /// Admits 1–256 nonempty text messages ending in a user message. Both content
    /// and the exact JSON array (roles and escaping included) fit 256 KiB. The
    /// streaming writer refuses before hashing any byte beyond that ceiling.
    pub fn from_messages(
        messages: &[RuntimeEndpointMessage<'_>],
    ) -> Result<Self, RuntimePortFailure> {
        if messages.is_empty()
            || messages.len() > MAX_MESSAGES
            || messages.last().map(|message| message.role) != Some(RuntimeEndpointMessageRole::User)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let mut content_bytes = 0_usize;
        for message in messages {
            if message.content.is_empty() {
                return Err(RuntimePortFailure::Invalid);
            }
            content_bytes = content_bytes
                .checked_add(message.content.len())
                .filter(|bytes| *bytes <= MAX_SERIALIZED_BYTES)
                .ok_or(RuntimePortFailure::ResourceExhausted)?;
        }
        let mut writer = ContextFingerprint::default();
        serde_json::to_writer(&mut writer, messages)
            .map_err(|_| RuntimePortFailure::ResourceExhausted)?;
        Ok(Self {
            version: 1,
            messages: EndpointModelContext {
                message_count: messages.len(),
                serialized_messages_bytes: writer.bytes,
                serialized_messages_sha256: writer
                    .hash
                    .finalize()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
            },
            content_bytes,
        })
    }

    /// Exact ordered JSON message-array observations, never rendered tokens.
    #[must_use]
    pub const fn messages(&self) -> &EndpointModelContext {
        &self.messages
    }

    /// Original UTF-8 content bytes excluding roles, JSON syntax and escaping.
    #[must_use]
    pub const fn content_bytes(&self) -> usize {
        self.content_bytes
    }
}

#[derive(Default)]
struct ContextFingerprint {
    bytes: usize,
    hash: Sha256,
}

impl Write for ContextFingerprint {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .checked_add(bytes.len())
            .filter(|next| *next <= MAX_SERIALIZED_BYTES)
            .ok_or_else(|| io::Error::other("endpoint context byte limit"))?;
        self.hash.update(bytes);
        self.bytes = next;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
