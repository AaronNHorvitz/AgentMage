// SPDX-License-Identifier: Apache-2.0
//! Explicit terminal operations on the original retained endpoint HTTP owner.

use super::*;
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeEndpointRunRequest, RuntimeEvent, RuntimeSessionMode,
};
use agentmage_kernel_engine::{
    runtime_coordinator::{
        RuntimeEndpointOutcome, verify_runtime_approval_challenge,
        verify_runtime_endpoint_run_request,
    },
    runtime_event::verify_runtime_event,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    request: RuntimeEndpointRunRequest,
    retention: String,
    configuration: String,
}

#[derive(Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
enum Step {
    #[serde(rename = "awaiting_approval")]
    AwaitingApproval { challenge: RuntimeApprovalChallenge },
    #[serde(rename = "terminal")]
    Terminal { outcome: RuntimeEndpointOutcome },
}

fn selected(client: &Client, control: bool) -> Result<RuntimeEndpointRunRequest> {
    let selection: Selection =
        serde_json::from_value(client.json("GET", "/v1/code/endpoint/request", control, None)?)
            .map_err(|_| ())?;
    verify_runtime_endpoint_run_request(&selection.request).map_err(|_| ())?;
    if selection.retention != "process-only"
        || selection.configuration != "fixed-for-service"
        || !matches!(
            selection.request.mode,
            RuntimeSessionMode::EphemeralReadOnly | RuntimeSessionMode::ControlledWrite
        )
        || selection.request.event_cursor.is_some()
    {
        return Err(());
    }
    Ok(selection.request)
}

pub(super) fn binding(run: &str, hash: &str) -> Result<Value> {
    if run.is_empty()
        || run.len() > 128
        || run.bytes().any(|b| !b.is_ascii_graphic())
        || !digest(hash)
    {
        return Err(());
    }
    Ok(json!({"run_id":run,"request_sha256":hash}))
}

pub(super) fn checked_selection(
    client: &Client,
    run: &str,
    hash: &str,
    control: bool,
) -> Result<RuntimeEndpointRunRequest> {
    binding(run, hash)?;
    let request = selected(client, control)?;
    if request.run_id.as_str() != run || request.request_sha256 != hash {
        return Err(());
    }
    Ok(request)
}

fn check_challenge(
    request: &RuntimeEndpointRunRequest,
    challenge: &RuntimeApprovalChallenge,
) -> Result<()> {
    verify_runtime_approval_challenge(challenge).map_err(|_| ())?;
    if challenge.run_id != request.run_id
        || challenge.task_id != request.task.task_id
        || !request.visible_tools.iter().any(|tool| {
            tool.tool_id == challenge.presentation.tool_id
                && tool.tool_version == challenge.presentation.tool_version
        })
    {
        return Err(());
    }
    Ok(())
}

fn advance(
    client: &Client,
    request: &RuntimeEndpointRunRequest,
    response: Option<RuntimeApprovalResponse>,
    disclosure_step: Option<u32>,
) -> Result<Step> {
    // Each command is a fresh disclosure decision, including deny: advancing
    // after a denied tool can still send observations to the selected model.
    let disclosure = match disclosure_step {
        Some(step) => format!("share {} step-{step}", request.request_sha256),
        None => format!("share {}", request.request_sha256),
    };
    client.confirm(
        &json!({"request":request,"response":response,"disclosure_step":disclosure_step,
        "warning":"Shares this run's task, consented source excerpts and current tool observations with the displayed endpoint. Advances the same retained coordinator until a tool approval or terminal observation. No automatic tool approval, retry or transcript retention. A lost reply is uncertain: inspect events before any further action."}),
        &disclosure,
    )?;
    let mut body = binding(request.run_id.as_str(), &request.request_sha256)?;
    body["approve_model_context"] = json!(true);
    body["response"] = serde_json::to_value(response).map_err(|_| ())?;
    let observed = client.json("POST", "/v1/code/endpoint/advance", true, Some(body))?;
    let step = serde_json::from_value::<Step>(observed.clone()).map_err(|_| ())?;
    match &step {
        Step::AwaitingApproval { challenge } => check_challenge(request, challenge)?,
        Step::Terminal { outcome } => {
            let outcome = &outcome.outcome;
            if outcome.schema_version != CONTRACT_SCHEMA_VERSION
                || outcome.run_id != request.run_id
                || outcome.session_id != request.session_id
                || outcome.task_id != request.task.task_id
                || outcome.request_sha256 != request.request_sha256
                || !outcome.state.is_terminal()
                || outcome.answer_evidence.is_some()
            {
                return Err(());
            }
        }
    }
    // A wire response cannot recreate the coordinator's accepted completion
    // claim. Preserve the envelope; never present it as independent verification.
    show(&json!({"step":observed,"verification":"service-observation-only"}))?;
    Ok(step)
}

