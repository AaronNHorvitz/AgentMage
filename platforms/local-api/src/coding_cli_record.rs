// SPDX-License-Identifier: Apache-2.0
//! Explicit event-record export and offline replay; never execution recovery.

use super::*;
use agentmage_kernel_contracts::{
    AgentStateKind, RuntimeEndpointRunRequest, RuntimeEvent, RuntimeEventKind, RuntimeSessionMode,
};
use agentmage_kernel_engine::{
    runtime_coordinator::verify_runtime_endpoint_run_request, runtime_event::RuntimeEventSequence,
};
use serde::{Deserialize, Serialize};

const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;
const MAX_RECORD_EVENTS: usize = 4096;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    request: RuntimeEndpointRunRequest,
    events: Vec<RuntimeEvent>,
}

// Reuse the original kernel event state machine, including hash chaining,
// causation, immutable stream binding, legal transitions and terminal ordering.
// Self-consistency proves neither authenticity nor the referenced payloads.
fn verify(record: &Record) -> Result<AgentStateKind> {
    verify_runtime_endpoint_run_request(&record.request).map_err(|_| ())?;
    if record.version != 1
        || !matches!(
            record.request.mode,
            RuntimeSessionMode::EphemeralReadOnly | RuntimeSessionMode::ControlledWrite
        )
        || record.request.event_cursor.is_some()
        || record.events.is_empty()
        || record.events.len() > MAX_RECORD_EVENTS
        || record.events.len() as u64 > u64::from(record.request.limits.max_events)
    {
        return Err(());
    }
    let Some(RuntimeEventKind::RunStarted { request_sha256 }) =
        record.events.first().map(|event| &event.kind)
    else {
        return Err(());
    };
    if request_sha256 != &record.request.request_sha256 {
        return Err(());
    }
    let mut chain = RuntimeEventSequence::new();
    for event in &record.events {
        if event.run_id != record.request.run_id
            || event.session_id != record.request.session_id
            || event.task_id != record.request.task.task_id
            || event.policy_id != record.request.policy_id
        {
            return Err(());
        }
        chain.push(event).map_err(|_| ())?;
    }
    match record.events.last().map(|event| &event.kind) {
        Some(RuntimeEventKind::RunTerminal { state, .. }) if chain.is_terminal() => Ok(*state),
        _ => Err(()),
    }
}

/// Print one complete terminal chain. No filesystem writes, model disclosure,
/// advance, approval, cancellation or release route is called. A running/faulted
/// prefix refuses. The caller chooses whether and where to retain stdout.
pub(super) fn export(client: &Client, request: RuntimeEndpointRunRequest) -> Result<()> {
    let client = Client {
        address: client.address,
        auth: client.auth.clone(),
        deadline: Some(Instant::now() + DEADLINE),
        controls: None,
    };
    export_observed(&client, request)
}

/// The guided continuation retains its original deadline and interruption latch.
/// Emission succeeds before it can ask to discard the process-held record.
pub(super) fn export_observed(client: &Client, request: RuntimeEndpointRunRequest) -> Result<()> {
    let encoded = collect(client, request)?;
    let mut output = std::io::stdout().lock();
    writeln!(output, "{encoded}").map_err(|_| ())?;
    output.flush().map_err(|_| ())
}

fn collect(client: &Client, request: RuntimeEndpointRunRequest) -> Result<String> {
    let mut record = Record {
        version: 1,
        request,
        events: Vec::new(),
    };
    let mut bytes = serde_json::to_vec(&record).map_err(|_| ())?.len();
    let mut total = None;
    loop {
        let page = endpoint::event_page(client, &record.request, record.events.len(), true)?;
        let page = &page["page"];
        let count = page["total"].as_u64().ok_or(())?;
        if count == 0
            || count > MAX_RECORD_EVENTS as u64
            || count > u64::from(record.request.limits.max_events)
            || total.is_some_and(|prior| prior != count)
        {
            return Err(());
        }
        total = Some(count);
        let mut events: Vec<RuntimeEvent> =
            serde_json::from_value(page["events"].clone()).map_err(|_| ())?;
        if events.len() != 1 {
            return Err(());
        }
        let event = events.pop().ok_or(())?;
        bytes = bytes
            .checked_add(serde_json::to_vec(&event).map_err(|_| ())?.len() + 1)
            .filter(|size| *size < MAX_RECORD_BYTES)
            .ok_or(())?;
        record.events.push(event);
        if record.events.len() as u64 == count {
            break;
        }
    }
    verify(&record)?;
    if client
        .deadline
        .is_none_or(|deadline| Instant::now() >= deadline)
    {
        return Err(());
    }
    // Escape terminal controls and non-ASCII text with the existing renderer.
    // Check its actual expanded output, including the final newline, before
    // emitting anything. Oversize/refused exports leave stdout empty.
    let encoded = safe(&serde_json::to_value(record).map_err(|_| ())?)?;
    if encoded.len() >= MAX_RECORD_BYTES {
        return Err(());
    }
    client.check_running()?;
    Ok(encoded)
}

