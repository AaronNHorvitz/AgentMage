// SPDX-License-Identifier: Apache-2.0
//! Actual terminal consumer against a synthetic HTTP peer; not a native run.
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::runtime_coordinator::seal_runtime_approval_challenge;
use agentmage_local_api::ApiTokens;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-runtime-terminal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        ApiTokens::initialize(&root.join("auth")).unwrap();
        Self { root }
    }
    fn cli(&self, address: SocketAddr, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .arg(address.to_string())
            .arg(self.root.join("auth"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
    fn tty(&self, address: SocketAddr, args: &[&str], answer: &str) -> Value {
        let output = Command::new("python3")
            .args([
                "-c",
                PTY,
                answer,
                env!("CARGO_BIN_EXE_agentmage-code"),
                &address.to_string(),
                self.root.join("auth").to_str().unwrap(),
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn peer(&self, replies: Vec<Value>) -> Peer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let token = fs::read_to_string(self.root.join("auth/control.token")).unwrap();
        let worker = {
            let stop = stop.clone();
            thread::spawn(move || {
                let mut observed = Vec::new();
                let deadline = Instant::now() + Duration::from_secs(15);
                while !stop.load(Ordering::Relaxed) {
                    assert!(Instant::now() < deadline, "terminal peer timed out");
                    let (mut socket, _) = match listener.accept() {
                        Ok(v) => v,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(e) => panic!("{e}"),
                    };
                    socket
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let mut header = Vec::new();
                    while !header.ends_with(b"\r\n\r\n") {
                        let mut byte = [0];
                        socket.read_exact(&mut byte).unwrap();
                        header.push(byte[0]);
                        assert!(header.len() < 8192);
                    }
                    let header = String::from_utf8(header).unwrap();
                    assert!(header.contains(&format!("\r\nAuthorization: Bearer {token}\r\n")));
                    let length: usize = header
                        .lines()
                        .find_map(|s| s.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).unwrap();
                    observed.push(json!({"route":header.lines().next().unwrap(),"body":if body.is_empty(){Value::Null}else{serde_json::from_slice(&body).unwrap()}}));
                    let reply = serde_json::to_vec(
                        replies
                            .get(observed.len() - 1)
                            .unwrap_or(&json!({"unexpected":true})),
                    )
                    .unwrap();
                    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",reply.len()).as_bytes()).unwrap();
                    socket.write_all(&reply).unwrap();
                }
                observed
            })
        };
        Peer {
            address,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
struct Peer {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Vec<Value>>>,
}
impl Peer {
    fn finish(mut self) -> Vec<Value> {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn challenge() -> RuntimeApprovalChallenge {
    // All values are synthetic. The kernel seals this complete presentation;
    // changing any argument below without resealing must make the client refuse.
    let arguments = ContractPayload {
        schema: SchemaReference {
            schema_id: SchemaId::from_raw("fixture.read"),
            schema_version: 1,
            schema_sha256: "a".repeat(64),
        },
        media_type: "application/json".into(),
        bytes: b"{}".to_vec(),
        sha256: "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".into(),
    };
    seal_runtime_approval_challenge(RuntimeApprovalChallenge {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("run"),
        task_id: TaskId::from_raw("task"),
        turn_id: RuntimeTurnId::from_raw("turn"),
        operation_id: RuntimeOperationId::from_raw("operation"),
        tool_call_id: ToolCallId::from_raw("call"),
        approval_id: ApprovalId::from_raw("approval"),
        proposed_grant_id: GrantId::from_raw("grant"),
        operation: GrantOperation::WorkspaceRead,
        presentation: RuntimeApprovalPresentation {
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".into(),
            display_name: "Synthetic read".into(),
            risk_level: ToolRiskLevel::Low,
            target_scope: "one synthetic source".into(),
            arguments,
            declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
            single_use: true,
            timeout_ms: 1000,
        },
        preview_sha256: "d".repeat(64),
        expires_at_epoch_ms: 4_000_000_000_000,
        challenge_sha256: "0".repeat(64),
    })
    .unwrap()
}
fn step(approval: Value) -> Value {
    json!({"run_id":"run","request_sha256":"b".repeat(64),"events":[],"artifacts":[],"approval":approval,"outcome":null})
}
#[test]
fn model_start_requires_a_terminal_and_preserves_the_prepared_request() {
    let f = Fixture::new();
    let request: RuntimeRunRequest = agentmage_host::runtime_transport::decode_exact(
        include_bytes!("../../../fixtures/runtime-producer/v5/run-request.json"),
    )
    .unwrap();
    let encoded = serde_json::to_string(&request).unwrap();
    let mut reply = step(Value::Null);
    reply["run_id"] = json!(request.run_id);
    reply["request_sha256"] = json!(request.request_sha256);
    let peer = f.peer(vec![reply.clone()]);
    assert!(
        !f.cli(peer.address, &["runtime-start", &encoded])
            .status
            .success()
    );
    assert!(peer.finish().is_empty());
    let peer = f.peer(vec![reply]);
    let result = f.tty(peer.address, &["runtime-start", &encoded], "approve");
    assert_eq!(result["status"], 0);
    let calls = peer.finish();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["body"]["request"], json!(request));
    assert_eq!(calls[0]["body"]["approve_model_context"], true);
}
#[test]
fn piped_prepare_and_approval_never_send_authority() {
    let f = Fixture::new();
    let peer = f.peer(vec![json!({"expected_entry_sha256":"a".repeat(64)})]);
    assert!(
        !f.cli(peer.address, &["runtime-prepare", "Fix add"])
            .status
            .success()
    );
    assert_eq!(peer.finish().len(), 1);
    let peer = f.peer(vec![step(json!(challenge()))]);
    assert!(
        !f.cli(peer.address, &["runtime-approve", "run", &"b".repeat(64)])
            .status
            .success()
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["body"]["response"], Value::Null);
}
#[test]
fn exact_typed_approval_and_denial_use_only_the_current_host_challenge() {
    let f = Fixture::new();
    let challenge = challenge();
    let peer = f.peer(vec![step(json!(challenge)), step(Value::Null)]);
    let output = f.tty(
        peer.address,
        &["runtime-approve", "run", &"b".repeat(64)],
        "approve",
    );
    assert_eq!(output["status"], 0);
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1]["body"]["response"]["challenge_sha256"],
        challenge.challenge_sha256
    );
    assert_eq!(calls[1]["body"]["response"]["grant_id"], "grant");
    assert_eq!(calls[1]["body"]["response"]["disposition"], "allow");
    assert!(
        output["transcript"]
            .as_str()
            .unwrap()
            .contains("single_use")
    );
    assert!(
        output["transcript"]
            .as_str()
            .unwrap()
            .contains("arguments_json")
    );
    let peer = f.peer(vec![step(json!(challenge)), step(Value::Null)]);
    assert!(
        f.cli(peer.address, &["runtime-deny", "run", &"b".repeat(64)])
            .status
            .success()
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1]["body"]["response"]["disposition"], "deny");
    assert_eq!(calls[1]["body"]["response"]["grant_id"], Value::Null);
}
#[test]
fn modified_challenge_or_foreign_step_never_sends_approval() {
    let f = Fixture::new();
    let mut bad = json!(challenge());
    bad["presentation"]["target_scope"] = json!("changed");
    for reply in [step(bad), {
        let mut foreign = step(json!(challenge()));
        foreign["run_id"] = json!("foreign");
        foreign
    }] {
        let peer = f.peer(vec![reply]);
        let output = f.tty(
            peer.address,
            &["runtime-approve", "run", &"b".repeat(64)],
            "approve",
        );
        assert_ne!(output["status"], 0);
        assert_eq!(peer.finish().len(), 1);
        assert_eq!(output["answered"], 0);
    }
}
#[test]
fn cancellation_uses_the_observed_job_revision_without_direct_cancel() {
    let f = Fixture::new();
    let status = json!({"schema_version":1,"run_id":"run","request_sha256":"b".repeat(64),"job":{"job_id":"run","revision":7,"phase":"running","head_sha256":"a".repeat(64),"cancellation_requested":false}});
    let peer = f.peer(vec![
        status,
        json!({"decision":{"decision":"applied","revision":8,"phase":"cancelling"}}),
    ]);
    assert!(
        f.cli(
            peer.address,
            &["runtime-control", "run", &"b".repeat(64), "cancel"]
        )
        .status
        .success()
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["route"], "POST /v1/code/runtime/job HTTP/1.1");
    assert_eq!(calls[1]["route"], "POST /v1/code/runtime/control HTTP/1.1");
    assert_eq!(calls[1]["body"]["request"]["observed_revision"], 7);
    assert_eq!(calls[1]["body"]["request"]["action"], "cancel");
    assert!(
        calls[1]["body"]["request"]["request_id"]
            .as_str()
            .unwrap()
            .starts_with("terminal-")
    );
}
const PTY: &str = r#"
import errno,fcntl,json,os,pty,select,signal,subprocess,sys,termios,time
master,slave=pty.openpty()
def session():
    os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
p=subprocess.Popen(sys.argv[2:],stdin=slave,stdout=slave,stderr=slave,preexec_fn=session)
os.close(slave);raw=b'';answered=0;started=time.monotonic()
try:
    while True:
        assert time.monotonic()-started<10,'terminal timeout'
        if not select.select([master],[],[],.05)[0]:
            if p.poll() is not None:break
            continue
        try:chunk=os.read(master,65536)
        except OSError as e:
            if e.errno==errno.EIO:break
            raise
        if not chunk:break
        raw+=chunk
        if raw.count(b'anything else declines:')>answered:
            prompt=raw.split(b'Type exactly ')[-1].split(b' to approve;')[0]
            os.write(master,(prompt if sys.argv[1]=='approve' else b'no')+b'\n');answered+=1
    status=p.wait(timeout=2)
    assert b'\x1b' not in raw
    print(json.dumps({'status':status,'answered':answered,'transcript':raw.decode()}))
finally:
    if p.poll() is None:os.killpg(p.pid,signal.SIGKILL);p.wait()
    os.close(master)
"#;
