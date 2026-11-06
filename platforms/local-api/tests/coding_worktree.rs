// SPDX-License-Identifier: Apache-2.0
//! Actual API/CLI state, consent and native refusal; no model or sandbox bypass.

use serde_json::{Value, json};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    child: Option<Child>,
    address: Option<SocketAddr>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-worktree-api-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for name in ["state", "owned", "repo", "repo/.git"] {
            fs::create_dir(root.join(name)).unwrap();
            fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
        }
        fs::write(root.join("repo/sentinel"), b"human work").unwrap();
        let status = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
            .args(["init-auth", root.join("auth").to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());
        let mut f = Self {
            root,
            child: None,
            address: None,
        };
        f.start();
        f
    }
    fn start(&mut self) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
            .args([
                "serve-coding",
                "127.0.0.1:0",
                self.root.join("auth").to_str().unwrap(),
                self.root.join("state").to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        // The API announces the listener before opening state. Bound the readiness
        // probe and keep the child in this fixture even on assertion failure.
        let mut stdout = child.stdout.take().unwrap();
        self.child = Some(child);
        let mut line = Vec::new();
        loop {
            let mut byte = [0];
            assert_eq!(stdout.read(&mut byte).unwrap(), 1);
            if byte[0] == b'\n' {
                break;
            }
            line.push(byte[0]);
            assert!(line.len() < 200);
        }
        self.address = Some(
            String::from_utf8(line)
                .unwrap()
                .split_whitespace()
                .last()
                .unwrap()
                .parse()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let output = self.cli(&["worktree"]);
            if output.status.success() {
                break;
            }
            assert!(Instant::now() < deadline, "API did not open coding state");
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            child.wait().unwrap();
        }
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .args([
                self.address.unwrap().to_string(),
                self.root.join("auth").to_str().unwrap().to_string(),
            ])
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
    fn call_bytes(&self, method: &str, path: &str, control: bool, body: &[u8]) -> (u16, Value) {
        let token = fs::read_to_string(self.root.join(if control {
            "auth/control.token"
        } else {
            "auth/client.token"
        }))
        .unwrap();
        let address = self.address.unwrap();
        let mut socket = TcpStream::connect(address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).as_bytes()).unwrap();
        socket.write_all(body).unwrap();
        let mut reply = Vec::new();
        loop {
            let mut bytes = [0; 4096];
            match socket.read(&mut bytes) {
                Ok(0) => break,
                Ok(n) => reply.extend_from_slice(&bytes[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => panic!("{e}"),
            }
        }
        let end = reply.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let status = std::str::from_utf8(&reply[..end])
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (status, serde_json::from_slice(&reply[end + 4..]).unwrap())
    }
    fn call(&self, method: &str, path: &str, control: bool, body: Value) -> (u16, Value) {
        self.call_bytes(method, path, control, &serde_json::to_vec(&body).unwrap())
    }
    fn prepare(&self) -> Value {
        json!({"root":self.root.join("repo"),"owned_root":self.root.join("owned"),"task_id":"fixture","base_commit":"1".repeat(40),"approve_inspection":true})
    }
    fn terminal(&self, args: &[&str], response: &str) -> Value {
        let output = Command::new("python3")
            .args([
                "-c",
                PTY,
                response,
                env!("CARGO_BIN_EXE_agentmage-code"),
                &self.address.unwrap().to_string(),
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
}

const PTY: &str = r#"
import errno,fcntl,json,os,pty,select,signal,subprocess,sys,termios,time
master,slave=pty.openpty()
def session():
    os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
p=subprocess.Popen(sys.argv[2:],stdin=slave,stdout=slave,stderr=slave,preexec_fn=session)
os.close(slave);raw=b'';sent=False;started=time.monotonic()
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
        if b'anything else declines:' in raw and not sent:
            prompt=raw.split(b'Type exactly ')[-1].split(b' to approve;')[0]
            if sys.argv[1] in ['replace-owned','replace-root']:
                from pathlib import Path
                root=Path(sys.argv[7] if sys.argv[1]=='replace-owned' else sys.argv[6]);root.rename(root.with_name('old-owned'));root.mkdir(mode=0o700)
            os.write(master,(prompt if sys.argv[1] in ['approve','replace-owned','replace-root'] else sys.argv[1].encode())+b'\n');sent=True
    status=p.wait(timeout=2)
    assert b'\x1b' not in raw
    print(json.dumps({'status':status,'sent':sent,'transcript':raw.decode()}))
finally:
    if p.poll() is None:os.killpg(p.pid,signal.SIGKILL);p.wait()
    os.close(master)
"#;
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop();
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn worktree_routes_require_control_and_explicit_object_consent() {
    let f = Fixture::new();
    let request = f.prepare();
    assert_eq!(
        f.call(
            "POST",
            "/v1/code/repository/prepare",
            false,
            request.clone()
        )
        .0,
        403
    );
    assert_eq!(
        f.call_bytes("GET", "/v1/code/repository", false, b"").0,
        403
    );
    assert_eq!(
        f.call(
            "POST",
            "/v1/code/repository/decide",
            false,
            json!({"preview_sha256":"a".repeat(64),"approve":true})
        )
        .0,
        403
    );
    let mut denied = request.clone();
    denied["approve_inspection"] = json!(false);
    assert_eq!(
        f.call("POST", "/v1/code/repository/prepare", true, denied)
            .0,
        403
    );
    let mut unknown = request.clone();
    unknown["command"] = json!("anything");
    assert_eq!(
        f.call("POST", "/v1/code/repository/prepare", true, unknown)
            .0,
        400
    );
    let mut string = request.clone();
    string["approve_inspection"] = json!("true");
    assert_eq!(
        f.call("POST", "/v1/code/repository/prepare", true, string)
            .0,
        400
    );
    assert_eq!(
        f.call("POST", "/v1/code/repository/prepare", true, json!([]))
            .0,
        400
    );
    let duplicate =
        serde_json::to_string(&request)
            .unwrap()
            .replacen('{', "{\"approve_inspection\":true,", 1);
    assert_eq!(
        f.call_bytes(
            "POST",
            "/v1/code/repository/prepare",
            true,
            duplicate.as_bytes()
        )
        .0,
        400
    );
    assert_eq!(
        f.call(
            "POST",
            "/v1/code/repository/decide",
            true,
            json!({"preview_sha256":"a".repeat(64),"approve":true})
        )
        .1["error"],
        "code.repository.choice-required"
    );
    let data: Value = serde_json::from_slice(&f.cli(&["worktree"]).stdout).unwrap();
    assert!(data["pending"].is_null());
    assert!(data["receipts"].as_array().unwrap().is_empty());
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"human work"
    );
    assert!(!f.root.join("owned/worktree").exists());
}

#[test]
fn executable_refuses_non_tty_inspection_and_reopens_canonical_state() {
    let mut f = Fixture::new();
    let output = f.cli(&[
        "worktree-prepare",
        f.root.join("repo").to_str().unwrap(),
        f.root.join("owned").to_str().unwrap(),
        "fixture",
        &"1".repeat(40),
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let output = f.cli(&[
        "worktree-create",
        f.root.join("repo").to_str().unwrap(),
        f.root.join("owned").to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(!f.cli(&["worktree-deny"]).status.success());
    let before: Value = serde_json::from_slice(&f.cli(&["worktree"]).stdout).unwrap();
    f.stop();
    f.start();
    let after: Value = serde_json::from_slice(&f.cli(&["worktree"]).stdout).unwrap();
    assert_eq!(after, before);
    assert_eq!(after["automatic_replay"], false);
    assert!(!f.root.join("owned/worktree").exists());
}

#[test]
fn actual_native_inspection_refusal_preserves_source_scope_and_disk() {
    let f = Fixture::new();
    let request = f.prepare();
    let (status, value) = f.call("POST", "/v1/code/repository/prepare", true, request);
    assert_eq!(status, 409);
    if fs::metadata("/usr/bin/git").unwrap().uid() != 0 {
        assert_eq!(value["error"], "linux.repository.git_artifact.invalid");
        println!(
            "native worktree admission refused: untrusted system Git ownership; no native creation measured"
        );
    } else {
        // This fixture deliberately has no Git objects, even on a native host.
        assert_eq!(value["error"], "linux.repository.observation.failed");
    }
    assert_eq!(value["effect"], "no-worktree-dispatch");
    let scope: Value = serde_json::from_slice(&f.cli(&["workspace"]).stdout).unwrap();
    assert_eq!(scope["selected"], false);
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"human work"
    );
    assert!(fs::read_dir(f.root.join("owned")).unwrap().next().is_none());
}

#[test]
fn controlling_terminal_inspection_requires_the_exact_disclosed_challenge() {
    let f = Fixture::new();
    let root = f.root.join("repo");
    let owned = f.root.join("owned");
    let base = "1".repeat(40);
    let invalid = f.terminal(
        &[
            "worktree-prepare",
            root.to_str().unwrap(),
            owned.to_str().unwrap(),
            "fixture",
            "bad\u{1b}[2J",
        ],
        "approve",
    );
    assert_ne!(invalid["status"], 0);
    assert_eq!(invalid["sent"], false);
    let args = [
        "worktree-prepare",
        root.to_str().unwrap(),
        owned.to_str().unwrap(),
        "fixture",
        &base,
    ];
    let denied = f.terminal(&args, "incorrect");
    assert!(denied["sent"].as_bool().unwrap());
    assert_ne!(denied["status"], 0);
    let display = denied["transcript"].as_str().unwrap();
    assert!(display.contains("Runs hardened local Git inspection"));
    assert!(display.contains(root.to_str().unwrap()));
    assert!(display.contains(owned.to_str().unwrap()));
    assert!(display.contains(&format!("inspect {base}")));
    assert!(!display.contains("API refusal"));
    let approved = f.terminal(&args, "approve");
    assert_ne!(approved["status"], 0);
    assert_eq!(approved["sent"], true);
    assert!(approved["transcript"].as_str().unwrap().contains(
        if fs::metadata("/usr/bin/git").unwrap().uid() != 0 {
            "linux.repository.git_artifact.invalid"
        } else {
            "linux.repository.observation.failed"
        }
    ));
    assert!(!owned.join("worktree").exists());
}

#[test]
fn synthetic_api_creation_preview_needs_exact_tty_consent_and_current_roots() {
    // This peer tests the terminal protocol only. Its success reply is synthetic;
    // no native worktree is created or qualified by this test.
    for response in ["incorrect", "approve", "replace-owned"] {
        let mut f = Fixture::new();
        f.stop();
        let root = f.root.join("repo");
        let owned = f.root.join("owned");
        let identity = |path: &std::path::Path| {
            agentmage_platform_linux::discover_linux_coding_map(path, "sentinel", || false)
                .unwrap()
                .root_sha256
        };
        let preview = json!({"preview_sha256":"a".repeat(64),"repository_root_sha256":identity(&root),
            "owned_root_sha256":identity(&owned),"plan":{"branch_ref":"refs/heads/agentmage/tasks/fixture","expected_new_object":"1".repeat(40)}});
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        f.address = Some(listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(12);
            let mut requests = Vec::new();
            while !done.load(Ordering::Relaxed) {
                assert!(Instant::now() < deadline);
                let (mut socket, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).unwrap();
                    header.push(byte[0]);
                    assert!(header.len() < 8192);
                }
                let header = String::from_utf8(header).unwrap();
                let route = header.lines().next().unwrap().to_owned();
                let length = header
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                assert!(length < 4096);
                let mut body = vec![0; length];
                socket.read_exact(&mut body).unwrap();
                let value = if route.starts_with("GET ") {
                    assert_eq!(route, "GET /v1/code/repository HTTP/1.1");
                    json!({"pending":preview})
                } else {
                    assert_eq!(route, "POST /v1/code/repository/decide HTTP/1.1");
                    json!({"outcome":"succeeded"})
                };
                requests.push((route, body));
                let bytes = serde_json::to_vec(&value).unwrap();
                socket.write_all(format!("HTTP/1.1 200 Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).unwrap();
                socket.write_all(&bytes).unwrap();
            }
            requests
        });
        let observed = f.terminal(
            &[
                "worktree-create",
                root.to_str().unwrap(),
                owned.to_str().unwrap(),
            ],
            response,
        );
        stop.store(true, Ordering::Relaxed);
        let requests = worker.join().unwrap();
        assert_eq!(observed["sent"], true);
        assert_eq!(observed["status"] == 0, response == "approve");
        assert!(
            observed["transcript"]
                .as_str()
                .unwrap()
                .contains("refs/heads/agentmage/tasks/fixture")
        );
        assert_eq!(requests.len(), if response == "approve" { 2 } else { 1 });
        if response == "approve" {
            assert_eq!(
                serde_json::from_slice::<Value>(&requests[1].1).unwrap(),
                json!({"preview_sha256":"a".repeat(64),"approve":true})
            );
        }
        assert!(!owned.join("worktree").exists());
    }
}

#[test]
fn command_api_requires_control_created_ownership_and_strict_requests() {
    let mut f = Fixture::new();
    let inspect = "/v1/code/repository/command";
    let prepare = "/v1/code/repository/command/prepare";
    let decide = "/v1/code/repository/command/decide";
    assert_eq!(f.call_bytes("GET", inspect, false, b"").0, 403);
    for (method, path, body) in [
        ("POST", prepare, json!({"command":{}})),
        (
            "POST",
            decide,
            json!({"preview_sha256":"a".repeat(64),"approve":true}),
        ),
    ] {
        assert_eq!(f.call(method, path, false, body).0, 403);
    }
    for body in [
        json!([]),
        json!({"command":{}}),
        json!({"command":{},"approve":true}),
    ] {
        assert_eq!(f.call("POST", prepare, true, body).0, 400);
    }
    for bytes in [
        br#"{"preview_sha256":"a","approve":true,"approve":false}"#.as_slice(),
        br#"{"preview_sha256":"a","approve":"true"}"#.as_slice(),
        br#"{"preview_sha256":"a","approve":true,"root":"/outside"}"#.as_slice(),
    ] {
        assert_eq!(f.call_bytes("POST", decide, true, bytes).0, 400);
    }
    let spec = json!({"schema_version":1,"template_id":"synthetic","template_version":"1.0.0",
        "executable":"/usr/bin/true","executable_sha256":"a".repeat(64),"arguments":[],
        "working_directory":"owned_worktree","environment":{},"risk":"moderate","grant_required":true,
        "interactive":false,"network":false,"inherit_environment":false,
        "bounds":{"timeout_ms":1000,"stdout_bytes":1024,"stderr_bytes":1024,"memory_bytes":134217728,"task_count":8,"cpu_percent":100},
        "spec_sha256":"a".repeat(64)});
    // No caller path can invent the provenance; refuse before native verification.
    let refused = f.call("POST", prepare, true, json!({"command":spec}));
    assert_eq!(refused.0, 409);
    assert_eq!(refused.1["error"], "code.command.worktree-required");
    assert_eq!(refused.1["effect"], "no-command-dispatch");
    let refused = f.call(
        "POST",
        decide,
        true,
        json!({"preview_sha256":"a".repeat(64),"approve":true}),
    );
    assert_eq!(refused.1["error"], "code.command.choice-required");
    assert_eq!(refused.1["effect"], "unknown");
    let output = f.cli(&["command"]);
    assert!(output.status.success());
    let before: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(before["pending"].is_null());
    assert!(before["worktree_root_sha256"].is_null());
    assert!(
        !f.cli(&["command-run", f.root.join("repo").to_str().unwrap()])
            .status
            .success()
    );
    assert!(!f.cli(&["command-deny"]).status.success());
    f.stop();
    f.start();
    let after: Value = serde_json::from_slice(&f.cli(&["command"]).stdout).unwrap();
    assert_eq!(before, after);
    assert!(
        f.call_bytes("GET", "/v1/code/repository", true, b"").1["receipts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"human work"
    );
}

#[test]
fn synthetic_api_command_preview_needs_exact_tty_consent_and_current_root() {
    // Synthetic peer checks terminal command approval only; no command is launched.
    for response in ["incorrect", "approve", "replace-root"] {
        let mut f = Fixture::new();
        f.stop();
        let root = f.root.join("repo");
        let owned = f.root.join("owned");
        let identity = |path: &std::path::Path| {
            agentmage_platform_linux::discover_linux_coding_map(path, "sentinel", || false)
                .unwrap()
                .root_sha256
        };
        let preview = json!({"preview_sha256":"a".repeat(64),"worktree_root_sha256":identity(&root),
            "owned_root_sha256":identity(&owned),"plan":{"branch_ref":"refs/heads/agentmage/tasks/fixture","expected_new_object":"1".repeat(40)}});
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        f.address = Some(listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(12);
            let mut requests = Vec::new();
            while !done.load(Ordering::Relaxed) {
                assert!(Instant::now() < deadline);
                let (mut socket, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).unwrap();
                    header.push(byte[0]);
                    assert!(header.len() < 8192);
                }
                let header = String::from_utf8(header).unwrap();
                let route = header.lines().next().unwrap().to_owned();
                let length = header
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                assert!(length < 4096);
                let mut body = vec![0; length];
                socket.read_exact(&mut body).unwrap();
                let value = if route.starts_with("GET ") {
                    assert_eq!(route, "GET /v1/code/repository/command HTTP/1.1");
                    json!({"pending":preview})
                } else {
                    assert_eq!(route, "POST /v1/code/repository/command/decide HTTP/1.1");
                    json!({"outcome":"succeeded"})
                };
                requests.push((route, body));
                let bytes = serde_json::to_vec(&value).unwrap();
                socket.write_all(format!("HTTP/1.1 200 Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).unwrap();
                socket.write_all(&bytes).unwrap();
            }
            requests
        });
        let observed = f.terminal(&["command-run", root.to_str().unwrap()], response);
        stop.store(true, Ordering::Relaxed);
        let requests = worker.join().unwrap();
        assert_eq!(observed["sent"], true);
        assert_eq!(observed["status"] == 0, response == "approve");
        assert!(
            observed["transcript"]
                .as_str()
                .unwrap()
                .contains("refs/heads/agentmage/tasks/fixture")
        );
        assert_eq!(requests.len(), if response == "approve" { 2 } else { 1 });
        if response == "approve" {
            assert_eq!(
                serde_json::from_slice::<Value>(&requests[1].1).unwrap(),
                json!({"preview_sha256":"a".repeat(64),"approve":true})
            );
        }
        assert!(!owned.join("worktree").exists());
    }
}
