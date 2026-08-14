//! Bounded payload boundary used by the isolated read-only worker binary.

use crate::protocol::parse_closed_json;
use crate::{NeverCancelled, ReadOnlyToolKind, WorkspaceSnapshot, execute_read_only};

/// Maximum serialized sealed workspace projection accepted by the worker.
const MAX_SNAPSHOT_WIRE_BYTES: usize = 128 * 1024 * 1024;
/// Maximum closed request bytes accepted by the worker.
const MAX_REQUEST_WIRE_BYTES: usize = 64 * 1024;
/// Maximum serialized result bytes emitted by the worker.
const MAX_RESULT_WIRE_BYTES: usize = 4 * 1024 * 1024;

/// Content-free failure while decoding or encoding one isolated worker payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadOnlyWorkerError {
    /// The exact tool identity is not part of the closed catalog.
    ToolDenied,
    /// The request or snapshot wire payload exceeds its hard byte bound.
    InputLimitExceeded,
    /// The snapshot is malformed, duplicate-keyed, or schema-incompatible.
    SnapshotMalformed,
    /// The terminal result could not be encoded or exceeded its wire bound.
    ResultDenied,
}

impl ReadOnlyWorkerError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ToolDenied => "read_only.worker.tool_denied",
            Self::InputLimitExceeded => "read_only.worker.input_limit_exceeded",
            Self::SnapshotMalformed => "read_only.worker.snapshot_malformed",
            Self::ResultDenied => "read_only.worker.result_denied",
        }
    }
}

/// Executes one bounded worker payload without opening files or acquiring authority.
///
/// The platform worker binary supplies bytes from two fixed read-only mounts. This
/// function remains pure and is independently testable without a process or sandbox.
pub fn execute_worker_payload(
    tool_id: &str,
    tool_version: &str,
    request_bytes: &[u8],
    snapshot_bytes: &[u8],
) -> Result<Vec<u8>, ReadOnlyWorkerError> {
    let kind = ReadOnlyToolKind::ALL
        .into_iter()
        .find(|kind| kind.id() == tool_id && tool_version == crate::READ_ONLY_TOOL_VERSION)
        .ok_or(ReadOnlyWorkerError::ToolDenied)?;
    if request_bytes.is_empty()
        || request_bytes.len() > MAX_REQUEST_WIRE_BYTES
        || snapshot_bytes.is_empty()
        || snapshot_bytes.len() > MAX_SNAPSHOT_WIRE_BYTES
    {
        return Err(ReadOnlyWorkerError::InputLimitExceeded);
    }
    let snapshot: WorkspaceSnapshot =
        parse_closed_json(snapshot_bytes).map_err(|_| ReadOnlyWorkerError::SnapshotMalformed)?;
    let result = execute_read_only(kind, request_bytes, &snapshot, &NeverCancelled);
    if !result.verify(kind) {
        return Err(ReadOnlyWorkerError::ResultDenied);
    }
    let encoded = serde_json::to_vec(&result).map_err(|_| ReadOnlyWorkerError::ResultDenied)?;
    if encoded.len() > MAX_RESULT_WIRE_BYTES {
        return Err(ReadOnlyWorkerError::ResultDenied);
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use crate::{
        ReadOnlyEncoding, ReadOnlyLimits, ReadOnlyOutcome, ReadOnlyRequest, ReadOnlyResult,
        SnapshotEntry, SnapshotEntryKind, WorkspaceSnapshot,
    };

    use super::{ReadOnlyWorkerError, execute_worker_payload};

    #[test]
    fn exact_worker_payload_round_trips_a_verified_result() {
        let request = ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["note.txt".to_owned()]],
            query: None,
            byte_offset: None,
            byte_count: None,
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        };
        let snapshot = WorkspaceSnapshot {
            entries: vec![SnapshotEntry {
                path: vec!["note.txt".to_owned()],
                kind: SnapshotEntryKind::RegularFile,
                bytes: b"bounded worker\n".to_vec(),
                executable: false,
            }],
        };
        let output = execute_worker_payload(
            "agentmage.workspace.read-file",
            "1.0.0",
            &serde_json::to_vec(&request).expect("request"),
            &serde_json::to_vec(&snapshot).expect("snapshot"),
        )
        .expect("worker output");
        let result: ReadOnlyResult = serde_json::from_slice(&output).expect("result");
        assert_eq!(result.outcome, ReadOnlyOutcome::Succeeded);
        assert!(result.verify(crate::ReadOnlyToolKind::ReadText));
    }

    #[test]
    fn unknown_tool_oversize_and_duplicate_snapshot_fail_closed() {
        assert_eq!(
            execute_worker_payload("agentmage.workspace.write-file", "1.0.0", b"{}", b"{}"),
            Err(ReadOnlyWorkerError::ToolDenied)
        );
        assert_eq!(
            execute_worker_payload(
                "agentmage.workspace.read-file",
                "1.0.0",
                &vec![b'x'; 64 * 1024 + 1],
                b"{}"
            ),
            Err(ReadOnlyWorkerError::InputLimitExceeded)
        );
        let duplicate = br#"{"entries":[],"entries":[]}"#;
        assert_eq!(
            execute_worker_payload("agentmage.workspace.read-file", "1.0.0", b"{}", duplicate),
            Err(ReadOnlyWorkerError::SnapshotMalformed)
        );
    }
}
