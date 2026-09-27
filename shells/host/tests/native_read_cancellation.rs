//! Actual CLI/host cancellation through consumed authority and the canonical store.
//! Explicit scripted-model, feature-gated fault-worker diagnostic; not model qualification.
#![cfg(target_os = "linux")]

#[cfg(test)]
mod tests {

    use std::fmt::Write;
    use std::fs::{self, File, OpenOptions};
    use std::io::Read;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use agentmage_host::coding_development_activation::{
        CODING_DEVELOPMENT_ACTIVATION, CodingDevelopmentActivation, CodingDevelopmentKeyProvider,
    };
    use agentmage_kernel_contracts::{
        AdapterInstanceId, AgentStateKind, AuthorityTransactionState, CONTRACT_SCHEMA_VERSION,
        GrantOperation, GrantStatus, OperationOutcome, RuntimeArtifactRef, RuntimeEvent,
        RuntimeEventKind,
    };
    use agentmage_kernel_engine::runtime_artifact::RuntimeArtifactReadRequest;
    use agentmage_kernel_engine::runtime_event::RuntimeEventSequence;
    use agentmage_platform_linux::{
        LinuxDevelopmentPlatformAdapter, open_linux_development_authority,
    };
    use sha2::{Digest, Sha256};

    const MAX_LOG_BYTES: u64 = 1024 * 1024;
    const READ_CALL: &str = "scripted-inspect-preimage";

    fn now_ms() -> u64 {
        u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    }