/// Explicit local record retention, not an execution checkpoint. Hold the
/// selected private directory across confirmation and never replace a name.
pub(super) fn save_observed(
    client: &Client,
    request: RuntimeEndpointRunRequest,
    directory: &Path,
) -> Result<()> {
    use rustix::fs::{Mode, OFlags, openat};
    use std::fs::File;
    use std::os::unix::fs::MetadataExt;

    fn selected(path: &Path) -> Result<File> {
        if !path.is_absolute()
            || path.as_os_str().len() > 4096
            || std::fs::canonicalize(path).map_err(|_| ())?.as_os_str() != path.as_os_str()
        {
            return Err(());
        }
        crate::protected_directory(path).map_err(|_| ())
    }
    fn same_directory(path: &Path, held: &File) -> Result<()> {
        let current = selected(path)?.metadata().map_err(|_| ())?;
        let held = held.metadata().map_err(|_| ())?;
        if (current.dev(), current.ino(), current.mode(), current.uid())
            != (held.dev(), held.ino(), held.mode(), held.uid())
        {
            return Err(());
        }
        Ok(())
    }

    let held = selected(directory)?;
    let name = format!("{}.json", request.request_sha256);
    let destination = directory.join(&name);
    let mut encoded = Zeroizing::new(collect(client, request.clone())?);
    encoded.push('\n');
    client.confirm(
        &json!({"operation":"save-terminal-record","request":request,
            "destination":destination,"byte_size":encoded.len(),
            "warning":"Saves the complete terminal event record as plaintext in this existing private directory. It may contain task and tool observations. Creates one owner-only file, never overwrites. No model, tool, release or execution recovery. A failure after creation may leave a partial file; inspect disk, never automatically retry."}),
        &format!("save-record {}", request.request_sha256),
    )?;
    client.check_running()?;
    if client
        .deadline
        .is_some_and(|deadline| Instant::now() >= deadline)
    {
        return Err(());
    }
    same_directory(directory, &held)?;
    let mut file = File::from(
        openat(
            &held,
            name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|_| ())?,
    );
    // Once the name is created all errors are potentially effectful. Retain it
    // for inspection; cleanup could delete a later user replacement. No retry.
    let saved = (|| -> Result<()> {
        file.write_all(encoded.as_bytes()).map_err(|_| ())?;
        file.sync_all().map_err(|_| ())?;
        held.sync_all().map_err(|_| ())?;
        same_directory(directory, &held)?;
        let metadata = file.metadata().map_err(|_| ())?;
        if !metadata.is_file()
            || metadata.mode() & 0o7777 != 0o600
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.nlink() != 1
            || metadata.len() != encoded.len() as u64
        {
            return Err(());
        }
        let named = File::from(
            openat(
                &held,
                name.as_str(),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| ())?,
        )
        .metadata()
        .map_err(|_| ())?;
        if (named.dev(), named.ino()) != (metadata.dev(), metadata.ino()) {
            return Err(());
        }
        Ok(())
    })();
    if saved.is_err() {
        eprintln!(
            "record-save.uncertain: inspect the displayed destination; do not retry automatically"
        );
        return Err(());
    }
    client.check_running()?;
    show(&json!({"operation":"save-terminal-record","saved":true,
        "destination":destination,"byte_size":encoded.len(),
        "request_sha256":request.request_sha256,"plaintext":true,
        "resumes_execution":false,"authenticity_verified":false,
        "referenced_payloads_verified":false}))?;
    std::io::stdout().flush().map_err(|_| ())
}

/// Reads only bounded explicitly supplied stdin, before any API client/auth.
/// Replay displays the exact recorded envelopes; it starts no model or tool and
/// cannot recreate a port, grant, approval, payload or continuation cursor.
pub(super) fn replay() -> Result<()> {
    let mut bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .lock()
        .take(MAX_RECORD_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(());
    }
    let record: Record = agentmage_host::runtime_transport::decode_exact(&bytes).ok_or(())?;
    let state = verify(&record)?;
    show(&json!({
        "verification":"canonical-event-chain-only",
        "authenticity_verified":false,
        "referenced_payloads_verified":false,
        "resumes_execution":false,
        "recorded_terminal_state":state,
        "event_count":record.events.len(),
        "record":record,
    }))
}
