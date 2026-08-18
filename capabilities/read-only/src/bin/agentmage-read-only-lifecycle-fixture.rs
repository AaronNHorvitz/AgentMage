#![forbid(unsafe_code)]
//! Feature-gated lifecycle fault fixture; never included in product packages.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

const REQUEST_PATH: &str = "/input/request";
const SNAPSHOT_PATH: &str = "/input/snapshot";
const SCRATCH_PATH: &str = "/tmp/agentmage-lifecycle-scratch";
const MAX_REQUEST_BYTES: u64 = 64 * 1024;
const MAX_SNAPSHOT_BYTES: u64 = 128 * 1024 * 1024;

fn main() -> Result<(), FixtureExit> {
    let arguments = std::env::args().collect::<Vec<_>>();
    let executable = Path::new(arguments.first().ok_or(FixtureExit)?);
    let name = executable
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(FixtureExit)?;
    let (_, phase) = lifecycle_mode(name).ok_or(FixtureExit)?;
    let tool_id = arguments.get(1).ok_or(FixtureExit)?;
    let tool_version = arguments.get(2).ok_or(FixtureExit)?;
    if arguments.len() != 3 {
        return Err(FixtureExit);
    }

    std::fs::write(SCRATCH_PATH, b"bounded lifecycle scratch").map_err(|_| FixtureExit)?;

    match phase {
        "before" => {}
        "during" => {
            std::io::stdout()
                .lock()
                .write_all(b"{")
                .map_err(|_| FixtureExit)?;
            std::io::stdout().lock().flush().map_err(|_| FixtureExit)?;
        }
        "after" => {
            let request = read_bounded(REQUEST_PATH, MAX_REQUEST_BYTES)?;
            let snapshot = read_bounded(SNAPSHOT_PATH, MAX_SNAPSHOT_BYTES)?;
            let output = agentmage_capability_read_only::execute_worker_payload(
                tool_id,
                tool_version,
                &request,
                &snapshot,
            )
            .map_err(|_| FixtureExit)?;
            std::io::stdout()
                .lock()
                .write_all(&output)
                .map_err(|_| FixtureExit)?;
            std::io::stdout().lock().flush().map_err(|_| FixtureExit)?;
        }
        _ => return Err(FixtureExit),
    }

    if name.contains("-crash-") {
        thread::sleep(Duration::from_millis(250));
        std::process::abort();
    }
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn lifecycle_mode(name: &str) -> Option<(&str, &str)> {
    let mode = name.strip_prefix("agentmage-lifecycle-")?;
    let (termination, phase) = mode.split_once('-')?;
    matches!(termination, "cancel" | "timeout" | "kill" | "crash")
        .then_some((termination, phase))
        .filter(|(_, phase)| matches!(*phase, "before" | "during" | "after"))
}

fn read_bounded(path: &str, limit: u64) -> Result<Vec<u8>, FixtureExit> {
    let file = File::open(path).map_err(|_| FixtureExit)?;
    let size = file.metadata().map_err(|_| FixtureExit)?.len();
    if size == 0 || size > limit {
        return Err(FixtureExit);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(size).map_err(|_| FixtureExit)?);
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| FixtureExit)?;
    if bytes.len() as u64 != size {
        return Err(FixtureExit);
    }
    Ok(bytes)
}

struct FixtureExit;

impl std::fmt::Debug for FixtureExit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("read_only.lifecycle_fixture.failed")
    }
}
