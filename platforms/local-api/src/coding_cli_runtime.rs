// SPDX-License-Identifier: Apache-2.0
//! Explicit terminal operations over the canonical host's HTTP projection.
use super::*;
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeRunRequest,
};
use agentmage_kernel_engine::runtime_coordinator::{
    verify_runtime_approval_challenge, verify_runtime_run_request,
};

fn binding(run: &str, digest_value: &str) -> Result<Value> {
    if run.is_empty() || run.len() > 128 || !digest(digest_value) {
        return Err(());
    }
    Ok(json!({"run_id":run,"request_sha256":digest_value}))
}
fn poll(client: &Client, run: &str, hash: &str, response: Option<Value>) -> Result<Value> {
    let mut body = binding(run, hash)?;
    body["after_event_cursor"] = Value::Null;
    body["response"] = response.unwrap_or(Value::Null);
    client.json("POST", "/v1/code/runtime/advance", true, Some(body))
}
fn check_step(step: &Value, run: &str, hash: &str) -> Result<()> {
    if step["run_id"] != run || step["request_sha256"] != hash {
        return Err(());
    }
    Ok(())
}

pub(super) fn execute(client: &Client, command: &str, args: &[String]) -> Result<()> {
    match (command, args) {
        ("runtime-run", [seconds, task]) => super::driver::run(client, seconds, task),
        ("runtime", []) => show(&client.json("GET", "/v1/code/runtime", true, None)?),
        ("runtime-prepare", [prompt]) => {
            let selection = client.json("GET", "/v1/code/runtime", true, None)?;
            let entry = selection["expected_entry_sha256"]
                .as_str()
                .filter(|s| digest(s))
                .ok_or(())?;
            confirm(
                &json!({"selection":selection,"prompt":prompt,
                "warning":"Inspects the startup-selected disposable repository through the native host. No model or tool approval is granted. The service already inspected it during explicit startup."}),
                &format!("inspect {entry}"),
            )?;
            show(&client.json(
                "POST",
                "/v1/code/runtime/prepare",
                true,
                Some(json!({"prompt":prompt,"approve_inspection":true})),
            )?)
        }
        ("runtime-start", [prepared]) => {
            if prepared.len() > MAX_REQUEST {
                return Err(());
            }
            let request: RuntimeRunRequest =
                agentmage_host::runtime_transport::decode_exact(prepared.as_bytes()).ok_or(())?;
            verify_runtime_run_request(&request).map_err(|_| ())?;
            confirm(
                &json!({"request":request,"warning":"Sends the task, repository context and subsequent tool observations to this exact native evaluation model through the existing coordinator. Every consequential tool still needs exact approval. Not the chat endpoint; no automatic session authority or transcript retention."}),
                &format!("start {}", request.request_sha256),
            )?;
            let step = client.json(
                "POST",
                "/v1/code/runtime/start",
                true,
                Some(json!({"request":request,"approve_model_context":true})),
            )?;
            check_step(&step, request.run_id.as_str(), &request.request_sha256)?;
            show(&step)
        }
        ("runtime-step", [run, hash]) => {
            let step = poll(client, run, hash, None)?;
            check_step(&step, run, hash)?;
            show(&step)
        }
        ("runtime-approve" | "runtime-deny", [run, hash]) => {
            let step = poll(client, run, hash, None)?;
            check_step(&step, run, hash)?;
            let challenge: RuntimeApprovalChallenge =
                serde_json::from_value(step["approval"].clone()).map_err(|_| ())?;
            verify_runtime_approval_challenge(&challenge).map_err(|_| ())?;
            if challenge.run_id.as_str() != run {
                return Err(());
            }
            let allow = command == "runtime-approve";
            if allow {
                let arguments: Value =
                    serde_json::from_slice(&challenge.presentation.arguments.bytes)
                        .map_err(|_| ())?;
                confirm(
                    &json!({"challenge":challenge,"arguments_json":arguments,"warning":"Approves only this exact host-bound operation, targets, preimages, arguments, limits and expiry. A lost reply is uncertain: inspect the run, never resend an approval automatically."}),
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
            let step = poll(
                client,
                run,
                hash,
                Some(serde_json::to_value(response).map_err(|_| ())?),
            )?;
            check_step(&step, run, hash)?;
            show(&step)
        }
        ("runtime-job" | "runtime-declarations" | "runtime-release", [run, hash]) => {
            let route = match command {
                "runtime-job" => "/v1/code/runtime/job",
                "runtime-declarations" => "/v1/code/runtime/declarations",
                _ => "/v1/code/runtime/release",
            };
            show(&client.json("POST", route, true, Some(binding(run, hash)?))?)
        }
        ("runtime-control", [run, hash, action])
            if matches!(action.as_str(), "cancel" | "suspend" | "resume") =>
        {
            let status = client.json(
                "POST",
                "/v1/code/runtime/job",
                true,
                Some(binding(run, hash)?),
            )?;
            if status["schema_version"] != 1
                || status["run_id"] != run.as_str()
                || status["request_sha256"] != hash.as_str()
                || status["job"]["job_id"] != run.as_str()
                || !status["job"]["head_sha256"].as_str().is_some_and(digest)
            {
                return Err(());
            }
            let revision = status["job"]["revision"].as_u64().ok_or(())?;
            if action == "resume" {
                confirm(
                    &json!({"status":status,"action":action,"warning":"Continues through the host's existing safe checkpoint. Prior approvals are not replayed."}),
                    &format!("resume {run} {revision}"),
                )?;
            }
            let mut random = [0; 16];
            if rustix::rand::getrandom(&mut random, rustix::rand::GetRandomFlags::empty())
                != Ok(random.len())
            {
                return Err(());
            }
            let request_id = random
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let mut body = binding(run, hash)?;
            body["request"] = json!({"schema_version":1,"job_id":run,"request_id":format!("terminal-{request_id}"),"action":action,"observed_revision":revision});
            show(&client.json("POST", "/v1/code/runtime/control", true, Some(body))?)
        }
        _ => Err(()),
    }
}