/// Schedule only fresh explicit decisions against the same process-held owner.
/// Stopping leaves the run held; no retry, cancellation or release is implicit.
fn guided(client: &Client, seconds: &str, run: &str, hash: &str) -> Result<()> {
    binding(run, hash)?;
    workflow::bounded(client, seconds.parse().map_err(|_| ())?, |client| {
        let request = checked_selection(client, run, hash, true)?;
        guided_steps(client, &request).map(|_| ())
    })
}

/// Uses the caller's original deadline and signal latch through later record
/// retention and diff review. A terminal envelope supplies no task-success proof.
pub(super) fn guided_steps(
    client: &Client,
    request: &RuntimeEndpointRunRequest,
) -> Result<RuntimeEndpointOutcome> {
    let mut response = None;
    let mut seen = BTreeSet::new();
    // A conforming owner can request at most its admitted tool budget, then
    // terminate. Bound even an authenticated but inconsistent HTTP peer.
    for step in 0..=request.limits.max_tool_calls {
        client.check_running()?;
        match advance(client, request, response.take(), Some(step))? {
            Step::Terminal { outcome } => return Ok(outcome),
            Step::AwaitingApproval { challenge } => {
                if !seen.insert(challenge.approval_id.clone())
                    || seen.len() as u64 > u64::from(request.limits.max_tool_calls)
                {
                    return Err(());
                }
                let arguments: Value = agentmage_host::runtime_transport::decode_exact(
                    &challenge.presentation.arguments.bytes,
                )
                .ok_or(())?;
                if !arguments.is_object() {
                    return Err(());
                }
                std::io::stdout().flush().map_err(|_| ())?;
                client.confirm(
                        &json!({"request":request,"challenge":challenge,"arguments_json":arguments,
                        "warning":"Approves only this exact displayed operation. Separate model disclosure consent follows; declining it sends no approval. Any declined confirmation stops this client and leaves the service run held. Inspect events after interruption or a lost reply; never resend automatically."}),
                        &format!("allow {}", challenge.challenge_sha256),
                    )?;
                response = Some(RuntimeApprovalResponse {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    run_id: challenge.run_id,
                    approval_id: challenge.approval_id,
                    challenge_sha256: challenge.challenge_sha256,
                    disposition: RuntimeApprovalDisposition::Allow,
                    grant_id: Some(challenge.proposed_grant_id),
                    selection: None,
                });
            }
        }
    }
    Err(())
}

/// Fetches and checks one canonical page. This proves individual event hashes
/// and scope only; a wire page cannot establish whole-stream completeness.
pub(super) fn event_page(
    client: &Client,
    request: &RuntimeEndpointRunRequest,
    after: usize,
    control: bool,
) -> Result<Value> {
    let mut body = binding(request.run_id.as_str(), &request.request_sha256)?;
    body["after"] = json!(after);
    let page = client.json("POST", "/v1/code/endpoint/events", control, Some(body))?;
    let events: Vec<RuntimeEvent> =
        serde_json::from_value(page["events"].clone()).map_err(|_| ())?;
    let total = page["total"].as_u64().ok_or(())?;
    let next = page["next"].as_u64().ok_or(())?;
    if events.len() > 1
        || after as u64 > total
        || Some(next) != (after as u64).checked_add(events.len() as u64)
        || next > total
        || (events.is_empty() && next != total)
        || page["retention"] != "process-only"
    {
        return Err(());
    }
    for event in &events {
        verify_runtime_event(event).map_err(|_| ())?;
        if event.run_id != request.run_id
            || event.session_id != request.session_id
            || event.task_id != request.task.task_id
            || event.policy_id != request.policy_id
            || event.sequence != after as u64
        {
            return Err(());
        }
    }
    Ok(
        json!({"page":page,"verification":"individual-event-hashes-only","index_is_resume_cursor":false}),
    )
}

