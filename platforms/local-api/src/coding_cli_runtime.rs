// SPDX-License-Identifier: Apache-2.0
//! Explicit terminal operations over the canonical host's HTTP projection.
use super::*;
use agentmage_host::coding_development_runtime::CodingPythonSourceSelection;
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeRunRequest,
};
use agentmage_kernel_engine::runtime_coordinator::{
    verify_runtime_approval_challenge, verify_runtime_run_request,
};

fn absolute_root(value: &str) -> Result<()> {
    if value.len() > 4096
        || value.contains('\0')
        || !Path::new(value).is_absolute()
        || Path::new(value).components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(());
    }
    Ok(())
}

// Local parsing performs no filesystem access. The API independently admits the
// exact live owner and activation after the person consents to inspection.
fn attach_body(args: &[String]) -> Result<Value> {
    let [state, disposable, workspace, model, scenario, sources @ ..] = args else {
        return Err(());
    };
    for root in [state, disposable, workspace] {
        absolute_root(root)?;
    }
    if !matches!(model.as_str(), "muse" | "gpt-oss")
        || !matches!(
            scenario.as_str(),
            "failed-test-repair" | "python-project" | "python-root-project"
        )
        || (!sources.is_empty() && scenario == "failed-test-repair")
    {
        return Err(());
    }
    let selected = if sources.is_empty() {
        None
    } else {
        Some(
            CodingPythonSourceSelection::new(
                sources
                    .iter()
                    .map(|path| path.split('/').map(str::to_owned).collect())
                    .collect(),
            )
            .map_err(|_| ())?,
        )
    };
    Ok(
        json!({"state_root":state,"disposable_root":disposable,"workspace_root":workspace,
        "model":model,"scenario":scenario,"source_paths":selected.as_ref().map(|s|s.paths()),
        "approve_inspection":true,"approve_handoff":true}),
    )
}

fn attach(client: &Client, args: &[String]) -> Result<()> {
    let mut body = attach_body(args)?;
    if !std::io::stdin().is_terminal() {
        return Err(());
    }
    let held = client.json("GET", "/v1/code/repository/edit", true, None)?;
    if held["version"] != 1 || !held["worktree_root_sha256"].as_str().is_some_and(digest) {
        return Err(());
    }
    body["expected_worktree_root_sha256"] = held["worktree_root_sha256"].clone();
    let encoded = serde_json::to_vec(&body).map_err(|_| ())?;
    if encoded.len() > MAX_REQUEST {
        return Err(());
    }
    let identity = held["worktree_root_sha256"].as_str().ok_or(())?;
    confirm(
        &json!({"request":body,"default_sources":if body["source_paths"].is_null() { "Existing src Python sources for project scenarios; fixed fixture otherwise" } else { "Only the listed existing Python files" },
        "warning":"Inspects the chosen private activation and repository and transfers this service's live created-worktree owner into the native evaluation coordinator. Pending manual decisions are discarded. No model call, transcript retention, tool permission or original-checkout apply is approved. The chat endpoint is not used. Release all runs before recovering the owner; restart loses process-only ownership."}),
        &format!("attach {identity}"),
    )?;
    let result = client.json("POST", "/v1/code/runtime/attach", true, Some(body))?;
    if result["attached"] != true
        || result["live_creation_owner"] != true
        || result["model_and_tools_approved"] != false
        || result["resume"] != false
        || result["retention"] != "process-only"
    {
        return Err(());
    }
    show(&result)
}

fn recover(client: &Client) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        return Err(());
    }
    let state = client.json("GET", "/v1/status", false, None)?;
    if state["version"] != 1 || state["busy"] != false {
        return Err(());
    }
    let selection = match (
        state["runtime_attached"].as_bool(),
        state["runtime_recovery_pending"].as_bool(),
    ) {
        (Some(true), Some(false)) => {
            let selected = client.json("GET", "/v1/code/runtime", true, None)?;
            if selected["live_creation_owner"] != true {
                return Err(());
            }
            selected
        }
        (Some(false), Some(true)) => Value::Null,
        _ => return Err(()),
    };
    confirm(
        &json!({"service_state":state,"runtime_selection":selection,
        "warning":"Returns this service's current process-held worktree owner to manual control. Prepared and active runs must already be explicitly released. A pending recovery retries only the same retained live owner. No run is cancelled or released by this command and no original-checkout write is approved. A fresh diff and separate apply approval are still required."}),
        "recover worktree owner",
    )?;
    let result = client.json(
        "POST",
        "/v1/code/runtime/recover",
        true,
        Some(json!({"approve_recovery":true})),
    )?;
    if result["recovered"] != true
        || result["apply_approved"] != false
        || result["fresh_diff_required"] != true
    {
        return Err(());
    }
    show(&result)
}

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
        ("runtime-attach", args) => attach(client, args),
        ("runtime-recover", []) => recover(client),
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
