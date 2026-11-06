// SPDX-License-Identifier: Apache-2.0
//! Actual terminal executable -> local API -> kernel/native write contracts.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use agentmage_local_api::{ApiError, ApiTokens, serve};
use serde_json::{Value, json};

static NEXT: AtomicU64 = AtomicU64::new(0);
const BEFORE: &str = "def add(a, b):\n    return a - b\n";

struct Fixture {
    root: PathBuf,
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), ApiError>>>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-terminal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        ApiTokens::initialize(&root.join("auth")).unwrap();
        fs::create_dir(root.join("workspace")).unwrap();
        fs::write(root.join("workspace/calc.py"), BEFORE).unwrap();
        fs::write(root.join("workspace/untouched.py"), "sentinel\n").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let stop = stop.clone();
            let tokens = ApiTokens::load(&root.join("auth")).unwrap();
            thread::spawn(move || serve(listener, tokens, stop))
        };
        Self {
            root,
            address,
            stop,
            worker: Some(worker),
        }
    }
    fn cli(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .args([
                self.address.to_string(),
                self.root.join("auth").to_str().unwrap().to_string(),
            ])
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
    fn json(&self, args: &[&str]) -> Value {
        let output = self.cli(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn api(&self, method: &str, path: &str, body: Value) -> Value {
        let token = fs::read_to_string(self.root.join("auth/control.token")).unwrap();
        let body = serde_json::to_vec(&body).unwrap();
        let mut socket = TcpStream::connect(self.address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        socket.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", self.address, body.len()).as_bytes()).unwrap();
        socket.write_all(&body).unwrap();
        let mut reply = Vec::new();
        socket.read_to_end(&mut reply).unwrap();
        assert!(
            reply.starts_with(b"HTTP/1.1 200 "),
            "{}",
            String::from_utf8_lossy(&reply)
        );
        let start = reply.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
        serde_json::from_slice(&reply[start..]).unwrap()
    }
    fn select(&self) -> Value {
        self.json(&[
            "select",
            self.root.join("workspace").to_str().unwrap(),
            "calc.py",
        ])
    }
    fn prepare(&self) -> Value {
        // The native preview itself hashes the real source; the strict checker
        // needs its SHA-256 from a proposal context, obtained without inference.
        let read = self.json(&["read"]);
        assert_eq!(read["files"][0]["content"], BEFORE);
        let revision = read["workspace_revision"].clone();
        // sha256 of the exact synthetic BEFORE, independent literal fixture.
        self.api("POST", "/v1/code/workspace/changes", json!({
            "workspace_revision":revision,"paths":["calc.py"],
            "expected_sources":[{"path":"calc.py","sha256":"e1a894022d1a082987b87adecb623438c9e386d86b2b621cff4a5fe7fdf7edc8"}],
            "task":"Fix add","max_message_bytes":4096,
            "answer":"{\"edits\":[{\"path\":\"calc.py\",\"old\":\"return a - b\",\"new\":\"return a + b\"}]}"
        }))
    }
    fn terminal(&self, args: &[&str], response: &str) -> Output {
        // A real controlling PTY is necessary: production code has no injected
        // confirmation provider, --yes switch or stdin fallback.
        const SCRIPT: &str = r#"
import errno, fcntl, json, os, pty, select, signal, subprocess, sys, termios, time
master, slave = pty.openpty()
def session():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)
p = subprocess.Popen(sys.argv[2:], stdin=slave, stdout=slave, stderr=slave, preexec_fn=session)
os.close(slave)
transcript = b''; sent = False; start = time.monotonic()
try:
    while True:
        if time.monotonic()-start > 10: raise RuntimeError('terminal timeout')
        if not select.select([master], [], [], .05)[0]:
            if p.poll() is not None: break
            continue
        try: chunk = os.read(master, 65536)
        except OSError as error:
            if error.errno == errno.EIO: break
            raise
        if not chunk: break
        transcript += chunk
        if b'anything else declines:' in transcript and not sent:
            prompt = transcript.split(b'Type exactly ')[-1].split(b' to approve;')[0]
            if sys.argv[1] == 'revoke':
                import http.client
                from pathlib import Path
                connection = http.client.HTTPConnection(sys.argv[3], timeout=2)
                token = (Path(sys.argv[4])/'control.token').read_text()
                connection.request('DELETE', '/v1/code/workspace', headers={'Authorization':'Bearer '+token,'Content-Length':'0'})
                reply = connection.getresponse(); assert reply.status == 200; reply.read(); connection.close()
            if sys.argv[1] == 'replace-root':
                from pathlib import Path
                root = Path(sys.argv[6]); root.rename(root.with_name('old-workspace')); root.mkdir()
                (root/'calc.py').write_text('replacement sentinel')
            answer = prompt if sys.argv[1] in ['approve', 'revoke', 'replace-root'] else sys.argv[1].encode()
            os.write(master, answer+b'\n'); sent = True
    status = p.wait(timeout=2)
    # No response/proposal can emit a raw terminal escape or Unicode bidi.
    assert b'\x1b' not in transcript and b'\xe2\x80\xae' not in transcript
    sys.stdout.write(json.dumps({'status':status, 'sent':sent, 'transcript':transcript.decode()}))
finally:
    if p.poll() is None:
        os.killpg(p.pid, signal.SIGKILL); p.wait()
    os.close(master)
"#;
        Command::new("python3")
            .args([
                "-B",
                "-c",
                SCRIPT,
                response,
                env!("CARGO_BIN_EXE_agentmage-code"),
                &self.address.to_string(),
                self.root.join("auth").to_str().unwrap(),
            ])
            .args(args)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        assert_eq!(self.worker.take().unwrap().join().unwrap(), Ok(()));
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn executable_reads_searches_and_revokes_only_the_explicit_selection() {
    let f = Fixture::new();
    assert_eq!(f.json(&["status"])["busy"], false);
    assert_eq!(f.select()["selected"], true);
    let read = f.json(&["read"]);
    assert_eq!(read["files"].as_array().unwrap().len(), 1);
    assert_eq!(read["files"][0]["content"], BEFORE);
    assert_eq!(
        f.json(&["search", "return"])["matches"][0]["text"],
        "return"
    );
    assert!(
        !f.cli(&[
            "select",
            f.root.join("workspace").to_str().unwrap(),
            "../auth/control.token"
        ])
        .status
        .success()
    );
    assert_eq!(f.json(&["workspace"])["paths"], json!(["calc.py"]));
    assert_eq!(f.json(&["revoke"])["selected"], false);
    assert!(!f.cli(&["read"]).status.success());
}

#[test]
fn executable_maps_without_reading_and_selects_ranked_paths_only_after_typed_consent() {
    let f = Fixture::new();
    let root = f.root.join("workspace");
    fs::write(root.join(".env"), "private secret").unwrap();
    fs::write(root.join("calc-binary.py"), b"\0unreadable as text").unwrap();
    let map = f.json(&["map", root.to_str().unwrap(), "calc"]);
    assert_eq!(map["entries"][0]["path"], "calc-binary.py");
    assert_eq!(map["excluded"], 1);
    assert_eq!(f.json(&["workspace"])["selected"], false);
    fs::remove_file(root.join("calc-binary.py")).unwrap();
    f.api(
        "PUT",
        "/v1/settings",
        json!({"endpoint":"http://127.0.0.1:19997/v1","approve_conversations":true}),
    );
    assert!(
        !f.cli(&["map-select", root.to_str().unwrap(), "calc"])
            .status
            .success()
    );
    let wrong = f.terminal(&["map-select", root.to_str().unwrap(), "calc"], "yes");
    let wrong: Value = serde_json::from_slice(&wrong.stdout).unwrap();
    assert_eq!(wrong["status"], 1);
    assert_eq!(f.json(&["workspace"])["selected"], false);
    let output = f.terminal(&["map-select", root.to_str().unwrap(), "calc"], "approve");
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["status"], 0, "{output}");
    assert_eq!(output["sent"], true);
    let selected = f.json(&["workspace"]);
    assert_eq!(selected["paths"], json!(["calc.py"]));
    assert_eq!(selected["model_context_approved"], true);
    assert_eq!(f.json(&["read"])["files"][0]["content"], BEFORE);
    let output = f.terminal(
        &["map-select", root.to_str().unwrap(), "no-match"],
        "approve",
    );
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["status"], 1);
    assert_eq!(output["sent"], false);
    assert_eq!(f.json(&["workspace"])["paths"], json!(["calc.py"]));
    assert_eq!(
        fs::read_to_string(root.join("untouched.py")).unwrap(),
        "sentinel\n"
    );
}

#[test]
fn ranked_source_selection_refuses_root_replacement_while_the_approval_is_displayed() {
    let f = Fixture::new();
    f.api(
        "PUT",
        "/v1/settings",
        json!({"endpoint":"http://127.0.0.1:19996/v1","approve_conversations":true}),
    );
    let root = f.root.join("workspace");
    let output = f.terminal(
        &["map-select", root.to_str().unwrap(), "calc"],
        "replace-root",
    );
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["sent"], true);
    assert_eq!(output["status"], 1, "{output}");
    assert_eq!(f.json(&["workspace"])["selected"], false);
    assert_eq!(
        fs::read_to_string(root.join("calc.py")).unwrap(),
        "replacement sentinel"
    );
    assert_eq!(
        fs::read_to_string(f.root.join("old-workspace/calc.py")).unwrap(),
        BEFORE
    );
}

#[test]
fn ranked_source_choice_stays_within_the_existing_sixteen_file_read_budget() {
    let f = Fixture::new();
    let root = f.root.join("workspace");
    for index in 0..20 {
        fs::write(root.join(format!("calc-{index:02}.py")), "source\n").unwrap();
    }
    f.api(
        "PUT",
        "/v1/settings",
        json!({"endpoint":"http://127.0.0.1:19995/v1","approve_conversations":true}),
    );
    let output = f.terminal(&["map-select", root.to_str().unwrap(), "calc"], "approve");
    let output: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output["status"], 0, "{output}");
    let selected = f.json(&["workspace"]);
    let paths = selected["paths"].as_array().unwrap();
    assert_eq!(paths.len(), 16);
    assert_eq!(paths[0], "calc-00.py");
    assert_eq!(paths[15], "calc-15.py");
    assert_eq!(f.json(&["read"])["files"].as_array().unwrap().len(), 16);
    assert_eq!(
        fs::read_to_string(root.join("calc-19.py")).unwrap(),
        "source\n"
    );
}

