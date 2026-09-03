//! Interface-neutral source-manifest identity shared by native Chat and headless callers.

use std::collections::BTreeSet;

use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_IDENTIFIER_BYTES: usize = 128;

/// Content-free failure while sealing an exact caller source manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceManifestError {
    /// A request, source, artifact, or reason identity is malformed.
    InvalidIdentity,
    /// A digest is not lowercase hexadecimal SHA-256.
    InvalidDigest,
    /// A source is not completely included with nonzero byte accounting.
    IncompleteSource,
    /// Two source records use the same current-request identity.
    DuplicateSource,
    /// Canonical serialization failed.
    Serialization,
}

/// Exact prompt artifact admitted for one participant or headless request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceManifestPrompt {
    /// Runtime artifact identity containing the exact prompt bytes.
    pub artifact_id: String,
    /// Exact prompt byte count.
    pub byte_length: u64,
    /// SHA-256 of the admitted prompt bytes.
    pub source_sha256: String,
}

/// Exact current-request source record shared across caller presentations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceManifestRecord {
    /// Runtime artifact identity containing the exact source bytes.
    pub artifact_id: String,
    /// Exact admitted source byte count.
    pub byte_length: u64,
    /// SHA-256 of the caller-visible source descriptor.
    pub descriptor_sha256: String,
    /// Stable content-free inclusion reason.
    pub reason_code: String,
    /// Caller-neutral identity of this current-request source.
    pub reference_id: String,
    /// SHA-256 of the admitted source bytes.
    pub source_sha256: String,
    /// Exact terminal accounting state; only `included` is admissible for submission.
    pub state: String,
}

/// Canonical source-manifest preimage used by native Chat and headless submission paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceManifestInput {
    /// Optional stable participant command, without prompt content.
    pub command: Option<String>,
    /// Exact admitted prompt artifact.
    pub prompt: SourceManifestPrompt,
    /// Current caller request identity.
    pub request_id: String,
    /// Ordered, completely accounted current-request sources.
    pub sources: Vec<SourceManifestRecord>,
}

/// Validates complete accounting and returns the canonical same-byte manifest SHA-256.
pub fn source_manifest_sha256(input: &SourceManifestInput) -> Result<String, SourceManifestError> {
    if !valid_identity(&input.request_id)
        || input
            .command
            .as_ref()
            .is_some_and(|value| !valid_identity(value))
        || !valid_identity(&input.prompt.artifact_id)
        || input.prompt.byte_length == 0
        || !valid_sha256(&input.prompt.source_sha256)
    {
        return Err(SourceManifestError::InvalidIdentity);
    }
    let mut identities = BTreeSet::new();
    for source in &input.sources {
        if !identities.insert(source.reference_id.as_str()) {
            return Err(SourceManifestError::DuplicateSource);
        }
        if !valid_identity(&source.reference_id)
            || !valid_identity(&source.artifact_id)
            || !valid_identity(&source.reason_code)
        {
            return Err(SourceManifestError::InvalidIdentity);
        }
        if !valid_sha256(&source.descriptor_sha256) || !valid_sha256(&source.source_sha256) {
            return Err(SourceManifestError::InvalidDigest);
        }
        if source.state != "included" || source.byte_length == 0 {
            return Err(SourceManifestError::IncompleteSource);
        }
    }
    if !valid_sha256(&input.prompt.source_sha256) {
        return Err(SourceManifestError::InvalidDigest);
    }
    let bytes = serde_json::to_vec(input).map_err(|_| SourceManifestError::Serialization)?;
    Ok(sha256(&bytes))
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{
        SourceManifestError, SourceManifestInput, SourceManifestPrompt, SourceManifestRecord,
        source_manifest_sha256,
    };

    fn record(
        reference: &str,
        artifact: &str,
        descriptor: char,
        source: char,
        bytes: u64,
    ) -> SourceManifestRecord {
        SourceManifestRecord {
            artifact_id: artifact.to_owned(),
            byte_length: bytes,
            descriptor_sha256: descriptor.to_string().repeat(64),
            reason_code: "participant.source.included".to_owned(),
            reference_id: reference.to_owned(),
            source_sha256: source.to_string().repeat(64),
            state: "included".to_owned(),
        }
    }

    fn fixture() -> SourceManifestInput {
        SourceManifestInput {
            command: None,
            prompt: SourceManifestPrompt {
                artifact_id: "artifact-prompt".to_owned(),
                byte_length: 7,
                source_sha256: "a".repeat(64),
            },
            request_id: "participant-parity".to_owned(),
            sources: vec![
                record("ref-a", "artifact-a", 'b', 'c', 5),
                record("ref-b", "artifact-b", 'd', 'e', 4),
            ],
        }
    }

    #[test]
    fn headless_and_native_chat_share_the_exact_manifest_vector() {
        assert_eq!(
            source_manifest_sha256(&fixture()),
            Ok("81c4e8382c63a5894b75756ca09843640e71fd7156907e478f9b858c415e8cd3".to_owned())
        );
    }

    #[test]
    fn incomplete_duplicate_or_mutated_sources_fail_closed() {
        let mut incomplete = fixture();
        incomplete.sources[0].state = "omitted".to_owned();
        assert_eq!(
            source_manifest_sha256(&incomplete),
            Err(SourceManifestError::IncompleteSource)
        );

        let mut duplicate = fixture();
        duplicate.sources[1].reference_id = "ref-a".to_owned();
        assert_eq!(
            source_manifest_sha256(&duplicate),
            Err(SourceManifestError::DuplicateSource)
        );

        let mut mutated = fixture();
        mutated.sources[1].source_sha256 = "G".repeat(64);
        assert_eq!(
            source_manifest_sha256(&mutated),
            Err(SourceManifestError::InvalidDigest)
        );
    }
}