pub(super) fn execute(client: &Client, command: &str, args: &[String]) -> Result<()> {
    match (command, args) {
        ("endpoint-run", [seconds, run, hash]) => guided(client, seconds, run, hash),
        ("endpoint-original-apply", args) => {
            super::endpoint_transfer::run(client, args, None, false)
        }
        ("endpoint-original-apply-saved", [directory, args @ ..]) => {
            super::endpoint_transfer::run(client, args, Some(Path::new(directory)), false)
        }
        ("endpoint-run-apply-saved", [directory, args @ ..]) => {
            super::endpoint_transfer::run(client, args, Some(Path::new(directory)), true)
        }
        ("endpoint", []) => show(
            &json!({"request":selected(client, true)?,"retention":"process-only","configuration":"fixed-for-service"}),
        ),
        ("endpoint-step", [run, hash]) => {
            if !std::io::stdin().is_terminal() {
                return Err(());
            }
            let request = checked_selection(client, run, hash, true)?;
            advance(client, &request, None, None).map(|_| ())
        }
        ("endpoint-approve" | "endpoint-deny", [run, hash, input]) => {
            if !std::io::stdin().is_terminal() || input.len() > MAX_REQUEST {
                return Err(());
            }
            binding(run, hash)?;
            let challenge: RuntimeApprovalChallenge =
                agentmage_host::runtime_transport::decode_exact(input.as_bytes()).ok_or(())?;
            verify_runtime_approval_challenge(&challenge).map_err(|_| ())?;
            if challenge.run_id.as_str() != run {
                return Err(());
            }
            let request = checked_selection(client, run, hash, true)?;
            check_challenge(&request, &challenge)?;
            let allow = command == "endpoint-approve";
            if allow {
                let arguments: Value = agentmage_host::runtime_transport::decode_exact(
                    &challenge.presentation.arguments.bytes,
                )
                .ok_or(())?;
                if !arguments.is_object() {
                    return Err(());
                }
                confirm(
                    &json!({"request":request,"challenge":challenge,"arguments_json":arguments,
                    "warning":"Approves only this exact operation and its displayed targets, arguments, preimages, limits and expiry. The service rechecks the original pending challenge. Separate model disclosure consent follows; declining it sends no approval. Never resend after a lost reply without inspecting events."}),
                    &format!("allow {}", challenge.challenge_sha256),
                )?;
            }
            let response = RuntimeApprovalResponse {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: challenge.run_id,
                approval_id: challenge.approval_id,
                challenge_sha256: challenge.challenge_sha256,
                disposition: if allow {
                    RuntimeApprovalDisposition::Allow
                } else {
                    RuntimeApprovalDisposition::Deny
                },
                grant_id: allow.then_some(challenge.proposed_grant_id),
                selection: None,
            };
            advance(client, &request, Some(response), None).map(|_| ())
        }
        ("endpoint-events", [run, hash, index]) => {
            let after = index.parse::<usize>().map_err(|_| ())?;
            let request = checked_selection(client, run, hash, true)?;
            show(&event_page(client, &request, after, true)?)
        }
        ("endpoint-record", [run, hash]) => {
            let request = checked_selection(client, run, hash, true)?;
            super::record::export(client, request)
        }
        ("endpoint-record-save", [run, hash, directory]) => {
            if !std::io::stdin().is_terminal() {
                return Err(());
            }
            let request = checked_selection(client, run, hash, true)?;
            workflow::bounded(client, 180, |client| {
                super::record::save_observed(client, request, Path::new(directory))
            })
        }
        ("endpoint-cancel", [run, hash]) => {
            // Cancellation itself discloses nothing to a model and needs no
            // terminal. The original owner checks this exact binding.
            let body = binding(run, hash)?;
            let result = client.json("POST", "/v1/code/endpoint/cancel", true, Some(body))?;
            if result != json!({"cancellation":"requested"}) {
                return Err(());
            }
            show(&result)
        }
        ("endpoint-release", [run, hash]) => {
            if !std::io::stdin().is_terminal() {
                return Err(());
            }
            let request = checked_selection(client, run, hash, true)?;
            confirm(
                &json!({"request":request,"warning":"Drops this run's process-only context, ports and event record after terminal completion or a fault. Export needed events first. The identity stays spent; this cannot resume or reset budgets."}),
                &format!("release {}", request.request_sha256),
            )?;
            let result = client.json(
                "POST",
                "/v1/code/endpoint/release",
                true,
                Some(binding(run, hash)?),
            )?;
            if result != json!({"released":true}) {
                return Err(());
            }
            show(&result)
        }
        _ => {
            eprintln!(
                "Endpoint commands: endpoint | endpoint-run <seconds> <run-id> <request-digest> | endpoint-step <run-id> <request-digest> | endpoint-approve|endpoint-deny <run-id> <request-digest> <challenge-json> | endpoint-events <run-id> <request-digest> <index> | endpoint-record <run-id> <request-digest> | endpoint-record-save <run-id> <request-digest> <private-directory> | endpoint-cancel|endpoint-release <run-id> <request-digest> | endpoint-original-apply <seconds> <run-id> <request-digest> <repository> <worktree> <task> <paths...> | endpoint-original-apply-saved <private-directory> <same-transfer-arguments...>. endpoint-run-apply-saved <private-directory> <same-transfer-arguments...>. Offline: agentmage-code endpoint-replay < record.json"
            );
            Err(())
        }
    }
}
