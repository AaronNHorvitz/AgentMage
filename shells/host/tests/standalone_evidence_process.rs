//! Actual standalone application and evidence host processes (Decision 0150):
//! the bridge binary launches and authenticates its sibling host over private
//! IPC, and every request and event crosses the development presentation.
//! The model is the term-match fixture; this is executable-scripted evidence,
//! not model qualification or a window presentation.
#![cfg(target_os = "linux")]

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc::{Receiver, RecvTimeoutError};
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};

    const PROJECT: &str = "# Aurora project\n\nAurora launches on 18 October 2026.\nProject lead: Mira Chen.\nThe Aurora budget is 42,000 credits.\n";
    const WAIT: Duration = Duration::from_secs(60);

    static NEXT: AtomicU64 = AtomicU64::new(1);

    struct Roots {
        base: PathBuf,
        state: PathBuf,
        disposable: PathBuf,
    }

    impl Roots {
        // The host's private socket lives in the state root, and a Unix
        // socket path is limited to 107 bytes, so the roots stay short.
        fn new(tag: &str) -> Self {
            let base = std::env::temp_dir().canonicalize().unwrap().join(format!(
                "ams-{}-{}-{}",
                &tag[..tag.len().min(4)],
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let state = base.join("state");
            let disposable = base.join("disposable");
            for path in [&base, &state, &disposable] {
                std::fs::create_dir(path).unwrap();
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
            Self {
                base,
                state,
                disposable,
            }
        }

        fn aurora(&self) -> PathBuf {
            let folder = self.disposable.join("aurora");
            std::fs::create_dir(&folder).unwrap();
            std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
            std::fs::write(folder.join("project.md"), PROJECT).unwrap();
            std::fs::write(folder.join("report.pdf"), b"%PDF-1.4").unwrap();
            folder
        }

        /// The host process identities named by their private sockets.
        fn host_pids(&self) -> Vec<i32> {
            std::fs::read_dir(&self.state)
                .unwrap()
                .filter_map(|entry| {
                    let name = entry.ok()?.file_name().into_string().ok()?;
                    let rest = name.strip_prefix("standalone-evidence-")?;
                    rest.split('-').next()?.parse().ok()
                })
                .collect()
        }
    }

    impl Drop for Roots {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }

    /// Whether a process still exists and has not exited.
    fn running(pid: i32) -> bool {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .ok()
            .and_then(|stat| {
                stat.rsplit_once(") ")
                    .and_then(|(_, rest)| rest.chars().next())
            })
            .is_some_and(|state| state != 'Z' && state != 'X')
    }

    struct Bridge {
        child: Child,
        input: ChildStdin,
        events: Receiver<Value>,
        sequence: u64,
    }

    impl Bridge {
        fn launch(roots: &Roots) -> Self {
            Self::launch_with_delay(roots, None)
        }

        fn launch_with_delay(roots: &Roots, delay_ms: Option<u16>) -> Self {
            let mut command = Command::new(env!("CARGO_BIN_EXE_agentmage-standalone"));
            command
                .arg("--development-stdio")
                .arg(&roots.state)
                .arg(&roots.disposable);
            if let Some(delay_ms) = delay_ms {
                command
                    .arg("--fixture-step-delay-ms")
                    .arg(delay_ms.to_string());
            }
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("the bridge starts");
            let input = child.stdin.take().unwrap();
            let output = child.stdout.take().unwrap();
            let (sender, events) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                for line in BufReader::new(output).lines() {
                    let Ok(line) = line else { break };
                    let event: Value =
                        serde_json::from_str(&line).expect("one closed event per line");
                    if sender.send(event).is_err() {
                        break;
                    }
                }
            });
            Self {
                child,
                input,
                events,
                sequence: 0,
            }
        }

        fn send(&mut self, mut request: Value) -> u64 {
            self.sequence += 1;
            request["sequence"] = json!(self.sequence);
            self.raw(&serde_json::to_string(&request).unwrap());
            self.sequence
        }

        fn raw(&mut self, line: &str) {
            writeln!(self.input, "{line}").unwrap();
            self.input.flush().unwrap();
        }

        fn next(&self) -> Value {
            match self.events.recv_timeout(WAIT) {
                Ok(event) => {
                    if std::env::var_os("STANDALONE_DEBUG").is_some() {
                        eprintln!("EVENT {event}");
                    }
                    event
                }
                Err(RecvTimeoutError::Timeout) => panic!("no event in time"),
                Err(RecvTimeoutError::Disconnected) => panic!("the bridge ended"),
            }
        }

        /// The next event of `kind`, skipping progress.
        fn until(&self, kind: &str) -> Value {
            loop {
                let event = self.next();
                if event["type"] == kind {
                    return event;
                }
                assert_eq!(event["type"], "progress", "unexpected {event}");
            }
        }

        fn exit_code(mut self) -> i32 {
            drop(self.input);
            let deadline = Instant::now() + WAIT;
            loop {
                if let Some(status) = self.child.try_wait().unwrap() {
                    return status.code().unwrap_or(-1);
                }
                assert!(Instant::now() < deadline, "the bridge exits in time");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }

    fn state(bridge: &Bridge) -> Value {
        let event = bridge.next();
        assert_eq!(event["type"], "state", "{event}");
        event["state"].clone()
    }

    fn admit(bridge: &mut Bridge, folder: &Path) -> Value {
        bridge.send(json!({"type": "admit_folder", "folder": folder.to_str().unwrap()}));
        let event = bridge.next();
        assert_eq!(event["type"], "folder", "{event}");
        event["admission"].clone()
    }

    fn ask(bridge: &mut Bridge, question: &str) -> Value {
        let sequence = bridge.send(json!({"type": "ask", "question": question}));
        let started = bridge.next();
        assert_eq!(started["type"], "question_started", "{started}");
        assert_eq!(started["sequence"], sequence);
        let answer = bridge.until("answer");
        assert_eq!(answer["sequence"], sequence);
        answer["answer"].clone()
    }

    #[test]
    fn the_bridge_answers_from_an_admitted_folder_through_its_own_host() {
        let roots = Roots::new("answers");
        let folder = roots.aurora();
        let mut bridge = Bridge::launch(&roots);
        assert_eq!(state(&bridge), json!({"state": "starting"}));
        assert_eq!(state(&bridge), json!({"state": "ready"}));
        let hosts = roots.host_pids();
        assert_eq!(hosts.len(), 1, "one authenticated host");
        assert!(running(hosts[0]));

        let admission = admit(&mut bridge, &folder);
        assert_eq!(admission["accepted"], 1);
        assert_eq!(admission["skipped"], 1);
        assert_eq!(admission["folder"], folder.to_str().unwrap());

        let answer = ask(&mut bridge, "When does Aurora launch?");
        assert_eq!(answer["state"], "verified", "{answer}");
        assert_eq!(
            answer["statements"][0]["text"],
            "Aurora launches on 18 October 2026."
        );
        let card = &answer["cards"][0];
        assert_eq!(card["path"], "project.md");
        assert_eq!(
            (card["first_line"].clone(), card["last_line"].clone()),
            (json!(3), json!(3))
        );
        assert_eq!(card["quote"], "Aurora launches on 18 October 2026.");
        assert!(
            answer["label"]
                .as_str()
                .unwrap()
                .contains("not a language model")
        );

        let unmatched = ask(&mut bridge, "What is the favorite flavor?");
        assert_eq!(unmatched["state"], "unverified");
        assert_eq!(unmatched["no_matching_text"], true);
        assert_eq!(unmatched["cards"], json!([]));

        // With no question in progress there is nothing to cancel.
        let idle = bridge.send(json!({"type": "cancel"}));
        let refused = bridge.next();
        assert_eq!(refused["code"], "standalone.shell.nothing-to-cancel");
        assert_eq!(refused["sequence"], idle);

        // Malformed and out-of-order requests are refused before the host.
        bridge.raw(r#"{"type":"ask","sequence":999,"question":"q","model":"other"}"#);
        assert_eq!(bridge.next()["code"], "standalone.shell.request-malformed");
        bridge.raw(r#"{"type":"cancel","sequence":1}"#);
        let refused = bridge.next();
        assert_eq!(refused["code"], "standalone.shell.sequence-not-increasing");
        assert_eq!(refused["sequence"], 1);

        assert_eq!(
            std::fs::read_to_string(folder.join("project.md")).unwrap(),
            PROJECT
        );
        bridge.send(json!({"type": "shutdown"}));
        assert_eq!(bridge.exit_code(), 0);
        assert!(!running(hosts[0]), "the host was stopped and reaped");
    }

    #[test]
    fn a_cancelled_question_stops_through_its_job_ledger() {
        // The fixture waits two seconds before each proposal, so the cancel
        // arrives while the question runs; the bridge sends it through the
        // run's job ledger and the host stops the run (Decision 0120).
        let roots = Roots::new("cancel");
        let folder = roots.aurora();
        let mut bridge = Bridge::launch_with_delay(&roots, Some(2_000));
        assert_eq!(state(&bridge), json!({"state": "starting"}));
        assert_eq!(state(&bridge), json!({"state": "ready"}));
        admit(&mut bridge, &folder);
        let started = Instant::now();
        let sequence = bridge.send(json!({"type": "ask", "question": "When does Aurora launch?"}));
        assert_eq!(bridge.next()["sequence"], sequence);
        // The run has begun before the cancel is sent.
        let first = bridge.next();
        assert_eq!(first["type"], "progress", "{first}");
        bridge.send(json!({"type": "cancel"}));
        let mut events = Vec::new();
        let answer = loop {
            let event = bridge.next();
            if event["type"] == "answer" {
                break event;
            }
            assert_eq!(event["type"], "progress", "{event}");
            events.push(event["event"].as_str().unwrap().to_owned());
        };
        assert_eq!(answer["sequence"], sequence);
        let answer = &answer["answer"];
        assert_eq!(answer["state"], "cancelled", "{answer}");
        assert_eq!(answer["cards"], json!([]));
        assert_eq!(answer["statements"], json!([]));
        assert!(
            events.contains(&"cancellation_requested".to_owned()),
            "{events:?}"
        );
        assert!(
            events.contains(&"cancellation_observed".to_owned()),
            "{events:?}"
        );
        assert!(
            !events.iter().any(|event| event == "tool_completed"),
            "{events:?}"
        );
        // The cancelled run did not wait out its delay.
        assert!(started.elapsed() < Duration::from_secs(10));
        // The bridge is still ready: the next question runs to its end.
        assert_eq!(
            ask(&mut bridge, "When does Aurora launch?")["state"],
            "verified"
        );
        bridge.send(json!({"type": "shutdown"}));
        assert_eq!(bridge.exit_code(), 0);
    }

    #[test]
    fn a_failed_host_puts_the_bridge_in_safe_mode_until_a_restart() {
        let roots = Roots::new("safe-mode");
        let folder = roots.aurora();
        let mut bridge = Bridge::launch(&roots);
        assert_eq!(state(&bridge), json!({"state": "starting"}));
        assert_eq!(state(&bridge), json!({"state": "ready"}));
        admit(&mut bridge, &folder);
        let host = roots.host_pids()[0];
        rustix::process::kill_process(
            rustix::process::Pid::from_raw(host).unwrap(),
            rustix::process::Signal::KILL,
        )
        .unwrap();
        let sequence = bridge.send(json!({"type": "ask", "question": "When does Aurora launch?"}));
        let mut safe = None;
        while safe.is_none() {
            let event = bridge.next();
            match event["type"].as_str() {
                Some("question_started") => assert_eq!(event["sequence"], sequence),
                Some("state") => safe = Some(event["state"].clone()),
                other => panic!("unexpected {other:?}: {event}"),
            }
        }
        let safe = safe.unwrap();
        assert_eq!(safe["state"], "safe_mode", "{safe}");
        assert!(safe["code"].as_str().is_some_and(|code| !code.is_empty()));
        // Nothing reaches a failed host.
        bridge.send(json!({"type": "ask", "question": "When does Aurora launch?"}));
        assert_eq!(bridge.next()["code"], "standalone.shell.not-ready");
        // A restart brings a new host; the earlier snapshot is gone.
        bridge.send(json!({"type": "restart"}));
        assert_eq!(state(&bridge), json!({"state": "starting"}));
        assert_eq!(state(&bridge), json!({"state": "ready"}));
        let sequence = bridge.send(json!({"type": "ask", "question": "When does Aurora launch?"}));
        let refused = bridge.next();
        assert_eq!(refused["code"], "standalone.shell.no-folder");
        assert_eq!(refused["sequence"], sequence);
        admit(&mut bridge, &folder);
        assert_eq!(
            ask(&mut bridge, "When does Aurora launch?")["state"],
            "verified"
        );
        // End of input stops the host too.
        let hosts = roots.host_pids();
        assert_eq!(bridge.exit_code(), 0);
        for pid in hosts {
            assert!(!running(pid), "host {pid} stopped");
        }
    }

    #[test]
    fn an_unusable_activation_is_reported_as_unavailable() {
        let roots = Roots::new("unavailable");
        std::fs::set_permissions(&roots.state, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut bridge = Bridge::launch(&roots);
        assert_eq!(state(&bridge), json!({"state": "starting"}));
        let unavailable = state(&bridge);
        assert_eq!(unavailable["state"], "unavailable", "{unavailable}");
        assert!(
            unavailable["code"]
                .as_str()
                .is_some_and(|code| !code.is_empty())
        );
        bridge.send(json!({"type": "admit_folder", "folder": roots.disposable.to_str().unwrap()}));
        assert_eq!(bridge.next()["code"], "standalone.shell.not-ready");
        bridge.send(json!({"type": "shutdown"}));
        assert_eq!(bridge.exit_code(), 0);
        assert!(roots.host_pids().is_empty());
    }

    #[test]
    fn without_a_window_the_application_says_so() {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-standalone"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(
            String::from_utf8(output.stderr).unwrap().trim(),
            "standalone.presentation.unavailable"
        );
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-standalone"))
            .arg("--development-stdio")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}
