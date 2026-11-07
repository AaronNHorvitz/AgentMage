// SPDX-License-Identifier: Apache-2.0
//! Explicit in-process handoff of live creation authority, never path adoption.

use super::coding_runtime_api::{RuntimePort, RuntimeSession, Selection};
use super::*;
use agentmage_host::{
    coding_development_activation::CodingDevelopmentActivation,
    coding_development_runtime::{
        CodingDevelopmentModel, CodingDevelopmentRuntimeFactory, CodingDevelopmentScenario,
    },
    coding_live_runtime::LiveCodingRuntimeService,
};
use agentmage_platform_linux::LinuxCodingWorktreeLease;

type Failure = (u16, &'static str);

#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attach {
    state_root: String,
    disposable_root: String,
    workspace_root: String,
    expected_worktree_root_sha256: String,
    model: String,
    scenario: String,
    approve_inspection: bool,
    approve_handoff: bool,
}

#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Recover {
    approve_recovery: bool,
}

fn restore(state: &Mutex<State>, owner: LinuxCodingRepository) -> Result<(), Failure> {
    let mut state = state.lock().map_err(|_| (500, "code.runtime.state"))?;
    if state.repository.is_some() || state.runtime_excludes_manual() {
        return Err((500, "code.runtime.state"));
    }
    state.repository = Some(Arc::new(Mutex::new(owner)));
    Ok(())
}

// The caller holds the service's controlled operation across extraction,
// construction and publication. A concurrent manual operation cannot start.
fn attach(state: &Mutex<State>, stop: &AtomicBool, input: Attach) -> Result<Value, Failure> {
    if !input.approve_handoff || !input.approve_inspection {
        return Err((403, "code.runtime.handoff-consent"));
    }
    let model = CodingDevelopmentModel::parse(&input.model)
        .filter(|_| matches!(input.model.as_str(), "muse" | "gpt-oss"))
        .ok_or((400, "code.runtime.model"))?;
    let scenario = CodingDevelopmentScenario::parse(&input.scenario)
        .filter(|_| {
            matches!(
                input.scenario.as_str(),
                "failed-test-repair" | "python-project"
            )
        })
        .ok_or((400, "code.runtime.scenario"))?;
    let owner = {
        let mut state = state.lock().map_err(|_| (500, "code.runtime.state"))?;
        if stop.load(Ordering::Relaxed) {
            return Err((503, "code.runtime.stopped"));
        }
        if state.runtime_excludes_manual() {
            return Err((409, "code.runtime.attached"));
        }
        let owner = state
            .repository
            .as_ref()
            .ok_or((428, "code.repository.state-required"))?;
        if owner.is_poisoned() {
            return Err((500, "code.runtime.owner-poisoned"));
        }
        let owner = state.repository.take().expect("checked repository");
        match Arc::try_unwrap(owner) {
            Ok(owner) => owner
                .into_inner()
                .map_err(|_| (500, "code.runtime.owner-poisoned"))?,
            Err(owner) => {
                state.repository = Some(owner);
                return Err((409, "code.runtime.busy"));
            }
        }
    };
    let lease = match owner.into_runtime_worktree(&input.expected_worktree_root_sha256) {
        Ok(lease) => lease,
        Err((owner, error)) => {
            restore(state, owner)?;
            return Err((409, error.code()));
        }
    };
    let built = build(&lease, &input, model, scenario, stop);
    match built {
        Ok(runtime) => {
            let mut guard = state.lock().map_err(|_| (500, "code.runtime.state"))?;
            // No other operation can publish a repository/runtime while the
            // controlled operation is held. Stop after construction drops the
            // empty service before returning the exact manual owner.
            if stop.load(Ordering::Relaxed) {
                drop(guard);
                drop(runtime);
                restore_lease(state, lease)?;
                return Err((503, "code.runtime.stopped"));
            }
            guard.pending_change = None;
            guard.runtime = Some(Arc::new(Mutex::new(runtime)));
            Ok(
                json!({"attached":true,"live_creation_owner":true,"resume":false,
                "model_and_tools_approved":false,"retention":"process-only"}),
            )
        }
        Err(error) => {
            restore_lease(state, lease)?;
            Err(error)
        }
    }
}