#[test]
fn executable_shows_bounded_escaped_refusals_without_treating_them_as_success() {
    let f = Fixture::new();
    let output = f.cli(&["map", f.root.join("workspace").to_str().unwrap(), "!!!"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("code.map.input"));
    assert!(output.stdout.is_empty());
    for body in [
        json!({"error":"code.write.stale"}),
        json!({"error":"native.failure\u{1b}\u{202e}\u{7f}","effect":"unknown"}),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let expected = if body["effect"] == "unknown" {
            "unknown"
        } else {
            "code.write.stale"
        };
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                assert!(request.len() < 8192);
            }
            let bytes = serde_json::to_vec(&body).unwrap();
            socket.write_all(format!("HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).as_bytes()).unwrap();
            socket.write_all(&bytes).unwrap();
        });
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .args([
                address.to_string(),
                f.root.join("auth").to_str().unwrap().to_owned(),
                "status".into(),
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        worker.join().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(text.contains(expected));
        assert!(!text.contains(['\u{1b}', '\u{202e}', '\u{7f}']));
        assert!(text.is_ascii());
    }
}

#[test]
fn executable_requires_a_fresh_typed_terminal_approval_for_exact_native_writes() {
    let f = Fixture::new();
    f.select();
    let prepared = f.prepare();
    assert_eq!(prepared["applied"], false);
    assert!(
        !f.cli(&["apply", f.root.join("workspace").to_str().unwrap()])
            .status
            .success()
    );
    let mut piped = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
        .args([
            f.address.to_string(),
            f.root.join("auth").to_str().unwrap().to_string(),
            "apply".into(),
            f.root.join("workspace").to_str().unwrap().to_string(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let challenge = format!(
        "apply {}\n",
        prepared["preview"]["preview_sha256"].as_str().unwrap()
    );
    let _ = piped.stdin.take().unwrap().write_all(challenge.as_bytes());
    assert!(!piped.wait_with_output().unwrap().status.success());
    assert_eq!(
        fs::read_to_string(f.root.join("workspace/calc.py")).unwrap(),
        BEFORE
    );
    let wrong = f.terminal(
        &["apply", f.root.join("workspace").to_str().unwrap()],
        "yes",
    );
    assert!(wrong.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&wrong.stdout).unwrap()["status"],
        1
    );
    assert_eq!(
        fs::read_to_string(f.root.join("workspace/calc.py")).unwrap(),
        BEFORE
    );
    let other = f.terminal(&["apply", f.root.to_str().unwrap()], "approve");
    let other: Value = serde_json::from_slice(&other.stdout).unwrap();
    assert_eq!(other["status"], 1);
    assert_eq!(other["sent"], false);
    let approved = f.terminal(
        &["apply", f.root.join("workspace").to_str().unwrap()],
        "approve",
    );
    assert!(
        approved.status.success(),
        "{}",
        String::from_utf8_lossy(&approved.stderr)
    );
    let approved: Value = serde_json::from_slice(&approved.stdout).unwrap();
    assert_eq!(approved["status"], 0, "{approved}");
    assert!(
        approved["transcript"]
            .as_str()
            .unwrap()
            .contains("committed")
    );
    assert_eq!(
        fs::read_to_string(f.root.join("workspace/calc.py")).unwrap(),
        BEFORE.replace("a - b", "a + b")
    );
    assert_eq!(
        fs::read_to_string(f.root.join("workspace/untouched.py")).unwrap(),
        "sentinel\n"
    );
    assert!(
        !f.cli(&["apply", f.root.join("workspace").to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(f.json(&["review"])["pending"], Value::Null);
}

#[test]
fn denial_is_inert_and_credential_file_links_are_refused() {
    let f = Fixture::new();
    f.select();
    f.prepare();
    assert_eq!(f.json(&["deny"])["outcome"], "denied");
    assert_eq!(
        fs::read_to_string(f.root.join("workspace/calc.py")).unwrap(),
        BEFORE
    );
    assert!(!f.cli(&["deny"]).status.success());
    fs::remove_file(f.root.join("auth/client.token")).unwrap();
    std::os::unix::fs::symlink(
        f.root.join("auth/control.token"),
        f.root.join("auth/client.token"),
    )
    .unwrap();
    assert!(!f.cli(&["status"]).status.success());
}

fn model(answer: &str, finish: &str) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let answer = answer.to_owned();
    let finish = finish.to_owned();
    let worker = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = std::time::Instant::now();
        let mut socket = loop {
            match listener.accept() {
                Ok((socket, _)) => break socket,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(start.elapsed() < Duration::from_secs(10));
                    thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("{e}"),
            }
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
        let n: usize = std::str::from_utf8(&header)
            .unwrap()
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .unwrap()
            .parse()
            .unwrap();
        let mut body = vec![0; n];
        socket.read_exact(&mut body).unwrap();
        let request: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(request["model"], "fixture");
        assert!(
            request["messages"][1]["content"]
                .as_str()
                .unwrap()
                .contains("calc.py")
        );
        assert!(!request.to_string().contains("control.token"));
        let body = format!(
            "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
            json!({"model":"fixture","choices":[{"index":0,"delta":{"content":answer},"finish_reason":null}]}),
            json!({"model":"fixture","choices":[{"index":0,"delta":{},"finish_reason":finish}]})
        );
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
    });
    (endpoint, worker)
}

#[test]
fn terminal_source_choice_and_streamed_proposal_prepare_without_applying() {
    for finish in ["stop", "length"] {
        let f = Fixture::new();
        let (endpoint, worker) = model(
            "{\"edits\":[{\"path\":\"calc.py\",\"old\":\"return a - b\",\"new\":\"return a + b\"}]}",
            finish,
        );
        f.api(
            "PUT",
            "/v1/settings",
            json!({"endpoint":endpoint,"approve_conversations":true}),
        );
        assert!(
            !f.cli(&[
                "select-model",
                f.root.join("workspace").to_str().unwrap(),
                "calc.py"
            ])
            .status
            .success()
        );
        assert_eq!(f.json(&["workspace"])["selected"], false);
        let selected = f.terminal(
            &[
                "select-model",
                f.root.join("workspace").to_str().unwrap(),
                "calc.py",
            ],
            "approve",
        );
        assert!(
            selected.status.success(),
            "{}",
            String::from_utf8_lossy(&selected.stderr)
        );
        let selected: Value = serde_json::from_slice(&selected.stdout).unwrap();
        assert_eq!(selected["status"], 0, "{selected}");
        assert_eq!(f.json(&["workspace"])["model_context_approved"], true);
        let proposed = f.cli(&["propose", "fixture", "4096", "512", "Fix add"]);
        worker.join().unwrap();
        assert_eq!(
            proposed.status.success(),
            finish == "stop",
            "{}",
            String::from_utf8_lossy(&proposed.stderr)
        );
        let pending = f.json(&["review"])["pending"].clone();
        assert_eq!(pending.is_object(), finish == "stop");
        assert_eq!(
            fs::read_to_string(f.root.join("workspace/calc.py")).unwrap(),
            BEFORE
        );
        let transcript = String::from_utf8(proposed.stdout).unwrap();
        assert!(transcript.contains(if finish == "stop" {
            "proposal"
        } else {
            "error"
        }));
    }
}

#[test]
fn a_preview_revoked_while_displayed_cannot_be_applied_by_the_terminal() {
    let f = Fixture::new();
    f.select();
    f.prepare();
    let stale = f.terminal(
        &["apply", f.root.join("workspace").to_str().unwrap()],
        "revoke",
    );
    assert!(
        stale.status.success(),
        "{}",
        String::from_utf8_lossy(&stale.stderr)
    );
    let stale: Value = serde_json::from_slice(&stale.stdout).unwrap();
    assert_eq!(stale["status"], 1);
    assert_eq!(stale["sent"], true);
    assert_eq!(
        fs::read_to_string(f.root.join("workspace/calc.py")).unwrap(),
        BEFORE
    );
    assert_eq!(f.json(&["review"])["pending"], Value::Null);
}
