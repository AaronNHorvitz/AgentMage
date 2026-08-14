#![forbid(unsafe_code)]
//! Fixed-path executable for one isolated read-only worker attempt.

use std::fs::File;
use std::io::{Read, Write};

const REQUEST_PATH: &str = "/input/request";
const SNAPSHOT_PATH: &str = "/input/snapshot";
const MAX_REQUEST_BYTES: u64 = 64 * 1024;
const MAX_SNAPSHOT_BYTES: u64 = 128 * 1024 * 1024;

fn main() -> Result<(), WorkerExit> {
    let mut arguments = std::env::args();
    let _program = arguments.next();
    let tool_id = arguments.next().ok_or(WorkerExit)?;
    let tool_version = arguments.next().ok_or(WorkerExit)?;
    if arguments.next().is_some() {
        return Err(WorkerExit);
    }
    let request = read_bounded(REQUEST_PATH, MAX_REQUEST_BYTES)?;
    let snapshot = read_bounded(SNAPSHOT_PATH, MAX_SNAPSHOT_BYTES)?;
    let output = agentmage_capability_read_only::execute_worker_payload(
        &tool_id,
        &tool_version,
        &request,
        &snapshot,
    )
    .map_err(|_| WorkerExit)?;
    std::io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|_| WorkerExit)
}

fn read_bounded(path: &str, limit: u64) -> Result<Vec<u8>, WorkerExit> {
    let file = File::open(path).map_err(|_| WorkerExit)?;
    let size = file.metadata().map_err(|_| WorkerExit)?.len();
    if size == 0 || size > limit {
        return Err(WorkerExit);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(size).map_err(|_| WorkerExit)?);
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| WorkerExit)?;
    if bytes.len() as u64 != size {
        return Err(WorkerExit);
    }
    Ok(bytes)
}

struct WorkerExit;

impl std::fmt::Debug for WorkerExit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("read_only.worker.failed")
    }
}