    fn digest(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            write!(&mut output, "{byte:02x}").expect("String write");
        }
        output
    }

    fn log_file(path: &Path) -> File {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap()
    }

    fn bounded_log(path: &Path) -> Vec<u8> {
        let file = File::open(path).unwrap();
        assert!(
            file.metadata().unwrap().len() <= MAX_LOG_BYTES,
            "fixture log ceiling"
        );
        let mut bytes = Vec::new();
        file.take(MAX_LOG_BYTES + 1)
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(bytes.len() as u64 <= MAX_LOG_BYTES);
        bytes
    }

    fn events(bytes: &[u8]) -> Vec<RuntimeEvent> {
        // Polls may see a final partial write. Every COMPLETE JSON line must parse;
        // terminal verification separately requires a final newline.
        bytes
            .split_inclusive(|byte| *byte == b'\n')
            .filter_map(|line| {
                if !line.ends_with(b"\n") {
                    return None;
                }
                let value: serde_json::Value = serde_json::from_slice(line).unwrap();
                if value.get("event_id").is_some() {
                    Some(serde_json::from_value(value).unwrap())
                } else {
                    None
                }
            })
            .collect()
    }

    struct OwnedCli(Child);

    impl OwnedCli {
        fn cancel(&self) {
            let pid = rustix::process::Pid::from_raw(i32::try_from(self.0.id()).unwrap()).unwrap();
            // The direct child is not reaped until completion, so its identity is held.
            rustix::process::kill_process(pid, rustix::process::Signal::INT).unwrap();
        }
    }

    impl Drop for OwnedCli {
        fn drop(&mut self) {
            match self.0.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {}
                Err(_) => {
                    // An unexpected reaping error is not permission to signal a
                    // numeric PID whose ownership can no longer be established.
                    eprintln!("native-descendant-cleanup-unresolved cli-wait-error=true");
                    return;
                }
            }
            let pid = rustix::process::Pid::from_raw(i32::try_from(self.0.id()).unwrap()).unwrap();
            let _ = rustix::process::kill_process(pid, rustix::process::Signal::INT);
            let deadline = Instant::now() + Duration::from_secs(30);
            while Instant::now() < deadline {
                match self.0.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => {}
                    Err(_) => {
                        eprintln!("native-descendant-cleanup-unresolved cli-wait-error=true");
                        return;
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
            // Stop/reap only the direct owned CLI; this cannot attest host/worker
            // cleanup. The supervised native lane must reconcile that failure before
            // another case. Never signal by a guessed descendant or unit name.
            let _ = self.0.kill();
            let reap_deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < reap_deadline {
                match self.0.try_wait() {
                    Ok(Some(_)) => {
                        eprintln!("native-descendant-cleanup-unresolved cli-reaped=true");
                        return;
                    }
                    Ok(None) => {}
                    Err(_) => {
                        eprintln!("native-descendant-cleanup-unresolved cli-wait-error=true");
                        return;
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
            eprintln!(
                "native-descendant-cleanup-unresolved cli-reaped=false owned-cli-pid={}",
                self.0.id()
            );
        }
    }

    fn fixture_setup(repo: &Path, base: &Path) {
        assert!(!base.exists());
        let result = Command::new("python3")
            .arg(repo.join("scripts/coding_harness.py"))
            .args(["setup", "--root"])
            .arg(base)
            .args(["--fixture", "repair"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "existing synthetic setup must pass"
        );
    }

    fn copy_binary(source: &Path, destination: &Path) -> String {
        assert!(!destination.exists());
        fs::copy(source, destination).unwrap();
        fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
        let expected = digest(&fs::read(source).unwrap());
        assert_eq!(digest(&fs::read(destination).unwrap()), expected);
        expected
    }

    fn run_cli(bundle: &Path, base: &Path, cancel_read: bool) -> (ExitStatus, Vec<RuntimeEvent>) {
        let stdout = base.join("stdout.jsonl");
        let stderr = base.join("stderr.log");
        let mut cli = OwnedCli(
            Command::new(bundle.join("agentmage"))
                .args(["--json", "code", "--development", "--state-root"])
                .arg(base.join("state"))
                .arg("--disposable-root")
                .arg(base.join("disposable"))
                .arg("--workspace-root")
                .arg(base.join("disposable/worktree"))
                .args([
                    "--scenario",
                    "failed-test-repair",
                    "--model",
                    "scripted",
                    "--objective",
                    "Repair the synthetic add function and validate it.",
                    "--approve-this-run",
                ])
                .stdin(Stdio::null())
                .stdout(log_file(&stdout))
                .stderr(log_file(&stderr))
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(90);
        let mut read_started = None;
        let mut cancellation_sent = false;
        let status = loop {
            let observed = events(&bounded_log(&stdout));
            let _ = bounded_log(&stderr);
            if cancel_read
                && read_started.is_none()
                && observed.iter().any(|event| {
                    matches!(&event.kind, RuntimeEventKind::ToolStarted { tool_call_id, .. }
                if tool_call_id.as_str() == READ_CALL)
                })
            {
                read_started = Some(Instant::now());
            }
            if !cancellation_sent
                && read_started.is_some_and(|start| start.elapsed() >= Duration::from_secs(2))
            {
                cli.cancel();
                cancellation_sent = true;
            }
            if let Some(status) = cli.0.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "bounded actual CLI campaign timed out"
            );
            thread::sleep(Duration::from_millis(20));
        };
        println!("diagnostic-cli-status={status} cancellation-sent={cancellation_sent}");
        assert_eq!(cancellation_sent, cancel_read);
        let bytes = bounded_log(&stdout);
        assert!(bytes.ends_with(b"\n"));
        let events = events(&bytes);
        let mut sequence = RuntimeEventSequence::new();
        for event in &events {
            sequence.push(event).unwrap();
        }
        assert!(sequence.is_terminal());
        (status, events)
    }

    #[test]
    #[ignore = "requires capped native Linux lane, built lifecycle fixture, and fresh private root"]
    fn actual_cli_native_read_cancel_retains_consumed_receipt_and_reopens() {
        // This is an explicitly scripted-model, fault-worker diagnostic. The real
        // product CLI, host, IPC, permissions, native sandbox and canonical owner run.
        let root = PathBuf::from(std::env::var_os("AGENTMAGE_NATIVE_CANCELLATION_ROOT").unwrap());
        assert!(root.is_absolute() && !root.exists());
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        assert_eq!(root.canonicalize().unwrap(), root);
        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let binary_root = repo.join("target/debug");
        let fixture = binary_root.join("agentmage-read-only-lifecycle-fixture");
        let bundle = root.join("fault-bin");
        fs::DirBuilder::new().mode(0o700).create(&bundle).unwrap();
        for name in ["agentmage", "agentmage-host"] {
            let identity = copy_binary(&binary_root.join(name), &bundle.join(name));
            println!("diagnostic-binary={name} sha256={identity}");
        }
        let fault_sha = copy_binary(&fixture, &bundle.join("agentmage-read-only-worker"));
        println!("diagnostic-fault-worker-sha256={fault_sha}");
        let base = root.join("cancel");
        fixture_setup(repo, &base);
        let preimage = fs::read(base.join("disposable/worktree/src/calc.py")).unwrap();
        let (status, events) = run_cli(&bundle, &base, true);
        assert_eq!(status.code(), Some(6));
        assert_eq!(
            fs::read(base.join("disposable/worktree/src/calc.py")).unwrap(),
            preimage
        );
        let start = events
            .iter()
            .position(|event| {
                matches!(&event.kind,
        RuntimeEventKind::ToolStarted { tool_call_id, .. } if tool_call_id.as_str() == READ_CALL)
            })
            .unwrap();
        let failed = events
            .iter()
            .position(|event| {
                matches!(&event.kind,
        RuntimeEventKind::ToolFailed { tool_call_id, .. } if tool_call_id.as_str() == READ_CALL)
            })
            .unwrap();
        assert!(start < failed);
        assert!(!events[start..].iter().any(|event| matches!(
            &event.kind,
            RuntimeEventKind::ToolCompleted { .. } | RuntimeEventKind::ArtifactCreated { .. }
        )));
        assert!(matches!(
            events.last().unwrap().kind,
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Cancelled,
                ..
            }
        ));
        let run_id = events[0].run_id.clone();
        let activation = CodingDevelopmentActivation::validate(
            &base.join("state"),
            &base.join("disposable"),
            &base.join("disposable/worktree"),
        )
        .unwrap();
        let platform = LinuxDevelopmentPlatformAdapter::activate(
            CODING_DEVELOPMENT_ACTIVATION,
            activation.marker_sha256().to_owned(),
            AdapterInstanceId::from_raw(format!(
                "coding-development-{}",
                &activation.marker_sha256()[..24]
            )),
        )
        .unwrap();
        let mut baseline = None;
        for _ in 0..2 {
            let mut key = CodingDevelopmentKeyProvider::open(&activation).unwrap();
            let runtime = open_linux_development_authority(
                &platform,
                activation.state_root(),
                &mut key,
                now_ms(),
            )
            .unwrap();
            let owner = runtime.authority();
            assert_eq!(owner.runtime_events(&run_id).unwrap(), events);
            let selected = owner
                .receipts()
                .iter()
                .filter(|receipt| {
                    receipt
                        .tool_call_id
                        .as_ref()
                        .is_some_and(|call| call.as_str() == READ_CALL)
                })
                .collect::<Vec<_>>();
            let [receipt] = selected.as_slice() else {
                panic!("one exact native read receipt required");
            };
            let receipt = *receipt;
            let transaction = owner
                .current_transaction(&receipt.authority_transaction_id)
                .unwrap();
            let grant = owner.current_grant(&receipt.grant_id).unwrap();
            assert_eq!(receipt.outcome, OperationOutcome::Cancelled);
            assert_eq!(receipt.operation.operation(), GrantOperation::WorkspaceRead);
            assert_eq!(transaction.state, AuthorityTransactionState::Terminal);
            assert_eq!(transaction.outcome, Some(OperationOutcome::Cancelled));
            assert!(!transaction.uncertain_effect);
            assert_eq!(grant.status, GrantStatus::Consumed);
            assert_eq!(grant.use_count, grant.use_limit);
            assert_eq!(transaction.grant_id, grant.grant_id);
            assert_eq!(transaction.session_id, receipt.session_id);
            assert_eq!(transaction.task_id, receipt.task_id);
            assert_eq!(transaction.correlation_id, receipt.correlation_id);
            assert_eq!(transaction.approval_id, receipt.approval_id);
            assert_eq!(transaction.action_id, receipt.action_id);
            assert_eq!(transaction.operation, receipt.operation);
            assert_eq!(transaction.tool_call_id.as_str(), READ_CALL);
            assert_eq!(grant.session_id, receipt.session_id);
            assert_eq!(grant.task_id, receipt.task_id);
            assert_eq!(grant.approval_id.as_ref(), Some(&receipt.approval_id));
            assert_eq!(grant.action_id.as_ref(), Some(&receipt.action_id));
            assert_eq!(grant.operation, receipt.operation);
            assert_eq!(
                transaction.operation_attempt_id,
                receipt.operation_attempt_id
            );
            assert_eq!(transaction.receipt_id.as_ref(), Some(&receipt.receipt_id));
            assert_eq!(
                transaction.receipt_sha256.as_ref(),
                Some(&receipt.receipt_sha256)
            );
            assert_eq!(
                transaction.consumed_grant_sha256.as_ref(),
                Some(&digest(
                    &agentmage_kernel_contracts::to_canonical_json(grant).unwrap()
                ))
            );
            // Driver hashes the raw stdout digest as redacted result material. This
            // proves native body entry, unlike a sleep or ToolStarted event alone.
            assert_eq!(
                transaction.result_sha256.as_ref(),
                Some(&digest(&Sha256::digest(b"{")))
            );
            // Read the complete earlier native validation output through its existing
            // canonical artifact owner, not a direct encrypted-store/path shortcut.
            let validation_end = events[..start]
                .iter()
                .position(|event| {
                    matches!(&event.kind,
            RuntimeEventKind::ToolCompleted { tool_call_id, .. }
                if tool_call_id.as_str() == "scripted-validation-failing")
                })
                .unwrap();
            let validation_event = &events[validation_end + 1];
            assert!(events[validation_end].turn_id.is_some());
            assert!(events[validation_end].operation_id.is_some());
            assert_eq!(validation_event.turn_id, events[validation_end].turn_id);
            assert_eq!(
                validation_event.operation_id,
                events[validation_end].operation_id
            );
            let RuntimeEventKind::ArtifactCreated {
                artifact_id,
                manifest_sha256,
            } = &validation_event.kind
            else {
                panic!("complete failed-test output must be retained");
            };
            let payload = validation_event.payload_reference.as_ref().unwrap();
            assert_eq!(&payload.artifact_id, artifact_id);
            assert_eq!(payload.media_type, "application/octet-stream");
            let reference = RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: artifact_id.clone(),
                manifest_sha256: manifest_sha256.clone(),
                payload_sha256: payload.sha256.clone(),
                byte_size: payload.byte_size,
                media_type: payload.media_type.clone(),
            };
            let raw = runtime
                .read_runtime_artifact(&RuntimeArtifactReadRequest {
                    session_id: receipt.session_id.clone(),
                    task_id: receipt.task_id.clone(),
                    policy_sha256: grant.policy_sha256.clone(),
                    reference,
                    now_epoch_ms: now_ms(),
                    maximum_bytes: MAX_LOG_BYTES,
                })
                .unwrap();
            assert_eq!(digest(&raw), payload.sha256);
            let validation: serde_json::Value = serde_json::from_slice(&raw).unwrap();
            assert_eq!(validation["status"], "assertion_failed");
            assert_eq!(validation["failed"], 1);
            assert_eq!(validation["passed"], 0);
            assert_eq!(
                validation["failed_names"],
                serde_json::json!(["fixture::add"])
            );
            assert!(
                matches!(&events[failed].kind, RuntimeEventKind::ToolFailed {
            receipt_id: Some(id), .. } if id == &receipt.receipt_id)
            );
            assert!(
                matches!(&events[failed + 1].kind, RuntimeEventKind::TurnCompleted {
            outcome_sha256 } if outcome_sha256 == &receipt.receipt_sha256)
            );
            let RuntimeEventKind::CancellationRequested { cancellation_id } =
                &events[failed + 2].kind
            else {
                panic!("original request must follow canonical turn closure");
            };
            assert_eq!(
                cancellation_id.as_str(),
                format!("cli-signal-{}-0000000000000001", run_id.as_str())
            );
            assert!(
                matches!(&events[failed + 3].kind, RuntimeEventKind::CancellationObserved {
            cancellation_id: observed } if observed == cancellation_id)
            );
            let current = (receipt.clone(), transaction.clone(), grant.clone());
            if let Some(expected) = &baseline {
                assert_eq!(&current, expected);
            }
            baseline = Some(current);
        }
        // Each reopen observes the exact consumed grant and immutable result. An
        // actual replay attempt and inter-commit crash recovery remain separate tests.
        println!(
            "continuous-native-read-cancel=pass canonical-reopens=2 model=scripted fault-worker=true"
        );
        let healthy = root.join("healthy-bin");
        fs::DirBuilder::new().mode(0o700).create(&healthy).unwrap();
        for name in ["agentmage", "agentmage-host", "agentmage-read-only-worker"] {
            let identity = copy_binary(&binary_root.join(name), &healthy.join(name));
            println!("healthy-binary={name} sha256={identity}");
        }
        let reuse = root.join("reuse");
        fixture_setup(repo, &reuse);
        let (status, healthy_events) = run_cli(&healthy, &reuse, false);
        assert_eq!(status.code(), Some(0));
        assert!(matches!(
            healthy_events.last().unwrap().kind,
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Success,
                ..
            }
        ));
        assert_eq!(
            fs::read(reuse.join("disposable/worktree/src/calc.py")).unwrap(),
            b"def add(left, right):\n    return left + right\n"
        );
        assert_eq!(
            digest(&fs::read(bundle.join("agentmage-read-only-worker")).unwrap()),
            fault_sha
        );
        println!(
            "fresh-host-native-read-reuse=pass scripted-repair=pass model-qualification=false"
        );
        // Logs, disposable repository and diagnostic bundle are deliberately retained.
    }
}
