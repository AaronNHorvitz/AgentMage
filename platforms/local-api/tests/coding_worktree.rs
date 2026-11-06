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
answered=0
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
        if raw.count(b'anything else declines:')>answered and (not sent or sys.argv[1]=='approve-all'):
            prompt=raw.split(b'Type exactly ')[-1].split(b' to approve;')[0]
            if sys.argv[1] in ['replace-owned','replace-root']:
                from pathlib import Path
                root=Path(sys.argv[7] if sys.argv[1]=='replace-owned' else sys.argv[6]);root.rename(root.with_name('old-owned'));root.mkdir(mode=0o700)
            os.write(master,(prompt if sys.argv[1] in ['approve','approve-all','replace-owned','replace-root'] else sys.argv[1].encode())+b'\n');sent=True;answered+=1
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
    let spec = synthetic_command_spec();
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
    for (kind, validation_passed, parser) in [
        ("command", true, "agent_mage_json_v1"),
        ("validation", true, "agent_mage_json_v1"),
        ("validation", false, "agent_mage_json_v1"),
        ("validation", true, "rust_libtest_text_v1"),
        ("validation", false, "rust_libtest_text_v1"),
        ("validation", true, "python_unittest_text_v1"),
        ("validation", false, "python_unittest_text_v1"),
        ("edit", true, "agent_mage_json_v1"),
    ] {
        let validation = kind == "validation";
        let edit = kind == "edit";
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
            let mut preview = json!({"preview_sha256":"a".repeat(64),"worktree_root_sha256":identity(&root),
            "operation":"execute-command", "creation_receipt_sha256":"b".repeat(64),
            "command":{"schema_version":1,"command_attempt_id":"synthetic-attempt",
                "template_id":"synthetic-check","template_version":"1.0.0",
                "executable":"/usr/bin/true","executable_sha256":"c".repeat(64),
                "arguments":["untrusted-λ\u{202e}\u{7f}"],"working_directory":"owned_worktree",
                "environment":{"LANG":"C.UTF-8"},"risk":"moderate",
                "grant_required":true,"interactive":false,"network":false,"inherit_environment":false,
                "bounds":{"timeout_ms":119_001,"stdout_bytes":65_535,"stderr_bytes":65_534,
                    "memory_bytes":134_217_728,"task_count":8,"cpu_percent":100},
                "preview_sha256":"d".repeat(64)}});
            let (inspect, decide) = if edit {
                preview = json!({"preview_sha256":"a".repeat(64),"worktree_root_sha256":identity(&root),
                    "operation":"edit-worktree","creation_receipt_sha256":"b".repeat(64),
                    "write_preview":{"change_set_sha256":"c".repeat(64),"preview_sha256":"d".repeat(64),
                        "diff":"untrusted-λ\u{202e}\u{7f}","before":"original","after":"replacement"}});
                (
                    "/v1/code/repository/edit",
                    "/v1/code/repository/edit/decide",
                )
            } else if validation {
                preview["operation"] = json!("execute-validation");
                preview["validation_template"] = json!({"kind":"unit","minimum_test_count":2,
                "parser":parser,"parser_sha256":"e".repeat(64)});
                preview["attempt"] = json!(2);
                preview["maximum_attempts"] = json!(3);
                preview["prior_receipt_sha256"] = json!("f".repeat(64));
                (
                    "/v1/code/repository/validation",
                    "/v1/code/repository/validation/decide",
                )
            } else {
                (
                    "/v1/code/repository/command",
                    "/v1/code/repository/command/decide",
                )
            };
            let expected_preview = preview.clone();
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
                        assert_eq!(route, format!("GET {inspect} HTTP/1.1"));
                        json!({"pending":preview})
                    } else {
                        assert_eq!(route, format!("POST {decide} HTTP/1.1"));
                        json!({"outcome":if edit {"committed"} else {"succeeded"},"validation":{"validation_passed":validation_passed}})
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
                    if edit {
                        "edit-apply"
                    } else if validation {
                        "validation-run"
                    } else {
                        "command-run"
                    },
                    root.to_str().unwrap(),
                ],
                response,
            );
            stop.store(true, Ordering::Relaxed);
            let requests = worker.join().unwrap();
            assert_eq!(observed["sent"], true);
            assert_eq!(
                observed["status"] == 0,
                response == "approve" && validation_passed
            );
            let transcript = observed["transcript"].as_str().unwrap();
            assert!(transcript.contains(if edit {
                "Type exactly edit "
            } else if validation {
                "Type exactly test "
            } else {
                "Type exactly run "
            }));
            assert!(transcript.is_ascii());
            assert!(!transcript.contains('\u{7f}'));
            assert!(transcript.contains("\\u03bb\\u202e\\u007f"));
            // Parse the displayed consent before the challenge. Every command field
            // must reach the person, including exact arguments, environment and limits.
            let consent: Value =
                serde_json::from_str(transcript.split("Type exactly ").next().unwrap()).unwrap();
            assert_eq!(consent["preview"], expected_preview);
            assert_eq!(consent["worktree"], root.to_str().unwrap());
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
}

