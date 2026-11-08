// SPDX-License-Identifier: Apache-2.0
//! Actual MCP binary and retained HTTP owner; synthetic tool/grant ports only.

use super::*;
use std::{
    fs,
    os::unix::fs::symlink,
    process::{Command, Output, Stdio},
    sync::atomic::AtomicU64,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Delegate(std::path::PathBuf);
impl Delegate {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-endpoint-mcp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        ApiTokens::initialize(&root).unwrap();
        fs::write(root.join("client.token"), CLIENT).unwrap();
        fs::remove_file(root.join("control.token")).unwrap();
        // A control-token load would refuse this deliberate invalid neighbor.
        symlink("not-a-control-credential", root.join("control.token")).unwrap();
        Self(root)
    }
    fn run(&self, address: SocketAddr, r: &Value, calls: &[Value], consent: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_agentmage-mcp"))
            .args([
                address.to_string(),
                self.0.to_str().unwrap().into(),
                r["run_id"].as_str().unwrap().into(),
                r["request_sha256"].as_str().unwrap().into(),
                consent.into(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut messages = vec![
            json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"synthetic-observer","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        ];
        messages.extend_from_slice(calls);
        // A refused startup can close stdin before this small write.
        let _ = child.stdin.take().unwrap().write_all(
            messages
                .iter()
                .map(|m| format!("{m}\n"))
                .collect::<String>()
                .as_bytes(),
        );
        child.wait_with_output().unwrap()
    }
}
impl Drop for Delegate {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn call(id: u64, name: &str, args: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})
}
fn replies(output: Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn content(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
fn fixture(delegate: bool) -> (Api, TcpListener, Arc<AtomicUsize>) {
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", model.local_addr().unwrap())).unwrap();
    let executions = Arc::new(AtomicUsize::new(0));
    (
        Api::start(endpoint, executions.clone(), delegate),
        model,
        executions,
    )
}
fn no_effects(model: &TcpListener, executions: &AtomicUsize) {
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(matches!(model.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn endpoint_observation_delegation_is_opt_in_and_never_delegates_controls() {
    for delegated in [false, true] {
        let (api, model, executions) = fixture(delegated);
        let r = api.request();
        let binding = json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"]});
        assert_eq!(
            api.call("GET", "/v1/code/endpoint/request", CLIENT, None).0,
            if delegated { 200 } else { 403 }
        );
        let mut events = binding.clone();
        events["after"] = json!(0);
        assert_eq!(
            api.call("POST", "/v1/code/endpoint/events", CLIENT, Some(events))
                .0,
            if delegated { 200 } else { 403 }
        );
        assert_eq!(
            api.call("GET", "/v1/code/endpoint/request", "", None).0,
            401
        );
        for (method, route, body) in [
            (
                "POST",
                "advance",
                json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"approve_model_context":true,"response":null}),
            ),
            ("POST", "cancel", binding.clone()),
            ("POST", "release", binding.clone()),
            ("POST", "request", json!({})),
            ("GET", "events", json!({})),
            ("POST", "approve", json!({"allow":true})),
        ] {
            assert_eq!(
                api.call(
                    method,
                    &format!("/v1/code/endpoint/{route}"),
                    CLIENT,
                    (method != "GET").then_some(body)
                )
                .0,
                403
            );
        }
        assert_eq!(
            api.call("POST", "/v1/settings", CLIENT, Some(json!({}))).0,
            403
        );
        assert_eq!(api.bind("events", &r).1["total"], 0);
        // A delegated cancel was refused: no consent still refuses advancement.
        assert_eq!(api.advance(&r, Value::Null, false).0, 403);
        no_effects(&model, &executions);
    }
}

#[test]
fn endpoint_mcp_binary_reads_only_exact_pinned_run_without_control_credential() {
    let (api, model, executions) = fixture(true);
    let r = api.request();
    let delegate = Delegate::new();
    let calls = vec![
        json!({"jsonrpc":"2.0","id":"list","method":"tools/list"}),
        call(1, "endpoint_request", json!({})),
        call(2, "endpoint_events", json!({"after":0})),
        call(3, "endpoint_advance", json!({"approve_model_context":true})),
        call(4, "endpoint_cancel", json!({})),
        call(5, "endpoint_release", json!({})),
        call(6, "workspace", json!({})),
        call(7, "endpoint_request", json!({"run_id":"other"})),
        call(8, "endpoint_events", json!({"after":-1})),
        call(9, "endpoint_events", json!({"after":4294967296_u64})),
        call(10, "endpoint_events", json!({"after":0,"approve":true})),
    ];
    let observed = replies(delegate.run(api.address, &r, &calls, "--share-selected-endpoint"));
    let catalog = observed[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(catalog.len(), 2);
    assert_eq!(catalog[0]["name"], "endpoint_request");
    assert_eq!(catalog[1]["name"], "endpoint_events");
    assert!(
        catalog
            .iter()
            .all(|t| t["annotations"]["readOnlyHint"] == true
                && t["annotations"]["openWorldHint"] == false)
    );
    assert_eq!(content(&observed[2])["request"], r);
    assert_eq!(
        content(&observed[2])["verification"],
        "service-observation-only"
    );
    assert_eq!(content(&observed[3])["page"]["total"], 0);
    for reply in &observed[4..] {
        assert!(
            reply["error"].is_object() || reply["result"]["isError"] == true,
            "{reply}"
        );
    }
    for consent in ["--share-selected-workspace", "--no-consent"] {
        let output = delegate.run(api.address, &r, &calls, consent);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    let mut foreign = r.clone();
    foreign["run_id"] = json!("foreign-run");
    for foreign in [foreign, {
        let mut v = r.clone();
        v["request_sha256"] = json!("0".repeat(64));
        v
    }] {
        let output = replies(delegate.run(
            api.address,
            &foreign,
            &[
                call(1, "endpoint_request", json!({})),
                call(2, "endpoint_events", json!({"after":0})),
            ],
            "--share-selected-endpoint",
        ));
        for reply in &output[1..] {
            assert_eq!(reply["result"]["isError"], true);
            assert!(
                !reply
                    .to_string()
                    .contains(r["task"]["objective"].as_str().unwrap())
            );
        }
    }
    let (other, other_model, other_executions) = fixture(true);
    let refused = replies(delegate.run(
        other.address,
        &r,
        &[call(1, "endpoint_request", json!({}))],
        "--share-selected-endpoint",
    ));
    assert_eq!(refused[1]["result"]["isError"], true);
    no_effects(&other_model, &other_executions);
    assert_eq!(api.bind("events", &r).1["total"], 0);
    no_effects(&model, &executions);
}

#[test]
fn endpoint_mcp_binary_exports_original_events_and_refuses_after_release_or_without_delegation() {
    let (api, model, executions) = fixture(true);
    let r = api.request();
    let delegate = Delegate::new();
    assert_eq!(api.bind("cancel", &r).0, 200);
    assert_eq!(api.advance(&r, Value::Null, false).0, 200);
    let first = api.bind("events", &r).1;
    let total = first["total"].as_u64().unwrap();
    assert!(total > 0);
    let calls = (0..total)
        .map(|after| call(after, "endpoint_events", json!({"after":after})))
        .collect::<Vec<_>>();
    let observed = replies(delegate.run(api.address, &r, &calls, "--share-selected-endpoint"));
    assert_eq!(observed.len(), total as usize + 1);
    for (after, reply) in observed[1..].iter().enumerate() {
        let got = content(reply);
        assert_eq!(got["verification"], "individual-event-hashes-only");
        assert_eq!(got["index_is_resume_cursor"], false);
        let expected = api.call("POST", "/v1/code/endpoint/events", CONTROL, Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"after":after}))).1;
        assert_eq!(got["page"], expected);
    }
    assert_eq!(api.bind("release", &r).0, 200);
    let observed = replies(delegate.run(
        api.address,
        &r,
        &[call(1, "endpoint_events", json!({"after":0}))],
        "--share-selected-endpoint",
    ));
    assert_eq!(observed[1]["result"]["isError"], true);
    no_effects(&model, &executions);
    let (api, model, executions) = fixture(false);
    let r = api.request();
    let observed = replies(delegate.run(
        api.address,
        &r,
        &[
            call(1, "endpoint_request", json!({})),
            call(2, "endpoint_events", json!({"after":0})),
        ],
        "--share-selected-endpoint",
    ));
    assert!(
        observed[1..]
            .iter()
            .all(|reply| reply["result"]["isError"] == true)
    );
    no_effects(&model, &executions);
}