fn restore_lease(state: &Mutex<State>, lease: LinuxCodingWorktreeLease) -> Result<(), Failure> {
    let mut state = state.lock().map_err(|_| (500, "code.runtime.state"))?;
    if state.repository.is_some() || state.runtime_excludes_manual() {
        return Err((500, "code.runtime.state"));
    }
    state.recovering = Some(lease);
    restore_recovery(&mut state)
}

fn restore_recovery(state: &mut State) -> Result<(), Failure> {
    let owner = LinuxCodingWorktreeLease::recover_repository(&mut state.recovering)
        .map_err(|e| (409, e.code()))?;
    state.repository = Some(Arc::new(Mutex::new(owner)));
    Ok(())
}

fn build(
    lease: &LinuxCodingWorktreeLease,
    input: &Attach,
    model: CodingDevelopmentModel,
    scenario: CodingDevelopmentScenario,
    stop: &AtomicBool,
) -> Result<RuntimeSession, Failure> {
    // Check creation provenance before following any activation path. Markers
    // remain separate explicit development/read consent, never ownership.
    lease
        .check_workspace(Path::new(&input.workspace_root))
        .map_err(|e| (409, e.code()))?;
    let activation = CodingDevelopmentActivation::validate(
        Path::new(&input.state_root),
        Path::new(&input.disposable_root),
        Path::new(&input.workspace_root),
    )
    .map_err(|e| (409, e.code()))?;
    lease
        .check_runtime_state_root(activation.state_root())
        .map_err(|e| (409, e.code()))?;
    if stop.load(Ordering::Relaxed) {
        return Err((503, "code.runtime.stopped"));
    }
    let selection = Selection {
        scenario: input.scenario.clone(),
        profile_id: model.profile_id().to_owned(),
        expected_entry_sha256: activation.marker_sha256().to_owned(),
        workspace_id: format!("coding-development-{}", &activation.marker_sha256()[..24]),
        workspace_root: activation
            .workspace_root()
            .to_str()
            .ok_or((400, "code.runtime.scope"))?
            .to_owned(),
        resume: false,
    };
    let mut factory = CodingDevelopmentRuntimeFactory::new(activation, scenario, model, false)
        .map_err(|e| (409, e.code()))?;
    factory
        .attach_owned_worktree(lease)
        .map_err(|e| (409, e.code()))?;
    Ok(RuntimeSession {
        port: RuntimePort::Owned(Box::new(LiveCodingRuntimeService::new(factory))),
        selection,
        owner: Some(lease.clone()),
    })
}

fn recover(state: &Mutex<State>, approve: bool) -> Result<Value, Failure> {
    if !approve {
        return Err((403, "code.runtime.recovery-consent"));
    }
    let runtime = {
        let mut state = state.lock().map_err(|_| (500, "code.runtime.state"))?;
        if state.repository.is_some() {
            return Err((409, "code.runtime.not-owned"));
        }
        if state.recovering.is_some() {
            restore_recovery(&mut state)?;
            return Ok(recovered());
        }
        let runtime = state
            .runtime
            .as_ref()
            .ok_or((428, "code.runtime.activation-required"))?;
        {
            let runtime = runtime.try_lock().map_err(|_| (409, "code.runtime.busy"))?;
            if runtime.owner.is_none() {
                return Err((409, "code.runtime.not-owned"));
            }
            if !runtime.port.recoverable() {
                return Err((409, "code.runtime.release-required"));
            }
        }
        let runtime = state.runtime.take().expect("checked runtime");
        match Arc::try_unwrap(runtime) {
            Ok(runtime) => runtime
                .into_inner()
                .map_err(|_| (500, "code.runtime.state"))?,
            Err(runtime) => {
                state.runtime = Some(runtime);
                return Err((409, "code.runtime.busy"));
            }
        }
    };
    let RuntimeSession { port, owner, .. } = runtime;
    // Dropping the empty service releases the factory's lease. Active and
    // prepared runs must have been explicitly released; no implicit job stop.
    drop(port);
    restore_lease(state, owner.expect("checked owned runtime"))?;
    Ok(recovered())
}