fn synthetic_command_spec() -> Value {
    json!({"schema_version":1,"template_id":"synthetic","template_version":"1.0.0",
        "executable":"/usr/bin/true","executable_sha256":"a".repeat(64),"arguments":[],
        "working_directory":"owned_worktree","environment":{},"risk":"moderate","grant_required":true,
        "interactive":false,"network":false,"inherit_environment":false,
        "bounds":{"timeout_ms":1000,"stdout_bytes":1024,"stderr_bytes":1024,"memory_bytes":134217728,"task_count":8,"cpu_percent":100},
        "spec_sha256":"a".repeat(64)})
}

#[test]
fn validation_api_and_cli_require_explicit_control_and_cannot_adopt_a_worktree() {
    let mut f = Fixture::new();
    let inspect = "/v1/code/repository/validation";
    let prepare = "/v1/code/repository/validation/prepare";
    let decide = "/v1/code/repository/validation/decide";
    let definition = json!({"command":synthetic_command_spec(),"kind":"unit",
        "minimum_test_count":2,"approve_definition":true});
    assert_eq!(f.call_bytes("GET", inspect, false, b"").0, 403);
    assert_eq!(f.call("POST", prepare, false, definition.clone()).0, 403);
    assert_eq!(
        f.call(
            "POST",
            decide,
            false,
            json!({"preview_sha256":"a".repeat(64),"approve":true})
        )
        .0,
        403
    );
    for key in [
        "approve_definition",
        "minimum_test_count",
        "kind",
        "command",
    ] {
        let mut missing = definition.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert_eq!(f.call("POST", prepare, true, missing).0, 400);
    }
    for (key, value) in [
        ("approve_definition", json!("true")),
        ("root", json!("/outside")),
        ("parser", json!("process_exit_v1")),
        ("maximum_attempts", json!(100)),
    ] {
        let mut invalid = definition.clone();
        invalid[key] = value;
        assert_eq!(f.call("POST", prepare, true, invalid).0, 400);
    }
    for format in [
        "agent_mage_json_v1",
        "rust_libtest_text_v1",
        "python_unittest_text_v1",
    ] {
        let mut input = definition.clone();
        input["output_format"] = json!(format);
        // Valid protocol selection still cannot supply ownership or execute.
        let refused = f.call("POST", prepare, true, input.clone());
        assert_eq!(refused.0, 409);
        assert_eq!(refused.1["error"], "code.command.worktree-required");
        let bytes = serde_json::to_string(&input).unwrap().replacen(
            '{',
            "{\"output_format\":\"rust_libtest_text_v1\",",
            1,
        );
        assert_eq!(f.call_bytes("POST", prepare, true, bytes.as_bytes()).0, 400);
    }
    for bad in [
        json!("process_exit_v1"),
        json!("auto"),
        json!(true),
        Value::Null,
    ] {
        let mut input = definition.clone();
        input["output_format"] = bad;
        assert_eq!(f.call("POST", prepare, true, input).0, 400);
    }
    let mut denied = definition.clone();
    denied["approve_definition"] = json!(false);
    assert_eq!(f.call("POST", prepare, true, denied).0, 403);
    assert_eq!(f.call("POST", prepare, true, json!([])).0, 400);
    let bytes = serde_json::to_string(&definition).unwrap().replacen(
        '{',
        "{\"approve_definition\":true,",
        1,
    );
    assert_eq!(f.call_bytes("POST", prepare, true, bytes.as_bytes()).0, 400);
    let refused = f.call("POST", prepare, true, definition.clone());
    assert_eq!(refused.0, 409);
    assert_eq!(refused.1["error"], "code.command.worktree-required");
    assert_eq!(refused.1["effect"], "no-command-dispatch");
    let refused = f.call(
        "POST",
        decide,
        true,
        json!({"preview_sha256":"a".repeat(64),"approve":true}),
    );
    assert_eq!(refused.1["error"], "code.validation.choice-required");
    assert_eq!(refused.1["effect"], "unknown");
    let before: Value = serde_json::from_slice(&f.cli(&["validation"]).stdout).unwrap();
    assert!(before["pending"].is_null());
    assert!(before["latest"].is_null());
    assert_eq!(before["maximum_attempts"], 3);
    assert!(
        !f.cli(&["validation-run", f.root.join("repo").to_str().unwrap()])
            .status
            .success()
    );
    assert!(!f.cli(&["validation-deny"]).status.success());
    let prepared = f.cli(&[
        "validation-prepare",
        &serde_json::to_string(&definition).unwrap(),
    ]);
    assert!(!prepared.status.success());
    assert!(
        String::from_utf8(prepared.stderr)
            .unwrap()
            .contains("code.command.worktree-required")
    );
    f.stop();
    f.start();
    let after: Value = serde_json::from_slice(&f.cli(&["validation"]).stdout).unwrap();
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
fn owned_edit_api_requires_control_read_consent_and_never_adopts_a_caller_root() {
    let mut f = Fixture::new();
    let inspect = "/v1/code/repository/edit";
    let prepare = "/v1/code/repository/edit/prepare";
    let decide = "/v1/code/repository/edit/decide";
    let definition = json!({"paths":["sentinel"],"expected_sources":[{"path":"sentinel","sha256":"a".repeat(64)}],
        "task":"Repair sentinel","max_message_bytes":4096,
        "answer":"{\"edits\":[{\"path\":\"sentinel\",\"old\":\"human work\",\"new\":\"replacement\"}]}","approve_read":true});
    assert_eq!(f.call_bytes("GET", inspect, false, b"").0, 403);
    for (route, body) in [
        (prepare, definition.clone()),
        (
            decide,
            json!({"preview_sha256":"a".repeat(64),"approve":true}),
        ),
    ] {
        assert_eq!(f.call("POST", route, false, body).0, 403);
    }
    for bytes in [
        br#"[]"#.as_slice(),
        br#"{"paths":[],"expected_sources":[],"task":"x","max_message_bytes":4096,"answer":"x","approve_read":"true"}"#.as_slice(),
        br#"{"paths":[],"expected_sources":[],"task":"x","max_message_bytes":4096,"answer":"x","approve_read":true,"approve_read":false}"#.as_slice(),
        br#"{"paths":[],"expected_sources":[["sentinel","hash"]],"task":"x","max_message_bytes":4096,"answer":"x","approve_read":true}"#.as_slice(),
    ] { assert_eq!(f.call_bytes("POST",prepare,true,bytes).0,400); }
    let mut no_consent = definition.clone();
    no_consent["approve_read"] = json!(false);
    assert_eq!(f.call("POST", prepare, true, no_consent).0, 403);
    for extra in ["root", "creation_receipt_sha256", "approve"] {
        let mut injected = definition.clone();
        injected[extra] = json!("caller-supplied");
        assert_eq!(f.call("POST", prepare, true, injected).0, 400);
    }
    let refused = f.call("POST", prepare, true, definition);
    assert_eq!(refused.0, 409);
    assert_eq!(refused.1["error"], "code.command.worktree-required");
    assert_eq!(refused.1["effect"], "no-write-dispatch");
    for bytes in [
        br#"{"preview_sha256":"a","approve":true,"approve":false}"#.as_slice(),
        br#"{"preview_sha256":"a","approve":"true"}"#.as_slice(),
        br#"{"preview_sha256":"a","approve":true,"root":"/outside"}"#.as_slice(),
        br#"["a",true]"#.as_slice(),
    ] {
        assert_eq!(f.call_bytes("POST", decide, true, bytes).0, 400);
    }
    let refused = f.call(
        "POST",
        decide,
        true,
        json!({"preview_sha256":"a".repeat(64),"approve":true}),
    );
    assert_eq!(refused.1["error"], "code.edit.choice-required");
    assert_eq!(refused.1["effect"], "unknown");
    let output = f.cli(&["edit"]);
    assert!(output.status.success());
    let before: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(before["pending"].is_null());
    assert!(before["latest"].is_null());
    assert!(before["worktree_root_sha256"].is_null());
    assert!(
        !f.cli(&["edit-apply", f.root.join("repo").to_str().unwrap()])
            .status
            .success()
    );
    assert!(!f.cli(&["edit-deny"]).status.success());
    f.stop();
    f.start();
    let output = f.cli(&["edit"]);
    assert!(output.status.success());
    let after: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(before, after);
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"human work"
    );
}

