// SPDX-License-Identifier: MIT OR Apache-2.0
//! Real terminal/HTTP/native transfer; synthetic creation and no model dispatch.
use super::*;
use std::process::{Command, Stdio};

#[path = "coding_endpoint_run_transfer.rs"]
mod guided_transfer;

fn start(fixture: &Fixture, checked: bool) -> (Api, TcpListener) {
    fs::create_dir(fixture.0.join("original/.git")).unwrap();
    fs::write(fixture.0.join("original/.git/index"), b"index sentinel").unwrap();
    fs::write(fixture.0.join("original/human"), b"human sentinel").unwrap();
    ApiTokens::initialize(&fixture.0.join("auth")).unwrap();
    fs::write(fixture.0.join("auth/client.token"), CLIENT).unwrap();
    fs::write(fixture.0.join("auth/control.token"), CONTROL).unwrap();
    let mut owner = fixture.owner();
    fixture.prepare(&mut owner, "first\n");
    let pending = owner.inspect_edit().unwrap();
    assert_eq!(
        owner
            .decide_edit(
                pending["pending"]["preview_sha256"].as_str().unwrap(),
                true,
                1000
            )
            .unwrap()["outcome"],
        "committed"
    );
    let (upstream, endpoint) = upstream();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = thread::spawn(move || {
        if checked {
            agentmage_local_api::serve_coding_endpoint_edits_and_checked_tests_with_repository_review(
                listener, ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(), worker_stop,
                CodingEndpointObservationAccess::ControlOnly,
                |id, session| Ok(checked_input(owner, endpoint, id, session)),
            )
        } else {
            agentmage_local_api::serve_coding_endpoint_strict_edits_with_repository_review(
                listener,
                ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                worker_stop,
                CodingEndpointObservationAccess::ControlOnly,
                |id, session| {
                    Ok(strict_input(
                        owner,
                        endpoint,
                        id,
                        session,
                        Arc::new(AtomicUsize::new(0)),
                    ))
                },
            )
        }
    });
    (
        Api {
            address,
            stop,
            worker: Some(worker),
        },
        upstream,
    )
}

fn terminal(
    api: &Api,
    fixture: &Fixture,
    request: &Value,
    scenario: &str,
    seconds: &str,
    path: &str,
) -> Value {
    terminal_at(api.address, fixture, request, scenario, seconds, path)
}

fn terminal_at(
    address: SocketAddr,
    fixture: &Fixture,
    request: &Value,
    scenario: &str,
    seconds: &str,
    path: &str,
) -> Value {
    terminal_command(
        address,
        fixture,
        scenario,
        &[
            "endpoint-original-apply",
            seconds,
            request["run_id"].as_str().unwrap(),
            request["request_sha256"].as_str().unwrap(),
            fixture.0.join("original").to_str().unwrap(),
            fixture.0.join("owned/worktree").to_str().unwrap(),
            "Transfer approved changes",
            path,
        ],
    )
}

fn terminal_command(
    address: SocketAddr,
    fixture: &Fixture,
    scenario: &str,
    args: &[&str],
) -> Value {
    const PTY: &str = r#"
import errno, fcntl, json, os, pty, select, signal, subprocess, sys, termios, time
from pathlib import Path
master, slave = pty.openpty()
def session():
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)
p = subprocess.Popen(sys.argv[3:], stdin=slave, stdout=slave, stderr=slave, preexec_fn=session)
os.close(slave)
transcript = b''; prompts = []; start = time.monotonic()
try:
    while True:
        if time.monotonic() - start > 12: raise RuntimeError('terminal timeout')
        if not select.select([master], [], [], .02)[0]:
            if p.poll() is not None: break
            continue
        try: chunk = os.read(master, 65536)
        except OSError as error:
            if error.errno == errno.EIO: break
            raise
        if not chunk: break
        transcript += chunk
        if transcript.count(b'anything else declines:') > len(prompts):
            prompt = transcript.rsplit(b'Type exactly ', 1)[1].split(b' to approve;')[0]
            prompts.append(prompt.decode())
            if sys.argv[1] == 'delay-each': time.sleep(1.1)
            if sys.argv[1] == 'interrupt-release' and prompt.startswith(b'release '):
                os.killpg(p.pid, signal.SIGTERM); continue
            if sys.argv[1] == 'timeout-read' and len(prompts) == 2: continue
            if sys.argv[1] == 'interrupt-apply' and len(prompts) == 3:
                os.killpg(p.pid, signal.SIGINT); continue
            if sys.argv[1] == 'drift-apply' and len(prompts) == 3:
                (Path(sys.argv[2])/'original/one.py').write_text('later human edit\n')
            answer = b'decline' if sys.argv[1] == 'decline-' + str(len(prompts)) else prompt
            os.write(master, answer + b'\n')
    status = p.wait(timeout=2)
    assert b'\x1b' not in transcript
    print(json.dumps({'status':status,'prompts':prompts,'transcript':transcript.decode()}))
