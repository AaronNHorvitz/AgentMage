// SPDX-License-Identifier: Apache-2.0
//! Bounded observation of an explicitly shared run. Never schedules an effect.

use super::*;
use agentmage_kernel_contracts::{RuntimeEvent, RuntimeEventKind};
use agentmage_kernel_engine::runtime_event::RuntimeEventSequence;

const MAX_EVENTS: usize = 4096;
const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_PAGES: usize = 40_096;
const POLL: Duration = Duration::from_millis(100);

pub(super) fn run(
    client: &Client,
    seconds: &str,
    run: &str,
    hash: &str,
    consent: &str,
) -> Result<()> {
    endpoint::binding(run, hash)?;
    let seconds = seconds.parse::<u64>().map_err(|_| ())?;
    if consent != "--share-selected-endpoint" || !(1..=3600).contains(&seconds) {
        return Err(());
    }
    let controls = driver::Signals::new(seconds)?;
    let client = Client {
        address: client.address,
        auth: client.auth.clone(),
        deadline: Some(controls.deadline()),
        controls: Some(controls),
    };
    // Ordinary credential only, even when a control credential is present.
    // The service must have explicitly delegated observations at startup.
    let request = endpoint::checked_selection(&client, run, hash, false)?;
    eprintln!(
        "Watching shared service observations. Task and tool data are untrusted. Stopping this watcher leaves the run held; use separate controls for decisions."
    );
    let mut chain = RuntimeEventSequence::new();
    let mut after = 0;
    let mut total = 0;
    let mut output_bytes = 0usize;
    for _ in 0..MAX_PAGES {
        client.check_running()?;
        let mut body = endpoint::binding(run, hash)?;
        body["after"] = json!(after);
        let Some(page) = client.observation_page(body)? else {
            client.check_running()?;
            std::thread::sleep(POLL);
            continue;
        };
        let page = endpoint::checked_event_page(&request, after, page)?;
        let page = &page["page"];
        let count = page["total"].as_u64().ok_or(())?;
        if count < total
            || count > MAX_EVENTS as u64
            || count > u64::from(request.limits.max_events)
        {
            return Err(());
        }
        total = count;
        let mut events: Vec<RuntimeEvent> =
            serde_json::from_value(page["events"].clone()).map_err(|_| ())?;
        if events.is_empty() {
            // No busy loop at the current end of an active prefix. Signals and
            // the original deadline are checked before the next HTTP request.
            client.check_running()?;
            std::thread::sleep(POLL);
            continue;
        }
        let event = events.pop().ok_or(())?;
        if after == 0
            && !matches!(&event.kind, RuntimeEventKind::RunStarted { request_sha256 }
                if request_sha256 == &request.request_sha256)
        {
            return Err(());
        }
        chain.push(&event).map_err(|_| ())?;
        after += 1;
        if chain.is_terminal() && after as u64 != total {
            return Err(());
        }
        let encoded = safe(&json!({
            "event":event,
            "verification":"self-consistent-service-observation",
            "index_is_resume_cursor":false,
            "terminal_observed":chain.is_terminal(),
        }))?;
        output_bytes = output_bytes
            .checked_add(encoded.len() + 1)
            .filter(|n| *n <= MAX_OUTPUT_BYTES)
            .ok_or(())?;
        client.check_running()?;
        let mut output = std::io::stdout().lock();
        writeln!(output, "{encoded}").map_err(|_| ())?;
        output.flush().map_err(|_| ())?;
        if chain.is_terminal() {
            return client.check_running();
        }
    }
    Err(())
}
