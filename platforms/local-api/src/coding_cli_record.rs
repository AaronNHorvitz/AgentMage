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
        || record.request.mode != RuntimeSessionMode::EphemeralReadOnly
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
    };
    let mut record = Record {
        version: 1,
        request,
        events: Vec::new(),
    };
    let mut bytes = serde_json::to_vec(&record).map_err(|_| ())?.len();
    let mut total = None;
    loop {
        let page = endpoint::event_page(&client, &record.request, record.events.len(), true)?;
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
    writeln!(std::io::stdout().lock(), "{encoded}").map_err(|_| ())
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
