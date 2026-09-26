//! Operation-scoped worker; the parent owns grants, admission, confinement and receipts.
//! No host registration or production activation is supplied by this optional binary.
#![forbid(unsafe_code)]

#[path = "../public_research_target.rs"]
mod target;
#[path = "../public_research_transport.rs"]
mod transport;

use std::io::{Read, Write};
use std::process::ExitCode;

use agentmage_kernel_engine::research_fetch::PublicGetWorkerPacket;

fn run() -> Result<(), transport::WorkerError> {
    if std::env::args_os().count() != 1
        || std::env::vars_os()
            .any(|(key, _)| !["LANG", "LC_ALL", "TZ"].contains(&key.to_string_lossy().as_ref()))
    {
        return Err(transport::WorkerError::Environment);
    }
    // Defense in depth, not proof of the exact sandbox. The independently admitted
    // parent must verify the full namespace/filter/runtime manifest before dispatch.
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|_| transport::WorkerError::Environment)?;
    if !status.lines().any(|line| line == "NoNewPrivs:\t1")
        || !status.lines().any(|line| line == "Seccomp:\t2")
    {
        return Err(transport::WorkerError::Environment);
    }
    // The sandbox uses stdin to install seccomp. The request is instead an exact
    // sealed descriptor projected read-only at this fixed path, never a workspace.
    let input = std::fs::File::from(
        rustix::fs::open(
            "/input/request",
            rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        )
        .map_err(|_| transport::WorkerError::Input)?,
    );
    let metadata = input
        .metadata()
        .map_err(|_| transport::WorkerError::Input)?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > 16 * 1024 {
        return Err(transport::WorkerError::Input);
    }
    let mut bytes = Vec::new();
    input
        .take(16 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| transport::WorkerError::Input)?;
    let now = transport::epoch_ms()?;
    let packet = PublicGetWorkerPacket::decode_worker_packet(&bytes, now)
        .map_err(|_| transport::WorkerError::Input)?;
    let response = transport::execute(&packet)?;
    std::io::stdout()
        .lock()
        .write_all(response.frame())
        .map_err(|_| transport::WorkerError::Output)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Never format a URL, library error, query, environment or response body.
            eprintln!("{}", error.code());
            ExitCode::from(5)
        }
    }
}