finally:
    if p.poll() is None:
        os.killpg(p.pid, signal.SIGKILL); p.wait()
    os.close(master)
"#;
    let output = Command::new("python3")
        .args([
            "-B",
            "-c",
            PTY,
            scenario,
            fixture.0.to_str().unwrap(),
            env!("CARGO_BIN_EXE_agentmage-code"),
            &address.to_string(),
            fixture.0.join("auth").to_str().unwrap(),
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

#[test]
fn terminal_original_transfer_lost_effect_reply_never_retries_or_claims_success() {
    for lost in [
        "POST /v1/code/endpoint/release",
        "POST /v1/code/repository/apply/decide",
    ] {
        let fixture = Fixture::new();
        let (api, upstream) = start(&fixture, false);
        let request = api.request();
        cancel(&api, &request);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let done = AtomicBool::new(false);
        let (output, routes) = thread::scope(|scope| {
            let proxy = scope.spawn(|| {
                let mut routes = Vec::new();
                while !done.load(Ordering::Acquire) {
                    let mut client = match listener.accept() {
                        Ok((socket, _)) => socket,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(error) => panic!("{error}"),
                    };
                    client
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let mut header = Vec::new();
                    while !header.ends_with(b"\r\n\r\n") {
                        let mut byte = [0];
                        client.read_exact(&mut byte).unwrap();
                        header.push(byte[0]);
                        assert!(header.len() < 8192);
                    }
                    let text = std::str::from_utf8(&header).unwrap();
                    let route = text
                        .lines()
                        .next()
                        .unwrap()
                        .strip_suffix(" HTTP/1.1")
                        .unwrap()
                        .to_owned();
                    routes.push(route.clone());
                    let length: usize = text
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    assert!(length < 256 * 1024);
                    let mut body = vec![0; length];
                    client.read_exact(&mut body).unwrap();
                    let mut target = TcpStream::connect(api.address).unwrap();
                    target
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    // Preserve production Host validation: only this fixture
                    // proxy's authority changes when forwarding to the API.
                    let header = text.replace(
                        &format!("\r\nHost: {address}\r\n"),
                        &format!("\r\nHost: {}\r\n", api.address),
                    );
                    target.write_all(header.as_bytes()).unwrap();
                    target.write_all(&body).unwrap();
                    let mut reply = Vec::new();
                    target.read_to_end(&mut reply).unwrap();
                    assert!(
                        reply.starts_with(b"HTTP/1.1 200 "),
                        "{route}: {}",
                        String::from_utf8_lossy(&reply)
                    );
                    if route != lost {
                        client.write_all(&reply).unwrap();
                    }
                    // The real effect finishes, but the selected successful
                    // response is dropped. Keep listening to detect retries.
                }
                routes
            });
            let output = terminal_at(address, &fixture, &request, "allow", "30", "one.py");
            done.store(true, Ordering::Release);
            (output, proxy.join().unwrap())
        });
        assert_ne!(output["status"], 0, "{output}");
        assert!(
            !output["transcript"]
                .as_str()
                .unwrap()
                .contains("endpoint-original-transfer")
        );
        assert_eq!(routes.last().unwrap(), lost);
        assert_eq!(routes.iter().filter(|route| *route == lost).count(), 1);
        let applied = lost.ends_with("/decide");
        assert_eq!(
            output["prompts"].as_array().unwrap().len(),
            if applied { 3 } else { 1 }
        );
        preserved(&fixture, if applied { b"first\n" } else { b"before\n" });
        let status = api
            .call("GET", "/v1/code/repository/apply", CONTROL, None)
            .1;
        assert!(status["pending"].is_null());
        assert_eq!(
            status["available_paths"],
            if applied {
                json!([])
            } else {
                json!(["one.py"])
            }
        );
        if applied {
            assert_eq!(status["latest"]["outcome"], "committed");
        }
        drop(api);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

fn cancel(api: &Api, request: &Value) {
    assert_eq!(api.bind("cancel", request).0, 200);
    let terminal = api.advance(request, Value::Null, false);
    assert_eq!(terminal.0, 200);
    assert_eq!(terminal.1["outcome"]["outcome"]["state"], "CANCELLED");
}

fn preserved(fixture: &Fixture, original: &[u8]) {
    assert_eq!(
        fs::read(fixture.0.join("original/one.py")).unwrap(),
        original
    );
    assert_eq!(
        fs::read(fixture.0.join("owned/worktree/one.py")).unwrap(),
        b"first\n"
    );
    assert_eq!(
        fs::read(fixture.0.join("original/human")).unwrap(),
        b"human sentinel"
    );
    assert_eq!(
        fs::read(fixture.0.join("original/.git/index")).unwrap(),
        b"index sentinel"
    );
}

#[test]
fn saved_terminal_transfer_retains_replayable_record_before_release_on_both_services() {
    use std::os::unix::fs::PermissionsExt;
    for checked in [false, true] {
        let fixture = Fixture::new();
        let (api, upstream) = start(&fixture, checked);
        let request = api.request();
        cancel(&api, &request);
        let archive = fixture.0.join("records");
        fs::create_dir(&archive).unwrap();
        fs::set_permissions(&archive, fs::Permissions::from_mode(0o700)).unwrap();
        let args = [
            "endpoint-original-apply-saved",
            archive.to_str().unwrap(),
            "30",
            request["run_id"].as_str().unwrap(),
            request["request_sha256"].as_str().unwrap(),
            fixture.0.join("original").to_str().unwrap(),
            fixture.0.join("owned/worktree").to_str().unwrap(),
            "Transfer approved changes",
            "one.py",
        ]
        .map(str::to_owned);
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let declined = terminal_command(api.address, &fixture, "decline-1", &args);
        assert_ne!(declined["status"], 0, "{declined}");
        assert_eq!(declined["prompts"].as_array().unwrap().len(), 1);
        assert_eq!(fs::read_dir(&archive).unwrap().count(), 0);
        assert_eq!(api.bind("events", &request).0, 200);
        preserved(&fixture, b"before\n");
        let destination = archive.join(format!(
            "{}.json",
            request["request_sha256"].as_str().unwrap()
        ));
        fs::write(&destination, b"human record sentinel").unwrap();
        let collision = terminal_command(api.address, &fixture, "allow", &args);
        assert_ne!(collision["status"], 0, "{collision}");
        assert_eq!(collision["prompts"].as_array().unwrap().len(), 1);
        assert_eq!(api.bind("events", &request).0, 200);
        assert_eq!(fs::read(&destination).unwrap(), b"human record sentinel");
        preserved(&fixture, b"before\n");
        fs::remove_file(&destination).unwrap();
        // Save succeeds but release is declined: the record survives and the
        // run is still held. Reusing the same filename must stop before release.
        let declined = terminal_command(api.address, &fixture, "decline-2", &args);
        assert_ne!(declined["status"], 0, "{declined}");
        assert_eq!(declined["prompts"].as_array().unwrap().len(), 2);
        let bytes = fs::read(&destination).unwrap();
        let record: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(record["request"], request);
        assert_eq!(api.bind("events", &request).0, 200);
        preserved(&fixture, b"before\n");
        let collision = terminal_command(api.address, &fixture, "allow", &args);
        assert_ne!(collision["status"], 0);
        assert_eq!(collision["prompts"].as_array().unwrap().len(), 1);
        assert_eq!(fs::read(&destination).unwrap(), bytes);
        // The person chooses a fresh private directory for a new exact save.
        let fresh = fixture.0.join("fresh-records");
        fs::create_dir(&fresh).unwrap();
        fs::set_permissions(&fresh, fs::Permissions::from_mode(0o700)).unwrap();
        let mut fresh_args = args.clone();
        fresh_args[1] = fresh.to_str().unwrap();
        let output = terminal_command(api.address, &fixture, "allow", &fresh_args);
        assert_eq!(output["status"], 0, "{output}");
        let prompts = output["prompts"].as_array().unwrap();
        assert_eq!(prompts.len(), 4);
        assert!(prompts[0].as_str().unwrap().starts_with("save-record "));
        assert!(prompts[1].as_str().unwrap().starts_with("release "));
        assert!(prompts[2].as_str().unwrap().starts_with("read "));
        assert!(prompts[3].as_str().unwrap().starts_with("apply-original "));
        let transcript = output["transcript"].as_str().unwrap();
        assert!(
            transcript.find("\"saved\": true").unwrap()
                < transcript.find("Type exactly release ").unwrap()
        );
        assert!(transcript.contains("endpoint-original-transfer"));
        assert_eq!(
            fs::read(fresh.join(destination.file_name().unwrap())).unwrap(),
            bytes
        );
        assert_eq!(api.bind("events", &request).0, 409);
        preserved(&fixture, b"first\n");
        drop(api);
        let retained = fs::read(fresh.join(destination.file_name().unwrap())).unwrap();
        assert_eq!(retained, bytes);
        let mut child = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .arg("endpoint-replay")
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&retained).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let replay: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(replay["record"], record);
        assert_eq!(replay["recorded_terminal_state"], "CANCELLED");
        assert_eq!(replay["resumes_execution"], false);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn terminal_original_transfer_exports_controlled_record_then_requires_three_fresh_decisions() {
    for checked in [false, true] {
        let fixture = Fixture::new();
        let (api, upstream) = start(&fixture, checked);
        let request = api.request();
        cancel(&api, &request);
        let started = Instant::now();
        let output = terminal(&api, &fixture, &request, "allow", "30", "one.py");
        eprintln!(
            "terminal original transfer checked={checked} elapsed_us={}",
            started.elapsed().as_micros()
        );
        assert_eq!(output["status"], 0, "{output}");
        let prompts = output["prompts"].as_array().unwrap();
        assert_eq!(prompts.len(), 3);
        assert_eq!(
            prompts[0],
            format!("release {}", request["request_sha256"].as_str().unwrap())
        );
        assert!(prompts[1].as_str().unwrap().starts_with("read "));
        assert!(prompts[2].as_str().unwrap().starts_with("apply-original "));
        let transcript = output["transcript"].as_str().unwrap();
        let record = serde_json::Deserializer::from_str(transcript)
            .into_iter::<Value>()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(record["request"], request);
        assert_eq!(record["request"]["mode"], "controlled_write");
        assert_eq!(
            record["events"].as_array().unwrap().last().unwrap()["kind"]["state"],
            "CANCELLED"
        );
        assert!(transcript.contains("endpoint-original-transfer"));
        assert!(transcript.contains("\"original_checkout_tested\": false"));
        assert!(
            transcript.find("\"events\"").unwrap()
                < transcript.find("Type exactly release ").unwrap()
        );
        assert!(
            transcript.find("\"write_preview\"").unwrap()
                < transcript.find("Type exactly apply-original ").unwrap()
        );
        let mut replay = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .arg("endpoint-replay")
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        replay
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&record).unwrap())
            .unwrap();
        let replay = replay.wait_with_output().unwrap();
        assert!(replay.status.success());
        let replay: Value = serde_json::from_slice(&replay.stdout).unwrap();
        assert_eq!(replay["resumes_execution"], false);
        assert_eq!(replay["record"], record);
        preserved(&fixture, b"first\n");
        assert_eq!(api.bind("events", &request).0, 409);
        let status = api.call("GET", "/v1/code/repository/apply", CONTROL, None);
        assert_eq!(status.0, 200);
        assert_eq!(status.1["available_paths"], json!([]));
        assert!(status.1["pending"].is_null());
        drop(api);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn terminal_original_transfer_declines_release_read_and_apply_without_writes() {
    for decision in 1..=3 {
        let fixture = Fixture::new();
        let (api, upstream) = start(&fixture, false);
        let request = api.request();
        cancel(&api, &request);
        let output = terminal(
            &api,
            &fixture,
            &request,
            &format!("decline-{decision}"),
            "30",
            "one.py",
        );
        assert_ne!(output["status"], 0, "{output}");
        assert_eq!(output["prompts"].as_array().unwrap().len(), decision);
        preserved(&fixture, b"before\n");
        if decision == 1 {
            assert_eq!(api.bind("events", &request).0, 200);
            assert_eq!(
                api.call("GET", "/v1/code/repository/apply", CONTROL, None)
                    .0,
                428
            );
        } else {
            assert_eq!(api.bind("events", &request).0, 409);
            let status = api
                .call("GET", "/v1/code/repository/apply", CONTROL, None)
                .1;
            assert_eq!(status["pending"].is_null(), decision == 2);
            assert!(status["latest"].is_null());
        }
        drop(api);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn standalone_original_commands_resolve_post_release_declines_on_both_review_services() {
    for checked in [false, true] {
        for decision in [2, 3] {
            for approve in [false, true] {
                let fixture = Fixture::new();
                let (api, upstream) = start(&fixture, checked);
                let request = api.request();
                cancel(&api, &request);
                let declined = terminal(
                    &api,
                    &fixture,
                    &request,
                    &format!("decline-{decision}"),
                    "30",
                    "one.py",
                );
                assert_ne!(declined["status"], 0, "{declined}");
                assert_eq!(declined["prompts"].as_array().unwrap().len(), decision);
                preserved(&fixture, b"before\n");
                assert_eq!(api.bind("events", &request).0, 409);

                let inspect = || {
                    let output = terminal_command(api.address, &fixture, "allow", &["original"]);
                    assert_eq!(output["status"], 0, "{output}");
                    assert_eq!(output["prompts"], json!([]));
                    let inspected: Value =
                        serde_json::from_str(output["transcript"].as_str().unwrap()).unwrap();
                    assert_eq!(
                        inspected,
                        api.call("GET", "/v1/code/repository/apply", CONTROL, None)
                            .1
                    );
                    inspected
                };
                let status = inspect();
                assert_eq!(status["pending"].is_null(), decision == 2);
                assert!(status["latest"].is_null());
                assert_eq!(status["available_paths"], json!(["one.py"]));

                if decision == 2 {
                    let prepared = terminal_command(
                        api.address,
                        &fixture,
                        "allow",
                        &[
                            "original-prepare",
                            fixture.0.join("original").to_str().unwrap(),
                            fixture.0.join("owned/worktree").to_str().unwrap(),
                            "Review retained changes",
                            "one.py",
                        ],
                    );
                    assert_eq!(prepared["status"], 0, "{prepared}");
                    let prompts = prepared["prompts"].as_array().unwrap();
                    assert_eq!(prompts.len(), 1);
                    assert_eq!(
                        prompts[0],
                        format!(
                            "read {} {}",
                            status["repository_root_sha256"].as_str().unwrap(),
                            status["worktree_root_sha256"].as_str().unwrap()
                        )
                    );
                    preserved(&fixture, b"before\n");
                }
                let preview = inspect()["pending"].clone();
                assert_eq!(preview["operation"], "apply-to-original");
                assert_eq!(preview["tested"], false);
                let original = fixture.0.join("original");
                let args = if approve {
                    vec!["original-apply", original.to_str().unwrap()]
                } else {
                    vec!["original-deny"]
                };
                let output = terminal_command(api.address, &fixture, "allow", &args);
                assert_eq!(output["status"], 0, "{output}");
                if approve {
                    assert_eq!(
                        output["prompts"],
                        json!([format!(
                            "apply-original {}",
                            preview["preview_sha256"].as_str().unwrap()
                        )])
                    );
                    assert!(
                        output["transcript"]
                            .as_str()
                            .unwrap()
                            .contains("write_preview")
                    );
                } else {
                    assert_eq!(output["prompts"], json!([]));
                }
                preserved(&fixture, if approve { b"first\n" } else { b"before\n" });
                let status = inspect();
                assert!(status["pending"].is_null());
                assert_eq!(
                    status["latest"]["outcome"],
                    if approve { "committed" } else { "denied" }
                );
                assert_eq!(
                    status["latest"]["preview_sha256"],
                    preview["preview_sha256"]
                );
                assert_eq!(
                    status["available_paths"],
                    if approve {
                        json!([])
                    } else {
                        json!(["one.py"])
                    }
                );

                // The consumed decision cannot be applied a second time, and
                // standalone recovery never restores the cancelled run.
                let repeated = terminal_command(
                    api.address,
                    &fixture,
                    "allow",
                    &["original-apply", original.to_str().unwrap()],
                );
                assert_ne!(repeated["status"], 0, "{repeated}");
                assert_eq!(repeated["prompts"], json!([]));
                assert_eq!(inspect(), status);
                assert_eq!(api.bind("events", &request).0, 409);
                assert_eq!(api.bind("release", &request).0, 409);
                preserved(&fixture, if approve { b"first\n" } else { b"before\n" });
                drop(api);
                assert!(
                    matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
                );
            }
        }
    }
}

#[test]
fn terminal_original_transfer_interrupt_deadline_and_original_drift_preserve_human_bytes() {
    for scenario in ["interrupt-apply", "timeout-read", "drift-apply"] {
        let fixture = Fixture::new();
        let (api, upstream) = start(&fixture, true);
        let request = api.request();
        cancel(&api, &request);
        let output = terminal(
            &api,
            &fixture,
            &request,
            scenario,
            if scenario == "timeout-read" {
                "2"
            } else {
                "30"
            },
            "one.py",
        );
        assert_ne!(output["status"], 0, "{output}");
        assert_eq!(
            output["prompts"].as_array().unwrap().len(),
            if scenario == "timeout-read" { 2 } else { 3 }
        );
        preserved(
            &fixture,
            if scenario == "drift-apply" {
                b"later human edit\n"
            } else {
                b"before\n"
            },
        );
        let status = api
            .call("GET", "/v1/code/repository/apply", CONTROL, None)
            .1;
        if scenario == "interrupt-apply" {
            assert!(!status["pending"].is_null());
        }
        if scenario == "timeout-read" {
            assert!(status["pending"].is_null());
        }
        drop(api);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn terminal_original_transfer_refuses_unstarted_foreign_or_invalid_input_before_release() {
    let fixture = Fixture::new();
    let (api, upstream) = start(&fixture, false);
    let request = api.request();
    let output = terminal(&api, &fixture, &request, "allow", "30", "one.py");
    assert_ne!(output["status"], 0);
    assert_eq!(output["prompts"], json!([]));
    assert_eq!(
        api.call("GET", "/v1/code/endpoint/request", CONTROL, None)
            .0,
        200
    );
    cancel(&api, &request);
    let mut foreign = request.clone();
    foreign["request_sha256"] = json!("0".repeat(64));
    let output = terminal(&api, &fixture, &foreign, "allow", "30", "one.py");
    assert_ne!(output["status"], 0);
    assert_eq!(output["prompts"], json!([]));
    for (seconds, path) in [
        ("0", "one.py"),
        ("3601", "one.py"),
        ("30", "../one.py"),
        ("30", ".git/config"),
    ] {
        let output = terminal(&api, &fixture, &request, "allow", seconds, path);
        assert_ne!(output["status"], 0, "{output}");
        assert_eq!(output["prompts"], json!([]));
    }
    std::os::unix::fs::symlink(fixture.0.join("original"), fixture.0.join("original-alias"))
        .unwrap();
    std::os::unix::fs::symlink(&fixture.0, fixture.0.join("root-alias")).unwrap();
    std::os::unix::fs::symlink(fixture.0.join("owned"), fixture.0.join("owned-alias")).unwrap();
    std::os::unix::fs::symlink(
        fixture.0.join("owned/worktree"),
        fixture.0.join("worktree-alias"),
    )
    .unwrap();
    fs::write(fixture.0.join("not-a-directory"), b"sentinel").unwrap();
    for (original, worktree) in [
        ("original-alias", "owned/worktree"),
        ("root-alias/original", "owned/worktree"),
        ("original", "worktree-alias"),
        ("original", "owned-alias/worktree"),
        ("original/.", "owned/worktree"),
        ("original/", "owned/worktree"),
        ("original", "owned//worktree"),
        ("original", "owned/worktree/"),
        ("missing", "owned/worktree"),
        ("original", "missing"),
        ("not-a-directory", "owned/worktree"),
        ("original", "not-a-directory"),
    ] {
        let output = terminal_command(
            api.address,
            &fixture,
            "allow",
            &[
                "endpoint-original-apply",
                "30",
                request["run_id"].as_str().unwrap(),
                request["request_sha256"].as_str().unwrap(),
                fixture.0.join(original).to_str().unwrap(),
                fixture.0.join(worktree).to_str().unwrap(),
                "Transfer approved changes",
                "one.py",
            ],
        );
        assert_ne!(output["status"], 0, "{original} {worktree}: {output}");
        assert_eq!(output["prompts"], json!([]));
        assert!(
            output["transcript"]
                .as_str()
                .unwrap()
                .starts_with("Coding request failed")
        );
        assert_eq!(api.bind("events", &request).0, 200);
        assert_eq!(
            api.call("GET", "/v1/code/repository/apply", CONTROL, None)
                .0,
            428
        );
        preserved(&fixture, b"before\n");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
        .args([
            &api.address.to_string(),
            fixture.0.join("auth").to_str().unwrap(),
            "endpoint-original-apply",
            "30",
            request["run_id"].as_str().unwrap(),
            request["request_sha256"].as_str().unwrap(),
            fixture.0.join("original").to_str().unwrap(),
            fixture.0.join("owned/worktree").to_str().unwrap(),
            "Transfer",
            "one.py",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(api.bind("events", &request).0, 200);
    preserved(&fixture, b"before\n");
    drop(api);
    assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}
