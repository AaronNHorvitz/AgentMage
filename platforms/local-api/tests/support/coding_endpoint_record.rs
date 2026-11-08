// SPDX-License-Identifier: Apache-2.0
//! Actual retained HTTP owner -> record -> fresh offline terminal processes.
//! Cancellation is real; tools, permissions and model endpoint are synthetic.

use super::*;
use std::{
    fs,
    process::{Command, Output, Stdio},
};

fn replay(bytes: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
        .arg("endpoint-replay")
        // No address/auth argument or inherited process configuration needed.
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(bytes);
    child.wait_with_output().unwrap()
}

fn reseal(record: &mut Value) {
    let mut previous = "0".repeat(64);
    for value in record["events"].as_array_mut().unwrap() {
        value["previous_event_sha256"] = json!(previous);
        let event: RuntimeEvent = serde_json::from_value(value.clone()).unwrap();
        let event = agentmage_kernel_engine::runtime_event::seal_runtime_event(event).unwrap();
        previous = event.event_sha256.clone();
        *value = json!(event);
    }
}

#[test]
fn terminal_record_replays_identically_offline_after_service_and_auth_are_gone() {
    let executions = Arc::new(AtomicUsize::new(0));
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", model.local_addr().unwrap())).unwrap();
    let api = Api::new(endpoint, executions.clone());
    let root =
        std::env::temp_dir().join(format!("agentmage-endpoint-record-{}", std::process::id()));
    ApiTokens::initialize(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            if self.0.exists() {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
    }
    let _cleanup = Cleanup(root.clone());
    fs::write(root.join("client.token"), CLIENT).unwrap();
    fs::write(root.join("control.token"), CONTROL).unwrap();
    let request = api.request();
    let export = || {
        Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .arg(api.address.to_string())
            .arg(&root)
            .args([
                "endpoint-record",
                request["run_id"].as_str().unwrap(),
                request["request_sha256"].as_str().unwrap(),
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let not_started = export();
    assert!(!not_started.status.success());
    assert!(not_started.stdout.is_empty());
    assert_eq!(api.bind("cancel", &request).0, 200);
    assert_eq!(api.advance(&request, Value::Null, false).0, 200);
    let output = export();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert_eq!(export().stdout, output.stdout);
    let record: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(record["version"], 1);
    assert_eq!(record["request"], request);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(matches!(model.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(api.bind("release", &request).0, 200);
    let released = export();
    assert!(!released.status.success());
    assert!(released.stdout.is_empty());
    drop(api);
    fs::remove_dir_all(&root).unwrap();

    let first = replay(&output.stdout);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.stderr.is_empty());
    assert_eq!(replay(&output.stdout).stdout, first.stdout);
    let observed: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(observed["record"], record);
    assert_eq!(observed["verification"], "canonical-event-chain-only");
    assert_eq!(observed["resumes_execution"], false);
    assert_eq!(observed["authenticity_verified"], false);
    assert_eq!(observed["referenced_payloads_verified"], false);
    assert_eq!(observed["recorded_terminal_state"], "CANCELLED");

    let events = record["events"].as_array().unwrap();
    assert!(events.len() >= 3);
    let mut refused = Vec::new();
    let mut bad = record.clone();
    bad["version"] = json!(2);
    refused.push(bad);
    let mut bad = record.clone();
    bad["extra"] = json!(true);
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"] = json!([]);
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"].as_array_mut().unwrap().pop();
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"].as_array_mut().unwrap().swap(0, 1);
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"].as_array_mut().unwrap().remove(1);
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"]
        .as_array_mut()
        .unwrap()
        .push(events.last().unwrap().clone());
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"][0]["event_sha256"] = json!("f".repeat(64));
    refused.push(bad);
    // A valid resealed request still cannot be substituted for the recorded start.
    let mut bad = record.clone();
    let mut replacement: RuntimeEndpointRunRequest =
        serde_json::from_value(request.clone()).unwrap();
    replacement.task.objective.push_str(" Changed objective.");
    bad["request"] = json!(seal_runtime_endpoint_run_request(replacement).unwrap());
    refused.push(bad);
    // Valid individual hashes and a complete re-chained sequence cannot hide
    // foreign policy/session/task/run binding or a different start digest.
    for field in ["run_id", "session_id", "task_id", "policy_id"] {
        let mut bad = record.clone();
        for event in bad["events"].as_array_mut().unwrap() {
            event[field] = json!("foreign");
        }
        reseal(&mut bad);
        refused.push(bad);
    }
    let mut bad = record.clone();
    bad["events"][0]["kind"]["request_sha256"] = json!("f".repeat(64));
    reseal(&mut bad);
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"][1]["causation_event_id"] = events.last().unwrap()["event_id"].clone();
    reseal(&mut bad);
    refused.push(bad);
    let mut bad = record.clone();
    bad["events"][1]["event_id"] = events[0]["event_id"].clone();
    reseal(&mut bad);
    refused.push(bad);
    for bad in refused {
        let output = replay(&serde_json::to_vec(&bad).unwrap());
        assert!(
            !output.status.success(),
            "unexpected accepted record: {bad}"
        );
        assert!(output.stdout.is_empty());
    }
    // The strict decoder also refuses duplicate members and positional structs.
    let compact = record.to_string();
    for bad in [
        compact.replacen("\"version\":1", "\"version\":1,\"version\":1", 1),
        format!("[1,{},{}]", record["request"], record["events"]),
        format!("{compact}{{}}"),
        " ".repeat(4 * 1024 * 1024 + 1),
    ] {
        let output = replay(bad.as_bytes());
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
