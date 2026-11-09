// SPDX-License-Identifier: Apache-2.0
//! Fresh terminal processes continue one actual retained HTTP coordinator.
//! Endpoint bytes, tools and verifier are synthetic; no native/model score.

use super::*;
use std::{
    fs,
    process::{Command, Stdio},
};

fn terminal(api: &Api, auth: &std::path::Path, request: &Value, decline_tool: bool) -> Value {
    const PTY: &str = r#"
import errno, fcntl, json, os, pty, select, signal, subprocess, sys, termios, time
master, slave = pty.openpty()
def session():
    os.setsid(); fcntl.ioctl(0, termios.TIOCSCTTY, 0)
p = subprocess.Popen(sys.argv[2:], stdin=slave, stdout=slave, stderr=slave, preexec_fn=session)
os.close(slave)
text = b''; prompts = []; started = time.monotonic()
try:
    while True:
        assert time.monotonic() - started < 12, 'terminal timeout'
        if not select.select([master], [], [], .02)[0]:
            if p.poll() is not None: break
            continue
        try: chunk = os.read(master, 65536)
        except OSError as e:
            if e.errno == errno.EIO: break
            raise
        if not chunk: break
        text += chunk
        if text.count(b'anything else declines:') > len(prompts):
            answer = text.rsplit(b'Type exactly ', 1)[1].split(b' to approve;')[0]
            prompts.append(answer.decode())
            if sys.argv[1] == 'decline-tool' and answer.startswith(b'allow '): answer = b'no'
            os.write(master, answer + b'\n')
    status = p.wait(timeout=2)
    assert b'\x1b' not in text
    print(json.dumps({'status':status,'prompts':prompts,'transcript':text.decode()}))
finally:
    if p.poll() is None: os.killpg(p.pid, signal.SIGKILL); p.wait()
    os.close(master)
"#;
    let output = Command::new("python3")
        .args([
            "-B",
            "-c",
            PTY,
            if decline_tool {
                "decline-tool"
            } else {
                "approve"
            },
            env!("CARGO_BIN_EXE_agentmage-code"),
            &api.address.to_string(),
            auth.to_str().unwrap(),
            "endpoint-run",
            "10",
            request["run_id"].as_str().unwrap(),
            request["request_sha256"].as_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn guided_terminal_restarts_continue_same_owner_without_repeating_tool_or_resetting_counts() {
    let (upstream, endpoint) = upstream();
    let executions = Arc::new(AtomicUsize::new(0));
    let model = thread::spawn(move || {
        let mut first = accept(&upstream);
        assert_eq!(read_upstream(&mut first)["model"], "fixture-model");
        respond(&mut first, &tool());
        drop(first);
        let mut second = accept(&upstream);
        let body = read_upstream(&mut second);
        assert!(body.to_string().contains("original retained boundary"));
        respond(&mut second, "unverified synthetic completion");
        drop(second);
        upstream
    });
    let api = Api::new(endpoint, executions.clone());
    let root =
        std::env::temp_dir().join(format!("agentmage-guided-endpoint-{}", std::process::id()));
    ApiTokens::initialize(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(root.clone());
    fs::write(root.join("client.token"), CLIENT).unwrap();
    fs::write(root.join("control.token"), CONTROL).unwrap();
    let request = api.request();
    let first = terminal(&api, &root, &request, true);
    assert_ne!(first["status"], 0, "{first}");
    assert_eq!(first["prompts"].as_array().unwrap().len(), 2);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    let (_, before) = api.bind("events", &request);
    assert_eq!(
        before["events"][0]["kind"]["request_sha256"],
        request["request_sha256"]
    );
    // New executable, same process-held run and original pending challenge.
    let second = terminal(&api, &root, &request, false);
    assert_eq!(second["status"], 0, "{second}");
    let prompts = second["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 3);
    assert_eq!(prompts[1], first["prompts"][1]);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let text = second["transcript"].as_str().unwrap();
    assert!(text.contains("FAILED") && text.contains("service-observation-only"));
    assert!(!text.contains("verified-outcome"));
    let (code, outcome) = api.advance(&request, Value::Null, true);
    assert_eq!(code, 200);
    assert_eq!(outcome["outcome"]["outcome"]["state"], "FAILED");
    assert_eq!(outcome["outcome"]["outcome"]["model_call_count"], 2);
    assert_eq!(outcome["outcome"]["outcome"]["tool_call_count"], 1);
    // A third fresh terminal observes completion, and never repeats the tool.
    let third = terminal(&api, &root, &request, false);
    assert_eq!(third["status"], 0, "{third}");
    assert_eq!(third["prompts"].as_array().unwrap().len(), 1);
    assert_eq!(api.advance(&request, Value::Null, true), (200, outcome));
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
        .args([
            &api.address.to_string(),
            root.to_str().unwrap(),
            "endpoint-record",
            request["run_id"].as_str().unwrap(),
            request["request_sha256"].as_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let record: Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
    for event in record["events"].as_array().unwrap() {
        sequence
            .push(&serde_json::from_value::<RuntimeEvent>(event.clone()).unwrap())
            .unwrap();
    }
    assert!(sequence.is_terminal());
    assert_eq!(api.bind("release", &request).0, 200);
    assert_eq!(
        model.join().unwrap().accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
