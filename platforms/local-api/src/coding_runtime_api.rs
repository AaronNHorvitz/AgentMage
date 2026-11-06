// SPDX-License-Identifier: Apache-2.0
//! Transport-only projection of the existing live coding host. No second loop.

use super::*;
use agentmage_host::{
    coding_development_activation::CodingDevelopmentActivation,
    coding_development_runtime::CodingDevelopmentModel,
    runtime_ipc::LinuxRuntimeIpcClient,
    runtime_transport::{RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort},
};
use agentmage_kernel_contracts::{
    RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEventCursor, RuntimeRunId,
    RuntimeRunRequest,
};
use agentmage_kernel_engine::job_control::JobControlRequest;
use agentmage_platform_linux::LinuxDevelopmentHostProcess;

/// Explicit disposable native-host activation. This is development-only, with
/// the existing exact candidate profiles and unchanged native trust checks.
/// Starting it authorizes repository inspection and private canonical state;
/// model work and consequential tools still need their separate approvals.
pub struct CodingRuntimeDevelopment<'a> {
    /// Existing private canonical host state, separate from disposable data.
    pub state_root: &'a Path,
    /// Existing private disposable root containing the marked Git workspace.
    pub disposable_root: &'a Path,
    /// Existing marked development workspace, never an unowned active checkout.
    pub workspace_root: &'a Path,
    /// Exact existing candidate label: `muse` or `gpt-oss`. No download/fallback.
    pub model: &'a str,
    /// Request reconstruction through the host's existing durable checkpoint path.
    pub resume: bool,
}

#[derive(Clone)]
struct Selection {
    profile_id: String,
    expected_entry_sha256: String,
    workspace_id: String,
    workspace_root: String,
    resume: bool,
}

pub(super) struct RuntimeSession {
    // Production construction below uses ONLY an authenticated IPC session.
    // The private trait object lets unit tests observe forwarding without a
    // production fake factory, source fixture or native trust bypass.
    port: Box<dyn RuntimeTransportPort + Send>,
    selection: Selection,
}

/// Serves the existing native coding coordinator through authenticated HTTP.
/// The verified sibling host owns model selection, workers, grants, receipts,
/// event history, validation/retries and persistence. This adapter starts no
/// model itself, accepts no endpoint/command from model output, and adds no
/// approval or effect authority. Every runtime route requires the control token.
/// The manual chat/workspace/repository routes are unavailable in this service.
/// On exit its authenticated channel closes and its owned child is reaped.
pub fn serve_coding_runtime_development(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    input: CodingRuntimeDevelopment<'_>,
) -> Result<(), ApiError> {
    if stop.load(Ordering::Relaxed)
        || !listener
            .local_addr()
            .map_err(|_| ApiError::Io)?
            .ip()
            .is_loopback()
        || !matches!(input.model, "muse" | "gpt-oss")
    {
        return Err(ApiError::Configuration);
    }
    let model = CodingDevelopmentModel::parse(input.model).ok_or(ApiError::Configuration)?;
    let activation = CodingDevelopmentActivation::validate(
        input.state_root,
        input.disposable_root,
        input.workspace_root,
    )
    .map_err(|_| ApiError::Configuration)?;
    let selection = Selection {
        profile_id: model.profile_id().to_owned(),
        expected_entry_sha256: activation.marker_sha256().to_owned(),
        workspace_id: format!("coding-development-{}", &activation.marker_sha256()[..24]),
        workspace_root: activation
            .workspace_root()
            .to_str()
            .ok_or(ApiError::Configuration)?
            .to_owned(),
        resume: input.resume,
    };
    let mut child = LinuxDevelopmentHostProcess::launch(
        activation.state_root(),
        activation.disposable_root(),
        activation.workspace_root(),
        "failed-test-repair",
        input.model,
        input.resume,
    )
    .map_err(|_| ApiError::Configuration)?;
    let result = (|| {
        let envelope = child
            .read_launch_envelope_cancellable(&stop)
            .map_err(|_| ApiError::Configuration)?;
        let channel = envelope
            .connect_development()
            .map_err(|_| ApiError::Configuration)?
            .with_frame_timeout(Duration::from_secs(30))
            .map_err(|_| ApiError::Configuration)?;
        let runtime = RuntimeSession {
            port: Box::new(LinuxRuntimeIpcClient::new(channel)),
            selection,
        };
        serve_state(
            listener,
            tokens,
            stop,
            State {
                runtime: Some(Arc::new(Mutex::new(runtime))),
                ..State::default()
            },
        )
    })();
    // Applies to errors and orderly stop alike. Never leave a launched sibling
    // or confuse a dropped HTTP connection with acknowledged cancellation.
    child.terminate_and_reap().map_err(|_| ApiError::Io)?;
    result
}