fn recovered() -> Value {
    json!({"recovered":true,"apply_approved":false,"fresh_diff_required":true})
}

pub(super) fn handle_owned(
    socket: &mut TcpStream,
    req: &Request,
    state: &Mutex<State>,
    stop: &AtomicBool,
) -> Result<(), ApiError> {
    if !req.control {
        return error(socket, 403);
    }
    if req.method != "POST" {
        return error(socket, 404);
    }
    let (mut op, _) = match begin_controlled(state, stop, "code.runtime.ownership") {
        Ok(op) => op,
        Err(status) => return error(socket, status),
    };
    // Metadata/ownership transitions are bounded non-interruptible steps, as
    // with repository previews. Never advertise cancellability for them.
    state.lock().map_err(|_| ApiError::Io)?.active_cancellable = false;
    let result = match req.path.as_str() {
        "/v1/code/runtime/attach" => {
            agentmage_host::runtime_transport::decode_exact::<Attach>(&req.body)
                .ok_or((400, "code.runtime.input"))
                .and_then(|input| attach(state, stop, input))
        }
        "/v1/code/runtime/recover" => {
            agentmage_host::runtime_transport::decode_exact::<Recover>(&req.body)
                .ok_or((400, "code.runtime.input"))
                .and_then(|input| recover(state, input.approve_recovery))
        }
        _ => Err((404, "code.runtime.route")),
    };
    let (status, body) = match result {
        Ok(body) => {
            op.outcome = "transferred";
            (200, body)
        }
        Err((status, code)) => {
            op.outcome = code;
            (status, json!({"error":code}))
        }
    };
    drop(op);
    response(socket, status, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::AtomicU64;

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-owned-api-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
        fn state(&self) -> Mutex<State> {
            Mutex::new(State {
                repository: Some(Arc::new(Mutex::new(
                    LinuxCodingRepository::open(
                        &self.0,
                        b"synthetic-owner-api-control-secret",
                        clock_ms().unwrap(),
                    )
                    .unwrap(),
                ))),
                ..State::default()
            })
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn input() -> Attach {
        Attach {
            state_root: "/not-opened/runtime".into(),
            disposable_root: "/not-opened/disposable".into(),
            workspace_root: "/not-opened/disposable/worktree".into(),
            expected_worktree_root_sha256: "a".repeat(64),
            model: "gpt-oss".into(),
            scenario: "python-project".into(),
            approve_inspection: true,
            approve_handoff: true,
        }
    }
    #[test]
    fn attach_without_live_creation_returns_exact_owner_and_never_reads_activation() {
        let f = Fixture::new();
        let state = f.state();
        for _ in 0..2 {
            assert_eq!(
                attach(&state, &AtomicBool::new(false), input()),
                Err((409, "code.runtime.worktree-required"))
            );
            let guard = state.lock().unwrap();
            assert!(guard.runtime.is_none());
            let owner = guard.repository.as_ref().unwrap().lock().unwrap();
            assert!(owner.inspect().unwrap()["pending"].is_null());
        }
    }
    #[test]
    fn consent_closed_selections_busy_stop_and_poison_preserve_repository() {
        let f = Fixture::new();
        let state = f.state();
        let stop = AtomicBool::new(false);
        let original = state.lock().unwrap().repository.clone().unwrap();
        for (field, expected) in [
            ("inspection", (403, "code.runtime.handoff-consent")),
            ("handoff", (403, "code.runtime.handoff-consent")),
            ("model", (400, "code.runtime.model")),
            ("scenario", (400, "code.runtime.scenario")),
        ] {
            let mut i = input();
            match field {
                "inspection" => i.approve_inspection = false,
                "handoff" => i.approve_handoff = false,
                "model" => i.model = "scripted".into(),
                _ => i.scenario = "arbitrary-command".into(),
            }
            assert_eq!(attach(&state, &stop, i), Err(expected));
        }
        assert_eq!(
            attach(&state, &stop, input()),
            Err((409, "code.runtime.busy"))
        );
        assert!(Arc::ptr_eq(
            &original,
            state.lock().unwrap().repository.as_ref().unwrap()
        ));
        stop.store(true, Ordering::Relaxed);
        assert_eq!(
            attach(&state, &stop, input()),
            Err((503, "code.runtime.stopped"))
        );
        stop.store(false, Ordering::Relaxed);
        let _ = std::panic::catch_unwind(|| {
            let _held = original.lock().unwrap();
            panic!("synthetic poison");
        });
        assert_eq!(
            attach(&state, &stop, input()),
            Err((500, "code.runtime.owner-poisoned"))
        );
        assert!(
            state
                .lock()
                .unwrap()
                .repository
                .as_ref()
                .unwrap()
                .is_poisoned()
        );
    }
    #[test]
    fn recovery_cannot_adopt_a_manual_owner_or_an_empty_state() {
        let f = Fixture::new();
        let state = f.state();
        assert_eq!(
            recover(&state, false),
            Err((403, "code.runtime.recovery-consent"))
        );
        assert_eq!(recover(&state, true), Err((409, "code.runtime.not-owned")));
        let empty = Mutex::new(State::default());
        assert_eq!(
            recover(&empty, true),
            Err((428, "code.runtime.activation-required"))
        );
    }
    #[test]
    fn real_http_handoff_requires_control_exact_input_and_records_content_free_refusals() {
        let f = Fixture::new();
        let state = f.state();
        let tokens = ApiTokens::new("1".repeat(64), "2".repeat(64)).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = AtomicBool::new(false);
        let call = |route: &str, control: bool, body: Value| {
            thread::scope(|scope| {
                let server = scope.spawn(|| {
                    let (mut socket, _) = listener.accept().unwrap();
                    handle(&mut socket, address, &tokens, &state, &stop).unwrap();
                });
                let mut socket = TcpStream::connect(address).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let body = serde_json::to_string(&body).unwrap();
                let token = if control {
                    "2".repeat(64)
                } else {
                    "1".repeat(64)
                };
                write!(&mut socket, "POST {route} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut chunk = [0; 4096];
                    match socket.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                        Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                        Err(e) => panic!("{e}"),
                    }
                }
                server.join().unwrap();
                let end = bytes.windows(4).position(|s| s == b"\r\n\r\n").unwrap();
                let status: u16 = std::str::from_utf8(&bytes[..end])
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .parse()
                    .unwrap();
                (
                    status,
                    serde_json::from_slice::<Value>(&bytes[end + 4..]).unwrap(),
                )
            })
        };
        let start = Instant::now();
        let body = serde_json::to_value(input()).unwrap();
        assert_eq!(call("/v1/code/runtime/attach", false, body.clone()).0, 403);
        let mut extra = body.clone();
        extra["resume"] = json!(true);
        assert_eq!(
            call("/v1/code/runtime/attach", true, extra).1["error"],
            "code.runtime.input"
        );
        assert_eq!(
            call("/v1/code/runtime/attach", true, body).1["error"],
            "code.runtime.worktree-required"
        );
        assert_eq!(
            call(
                "/v1/code/runtime/recover",
                true,
                json!({"approve_recovery":true})
            )
            .1["error"],
            "code.runtime.not-owned"
        );
        assert!(state.lock().unwrap().repository.is_some());
        let activity = serde_json::to_string(&state.lock().unwrap().activity).unwrap();
        assert!(!activity.contains("not-opened"));
        assert!(activity.contains("code.runtime.worktree-required"));
        println!(
            "owned_api_four_refusal_exchanges_us={}",
            start.elapsed().as_micros()
        );
    }
}