#[test]
fn repair_api_requires_control_exact_object_disclosure_and_owned_failure() {
    let f = Fixture::new();
    assert_eq!(
        f.call_bytes("GET", "/v1/code/repository/repair", false, b"")
            .0,
        403
    );
    let (code, value) = f.call_bytes("GET", "/v1/code/repository/repair", true, b"");
    assert_eq!(code, 409);
    assert_eq!(value["error"], "code.command.worktree-required");
    let request = json!({"workspace_revision":1,"paths":["sentinel"],"model":"synthetic-model",
        "task":"Repair sentinel","max_message_bytes":8192,"max_tokens":128,
        "receipt_sha256":"a".repeat(64),"feedback_sha256":"b".repeat(64),
        "settings_revision":0,"settings_endpoint":"http://127.0.0.1:12345/v1",
        "approve_failure_disclosure":true});
    let route = "/v1/code/repository/repair/proposals";
    assert_eq!(f.call("POST", route, false, request.clone()).0, 403);
    for body in [
        json!([]),
        json!({}),
        {
            let mut body = request.clone();
            body["unexpected"] = json!(true);
            body
        },
        {
            let mut body = request.clone();
            body["approve_failure_disclosure"] = json!("true");
            body
        },
    ] {
        assert_eq!(f.call("POST", route, true, body).0, 400);
    }
    let duplicate = serde_json::to_string(&request).unwrap().replacen(
        '{',
        "{\"approve_failure_disclosure\":true,",
        1,
    );
    assert_eq!(
        f.call_bytes("POST", route, true, duplicate.as_bytes()).0,
        400
    );
    let mut refused = request.clone();
    refused["approve_failure_disclosure"] = json!(false);
    assert_eq!(
        f.call("POST", route, true, refused).1["error"],
        "code.repair.disclosure-required"
    );
    assert_eq!(f.call("POST", route, true, request.clone()).0, 428);
    let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
    upstream.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", upstream.local_addr().unwrap());
    assert_eq!(
        f.call(
            "PUT",
            "/v1/settings",
            true,
            json!({"endpoint":endpoint,
        "approve_conversations":true})
        )
        .0,
        200
    );
    let mut stale = request.clone();
    stale["settings_endpoint"] = json!(endpoint);
    assert_eq!(
        f.call("POST", route, true, stale.clone()).1["error"],
        "code.repair.settings-stale"
    );
    stale["settings_revision"] = json!(1);
    assert_eq!(
        f.call("POST", route, true, stale.clone()).1["error"],
        "code.workspace.choice-required"
    );
    assert_eq!(
        f.call(
            "PUT",
            "/v1/code/workspace",
            true,
            json!({
        "root":f.root.join("repo"),"paths":["sentinel"],"approve_read":true,
        "approve_model_context":true,"settings_revision":1,"settings_endpoint":endpoint})
        )
        .0,
        200
    );
    assert_eq!(
        f.call("POST", route, true, stale).1["error"],
        "code.command.worktree-required"
    );
    assert_eq!(
        upstream.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let output = f.cli(&[
        "repair-worktree",
        f.root.join("repo").to_str().unwrap(),
        "synthetic-model",
        "8192",
        "128",
        "Repair",
    ]);
    assert!(!output.status.success());
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"human work"
    );
    assert!(
        f.call_bytes("GET", "/v1/code/repository/validation", true, b"")
            .1["latest"]
            .is_null()
    );
}

