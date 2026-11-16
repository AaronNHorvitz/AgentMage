// SPDX-License-Identifier: MIT OR Apache-2.0
//! Actual HTTP/coordinator release, synthetic extraction refusal; no native run.
use super::*;
use agentmage_host::runtime_tools::read_only_runtime_registry;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    runtime_coordinator::{runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request},
    runtime_loop::*,
};
use agentmage_platform_linux_inference::{
    local_chat::LocalChatEndpoint,
    local_chat_transport::{LocalChatApproval, LocalChatTransport, TransportLimits},
};
use std::{io, sync::atomic::AtomicUsize};

const CLIENT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONTROL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Clock;
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(1000)
    }
}
struct Boundary(Arc<AtomicUsize>);
impl RuntimeToolBoundary for Boundary {
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("no tool evaluation expected")
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("no tool resolution expected")
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        panic!("no tool execution expected")
    }
}
struct Verifier(VerifierId);
impl RuntimeVerifierPort for Verifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("no verification expected")
    }
}
fn compose(
    run_id: RuntimeRunId,
    session_id: SessionId,
    endpoint: LocalChatEndpoint,
    attempts: Arc<AtomicUsize>,
) -> CodingEndpointHttpRun<Boundary, Verifier, Clock> {
    let transport = Arc::new(
        LocalChatTransport::new(
            endpoint.clone(),
            None,
            TransportLimits {
                total: Duration::from_secs(2),
                idle: Duration::from_secs(1),
            },
        )
        .unwrap(),
    );
    let registry = read_only_runtime_registry().unwrap();
    // Reuse only first-party common contract fields; no native profile reaches
    // the endpoint request, model port or coordinator.
    let mut value: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/runtime-producer/v5/run-request.json"
    ))
    .unwrap();
    value.as_object_mut().unwrap().remove("model_profile");
    value["run_id"] = json!(run_id);
    value["session_id"] = json!(session_id);
    value["task"]["session_id"] = json!(session_id);
    value["task"]["constraints"] = json!(["Synthetic HTTP ownership fixture"]);
    value["mode"] = json!("ephemeral_read_only");
    value["event_cursor"] = Value::Null;
    value["model_selection"] = json!(transport.model_selection("fixture-model"));
    value["max_output_tokens"] = json!(128);
    value["context_budget"] = json!({"max_messages":16,"max_content_bytes":32768,"max_serialized_messages_bytes":65536,"max_request_body_bytes":131072});
    let catalog = ToolCatalogId::from_raw("synthetic-http-read-catalog");
    let visible = runtime_tool_references(&registry).unwrap();
    value["tool_catalog_id"] = json!(catalog);
    value["tool_catalog_sha256"] = json!(runtime_tool_catalog_sha256(&catalog, &visible).unwrap());
    value["visible_tools"] = json!(visible);
    value["limits"]["max_output_bytes"] = json!(4 * 1024 * 1024);
    let request =
        seal_runtime_endpoint_run_request(serde_json::from_value(value).unwrap()).unwrap();
    CodingEndpointHttpRun {
        request,
        connection: CodingEndpointConnection::Shared {
            transport,
            approval: Arc::new(LocalChatApproval::for_endpoint(endpoint)),
        },
        registry,
        sources: vec![],
        source_budget: 2048,
        include_usage: true,
        tool_boundary: Boundary(attempts),
        verifier: Verifier(VerifierId::from_raw("synthetic-verifier")),
        clock: Clock,
    }
}
struct Service {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Result<Option<Boundary>, ApiError>>>,
}
impl Drop for Service {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        assert!(
            self.worker
                .take()
                .unwrap()
                .join()
                .unwrap()
                .unwrap()
                .is_none()
        );
    }
}
impl Service {
    fn call(&self, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
        let mut socket = TcpStream::connect(self.address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let body = if body.is_null() {
            String::new()
        } else {
            body.to_string()
        };
        write!(socket, "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", self.address, body.len()).unwrap();
        let mut reply = Vec::new();
        socket.take(65536).read_to_end(&mut reply).unwrap();
        let split = reply
            .windows(4)
            .position(|p| p == b"\r\n\r\n")
            .expect("complete HTTP error response required");
        let status = std::str::from_utf8(&reply[..split])
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (status, serde_json::from_slice(&reply[split + 4..]).unwrap())
    }
}
fn refuse_configuration(tool: Boundary) -> Result<LinuxCodingRepository, ApiError> {
    tool.0.fetch_add(1, Ordering::SeqCst);
    Err(ApiError::Configuration)
}
fn refuse_io(tool: Boundary) -> Result<LinuxCodingRepository, ApiError> {
    tool.0.fetch_add(1, Ordering::SeqCst);
    Err(ApiError::Io)
}

#[test]
fn extraction_refusal_responds_after_actual_release_without_restoring_the_run() {
    for convert in [
        refuse_configuration as fn(Boundary) -> Result<LinuxCodingRepository, ApiError>,
        refuse_io,
    ] {
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        upstream.set_nonblocking(true).unwrap();
        let endpoint =
            LocalChatEndpoint::parse(&format!("http://{}/v1", upstream.local_addr().unwrap()))
                .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let attempts = Arc::new(AtomicUsize::new(0));
        let worker_attempts = attempts.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            serve_admitted_endpoint_inner(
                listener,
                ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                worker_stop,
                CodingEndpointObservationAccess::ControlOnly,
                |id, session, runs: &mut CodingEndpointRuns<'static, Boundary, Verifier, Clock>| {
                    let input = compose(id, session, endpoint, worker_attempts);
                    let request = input.request.clone();
                    runs.admit(CodingEndpointAdmission {
                        request: input.request,
                        connection: input.connection,
                        registry: input.registry,
                        sources: &input.sources,
                        source_budget: input.source_budget,
                        include_usage: input.include_usage,
                        tool_boundary: input.tool_boundary,
                        verifier: input.verifier,
                        clock: input.clock,
                    })
                    .map_err(|_| ApiError::Configuration)?;
                    Ok(request)
                },
                TerminalToolDisposition::RepositoryReview(convert),
            )
        });
        let api = Service {
            address,
            stop,
            worker: Some(worker),
        };
        let (code, selection) = api.call("GET", "/v1/code/endpoint/request", CONTROL, Value::Null);
        assert_eq!(code, 200);
        let r = &selection["request"];
        let binding = json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"]});
        let release = "/v1/code/endpoint/release";
        // Neither an observer, a foreign binding nor an unstarted run reaches extraction.
        assert_eq!(api.call("POST", release, CLIENT, binding.clone()).0, 403);
        let mut foreign = binding.clone();
        foreign["request_sha256"] = json!("0".repeat(64));
        assert_eq!(api.call("POST", release, CONTROL, foreign).0, 409);
        assert_eq!(api.call("POST", release, CONTROL, binding.clone()).0, 409);
        assert_eq!(attempts.load(Ordering::SeqCst), 0);
        assert_eq!(
            api.call("POST", "/v1/code/endpoint/cancel", CONTROL, binding.clone())
                .0,
            200
        );
        let mut advance = binding.clone();
        advance["approve_model_context"] = json!(false);
        advance["response"] = Value::Null;
        let terminal = api.call(
            "POST",
            "/v1/code/endpoint/advance",
            CONTROL,
            advance.clone(),
        );
        assert_eq!(terminal.0, 200);
        assert_eq!(terminal.1["outcome"]["outcome"]["state"], "CANCELLED");
        let mut events = binding.clone();
        events["after"] = json!(0);
        let prefix = api.call("POST", "/v1/code/endpoint/events", CONTROL, events.clone());
        assert_eq!(prefix.0, 200);
        assert!(prefix.1["total"].as_u64().unwrap() > 0);
        let (code, failure) = api.call("POST", release, CONTROL, binding.clone());
        assert_eq!(code, 409);
        assert_eq!(
            failure["error"],
            "code.endpoint.original-review-unavailable"
        );
        assert_eq!(failure["released"], true);
        assert_eq!(failure["original_review_available"], false);
        assert!(
            failure["message"]
                .as_str()
                .unwrap()
                .contains("do not repeat")
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        for (route, body) in [
            (release, binding),
            ("/v1/code/endpoint/events", events),
            ("/v1/code/endpoint/advance", advance),
        ] {
            let refused = api.call("POST", route, CONTROL, body);
            assert_eq!(refused, (409, json!({"error":"code.endpoint.not-held"})));
        }
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(
            api.call("GET", "/v1/code/repository/apply", CONTROL, Value::Null)
                .0,
            428
        );
        assert_eq!(
            api.call(
                "POST",
                "/v1/code/repository/apply/prepare",
                CONTROL,
                json!({})
            )
            .0,
            428
        );
        // The listener remains usable, but no run or owner is recreated.
        assert_eq!(
            api.call("GET", "/v1/code/endpoint/request", CONTROL, Value::Null)
                .0,
            200
        );
        drop(api);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == io::ErrorKind::WouldBlock));
    }
}
