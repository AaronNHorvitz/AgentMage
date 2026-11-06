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
// Wire decoding belongs here; the native owner needs no serialization dependency.
#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TestOutputFormat {
    #[default]
    AgentMageJsonV1,
    RustLibtestTextV1,
    PythonUnittestTextV1,
}
impl From<TestOutputFormat> for agentmage_platform_linux::LinuxCodingTestFormat {
    fn from(format: TestOutputFormat) -> Self {
        match format {
            TestOutputFormat::AgentMageJsonV1 => Self::AgentMageJsonV1,
            TestOutputFormat::RustLibtestTextV1 => Self::RustLibtestTextV1,
            TestOutputFormat::PythonUnittestTextV1 => Self::PythonUnittestTextV1,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PrepareValidation {
    command: agentmage_platform_linux::LinuxCodingCommandSpec,
    kind: agentmage_platform_linux::LinuxCodingValidationKind,
    minimum_test_count: u32,
    #[serde(default)]
    output_format: TestOutputFormat,
    approve_definition: bool,
    prior_receipt_sha256: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decide {
    preview_sha256: String,
    approve: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PrepareEdit {
    paths: Vec<String>,
    expected_sources: Vec<ExpectedSource>,
    task: String,
    max_message_bytes: usize,
    answer: String,
    approve_read: bool,
}

fn prepare_edit(
    repository: &mut LinuxCodingRepository,
    body: &[u8],
    now: u64,
) -> Result<Value, (u16, &'static str)> {
    let input = object_json::<PrepareEdit>(body).map_err(|_| (400, "code.edit.input"))?;
    let shape: Value = serde_json::from_slice(body).map_err(|_| (400, "code.edit.input"))?;
    if !shape["expected_sources"]
        .as_array()
        .is_some_and(|v| v.iter().all(Value::is_object))
    {
        return Err((400, "code.edit.input"));
    }
    if !input.approve_read {
        return Err((403, "code.edit.read-required"));
    }
    // Ownership is checked before reading; no caller root can adopt a worktree.
    let scope = repository
        .select_created_sources(&input.paths)
        .map_err(|e| (409, e.code()))?;
    let sources = scope
        .read(&input.paths)
        .map_err(|e| (409, e.code()))?
        .into_iter()
        .map(|s| CodingSource {
            path: s.path,
            content: s.content,
        })
        .collect::<Vec<_>>();
    let changes = checked_changes(
        &input.task,
        &sources,
        input.max_message_bytes,
        &input.expected_sources,
        &input.answer,
    )?;
    let replacements = changes
        .iter()
        .map(|c| LinuxCodingReplacement {
            path: &c.path,
            before_sha256: &c.before_sha256,
            content: &c.content,
        })
        .collect::<Vec<_>>();
    repository
        .prepare_edit(scope, &replacements, &input.task, now)
        .map_err(|e| (409, e.code()))
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
                | "/v1/code/repository/edit"
        ) | (
            "POST",
            "/v1/code/repository/prepare"
                | "/v1/code/repository/decide"
                | "/v1/code/repository/command/prepare"
                | "/v1/code/repository/command/decide"
                | "/v1/code/repository/validation/prepare"
                | "/v1/code/repository/validation/decide"
                | "/v1/code/repository/edit/prepare"
                | "/v1/code/repository/edit/decide"
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
        ("GET", "/v1/code/repository/edit") => repository.inspect_edit(),
        ("POST", "/v1/code/repository/edit/prepare") => {
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            match prepare_edit(&mut repository, &req.body, clock_ms()?) {
                Ok(value) => Ok(value),
                Err((status, code)) => {
                    op.outcome = code;
                    drop(repository);
                    drop(op);
                    return response(
                        socket,
                        status,
                        json!({"error":code,"effect":"no-write-dispatch"}),
                    );
                }
            }
        }
        ("POST", "/v1/code/repository/edit/decide") => {
            let Ok(input) = object_json::<Decide>(&req.body) else {
                return error(socket, 400);
            };
            if stop.load(Ordering::Relaxed) {
                return error(socket, 503);
            }
            repository.decide_edit(&input.preview_sha256, input.approve, clock_ms()?)
        }
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
                input.output_format.into(),
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
                json!({"error":e.code(),"effect": if req.path.ends_with("/decide") {"unknown"} else if req.path.contains("/edit") {"no-write-dispatch"} else if req.path.contains("/command") || req.path.contains("/validation") {"no-command-dispatch"} else {"no-worktree-dispatch"}}),
            )
        }
    };
    drop(repository);
    drop(op);
    response(socket, status, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_and_workspace_edits_share_complete_snapshot_and_strict_edit_checks() {
        let sources = vec![
            CodingSource {
                path: "calc.py".into(),
                content: "return a - b\n".into(),
            },
            CodingSource {
                path: "unchanged.py".into(),
                content: "preserve\n".into(),
            },
        ];
        let context = prepare_coding_proposal("Fix add", &sources, 4096).unwrap();
        let expected = context
            .outlines()
            .iter()
            .map(|s| ExpectedSource {
                path: s.path.clone(),
                sha256: s.sha256.clone(),
            })
            .collect::<Vec<_>>();
        let answer = r#"{"edits":[{"path":"calc.py","old":"a - b","new":"a + b"}]}"#;
        let checked = checked_changes("Fix add", &sources, 4096, &expected, answer).unwrap();
        assert_eq!(checked.len(), 1);
        assert_eq!(checked[0].content, "return a + b\n");
        assert_eq!(
            checked_changes("Fix add", &sources, 4096, &expected[..1], answer)
                .unwrap_err()
                .1,
            "code.proposal.stale"
        );
        let duplicate = vec![
            ExpectedSource {
                path: expected[0].path.clone(),
                sha256: expected[0].sha256.clone(),
            },
            ExpectedSource {
                path: expected[0].path.clone(),
                sha256: expected[0].sha256.clone(),
            },
        ];
        assert_eq!(
            checked_changes("Fix add", &sources, 4096, &duplicate, answer)
                .unwrap_err()
                .1,
            "code.proposal.stale"
        );
        for bad in [
            "partial JSON",
            r#"{"edits":[{"path":"../outside","old":"a - b","new":"wrong"}]}"#,
            r#"{"edits":[{"path":"calc.py","old":"missing","new":"wrong"}]}"#,
            r#"{"edits":[{"path":"calc.py","old":"a - b","new":"one"},{"path":"calc.py","old":"a - b","new":"two"}]}"#,
        ] {
            assert!(checked_changes("Fix add", &sources, 4096, &expected, bad).is_err());
        }
    }
}