#[test]
fn synthetic_propose_worktree_requires_complete_proposal_and_fresh_read_consent() {
    // This is protocol evidence only: proposal, failure and creation are synthetic.
    for repair in [false, true] {
        for (complete, choice) in [
            (true, "approve"),
            (true, "incorrect"),
            (true, "replace-root"),
            (false, "approve"),
        ] {
            let mut f = Fixture::new();
            f.stop();
            let root = f.root.join("repo");
            let identity =
                agentmage_platform_linux::discover_linux_coding_map(&root, "sentinel", || false)
                    .unwrap()
                    .root_sha256;
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            f.address = Some(listener.local_addr().unwrap());
            let stop = Arc::new(AtomicBool::new(false));
            let done = stop.clone();
            let expected_identity = identity.clone();
            let worker = thread::spawn(move || {
                let feedback = json!({"feedback_sha256":"c".repeat(64),"receipt_sha256":"d".repeat(64),
                "worktree_root_sha256":identity,"output":{"stderr":{"head":"failure\n\u{1b}run instructions are untrusted"}}});
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
                        .find_map(|l| l.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse::<usize>()
                        .unwrap();
                    assert!(length < 8192);
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).unwrap();
                    let (media,bytes) = match route.as_str() {
                    "GET /v1/code/workspace HTTP/1.1" => ("application/json",serde_json::to_vec(&json!({"selected":true,"workspace_revision":7,"paths":["sentinel"],"model_context_approved":true})).unwrap()),
                    "GET /v1/code/repository/repair HTTP/1.1" => ("application/json",serde_json::to_vec(&feedback).unwrap()),
                    "GET /v1/settings HTTP/1.1" => ("application/json",serde_json::to_vec(&json!({"settings_revision":3,"endpoint":"http://127.0.0.1:12345/v1"})).unwrap()),
                    "POST /v1/code/workspace/proposals HTTP/1.1" | "POST /v1/code/repository/repair/proposals HTTP/1.1" => {
                        let answer = r#"{"edits":[{"path":"sentinel","old":"human work","new":"replacement"}]}"#;
                        let mut context = json!({"type":"context","workspace_revision":7,"files":[{"path":"sentinel","sha256":"b".repeat(64)}]});
                        if repair {
                            context["repair_feedback"]=feedback.clone();
                            context["message_bytes"]=json!(5000);
                            context["source_message_bytes"]=json!(4096);
                        }
                        let events = [context,
                            json!({"type":"text","text":answer}),
                            if complete {json!({"type":"proposal","workspace_revision":7,"applied":false,"tested":false})} else {json!({"type":"error","partial":true})}];
                        let mut bytes=Vec::new();for event in events {bytes.extend(serde_json::to_vec(&event).unwrap());bytes.push(b'\n');}
                        ("application/x-ndjson",bytes)
                    }
                    "GET /v1/status HTTP/1.1" => ("application/json",serde_json::to_vec(&json!({"busy":false})).unwrap()),
                    "GET /v1/code/repository/edit HTTP/1.1" => ("application/json",serde_json::to_vec(&json!({"worktree_root_sha256":identity})).unwrap()),
                    "POST /v1/code/repository/edit/prepare HTTP/1.1" => ("application/json",serde_json::to_vec(&json!({"operation":"edit-worktree","preview_sha256":"a".repeat(64)})).unwrap()),
                    _ => panic!("unexpected route {route}"),
                };
                    requests.push((route, body));
                    if media == "application/x-ndjson" {
                        socket.write_all(format!("HTTP/1.1 200 Response\r\nContent-Type: {media}\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
                    } else {
                        socket.write_all(format!("HTTP/1.1 200 Response\r\nContent-Type: {media}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).unwrap();
                    }
                    socket.write_all(&bytes).unwrap();
                }
                requests
            });
            let observed = f.terminal(
                &[
                    if repair {
                        "repair-worktree"
                    } else {
                        "propose-worktree"
                    },
                    root.to_str().unwrap(),
                    "synthetic-model",
                    if repair { "8192" } else { "4096" },
                    "128",
                    "Repair sentinel",
                ],
                if repair && choice == "approve" {
                    "approve-all"
                } else {
                    choice
                },
            );
            stop.store(true, Ordering::Relaxed);
            let requests = worker.join().unwrap();
            let approved = complete && choice == "approve";
            assert_eq!(observed["status"] == 0, approved);
            assert_eq!(observed["sent"], complete || repair);
            let prepared = requests
                .iter()
                .filter(|(route, _)| route == "POST /v1/code/repository/edit/prepare HTTP/1.1")
                .collect::<Vec<_>>();
            assert_eq!(prepared.len(), usize::from(approved));
            assert_eq!(
                requests
                    .iter()
                    .filter(|(r, _)| r
                        == if repair {
                            "POST /v1/code/repository/repair/proposals HTTP/1.1"
                        } else {
                            "POST /v1/code/workspace/proposals HTTP/1.1"
                        })
                    .count(),
                if repair && choice != "approve" { 0 } else { 1 }
            );
            if approved {
                let body: Value = serde_json::from_slice(&prepared[0].1).unwrap();
                assert_eq!(
                    body,
                    json!({"paths":["sentinel"],"expected_sources":[{"path":"sentinel","sha256":"b".repeat(64)}],
                "task":"Repair sentinel","max_message_bytes":4096,"answer":r#"{"edits":[{"path":"sentinel","old":"human work","new":"replacement"}]}"#,"approve_read":true})
                );
                assert!(
                    observed["transcript"]
                        .as_str()
                        .unwrap()
                        .contains(&format!("read {expected_identity}"))
                );
            }
            if repair && (choice == "approve") {
                let body: Value = serde_json::from_slice(
                    &requests
                        .iter()
                        .find(|(r, _)| r == "POST /v1/code/repository/repair/proposals HTTP/1.1")
                        .unwrap()
                        .1,
                )
                .unwrap();
                assert_eq!(body["settings_revision"], 3);
                assert_eq!(body["settings_endpoint"], "http://127.0.0.1:12345/v1");
                assert_eq!(body["feedback_sha256"], "c".repeat(64));
                assert_eq!(body["receipt_sha256"], "d".repeat(64));
                assert_eq!(body["approve_failure_disclosure"], true);
                assert!(
                    observed["transcript"]
                        .as_str()
                        .unwrap()
                        .contains("share-failure")
                );
            }
            assert!(!requests.iter().any(|(r, _)| r.contains("/decide")));
        }
    }
}
