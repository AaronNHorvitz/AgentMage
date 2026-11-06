// SPDX-License-Identifier: Apache-2.0
//! Exact local worktree approvals. No model can approve these routes.

use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepare {
    root: String,
    owned_root: String,
    task_id: String,
    base_commit: String,
    approve_inspection: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareCommand {
    command: agentmage_platform_linux::LinuxCodingCommandSpec,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PrepareValidation {
    command: agentmage_platform_linux::LinuxCodingCommandSpec,
    kind: agentmage_platform_linux::LinuxCodingValidationKind,
    minimum_test_count: u32,
    approve_definition: bool,
    prior_receipt_sha256: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decide {
    preview_sha256: String,
    approve: bool,
}

pub(super) fn handle_repository(
    socket: &mut TcpStream,
    req: &Request,
    state: &Mutex<State>,
    stop: &AtomicBool,
) -> Result<(), ApiError> {
    let route = (req.method.as_str(), req.path.as_str());
    if !matches!(
        route,
        (
            "GET",
            "/v1/code/repository"
                | "/v1/code/repository/command"
                | "/v1/code/repository/validation"
        ) | (
            "POST",
            "/v1/code/repository/prepare"
                | "/v1/code/repository/decide"
                | "/v1/code/repository/command/prepare"
                | "/v1/code/repository/command/decide"
                | "/v1/code/repository/validation/prepare"
                | "/v1/code/repository/validation/decide"
        )
    ) {
        return error(socket, 404);
    }
    if !req.control {
        return error(socket, 403);
    }
    let (mut op, _) = match begin(state, stop, "code.repository") {
        Ok(op) => op,
        Err(status) => return error(socket, status),
    };
    // Native manifest collection and worktree transactions are bounded but not
    // interruptible through the chat token. Never acknowledge a fake cancellation.
    let repository = {
        let mut state = state.lock().map_err(|_| ApiError::Io)?;
        if op.cancel.is_cancelled() || stop.load(Ordering::Relaxed) {
            op.outcome = "code.repository.cancelled";
            drop(state);
            drop(op);
            return response(
                socket,
                409,
                json!({"error":"code.repository.cancelled","effect":"none"}),
            );
        }
        state.active_cancellable = false;
        state.repository.clone()
    };
    let Some(repository) = repository else {
        op.outcome = "code.repository.state-required";
        drop(op);
        return response(
            socket,
            428,
            json!({"error":"code.repository.state-required"}),
        );
    };
    let mut repository = repository.lock().map_err(|_| ApiError::Io)?;
    let result = match route {
        ("GET", "/v1/code/repository") => repository.inspect(),
        ("POST", "/v1/code/repository/prepare") => {
            let Ok(input) = object_json::<Prepare>(&req.body) else {
                return error(socket, 400);
            };
            if !input.approve_inspection {
                return error(socket, 403);
            }
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            repository.prepare(
                Path::new(&input.root),
                Path::new(&input.owned_root),
                &input.task_id,
                &input.base_commit,
                clock_ms()?,
            )
        }
        ("POST", "/v1/code/repository/decide") => {
            let Ok(input) = object_json::<Decide>(&req.body) else {
                return error(socket, 400);
            };
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            repository.decide(&input.preview_sha256, input.approve, clock_ms()?)
        }
        ("GET", "/v1/code/repository/command") => repository.inspect_command(),
        ("POST", "/v1/code/repository/command/prepare") => {
            let Ok(input) = object_json::<PrepareCommand>(&req.body) else {
                return error(socket, 400);
            };
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            repository.prepare_command(input.command, clock_ms()?)
        }
        ("POST", "/v1/code/repository/command/decide") => {
            let Ok(input) = object_json::<Decide>(&req.body) else {
                return error(socket, 400);
            };
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            repository.decide_command(&input.preview_sha256, input.approve, clock_ms()?)
        }
        ("GET", "/v1/code/repository/validation") => repository.inspect_validation(),
        ("POST", "/v1/code/repository/validation/prepare") => {
            let Ok(input) = object_json::<PrepareValidation>(&req.body) else {
                return error(socket, 400);
            };
            if !input.approve_definition {
                return error(socket, 403);
            }
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            repository.prepare_validation(
                input.command,
                input.kind,
                input.minimum_test_count,
                input.prior_receipt_sha256.as_deref(),
                clock_ms()?,
            )
        }
        ("POST", "/v1/code/repository/validation/decide") => {
            let Ok(input) = object_json::<Decide>(&req.body) else {
                return error(socket, 400);
            };
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            repository.decide_validation(&input.preview_sha256, input.approve, clock_ms()?)
        }
        _ => unreachable!(),
    };
    let (status, value) = match result {
        Ok(value) => {
            op.outcome = "completed";
            (200, value)
        }
        Err(e) => {
            op.outcome = e.code();
            // Conservatively never assert an unchanged disk after a decision.
            (
                409,
                json!({"error":e.code(),"effect": if req.path.ends_with("/decide") {"unknown"} else if req.path.contains("/command") || req.path.contains("/validation") {"no-command-dispatch"} else {"no-worktree-dispatch"}}),
            )
        }
    };
    drop(repository);
    drop(op);
    response(socket, status, value)
}