#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PrepareRun {
    prompt: String,
    approve_inspection: bool,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    request: RuntimeRunRequest,
    approve_model_context: bool,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    run_id: RuntimeRunId,
    request_sha256: String,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Advance {
    run_id: RuntimeRunId,
    request_sha256: String,
    after_event_cursor: Option<RuntimeEventCursor>,
    response: Option<RuntimeApprovalResponse>,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    run_id: RuntimeRunId,
    request_sha256: String,
    request: JobControlRequest,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    run_id: RuntimeRunId,
    request_sha256: String,
    reference: RuntimeArtifactRef,
    offset: u64,
    maximum_bytes: u32,
}

type RuntimeResult = Result<Value, (u16, &'static str)>;
fn input<T: serde::de::DeserializeOwned + serde::Serialize>(
    bytes: &[u8],
) -> Result<T, (u16, &'static str)> {
    agentmage_host::runtime_transport::decode_exact(bytes).ok_or((400, "code.runtime.input"))
}
fn forwarded<T: serde::Serialize>(result: Result<T, RuntimeTransportError>) -> RuntimeResult {
    let result = result.map_err(|e| (409, e.code()))?;
    serde_json::to_value(result).map_err(|_| (500, "code.runtime.encoding"))
}

impl RuntimeSession {
    fn dispatch(&mut self, method: &str, path: &str, body: &[u8]) -> RuntimeResult {
        match (method, path) {
            ("GET", "/v1/code/runtime") => Ok(json!({
                "version":1,"runtime":"existing-live-native-coding-host",
                "activation":"coding-development-v1","profile_id":self.selection.profile_id,
                "workspace_id":self.selection.workspace_id,"workspace_root":self.selection.workspace_root,
                "expected_entry_sha256":self.selection.expected_entry_sha256,"resume":self.selection.resume,
                "requires_control":true,"preauthorization":false,
                "chat_endpoint_used":false,"model_qualification":"evaluation-only"
            })),
            ("POST", "/v1/code/runtime/prepare") => {
                let request: PrepareRun = input(body)?;
                if !request.approve_inspection {
                    return Err((403, "code.runtime.inspection-required"));
                }
                // The HTTP caller never supplies a root/profile/entry digest,
                // preauthorization, recipe, probe or recording-consent flag.
                forwarded(self.port.prepare(RuntimePrepareInput {
                    resume: self.selection.resume,
                    record_session: false,
                    slow_subscriber_probe: false,
                    preauthorization: None,
                    engineering_session_id: None,
                    profile_id: self.selection.profile_id.clone(),
                    expected_entry_sha256: self.selection.expected_entry_sha256.clone(),
                    workspace_id: self.selection.workspace_id.clone(),
                    workspace_root: self.selection.workspace_root.clone(),
                    prompt: request.prompt,
                    recipe: None,
                }))
            }
            ("POST", "/v1/code/runtime/start") => {
                let request: Start = input(body)?;
                if !request.approve_model_context {
                    return Err((403, "code.runtime.disclosure-required"));
                }
                // The host consumes only its own exact previously prepared request.
                forwarded(self.port.start(request.request))
            }
            ("POST", "/v1/code/runtime/advance") => {
                let request: Advance = input(body)?;
                forwarded(self.port.advance(
                    &request.run_id,
                    &request.request_sha256,
                    request.after_event_cursor.as_ref(),
                    request.response.as_ref(),
                ))
            }
            ("POST", "/v1/code/runtime/control") => {
                let request: Control = input(body)?;
                // IPC derives the job-control scope from its authenticated peer.
                // Do not call cancel directly or invent a caller-supplied scope.
                forwarded(self.port.control_job(
                    &request.run_id,
                    &request.request_sha256,
                    &request.request,
                ))
            }
            ("POST", "/v1/code/runtime/job") => {
                let request: Binding = input(body)?;
                forwarded(
                    self.port
                        .job_status(&request.run_id, &request.request_sha256),
                )
            }
            ("POST", "/v1/code/runtime/declarations") => {
                let request: Binding = input(body)?;
                forwarded(
                    self.port
                        .run_declarations(&request.run_id, &request.request_sha256),
                )
            }
            ("POST", "/v1/code/runtime/artifact") => {
                let request: Artifact = input(body)?;
                if !(1..=65536).contains(&request.maximum_bytes) {
                    return Err((400, "code.runtime.page-limit"));
                }
                forwarded(self.port.read_artifact_page(
                    &request.run_id,
                    &request.request_sha256,
                    &request.reference,
                    request.offset,
                    request.maximum_bytes,
                ))
            }
            ("POST", "/v1/code/runtime/release") => {
                let request: Binding = input(body)?;
                forwarded(self.port.release(&request.run_id, &request.request_sha256))?;
                Ok(json!({"released":true}))
            }
            _ => Err((404, "code.runtime.route")),
        }
    }
}

pub(super) fn handle_runtime(
    socket: &mut TcpStream,
    req: &Request,
    state: &Mutex<State>,
    stop: &AtomicBool,
) -> Result<(), ApiError> {
    if !req.control {
        return error(socket, 403);
    }
    if stop.load(Ordering::Relaxed) {
        return error(socket, 503);
    }
    let runtime = state.lock().map_err(|_| ApiError::Io)?.runtime.clone();
    let Some(runtime) = runtime else {
        return response(
            socket,
            428,
            json!({"error":"code.runtime.activation-required"}),
        );
    };
    // The IPC host performs bounded polling; inference/tools run on its existing
    // worker. Never hold the service-state mutex while waiting on IPC.
    let result = match runtime.try_lock() {
        Ok(mut runtime) => runtime.dispatch(&req.method, &req.path, &req.body),
        Err(std::sync::TryLockError::WouldBlock) => Err((409, "code.runtime.busy")),
        Err(std::sync::TryLockError::Poisoned(_)) => Err((500, "code.runtime.state")),
    };
    let (status, value, outcome) = match result {
        Ok(value) => (200, value, "forwarded"),
        Err((status, code)) => (status, json!({"error":code}), code),
    };
    state
        .lock()
        .map_err(|_| ApiError::Io)?
        .record("code.runtime", outcome);
    let bytes = Zeroizing::new(serde_json::to_vec(&value).map_err(|_| ApiError::Io)?);
    if bytes.len() > 4 * 1024 * 1024 {
        return response(
            socket,
            413,
            json!({"error":"code.runtime.reply-limit","effect":"consult-host-record"}),
        );
    }
    headers(socket, status, "application/json", Some(bytes.len()))?;
    write(socket, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_host::runtime_transport::{RuntimeJobControl, RuntimeTransportStep};
    use agentmage_kernel_contracts::{CancellationId, SessionId};
    use agentmage_kernel_engine::job_control::JobControlAction;

    #[derive(Default)]
    struct Observed {
        calls: Vec<Value>,
    }
    struct Peer(Arc<Mutex<Observed>>);
    impl RuntimeTransportPort for Peer {
        fn prepare(
            &mut self,
            input: RuntimePrepareInput,
        ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
            self.0.lock().unwrap().calls.push(json!({"prepare":input}));
            Err(RuntimeTransportError::RequestDenied)
        }
        fn start(
            &mut self,
            request: RuntimeRunRequest,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            self.0.lock().unwrap().calls.push(json!({"start":request}));
            Err(RuntimeTransportError::RequestDenied)
        }
        fn advance(
            &mut self,
            run: &RuntimeRunId,
            digest: &str,
            cursor: Option<&RuntimeEventCursor>,
            response: Option<&RuntimeApprovalResponse>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            self.0
                .lock()
                .unwrap()
                .calls
                .push(json!({"advance":run,"digest":digest,"cursor":cursor,"response":response}));
            Err(RuntimeTransportError::ApprovalDenied)
        }
        fn cancel(
            &mut self,
            _: &RuntimeRunId,
            _: &str,
            _: CancellationId,
            _: Option<&RuntimeEventCursor>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            panic!("HTTP must never bypass the job owner with direct cancellation")
        }
        fn control_job(
            &mut self,
            run: &RuntimeRunId,
            digest: &str,
            request: &JobControlRequest,
        ) -> Result<RuntimeJobControl, RuntimeTransportError> {
            self.0
                .lock()
                .unwrap()
                .calls
                .push(json!({"control":run,"digest":digest,"request":request}));
            Err(RuntimeTransportError::JobControlUnavailable)
        }
        fn revoke_session_preauthorization(
            &mut self,
            _: &SessionId,
            _: &str,
        ) -> Result<(), RuntimeTransportError> {
            panic!("No preauthorization route")
        }
        fn release(
            &mut self,
            run: &RuntimeRunId,
            digest: &str,
        ) -> Result<(), RuntimeTransportError> {
            self.0
                .lock()
                .unwrap()
                .calls
                .push(json!({"release":run,"digest":digest}));
            Ok(())
        }
    }
    fn fixture() -> (RuntimeSession, Arc<Mutex<Observed>>) {
        let observed = Arc::new(Mutex::new(Observed::default()));
        (
            RuntimeSession {
                port: Box::new(Peer(observed.clone())),
                selection: Selection {
                    profile_id: "configured-evaluation-profile".into(),
                    expected_entry_sha256: "a".repeat(64),
                    workspace_id: "owned-workspace".into(),
                    workspace_root: "/synthetic/workspace".into(),
                    resume: false,
                },
            },
            observed,
        )
    }
    fn dispatch(runtime: &mut RuntimeSession, route: &str, body: Value) -> RuntimeResult {
        runtime.dispatch(
            "POST",
            &format!("/v1/code/runtime/{route}"),
            &serde_json::to_vec(&body).unwrap(),
        )
    }
    #[test]
    fn startup_refuses_scripted_and_stopped_services_before_any_host_launch() {
        for (model, stopped) in [("scripted", false), ("muse", true)] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let result = serve_coding_runtime_development(
                listener,
                ApiTokens::new("1".repeat(64), "2".repeat(64)).unwrap(),
                Arc::new(AtomicBool::new(stopped)),
                CodingRuntimeDevelopment {
                    state_root: Path::new("/not-an-activation"),
                    disposable_root: Path::new("/not-an-activation"),
                    workspace_root: Path::new("/not-an-activation"),
                    model,
                    resume: false,
                },
            );
            assert_eq!(result, Err(ApiError::Configuration));
        }
    }
    #[test]
    fn start_requires_disclosure_and_preserves_the_complete_exact_host_request() {
        let (mut runtime, observed) = fixture();
        let request: RuntimeRunRequest = agentmage_host::runtime_transport::decode_exact(
            include_bytes!("../../../fixtures/runtime-producer/v5/run-request.json"),
        )
        .unwrap();
        assert_eq!(
            dispatch(
                &mut runtime,
                "start",
                json!({"request":request,"approve_model_context":false})
            ),
            Err((403, "code.runtime.disclosure-required"))
        );
        assert!(observed.lock().unwrap().calls.is_empty());
        assert_eq!(
            dispatch(
                &mut runtime,
                "start",
                json!({"request":request,"approve_model_context":true})
            ),
            Err((409, "host.runtime.request_denied"))
        );
        let calls = &observed.lock().unwrap().calls;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["start"], json!(request));
    }
    #[test]
    fn prepare_is_bound_to_startup_selection_without_new_authority() {
        let (mut runtime, observed) = fixture();
        assert_eq!(
            dispatch(
                &mut runtime,
                "prepare",
                json!({"prompt":"Fix add","approve_inspection":false})
            ),
            Err((403, "code.runtime.inspection-required"))
        );
        assert!(observed.lock().unwrap().calls.is_empty());
        assert_eq!(
            dispatch(
                &mut runtime,
                "prepare",
                json!({"prompt":"Fix add","approve_inspection":true})
            ),
            Err((409, "host.runtime.request_denied"))
        );
        let calls = &observed.lock().unwrap().calls;
        let input = &calls[0]["prepare"];
        assert_eq!(input["profile_id"], "configured-evaluation-profile");
        assert_eq!(input["workspace_root"], "/synthetic/workspace");
        assert_eq!(input["expected_entry_sha256"], "a".repeat(64));
        assert_eq!(input["workspace_id"], "owned-workspace");
        assert_eq!(input["prompt"], "Fix add");
        assert_eq!(input["preauthorization"], Value::Null);
        assert_eq!(input["record_session"], false);
        assert_eq!(input["slow_subscriber_probe"], false);
        assert!(input.get("recipe").is_none());
    }
    #[test]
    fn exact_decoder_rejects_nested_arrays_duplicates_and_extra_authority() {
        let (mut runtime, observed) = fixture();
        for body in [
            json!({"prompt":"Fix","approve_inspection":true,"preauthorization":{}}),
            json!({"prompt":"Fix","approve_inspection":true,"workspace_root":"/foreign"}),
            json!(["Fix", true]),
        ] {
            assert_eq!(
                dispatch(&mut runtime, "prepare", body),
                Err((400, "code.runtime.input"))
            );
        }
        assert_eq!(
            runtime.dispatch(
                "POST",
                "/v1/code/runtime/prepare",
                br#"{"prompt":"one","prompt":"two","approve_inspection":true}"#
            ),
            Err((400, "code.runtime.input"))
        );
        let request = json!({"schema_version":1,"job_id":"job","request_id":"control","action":"cancel","observed_revision":3});
        let mut body = json!({"run_id":"run","request_sha256":"b".repeat(64),"request":request});
        body["request"] = json!([1, "job", "control", "cancel", 3]);
        assert_eq!(
            dispatch(&mut runtime, "control", body),
            Err((400, "code.runtime.input"))
        );
        assert!(observed.lock().unwrap().calls.is_empty());
    }
    #[test]
    fn exact_approval_is_forwarded_once_and_host_refusal_remains_a_refusal() {
        let (mut runtime, observed) = fixture();
        let approval = json!({"schema_version":1,"run_id":"run","approval_id":"approval","challenge_sha256":"c".repeat(64),"disposition":"deny","grant_id":null});
        let body = json!({"run_id":"run","request_sha256":"b".repeat(64),"after_event_cursor":null,"response":approval});
        assert_eq!(
            dispatch(&mut runtime, "advance", body),
            Err((409, "host.runtime.approval_denied"))
        );
        let calls = &observed.lock().unwrap().calls;
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["response"], approval);
        assert_eq!(calls[0]["digest"], "b".repeat(64));
    }
    #[test]
    fn cancellation_and_resume_go_through_existing_scoped_job_control() {
        let (mut runtime, observed) = fixture();
        for action in [
            JobControlAction::Cancel,
            JobControlAction::Suspend,
            JobControlAction::Resume,
        ] {
            let request = JobControlRequest {
                schema_version: 1,
                job_id: "job".into(),
                request_id: "control".into(),
                action,
                observed_revision: 3,
            };
            assert_eq!(
                dispatch(
                    &mut runtime,
                    "control",
                    json!({"run_id":"run","request_sha256":"b".repeat(64),"request":request})
                ),
                Err((409, "host.runtime.job_control_unavailable"))
            );
        }
        assert_eq!(observed.lock().unwrap().calls.len(), 3);
        assert_eq!(
            dispatch(
                &mut runtime,
                "release",
                json!({"run_id":"run","request_sha256":"b".repeat(64)})
            ),
            Ok(json!({"released":true}))
        );
        assert_eq!(observed.lock().unwrap().calls.len(), 4);
    }

    #[test]
    fn real_http_requires_control_refuses_mixing_and_keeps_status_available() {
        let (runtime, observed) = fixture();
        let state = Mutex::new(State {
            runtime: Some(Arc::new(Mutex::new(runtime))),
            ..State::default()
        });
        let tokens = ApiTokens::new("1".repeat(64), "2".repeat(64)).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = AtomicBool::new(false);
        let call = |method: &str, route: &str, control: bool, body: &str| {
            thread::scope(|scope| {
                let task = scope.spawn(|| {
                    let (mut socket, _) = listener.accept().unwrap();
                    handle(&mut socket, address, &tokens, &state, &stop).unwrap();
                });
                let mut socket = TcpStream::connect(address).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let token = if control {
                    "2".repeat(64)
                } else {
                    "1".repeat(64)
                };
                socket.write_all(format!("{method} {route} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).unwrap();
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
                task.join().unwrap();
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
        assert_eq!(call("GET", "/v1/code/runtime", false, "").0, 403);
        let selection = call("GET", "/v1/code/runtime", true, "");
        assert_eq!(selection.0, 200);
        assert_eq!(selection.1["chat_endpoint_used"], false);
        assert_eq!(
            call(
                "POST",
                "/v1/code/runtime/prepare",
                false,
                r#"{"prompt":"Fix","approve_inspection":true}"#
            )
            .0,
            403
        );
        assert_eq!(
            call(
                "POST",
                "/v1/code/runtime/prepare",
                true,
                r#"{"prompt":"Fix","approve_inspection":true}"#
            )
            .0,
            409
        );
        assert_eq!(observed.lock().unwrap().calls.len(), 1);
        assert_eq!(
            call("POST", "/v1/code/workspace/apply", true, "{}").1["error"],
            "code.runtime.dedicated-service"
        );
        assert_eq!(
            call("POST", "/v1/cancel", true, "").1["error"],
            "code.runtime.dedicated-service"
        );
        let runtime = state.lock().unwrap().runtime.clone().unwrap();
        let _held = runtime.lock().unwrap();
        assert_eq!(
            call("GET", "/v1/code/runtime", true, "").1["error"],
            "code.runtime.busy"
        );
        assert_eq!(
            call("GET", "/v1/status", false, "").1["runtime_attached"],
            true
        );
        let activity = call("GET", "/v1/activity", false, "");
        assert_eq!(activity.0, 200);
        assert!(!serde_json::to_string(&activity).unwrap().contains("Fix"));
        stop.store(true, Ordering::Relaxed);
        // The unchanged HTTP parser notices stop before route dispatch.
        assert_eq!(call("GET", "/v1/code/runtime", true, "").0, 408);
    }
}
